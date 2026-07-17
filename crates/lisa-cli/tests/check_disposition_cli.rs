//! Black-box contract for the reviewer-side `lisa check-disposition` command.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const TICKET_ID: &str = "T-CHECK-01";
const ATTEMPT_ID: &str = "3";
const FIELD_ASK: &str = "The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant bypassed the old binary through managed mode instead of recording the required recovery through Lisa's error strings; John must either provide conforming reruns or explicitly amend both acceptance requirements before Review can pass.";

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }

    fn private_disposition(&self) -> PathBuf {
        self.root
            .path()
            .join(".lisa/attempts")
            .join(TICKET_ID)
            .join(ATTEMPT_ID)
            .join("work/review-disposition.json")
    }

    fn canonical_disposition(&self) -> PathBuf {
        self.root
            .path()
            .join("docs/active/work")
            .join(TICKET_ID)
            .join("review-disposition.json")
    }

    fn write(path: &Path, document: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, document).unwrap();
    }

    fn run_private(&self, document: &str) -> Output {
        Self::write(&self.private_disposition(), document);
        self.command()
            .env("LISA_TICKET_ID", TICKET_ID)
            .env("LISA_ATTEMPT_ID", ATTEMPT_ID)
            .output()
            .unwrap()
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_lisa"));
        command
            .args([
                "check-disposition",
                TICKET_ID,
                "--path",
                path_text(self.root.path()),
            ])
            .env_remove("LISA_TICKET_ID")
            .env_remove("LISA_ATTEMPT_ID");
        command
    }
}

fn path_text(path: &Path) -> &str {
    path.to_str().expect("temporary path should be UTF-8")
}

fn assert_success(output: Output) {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("Review disposition is valid for {TICKET_ID}")));
    assert!(stdout.contains("review-disposition.json"));
}

fn assert_fix(output: Output, expected: &str) {
    assert!(
        !output.status.success(),
        "invalid disposition unexpectedly passed"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("Error: Fix review-disposition.json:"),
        "failure did not lead with the artifact fix: {stderr}"
    );
    assert!(
        stderr.contains(expected),
        "failure did not name fix {expected:?}: {stderr}"
    );
}

#[test]
fn well_formed_pass_block_and_note_each_pass_in_the_active_attempt() {
    for document in [
        r#"{"disposition":"pass","reason":null}"#,
        r#"{"disposition":"block","reason":"release missing","remedy_owner":"world","ask":"Publish the release.","steps":["Run just release"],"check":"test -f release"}"#,
        r#"{"disposition":"note","reason":null,"criterion_quote":"approximately 200 MiB","evidence_citation":"docs/active/work/T-046-06-03/run-record.md","summary":"The measurement supports completion while the written criterion is stale."}"#,
    ] {
        assert_success(Fixture::new().run_private(document));
    }
}

#[test]
fn malformed_schema_and_missing_note_citation_name_the_fix() {
    let fixture = Fixture::new();
    assert_fix(fixture.run_private("{not json"), "write valid JSON");

    let fixture = Fixture::new();
    assert_fix(
        fixture.run_private(
            r#"{"disposition":"note","reason":null,"criterion_quote":"criterion","summary":"The criterion is stale."}"#,
        ),
        "evidence_citation",
    );
}

#[test]
fn work_complaint_note_and_unstructured_block_name_the_class_fix() {
    let fixture = Fixture::new();
    assert_fix(
        fixture.run_private(
            r#"{"disposition":"note","reason":null,"criterion_quote":"criterion","evidence_citation":"docs/evidence.md","summary":"The implementation is poor.","work_complaint":"Tests are failing."}"#,
        ),
        "remove work-quality complaints from a note; use a block",
    );

    let fixture = Fixture::new();
    assert_fix(
        fixture.run_private(r#"{"disposition":"block","reason":"tests are failing"}"#),
        "remedy_owner",
    );
}

#[test]
fn technical_field_paragraph_fails_the_shared_ask_floor() {
    let fixture = Fixture::new();
    let document = serde_json::json!({
        "disposition": "block",
        "reason": "criteria and measurements disagree",
        "remedy_owner": "operator",
        "ask": FIELD_ASK,
    });

    assert_fix(
        fixture.run_private(&serde_json::to_string(&document).unwrap()),
        "make ask's first sentence one short line",
    );
}

#[test]
fn direct_use_without_pane_environment_checks_canonical_work() {
    let fixture = Fixture::new();
    Fixture::write(
        &fixture.canonical_disposition(),
        r#"{"disposition":"pass","reason":null}"#,
    );

    assert_success(fixture.command().output().unwrap());
}

#[test]
fn active_pane_cannot_silently_check_a_different_ticket() {
    let fixture = Fixture::new();
    Fixture::write(
        &fixture.private_disposition(),
        r#"{"disposition":"pass","reason":null}"#,
    );
    let output = fixture
        .command()
        .env("LISA_TICKET_ID", "T-OTHER")
        .env("LISA_ATTEMPT_ID", ATTEMPT_ID)
        .output()
        .unwrap();

    assert_fix(output, "run the check for active ticket \"T-OTHER\"");
}
