//! Which channel the package that installed this Lisa names, and who moves it.
//!
//! `S-069-01` turned the channel into a package: `brew install lisa-nightly`,
//! or one word in an apt sources line. That makes the channel a fact about the
//! box rather than a setting on it, and this module is where that fact is read.
//!
//! ## Which source wins where
//!
//! | how lisa got here | the channel is | the config field |
//! | --- | --- | --- |
//! | a Homebrew formula | the formula name — `lisa`, `lisa-nightly`, `lisa-canary` | not read |
//! | an apt suite | the suite word in Lisa's sources line | not read |
//! | the shell installer, or a build | the `channel` field in the machine config | the only answer |
//!
//! The rule is one sentence: **a package-managed box reads its channel off the
//! package, and every other box reads it out of the config file.** A machine
//! that could answer two ways is the bug this exists to prevent, so on a
//! package-managed box a `channel` line in the config is inert — `doctor` says
//! so rather than letting it look load-bearing.
//!
//! ## Who moves it
//!
//! The package manager does. Lisa keeps the parts no package manager has — the
//! schedule, the refusal to move under a live Zellij, the alarm, the drift row
//! — and hands fetch, verify and swap to `brew` or `apt-get`. The commands are
//! built here, as argv rather than shell strings, so what Lisa prints and what
//! Lisa runs cannot drift apart.

use std::path::{Path, PathBuf};
use std::process::Command;

use semver::Version;

use crate::channel::Channel;

use super::InstallMethod;

/// The tap the three formulae live in. Named in full when installing, because
/// `brew install lisa-nightly` without it finds nothing.
pub(crate) const TAP: &str = "johnhkchen/lisa";

/// The apt archive's URI, which is what tells Lisa's sources line apart from
/// every other line in `/etc/apt`.
pub(crate) const APT_ARCHIVE: &str = "johnhkchen.github.io/lisa";

/// The two packages an apt box carries: the CLI and the Zellij it was built
/// against. They move together — a canary `lisa` with a stable runtime is a
/// pairing apt would otherwise make quietly.
pub(crate) const APT_PACKAGES: [&str; 2] = ["lisa", "lisa-runtime-zellij"];

/// Where apt keeps its sources. Overridable so a test can hand Lisa a sources
/// tree instead of the machine's own.
const APT_ROOT_ENV: &str = "LISA_APT_SOURCES_DIR";

/// The default apt configuration root.
const APT_ROOT: &str = "/etc/apt";

/// The file an apt-installed Lisa lives in.
const APT_LISA: &str = "/usr/bin/lisa";

/// Where that file is, overridable so a test can hand Lisa a box instead of
/// looking at `/usr/bin`, which cannot be faked in a temporary directory.
const APT_LISA_ENV: &str = "LISA_APT_LISA";

/// The formula a channel is published as.
pub(crate) const fn formula_for(channel: Channel) -> &'static str {
    match channel {
        Channel::Stable => "lisa",
        Channel::Nightly => "lisa-nightly",
        Channel::Canary => "lisa-canary",
    }
}

/// The channel a formula name means, or `None` for a formula that is not one of
/// Lisa's three.
pub(crate) fn channel_of_formula(formula: &str) -> Option<Channel> {
    Channel::ALL
        .into_iter()
        .find(|channel| formula_for(*channel) == formula)
}

/// The suite a channel is published as. The suite word *is* the channel name,
/// which is the whole reason the apt half needs no table.
pub(crate) fn suite_for(channel: Channel) -> &'static str {
    channel.as_str()
}

/// Where a derived channel came from, in the words a report prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Source {
    /// The Homebrew formula that owns the running binary.
    Formula(String),
    /// The suite word in Lisa's own apt sources line, and the file it is in.
    Suite { suite: String, file: PathBuf },
    /// No package manager owns this Lisa, so the machine config is the answer.
    Config,
    /// A package manager owns this Lisa and could not be asked, which is a
    /// state an operator has to be told rather than have guessed around.
    Unreadable {
        manager: &'static str,
        reason: String,
    },
}

impl Source {
    /// The phrase a row uses: "from {this}".
    pub(crate) fn describe(&self) -> String {
        match self {
            Source::Formula(formula) => format!("the Homebrew formula {formula}"),
            Source::Suite { suite, file } => {
                format!("the apt suite {suite} in {}", file.display())
            }
            Source::Config => "the machine config".to_string(),
            Source::Unreadable { manager, reason } => {
                format!("{manager}, which could not be asked: {reason}")
            }
        }
    }

    /// The one word a script sorts on.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Source::Formula(_) => "homebrew-formula",
            Source::Suite { .. } => "apt-suite",
            Source::Config => "config",
            Source::Unreadable { .. } => "package-unreadable",
        }
    }
}

/// What the installed package says about this machine's channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Derived {
    /// The channel the package names, or `None` when the config is the answer
    /// (or when the package could not be read).
    pub(crate) channel: Option<Channel>,
    /// Where that answer came from.
    pub(crate) source: Source,
}

impl Derived {
    /// The machine config governs this box.
    fn from_config() -> Self {
        Self {
            channel: None,
            source: Source::Config,
        }
    }
}

/// The Homebrew formula that owns an executable, read off its path.
///
/// Homebrew's own layout is the record: a formula's files live under
/// `<prefix>/Cellar/<formula>/<version>/`, and the linked copy on `PATH` is a
/// symlink into it. `upgrade` canonicalises before classifying, so the Cellar
/// path is what arrives here. The `opt` form is accepted too, for a caller that
/// did not resolve symlinks.
pub(crate) fn formula_from_exe(exe: &Path) -> Option<String> {
    let mut parts = exe.components().peekable();
    while let Some(part) = parts.next() {
        let part = part.as_os_str().to_string_lossy();
        if part == "Cellar" || part == "opt" {
            let formula = parts.peek()?.as_os_str().to_string_lossy().to_string();
            if formula != "lisa" && !formula.starts_with("lisa-") {
                continue;
            }
            return Some(formula);
        }
    }
    None
}

/// The apt configuration root this machine reads.
pub(crate) fn apt_root() -> PathBuf {
    std::env::var_os(APT_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(APT_ROOT))
}

/// Lisa's own sources lines, wherever apt keeps them.
///
/// Both grammars apt reads are accepted: the one-line `deb <uri> <suite>
/// <components>` form the README publishes, and the deb822 `.sources` form with
/// `URIs:` and `Suites:` fields. A line that does not point at Lisa's archive is
/// not Lisa's business and is skipped.
pub(crate) fn suite_from_apt(root: &Path) -> Result<(String, PathBuf), String> {
    let mut files = vec![root.join("sources.list")];
    if let Ok(entries) = std::fs::read_dir(root.join("sources.list.d")) {
        let mut listed: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("list") | Some("sources")
                )
            })
            .collect();
        listed.sort();
        files.extend(listed);
    }

    let mut found: Vec<(String, PathBuf)> = Vec::new();
    for file in files {
        let Ok(contents) = std::fs::read_to_string(&file) else {
            continue;
        };
        for suite in suites_in(&contents) {
            found.push((suite, file.clone()));
        }
    }

    match found.len() {
        0 => Err(format!(
            "no sources line in {} points at {APT_ARCHIVE}, so apt is not carrying this lisa \
             from Lisa's own archive",
            root.display()
        )),
        1 => Ok(found.remove(0)),
        _ => {
            let distinct: Vec<&str> = {
                let mut names: Vec<&str> = found.iter().map(|(suite, _)| suite.as_str()).collect();
                names.sort_unstable();
                names.dedup();
                names
            };
            if distinct.len() == 1 {
                Ok(found.remove(0))
            } else {
                Err(format!(
                    "apt has this machine on {} at once ({}); one channel per box, so remove \
                     the sources lines you did not mean",
                    distinct.join(" and "),
                    found
                        .iter()
                        .map(|(_, file)| file.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            }
        }
    }
}

/// Every Lisa suite one sources file names.
fn suites_in(contents: &str) -> Vec<String> {
    let mut found = Vec::new();

    // The one-line form: `deb [options] <uri> <suite> <components...>`.
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if !line.starts_with("deb ") && !line.starts_with("deb-src ") {
            continue;
        }
        let mut words = Vec::new();
        let mut rest = line;
        // `[arch=… signed-by=…]` is one field however many spaces are inside it.
        while let Some(open) = rest.find('[') {
            let Some(close) = rest[open..].find(']') else {
                break;
            };
            words.extend(rest[..open].split_whitespace().map(str::to_string));
            rest = &rest[open + close + 1..];
        }
        words.extend(rest.split_whitespace().map(str::to_string));
        let Some(uri_at) = words.iter().position(|word| word.contains(APT_ARCHIVE)) else {
            continue;
        };
        if let Some(suite) = words.get(uri_at + 1) {
            found.push(suite.clone());
        }
    }

    // The deb822 form: stanzas of `Field: value`, blank-line separated.
    for stanza in contents.split("\n\n") {
        let mut uris = String::new();
        let mut suites = String::new();
        for line in stanza.lines() {
            let Some((field, value)) = line.split_once(':') else {
                continue;
            };
            match field.trim().to_ascii_lowercase().as_str() {
                "uris" => uris = value.trim().to_string(),
                "suites" => suites = value.trim().to_string(),
                _ => {}
            }
        }
        if uris.contains(APT_ARCHIVE) {
            found.extend(suites.split_whitespace().map(str::to_string));
        }
    }

    found
}

/// Rewrite Lisa's sources lines onto another suite, leaving every other line in
/// the file exactly as it was.
///
/// The whole file is rewritten rather than edited in place with `sed`, because
/// the thing being changed is the file that decides what this machine installs
/// and a half-applied regular expression is not a state worth risking.
pub(crate) fn rewrite_suite(contents: &str, to: Channel) -> Result<String, String> {
    let suite = suite_for(to);
    let mut rewritten = String::with_capacity(contents.len());
    let mut changed = false;

    for line in contents.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let ending = &line[body.len()..];
        let bare = body.split('#').next().unwrap_or("").trim();

        let replaced = if (bare.starts_with("deb ") || bare.starts_with("deb-src "))
            && bare.contains(APT_ARCHIVE)
        {
            replace_suite_after_uri(body, suite)
        } else if bare.to_ascii_lowercase().starts_with("suites:") {
            // deb822: the stanza's URI line is checked by the caller, which only
            // hands this function a file that names Lisa's archive.
            Some(format!("Suites: {suite}"))
        } else {
            None
        };

        match replaced {
            Some(new) => {
                changed = true;
                rewritten.push_str(&new);
                rewritten.push_str(ending);
            }
            None => rewritten.push_str(line),
        }
    }

    if changed {
        Ok(rewritten)
    } else {
        Err(format!(
            "no line in this sources file points at {APT_ARCHIVE}, so there is no suite \
             word to change"
        ))
    }
}

/// Swap the word that follows Lisa's archive URI, keeping the rest of the line
/// — options, components, spacing — as it was written.
fn replace_suite_after_uri(line: &str, suite: &str) -> Option<String> {
    let uri_at = line.find(APT_ARCHIVE)?;
    let after_uri = line[uri_at..].find(char::is_whitespace)? + uri_at;
    let suite_start = after_uri + line[after_uri..].find(|c: char| !c.is_whitespace())?;
    let suite_end = suite_start
        + line[suite_start..]
            .find(char::is_whitespace)
            .unwrap_or(line.len() - suite_start);
    Some(format!(
        "{}{suite}{}",
        &line[..suite_start],
        &line[suite_end..]
    ))
}

/// Ask the install which channel this machine is on.
pub(crate) fn derive(method: &InstallMethod, exe: &Path) -> Derived {
    match method {
        InstallMethod::Homebrew => match formula_from_exe(exe) {
            Some(formula) => Derived {
                channel: channel_of_formula(&formula),
                source: Source::Formula(formula),
            },
            None => Derived {
                channel: None,
                source: Source::Unreadable {
                    manager: "Homebrew",
                    reason: format!(
                        "{} is not under a Cellar, so the formula that owns it cannot be read \
                         off its path",
                        exe.display()
                    ),
                },
            },
        },
        InstallMethod::Apt => match suite_from_apt(&apt_root()) {
            Ok((suite, file)) => Derived {
                channel: Channel::parse(&suite).ok(),
                source: Source::Suite { suite, file },
            },
            Err(reason) => Derived {
                channel: None,
                source: Source::Unreadable {
                    manager: "apt",
                    reason,
                },
            },
        },
        InstallMethod::ShellInstaller | InstallMethod::Elsewhere => Derived::from_config(),
    }
}

/// One command Lisa hands to a package manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mover {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
}

impl Mover {
    fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
        }
    }

    /// The same command as a person would type it, which is what Lisa prints
    /// before it runs it and what it prints instead when it will not.
    pub(crate) fn line(&self) -> String {
        std::iter::once(self.program.clone())
            .chain(self.args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Run it, letting its output through to the operator's terminal.
    pub(crate) fn run(&self) -> Result<(), String> {
        let status = Command::new(&self.program)
            .args(&self.args)
            .status()
            .map_err(|error| format!("cannot run `{}`: {error}", self.line()))?;
        if status.success() {
            return Ok(());
        }
        Err(format!(
            "`{}` exited {}",
            self.line(),
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "on a signal".to_string()),
        ))
    }
}

/// What Lisa will do, and what it will not do for you.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    /// One line naming who is moving this machine.
    pub(crate) headline: String,
    /// The commands Lisa runs, in order.
    pub(crate) steps: Vec<Mover>,
    /// What Lisa will not do on its own, for the operator to run. A half-done
    /// channel switch is worse than a printed command.
    pub(crate) by_hand: Vec<String>,
}

impl Plan {
    /// The plan as a block a person reads, indented like every other command
    /// Lisa prints.
    pub(crate) fn describe(&self) -> String {
        let mut block = self.headline.clone();
        for step in &self.steps {
            block.push_str(&format!("\n    {}", step.line()));
        }
        for line in &self.by_hand {
            block.push_str(&format!("\n{line}"));
        }
        block
    }
}

/// Whether this process can write the files a package manager owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Privilege {
    /// Running as root: apt commands go straight in.
    Root,
    /// Not root, and `sudo` is here: apt commands are prefixed with it.
    Sudo,
    /// Not root and no `sudo`: Lisa prints the commands rather than failing at
    /// a permission error halfway through.
    Neither,
}

impl Privilege {
    /// What this machine can do right now.
    pub(crate) fn look() -> Self {
        if running_as_root() {
            Privilege::Root
        } else if crate::doctor::which("sudo") {
            Privilege::Sudo
        } else {
            Privilege::Neither
        }
    }

    /// The same command, with whatever it takes to run it as root.
    fn elevate(self, mover: Mover) -> Mover {
        match self {
            Privilege::Root => mover,
            Privilege::Sudo | Privilege::Neither => {
                let mut args = vec![mover.program];
                args.extend(mover.args);
                Mover {
                    program: "sudo".to_string(),
                    args,
                }
            }
        }
    }
}

/// Whether this process is root, asked of the system rather than assumed from
/// the environment, which anything can set.
fn running_as_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
        .unwrap_or(false)
}

/// Move a Homebrew box to whatever its formula carries now.
pub(crate) fn brew_upgrade(formula: &str) -> Plan {
    Plan {
        headline: format!("Homebrew moves this machine, and {formula} is the channel:"),
        steps: vec![
            Mover::new("brew", &["update"]),
            Mover::new("brew", &["upgrade", formula]),
        ],
        by_hand: Vec::new(),
    }
}

/// Move a Homebrew box onto another channel.
///
/// The three formulae conflict on purpose — they all install the same `lisa` —
/// so a switch is an uninstall and an install, and there is a moment in the
/// middle with no `lisa` on the box. The bottle is fetched first so that moment
/// is as short as a local copy, and the way back is printed before anything is
/// removed rather than after something fails.
pub(crate) fn brew_switch(from_formula: &str, to: Channel) -> Plan {
    let target = formula_for(to);
    Plan {
        headline: format!(
            "Homebrew carries one formula at a time, so moving to {to} is an uninstall and an \
             install:"
        ),
        steps: vec![
            Mover::new("brew", &["fetch", &format!("{TAP}/{target}")]),
            Mover::new("brew", &["uninstall", from_formula]),
            Mover::new("brew", &["install", &format!("{TAP}/{target}")]),
        ],
        by_hand: vec![format!(
            "  If the install fails, this machine has no lisa until you run either:\n      \
             brew install {TAP}/{target}\n      brew install {TAP}/{from_formula}"
        )],
    }
}

/// Move an apt box to whatever its suite carries now.
pub(crate) fn apt_upgrade(privilege: Privilege) -> Plan {
    let mut install = vec!["install", "--only-upgrade", "-y"];
    install.extend(APT_PACKAGES);
    Plan {
        headline: "apt moves this machine, and the suite in its sources line is the channel:"
            .to_string(),
        steps: vec![
            privilege.elevate(Mover::new("apt-get", &["update"])),
            privilege.elevate(Mover::new("apt-get", &install)),
        ],
        by_hand: privilege.printed_if_powerless(),
    }
}

/// Put an apt box on another channel: the suite word, an update, and an install.
///
/// Lisa rewrites the sources file itself when it can — that is the whole change
/// — and the install that follows only moves the box *up*. Coming back down a
/// channel asks apt for an older version than the one on the machine, which apt
/// refuses without being told twice, so that command is printed rather than run.
pub(crate) fn apt_switch(privilege: Privilege, file: &Path, to: Channel) -> Plan {
    let mut plan = apt_upgrade(privilege);
    plan.headline = format!(
        "Putting {} on {to}, then letting apt move the machine:",
        file.display()
    );
    plan.by_hand.push(format!(
        "  Going back down a channel asks for an older version than this box has, which apt \
         calls a downgrade and will not do on its own:\n      apt-cache madison lisa\n      \
         {}apt-get install --allow-downgrades lisa=<version> lisa-runtime-zellij=<version>",
        if matches!(privilege, Privilege::Root) {
            ""
        } else {
            "sudo "
        }
    ));
    plan
}

/// Put an apt box on one exact release. This is a real rollback: the pool keeps
/// every version any suite ever carried.
pub(crate) fn apt_pin(privilege: Privilege, version: &Version) -> Plan {
    let deb = deb_version(version);
    let pinned: Vec<String> = APT_PACKAGES
        .iter()
        .map(|package| format!("{package}={deb}"))
        .collect();
    let mut args = vec![
        "install".to_string(),
        "--allow-downgrades".to_string(),
        "-y".to_string(),
    ];
    args.extend(pinned);

    Plan {
        headline: format!("apt keeps every version it has carried, so {deb} is one command away:"),
        steps: vec![
            privilege.elevate(Mover::new("apt-get", &["update"])),
            privilege.elevate(Mover {
                program: "apt-get".to_string(),
                args,
            }),
        ],
        by_hand: {
            let mut by_hand = privilege.printed_if_powerless();
            by_hand.push(
                "  If apt says that version has no installation candidate, ask which ones it \
                 has:\n      apt-cache madison lisa"
                    .to_string(),
            );
            by_hand
        },
    }
}

impl Privilege {
    /// The note a box with no way to become root needs, so the printed commands
    /// are explained rather than mysterious.
    fn printed_if_powerless(self) -> Vec<String> {
        match self {
            Privilege::Neither => vec![
                "  This process is not root and there is no sudo on this machine, so nothing \
                 above was run. Run it as a user who can."
                    .to_string(),
            ],
            Privilege::Root | Privilege::Sudo => Vec::new(),
        }
    }

    /// Whether Lisa can actually run what it planned.
    pub(crate) fn can_run(self) -> bool {
        !matches!(self, Privilege::Neither)
    }
}

/// The Debian version string a Lisa release is packaged as.
///
/// `packaging/nfpm/lisa.yaml` declares `version_schema: semver` and `release:
/// "1"`, which makes a prerelease sort below its own release the way Debian
/// expects: `0.5.0-rc.2` is packaged `0.5.0~rc.2-1`.
pub(crate) fn deb_version(version: &Version) -> String {
    let mut deb = format!("{}.{}.{}", version.major, version.minor, version.patch);
    if !version.pre.is_empty() {
        deb.push('~');
        deb.push_str(version.pre.as_str());
    }
    deb.push_str("-1");
    deb
}

/// The file an apt box's `lisa` is, wherever this machine keeps it.
pub(crate) fn apt_lisa_path() -> PathBuf {
    std::env::var_os(APT_LISA_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(APT_LISA))
}

/// The file the package manager keeps its `lisa` in, which is what to ask for a
/// version once the mover has run. Homebrew relinks `<prefix>/bin/lisa` on
/// every upgrade, so the path outlives the Cellar directory the old version was
/// in.
pub(crate) fn package_lisa_path(method: &InstallMethod, exe: &Path) -> Option<PathBuf> {
    match method {
        InstallMethod::Apt => Some(apt_lisa_path()),
        InstallMethod::Homebrew => {
            let cellar = exe.to_string_lossy();
            let prefix = cellar.split("/Cellar/").next()?;
            if prefix.is_empty() || prefix == cellar {
                return None;
            }
            Some(PathBuf::from(prefix).join("bin").join("lisa"))
        }
        InstallMethod::ShellInstaller | InstallMethod::Elsewhere => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_formula_name_is_the_channel_and_the_channel_is_the_formula_name() {
        assert_eq!(formula_for(Channel::Stable), "lisa");
        assert_eq!(formula_for(Channel::Nightly), "lisa-nightly");
        assert_eq!(formula_for(Channel::Canary), "lisa-canary");

        for channel in Channel::ALL {
            assert_eq!(channel_of_formula(formula_for(channel)), Some(channel));
        }
        assert_eq!(channel_of_formula("lisa-beta"), None);
    }

    #[test]
    fn the_formula_is_read_off_the_cellar_path_homebrew_installs_into() {
        assert_eq!(
            formula_from_exe(Path::new(
                "/opt/homebrew/Cellar/lisa-nightly/0.5.0-rc.2/bin/lisa"
            ))
            .as_deref(),
            Some("lisa-nightly")
        );
        assert_eq!(
            formula_from_exe(Path::new("/usr/local/Cellar/lisa/0.4.4/bin/lisa")).as_deref(),
            Some("lisa")
        );
        // The linked form, for a caller that did not resolve symlinks.
        assert_eq!(
            formula_from_exe(Path::new("/opt/homebrew/opt/lisa-canary/bin/lisa")).as_deref(),
            Some("lisa-canary")
        );
        // A Cellar with something else in it is not this machine's lisa.
        assert_eq!(
            formula_from_exe(Path::new("/opt/homebrew/Cellar/zellij/0.43.1/bin/zellij")),
            None
        );
        assert_eq!(
            formula_from_exe(Path::new("/Users/someone/.local/bin/lisa")),
            None
        );
    }

    /// The sources line the README publishes, verbatim.
    fn published_sources_line(suite: &str) -> String {
        format!(
            "deb [arch=amd64 signed-by=/usr/share/keyrings/lisa-archive-keyring.gpg] \
             https://johnhkchen.github.io/lisa {suite} main\n"
        )
    }

    fn apt_tree(files: &[(&str, String)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("sources.list.d")).unwrap();
        std::fs::write(
            root.path().join("sources.list"),
            "deb http://deb.debian.org/debian bookworm main\n",
        )
        .unwrap();
        for (name, contents) in files {
            std::fs::write(root.path().join("sources.list.d").join(name), contents).unwrap();
        }
        root
    }

    #[test]
    fn the_suite_word_in_lisas_own_sources_line_is_the_channel() {
        let root = apt_tree(&[("lisa.list", published_sources_line("nightly"))]);
        let (suite, file) = suite_from_apt(root.path()).unwrap();
        assert_eq!(suite, "nightly");
        assert!(file.ends_with("lisa.list"), "{}", file.display());
        assert_eq!(Channel::parse(&suite).unwrap(), Channel::Nightly);
    }

    #[test]
    fn another_projects_sources_line_is_not_lisas_channel() {
        let root = apt_tree(&[(
            "docker.list",
            "deb [arch=amd64] https://download.docker.com/linux/debian bookworm stable\n"
                .to_string(),
        )]);
        let error = suite_from_apt(root.path()).unwrap_err();
        assert!(error.contains("johnhkchen.github.io/lisa"), "{error}");
    }

    #[test]
    fn a_commented_out_sources_line_names_no_channel() {
        let root = apt_tree(&[(
            "lisa.list",
            format!("# {}", published_sources_line("canary")),
        )]);
        assert!(suite_from_apt(root.path()).is_err());
    }

    #[test]
    fn the_deb822_grammar_answers_the_same_question() {
        let root = apt_tree(&[(
            "lisa.sources",
            "Types: deb\nURIs: https://johnhkchen.github.io/lisa\nSuites: canary\n\
             Components: main\nSigned-By: /usr/share/keyrings/lisa-archive-keyring.gpg\n"
                .to_string(),
        )]);
        let (suite, _) = suite_from_apt(root.path()).unwrap();
        assert_eq!(suite, "canary");
    }

    /// Two channels at once is the state this whole story exists to prevent, so
    /// it is named rather than resolved by whichever file sorted first.
    #[test]
    fn two_sources_lines_naming_two_channels_is_reported_not_picked_between() {
        let root = apt_tree(&[
            ("lisa.list", published_sources_line("stable")),
            ("lisa-extra.list", published_sources_line("canary")),
        ]);
        let error = suite_from_apt(root.path()).unwrap_err();
        assert!(error.contains("canary"), "{error}");
        assert!(error.contains("stable"), "{error}");
        assert!(error.contains("one channel per box"), "{error}");
    }

    #[test]
    fn the_same_channel_named_twice_is_still_one_channel() {
        let root = apt_tree(&[
            ("lisa.list", published_sources_line("nightly")),
            ("lisa-copy.list", published_sources_line("nightly")),
        ]);
        assert_eq!(suite_from_apt(root.path()).unwrap().0, "nightly");
    }

    #[test]
    fn rewriting_the_suite_changes_lisas_line_and_nothing_else() {
        let file = format!(
            "# Lisa\n{}deb http://deb.debian.org/debian bookworm main\n",
            published_sources_line("stable")
        );
        let rewritten = rewrite_suite(&file, Channel::Nightly).unwrap();

        assert!(rewritten.contains("/lisa nightly main"), "{rewritten}");
        assert!(!rewritten.contains("/lisa stable main"), "{rewritten}");
        assert!(
            rewritten.contains("deb http://deb.debian.org/debian bookworm main"),
            "an unrelated line must survive verbatim: {rewritten}"
        );
        assert!(
            rewritten.contains("signed-by=/usr/share/keyrings/lisa-archive-keyring.gpg"),
            "the options on the line must survive: {rewritten}"
        );
    }

    #[test]
    fn rewriting_a_file_that_names_no_lisa_archive_is_an_error_not_a_silent_no_op() {
        let error = rewrite_suite(
            "deb http://deb.debian.org/debian bookworm main\n",
            Channel::Canary,
        )
        .unwrap_err();
        assert!(error.contains("no suite word to change"), "{error}");
    }

    #[test]
    fn a_deb822_stanza_is_rewritten_too() {
        let file = "Types: deb\nURIs: https://johnhkchen.github.io/lisa\nSuites: stable\n\
                    Components: main\n";
        let rewritten = rewrite_suite(file, Channel::Canary).unwrap();
        assert!(rewritten.contains("Suites: canary"), "{rewritten}");
        assert!(rewritten.contains("Components: main"), "{rewritten}");
    }

    #[test]
    fn a_package_managed_box_derives_its_channel_and_a_built_one_reads_the_config() {
        let brew = derive(
            &InstallMethod::Homebrew,
            Path::new("/opt/homebrew/Cellar/lisa-canary/0.5.0-rc.2/bin/lisa"),
        );
        assert_eq!(brew.channel, Some(Channel::Canary));
        assert_eq!(brew.source.describe(), "the Homebrew formula lisa-canary");

        let built = derive(
            &InstallMethod::Elsewhere,
            Path::new("/Users/someone/src/lisa/target/debug/lisa"),
        );
        assert_eq!(built.channel, None);
        assert_eq!(built.source, Source::Config);
    }

    /// A formula Lisa does not publish is not a channel, and saying so beats
    /// falling back to a config field the operator cannot see being used.
    #[test]
    fn a_homebrew_lisa_outside_a_cellar_is_named_as_unreadable() {
        let derived = derive(
            &InstallMethod::Homebrew,
            Path::new("/home/linuxbrew/bin/lisa"),
        );
        assert_eq!(derived.channel, None);
        assert_eq!(derived.source.name(), "package-unreadable");
        assert!(derived.source.describe().contains("Homebrew"));
    }

    #[test]
    fn the_brew_upgrade_plan_is_the_two_commands_a_person_would_type() {
        let plan = brew_upgrade("lisa-nightly");
        let lines: Vec<String> = plan.steps.iter().map(Mover::line).collect();
        assert_eq!(lines, ["brew update", "brew upgrade lisa-nightly"]);
        assert!(plan.by_hand.is_empty());
    }

    #[test]
    fn switching_formulae_fetches_before_it_uninstalls_and_names_the_way_back() {
        let plan = brew_switch("lisa", Channel::Nightly);
        let lines: Vec<String> = plan.steps.iter().map(Mover::line).collect();
        assert_eq!(
            lines,
            [
                "brew fetch johnhkchen/lisa/lisa-nightly",
                "brew uninstall lisa",
                "brew install johnhkchen/lisa/lisa-nightly",
            ],
            "the bottle is local before the running lisa is removed"
        );
        let back = plan.by_hand.join("\n");
        assert!(back.contains("brew install johnhkchen/lisa/lisa"), "{back}");
    }

    #[test]
    fn apt_commands_are_elevated_only_when_this_process_is_not_root() {
        let as_root: Vec<String> = apt_upgrade(Privilege::Root)
            .steps
            .iter()
            .map(Mover::line)
            .collect();
        assert_eq!(
            as_root,
            [
                "apt-get update",
                "apt-get install --only-upgrade -y lisa lisa-runtime-zellij"
            ]
        );

        let as_user: Vec<String> = apt_upgrade(Privilege::Sudo)
            .steps
            .iter()
            .map(Mover::line)
            .collect();
        assert_eq!(as_user[0], "sudo apt-get update");
        assert!(as_user[1].starts_with("sudo apt-get install --only-upgrade -y"));
    }

    #[test]
    fn a_box_with_no_way_to_become_root_gets_the_commands_printed() {
        let plan = apt_upgrade(Privilege::Neither);
        assert!(!Privilege::Neither.can_run());
        assert!(
            plan.by_hand.join(" ").contains("nothing above was run"),
            "{:?}",
            plan.by_hand
        );
    }

    /// The sharp edge from the story: apt has a real rollback and Homebrew does
    /// not, so only one of the two gets an exact-version command.
    #[test]
    fn an_apt_rollback_names_the_exact_version_of_both_packages() {
        let plan = apt_pin(Privilege::Root, &Version::parse("0.4.4").unwrap());
        let install = plan.steps.last().unwrap().line();
        assert!(install.contains("lisa=0.4.4-1"), "{install}");
        assert!(install.contains("lisa-runtime-zellij=0.4.4-1"), "{install}");
        assert!(install.contains("--allow-downgrades"), "{install}");
    }

    #[test]
    fn a_prerelease_is_packaged_the_way_debian_sorts_it() {
        assert_eq!(deb_version(&Version::parse("0.4.4").unwrap()), "0.4.4-1");
        assert_eq!(
            deb_version(&Version::parse("0.5.0-rc.2").unwrap()),
            "0.5.0~rc.2-1"
        );
    }

    #[test]
    fn switching_apt_channels_says_what_it_will_not_do_for_you() {
        let plan = apt_switch(
            Privilege::Sudo,
            Path::new("/etc/apt/sources.list.d/lisa.list"),
            Channel::Nightly,
        );
        assert!(plan.headline.contains("nightly"), "{}", plan.headline);
        let by_hand = plan.by_hand.join("\n");
        assert!(by_hand.contains("--allow-downgrades"), "{by_hand}");
        assert!(by_hand.contains("apt-cache madison lisa"), "{by_hand}");
    }

    #[test]
    fn the_version_to_ask_after_a_move_is_the_one_the_package_manager_relinks() {
        assert_eq!(
            package_lisa_path(
                &InstallMethod::Homebrew,
                Path::new("/opt/homebrew/Cellar/lisa-nightly/0.5.0/bin/lisa")
            ),
            Some(PathBuf::from("/opt/homebrew/bin/lisa"))
        );
        assert_eq!(
            package_lisa_path(&InstallMethod::Apt, Path::new("/usr/bin/lisa")),
            Some(PathBuf::from("/usr/bin/lisa"))
        );
        assert_eq!(
            package_lisa_path(
                &InstallMethod::ShellInstaller,
                Path::new("/Users/someone/.local/bin/lisa")
            ),
            None
        );
    }
}
