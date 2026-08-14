//! `lisa upgrade`: move this machine to the release its channel names.
//!
//! The channel rules live in [`crate::channel`]; this module is the part that
//! touches the world — reading the published release list, saying what it is
//! about to do, and running the release's own installer.
//!
//! ## Three decisions this command makes, and why
//!
//! **Where the release list comes from.** Straight from the GitHub releases
//! API for this repository. GitHub's `/releases/latest` redirect is the thing
//! that froze the curl-installed machines on v0.4.4 — it skips prereleases and
//! there is no way to ask it not to — so `upgrade` reads the full list and
//! applies the channel rule itself.
//!
//! **What happens with no network.** It fails loudly, exits non-zero, and
//! leaves the installed Lisa exactly where it was. An upgrader that guesses
//! when it cannot see is worse than one that stops.
//!
//! **What happens on a brew- or apt-managed box.** It refuses, and names both
//! ways forward. One Homebrew formula and one apt suite cannot carry three
//! channels, and writing over a file a package manager owns produces a machine
//! whose next `brew upgrade` silently undoes the channel. Brew and apt stay the
//! hands-off stable door; the shell installer's `~/.local/bin` is the
//! channel-aware path.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use semver::Version;

use crate::channel::{self, Channel, Release};

/// The release list Lisa resolves channels against.
const RELEASES_API: &str = "https://api.github.com/repos/johnhkchen/lisa/releases?per_page=100";

/// Environment override for the release list, so a test can serve a fixed one.
const RELEASES_URL_ENV: &str = "LISA_RELEASES_URL";

/// Where a release's artifacts are published.
const DOWNLOAD_BASE: &str = "https://github.com/johnhkchen/lisa/releases/download";

/// Environment override for the artifact base, paired with [`RELEASES_URL_ENV`].
const DOWNLOAD_BASE_ENV: &str = "LISA_DOWNLOAD_BASE";

/// The shell installer cargo-dist publishes with every release.
const INSTALLER_NAME: &str = "lisa-cli-installer.sh";

/// The one-command install line, quoted verbatim in the package-manager
/// refusal because that is the command that moves a box onto the channel path.
const INSTALL_COMMAND: &str = "curl --proto '=https' --tlsv1.2 -LsSf \
    https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh";

/// What `lisa upgrade` was asked to do.
pub(crate) struct UpgradeArgs {
    /// Channel to put this machine on before upgrading.
    pub(crate) channel: Option<String>,
    /// Exact release to pin to, ignoring the channel. This is the rollback.
    pub(crate) tag: Option<String>,
    /// Resolve and report, download and install nothing.
    pub(crate) dry_run: bool,
}

/// How the running `lisa` got onto this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InstallMethod {
    /// Homebrew owns this file.
    Homebrew,
    /// A Debian package owns this file.
    Apt,
    /// The shell installer's own directory, `~/.local/bin`.
    ShellInstaller,
    /// Somewhere else — a cargo build, a hand-placed copy, a different prefix.
    Elsewhere,
}

/// Classify an executable path by who owns it.
///
/// Resolve symlinks before calling: Homebrew puts `lisa` on `PATH` as a symlink
/// into its Cellar, and it is the Cellar path that identifies it.
pub(crate) fn classify_install(exe: &Path, home: Option<&Path>) -> InstallMethod {
    let path = exe.to_string_lossy();

    if path.contains("/Cellar/") || path.contains("/homebrew/") || path.contains("/linuxbrew/") {
        return InstallMethod::Homebrew;
    }
    if path.starts_with("/usr/bin/") || path.starts_with("/usr/libexec/") {
        return InstallMethod::Apt;
    }
    if let Some(home) = home {
        if exe.parent() == Some(&home.join(".local").join("bin")) {
            return InstallMethod::ShellInstaller;
        }
    }
    InstallMethod::Elsewhere
}

/// The refusal a package-managed Lisa gets, naming both ways forward.
fn package_managed_refusal(method: &InstallMethod, exe: &Path) -> String {
    let (manager, upgrade_command, remove_command) = match method {
        InstallMethod::Homebrew => (
            "Homebrew",
            "brew update && brew upgrade lisa",
            "brew uninstall lisa",
        ),
        InstallMethod::Apt => (
            "apt",
            "sudo apt-get update && sudo apt-get install --only-upgrade lisa",
            "sudo apt-get remove lisa",
        ),
        _ => unreachable!("only package-managed installs are refused"),
    };

    format!(
        "this lisa is managed by {manager} ({}), and lisa upgrade will not write over a file \
         {manager} owns.\n\
         {manager} carries one current version, so it cannot carry three channels. Either:\n  \
         - stay with {manager}, which tracks stable, and upgrade with:\n      {upgrade_command}\n  \
         - or move this machine onto channels, which live in ~/.local/bin:\n      \
         {remove_command}\n      {INSTALL_COMMAND}\n    then run: lisa upgrade --channel <name>",
        exe.display(),
    )
}

/// How long `upgrade` waits on the release list. It is about to download and
/// run an installer, so it can afford to wait.
const UPGRADE_LIST_TIMEOUT: Duration = Duration::from_secs(30);

/// How long `doctor` waits on the same list. Shorter on purpose: `doctor` is
/// one row in a report a person is reading, and an unreachable list is an
/// answer it can give rather than a reason to sit there.
pub(crate) const DOCTOR_LIST_TIMEOUT: Duration = Duration::from_secs(8);

/// Read the published release list.
pub(crate) fn fetch_releases() -> Result<Vec<Release>, String> {
    fetch_releases_within(UPGRADE_LIST_TIMEOUT)
}

/// Read the published release list, waiting no longer than `read_timeout` for
/// the body.
pub(crate) fn fetch_releases_within(read_timeout: Duration) -> Result<Vec<Release>, String> {
    let url = std::env::var(RELEASES_URL_ENV).unwrap_or_else(|_| RELEASES_API.to_string());
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10).min(read_timeout))
        .timeout_read(read_timeout)
        .timeout_write(Duration::from_secs(10).min(read_timeout))
        .redirects(5)
        .build();

    let body = agent
        .get(&url)
        .set("User-Agent", concat!("lisa/", env!("CARGO_PKG_VERSION")))
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| format!("cannot read the release list at {url}: {error}"))?
        .into_string()
        .map_err(|error| format!("cannot read the release list at {url}: {error}"))?;

    parse_releases(&body)
}

/// One entry of the GitHub releases API, reduced to the fields resolution uses.
#[derive(Debug, serde::Deserialize)]
struct PublishedRelease {
    tag_name: String,
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
}

/// Turn a releases-API response into releases, dropping drafts and any tag that
/// is not a `v<semver>` Lisa release.
pub(crate) fn parse_releases(body: &str) -> Result<Vec<Release>, String> {
    let published: Vec<PublishedRelease> = serde_json::from_str(body)
        .map_err(|error| format!("the release list was not readable: {error}"))?;

    Ok(published
        .into_iter()
        .filter(|entry| !entry.draft)
        .filter_map(|entry| {
            let published_at = channel::parse_rfc3339_utc(entry.published_at.as_deref()?)?;
            Release::from_tag(&entry.tag_name, published_at)
        })
        .collect())
}

/// Download a release's shell installer and run it.
///
/// The installer writes a new binary into `~/.local/bin`; nothing here removes
/// or truncates the running one, so a failed download or a failed installer
/// leaves the working Lisa in place.
fn install(release: &Release) -> Result<(), String> {
    let base = std::env::var(DOWNLOAD_BASE_ENV).unwrap_or_else(|_| DOWNLOAD_BASE.to_string());
    let url = format!("{base}/{}/{INSTALLER_NAME}", release.tag);

    let scratch = tempfile::tempdir()
        .map_err(|error| format!("cannot create a temporary directory: {error}"))?;
    let script = scratch.path().join(INSTALLER_NAME);

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .timeout_write(Duration::from_secs(10))
        .redirects(5)
        .build();
    let response = agent
        .get(&url)
        .set("User-Agent", concat!("lisa/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|error| format!("cannot download {url}: {error}"))?;

    let mut writer = BufWriter::new(
        File::create(&script)
            .map_err(|error| format!("cannot create {}: {error}", script.display()))?,
    );
    io::copy(&mut response.into_reader(), &mut writer)
        .map_err(|error| format!("cannot store {url}: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("cannot store {url}: {error}"))?;
    drop(writer);

    let status = Command::new("sh")
        .arg(&script)
        .status()
        .map_err(|error| format!("cannot run the installer for {}: {error}", release.tag))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "the installer for {} exited {}",
            release.tag,
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "on a signal".to_string()),
        ))
    }
}

/// Find an exact tag in the release list, accepting it with or without the `v`.
fn find_tag<'a>(releases: &'a [Release], requested: &str) -> Result<&'a Release, String> {
    let with_prefix = if requested.starts_with('v') {
        requested.to_string()
    } else {
        format!("v{requested}")
    };

    releases
        .iter()
        .find(|release| release.tag == with_prefix)
        .ok_or_else(|| {
            let mut known: Vec<&str> = releases
                .iter()
                .map(|release| release.tag.as_str())
                .collect();
            known.truncate(5);
            format!(
                "no release is tagged {with_prefix}; the newest published tags are {}",
                if known.is_empty() {
                    "none".to_string()
                } else {
                    known.join(", ")
                }
            )
        })
}

/// Run `lisa upgrade`.
pub(crate) fn run_upgrade(args: UpgradeArgs) -> Result<(), String> {
    let installed = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("this build's own version is unreadable: {error}"))?;

    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot find the running lisa: {error}"))?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let method = classify_install(&exe, home.as_deref());

    // Refuse before touching config or the network: on these boxes there is
    // nothing `upgrade` can do, and recording a channel it cannot honour would
    // only make the machine lie about itself.
    if matches!(method, InstallMethod::Homebrew | InstallMethod::Apt) {
        return Err(package_managed_refusal(&method, &exe));
    }

    let config_path = channel::config_path()?;
    let mut config = channel::load_from(&config_path)?;

    if let Some(requested) = &args.channel {
        let requested = Channel::parse(requested)?;
        if args.dry_run {
            println!(
                "Would put this machine on channel {requested} in {}.",
                config_path.display()
            );
            config.channel = Some(requested);
        } else {
            config = channel::set_channel_at(&config_path, requested)?;
            println!(
                "This machine is on channel {requested} — {}. Recorded in {}.",
                requested.rule(),
                config_path.display()
            );
        }
    }

    let selected = config.effective_channel();
    match (&args.tag, config.channel) {
        (Some(tag), _) => println!("Pinning to {tag}, ignoring the channel."),
        (None, Some(chosen)) => println!("Channel {chosen} — {}.", chosen.rule()),
        (None, None) => println!(
            "No channel is set in {}, so this machine is treated as {selected} — {}.",
            config_path.display(),
            selected.rule()
        ),
    }

    let releases = fetch_releases().map_err(|error| {
        format!(
            "{error}\nlisa {installed} at {} is unchanged.",
            exe.display()
        )
    })?;

    let target = match &args.tag {
        Some(tag) => find_tag(&releases, tag)?.clone(),
        None => {
            let resolved =
                channel::resolve(selected, &releases, channel::now_unix(), config.soak());
            match resolved.release() {
                Some(release) => release.clone(),
                None => {
                    println!(
                        "Channel {selected} is not moving this cycle: {}.\n\
                         lisa {installed} stays in place.",
                        resolved
                            .waiting_reason()
                            .unwrap_or("the channel resolves to no release")
                    );
                    return Ok(());
                }
            }
        }
    };

    if target.version == installed {
        println!("lisa {installed} is already {}. Nothing to do.", target.tag);
        return Ok(());
    }

    let direction = if target.version < installed {
        " (a move back to an older release)"
    } else {
        ""
    };
    println!(
        "Moving lisa {installed} → {} [{}]{direction}.",
        target.version, target.tag
    );
    if matches!(method, InstallMethod::Elsewhere) {
        println!(
            "Note: this lisa runs from {}, and the installer writes to ~/.local/bin. \
             Check which one your shell finds afterwards with: command -v lisa",
            exe.display()
        );
    }
    let _ = io::stdout().flush();

    if args.dry_run {
        println!("--dry-run: nothing was downloaded and nothing was installed.");
        return Ok(());
    }

    install(&target).map_err(|error| {
        format!(
            "{error}\nlisa {installed} at {} is still in place.",
            exe.display()
        )
    })?;

    println!(
        "lisa {} is installed. Open a new shell, then check it with: lisa --version",
        target.version
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homebrew_and_apt_paths_are_recognised_as_package_managed() {
        let home = PathBuf::from("/Users/someone");
        for brew in [
            "/opt/homebrew/Cellar/lisa/0.5.0-rc.2/bin/lisa",
            "/usr/local/Cellar/lisa/0.4.4/bin/lisa",
            "/home/linuxbrew/.linuxbrew/bin/lisa",
        ] {
            assert_eq!(
                classify_install(Path::new(brew), Some(&home)),
                InstallMethod::Homebrew,
                "{brew}"
            );
        }

        assert_eq!(
            classify_install(Path::new("/usr/bin/lisa"), Some(&home)),
            InstallMethod::Apt
        );
    }

    #[test]
    fn the_shell_installer_directory_is_the_channel_aware_one() {
        let home = PathBuf::from("/Users/someone");
        assert_eq!(
            classify_install(Path::new("/Users/someone/.local/bin/lisa"), Some(&home)),
            InstallMethod::ShellInstaller
        );
        assert_eq!(
            classify_install(
                Path::new("/Users/someone/src/lisa/target/debug/lisa"),
                Some(&home)
            ),
            InstallMethod::Elsewhere
        );
    }

    #[test]
    fn the_refusal_names_both_ways_forward() {
        let refusal = package_managed_refusal(
            &InstallMethod::Homebrew,
            Path::new("/opt/homebrew/Cellar/lisa/0.5.0-rc.2/bin/lisa"),
        );
        assert!(refusal.contains("brew upgrade lisa"), "{refusal}");
        assert!(refusal.contains("brew uninstall lisa"), "{refusal}");
        assert!(refusal.contains("lisa upgrade --channel"), "{refusal}");

        let refusal = package_managed_refusal(&InstallMethod::Apt, Path::new("/usr/bin/lisa"));
        assert!(
            refusal.contains("apt-get install --only-upgrade lisa"),
            "{refusal}"
        );
    }

    #[test]
    fn the_release_list_drops_drafts_and_tags_that_are_not_lisa_releases() {
        let body = r#"[
            {"tag_name": "v0.5.0-rc.2", "published_at": "2026-08-09T00:00:00Z", "draft": false},
            {"tag_name": "v0.5.0-rc.3", "published_at": "2026-08-14T00:00:00Z", "draft": true},
            {"tag_name": "nightly", "published_at": "2026-08-10T00:00:00Z", "draft": false},
            {"tag_name": "v0.4.4", "published_at": "2026-07-19T00:00:00Z", "draft": false},
            {"tag_name": "v0.4.3", "published_at": null, "draft": false}
        ]"#;

        let releases = parse_releases(body).unwrap();
        let tags: Vec<&str> = releases.iter().map(|r| r.tag.as_str()).collect();
        assert_eq!(tags, ["v0.5.0-rc.2", "v0.4.4"]);
        assert_eq!(
            releases[1].published_at,
            channel::parse_rfc3339_utc("2026-07-19T00:00:00Z").unwrap()
        );
    }

    #[test]
    fn an_unreadable_release_list_is_an_error_not_an_empty_list() {
        assert!(parse_releases("not json").is_err());
    }

    #[test]
    fn a_pin_accepts_the_tag_with_or_without_its_v() {
        let releases = vec![
            Release::from_tag("v0.4.4", 0).unwrap(),
            Release::from_tag("v0.5.0-rc.2", 1).unwrap(),
        ];
        assert_eq!(find_tag(&releases, "v0.4.4").unwrap().tag, "v0.4.4");
        assert_eq!(find_tag(&releases, "0.4.4").unwrap().tag, "v0.4.4");

        let error = find_tag(&releases, "v9.9.9").unwrap_err();
        assert!(error.contains("no release is tagged v9.9.9"), "{error}");
        assert!(error.contains("v0.4.4"), "{error}");
    }
}
