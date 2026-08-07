//! Black-box fixtures for the parked-ticket human surface.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use lisa_core::dag::Dag;
use lisa_core::parking::LEGACY_BLOCK_ASK;
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
    unblock_with(root, ticket_id, &[])
}

fn unblock_with(root: &Path, ticket_id: &str, extra: &[&str]) -> Output {
    let mut args = vec!["unblock", ticket_id, "--path", root.to_str().unwrap()];
    args.extend_from_slice(extra);
    lisa(&args)
}

/// Every provenance row this project has written, in order. An absent ledger is
/// no rows, not a failure — an ordinary unblock never creates one.
fn provenance_rows(root: &Path) -> Vec<serde_json::Value> {
    let Ok(body) = fs::read_to_string(root.join(".lisa/provenance.jsonl")) else {
        return Vec::new();
    };
    body.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn check_override_rows(root: &Path) -> Vec<serde_json::Value> {
    provenance_rows(root)
        .into_iter()
        .filter(|row| row["record_type"] == "check-override")
        .collect()
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
fn status_opens_with_the_operator_ask_then_the_reviewers_reason() {
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
    assert!(stdout.starts_with(
        "Waiting on you\nT-ASK  Run the checkout test exactly once.\n       Reviewer's note: engineering-only release gate reason\n\nNotes for you\nNothing to read.\n\nDAG:"
    ));
    assert!(
        stdout.find("Run the checkout test exactly once.").unwrap()
            < stdout.find("engineering-only release gate reason").unwrap()
    );
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
        "Waiting on you\nT-WORLD  Wait for the release link. — Lisa checks on its own.\n       Reviewer's note: release absent\n\nNotes for you\nNothing to read.\n\nDAG:"
    ));
}

#[test]
fn status_legacy_field_block_never_opens_with_the_raw_reason() {
    const FIELD_REASON: &str = "The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant bypassed the old binary through managed mode instead of recording the required recovery through Lisa's error strings; John must either provide conforming reruns or explicitly amend both acceptance requirements before Review can pass.";
    let (_temp, root) = project();
    write_ticket(&root, "T-046-06-03", "blocked");
    write_disposition(
        &root,
        "T-046-06-03",
        &serde_json::json!({
            "disposition": "block",
            "reason": FIELD_REASON,
        })
        .to_string(),
    );

    let output = lisa(&["status", "--path", root.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.starts_with(&format!(
        "Waiting on you\nT-046-06-03  {LEGACY_BLOCK_ASK}\n       Reviewer's note: {FIELD_REASON}\n\nNotes for you\nNothing to read.\n\nDAG:"
    )));
    assert!(stdout.find(LEGACY_BLOCK_ASK).unwrap() < stdout.find(FIELD_REASON).unwrap());
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
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    // Lisa's own finding leads, and the check's words stay the check's.
    assert_eq!(
        stderr.lines().next().unwrap(),
        "That didn't work yet — the check ran and did not pass."
    );
    assert!(stderr.contains("  the check wrote to stderr:\n    the key link still returns 404\n    more tool output\n"), "{stderr}");
    assert!(stderr.contains("  exit code: 1\n"), "{stderr}");
    assert!(!stderr.contains("Error:"));
    assert_ticket_status(&ticket, TicketStatus::Blocked);
    assert_ready(&root, "T-FAIL", false);
}

/// Criterion 1: a check-caused decline shows its work — the command as
/// recorded, the folder it ran in, the exit code, and both streams labelled.
#[test]
fn a_declined_check_reports_the_command_the_directory_the_code_and_both_streams() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-SHOW", "blocked");
    write_disposition(
        &root,
        "T-SHOW",
        r#"{"disposition":"block","reason":"probe missing","remedy_owner":"operator","ask":"Run the probe.","check":"pwd -P; printf 'on the error stream\n' >&2; exit 3"}"#,
    );

    let output = unblock(&root, "T-SHOW");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("  what ran:  pwd -P; printf 'on the error stream ' >&2; exit 3\n"),
        "{stderr}"
    );
    assert!(stderr.contains("  exit code: 3\n"), "{stderr}");
    assert!(
        stderr.contains("  the check wrote to stderr:\n    on the error stream\n"),
        "{stderr}"
    );
    // The check printed its own working directory, and the report names the
    // same one.
    let seen = stderr
        .lines()
        .skip_while(|line| !line.starts_with("  the check wrote to stdout:"))
        .nth(1)
        .expect("the check's own cwd is shown")
        .trim()
        .to_string();
    let reported = stderr
        .lines()
        .find_map(|line| line.strip_prefix("  ran in:    "))
        .expect("the report names a directory")
        .to_string();
    assert!(
        seen == reported || seen.ends_with(&reported),
        "check ran in {seen}, report named {reported}"
    );
    assert_ticket_status(&ticket, TicketStatus::Blocked);
}

/// Criteria 2 and 6: the field line, reproduced and then improved. The script
/// exits 2 meaning "I could not look", and Lisa no longer relays that as a
/// verdict on work the operator has already done.
#[test]
fn a_check_that_cannot_look_reads_as_inconclusive_not_as_a_verdict() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-010-03", "blocked");
    write_disposition(
        &root,
        "T-010-03",
        r#"{"disposition":"block","reason":"mobile sweep pending","remedy_owner":"operator","ask":"Run the mobile check.","check":"printf 'No build at dist/. Run: npm run build\n' >&2; exit 2"}"#,
    );

    let output = unblock(&root, "T-010-03");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let header = stderr.lines().next().unwrap();

    assert!(!output.status.success());
    assert_eq!(
        header,
        "Lisa can't tell yet — the check stopped before it could look, so this isn't a judgement on your work."
    );
    // The 0.4.4 field output, which must not come back.
    assert!(!stderr.contains("That didn't work yet — No build at dist/"));
    assert!(!header.contains("No build at dist/"));
    assert!(
        stderr
            .contains("  what ran:  printf 'No build at dist/. Run: npm run build ' >&2; exit 2\n"),
        "{stderr}"
    );
    assert!(stderr.contains("  ran in:    /"), "{stderr}");
    assert!(stderr.contains("  exit code: 2\n"), "{stderr}");
    assert!(
        stderr
            .contains("  the check wrote to stderr:\n    No build at dist/. Run: npm run build\n"),
        "{stderr}"
    );
    assert!(
        stderr
            .trim_end()
            .ends_with("lisa unblock T-010-03 --override-check"),
        "{stderr}"
    );
    assert_ticket_status(&ticket, TicketStatus::Blocked);
}

/// Criterion 5: the sanitizer still covers everything shown.
#[test]
fn escape_sequences_and_tabs_are_stripped_from_everything_shown() {
    let (_temp, root) = project();
    write_ticket(&root, "T-ANSI", "blocked");
    write_disposition(
        &root,
        "T-ANSI",
        r#"{"disposition":"block","reason":"colourful probe","remedy_owner":"operator","ask":"Run the probe.","check":"printf '\u001b[31mred\tcolumns\u001b[0m\n' >&2; printf '\u001b[1mbold out\n'; exit 1"}"#,
    );

    let output = unblock(&root, "T-ANSI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(!stderr.contains('\u{1b}'), "an escape reached the terminal");
    assert!(!stderr.contains('\t'), "a tab reached the terminal");
    assert!(
        stderr.contains("  the check wrote to stderr:\n    red columns\n"),
        "{stderr}"
    );
    assert!(
        stderr.contains("  the check wrote to stdout:\n    bold out\n"),
        "{stderr}"
    );
}

/// Criteria 3 and 4: the way through always exists, and using it leaves a mark.
#[test]
fn override_check_reopens_and_leaves_a_record() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-FORCED", "blocked");
    write_disposition(
        &root,
        "T-FORCED",
        r#"{"disposition":"block","reason":"mobile sweep pending","remedy_owner":"operator","ask":"Run the mobile check.","check":"printf 'No build at dist/. Run: npm run build\n' >&2; exit 2"}"#,
    );

    // Without the flag: declined, still waiting, and nothing recorded.
    let declined = unblock(&root, "T-FORCED");
    assert!(!declined.status.success());
    assert_ticket_status(&ticket, TicketStatus::Blocked);
    assert!(
        check_override_rows(&root).is_empty(),
        "an ordinary unblock must write no override record"
    );

    // With it: reopened, exit 0, and a receipt naming who overrode what.
    let forced = unblock_with(&root, "T-FORCED", &["--override-check"]);
    assert!(forced.status.success());
    assert_eq!(String::from_utf8_lossy(&forced.stderr), "");
    assert_eq!(
        String::from_utf8_lossy(&forced.stdout),
        "T-FORCED can run again — you overrode its check.\n"
    );
    assert_ticket_status(&ticket, TicketStatus::Open);
    assert_ready(&root, "T-FORCED", true);

    let rows = check_override_rows(&root);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["ticket_id"], "T-FORCED");
    assert_eq!(row["actor"], "operator");
    assert_eq!(row["result"], "inconclusive");
    assert_eq!(row["exit_code"], 2);
    assert_eq!(
        row["check"],
        "printf 'No build at dist/. Run: npm run build\n' >&2; exit 2"
    );
    assert_eq!(row["observed"][0], "No build at dist/. Run: npm run build");
    assert!(row["directory"].as_str().unwrap().starts_with('/'));
}

/// A passing check needs no override, and using the flag anyway records
/// nothing — the receipt marks a decision, not a command line.
#[test]
fn override_check_records_nothing_when_the_check_passes() {
    let (_temp, root) = project();
    let ticket = write_ticket(&root, "T-CLEAN", "blocked");
    fs::write(root.join("release-ready"), "ready\n").unwrap();
    write_disposition(
        &root,
        "T-CLEAN",
        r#"{"disposition":"block","reason":"release missing","remedy_owner":"world","ask":"Wait for the release.","check":"test -f release-ready"}"#,
    );

    let output = unblock_with(&root, "T-CLEAN", &["--override-check"]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "T-CLEAN can run again.\n"
    );
    assert_ticket_status(&ticket, TicketStatus::Open);
    assert!(check_override_rows(&root).is_empty());
}

/// The override covers one gate: a check that declined. It is not a skeleton
/// key for the declines that having done the ask would not clear.
#[test]
fn override_check_does_not_bypass_the_non_check_declines() {
    let (_temp, root) = project();
    write_ticket(&root, "T-OPEN", "open");
    write_ticket(&root, "T-NO-REMEDY", "blocked");

    let open = unblock_with(&root, "T-OPEN", &["--override-check"]);
    assert!(!open.status.success());
    assert_eq!(
        String::from_utf8_lossy(&open.stderr),
        "T-OPEN isn't waiting.\n"
    );

    let missing = unblock_with(&root, "T-NO-REMEDY", &["--override-check"]);
    assert!(!missing.status.success());
    assert_eq!(
        String::from_utf8_lossy(&missing.stderr),
        "I couldn't find what T-NO-REMEDY is waiting for.\n"
    );
    assert!(check_override_rows(&root).is_empty());
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

/// Unblock's one new refusal, and the boundary of it.
///
/// A ticket Lisa has stopped trying to record is not waiting on its review, so
/// reopening it would only hand it a session to fail in again. Every ticket
/// under the bound — which is every ordinary parked ticket — keeps unblock's
/// existing behavior exactly.
#[test]
fn unblock_steps_aside_only_for_a_completion_lisa_stopped_recording() {
    let (_temp, root) = project();
    let stuck = write_ticket(&root, "T-STUCK", "blocked");
    let ordinary = write_ticket(&root, "T-ORDINARY", "blocked");
    for ticket_id in ["T-STUCK", "T-ORDINARY"] {
        write_disposition(
            &root,
            ticket_id,
            r#"{"disposition":"block","origin":"internal-command","reason":"Lisa could not record this ticket's finished work.","remedy_owner":"operator","ask":"Run the recovery command."}"#,
        );
    }
    fs::create_dir_all(root.join(".lisa")).unwrap();
    // T-STUCK has spent both generations; T-ORDINARY has spent one.
    fs::write(
        root.join(".lisa/completion-journal.jsonl"),
        format!(
            "{}{}",
            action_required_generations("T-STUCK", 2),
            action_required_generations("T-ORDINARY", 1)
        ),
    )
    .unwrap();

    let stopped = unblock(&root, "T-STUCK");
    assert!(!stopped.status.success());
    let message = String::from_utf8_lossy(&stopped.stderr);
    assert!(message.contains("lisa already-done T-STUCK"), "{message}");
    assert!(message.contains("not because of the review"), "{message}");
    assert_ticket_status(&stuck, TicketStatus::Blocked);

    let under_the_bound = unblock(&root, "T-ORDINARY");
    assert!(under_the_bound.status.success());
    assert_eq!(
        String::from_utf8_lossy(&under_the_bound.stdout),
        "T-ORDINARY can run again.\n"
    );
    assert_ticket_status(&ordinary, TicketStatus::Open);
}

/// Journal rows folding to `count` action-required generations for one ticket.
fn action_required_generations(ticket_id: &str, count: u64) -> String {
    (1..=count)
        .map(|generation| {
            format!(
                "{{\"schema_version\":5,\"seal\":\"commit\",\"state\":\"requested\",\"completion_id\":\"{ticket_id}\",\"attempt_id\":\"1\",\"generation\":{generation},\"prior_phase\":\"review\",\"prior_status\":\"open\"}}\n\
                 {{\"schema_version\":5,\"seal\":\"commit\",\"state\":\"command-in-flight\",\"completion_id\":\"{ticket_id}\",\"attempt_id\":\"1\",\"generation\":{generation},\"correlation_id\":\"c{generation}\",\"reconciliation_deadline_unix_ms\":42}}\n\
                 {{\"schema_version\":5,\"seal\":\"commit\",\"state\":\"rejected\",\"completion_id\":\"{ticket_id}\",\"attempt_id\":\"1\",\"generation\":{generation},\"correlation_id\":\"c{generation}\",\"reason\":\"no changes in the requested include paths\",\"retryability\":\"action-required\"}}\n"
            )
        })
        .collect()
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
