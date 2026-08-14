//! A box whose Lisa came from a package reads its channel off that package
//! (T-069-01-04).
//!
//! `S-069-01` publishes one Homebrew formula and one apt suite per channel, so
//! the installed package answers "which line of Lisa is this box on" — and the
//! package manager, not Lisa, is what moves it. These cases run the real binary
//! from a directory shaped like a Homebrew Cellar, which is exactly how
//! `classify_install` recognises a brew box, and pin what an operator sees:
//! the channel, where it was derived from, and the commands Lisa would hand
//! over.
//!
//! Nothing here runs `brew`. Every upgrade case is `--dry-run`, which is the
//! whole point of that flag: the plan is printed and nothing on the machine
//! running the suite is touched. The apt half of the derivation is unit-tested
//! in `src/upgrade/install_channel.rs`, because `/usr/bin` cannot be faked in a
//! temporary directory.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// The version this build is, which is the version a copy of it reports.
const INSTALLED: &str = env!("CARGO_PKG_VERSION");

/// A Homebrew prefix with this build installed as one of the three formulae.
///
/// `<prefix>/Cellar/<formula>/<version>/bin/lisa` is Homebrew's own layout, and
/// the `Cellar` segment is what tells Lisa a package manager owns this file.
struct BrewBox {
    _prefix: TempDir,
    lisa: PathBuf,
}

fn brew_box(formula: &str) -> BrewBox {
    let prefix = TempDir::new().unwrap();
    let lisa = install_keg(prefix.path(), formula);
    link_keg(prefix.path(), formula);

    BrewBox {
        _prefix: prefix,
        lisa,
    }
}

/// Put a keg in a Homebrew prefix, in Homebrew's own layout: the versioned
/// directory under `Cellar`, and `opt/<formula>` pointed at it. The `opt` link
/// is there whether or not the formula is linked onto `PATH`, which is why it
/// is what the census reads.
fn install_keg(prefix: &Path, formula: &str) -> PathBuf {
    let cellar = prefix
        .join("Cellar")
        .join(formula)
        .join(INSTALLED)
        .join("bin");
    std::fs::create_dir_all(&cellar).unwrap();
    let lisa = cellar.join("lisa");
    std::fs::copy(env!("CARGO_BIN_EXE_lisa"), &lisa).expect("copy this build into a Cellar");

    std::fs::create_dir_all(prefix.join("opt")).unwrap();
    let _ = std::fs::remove_file(prefix.join("opt").join(formula));
    std::os::unix::fs::symlink(
        prefix.join("Cellar").join(formula).join(INSTALLED),
        prefix.join("opt").join(formula),
    )
    .unwrap();

    lisa
}

/// `brew link`: the name on `PATH` is a symlink into the keg, which is why one
/// linked formula is one lisa under two names rather than two lisas.
fn link_keg(prefix: &Path, formula: &str) {
    std::fs::create_dir_all(prefix.join("bin")).unwrap();
    std::os::unix::fs::symlink(
        prefix.join("opt").join(formula).join("bin").join("lisa"),
        prefix.join("bin").join("lisa"),
    )
    .unwrap();
}

/// A project Lisa will answer about.
fn project() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
    std::fs::write(
        dir.path().join(".lisa.toml"),
        format!("version = \"{INSTALLED}\"\n"),
    )
    .unwrap();
    dir
}

/// A machine config that names a channel, which a package-managed box does not
/// read.
fn machine_on(channel: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("config.toml"),
        format!("channel = \"{channel}\"\n"),
    )
    .unwrap();
    dir
}

fn run(lisa: &Path, config_dir: &Path, args: &[&str]) -> Output {
    Command::new(lisa)
        .args(args)
        .env("LISA_CONFIG_DIR", config_dir)
        // Nothing here should reach the network; a port nothing answers on
        // makes that a failed lookup rather than a slow one if it ever does.
        .env("LISA_RELEASES_URL", "http://127.0.0.1:1/releases")
        .output()
        .expect("run lisa")
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A machine with nothing on it but what a case puts there (T-069-01-06).
///
/// `doctor`'s install census asks the box, not the process — so every place it
/// looks has to be a place this suite owns, or the cases read the laptop
/// running them. That laptop is not hypothetical: the box this ticket was
/// measured on carries three lisas, and a case that counted those would be
/// measuring the wrong machine.
struct Machine {
    root: TempDir,
    home: TempDir,
    config: TempDir,
    project: TempDir,
}

impl Machine {
    fn new() -> Self {
        let machine = Machine {
            root: TempDir::new().unwrap(),
            home: TempDir::new().unwrap(),
            config: TempDir::new().unwrap(),
            project: project(),
        };
        std::fs::create_dir_all(machine.home.path().join(".local/bin")).unwrap();
        std::fs::create_dir_all(machine.brew().join("bin")).unwrap();
        machine
    }

    /// The one Homebrew prefix this box has.
    fn brew(&self) -> PathBuf {
        self.root.path().join("brew")
    }

    /// What the shell installer wrote, if a case says it did. Executable,
    /// because `PATH` order only decides between things a shell will run.
    fn installer_copy(&self) -> PathBuf {
        let lisa = self.home.path().join(".local/bin/lisa");
        std::fs::copy(env!("CARGO_BIN_EXE_lisa"), &lisa)
            .expect("copy this build into ~/.local/bin");
        lisa
    }

    /// A channel named in the machine config, which a package-managed box does
    /// not read.
    fn config_channel(&self, channel: &str) -> &Self {
        std::fs::write(
            self.config.path().join("config.toml"),
            format!("channel = \"{channel}\"\n"),
        )
        .unwrap();
        self
    }

    /// `lisa doctor --json`, run from whichever lisa a case is asking.
    ///
    /// `PATH` is the two directories a real box would have plus `/bin` for
    /// `sh`, in the order a real box has them: `~/.local/bin` before Homebrew
    /// is why a pinned copy shadows a keg. Nothing else is on it — the suite
    /// has to run the same way on a Debian box that really does carry
    /// `/usr/bin/lisa`.
    fn doctor(&self, lisa: &Path) -> serde_json::Value {
        let output = Command::new(lisa)
            .arg("doctor")
            .arg("--path")
            .arg(self.project.path())
            .arg("--json")
            .env("HOME", self.home.path())
            .env(
                "PATH",
                format!(
                    "{}/.local/bin:{}/bin:/bin",
                    self.home.path().display(),
                    self.brew().display()
                ),
            )
            .env("LISA_HOMEBREW_PREFIXES", self.brew())
            .env("LISA_APT_LISA", self.root.path().join("no-apt-here/lisa"))
            .env("LISA_CONFIG_DIR", self.config.path())
            .env("LISA_RELEASES_URL", "http://127.0.0.1:1/releases")
            .output()
            .expect("run lisa doctor");

        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("one JSON document")
    }
}

/// The `lisa install` row: which lisas this box has.
fn install_row(document: &serde_json::Value) -> serde_json::Value {
    document["data"]["checks"]
        .as_array()
        .expect("doctor lists its checks")
        .iter()
        .find(|check| check["name"] == "lisa install")
        .expect("the row is always there")
        .clone()
}

/// The change this ticket is: `lisa upgrade` on a brew box used to refuse and
/// print `brew upgrade lisa`. Now it hands the move to the formula it is on.
#[test]
fn upgrade_on_a_brew_box_hands_the_move_to_the_formula_that_installed_it() {
    let machine = machine_on("stable");
    let brew = brew_box("lisa-nightly");

    let output = run(&brew.lisa, machine.path(), &["upgrade", "--dry-run"]);
    let said = said(&output);

    assert!(output.status.success(), "{said}");
    assert!(said.contains("Channel nightly"), "{said}");
    assert!(
        said.contains("from the Homebrew formula lisa-nightly"),
        "the row has to say which of the three sources answered:\n{said}"
    );
    assert!(said.contains("brew update"), "{said}");
    assert!(said.contains("brew upgrade lisa-nightly"), "{said}");
    assert!(
        !said.contains("will not write over a file"),
        "the old refusal is gone:\n{said}"
    );
    assert!(said.contains("--dry-run: nothing was run"), "{said}");
}

/// The config field is the second source of truth this ticket removes: on a
/// package-managed box it is not read, whatever it says.
#[test]
fn the_config_channel_does_not_override_the_package() {
    let machine = machine_on("canary");
    let brew = brew_box("lisa");

    let said = said(&run(&brew.lisa, machine.path(), &["upgrade", "--dry-run"]));

    assert!(said.contains("Channel stable"), "{said}");
    assert!(said.contains("the Homebrew formula lisa"), "{said}");
    assert!(
        !said.contains("Channel canary"),
        "a config field must not out-talk the installed package:\n{said}"
    );
}

/// Switching channel is switching packages, and nothing is written to the
/// config file while doing it.
#[test]
fn asking_for_another_channel_switches_the_formula_and_writes_no_config() {
    let machine = TempDir::new().unwrap();
    let brew = brew_box("lisa");

    let said = said(&run(
        &brew.lisa,
        machine.path(),
        &["upgrade", "--channel", "nightly", "--dry-run"],
    ));

    assert!(said.contains("brew uninstall lisa"), "{said}");
    assert!(
        said.contains("brew install johnhkchen/lisa/lisa-nightly"),
        "{said}"
    );
    assert!(
        said.contains("nothing was written to Lisa's config file"),
        "the operator has to be told the config is not where this lives:\n{said}"
    );
    assert!(
        !machine.path().join("config.toml").exists(),
        "a package-managed box must not grow a config file that disagrees with its package"
    );
}

/// Homebrew has no rollback — `brew switch` is gone — so `--tag` stays the
/// escape hatch, and the two-lisa state it creates is said out loud.
#[test]
fn a_pin_on_a_brew_box_says_it_is_the_installer_and_names_the_state_it_leaves() {
    let machine = TempDir::new().unwrap();
    let brew = brew_box("lisa-canary");

    let output = run(
        &brew.lisa,
        machine.path(),
        &["upgrade", "--tag", "v0.4.4", "--dry-run"],
    );
    let said = said(&output);

    assert!(said.contains("Homebrew cannot do this"), "{said}");
    assert!(said.contains("~/.local/bin"), "{said}");
    assert!(
        said.contains("two lisas"),
        "the state a pin leaves behind is the one doctor then reports:\n{said}"
    );
    // The release list is unreachable in this suite, so the pin stops there —
    // and stopping without touching anything is the right answer.
    assert!(
        !output.status.success(),
        "an unreachable release list is a failure, not a silent no-op:\n{said}"
    );
    assert!(said.contains("is unchanged"), "{said}");
}

/// `doctor` is where an operator reads a surprising channel, so it names the
/// formula, and names the config line that is not being read.
#[test]
fn doctor_reports_the_channel_it_derived_and_the_config_it_ignored() {
    let machine = Machine::new();
    machine.config_channel("canary");
    let keg = install_keg(&machine.brew(), "lisa-nightly");
    link_keg(&machine.brew(), "lisa-nightly");

    let document = machine.doctor(&keg);
    let lisa = &document["data"]["lisa"];

    assert_eq!(lisa["channel"], "nightly");
    assert_eq!(lisa["effective_channel"], "nightly");
    assert_eq!(lisa["channel_source"], "homebrew-formula");
    assert_eq!(
        lisa["channel_source_detail"],
        "the Homebrew formula lisa-nightly"
    );
    let conflict = lisa["channel_conflict"]
        .as_str()
        .expect("a config field that disagrees is reported, not silently ignored");
    assert!(conflict.contains("canary"), "{conflict}");
    assert!(conflict.contains("is not read"), "{conflict}");
}

/// Two lisas on one box is a real state — a `--tag` pin on Homebrew creates it
/// deliberately — and PATH order deciding which one runs is how this went wrong
/// before. `doctor` reports it; nothing removes anything.
#[test]
fn doctor_reports_a_box_carrying_both_a_packaged_lisa_and_an_installed_one() {
    let machine = Machine::new();
    let keg = install_keg(&machine.brew(), "lisa");
    link_keg(&machine.brew(), "lisa");
    let pinned = machine.installer_copy();

    let document = machine.doctor(&keg);
    let row = install_row(&document);

    assert_eq!(row["status"], "unsupported");
    let detail = row["detail"].as_str().unwrap();
    assert!(detail.contains("2 lisas"), "{detail}");
    assert!(detail.contains(".local/bin/lisa"), "{detail}");
    assert!(row["remedy"].as_str().unwrap().contains("rm "), "{row}");
    assert_eq!(
        row["required"], false,
        "two lisas is a finding to report, not a reason to stop the machine working"
    );

    let listed = document["data"]["lisa_installs"].as_array().unwrap();
    assert_eq!(listed.len(), 2, "{listed:?}");
    assert_eq!(listed[0]["origin"], "homebrew");
    assert_eq!(listed[0]["formula"], "lisa");
    assert_eq!(
        listed[1]["path"],
        pinned.display().to_string(),
        "the script sees the same two files the person sees"
    );
    assert_eq!(
        listed[1]["first_on_path"], true,
        "~/.local/bin comes before Homebrew on this PATH, which is the point"
    );
}

/// A box with one lisa says so, names it, and moves on.
#[test]
fn doctor_is_quiet_when_there_is_only_one_lisa() {
    let machine = Machine::new();
    let keg = install_keg(&machine.brew(), "lisa");
    link_keg(&machine.brew(), "lisa");

    let document = machine.doctor(&keg);
    let row = install_row(&document);

    assert_eq!(row["status"], "ok");
    let detail = row["detail"].as_str().unwrap();
    assert!(detail.contains("one lisa"), "{detail}");
    assert!(
        detail.contains(&machine.brew().join("bin/lisa").display().to_string()),
        "one lisa still says which one answered `lisa`: {detail}"
    );
    assert!(detail.contains("Homebrew's lisa"), "{detail}");
    assert_eq!(
        document["data"]["lisa_installs"].as_array().unwrap().len(),
        1,
        "the linked name and the keg it points at are one file, not two lisas"
    );
}

/// The bug this ticket is (T-069-01-06). Measured on a MacBook on 2026-08-14: a
/// Homebrew keg that is not what answers `lisa`, and `doctor` said *one lisa*
/// and `OK` — because it derived the packaged copy from the running process,
/// which is the one case where the packaged copy is safe.
///
/// This is the dangerous shape: `brew upgrade lisa` moves the keg, the shell
/// keeps running `~/.local/bin/lisa`, and the channel the box reports describes
/// a binary nobody executes.
#[test]
fn doctor_finds_a_packaged_lisa_that_is_not_the_one_lisa_runs() {
    let machine = Machine::new();
    // Installed but unlinked, which is the measured state: `<prefix>/bin/lisa`
    // is absent while `<prefix>/opt/lisa/bin/lisa` runs.
    install_keg(&machine.brew(), "lisa");
    let pinned = machine.installer_copy();

    // Asked of the pinned copy, which is what the shell finds — the case the
    // old check could not see.
    let document = machine.doctor(&pinned);
    let row = install_row(&document);

    assert_ne!(
        row["status"], "ok",
        "a keg `brew upgrade` moves and nothing runs is not an OK machine: {row}"
    );
    let detail = row["detail"].as_str().unwrap();
    assert!(detail.contains("2 lisas"), "{detail}");
    assert!(
        detail.contains(
            &machine
                .brew()
                .join("opt/lisa/bin/lisa")
                .display()
                .to_string()
        ),
        "the unlinked keg is named, not just counted: {detail}"
    );
    assert!(
        detail.contains(&pinned.display().to_string()),
        "and so is the one that answered: {detail}"
    );

    let remedy = row["remedy"].as_str().unwrap();
    assert!(
        remedy.contains("Homebrew upgrades move"),
        "the remedy says what a channel move would and would not touch: {remedy}"
    );
    assert!(remedy.contains("brew link lisa"), "{remedy}");

    let listed = document["data"]["lisa_installs"].as_array().unwrap();
    assert_eq!(listed.len(), 2, "{listed:?}");
    assert_eq!(listed[0]["package_managed"], true);
    assert_eq!(
        listed[0]["first_on_path"], false,
        "the packaged lisa is not what runs, which is the whole finding"
    );
    assert_eq!(listed[1]["first_on_path"], true);
}

/// A nightly schedule on a package box follows the package, so it refuses to be
/// installed on a machine whose package is a different channel — quietly
/// following stable while calling itself nightly is the failure this prevents.
#[test]
fn nightly_install_refuses_a_box_whose_package_is_not_nightly() {
    let machine = TempDir::new().unwrap();
    let brew = brew_box("lisa-canary");

    let output = run(
        &brew.lisa,
        machine.path(),
        &["nightly", "install", "--dry-run"],
    );
    let said = said(&output);

    assert!(!output.status.success(), "{said}");
    assert!(said.contains("channel canary"), "{said}");
    assert!(said.contains("lisa upgrade --channel nightly"), "{said}");
}
