//! Black-box snapshots for CLI surfaces that would otherwise be blank or bare.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SETUP_FIRST_LINE: &str = "This folder isn't set up yet. Run: lisa init";
const EMPTY_VALIDATE: &str = "No tickets yet. A ticket is a Markdown file that tells Lisa what work to schedule; put one in docs/active/tickets/, then run `lisa validate` again.\n";

fn lisa(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(args)
        .output()
        .unwrap()
}

fn initialized_empty_project() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project with spaces");
    fs::create_dir(&root).unwrap();
    let output = lisa(&["init", "--no-history", "--path", root.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "init failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (temp, root)
}

fn assert_success(output: &Output, command: &str) {
    assert!(
        output.status.success(),
        "{command} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_ready_ticket(root: &Path) {
    fs::write(
        root.join("docs/active/tickets/T-FIRST.md"),
        "---\nid: T-FIRST\ntitle: first work\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\n## Acceptance Criteria\n\n- The first task is clear.\n",
    )
    .unwrap();
}

#[test]
fn pre_init_project_commands_lead_with_the_setup_sentence() {
    for (command, extra) in [
        ("loop", Some("--dry-run")),
        ("status", None),
        ("validate", None),
        ("doctor", None),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("untouched folder");
        fs::create_dir(&root).unwrap();
        let mut args = vec![command, "--path", root.to_str().unwrap()];
        args.extend(extra);

        let output = lisa(&args);
        assert_eq!(output.status.code(), Some(1), "{command}");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "", "{command}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!(
                "{SETUP_FIRST_LINE}\n\nTechnical detail: Lisa couldn't find .lisa.toml or docs/active/tickets/ in {}.\n",
                root.display()
            ),
            "{command}"
        );
    }
}

#[test]
fn empty_notes_prints_nothing_to_read() {
    let (_temp, root) = initialized_empty_project();
    let output = lisa(&["notes", "--path", root.to_str().unwrap()]);

    assert_success(&output, "lisa notes");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Nothing to read.\n"
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn validate_empty_board_exits_zero_with_ticket_guidance() {
    let (_temp, root) = initialized_empty_project();
    let output = lisa(&["validate", "--path", root.to_str().unwrap()]);

    assert_success(&output, "lisa validate");
    assert_eq!(String::from_utf8_lossy(&output.stdout), EMPTY_VALIDATE);
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn status_empty_board_names_absent_operator_sections() {
    let (_temp, root) = initialized_empty_project();
    let output = lisa(&["status", "--path", root.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_success(&output, "lisa status");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    let waiting = "Waiting on you\nNothing waiting.\n\n";
    let notes = "Notes for you\nNothing to read.\n\n";
    assert!(stdout.starts_with(waiting), "{stdout}");
    assert!(stdout.contains(notes), "{stdout}");
    assert!(stdout.find(waiting).unwrap() < stdout.find(notes).unwrap());
    assert!(stdout.find(notes).unwrap() < stdout.find("No tickets found").unwrap());
}

#[test]
fn initialized_nonempty_validate_snapshot_is_unchanged() {
    let (_temp, root) = initialized_empty_project();
    write_ready_ticket(&root);
    let output = lisa(&["validate", "--path", root.to_str().unwrap()]);

    assert_success(&output, "lisa validate");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "All checks passed. 1 tickets, 1 ready, DAG valid. Run `lisa loop` to start.\nConfig: max_threads=2, session_timeout=3600s\n"
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}
