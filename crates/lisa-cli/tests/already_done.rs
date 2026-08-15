//! Black-box fixtures for `lisa already-done`.
//!
//! The command settles a completion two ways — by adopting a seal already in
//! history, or by writing the seal Lisa failed to write — and refuses on
//! anything that is neither. Most of this file is those refusals. The adopting
//! case lives in the plugin crate, where a real completion transaction can
//! produce the commit that counts; the writing case is here, because the whole
//! point of it is that no commit carries the key yet.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use lisa_core::ticket::parse_ticket;
use lisa_core::types::TicketStatus;

fn project() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project with spaces");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(root.join("docs/active/work")).unwrap();
    fs::create_dir_all(root.join(".lisa")).unwrap();
    git(&root, &["init", "--initial-branch=main"]);
    git(&root, &["config", "user.name", "Fixture"]);
    git(&root, &["config", "user.email", "fixture@example.test"]);
    (temp, root)
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_ticket(root: &Path, ticket_id: &str, status: &str) -> PathBuf {
    let path = root
        .join("docs/active/tickets")
        .join(format!("{ticket_id}.md"));
    fs::write(
        &path,
        format!(
            "---\nid: {ticket_id}\ntitle: recovery fixture\ntype: task\nstatus: {status}\npriority: high\nphase: review\n---\n\nFixture\n"
        ),
    )
    .unwrap();
    path
}

fn journal_path(root: &Path) -> PathBuf {
    root.join(".lisa/completion-journal.jsonl")
}

/// A journal whose one aggregate for `ticket_id` folds to the requested final
/// state. Written as raw rows on purpose: these are the exact bytes a real run
/// leaves, and the fold is what has to accept them.
fn write_journal(root: &Path, ticket_id: &str, final_state: &str) {
    let mut rows = format!(
        "{{\"schema_version\":5,\"seal\":\"commit\",\"state\":\"requested\",\"completion_id\":\"{ticket_id}\",\"attempt_id\":\"1\",\"generation\":1,\"prior_phase\":\"review\",\"prior_status\":\"open\"}}\n\
         {{\"schema_version\":5,\"seal\":\"commit\",\"state\":\"command-in-flight\",\"completion_id\":\"{ticket_id}\",\"attempt_id\":\"1\",\"generation\":1,\"correlation_id\":\"c1\",\"reconciliation_deadline_unix_ms\":42}}\n"
    );
    rows.push_str(&match final_state {
        "rejected" => format!(
            "{{\"schema_version\":5,\"seal\":\"commit\",\"state\":\"rejected\",\"completion_id\":\"{ticket_id}\",\"attempt_id\":\"1\",\"generation\":1,\"correlation_id\":\"c1\",\"reason\":\"no changes in the requested include paths\",\"retryability\":\"action-required\"}}\n"
        ),
        "confirmed" => format!(
            "{{\"schema_version\":5,\"seal\":\"commit\",\"state\":\"confirmed\",\"completion_id\":\"{ticket_id}\",\"attempt_id\":\"1\",\"generation\":1,\"correlation_id\":\"c1\",\"commit_id\":\"{}\"}}\n",
            "a".repeat(40)
        ),
        "in-flight" => String::new(),
        other => panic!("unknown fixture state {other}"),
    });
    fs::write(journal_path(root), rows).unwrap();
}

fn already_done(root: &Path, ticket_id: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["already-done", ticket_id, "--path", root.to_str().unwrap()])
        .output()
        .unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Publish the artifacts Lisa admits before it requests a completion.
fn publish_review(root: &Path, ticket_id: &str, disposition: &str) -> PathBuf {
    let dir = root.join("docs/active/work").join(ticket_id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("review.md"),
        "# Review\n\nAll five commits landed.\n",
    )
    .unwrap();
    fs::write(dir.join("review-disposition.json"), disposition).unwrap();
    dir
}

fn head_message(root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "-1", "--format=%B"])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap()
}

/// The field case from 2026-08-13: the work is committed, the review is
/// published, and the *completion commit* is the thing that failed — so no
/// commit anywhere carries the key. Before this the command refused here, on a
/// ticket no other command could finish either.
#[test]
fn a_rejection_whose_seal_never_landed_is_sealed_by_the_command() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-UNSEALED", "blocked");
    publish_review(
        &root,
        "T-UNSEALED",
        "{\"disposition\":\"pass\",\"reason\":null}",
    );
    write_journal(&root, "T-UNSEALED", "rejected");
    fs::write(root.join("src.txt"), "the work itself\n").unwrap();
    git(&root, &["add", "docs", "src.txt"]);
    git(
        &root,
        &["commit", "-m", "T-UNSEALED: the work, with no seal"],
    );

    let output = already_done(&root, "T-UNSEALED");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    // The ticket is done, and its Done bytes are committed rather than left
    // for the operator to notice and commit by hand.
    assert_ticket_status(&ticket, TicketStatus::Done);
    assert!(head_message(&root).contains("Lisa-Completion-Key: "));
    let porcelain = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain", "docs/active/tickets"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&porcelain.stdout).trim().is_empty(),
        "the seal must commit the ticket file it rewrote"
    );
    // And the journal is terminal, so nothing re-attempts it.
    let journal = fs::read_to_string(journal_path(&root)).unwrap();
    assert!(journal.contains("\"state\":\"confirmed\""), "{journal}");

    // Running it again finds its own seal and says the ticket is finished.
    let again = already_done(&root, "T-UNSEALED");
    assert!(!again.status.success());
    assert!(
        stderr(&again).contains("already finished"),
        "{}",
        stderr(&again)
    );
}

/// A reviewer's block outranks this command; Lisa's own recording-failure
/// block is the state it exists to clear.
#[test]
fn a_reviewers_block_is_refused_and_lisas_own_recording_block_is_not() {
    let (_temp, root) = project();
    write_ticket(&root, "T-REVIEWBLOCK", "blocked");
    write_ticket(&root, "T-RECORDFAIL", "blocked");
    publish_review(
        &root,
        "T-REVIEWBLOCK",
        "{\"disposition\":\"block\",\"reason\":\"the tests do not pass\",\"remedy_owner\":\"agent\",\"ask\":\"Make the tests pass.\"}",
    );
    publish_review(
        &root,
        "T-RECORDFAIL",
        "{\"disposition\":\"block\",\"origin\":\"internal-command\",\"reason\":\"Lisa could not record this work.\",\"remedy_owner\":\"operator\",\"ask\":\"Run lisa already-done T-RECORDFAIL.\"}",
    );
    write_journal(&root, "T-REVIEWBLOCK", "rejected");
    git(&root, &["add", "docs"]);
    git(&root, &["commit", "-m", "two parked tickets"]);

    let refused = already_done(&root, "T-REVIEWBLOCK");
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains("the tests do not pass"),
        "{}",
        stderr(&refused)
    );

    write_journal(&root, "T-RECORDFAIL", "rejected");
    let sealed = already_done(&root, "T-RECORDFAIL");
    assert!(
        sealed.status.success(),
        "a recording failure is not a verdict on the work: {}",
        stderr(&sealed)
    );
}

/// The negative fixture. A rejected completion with nothing in history to show
/// for it is not recoverable, and saying so is the whole difference between
/// this command and a lie. An implementation that takes the operator's word
/// fails here.
#[test]
fn a_rejection_with_no_keyed_commit_in_history_declines_and_changes_nothing() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-UNSEALED", "blocked");
    write_journal(&root, "T-UNSEALED", "rejected");
    fs::write(root.join("unrelated.txt"), "content\n").unwrap();
    git(&root, &["add", "unrelated.txt"]);
    git(&root, &["commit", "-m", "an ordinary commit with no key"]);
    let journal_before = fs::read_to_string(journal_path(&root)).unwrap();
    let ticket_before = fs::read_to_string(&ticket).unwrap();

    let output = already_done(&root, "T-UNSEALED");

    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("can't find"), "{message}");
    assert!(message.contains("history"), "{message}");
    assert!(message.contains("Nothing changed"), "{message}");
    assert_eq!(
        fs::read_to_string(journal_path(&root)).unwrap(),
        journal_before
    );
    assert_eq!(fs::read_to_string(&ticket).unwrap(), ticket_before);
    assert_ticket_status(&ticket, TicketStatus::Blocked);
}

#[test]
fn an_unborn_repository_declines_rather_than_failing_on_git() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-UNBORN", "blocked");
    write_journal(&root, "T-UNBORN", "rejected");

    let output = already_done(&root, "T-UNBORN");

    assert!(!output.status.success());
    assert!(stderr(&output).contains("history"), "{}", stderr(&output));
    assert_ticket_status(&ticket, TicketStatus::Blocked);
}

#[test]
fn every_wrong_state_declines_by_name_and_writes_nothing() {
    let (_temp, root) = project();
    write_ticket(&root, "T-CONFIRMED", "done");
    write_ticket(&root, "T-INFLIGHT", "review");
    write_ticket(&root, "T-NOJOURNAL", "blocked");

    write_journal(&root, "T-CONFIRMED", "confirmed");
    let confirmed = already_done(&root, "T-CONFIRMED");
    assert!(!confirmed.status.success());
    assert!(
        stderr(&confirmed).contains("already finished"),
        "{}",
        stderr(&confirmed)
    );

    write_journal(&root, "T-INFLIGHT", "in-flight");
    let in_flight = already_done(&root, "T-INFLIGHT");
    assert!(!in_flight.status.success());
    assert!(
        stderr(&in_flight).contains("isn't stuck"),
        "{}",
        stderr(&in_flight)
    );

    fs::write(journal_path(&root), "").unwrap();
    let no_record = already_done(&root, "T-NOJOURNAL");
    assert!(!no_record.status.success());
    assert!(
        stderr(&no_record).contains("no record"),
        "{}",
        stderr(&no_record)
    );

    let unknown = already_done(&root, "T-NOWHERE");
    assert!(!unknown.status.success());
    assert!(
        stderr(&unknown).contains("couldn't find"),
        "{}",
        stderr(&unknown)
    );
}

/// The anti-brick property at the command's own boundary: a journal the plugin
/// could not replay stops this command too, rather than growing by three rows.
#[test]
fn an_unreplayable_journal_fails_the_command_instead_of_growing() {
    let (_temp, root) = project();
    write_ticket(&root, "T-TORN", "blocked");
    let torn = "{\"schema_version\":5,\"seal\":\"commit\",\"state\":\"requested\",\"completion_id\":\"T-TORN\"";
    fs::write(journal_path(&root), torn).unwrap();

    let output = already_done(&root, "T-TORN");

    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(journal_path(&root)).unwrap(), torn);
}

fn assert_ticket_status(ticket_path: &Path, expected: TicketStatus) {
    assert_eq!(parse_ticket(ticket_path).unwrap().status, expected);
}
