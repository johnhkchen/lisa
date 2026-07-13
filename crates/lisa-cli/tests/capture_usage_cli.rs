use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::time::SystemTime;

use lisa_core::capture::CaptureRecord;
use lisa_core::provenance::system_time_to_epoch;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct NoCaptureMarker {
    pane_id: u32,
    session_id: String,
    captured_at: u64,
    reason: String,
}

fn capture_usage(
    root: &std::path::Path,
    pane_id: u32,
    session_id: &str,
    transcript: &str,
) -> Output {
    let payload = serde_json::json!({
        "session_id": session_id,
        "transcript_path": transcript,
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["capture-usage", "--cwd"])
        .arg(root)
        .env("LISA_PANE_ID", pane_id.to_string())
        .env("LISA_TICKET_ID", "T-STALE-FIRST-TICKET")
        .env_remove("LISA_AGENT_CLIENT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.to_string().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn assert_capture_succeeded(output: &Output) {
    assert!(
        output.status.success(),
        "capture-usage failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn two_cli_captures_for_one_pane_append_honest_records_without_ticket_artifact() {
    let root = tempfile::tempdir().unwrap();
    let first_transcript = root.path().join("first.jsonl");
    let second_transcript = root.path().join("second.jsonl");
    std::fs::write(
        &first_transcript,
        r#"{"type":"assistant","message":{"usage":{"input_tokens":11,"cache_creation_input_tokens":2,"cache_read_input_tokens":3,"output_tokens":5}}}
"#,
    )
    .unwrap();
    std::fs::write(
        &second_transcript,
        r#"{"type":"assistant","message":{"usage":{"input_tokens":101,"cache_creation_input_tokens":20,"cache_read_input_tokens":30,"output_tokens":50}}}
"#,
    )
    .unwrap();

    let before = system_time_to_epoch(SystemTime::now());
    let first_output = capture_usage(
        root.path(),
        42,
        "session-first",
        first_transcript.to_str().unwrap(),
    );
    assert_capture_succeeded(&first_output);
    let second_output = capture_usage(
        root.path(),
        42,
        "session-second",
        second_transcript.to_str().unwrap(),
    );
    assert_capture_succeeded(&second_output);
    let after = system_time_to_epoch(SystemTime::now());

    let capture_path = root.path().join(".lisa/claude/captures.jsonl");
    let records: Vec<CaptureRecord> = std::fs::read_to_string(capture_path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].pane_id, 42);
    assert_eq!(records[0].session_id, "session-first");
    assert_eq!(records[0].input_tokens, 16);
    assert_eq!(records[0].output_tokens, 5);
    assert_eq!(records[1].pane_id, 42);
    assert_eq!(records[1].session_id, "session-second");
    assert_eq!(records[1].input_tokens, 151);
    assert_eq!(records[1].output_tokens, 50);
    assert!(
        records
            .iter()
            .all(|record| (before..=after).contains(&record.captured_at)),
        "capture timestamps should fall within the two CLI invocations: {records:?}"
    );
    assert!(
        !root
            .path()
            .join(".lisa/claude/T-STALE-FIRST-TICKET.usage.json")
            .exists(),
        "capture-usage must not trust inherited LISA_TICKET_ID"
    );
}

#[test]
fn empty_and_unreadable_transcripts_append_visible_no_capture_markers() {
    let root = tempfile::tempdir().unwrap();
    let empty_transcript = root.path().join("empty.jsonl");
    let unreadable_transcript = root.path().join("does-not-exist.jsonl");
    std::fs::write(&empty_transcript, "").unwrap();

    let before = system_time_to_epoch(SystemTime::now());
    let empty_output = capture_usage(
        root.path(),
        73,
        "session-empty",
        empty_transcript.to_str().unwrap(),
    );
    let unreadable_output = capture_usage(
        root.path(),
        73,
        "session-unreadable",
        unreadable_transcript.to_str().unwrap(),
    );
    let after = system_time_to_epoch(SystemTime::now());

    assert_capture_succeeded(&empty_output);
    assert_capture_succeeded(&unreadable_output);

    let empty_stderr = String::from_utf8_lossy(&empty_output.stderr);
    assert!(empty_stderr.contains("lisa capture-usage: no capture"));
    assert!(empty_stderr.contains("empty-transcript"));
    let unreadable_stderr = String::from_utf8_lossy(&unreadable_output.stderr);
    assert!(unreadable_stderr.contains("lisa capture-usage: no capture"));
    assert!(unreadable_stderr.contains("unreadable-transcript"));

    assert!(
        !root.path().join(".lisa/claude/captures.jsonl").exists(),
        "unsuccessful observations must not be represented as measured token totals"
    );
    let markers: Vec<NoCaptureMarker> =
        std::fs::read_to_string(root.path().join(".lisa/claude/no-captures.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0].pane_id, 73);
    assert_eq!(markers[0].session_id, "session-empty");
    assert_eq!(markers[0].reason, "empty-transcript");
    assert_eq!(markers[1].pane_id, 73);
    assert_eq!(markers[1].session_id, "session-unreadable");
    assert_eq!(markers[1].reason, "unreadable-transcript");
    assert!(
        markers
            .iter()
            .all(|marker| (before..=after).contains(&marker.captured_at)),
        "marker timestamps should fall within the CLI invocations: {markers:?}"
    );
}
