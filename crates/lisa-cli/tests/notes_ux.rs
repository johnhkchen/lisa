//! Black-box lifecycle fixtures for the deferred completion-note queue.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use lisa_core::dag::Dag;
use lisa_core::notes::NoteAcknowledgmentRecord;
use lisa_core::ticket::scan_tickets;

const SUMMARY: &str = "The recorded measurement and criterion text disagree.";
const SECOND_SUMMARY: &str = "The later measurement also needs review.";
const CRITERION: &str = "approximately 200 MiB";
const EVIDENCE: &str = "docs/active/work/T-046-06-03/review.md#measurement";

fn project() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project with spaces");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::write(
        root.join("docs/active/tickets/T-046-06-03.md"),
        "---\nid: T-046-06-03\ntitle: measurement fixture\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n\nFixture\n",
    )
    .unwrap();
    (temp, root)
}

fn write_note_journal(root: &Path) {
    write_note_journal_generations(root, &[("attempt-field", 1, SUMMARY)]);
}

fn write_note_journal_generations(root: &Path, notes: &[(&str, u64, &str)]) {
    fs::create_dir_all(root.join(".lisa")).unwrap();
    let mut journal = String::new();
    for (attempt_id, generation, summary) in notes {
        let note = serde_json::json!({
            "criterion_quote": CRITERION,
            "evidence_citation": EVIDENCE,
            "summary": summary,
        });
        let requested = serde_json::json!({
            "schema_version": 5,
            "seal": "commit",
            "state": "requested",
            "completion_id": "T-046-06-03",
            "attempt_id": attempt_id,
            "generation": generation,
            "prior_phase": "review",
            "prior_status": "in-progress",
            "note": note,
        });
        let confirmed = serde_json::json!({
            "schema_version": 5,
            "seal": "commit",
            "state": "confirmed",
            "completion_id": "T-046-06-03",
            "attempt_id": attempt_id,
            "generation": generation,
            "correlation_id": format!("correlation-{generation}"),
            "commit_id": "0123456789abcdef",
            "note": note,
        });
        journal.push_str(&format!("{requested}\n{confirmed}\n"));
    }
    fs::write(root.join(".lisa/completion-journal.jsonl"), journal).unwrap();
}

fn write_waiting_ticket(root: &Path) {
    fs::write(
        root.join("docs/active/tickets/T-WAIT.md"),
        "---\nid: T-WAIT\ntitle: waiting fixture\ntype: task\nstatus: blocked\npriority: high\nphase: review\n---\n\nFixture\n",
    )
    .unwrap();
    let work = root.join("docs/active/work/T-WAIT");
    fs::create_dir_all(&work).unwrap();
    fs::write(
        work.join("review-disposition.json"),
        r#"{"disposition":"block","reason":"Release evidence is absent.","remedy_owner":"operator","ask":"Run the release check."}"#,
    )
    .unwrap();
}

fn lisa(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(args)
        .output()
        .unwrap()
}

fn notes(root: &Path) -> Output {
    lisa(&["notes", "--path", root.to_str().unwrap()])
}

#[test]
fn list_ack_and_restart_processes_follow_durable_lifecycle() {
    let (_temp, root) = project();
    write_note_journal(&root);
    let ticket_path = root.join("docs/active/tickets/T-046-06-03.md");
    let ticket_before = fs::read(&ticket_path).unwrap();
    let ready_before = Dag::from_tickets(scan_tickets(root.join("docs/active/tickets")).unwrap())
        .unwrap()
        .get_ready_tickets();

    let listed = notes(&root);
    let stdout = String::from_utf8_lossy(&listed.stdout);
    assert!(listed.status.success());
    assert!(stdout.starts_with(&format!("Notes for you (1)\nT-046-06-03  {SUMMARY}\n")));
    assert!(stdout.find(SUMMARY).unwrap() < stdout.find(CRITERION).unwrap());
    assert!(stdout.find(CRITERION).unwrap() < stdout.find(EVIDENCE).unwrap());

    let acknowledged = lisa(&[
        "notes",
        "--path",
        root.to_str().unwrap(),
        "ack",
        "T-046-06-03",
    ]);
    assert!(acknowledged.status.success());
    assert_eq!(
        String::from_utf8_lossy(&acknowledged.stdout),
        "T-046-06-03 acknowledged.\n"
    );
    let row: NoteAcknowledgmentRecord = serde_json::from_str(
        fs::read_to_string(root.join(".lisa/provenance.jsonl"))
            .unwrap()
            .trim(),
    )
    .unwrap();
    assert_eq!(row.ticket_id, "T-046-06-03");
    assert_eq!(row.attempt_id, "attempt-field");
    assert_eq!(row.generation, 1);

    let after = notes(&root);
    assert!(after.status.success());
    assert_eq!(String::from_utf8_lossy(&after.stdout), "Nothing to read.\n");
    let duplicate = lisa(&[
        "notes",
        "--path",
        root.to_str().unwrap(),
        "ack",
        "T-046-06-03",
    ]);
    assert!(duplicate.status.success());
    assert_eq!(
        String::from_utf8_lossy(&duplicate.stdout),
        "Nothing to read for T-046-06-03.\n"
    );
    assert_eq!(String::from_utf8_lossy(&duplicate.stderr), "");
    assert_eq!(
        fs::read_to_string(root.join(".lisa/provenance.jsonl"))
            .unwrap()
            .lines()
            .count(),
        1
    );

    assert_eq!(fs::read(&ticket_path).unwrap(), ticket_before);
    let ready_after = Dag::from_tickets(scan_tickets(root.join("docs/active/tickets")).unwrap())
        .unwrap()
        .get_ready_tickets();
    assert_eq!(ready_after, ready_before);
}

#[test]
fn two_active_notes_are_labeled_and_bare_ack_drains_oldest_first() {
    let (_temp, root) = project();
    write_note_journal_generations(
        &root,
        &[("attempt-a", 1, SUMMARY), ("attempt-b", 2, SECOND_SUMMARY)],
    );

    let listed = notes(&root);
    assert!(listed.status.success());
    let stdout = String::from_utf8_lossy(&listed.stdout);
    assert!(stdout.starts_with(&format!(
        "Notes for you (2)\nT-046-06-03  Generation 1  {SUMMARY}\n"
    )));
    assert!(stdout.contains(&format!("T-046-06-03  Generation 2  {SECOND_SUMMARY}\n")));

    let first = lisa(&[
        "notes",
        "--path",
        root.to_str().unwrap(),
        "ack",
        "T-046-06-03",
    ]);
    assert!(first.status.success());
    assert_eq!(
        String::from_utf8_lossy(&first.stdout),
        "Marked the oldest note read — 1 more remains.\n"
    );

    let after_first = notes(&root);
    assert!(after_first.status.success());
    let stdout = String::from_utf8_lossy(&after_first.stdout);
    assert!(stdout.starts_with(&format!(
        "Notes for you (1)\nT-046-06-03  {SECOND_SUMMARY}\n"
    )));
    assert!(!stdout.contains("Generation 2"));

    let second = lisa(&[
        "notes",
        "--path",
        root.to_str().unwrap(),
        "ack",
        "T-046-06-03",
    ]);
    assert!(second.status.success());
    assert_eq!(
        String::from_utf8_lossy(&second.stdout),
        "T-046-06-03 acknowledged.\n"
    );

    let after_second = notes(&root);
    assert!(after_second.status.success());
    assert_eq!(
        String::from_utf8_lossy(&after_second.stdout),
        "Nothing to read.\n"
    );

    let records: Vec<NoteAcknowledgmentRecord> =
        fs::read_to_string(root.join(".lisa/provenance.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].generation, 1);
    assert_eq!(records[0].attempt_id, "attempt-a");
    assert_eq!(records[1].generation, 2);
    assert_eq!(records[1].attempt_id, "attempt-b");
}

#[test]
fn generation_flag_targets_exactly_and_unknown_names_listed_generations() {
    let (_temp, root) = project();
    write_note_journal_generations(
        &root,
        &[("attempt-a", 1, SUMMARY), ("attempt-b", 2, SECOND_SUMMARY)],
    );

    let unknown = lisa(&[
        "notes",
        "--path",
        root.to_str().unwrap(),
        "ack",
        "T-046-06-03",
        "--generation",
        "9",
    ]);
    assert!(!unknown.status.success());
    assert_eq!(String::from_utf8_lossy(&unknown.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&unknown.stderr),
        "Error: Generation 9 is not listed for T-046-06-03. Listed generations: 1, 2.\n"
    );
    assert!(!root.join(".lisa/provenance.jsonl").exists());

    let exact = lisa(&[
        "notes",
        "--path",
        root.to_str().unwrap(),
        "ack",
        "T-046-06-03",
        "--generation",
        "2",
    ]);
    assert!(exact.status.success());
    assert_eq!(
        String::from_utf8_lossy(&exact.stdout),
        "T-046-06-03 generation 2 acknowledged.\n"
    );

    let row: NoteAcknowledgmentRecord = serde_json::from_str(
        fs::read_to_string(root.join(".lisa/provenance.jsonl"))
            .unwrap()
            .trim(),
    )
    .unwrap();
    assert_eq!(row.generation, 2);
    assert_eq!(row.attempt_id, "attempt-b");

    let after = notes(&root);
    assert!(after.status.success());
    let stdout = String::from_utf8_lossy(&after.stdout);
    assert!(stdout.starts_with(&format!("Notes for you (1)\nT-046-06-03  {SUMMARY}\n")));
    assert!(!stdout.contains(SECOND_SUMMARY));
}

#[test]
fn empty_queue_renders_nothing_to_read() {
    let (_temp, root) = project();
    let output = notes(&root);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Nothing to read.\n"
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn status_separates_urgent_waiting_from_deferred_notes() {
    let (_temp, root) = project();
    write_note_journal(&root);
    write_waiting_ticket(&root);

    let output = lisa(&["status", "--path", root.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    let waiting = stdout.find("Waiting on you").unwrap();
    let notes = stdout.find("Notes for you (1)").unwrap();
    let dag = stdout.find("DAG:").unwrap();
    assert!(waiting < notes);
    assert!(notes < dag);
    assert!(stdout.contains("T-WAIT  Run the release check."));
    assert!(stdout.contains(&format!("T-046-06-03  {SUMMARY}")));
}
