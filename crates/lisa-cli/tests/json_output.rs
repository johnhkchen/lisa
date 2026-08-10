//! Black-box fixtures for `lisa status --json` and `lisa validate --json`
//! (S-059-01).
//!
//! The point of these documents is that a second reader — a status strip, a
//! dashboard — can stop scraping prose. So the assertions are not "the JSON
//! parses"; they are "the JSON says the same thing the prose says". A body that
//! disagrees with the sentence beside it is worse than no body at all, and only
//! a test that reads both can catch that.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const READY_TICKET: &str = "---\nid: T-001\ntitle: first-thing\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\n## Acceptance Criteria\n\n- It works\n";
const BLOCKED_TICKET: &str = "---\nid: T-002\ntitle: second-thing\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-001]\n---\n\n## Acceptance Criteria\n\n- It works\n";

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

/// A scaffolded project with two tickets: one ready, one waiting on it.
fn project() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("board");
    fs::create_dir_all(&root).unwrap();
    let init = lisa(&root, &["init", "--no-history"]);
    assert!(
        init.status.success(),
        "lisa init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    let tickets = root.join("docs/active/tickets");
    fs::write(tickets.join("T-001.md"), READY_TICKET).unwrap();
    fs::write(tickets.join("T-002.md"), BLOCKED_TICKET).unwrap();
    (temp, root)
}

/// Parse the one document a `--json` run prints, insisting it is alone on
/// stdout: one line, nothing before it, nothing after it.
fn document(output: &Output) -> Value {
    let stdout = stdout_of(output);
    assert!(
        stdout.ends_with('\n'),
        "a document must end with one newline: {stdout:?}"
    );
    let body = stdout.trim_end_matches('\n');
    assert_eq!(
        body.lines().count(),
        1,
        "stdout must carry one document and nothing else: {stdout:?}"
    );
    serde_json::from_str(body)
        .unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {body}"))
}

fn data(output: &Output) -> Value {
    let document = document(output);
    assert_eq!(document["ok"], true, "expected an answer: {document}");
    assert!(document["error"].is_null(), "expected no error: {document}");
    document["data"].clone()
}

/// The numbers in a prose line like `Status: 0 done, 1 in progress, …`.
fn numbers_in(prose: &str, line_prefix: &str) -> Vec<u64> {
    let line = prose
        .lines()
        .find(|line| line.starts_with(line_prefix))
        .unwrap_or_else(|| panic!("prose has no line starting {line_prefix:?}:\n{prose}"));
    line.split(|character: char| !character.is_ascii_digit())
        .filter(|token| !token.is_empty())
        .map(|token| token.parse().unwrap())
        .collect()
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("expected an array")
        .iter()
        .map(|entry| entry.as_str().expect("expected a string").to_string())
        .collect()
}

/// Every number, name and structure the board prose states is in the document,
/// saying the same thing.
#[test]
fn status_json_document_agrees_with_the_prose() {
    let (_temp, root) = project();
    let prose = stdout_of(&lisa(&root, &["status"]));
    let output = lisa(&root, &["status", "--json"]);
    assert!(output.status.success());
    let data = data(&output);

    // Status: 0 done, 0 in progress, 1 ready, 1 blocked
    let counts = numbers_in(&prose, "Status: ");
    assert_eq!(data["counts"]["done"], counts[0]);
    assert_eq!(data["counts"]["in_progress"], counts[1]);
    assert_eq!(data["counts"]["ready"], counts[2]);
    assert_eq!(data["counts"]["blocked"], counts[3]);

    // DAG: 2 tickets, 1 edges, no cycles  /  Critical path: 2 tickets
    let structure = numbers_in(&prose, "DAG: ");
    assert_eq!(data["counts"]["total"], structure[0]);
    assert_eq!(data["edge_count"], structure[1]);
    assert_eq!(
        data["critical_path_length"],
        numbers_in(&prose, "Critical path: ")[0]
    );

    // Every ticket the prose lists, with the same spellings.
    let tickets = data["tickets"].as_array().unwrap();
    assert_eq!(tickets.len(), 2, "both tickets are described: {data}");
    for ticket in tickets {
        let id = ticket["id"].as_str().unwrap();
        let row = prose
            .lines()
            .find(|line| line.trim_start().starts_with(id))
            .unwrap_or_else(|| panic!("prose has no row for {id}:\n{prose}"));
        for field in ["title", "status", "phase"] {
            let value = ticket[field].as_str().unwrap();
            assert!(
                row.contains(value),
                "{id} row {row:?} does not carry {field} {value:?}"
            );
        }
        for dependency in strings(&ticket["depends_on"]) {
            assert!(
                row.contains(&format!("deps: {dependency}")),
                "{id} row {row:?} does not carry deps {dependency}"
            );
        }
        for blocked in strings(&ticket["blocks"]) {
            assert!(
                row.contains(&format!("blocks: {blocked}")),
                "{id} row {row:?} does not carry blocks {blocked}"
            );
        }
    }

    // The wave structure, wave by wave.
    let waves = data["waves"].as_array().unwrap();
    assert_eq!(waves.len(), 2, "two waves: {data}");
    assert!(waves[0]["depends_on_wave"].is_null());
    assert_eq!(waves[1]["depends_on_wave"], 0);
    for wave in waves {
        let index = wave["index"].as_u64().unwrap();
        assert!(
            prose.contains(&format!("Wave {index} (")),
            "prose is missing wave {index}:\n{prose}"
        );
        for id in strings(&wave["ticket_ids"]) {
            assert!(
                prose.contains(&id),
                "prose is missing {id} from wave {index}"
            );
        }
    }

    // Ready to schedule: T-001
    let ready = strings(&data["ready"]);
    assert_eq!(ready, vec!["T-001".to_string()]);
    assert!(prose.contains(&format!("Ready to schedule: {}", ready.join(", "))));

    // The settings line, and the seal said in one word rather than a sentence.
    assert!(prose.contains(&format!(
        "max_threads={}",
        data["config"]["max_threads"].as_u64().unwrap()
    )));
    assert_eq!(data["completion_seal"], "journal");
    assert!(prose.contains("completion seal: journal-only"));

    // Notes and waiting-on-you are empty here, and say so in both renderings.
    assert!(data["notes"].as_array().unwrap().is_empty());
    assert!(data["waiting_on_you"].as_array().unwrap().is_empty());
    assert!(prose.contains("Nothing to read."));
    assert!(prose.contains("Nothing waiting."));

    // Run summary: Completed: 0 of 2 tickets; 2 remain.
    assert_eq!(data["run_summary"]["tickets_total"], 2);
    assert_eq!(data["run_summary"]["tickets_completed"], 0);
    assert_eq!(data["run_summary"]["tickets_remaining"], 2);
    assert!(prose.contains("Completed: 0 of 2 tickets; 2 remain."));
}

/// The verdict, the ready count, and every problem named by file and reason.
#[test]
fn validate_json_document_agrees_with_the_prose() {
    let (_temp, root) = project();
    let prose = stdout_of(&lisa(&root, &["validate"]));
    let output = lisa(&root, &["validate", "--json"]);
    assert!(output.status.success(), "a clean project validates");
    let data = data(&output);

    // All checks passed. 2 tickets, 1 ready, DAG valid.
    let counts = numbers_in(&prose, "All checks passed. ");
    assert_eq!(data["verdict"], "passed");
    assert_eq!(data["ticket_count"], counts[0]);
    assert_eq!(data["ready_count"], counts[1]);
    assert_eq!(data["error_count"], 0);
    assert!(data["problems"].as_array().unwrap().is_empty());
}

/// A verdict of "no" keeps its exit status and carries the problems in the
/// body, rather than making the caller read a second format to find them.
#[test]
fn validate_failure_keeps_its_exit_status_and_still_carries_the_problems() {
    let (_temp, root) = project();
    fs::remove_file(root.join(".lisa/hooks/on-stop.sh")).unwrap();

    let plain = lisa(&root, &["validate"]);
    let output = lisa(&root, &["validate", "--json"]);
    assert_eq!(
        output.status.code(),
        plain.status.code(),
        "--json must not change what the exit status means"
    );
    assert_eq!(output.status.code(), Some(1));

    let document = document(&output);
    assert_eq!(document["ok"], true, "Lisa answered; the answer was no");
    let data = &document["data"];
    assert_eq!(data["verdict"], "failed");
    assert!(data["error_count"].as_u64().unwrap() >= 1);

    let prose = stdout_of(&plain);
    let problem = data["problems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|problem| problem["path"] == ".lisa/hooks/on-stop.sh")
        .unwrap_or_else(|| panic!("no problem names the missing hook: {data}"));
    assert_eq!(problem["severity"], "error");
    assert!(!problem["message"].as_str().unwrap().is_empty());
    assert!(
        prose.contains(".lisa/hooks/on-stop.sh"),
        "the prose names the same file:\n{prose}"
    );
    assert!(prose.contains(problem["message"].as_str().unwrap()));
}

/// Both commands print one document and nothing else, and both documents carry
/// the version marker a consumer checks before trusting a field.
#[test]
fn json_documents_are_one_line_carrying_the_version_marker() {
    let (_temp, root) = project();
    for command in ["status", "validate"] {
        let output = lisa(&root, &[command, "--json"]);
        let document = document(&output);
        assert_eq!(document["schema"], "lisa.cli/v1");
        assert_eq!(document["schema_version"], 1);
        assert_eq!(document["command"], command);
        assert!(!document["lisa_version"].as_str().unwrap().is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).is_empty(),
            "nothing but the document is printed"
        );
    }
}

/// When Lisa cannot answer at all, the caller still parses one format.
#[test]
fn a_failure_prints_a_json_error_document_with_the_same_exit_status() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("not-a-lisa-project");
    fs::create_dir_all(&root).unwrap();

    for command in ["status", "validate"] {
        let plain = lisa(&root, &[command]);
        let output = lisa(&root, &[command, "--json"]);
        assert_eq!(
            output.status.code(),
            plain.status.code(),
            "{command} --json must exit exactly as {command} does"
        );
        let document = document(&output);
        assert_eq!(document["ok"], false);
        assert!(document["data"].is_null());
        assert!(!document["error"]["message"].as_str().unwrap().is_empty());
    }
}

/// `--json` reports the board. `--ticket` asks something else with a different
/// shape, and saying so beats handing a caller a shape it did not ask for.
#[test]
fn status_json_refuses_the_ticket_detail_view_in_a_document() {
    let (_temp, root) = project();
    let output = lisa(&root, &["status", "--json", "--ticket", "T-001"]);
    assert_eq!(output.status.code(), Some(1));
    let document = document(&output);
    assert_eq!(document["ok"], false);
    assert!(document["error"]["message"]
        .as_str()
        .unwrap()
        .contains("--ticket"));
}

/// The in-flight list comes from the lease markers Lisa itself publishes, and
/// carries the board phase that tells a live seat from a finished one.
#[test]
fn attempts_name_the_seat_lisa_published_and_its_ticket_phase() {
    let (_temp, root) = project();
    let signals = root.join(".lisa/signals");
    fs::create_dir_all(&signals).unwrap();
    fs::write(
        signals.join("pane-7.lease"),
        r#"{"ticket_id":"T-001","attempt_id":3}"#,
    )
    .unwrap();
    // An older attempt for the same ticket, still holding a seat of its own.
    fs::write(
        signals.join("pane-6.lease"),
        r#"{"ticket_id":"T-001","attempt_id":1}"#,
    )
    .unwrap();
    // A marker for a ticket that is no longer on the board, and one that is not
    // a lease at all: neither may become a failure for the reader.
    fs::write(
        signals.join("pane-8.lease"),
        r#"{"ticket_id":"T-GONE","attempt_id":1}"#,
    )
    .unwrap();
    fs::write(signals.join("pane-9.lease"), "not a lease").unwrap();

    let data = data(&lisa(&root, &["status", "--json"]));
    let attempts = data["attempts"].as_array().unwrap();
    assert_eq!(
        attempts.len(),
        3,
        "the unreadable marker is skipped: {data}"
    );

    // Pane order, so a consumer can line the list up with its own seats.
    assert_eq!(attempts[0]["pane_id"], 6);
    assert_eq!(attempts[1]["pane_id"], 7);
    assert_eq!(attempts[2]["pane_id"], 8);

    // The older attempt for T-001 says so; the newer one does not.
    assert_eq!(attempts[0]["attempt_id"], 1);
    assert_eq!(attempts[0]["superseded"], true);
    assert_eq!(attempts[1]["ticket_id"], "T-001");
    assert_eq!(attempts[1]["attempt_id"], 3);
    assert_eq!(attempts[1]["superseded"], false);
    assert_eq!(attempts[1]["ticket_phase"], "ready");

    assert!(
        attempts[2]["ticket_phase"].is_null(),
        "a lease naming no live ticket has no phase to report"
    );
    assert_eq!(attempts[2]["superseded"], false);
}

/// Adding `--json` did not change what a person sees.
#[test]
fn human_output_is_unchanged_when_the_flag_is_absent() {
    let (_temp, root) = project();

    let status = stdout_of(&lisa(&root, &["status"]));
    for line in [
        "Waiting on you",
        "Notes for you",
        "DAG: 2 tickets, 1 edges, no cycles",
        "Critical path: 2 tickets",
        "Status: 0 done, 0 in progress, 1 ready, 1 blocked",
        "Token usage",
        "Wave 0 (no dependencies):",
        "Wave 1 (depends on wave 0):",
        "Ready to schedule: T-001",
        "Run summary:",
    ] {
        assert!(
            status.contains(line),
            "status prose lost {line:?}:\n{status}"
        );
    }
    assert!(!status.starts_with('{'), "bare status is not JSON");

    let validate = stdout_of(&lisa(&root, &["validate"]));
    assert!(validate.contains("All checks passed. 2 tickets, 1 ready, DAG valid."));
    assert!(!validate.starts_with('{'), "bare validate is not JSON");
}

/// The shape is documented where a consumer will look.
#[test]
fn the_guide_and_the_help_point_at_each_other() {
    let guide = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("json-guide")
        .output()
        .unwrap();
    assert!(guide.status.success());
    let guide = stdout_of(&guide);
    for marker in [
        "lisa status --json",
        "lisa validate --json",
        "lisa.cli/v1",
        "Ignore fields you do not know",
        "Exit status",
        "attempts",
    ] {
        assert!(guide.contains(marker), "json-guide is missing {marker:?}");
    }

    for command in ["status", "validate"] {
        let help = stdout_of(
            &Command::new(env!("CARGO_BIN_EXE_lisa"))
                .args([command, "--help"])
                .output()
                .unwrap(),
        );
        assert!(help.contains("--json"), "{command} --help hides --json");
        assert!(
            help.contains("lisa json-guide"),
            "{command} --help does not say where the shape is written down"
        );
    }
}
