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

/// The field failure end to end: a ticket sitting in `review` that no seat
/// holds is named in both renderings, with the ledger's reason beside it.
///
/// Before this, the same board printed as ordinary work in progress and the
/// document said nothing at all, which is how twelve minutes of agent time on
/// two tickets came to leave the desk's own accounting empty.
#[test]
fn a_ticket_under_way_that_nothing_is_working_is_named_in_both_renderings() {
    let (_temp, root) = project();
    fs::write(
        root.join("docs/active/tickets/T-001.md"),
        "---\nid: T-001\ntitle: first-thing\ntype: task\nstatus: in_progress\npriority: high\nphase: review\n---\n\n## Acceptance Criteria\n\n- It works\n",
    )
    .unwrap();
    fs::write(
        root.join(".lisa/provenance.jsonl"),
        concat!(
            r#"{"schema_version":11,"seal":"journal","record_type":"attempt-launch","ticket_id":"T-001","attempt_lease":{"ticket_id":"T-001","attempt_id":1},"pane_id":1,"provider":"anthropic","assignment":"assignment-1-77.md","occurred_at":1786650742}"#,
            "\n",
            r#"{"schema_version":11,"seal":"journal","ticket_id":"T-001","attempt_lease":{"ticket_id":"T-001","attempt_id":1},"outcome":"seat-lost","reason":"positive shell readiness was never proven","authoritative":false,"fenced":true,"requested":{"method":"claude","provider":"anthropic","model":null},"actual":{"method":"claude","provider":"anthropic","model":null},"started_at":1786650000,"ended_at":1786650720,"wall_clock_secs":720,"tokens_in":null,"tokens_out":null,"cost_usd":null,"concurrency_at_spawn":1,"pane_id":1}"#,
            "\n",
        ),
    )
    .unwrap();

    let prose = stdout_of(&lisa(&root, &["status"]));
    let data = data(&lisa(&root, &["status", "--json"]));

    let stranded = data["stranded"].as_array().unwrap();
    assert_eq!(stranded.len(), 1, "{data}");
    assert_eq!(stranded[0]["ticket_id"], "T-001");
    assert_eq!(stranded[0]["phase"], "review");
    assert_eq!(stranded[0]["attempt_id"], 1);
    let evidence = stranded[0]["evidence"].as_str().unwrap();
    assert!(evidence.contains("lost its seat"), "{evidence}");
    assert!(
        evidence.contains("positive shell readiness was never proven"),
        "{evidence}"
    );

    assert!(prose.contains("Tickets nobody is working"), "{prose}");
    assert!(prose.contains("T-001"), "{prose}");
    assert!(prose.contains(evidence), "{prose}");

    // And the spend that went with it is counted as lost rather than omitted.
    let lost = strings(&data["token_usage"]["lost_with_the_seat"]);
    assert_eq!(lost, vec!["T-001".to_string()]);
    assert!(prose.contains("Lost with the seat: 1 ticket"), "{prose}");
}

/// The shape is documented where a consumer is told to look.
#[test]
fn the_guide_names_the_tickets_nobody_is_working() {
    let guide = stdout_of(
        &Command::new(env!("CARGO_BIN_EXE_lisa"))
            .arg("json-guide")
            .output()
            .unwrap(),
    );
    for marker in ["stranded", "lost_with_the_seat", "attempt-launch"] {
        assert!(guide.contains(marker), "json-guide is missing {marker:?}");
    }
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

/// A board that names what it runs says so in one read, before anything has
/// run on it. A consumer choosing *where* to send work asks this question of
/// six boards at once and cannot open six private `.lisa.toml` files to get it.
#[test]
fn a_board_naming_a_client_and_model_carries_both_in_its_envelope() {
    let (_temp, root) = project();
    fs::write(
        root.join(".lisa.toml"),
        "[agent]\nclient = \"codex\"\nmodel = \"gpt-5-mini\"\n",
    )
    .unwrap();

    for command in ["status", "validate"] {
        let data = data(&lisa(&root, &[command, "--json"]));
        assert_eq!(
            data["config"]["client"], "codex",
            "{command} lost the configured client: {data}"
        );
        assert_eq!(
            data["config"]["model"], "gpt-5-mini",
            "{command} lost the configured model: {data}"
        );
    }
}

/// A board that names neither still answers: the client it would run — one of
/// the names Lisa knows — and `null` for the model, meaning "whatever that
/// client runs by default". Null is an answer here, not a missing field.
#[test]
fn a_board_naming_nothing_still_answers_with_a_client_and_a_null_model() {
    let (_temp, root) = project();
    fs::write(root.join(".lisa.toml"), "[scheduling]\nmax_threads = 2\n").unwrap();

    for command in ["status", "validate"] {
        let data = data(&lisa(&root, &[command, "--json"]));
        let client = data["config"]["client"]
            .as_str()
            .unwrap_or_else(|| panic!("{command} must name a client: {data}"));
        assert!(
            ["claude", "codex"].contains(&client),
            "{command} named an unknown client {client:?}"
        );
        assert!(
            data["config"]["model"].is_null(),
            "{command} invented a model for a board that names none: {data}"
        );
    }
}

// ---------------------------------------------------------------------------
// `run_location` — where the run on this board is (S-066-01).
//
// The fixtures below write scheduler records by hand rather than starting a
// Zellij session, which no test can do. That is exactly the shape the plugin
// publishes into `.lisa/schedulers/`, and `lisa-core::schedulers` round-trips
// it in its own unit tests; what these fixtures are for is the decision Lisa
// makes on top of it.
// ---------------------------------------------------------------------------

fn seconds_since_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("this machine's clock is before 1970")
        .as_secs()
}

/// Put a scheduler on this board, stamped now, in the session named.
///
/// `session` of `None` is a real case, not a defensive one: a run Lisa was
/// never told the session name of is a run it knows is here and cannot place.
fn scheduler_on(root: &Path, id: &str, session: Option<&str>) {
    let now = seconds_since_epoch();
    let dir = root.join(".lisa/schedulers");
    fs::create_dir_all(&dir).unwrap();
    let record = serde_json::json!({
        "schema_version": 1,
        "scheduler_id": id,
        "session_name": session,
        "zellij_pid": 9450,
        "started_at": now.saturating_sub(3600),
        "stamped_at": now,
        "poll_interval_secs": 5,
    });
    fs::write(
        dir.join(format!("{id}.alive")),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
}

/// Age `.lisa/signals/` so nothing reads as having moved in it.
///
/// A board that has finished every ticket looks precisely like this: the
/// scheduler is still stamping and the signal directory has been still for
/// hours. `lisa init` creates the directory a moment before the test runs, so
/// without this every fixture would read as freshly worked.
fn quiet_signals(root: &Path) {
    let status = Command::new("touch")
        .arg("-t")
        .arg("202001010000")
        .arg(root.join(".lisa/signals"))
        .status()
        .expect("failed to run touch");
    assert!(status.success(), "could not age the signal directory");
}

fn run_location(root: &Path) -> Value {
    data(&lisa(root, &["status", "--json"]))["run_location"].clone()
}

/// The whole point of the field: a board with a run on it says which session
/// holds it, and says it in the envelope rather than inside a refusal nobody
/// asked for. `rail up` had to inspect panes for this; a phone had to guess.
#[test]
fn a_board_with_a_run_says_which_session_holds_it() {
    let (_temp, root) = project();
    scheduler_on(&root, "lisa-9c1f4b0a", Some("fascinating-drum"));

    let location = run_location(&root);

    assert_eq!(location["state"], "working", "{location}");
    assert_eq!(location["session"], "fascinating-drum", "{location}");
    assert_eq!(strings(&location["sessions"]), ["fascinating-drum"]);
    assert_eq!(
        location["attach_command"], "zellij attach fascinating-drum",
        "the one command a phone over SSH needs: {location}"
    );
}

/// Absent means absent. A board nobody is running says so, rather than leaving
/// a reader to conclude it from silence — which is what every field in the
/// envelope did before this one.
#[test]
fn a_board_with_no_run_says_none_rather_than_falling_silent() {
    let (_temp, root) = project();

    let location = run_location(&root);

    assert_eq!(location["state"], "none", "{location}");
    assert!(location["session"].is_null(), "{location}");
    assert!(strings(&location["sessions"]).is_empty(), "{location}");
    assert!(location["attach_command"].is_null(), "{location}");
}

/// The 2026-08-12 incident, asked of the envelope instead of the loop. A run
/// that has finished every ticket is still resident, still holds the board, and
/// is still the session an operator attaches to — so it is reported, and
/// reported as `idle` rather than as work in flight. Reading it as absent is
/// what put a second scheduler on this board.
#[test]
fn a_board_whose_run_has_finished_still_says_where_it_is() {
    let (_temp, root) = project();
    let tickets = root.join("docs/active/tickets");
    for id in ["T-001", "T-002"] {
        fs::write(
            tickets.join(format!("{id}.md")),
            format!("---\nid: {id}\ntitle: finished-thing\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n\n## Acceptance Criteria\n\n- It works\n"),
        )
        .unwrap();
    }
    scheduler_on(&root, "lisa-9c1f4b0a", Some("fascinating-drum"));
    quiet_signals(&root);

    let data = data(&lisa(&root, &["status", "--json"]));
    let location = &data["run_location"];

    assert_eq!(
        data["counts"]["done"], data["counts"]["total"],
        "this fixture is only interesting on a drained board: {data}"
    );
    assert_ne!(
        location["state"], "none",
        "a finished run is not an absent one: {location}"
    );
    assert_eq!(
        location["state"], "idle",
        "a resident run with nothing moving is not work in flight: {location}"
    );
    assert_eq!(location["session"], "fascinating-drum", "{location}");
    assert_eq!(
        location["attach_command"], "zellij attach fascinating-drum",
        "{location}"
    );
}

/// The other half of "absent means absent": a run Lisa knows is here but was
/// never told the session name of. Today both this and an empty board are
/// silence, and they are opposite answers — one may be started on, one must
/// not be.
#[test]
fn a_run_lisa_cannot_place_is_not_a_board_with_no_run() {
    let (_temp, root) = project();
    scheduler_on(&root, "scheduler-1f0ab3c4", None);
    quiet_signals(&root);

    let location = run_location(&root);

    assert_eq!(
        location["state"], "idle",
        "a scheduler with no session name is still a scheduler: {location}"
    );
    assert!(
        location["session"].is_null() && strings(&location["sessions"]).is_empty(),
        "Lisa must not invent a session it was never told: {location}"
    );
    assert!(location["attach_command"].is_null(), "{location}");
}

/// Seven Zellij sessions have run on this desk at once, and two schedulers have
/// run on one board. Naming either one as *the* run would send a caller to an
/// arbitrary half of the problem, so `session` declines and `sessions` lists
/// both.
#[test]
fn a_board_with_two_runs_names_both_and_picks_neither() {
    let (_temp, root) = project();
    scheduler_on(&root, "lisa-9c1f4b0a", Some("fascinating-drum"));
    scheduler_on(&root, "lisa-2-77b10e5d", Some("blossoming-cymbal"));

    let location = run_location(&root);

    assert_eq!(
        strings(&location["sessions"]),
        ["blossoming-cymbal", "fascinating-drum"],
        "sorted, so a consumer can compare two readings: {location}"
    );
    assert!(location["session"].is_null(), "{location}");
    assert!(location["attach_command"].is_null(), "{location}");
    assert_ne!(location["state"], "none", "{location}");
}

/// The field is documented where a consumer is told to look, with the same
/// promise as everything else in the guide. A shape nobody can find is not an
/// interface.
#[test]
fn the_guide_names_where_the_run_is() {
    let guide = stdout_of(
        &Command::new(env!("CARGO_BIN_EXE_lisa"))
            .arg("json-guide")
            .output()
            .unwrap(),
    );
    for marker in [
        "run_location",
        "attach_command",
        "\"idle\"",
        "gh codespace ssh",
    ] {
        assert!(guide.contains(marker), "json-guide is missing {marker:?}");
    }
}
