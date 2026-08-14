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
    /// Move even though a run is live on this machine.
    pub(crate) anyway: bool,
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

/// The refusal an upgrade gets while a run is live on this machine.
///
/// Naming both ways out, because "wait" is usually right and "move anyway" is
/// sometimes right, and the operator standing here is the one who knows which.
pub(crate) fn live_run_refusal(busy: &crate::busy::Busy) -> String {
    format!(
        "not moving lisa while this machine is working: {}.\n\
         An upgrade swaps the binary a running loop is calling, which breaks the run \
         it lands in — being one release behind is the cheaper mistake. Either:\n  \
         - wait for the run to finish and run this again, or\n  \
         - move now and accept the risk:\n      lisa upgrade --anyway",
        busy.describe(),
    )
}

/// Download a release's shell installer and run it.
///
/// The installer writes a new binary into `~/.local/bin`; nothing here removes
/// or truncates the running one, so a failed download or a failed installer
/// leaves the working Lisa in place.
pub(crate) fn install(release: &Release) -> Result<(), String> {
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

/// The lisa an upgrade is about: the one the shell installer maintains.
///
/// Usually that is the process asking, and then this is just its own version.
/// It is not the same thing on a box where the running `lisa` came from
/// somewhere else — a `cargo build`, another prefix — and there the difference
/// matters most in the case you least want to get wrong: `--tag` is the
/// rollback, and a rollback that reads the *runner's* version decides the
/// machine is already where it needs to be and moves nothing.
#[derive(Debug, Clone)]
pub(crate) struct Installed {
    /// The version this machine has on the channel-aware path.
    pub(crate) version: Version,
    /// The file that version lives in.
    pub(crate) path: PathBuf,
    /// Whether that file is the one running right now.
    pub(crate) is_running: bool,
}

/// The path the shell installer writes, whether or not anything is there.
pub(crate) fn installer_owned_path(home: &Path) -> PathBuf {
    home.join(".local").join("bin").join("lisa")
}

/// Work out which lisa this machine has, and how old it is.
pub(crate) fn installed_lisa(exe: &Path, home: Option<&Path>) -> Result<Installed, String> {
    let own = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("this build's own version is unreadable: {error}"))?;

    let mine = Installed {
        version: own.clone(),
        path: exe.to_path_buf(),
        is_running: true,
    };

    let Some(home) = home else { return Ok(mine) };
    let installed = installer_owned_path(home);
    if !installed.exists()
        || installed
            .canonicalize()
            .is_ok_and(|resolved| resolved == exe)
    {
        return Ok(mine);
    }

    // Ask it rather than assume: an installed lisa is the one an upgrade
    // replaces, and its version is a fact about the machine, not about this
    // build.
    let reported = Command::new(&installed).arg("--version").output();
    let version = reported
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            String::from_utf8_lossy(&output.stdout)
                .split_whitespace()
                .nth(1)
                .and_then(|version| Version::parse(version).ok())
        });

    Ok(match version {
        Some(version) => Installed {
            version,
            path: installed,
            is_running: false,
        },
        None => mine,
    })
}

/// Run `lisa upgrade`.
pub(crate) fn run_upgrade(args: UpgradeArgs) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot find the running lisa: {error}"))?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let method = classify_install(&exe, home.as_deref());
    let target_of_the_move = installed_lisa(&exe, home.as_deref())?;
    let installed = target_of_the_move.version.clone();

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
            target_of_the_move.path.display()
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
        if target_of_the_move.is_running {
            println!(
                "Note: this lisa runs from {}, and the installer writes to ~/.local/bin. \
                 Check which one your shell finds afterwards with: command -v lisa",
                exe.display()
            );
        } else {
            println!(
                "Note: this command is running from {}, and the lisa it moves is the \
                 installed one at {} ({installed}). Check which one your shell finds \
                 afterwards with: command -v lisa",
                exe.display(),
                target_of_the_move.path.display()
            );
        }
    }
    let _ = io::stdout().flush();

    // Last thing before anything is downloaded: a run on this machine is
    // calling the binary this is about to replace.
    let busy = crate::busy::look();

    if args.dry_run {
        if busy.is_busy() {
            println!(
                "This machine is working — {} — so a real run would stop here \
                 rather than swap the binary underneath it.",
                busy.describe()
            );
        }
        println!("--dry-run: nothing was downloaded and nothing was installed.");
        return Ok(());
    }

    if busy.is_busy() && !args.anyway {
        return Err(format!(
            "{}\nlisa {installed} at {} is unchanged.",
            live_run_refusal(&busy),
            target_of_the_move.path.display()
        ));
    }
    if busy.is_busy() {
        println!("--anyway: moving with {}.", busy.describe());
    }

    install(&target).map_err(|error| {
        format!(
            "{error}\nlisa {installed} at {} is still in place.",
            target_of_the_move.path.display()
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
