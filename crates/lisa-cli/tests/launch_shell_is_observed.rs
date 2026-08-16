//! The same board, started from two different shells (T-073-01-02).
//!
//! A run started at the desk and a run started over `ssh` differ in what their
//! panes can do — the login keychain, an `ssh-agent`, `PATH` — and the
//! difference is invisible once the run is going. This measures the record's
//! source end through the real binary: the same project, started twice, with
//! nothing different between the two runs except the environment of the shell
//! that asked for them.
//!
//! `lisa loop --dry-run` is used because it prints the exact layout a real run
//! hands the plugin and starts nothing. The other half of the road — layout in,
//! `.lisa/schedulers/*.alive` out — is measured in the plugin's own tests,
//! where the record is really written.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn project() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("board");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::write(
        root.join(".lisa.toml"),
        format!("version = {:?}\n", env!("CARGO_PKG_VERSION")),
    )
    .unwrap();
    fs::write(
        root.join("docs/active/tickets/T-FIXTURE.md"),
        "---\nid: T-FIXTURE\ntitle: fixture\ntype: task\nstatus: open\npriority: medium\nphase: ready\n---\n\nFixture\n",
    )
    .unwrap();
    (temp, root)
}

/// One dry run, started from a shell described by `shell_env`.
fn started_from(root: &Path, shell_env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lisa"));
    command
        .arg("loop")
        .arg("--dry-run")
        .arg("--path")
        .arg(root)
        .env_remove("SSH_CONNECTION")
        .env_remove("SSH_AUTH_SOCK")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    for (name, value) in shell_env {
        command.env(name, value);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "the dry run failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

/// The reproduction the ticket asks for, with `ssh` simulated by the two
/// variables an `ssh` login is defined by. Nothing about the board, the
/// project, or the command differs between these two runs.
#[test]
fn the_same_board_started_from_two_shells_records_two_different_things() {
    let (_temp, root) = project();

    // A shell at the desk: a launchd agent socket, no ssh connection. This is
    // the machine's own terminal, however it was opened.
    let desk = stdout(&started_from(
        &root,
        &[(
            "SSH_AUTH_SOCK",
            "/private/tmp/com.apple.launchd.7Qh/Listeners",
        )],
    ));

    // An overnight run reached over ssh: a connection, and no agent forwarded
    // with it — which is the shell that could not push.
    let overnight = stdout(&started_from(
        &root,
        &[("SSH_CONNECTION", "192.168.1.44 51988 192.168.1.9 22")],
    ));

    // The test process's stdin is a pipe, so neither run had a terminal — and
    // both say so, which is the third fact.
    assert!(
        desk.contains("launch_shell \"ssh=no,agent=yes,tty=no\""),
        "the desk's run:\n{desk}"
    );
    assert!(
        overnight.contains("launch_shell \"ssh=yes,agent=no,tty=no\""),
        "the overnight run:\n{overnight}"
    );
}

/// Nothing the caller has to remember: no flag sets this, and none is needed
/// to get it.
#[test]
fn the_shell_is_observed_rather_than_asked_for() {
    let (_temp, root) = project();
    let plain = stdout(&started_from(&root, &[]));

    assert!(
        plain.contains("launch_shell \"ssh=no,agent=no,tty=no\""),
        "a shell with nothing in it is still measured, not skipped:\n{plain}"
    );

    let help = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("loop")
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(
        !help.contains("launch-shell") && !help.contains("ssh"),
        "`lisa loop` must not grow a flag the caller has to get right:\n{help}"
    );
}

/// The value of `SSH_AUTH_SOCK` is a live socket path and `SSH_CONNECTION`'s is
/// a pair of addresses. Only their presence is a fact worth keeping; the values
/// must never reach a file that gets committed, printed, or pasted into a
/// ticket.
#[test]
fn no_value_of_any_variable_reaches_the_layout() {
    let (_temp, root) = project();
    let socket = "/private/tmp/com.apple.launchd.7Qh/Listeners";
    let connection = "192.168.1.44 51988 192.168.1.9 22";
    let printed = stdout(&started_from(
        &root,
        &[("SSH_AUTH_SOCK", socket), ("SSH_CONNECTION", connection)],
    ));

    assert!(printed.contains("launch_shell \"ssh=yes,agent=yes,tty=no\""));
    assert!(!printed.contains(socket), "the socket leaked:\n{printed}");
    assert!(
        !printed.contains("192.168"),
        "the address leaked:\n{printed}"
    );
    assert!(!printed.contains("51988"), "the port leaked:\n{printed}");
}
