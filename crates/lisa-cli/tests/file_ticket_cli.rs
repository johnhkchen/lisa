//! Black-box fixtures for `lisa file-ticket` (S-065-01).
//!
//! The unit tests beside the module cover what filing decides. These cover the
//! part only the real binary can prove: a draft arrives **on a pipe**, and what
//! comes back out is a board `lisa validate` accepts. Both consumers this word
//! exists for — `steer cross`, and a slot that files when the board runs dry —
//! reach it exactly this way, so a fixture that calls the function directly
//! would be testing the half of the contract nobody uses.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const DRAFT: &str = "---\ntitle: the-board-takes-a-draft\ntype: task\npriority: high\n---\n\n## Context\n\nA program wrote this, not a person with an editor open.\n\n## Acceptance Criteria\n\n- It is on the board.\n";

const STORY: &str = "---\nid: S-065-01\ntitle: a-story-with-room-in-it\ntype: story\nstatus: open\npriority: high\ntickets: [T-065-01-01]\n---\n\n**Scope:** words a person wrote.\n";

const FIRST_TICKET: &str = "---\nid: T-065-01-01\nstory: S-065-01\ntitle: the-one-already-here\ntype: task\nstatus: open\npriority: high\ndepends_on: []\nphase: ready\n---\n\n## Acceptance Criteria\n\n- Already here.\n";

/// Run `lisa` with `draft` on stdin — the way both callers reach this command.
fn file_ticket(root: &Path, draft: &str, extra: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("file-ticket")
        .arg("--path")
        .arg(root)
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn lisa file-ticket");
    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(draft.as_bytes())
        .expect("failed to write the draft to the pipe");
    child
        .wait_with_output()
        .expect("lisa file-ticket never ended")
}

fn lisa(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(args)
        .arg("--path")
        .arg(root)
        .output()
        .unwrap_or_else(|error| panic!("failed to spawn lisa {args:?}: {error}"))
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Parse the one document `--json` prints, insisting it is alone on stdout.
fn document(output: &Output) -> Value {
    let stdout = stdout_of(output);
    let body = stdout.trim_end_matches('\n');
    assert_eq!(
        body.lines().count(),
        1,
        "a document is exactly one line: {stdout:?}"
    );
    serde_json::from_str(body).expect("document must parse")
}

/// A scaffolded board with one story and one ticket already on it.
fn project() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("board");
    fs::create_dir_all(&root).unwrap();
    let init = lisa(&root, &["init", "--no-history"]);
    assert!(
        init.status.success(),
        "lisa init failed: {}",
        stderr_of(&init)
    );
    let stories = root.join("docs/active/stories");
    fs::create_dir_all(&stories).unwrap();
    fs::write(stories.join("S-065-01.md"), STORY).unwrap();
    fs::write(
        root.join("docs/active/tickets/T-065-01-01.md"),
        FIRST_TICKET,
    )
    .unwrap();
    (temp, root)
}

/// The whole word, end to end: a draft on a pipe becomes a ticket, and the
/// board it lands on is one `lisa validate` accepts. Validating *after* is how
/// this used to be found out; the point is that it now cannot fail.
#[test]
fn a_piped_draft_lands_on_a_board_that_still_validates() {
    let (_temp, root) = project();

    let filed = file_ticket(&root, DRAFT, &["--story", "S-065-01", "--json"]);
    assert!(
        filed.status.success(),
        "filing failed: {}",
        stderr_of(&filed)
    );

    let document = document(&filed);
    assert_eq!(document["command"], "file-ticket");
    assert_eq!(document["ok"], true);
    assert!(document["error"].is_null());
    assert_eq!(document["data"]["ticket_id"], "T-065-01-02");
    assert_eq!(
        document["data"]["path"],
        "docs/active/tickets/T-065-01-02.md"
    );
    assert_eq!(document["data"]["story"], "S-065-01");
    assert_eq!(document["data"]["story_list_updated"], true);
    assert_eq!(document["data"]["phase"], "ready");
    assert_eq!(document["data"]["warnings"].as_array().unwrap().len(), 0);

    let written = fs::read_to_string(root.join("docs/active/tickets/T-065-01-02.md")).unwrap();
    assert!(written.contains("id: T-065-01-02"), "{written}");
    assert!(written.contains("story: S-065-01"), "{written}");
    assert!(written.contains("phase: ready"), "{written}");
    assert!(
        written.contains("A program wrote this, not a person with an editor open."),
        "the body is the caller's, unedited: {written}"
    );

    let story = fs::read_to_string(root.join("docs/active/stories/S-065-01.md")).unwrap();
    assert!(
        story.contains("tickets: [T-065-01-01, T-065-01-02]"),
        "{story}"
    );

    let validated = lisa(&root, &["validate"]);
    assert!(
        validated.status.success(),
        "the filed board no longer validates:\n{}\n{}",
        stdout_of(&validated),
        stderr_of(&validated)
    );
    assert!(
        stdout_of(&validated).contains("2 tickets"),
        "{}",
        stdout_of(&validated)
    );
}

/// A refusal is a document too, and it costs the board nothing.
#[test]
fn a_refused_draft_writes_nothing_and_says_why() {
    let (_temp, root) = project();
    let draft =
        "---\ntitle: hopeful\ndepends_on: [T-404-01-01]\n---\n\n## Acceptance Criteria\n\n- No.\n";

    let refused = file_ticket(&root, draft, &["--story", "S-065-01", "--json"]);
    assert_eq!(refused.status.code(), Some(1), "{}", stderr_of(&refused));

    let document = document(&refused);
    assert_eq!(document["ok"], false);
    assert!(document["data"].is_null());
    let message = document["error"]["message"].as_str().unwrap();
    assert!(message.contains("T-404-01-01"), "{message}");
    assert!(message.contains("nothing was filed"), "{message}");

    assert!(!root.join("docs/active/tickets/T-065-01-02.md").exists());
    let story = fs::read_to_string(root.join("docs/active/stories/S-065-01.md")).unwrap();
    assert_eq!(story, STORY, "the story was left exactly as it was");
    assert!(lisa(&root, &["validate"]).status.success());
}

/// Without `--json` the same facts arrive as sentences, and a refusal goes to
/// stderr where a person and a shell both expect it.
#[test]
fn the_prose_path_says_the_same_thing() {
    let (_temp, root) = project();

    let filed = file_ticket(&root, DRAFT, &["--story", "S-065-01"]);
    assert!(filed.status.success(), "{}", stderr_of(&filed));
    let prose = stdout_of(&filed);
    assert!(prose.contains("Filed T-065-01-02"), "{prose}");
    assert!(
        prose.contains("docs/active/tickets/T-065-01-02.md"),
        "{prose}"
    );
    assert!(prose.contains("S-065-01 now lists it."), "{prose}");

    let refused = file_ticket(&root, "not a draft at all\n", &["--story", "S-065-01"]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(stdout_of(&refused).is_empty(), "a refusal writes no answer");
    assert!(
        stderr_of(&refused).contains("no frontmatter"),
        "{}",
        stderr_of(&refused)
    );
}

/// Filing twice in a row is the case a slot produces: two ids, two files, one
/// story list naming both, with nobody reading the folder to guess a number.
#[test]
fn filing_twice_allocates_two_ids_without_the_caller_choosing_either() {
    let (_temp, root) = project();

    let first = document(&file_ticket(
        &root,
        DRAFT,
        &["--story", "S-065-01", "--json"],
    ));
    let second = document(&file_ticket(
        &root,
        DRAFT,
        &["--story", "S-065-01", "--json"],
    ));

    assert_eq!(first["data"]["ticket_id"], "T-065-01-02");
    assert_eq!(second["data"]["ticket_id"], "T-065-01-03");

    let story = fs::read_to_string(root.join("docs/active/stories/S-065-01.md")).unwrap();
    assert!(
        story.contains("tickets: [T-065-01-01, T-065-01-02, T-065-01-03]"),
        "{story}"
    );
    assert!(lisa(&root, &["validate"]).status.success());
}
