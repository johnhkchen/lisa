//! Black-box fixtures for the parked-ticket human surface.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use lisa_core::dag::Dag;
use lisa_core::ticket::{parse_ticket, scan_tickets};
use lisa_core::types::TicketStatus;

fn project() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project with spaces");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(root.join("docs/active/work")).unwrap();
    fs::write(root.join("CLAUDE.md"), "# Fixture\n").unwrap();
    (temp, root)
}

fn write_ticket(root: &Path, ticket_id: &str, status: &str) -> PathBuf {
    let path = root
        .join("docs/active/tickets")
        .join(format!("{ticket_id}.md"));
    fs::write(
        &path,
        format!(
            "---\nid: {ticket_id}\ntitle: parked fixture\ntype: task\nstatus: {status}\npriority: high\nphase: review\n---\n\nFixture\n"
        ),
    )
    .unwrap();
    path
}

fn write_disposition(root: &Path, ticket_id: &str, document: &str) {
    let work = root.join("docs/active/work").join(ticket_id);
    fs::create_dir_all(&work).unwrap();
    fs::write(work.join("review-disposition.json"), document).unwrap();
}

fn lisa(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(args)
        .output()
        .unwrap()
}

fn unblock(root: &Path, ticket_id: &str) -> Output {
    lisa(&["unblock", ticket_id, "--path", root.to_str().unwrap()])
}

fn recheck_world(root: &Path) -> Output {
    lisa(&["recheck-world", "--path", root.to_str().unwrap()])
}

fn assert_ticket_status(ticket_path: &Path, expected: TicketStatus) {
    assert_eq!(parse_ticket(ticket_path).unwrap().status, expected);
}

fn assert_ready(root: &Path, ticket_id: &str, expected: bool) {
    let tickets = scan_tickets(root.join("docs/active/tickets")).unwrap();
    let dag = Dag::from_tickets(tickets).unwrap();
    assert_eq!(
        dag.get_ready_tickets()
            .iter()
            .any(|ready| ready == ticket_id),
        expected
    );
}

#[test]
fn status_opens_with_the_operator_ask_and_no_block_internals() {
    let (_temp, root) = project();
    write_ticket(&root, "T-ASK", "blocked");
    write_disposition(
        &root,
        "T-ASK",
        r#"{"disposition":"block","reason":"engineering-only release gate reason","remedy_owner":"operator","ask":"Run the checkout test exactly once.","steps":["hidden implementation step"]}"#,
    );

    let output = lisa(&["status", "--path", root.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert!(
        stdout.starts_with("Waiting on you\nT-ASK  Run the checkout test exactly once.\n\nDAG:")
    );
    assert!(!stdout.contains("engineering-only release gate reason"));
    assert!(!stdout.contains("hidden implementation step"));
    assert!(!stdout.contains("remedy_owner"));
    assert!(!stdout.contains("operator"));
}

#[test]
fn status_explains_that_lisa_checks_world_owned_waiting() {
    let (_temp, root) = project();
    write_ticket(&root, "T-WORLD", "blocked");
    write_disposition(
        &root,
        "T-WORLD",
        r#"{"disposition":"block","reason":"release absent","remedy_owner":"world","ask":"Wait for the release link.","check":"test -f release"}"#,
    );

    let output = lisa(&["status", "--path", root.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.starts_with(
        "Waiting on you\nT-WORLD  Wait for the release link. — Lisa checks on its own.\n\nDAG:"
    ));
}

#[test]
fn failing_check_declines_plainly_and_leaves_the_ticket_waiting() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-FAIL", "blocked");
    write_disposition(
        &root,
        "T-FAIL",
        r#"{"disposition":"block","reason":"link missing","remedy_owner":"operator","ask":"Open the key link.","check":"printf 'the key link still returns 404\nmore tool output\n' >&2; exit 1"}"#,
    );

    let output = unblock(&root, "T-FAIL");

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "That didn't work yet — the key link still returns 404\n"
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Error:"));
    assert_ticket_status(&ticket, TicketStatus::Blocked);
    assert_ready(&root, "T-FAIL", false);
}

#[test]
fn passing_check_reopens_and_the_next_schedule_sees_the_ticket() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-PASS", "blocked");
    fs::write(root.join("release-ready"), "ready\n").unwrap();
    write_disposition(
        &root,
        "T-PASS",
        r#"{"disposition":"block","reason":"release missing","remedy_owner":"world","ask":"Wait for the release.","check":"test -f release-ready"}"#,
    );

    let output = unblock(&root, "T-PASS");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "T-PASS can run again.\n"
    );
    assert_ticket_status(&ticket, TicketStatus::Open);
    assert_ready(&root, "T-PASS", true);
}

#[test]
fn world_owned_passing_check_self_clears_without_an_operator_command() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-AUTO-PASS", "blocked");
    fs::write(root.join("release-ready"), "ready\n").unwrap();
    write_disposition(
        &root,
        "T-AUTO-PASS",
        r#"{"disposition":"block","reason":"release missing","remedy_owner":"world","ask":"Wait for the release.","check":"test -f release-ready"}"#,
    );

    let output = recheck_world(&root);

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "T-AUTO-PASS\n");
    assert_ticket_status(&ticket, TicketStatus::Open);
    assert_ready(&root, "T-AUTO-PASS", true);
}

#[test]
fn world_owned_failing_check_stays_parked_without_churn() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-AUTO-FAIL", "blocked");
    write_disposition(
        &root,
        "T-AUTO-FAIL",
        r#"{"disposition":"block","reason":"release missing","remedy_owner":"world","ask":"Wait for the release.","check":"exit 1"}"#,
    );
    let before = fs::read(&ticket).unwrap();

    let output = recheck_world(&root);

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_eq!(fs::read(&ticket).unwrap(), before);
    assert_ticket_status(&ticket, TicketStatus::Blocked);
    assert_ready(&root, "T-AUTO-FAIL", false);
}

#[test]
fn automatic_recheck_ignores_operator_owned_passing_checks() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-AUTO-OPERATOR", "blocked");
    fs::write(root.join("operator-ready"), "ready\n").unwrap();
    write_disposition(
        &root,
        "T-AUTO-OPERATOR",
        r#"{"disposition":"block","reason":"manual approval missing","remedy_owner":"operator","ask":"Approve the release.","check":"test -f operator-ready"}"#,
    );

    let output = recheck_world(&root);

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_ticket_status(&ticket, TicketStatus::Blocked);
    assert_ready(&root, "T-AUTO-OPERATOR", false);
}

#[test]
fn automatic_recheck_write_attempt_is_disposable_and_cannot_reopen() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-AUTO-WRITE", "blocked");
    write_disposition(
        &root,
        "T-AUTO-WRITE",
        r#"{"disposition":"block","reason":"write probe","remedy_owner":"world","ask":"Wait for the marker.","check":"touch must-not-exist"}"#,
    );

    let output = recheck_world(&root);

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert!(!root.join("must-not-exist").exists());
    assert_ticket_status(&ticket, TicketStatus::Blocked);
    assert_ready(&root, "T-AUTO-WRITE", false);
}

#[test]
fn automatic_recheck_timeout_is_bounded_and_cannot_reopen() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-AUTO-TIMEOUT", "blocked");
    write_disposition(
        &root,
        "T-AUTO-TIMEOUT",
        r#"{"disposition":"block","reason":"slow probe","remedy_owner":"world","ask":"Wait for the probe.","check":"sleep 30"}"#,
    );
    let started = Instant::now();

    let output = recheck_world(&root);

    assert!(output.status.success());
    assert!(started.elapsed() >= Duration::from_secs(4));
    assert!(started.elapsed() < Duration::from_secs(8));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_ticket_status(&ticket, TicketStatus::Blocked);
    assert_ready(&root, "T-AUTO-TIMEOUT", false);
}

#[test]
fn absent_check_reopens_without_trying_to_remediate() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-NOCHECK", "blocked");
    write_disposition(
        &root,
        "T-NOCHECK",
        r#"{"disposition":"block","reason":"manual test missing","remedy_owner":"operator","ask":"Run the manual test."}"#,
    );

    let output = unblock(&root, "T-NOCHECK");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "T-NOCHECK can run again.\n"
    );
    assert_ticket_status(&ticket, TicketStatus::Open);
    assert_ready(&root, "T-NOCHECK", true);
}

#[test]
fn attempted_write_is_disposable_reported_plainly_and_does_not_reopen() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-WRITE", "blocked");
    write_disposition(
        &root,
        "T-WRITE",
        r#"{"disposition":"block","reason":"write probe","remedy_owner":"operator","ask":"Check the marker.","check":"touch must-not-exist"}"#,
    );

    let output = unblock(&root, "T-WRITE");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.starts_with("That didn't work yet — "));
    assert_eq!(stderr.lines().count(), 1);
    assert!(!stderr.contains("Error:"));
    assert!(!root.join("must-not-exist").exists());
    assert_ticket_status(&ticket, TicketStatus::Blocked);
    assert_ready(&root, "T-WRITE", false);
}

#[test]
fn unknown_open_and_missing_remedy_cases_use_pinned_plain_copy() {
    let (_temp, root) = project();
    write_ticket(&root, "T-OPEN", "open");
    write_ticket(&root, "T-NO-REMEDY", "blocked");

    let unknown = unblock(&root, "T-UNKNOWN");
    assert!(!unknown.status.success());
    assert_eq!(
        String::from_utf8_lossy(&unknown.stderr),
        "I couldn't find T-UNKNOWN.\n"
    );

    let open = unblock(&root, "T-OPEN");
    assert!(!open.status.success());
    assert_eq!(
        String::from_utf8_lossy(&open.stderr),
        "T-OPEN isn't waiting.\n"
    );

    let missing = unblock(&root, "T-NO-REMEDY");
    assert!(!missing.status.success());
    assert_eq!(
        String::from_utf8_lossy(&missing.stderr),
        "I couldn't find what T-NO-REMEDY is waiting for.\n"
    );
}
