//! A board run on a host with no terminal (T-066-01-02).
//!
//! The test process has no controlling terminal, which is the condition this
//! is about: `cargo test` is as headless as a Codespace reached by
//! `gh codespace ssh`. So the refusal, the launch, and the terminal the client
//! ends up with are all measured here rather than described.
//!
//! Zellij is stubbed. What the stub records — whether its standard descriptors
//! are a terminal, which one, and how big — is the whole question: an agent
//! pane is a child of that client, and a client with no terminal never starts
//! to open one.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    bin: PathBuf,
    home: PathBuf,
    /// Where the stubbed Zellij writes down what it was handed.
    record: PathBuf,
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

/// A project Lisa will agree to start a run on, with every external tool
/// stubbed: nothing here reaches the network, a real Zellij, or a real agent.
fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    let home = temp.path().join("home");
    let record = temp.path().join("zellij-was-handed.txt");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(root.join("docs/active/stories")).unwrap();
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&home).unwrap();

    fs::write(root.join(".codex/hooks.json"), "{}\n").unwrap();
    // Claude's one-time confirmation, which Lisa refuses to grant for the
    // operator and which is not what this test is about.
    fs::write(
        home.join(".claude.json"),
        "{\"bypassPermissionsModeAccepted\": true}\n",
    )
    .unwrap();

    let zellij = bin.join("zellij");
    fs::write(
        root.join(".lisa.toml"),
        format!(
            "version = {:?}\n\n[runtime]\nzellij = {:?}\n\n[guards]\ncompletion = \"journal\"\n\n\
             [agent]\nclient = \"claude\"\n",
            env!("CARGO_PKG_VERSION"),
            zellij.to_str().unwrap(),
        ),
    )
    .unwrap();

    write_executable(
        &bin.join("git"),
        "#!/bin/sh\nprintf '%s\\n' 'git version 2.50.0'\nexit 0\n",
    );
    write_executable(
        &bin.join("claude"),
        "#!/bin/sh\nprintf '%s\\n' 'claude 1.0.0'\nexit 0\n",
    );
    // Everything the launch hands the client, written where the pty cannot
    // swallow it: the stub's own stdout *is* the terminal under test.
    write_executable(
        &zellij,
        r#"#!/bin/sh
case "${1:-}" in
    --version) printf '%s\n' 'zellij 0.44.3'; exit 0 ;;
    # What this machine is holding, stated by whoever ran the test. Lisa asks
    # twice — once to name this run's session, once to find out whether the
    # scheduler a record names still exists — and both answers come from here.
    list-sessions)
        if [ -n "${LISA_STUB_SESSIONS:-}" ]; then
            printf '%s\n' "$LISA_STUB_SESSIONS"; exit 0
        fi
        exit 1 ;;
esac
# Asked before the redirect below replaces this shell's stdout, which would
# otherwise answer for the file rather than for what Lisa handed the client.
if [ -t 0 ]; then stdin=terminal; else stdin='not a terminal'; fi
if [ -t 1 ]; then stdout=terminal; else stdout='not a terminal'; fi
if [ -t 2 ]; then stderr=terminal; else stderr='not a terminal'; fi
name=$(tty 2>&1 || true)
size=$(stty size 2>&1 || true)
{
    printf 'argv: %s\n' "$*"
    printf 'stdin: %s\n' "$stdin"
    printf 'stdout: %s\n' "$stdout"
    printf 'stderr: %s\n' "$stderr"
    printf 'tty: %s\n' "$name"
    printf 'size: %s\n' "$size"
} > "$LISA_STUB_RECORD"
printf 'a dashboard nobody is reading\n'
exit 0
"#,
    );

    Fixture {
        _temp: temp,
        root,
        bin,
        home,
        record,
    }
}

impl Fixture {
    fn run_loop(&self, extra: &[&str]) -> Output {
        self.run_loop_holding(extra, "")
    }

    /// A run started while the stubbed Zellij reports these sessions.
    fn run_loop_holding(&self, extra: &[&str], sessions: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_lisa"))
            .arg("loop")
            .args(extra)
            .args(["--path", self.root.to_str().unwrap()])
            .env("LISA_STUB_SESSIONS", sessions)
            // The stubs lead, so they are what Lisa finds; the system paths
            // follow only so the stubbed Zellij can ask `tty` and `stty` what
            // it was handed.
            .env("PATH", format!("{}:/usr/bin:/bin", self.bin.display()))
            .env("HOME", &self.home)
            .env("LISA_STUB_RECORD", &self.record)
            .output()
            .unwrap()
    }

    fn handed_to_zellij(&self) -> String {
        fs::read_to_string(&self.record).unwrap_or_default()
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// The measured failure this is all about, on a host that has no terminal at
/// all:
///
/// ```text
/// could not enable raw mode: Os { code: 6, message: "Device not configured" }
/// Error: zellij exited with status: exit status: 101
/// ```
///
/// A caller now hears it from Lisa, before Zellij is started, with the word
/// that starts a run anyway in the same sentence.
#[test]
fn a_loop_started_where_there_is_no_terminal_names_the_way_through() {
    let fixture = fixture();
    let output = fixture.run_loop(&[]);
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(!output.status.success(), "the run should not have started");
    assert!(stderr.contains("no terminal"), "{stderr}");
    assert!(stderr.contains("lisa loop --headless"), "{stderr}");
    assert!(stderr.contains("lisa status"), "{stderr}");
    assert!(stderr.contains("zellij kill-session"), "{stderr}");
    assert!(
        !stderr.contains("raw mode"),
        "the operator should not have to read Zellij's error: {stderr}"
    );
    assert_eq!(
        fixture.handed_to_zellij(),
        "",
        "nothing should have been started"
    );
}

/// The point of the whole ticket: the run starts, and the client that gives
/// every agent its pane is holding a real terminal — on a process that has
/// none.
#[test]
fn a_headless_run_hands_zellij_a_terminal_of_lisas_own() {
    let fixture = fixture();
    let output = fixture.run_loop(&["--headless"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let handed = fixture.handed_to_zellij();
    assert!(
        handed.contains("stdin: terminal")
            && handed.contains("stdout: terminal")
            && handed.contains("stderr: terminal"),
        "the client was not given a terminal:\n{handed}"
    );
    assert!(
        handed.contains("--new-session-with-layout"),
        "the same launch as always, on a terminal Lisa opened:\n{handed}"
    );
    assert!(
        handed.contains("size: 50 200"),
        "an unsized pty is 0x0 and Zellij lays panes out against it:\n{handed}"
    );

    // What is lost, said out loud, with what to read instead.
    assert!(stdout.contains("no dashboard is being drawn"), "{stdout}");
    assert!(stdout.contains("lisa status --json"), "{stdout}");
    assert!(
        stdout.contains("hooks write the same signals"),
        "a pane is still a Zellij pane: {stdout}"
    );
}

/// `T-065-01-03`'s refusal has to hold where nobody is looking. Two schedulers
/// split the signals the panes write between them, and on a headless host
/// there is no dashboard to notice it in.
#[test]
fn a_headless_run_still_refuses_to_be_the_second_scheduler_on_a_board() {
    let fixture = fixture();
    let now = now_secs();
    lisa_core::schedulers::write_record(
        &lisa_core::schedulers::roster_dir(&fixture.root),
        &lisa_core::schedulers::SchedulerRecord::new(
            "lisa-9c1f4b0a",
            Some("lisa".to_string()),
            Some(9450),
            now - 600,
            now - 3,
            5,
        ),
    )
    .unwrap();

    // The session that record names is still up, which is what makes it a
    // scheduler rather than a record: since T-070-01-01 a run whose Zellij
    // server this machine cannot find no longer holds a board.
    let output = fixture.run_loop_holding(&["--headless"], "lisa [Created 6m 18s ago] (current)");
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(!output.status.success(), "a second scheduler started");
    assert!(stderr.contains("already running on this board"), "{stderr}");
    assert!(stderr.contains("zellij kill-session lisa"), "{stderr}");
    assert_eq!(
        fixture.handed_to_zellij(),
        "",
        "no second client should have been started"
    );
}
