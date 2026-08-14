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
//! **What happens on a brew- or apt-managed box.** It delegates. The tap
//! carries a formula per channel and the apt archive a suite per channel
//! (`S-069-01`), so the installed package already answers "which line of Lisa
//! is this box on" — and the package manager already knows how to fetch, verify
//! and swap. Lisa keeps the parts no package manager has: reading the channel
//! off the package, refusing to move under a live Zellij session, and saying
//! what moved. See [`crate::install_channel`] for which source wins where.
//!
//! The download-and-swap path below is still the whole story on a curl-installed
//! or source-built box — and on Homebrew it is the only rollback there is, since
//! `brew switch` is gone.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use semver::Version;

use crate::channel::{self, Channel, Release};
use install_channel::{Derived, Plan, Privilege, Source};

/// Which channel the installed package names, and who moves it. Upgrade's own
/// business, so it lives inside this module rather than beside it.
pub(crate) mod install_channel;

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

/// Whether a package manager owns this Lisa, and therefore whether the move is
/// its job rather than Lisa's.
pub(crate) fn is_package_managed(method: &InstallMethod) -> bool {
    matches!(method, InstallMethod::Homebrew | InstallMethod::Apt)
}

/// The refusal a package-managed Lisa gets when its own package cannot be read.
///
/// Not a refusal to move — that is delegated now — but a refusal to *guess*.
/// A box whose channel cannot be derived is exactly the box that must not fall
/// back to a config field nobody can see being used.
fn unreadable_package_refusal(manager: &str, reason: &str, exe: &Path) -> String {
    format!(
        "this lisa is managed by {manager} ({}), so {manager} is what says which channel it is \
         on — and that could not be read: {reason}.\n\
         Nothing was moved. Either put the install back in a shape {manager} recognises, or \
         reinstall on the channel you meant:\n  \
         - Homebrew: brew install {tap}/lisa-nightly   (or lisa-canary, or lisa)\n  \
         - apt: put the channel in the suite word of /etc/apt/sources.list.d/lisa.list",
        exe.display(),
        tap = install_channel::TAP,
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

/// Ask a lisa on this machine which version it is.
///
/// Asking beats assuming everywhere it is used: after a package manager has
/// moved the box, the version on disk is the only honest source of what moved,
/// and the process asking is still the old binary.
pub(crate) fn version_of(lisa: &Path) -> Option<Version> {
    Command::new(lisa)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            String::from_utf8_lossy(&output.stdout)
                .split_whitespace()
                .nth(1)
                .and_then(|version| Version::parse(version).ok())
        })
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
    let version = version_of(&installed);

    Ok(match version {
        Some(version) => Installed {
            version,
            path: installed,
            is_running: false,
        },
        None => mine,
    })
}

/// Put an apt box's sources line on another suite.
///
/// The file is rewritten whole from a value Lisa computed, rather than edited
/// in place by a regular expression: the file being changed is the one that
/// decides what this machine installs, and a half-applied edit to it is not a
/// state worth risking. Everything that is not Lisa's own line survives byte for
/// byte.
fn put_apt_on(file: &Path, to: Channel, privilege: Privilege) -> Result<(), String> {
    let contents = std::fs::read_to_string(file)
        .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    let rewritten = install_channel::rewrite_suite(&contents, to)
        .map_err(|error| format!("{}: {error}", file.display()))?;
    if rewritten == contents {
        return Ok(());
    }

    match privilege {
        Privilege::Root => std::fs::write(file, rewritten)
            .map_err(|error| format!("cannot write {}: {error}", file.display())),
        Privilege::Sudo => {
            use std::process::Stdio;
            let mut tee = Command::new("sudo")
                .arg("tee")
                .arg(file)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()
                .map_err(|error| format!("cannot run sudo tee {}: {error}", file.display()))?;
            tee.stdin
                .as_mut()
                .ok_or_else(|| "cannot write to sudo tee".to_string())?
                .write_all(rewritten.as_bytes())
                .map_err(|error| format!("cannot write {}: {error}", file.display()))?;
            let status = tee
                .wait()
                .map_err(|error| format!("cannot write {}: {error}", file.display()))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "sudo tee {} exited {status}, so the channel was not changed",
                    file.display()
                ))
            }
        }
        Privilege::Neither => Err(format!(
            "{} is root-owned, this process is not root, and there is no sudo on this machine",
            file.display()
        )),
    }
}

/// What the delegated `upgrade` is about to hand the package manager.
#[derive(Debug)]
struct Delegation {
    plan: Plan,
    /// The suite file to put on another channel first, when this is a switch.
    sources: Option<(PathBuf, Channel)>,
}

/// Work out what moves a package-managed box, given what was asked for.
fn plan_for(
    method: &InstallMethod,
    derived: &Derived,
    requested: Option<Channel>,
    privilege: Privilege,
) -> Result<Delegation, String> {
    let on = derived.channel;
    match (&derived.source, requested) {
        (Source::Formula(formula), Some(channel)) if Some(channel) != on => Ok(Delegation {
            plan: install_channel::brew_switch(formula, channel),
            sources: None,
        }),
        (Source::Formula(formula), _) => Ok(Delegation {
            plan: install_channel::brew_upgrade(formula),
            sources: None,
        }),
        (Source::Suite { file, .. }, Some(channel)) if Some(channel) != on => Ok(Delegation {
            plan: install_channel::apt_switch(privilege, file, channel),
            sources: Some((file.clone(), channel)),
        }),
        (Source::Suite { .. }, _) => Ok(Delegation {
            plan: install_channel::apt_upgrade(privilege),
            sources: None,
        }),
        (Source::Unreadable { manager, reason }, _) => Err(unreadable_package_refusal(
            manager,
            reason,
            Path::new("this lisa"),
        )),
        (Source::Config, _) => Err(format!(
            "{method:?} is not a package-managed install, so there is nothing to delegate to"
        )),
    }
}

/// Run `lisa upgrade` on a box a package manager owns.
///
/// The shape is the one `upgrade` already had — say where the channel comes
/// from, say what is about to happen, refuse under a live run, then report what
/// moved — with `brew` or `apt-get` doing the fetch, verify and swap in the
/// middle.
fn run_package_upgrade(
    args: &UpgradeArgs,
    method: InstallMethod,
    exe: &Path,
    derived: Derived,
) -> Result<(), String> {
    let lisa =
        install_channel::package_lisa_path(&method, exe).unwrap_or_else(|| exe.to_path_buf());
    let installed = version_of(&lisa)
        .or_else(|| Version::parse(env!("CARGO_PKG_VERSION")).ok())
        .ok_or_else(|| "this build's own version is unreadable".to_string())?;

    if let Source::Unreadable { manager, reason } = &derived.source {
        return Err(unreadable_package_refusal(manager, reason, exe));
    }

    match derived.channel {
        Some(channel) => println!(
            "Channel {channel} — {}, from {}.",
            channel.rule(),
            derived.source.describe()
        ),
        None => println!(
            "This machine is on {}, which is not one of Lisa's three channels.",
            derived.source.describe()
        ),
    }

    let privilege = Privilege::look();

    // A pin is the rollback, and rollback is the one thing the two package
    // managers do not agree on: apt keeps every version it has carried, and
    // Homebrew keeps none.
    if let Some(tag) = &args.tag {
        return pin_package_box(args, &method, exe, &lisa, &installed, tag, privilege);
    }

    let requested = match &args.channel {
        Some(name) => Some(Channel::parse(name)?),
        None => None,
    };
    if let (Some(requested), Some(on)) = (requested, derived.channel) {
        if requested == on {
            println!("This machine is already on {requested}; nothing to switch.");
        }
    }
    if requested.is_some() {
        println!(
            "The channel is the package, so nothing was written to Lisa's config file — \
             the install is what says which channel this box is on."
        );
    }

    let delegation = plan_for(&method, &derived, requested, privilege)?;
    println!("{}", delegation.plan.describe());
    let _ = io::stdout().flush();

    // Last thing before anything moves: a run on this machine is calling the
    // binary the package manager is about to replace. Delegating the swap does
    // not delegate this — a `brew upgrade` under a live Zellij breaks the run
    // exactly as Lisa's own installer would.
    let busy = crate::busy::look();

    if args.dry_run {
        if busy.is_busy() {
            println!(
                "This machine is working — {} — so a real run would stop here rather than let \
                 a package manager swap the binary underneath it.",
                busy.describe()
            );
        }
        println!("--dry-run: nothing was run and nothing was installed.");
        return Ok(());
    }

    if busy.is_busy() && !args.anyway {
        return Err(format!(
            "{}\nlisa {installed} at {} is unchanged.",
            live_run_refusal(&busy),
            lisa.display()
        ));
    }
    if busy.is_busy() {
        println!("--anyway: moving with {}.", busy.describe());
    }

    if !privilege.can_run() {
        return Err(format!(
            "this box needs root to move an apt package and this process has no way to become \
             it. Nothing was changed; run this yourself:\n{}",
            delegation.plan.describe()
        ));
    }

    if let Some((file, channel)) = &delegation.sources {
        put_apt_on(file, *channel, privilege)?;
        println!("{} now says {channel}.", file.display());
    }

    for step in &delegation.plan.steps {
        step.run().map_err(|error| {
            format!(
                "{error}\nlisa {installed} at {} is what this machine has.{}",
                lisa.display(),
                by_hand_suffix(&delegation.plan)
            )
        })?;
    }

    report_move(&lisa, &installed);
    Ok(())
}

/// Whatever the plan said Lisa would not do for you, appended to an error so the
/// operator standing at a failure has it in front of them.
fn by_hand_suffix(plan: &Plan) -> String {
    if plan.by_hand.is_empty() {
        String::new()
    } else {
        format!("\n{}", plan.by_hand.join("\n"))
    }
}

/// Say what the package manager actually did, in the before-and-after shape
/// `upgrade` already uses.
fn report_move(lisa: &Path, before: &Version) {
    match version_of(lisa) {
        Some(after) if &after == before => {
            println!("lisa {before} is what this channel carries. Nothing moved.");
        }
        Some(after) => {
            println!(
                "lisa {before} → {after} at {}. Open a new shell, then check it with: \
                 lisa --version",
                lisa.display()
            );
        }
        None => println!(
            "The mover finished, and {} did not answer --version. Check it with: \
             command -v lisa && lisa --version",
            lisa.display()
        ),
    }
}

/// `lisa upgrade --tag` on a package-managed box.
///
/// **On apt this is a real rollback.** The pool keeps every version any suite
/// has carried, so `apt-get install lisa=<version>` fetches it back.
///
/// **On Homebrew there is no such thing.** `brew switch` was removed and a
/// formula carries one version, so the escape hatch is Lisa's own installer
/// writing into `~/.local/bin` — which leaves two lisas on the box and PATH
/// order deciding which one runs. That is a real state, so it is said out loud
/// here and reported by `doctor` afterwards rather than discovered later.
fn pin_package_box(
    args: &UpgradeArgs,
    method: &InstallMethod,
    exe: &Path,
    lisa: &Path,
    installed: &Version,
    tag: &str,
    privilege: Privilege,
) -> Result<(), String> {
    let busy = crate::busy::look();
    let guard = |dry_run: bool| -> Result<(), String> {
        if dry_run || !busy.is_busy() || args.anyway {
            Ok(())
        } else {
            Err(format!(
                "{}\nlisa {installed} at {} is unchanged.",
                live_run_refusal(&busy),
                lisa.display()
            ))
        }
    };

    match method {
        InstallMethod::Apt => {
            let version = Version::parse(tag.strip_prefix('v').unwrap_or(tag))
                .map_err(|error| format!("{tag} is not a release version: {error}"))?;
            let plan = install_channel::apt_pin(privilege, &version);
            println!("Pinning to {tag}, ignoring the channel.");
            println!("{}", plan.describe());
            if args.dry_run {
                println!("--dry-run: nothing was run and nothing was installed.");
                return Ok(());
            }
            guard(false)?;
            if !privilege.can_run() {
                return Err(format!(
                    "this box needs root to move an apt package and this process has no way to \
                     become it. Nothing was changed; run this yourself:\n{}",
                    plan.describe()
                ));
            }
            for step in &plan.steps {
                step.run().map_err(|error| {
                    format!(
                        "{error}\nlisa {installed} at {} is what this machine has.{}",
                        lisa.display(),
                        by_hand_suffix(&plan)
                    )
                })?;
            }
            report_move(lisa, installed);
            Ok(())
        }
        InstallMethod::Homebrew => {
            println!(
                "Pinning to {tag}, ignoring the channel. Homebrew cannot do this — `brew switch` \
                 is gone and a formula carries one version — so Lisa's own installer writes \
                 {tag} into ~/.local/bin instead."
            );
            println!(
                "That leaves two lisas on this machine: the Homebrew one at {} and the pinned \
                 one. PATH order decides which runs, and `lisa doctor` reports the pair until \
                 you take one away.",
                exe.display()
            );
            let releases = fetch_releases()
                .map_err(|error| format!("{error}\nlisa {installed} is unchanged."))?;
            let target = find_tag(&releases, tag)?.clone();
            if args.dry_run {
                println!("--dry-run: nothing was downloaded and nothing was installed.");
                return Ok(());
            }
            guard(false)?;
            install(&target).map_err(|error| {
                format!(
                    "{error}\nlisa {installed} at {} is still in place.",
                    lisa.display()
                )
            })?;
            println!(
                "lisa {} is installed in ~/.local/bin. Check which one your shell finds with: \
                 command -v lisa",
                target.version
            );
            println!(
                "To go back to the Homebrew one: rm ~/.local/bin/lisa   (or `brew upgrade {}`)",
                install_channel::formula_from_exe(exe).unwrap_or_else(|| "lisa".to_string())
            );
            Ok(())
        }
        InstallMethod::ShellInstaller | InstallMethod::Elsewhere => {
            unreachable!("only package-managed installs are pinned here")
        }
    }
}

/// Run `lisa upgrade`.
pub(crate) fn run_upgrade(args: UpgradeArgs) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot find the running lisa: {error}"))?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let method = classify_install(&exe, home.as_deref());

    // A package-managed box does not read a channel out of a config file and
    // does not need Lisa to fetch anything: the package it has is the channel,
    // and the package manager is the mover.
    if is_package_managed(&method) {
        let derived = install_channel::derive(&method, &exe);
        return run_package_upgrade(&args, method, &exe, derived);
    }

    let target_of_the_move = installed_lisa(&exe, home.as_deref())?;
    let installed = target_of_the_move.version.clone();

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

    /// A Homebrew box on `lisa-nightly` asked to upgrade hands the whole move
    /// to `brew`, which is the change this ticket is: the old code printed
    /// `brew upgrade lisa` and refused.
    #[test]
    fn a_brew_box_delegates_the_move_to_the_formula_it_is_on() {
        let derived = install_channel::derive(
            &InstallMethod::Homebrew,
            Path::new("/opt/homebrew/Cellar/lisa-nightly/0.5.0-rc.2/bin/lisa"),
        );
        assert_eq!(derived.channel, Some(Channel::Nightly));

        let delegation =
            plan_for(&InstallMethod::Homebrew, &derived, None, Privilege::Root).unwrap();
        let block = delegation.plan.describe();
        assert!(block.contains("brew upgrade lisa-nightly"), "{block}");
        assert!(delegation.sources.is_none());
    }

    #[test]
    fn asking_a_brew_box_for_another_channel_switches_packages() {
        let derived = install_channel::derive(
            &InstallMethod::Homebrew,
            Path::new("/opt/homebrew/Cellar/lisa/0.4.4/bin/lisa"),
        );
        let block = plan_for(
            &InstallMethod::Homebrew,
            &derived,
            Some(Channel::Canary),
            Privilege::Root,
        )
        .unwrap()
        .plan
        .describe();

        assert!(block.contains("brew uninstall lisa"), "{block}");
        assert!(
            block.contains("brew install johnhkchen/lisa/lisa-canary"),
            "{block}"
        );
    }

    /// The channel an apt box is on is the suite word, and switching it is the
    /// sources-line change before the install rather than a config edit.
    #[test]
    fn asking_an_apt_box_for_another_channel_rewrites_the_sources_line_first() {
        let derived = Derived {
            channel: Some(Channel::Stable),
            source: Source::Suite {
                suite: "stable".to_string(),
                file: PathBuf::from("/etc/apt/sources.list.d/lisa.list"),
            },
        };
        let delegation = plan_for(
            &InstallMethod::Apt,
            &derived,
            Some(Channel::Nightly),
            Privilege::Root,
        )
        .unwrap();

        assert_eq!(
            delegation.sources,
            Some((
                PathBuf::from("/etc/apt/sources.list.d/lisa.list"),
                Channel::Nightly
            ))
        );
        let block = delegation.plan.describe();
        assert!(block.contains("apt-get update"), "{block}");
        assert!(
            block.contains("apt-get install --only-upgrade -y lisa lisa-runtime-zellij"),
            "{block}"
        );
    }

    /// A package-managed box whose package cannot be read must not quietly fall
    /// back to the config file — that is the second source of truth this ticket
    /// exists to remove.
    #[test]
    fn a_package_that_cannot_be_read_is_a_refusal_that_names_the_fix() {
        let derived = install_channel::derive(
            &InstallMethod::Homebrew,
            Path::new("/home/linuxbrew/bin/lisa"),
        );
        let error =
            plan_for(&InstallMethod::Homebrew, &derived, None, Privilege::Root).unwrap_err();
        assert!(error.contains("Homebrew"), "{error}");
        assert!(error.contains("Nothing was moved"), "{error}");
        assert!(error.contains("brew install johnhkchen/lisa/"), "{error}");
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
