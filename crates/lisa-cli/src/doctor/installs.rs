//! Every lisa this machine could run, asked of the box rather than of the
//! process asking.
//!
//! The first version of this check derived the packaged copy from the running
//! binary — `classify_install(current_exe())` — which answers "what installed
//! *this* exe". That is the wrong question. Whether a box carries a
//! package-managed Lisa is a fact about the box: the keg is on disk whether or
//! not it is what `lisa` runs, and the dangerous case is exactly the one where
//! it is not. A brew keg nobody executes is a keg `brew upgrade` moves while
//! the machine goes on running something else, and the old check reported that
//! box as `OK` because the packaged copy was not the process asking.
//!
//! So the census reads the machine's own records:
//!
//! - **Homebrew** keeps `<prefix>/opt/<formula>/bin/lisa` pointed at the
//!   installed keg whether or not the formula is linked, and
//!   `<prefix>/bin/lisa` only while it is. Both are Homebrew's own layout, they
//!   are on disk, and reading them costs no subprocess and works on a box where
//!   `brew` is not on this PATH — which is the same box this check is about.
//! - **apt** owns `/usr/bin/lisa`, which is what `classify_install` has always
//!   meant by an apt Lisa.
//! - the shell installer owns `~/.local/bin/lisa`, `cargo install` owns
//!   `~/.cargo/bin/lisa`, and the running binary and whatever `lisa` resolves to
//!   on this PATH are two more places a real one was measured.
//!
//! Nothing here removes anything. Several lisas on one box is a state an
//! operator creates deliberately — `lisa upgrade --tag` is Homebrew's only
//! rollback — and the fix is saying how many there are, where, and which one
//! answers `lisa`.

use std::path::{Path, PathBuf};

use crate::channel::Channel;
use crate::upgrade::install_channel;
use crate::upgrade::InstallMethod;

/// The Homebrew prefixes a box can have. Overridable, colon-separated, so a
/// test can hand Lisa a prefix instead of the machine's own.
const BREW_PREFIXES_ENV: &str = "LISA_HOMEBREW_PREFIXES";

/// Where Homebrew installs, on the three layouts it ships: Apple silicon,
/// Intel macOS, and Linuxbrew.
const BREW_PREFIXES: [&str; 3] = ["/opt/homebrew", "/usr/local", "/home/linuxbrew/.linuxbrew"];

/// Who put a lisa where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Origin {
    /// A Homebrew keg, named by the formula that owns it, and whether that
    /// formula is linked onto `<prefix>/bin`.
    Homebrew {
        formula: Option<String>,
        linked: bool,
    },
    /// The Debian package.
    Apt,
    /// The shell installer's own directory, `~/.local/bin`.
    ShellInstaller,
    /// `cargo install`'s bin directory.
    Cargo,
    /// Somewhere else: a build tree, another prefix, a hand-placed copy.
    Elsewhere,
}

impl Origin {
    /// The phrase the row prints beside a path.
    pub(crate) fn describe(&self) -> String {
        match self {
            Origin::Homebrew {
                formula: Some(formula),
                linked: true,
            } => format!("Homebrew's {formula}"),
            Origin::Homebrew {
                formula: Some(formula),
                linked: false,
            } => format!("Homebrew's {formula}, unlinked"),
            Origin::Homebrew { formula: None, .. } => "Homebrew's prefix".to_string(),
            Origin::Apt => "apt".to_string(),
            Origin::ShellInstaller => "the shell installer".to_string(),
            Origin::Cargo => "cargo install".to_string(),
            Origin::Elsewhere => "no package manager".to_string(),
        }
    }

    /// The one word a script sorts on.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Origin::Homebrew { .. } => "homebrew",
            Origin::Apt => "apt",
            Origin::ShellInstaller => "shell-installer",
            Origin::Cargo => "cargo",
            Origin::Elsewhere => "elsewhere",
        }
    }

    /// The formula a Homebrew keg belongs to, for a script and for `brew link`.
    pub(crate) fn formula(&self) -> Option<&str> {
        match self {
            Origin::Homebrew { formula, .. } => formula.as_deref(),
            _ => None,
        }
    }

    /// Whether a package manager owns this copy, and therefore whether a
    /// channel move is about it.
    pub(crate) fn package_managed(&self) -> bool {
        matches!(self, Origin::Homebrew { .. } | Origin::Apt)
    }

    /// The package manager's name, as the remedy says it.
    fn manager(&self) -> &'static str {
        match self {
            Origin::Homebrew { .. } => "Homebrew",
            _ => "apt",
        }
    }
}

/// One lisa this machine could run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Install {
    /// The path an operator would type, which is the linked one where Homebrew
    /// keeps a link and the keg itself where it does not.
    pub(crate) path: PathBuf,
    pub(crate) origin: Origin,
    /// What that copy says it is, asked rather than assumed. `None` when it
    /// could not be asked.
    pub(crate) version: Option<String>,
    /// Whether this is the binary running right now.
    pub(crate) running: bool,
    /// Whether this is the one the operator's shell finds first.
    pub(crate) first_on_path: bool,
}

/// The box the census is taken of. Every field is a place to look, so a test
/// can hand this a machine instead of measuring the one it runs on.
pub(crate) struct Machine {
    pub(crate) brew_prefixes: Vec<PathBuf>,
    pub(crate) apt_lisa: PathBuf,
    pub(crate) home: Option<PathBuf>,
    /// The binary asking, which is one of the answers and never the question.
    pub(crate) running: Option<PathBuf>,
    /// What `lisa` resolves to on this PATH.
    pub(crate) on_path: Option<PathBuf>,
}

impl Machine {
    /// This box, as it is.
    pub(crate) fn look() -> Self {
        Self {
            brew_prefixes: brew_prefixes(),
            apt_lisa: install_channel::apt_lisa_path(),
            home: std::env::var_os("HOME").map(PathBuf::from),
            running: std::env::current_exe().ok(),
            on_path: super::first_lisa_on_path(),
        }
    }
}

/// The Homebrew prefixes to look in.
fn brew_prefixes() -> Vec<PathBuf> {
    match std::env::var_os(BREW_PREFIXES_ENV) {
        Some(listed) => std::env::split_paths(&listed).collect(),
        None => BREW_PREFIXES.iter().map(PathBuf::from).collect(),
    }
}

/// The formulae Lisa publishes, which are the kegs a box can be carrying.
fn formulae() -> Vec<&'static str> {
    Channel::ALL
        .into_iter()
        .map(install_channel::formula_for)
        .collect()
}

/// A path with its symlinks resolved, which is how two names for one file are
/// recognised as one lisa. Unresolvable paths keep the name they were given.
fn real(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Every lisa on this machine, in the order an operator should read them:
/// package-managed copies first, because those are the ones a channel move is
/// about.
pub(crate) fn census(machine: &Machine) -> Vec<Install> {
    let mut found: Vec<Install> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    let home = machine.home.as_deref();

    for prefix in &machine.brew_prefixes {
        // The linked copy first: where a formula is linked, `<prefix>/bin/lisa`
        // is the name an operator has in their PATH, and canonicalising it
        // collapses it with the keg it points at.
        let linked = prefix.join("bin").join("lisa");
        let formula = install_channel::formula_from_exe(&real(&linked));
        note(
            &mut found,
            &mut seen,
            linked,
            Origin::Homebrew {
                formula,
                linked: true,
            },
        );

        // Then the kegs themselves. An unlinked keg is still installed, still
        // moved by `brew upgrade`, and still absent from PATH — which is the
        // case this whole check exists for.
        for formula in formulae() {
            note(
                &mut found,
                &mut seen,
                prefix.join("opt").join(formula).join("bin").join("lisa"),
                Origin::Homebrew {
                    formula: Some(formula.to_string()),
                    linked: false,
                },
            );
        }
    }

    note(&mut found, &mut seen, machine.apt_lisa.clone(), Origin::Apt);

    if let Some(home) = home {
        note(
            &mut found,
            &mut seen,
            crate::upgrade::installer_owned_path(home),
            Origin::ShellInstaller,
        );
        note(
            &mut found,
            &mut seen,
            home.join(".cargo").join("bin").join("lisa"),
            Origin::Cargo,
        );
    }

    for path in [machine.running.clone(), machine.on_path.clone()]
        .into_iter()
        .flatten()
    {
        let origin = classify(&path, home);
        note(&mut found, &mut seen, path, origin);
    }

    let running = machine.running.as_deref().map(real);
    let on_path = machine.on_path.as_deref().map(real);
    for install in &mut found {
        let key = real(&install.path);
        install.running = running.as_ref() == Some(&key);
        install.first_on_path = on_path.as_ref() == Some(&key);
        // The running binary knows its own version without being asked, and
        // asking it would mean this process spawning itself.
        install.version = if install.running {
            Some(env!("CARGO_PKG_VERSION").to_string())
        } else {
            crate::upgrade::version_of(&install.path).map(|version| version.to_string())
        };
    }

    found
}

/// Record a copy, unless the file is not there or is one already recorded under
/// another name.
fn note(found: &mut Vec<Install>, seen: &mut Vec<PathBuf>, path: PathBuf, origin: Origin) {
    if !path.exists() {
        return;
    }
    let key = real(&path);
    if seen.contains(&key) {
        return;
    }
    seen.push(key);
    found.push(Install {
        path,
        origin,
        version: None,
        running: false,
        first_on_path: false,
    });
}

/// Who owns a copy found by path rather than by looking where a manager keeps
/// its own.
///
/// The name is classified before its symlinks are resolved and after, because
/// both carry the answer in different cases: `/opt/homebrew/bin/lisa` says
/// Homebrew by its own name, and a symlink into a Cellar says it only once
/// resolved.
fn classify(path: &Path, home: Option<&Path>) -> Origin {
    let resolved = real(path);
    let method = match crate::upgrade::classify_install(path, home) {
        InstallMethod::Elsewhere => crate::upgrade::classify_install(&resolved, home),
        named => named,
    };

    match method {
        InstallMethod::Homebrew => Origin::Homebrew {
            formula: install_channel::formula_from_exe(&resolved),
            linked: false,
        },
        InstallMethod::Apt => Origin::Apt,
        InstallMethod::ShellInstaller => Origin::ShellInstaller,
        InstallMethod::Elsewhere => match home {
            Some(home) if path.parent() == Some(&home.join(".cargo").join("bin")) => Origin::Cargo,
            _ => Origin::Elsewhere,
        },
    }
}

/// What the census means for the operator reading it.
pub(crate) enum Verdict {
    /// One lisa, and no doubt about which one answers.
    Settled(String),
    /// More than one lisa, or a packaged one that is not what runs. Either way
    /// the machine's channel and the machine's behaviour can disagree.
    Muddled { description: String, remedy: String },
}

/// Read the census.
///
/// Not OK on two counts, and the second is the one this ticket is: more than
/// one lisa is worth saying out loud, and a package-managed lisa that is not
/// what `lisa` runs is worth saying however few there are — `brew upgrade`
/// moving a binary nobody executes is a machine whose channel describes
/// something that never runs.
pub(crate) fn read(found: &[Install]) -> Option<Verdict> {
    let first = found.iter().find(|install| install.first_on_path);
    let shadowed = found
        .iter()
        .find(|install| install.origin.package_managed() && !install.first_on_path);

    if found.len() <= 1 && shadowed.is_none() {
        let one = found.first()?;
        return Some(Verdict::Settled(format!(
            "one lisa, at {} ({}){}",
            one.path.display(),
            one.origin.describe(),
            match first {
                Some(_) => " — and `lisa` runs it".to_string(),
                None => ", which is not on this PATH".to_string(),
            }
        )));
    }

    // "this machine has N lisas" was a completeness claim this method cannot
    // support: it looks in the install locations lisa knows about, plus whatever
    // is running. `screen-design` found a fifth on a box this reported four on —
    // a sibling build artifact nothing had invoked. Claiming only what was
    // checked is the same discipline the rest of this check is for.
    let mut description = format!(
        "{} lisa{} where lisa looks, plus the one running this:",
        found.len(),
        if found.len() == 1 { "" } else { "s" }
    );
    for install in found {
        description.push_str(&format!("\n      {}", line(install)));
    }
    description.push_str(&match first {
        Some(first) => format!(
            "\n      `lisa` runs {}, and PATH order is what decides that",
            first.path.display()
        ),
        None => "\n      nothing named lisa is on this PATH".to_string(),
    });

    Some(Verdict::Muddled {
        description,
        remedy: remedy(found, shadowed),
    })
}

/// One copy, as a line in the list.
fn line(install: &Install) -> String {
    format!(
        "{}  ({}{}){}",
        install.path.display(),
        install.origin.describe(),
        match &install.version {
            Some(version) => format!(", {version}"),
            None => String::new(),
        },
        match (install.first_on_path, install.running) {
            (true, _) => "  <- `lisa` runs this one",
            (false, true) => "  <- this doctor",
            (false, false) => "",
        }
    )
}

/// What to do about it, naming the packaged copy first because that is the one
/// a channel move is about.
fn remedy(found: &[Install], shadowed: Option<&Install>) -> String {
    let mut lines: Vec<String> = Vec::new();

    if let Some(shadowed) = shadowed {
        lines.push(format!(
            "{} upgrades move {}, which is not what `lisa` runs here, so a channel move on this \
             box changes nothing you execute.",
            shadowed.origin.manager(),
            shadowed.path.display()
        ));
        if let (Some(formula), Origin::Homebrew { linked: false, .. }) =
            (shadowed.origin.formula(), &shadowed.origin)
        {
            lines.push(format!(
                "That keg is unlinked, so nothing puts it on PATH:\n    brew link {formula}"
            ));
        }
    }

    let packaged = found
        .iter()
        .find(|install| install.origin.package_managed());
    if let (Some(packaged), Some(installed)) = (
        packaged,
        found
            .iter()
            .find(|install| matches!(install.origin, Origin::ShellInstaller)),
    ) {
        lines.push(format!(
            "To go back to the packaged lisa:\n    rm {}\nTo keep the pinned one instead, leave \
             it and remember that {} upgrades will not move it.",
            installed.path.display(),
            packaged.origin.manager()
        ));
    }

    if lines.is_empty() {
        lines.push(
            "Keep one. Remove the copies you did not mean to have, or know which one you are \
             reading a version off — every command that says `installed` says it about one of \
             these files."
                .to_string(),
        );
    }

    // One sentence per line, each command indented under it, which is how every
    // other remedy Lisa prints reads.
    lines.join("\n")
}

/// The census as a script reads it.
pub(crate) fn to_json(found: &[Install]) -> serde_json::Value {
    serde_json::Value::Array(
        found
            .iter()
            .map(|install| {
                serde_json::json!({
                    "path": install.path.display().to_string(),
                    "origin": install.origin.name(),
                    "formula": install.origin.formula(),
                    "package_managed": install.origin.package_managed(),
                    "version": install.version,
                    "running": install.running,
                    "first_on_path": install.first_on_path,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A box with nothing on it, for a test to put lisas on.
    fn empty_machine(root: &Path) -> Machine {
        Machine {
            brew_prefixes: vec![root.join("brew")],
            apt_lisa: root.join("nowhere").join("usr").join("bin").join("lisa"),
            home: Some(root.join("home")),
            running: None,
            on_path: None,
        }
    }

    /// Something that exists at a path and is not a lisa that can be asked for
    /// a version. The census is about which files are there; what they answer
    /// is best-effort by design.
    fn put(path: &Path) -> PathBuf {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "#!/bin/sh\nexit 1\n").unwrap();
        path.to_path_buf()
    }

    /// Homebrew's own layout: the keg under `Cellar`, and `opt/<formula>`
    /// pointed at it whether or not the formula is linked.
    fn keg(prefix: &Path, formula: &str, version: &str) -> PathBuf {
        let cellar = prefix.join("Cellar").join(formula).join(version);
        put(&cellar.join("bin").join("lisa"));
        std::fs::create_dir_all(prefix.join("opt")).unwrap();
        std::os::unix::fs::symlink(&cellar, prefix.join("opt").join(formula)).unwrap();
        prefix.join("opt").join(formula).join("bin").join("lisa")
    }

    /// `brew link`: the linked name on PATH is a symlink into the keg.
    fn link(prefix: &Path, formula: &str) {
        std::fs::create_dir_all(prefix.join("bin")).unwrap();
        std::os::unix::fs::symlink(
            prefix.join("opt").join(formula).join("bin").join("lisa"),
            prefix.join("bin").join("lisa"),
        )
        .unwrap();
    }

    /// The measured state on this MacBook, 2026-08-14: a brew keg that is not
    /// what answers `lisa`, and the old check called it one lisa and OK.
    #[test]
    fn an_unlinked_keg_is_found_even_though_no_process_here_came_from_it() {
        let root = tempfile::tempdir().unwrap();
        let prefix = root.path().join("brew");
        let unlinked = keg(&prefix, "lisa", "0.4.4");
        let installed = put(&root.path().join("home/.local/bin/lisa"));

        let machine = Machine {
            on_path: Some(installed.clone()),
            running: Some(installed.clone()),
            ..empty_machine(root.path())
        };
        let found = census(&machine);

        assert_eq!(
            found.iter().map(|it| it.path.clone()).collect::<Vec<_>>(),
            vec![unlinked.clone(), installed.clone()],
            "the packaged copy is found by looking at the box, and is listed first"
        );
        assert_eq!(
            found[0].origin,
            Origin::Homebrew {
                formula: Some("lisa".to_string()),
                linked: false
            }
        );
        assert!(!found[0].first_on_path);
        assert!(found[1].first_on_path);

        let Some(Verdict::Muddled {
            description,
            remedy,
        }) = read(&found)
        else {
            panic!("a keg nobody runs is not OK");
        };
        assert!(description.contains("2 lisas"), "{description}");
        assert!(
            description.contains(&unlinked.display().to_string()),
            "{description}"
        );
        assert!(
            description.contains(&installed.display().to_string()),
            "{description}"
        );
        assert!(remedy.contains("brew link lisa"), "{remedy}");
        assert!(remedy.contains("Homebrew upgrades move"), "{remedy}");
    }

    /// A linked formula is one lisa under two names, not two lisas.
    #[test]
    fn the_linked_name_and_the_keg_it_points_at_are_one_lisa() {
        let root = tempfile::tempdir().unwrap();
        let prefix = root.path().join("brew");
        keg(&prefix, "lisa-nightly", "0.5.0-rc.2");
        link(&prefix, "lisa-nightly");
        let linked = prefix.join("bin").join("lisa");

        let machine = Machine {
            on_path: Some(linked.clone()),
            running: Some(linked.clone()),
            ..empty_machine(root.path())
        };
        let found = census(&machine);

        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(
            found[0].path, linked,
            "the name on PATH is the one to print"
        );
        assert_eq!(
            found[0].origin,
            Origin::Homebrew {
                formula: Some("lisa-nightly".to_string()),
                linked: true
            }
        );

        let Some(Verdict::Settled(said)) = read(&found) else {
            panic!("one lisa, linked and running, is the settled case");
        };
        assert!(said.contains("one lisa"), "{said}");
        assert!(said.contains("Homebrew's lisa-nightly"), "{said}");
        assert!(
            said.contains("`lisa` runs it"),
            "even a box with one lisa says which one answered: {said}"
        );
    }

    /// Three is a number the old wording could not say. This box has one.
    #[test]
    fn a_third_copy_is_counted_and_listed_rather_than_rounded_to_two() {
        let root = tempfile::tempdir().unwrap();
        let prefix = root.path().join("brew");
        keg(&prefix, "lisa", "0.4.4");
        let installed = put(&root.path().join("home/.local/bin/lisa"));
        let cargo = put(&root.path().join("home/.cargo/bin/lisa"));

        let machine = Machine {
            on_path: Some(installed.clone()),
            running: Some(cargo.clone()),
            ..empty_machine(root.path())
        };
        let found = census(&machine);

        assert_eq!(found.len(), 3, "{found:?}");
        assert_eq!(found[2].origin, Origin::Cargo);

        let Some(Verdict::Muddled { description, .. }) = read(&found) else {
            panic!("three lisas is not OK");
        };
        assert!(description.contains("3 lisas"), "{description}");
        assert!(
            description.contains(&cargo.display().to_string()),
            "the third one is listed, not implied: {description}"
        );
        assert!(
            description.contains("`lisa` runs"),
            "the list names which one answered: {description}"
        );
    }

    /// A box with one lisa and no package manager anywhere near it is the
    /// common case, and it stays quiet.
    #[test]
    fn one_shell_installed_lisa_is_settled() {
        let root = tempfile::tempdir().unwrap();
        let installed = put(&root.path().join("home/.local/bin/lisa"));

        let machine = Machine {
            on_path: Some(installed.clone()),
            running: Some(installed),
            ..empty_machine(root.path())
        };
        let found = census(&machine);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].origin, Origin::ShellInstaller);
        assert!(matches!(read(&found), Some(Verdict::Settled(_))));
    }

    /// Nothing on PATH is its own answer: the one lisa here is a keg no shell
    /// can reach, which is the state `brew upgrade` is most misleading about.
    #[test]
    fn a_keg_with_nothing_on_path_is_not_ok_even_though_it_is_the_only_lisa() {
        let root = tempfile::tempdir().unwrap();
        let prefix = root.path().join("brew");
        keg(&prefix, "lisa", "0.4.4");

        let machine = empty_machine(root.path());
        let found = census(&machine);

        assert_eq!(found.len(), 1);
        let Some(Verdict::Muddled { description, .. }) = read(&found) else {
            panic!("a keg nothing can run is not OK");
        };
        assert!(
            description.contains("nothing named lisa is on this PATH"),
            "{description}"
        );
    }

    /// A script reads the same list the person reads.
    #[test]
    fn the_json_carries_every_copy_with_where_it_came_from() {
        let root = tempfile::tempdir().unwrap();
        let prefix = root.path().join("brew");
        keg(&prefix, "lisa-canary", "0.5.0-rc.2");
        let installed = put(&root.path().join("home/.local/bin/lisa"));

        let machine = Machine {
            on_path: Some(installed),
            running: None,
            ..empty_machine(root.path())
        };
        let document = to_json(&census(&machine));
        let listed = document.as_array().unwrap();

        assert_eq!(listed.len(), 2, "{document}");
        assert_eq!(listed[0]["origin"], "homebrew");
        assert_eq!(listed[0]["formula"], "lisa-canary");
        assert_eq!(listed[0]["package_managed"], true);
        assert_eq!(listed[0]["first_on_path"], false);
        assert_eq!(listed[1]["origin"], "shell-installer");
        assert_eq!(listed[1]["first_on_path"], true);
    }
}
