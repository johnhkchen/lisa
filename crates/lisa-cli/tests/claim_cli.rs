//! Black-box contract for the lease-bound `lisa claim` plumbing command.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use lisa_core::claim::{assignment_file_name, AssignmentClaim};
use lisa_core::types::AttemptLease;

const TICKET_ID: &str = "T-CLAIM-01";
const PANE_ID: u32 = 7;

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new(current_attempt: u64) -> Self {
        let root = tempfile::tempdir().unwrap();
        let signal_dir = root.path().join(".lisa/signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(
            signal_dir.join(format!("pane-{PANE_ID}.lease")),
            serde_json::to_vec(&AttemptLease {
                ticket_id: TICKET_ID.to_string(),
                attempt_id: current_attempt,
            })
            .unwrap(),
        )
        .unwrap();
        Self { root }
    }

    fn assignment(&self, attempt_id: u64, nonce: u128) -> PathBuf {
        let work = self
            .root
            .path()
            .join(".lisa/attempts")
            .join(TICKET_ID)
            .join(attempt_id.to_string())
            .join("work");
        fs::create_dir_all(&work).unwrap();
        let path = work.join(assignment_file_name(attempt_id, nonce));
        fs::write(&path, b"complete assignment bytes").unwrap();
        path
    }

    fn claim_path(&self) -> PathBuf {
        self.root
            .path()
            .join(format!(".lisa/signals/pane-{PANE_ID}.claim"))
    }

    fn run(&self, attempt_id: u64, nonce: u128) -> Output {
        Command::new(env!("CARGO_BIN_EXE_lisa"))
            .args([
                "claim",
                "--path",
                path_text(self.root.path()),
                "--ticket-id",
                TICKET_ID,
                "--attempt-id",
                &attempt_id.to_string(),
                "--nonce",
                &nonce.to_string(),
            ])
            .env("LISA_PANE_ID", PANE_ID.to_string())
            .env_remove("LISA_TICKET_ID")
            .env_remove("LISA_ATTEMPT_ID")
            .output()
            .expect("the lisa binary should run")
    }
}

fn path_text(path: &Path) -> &str {
    path.to_str().expect("temporary path should be UTF-8")
}

#[test]
fn current_attempt_and_nonce_under_held_lease_publish_claim() {
    let fixture = Fixture::new(2);
    fixture.assignment(2, 100);

    let output = fixture.run(2, 100);

    assert!(
        output.status.success(),
        "claim exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Claim accepted: T-CLAIM-01 attempt 2 nonce 100\n"
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_eq!(
        serde_json::from_slice::<AssignmentClaim>(&fs::read(fixture.claim_path()).unwrap())
            .unwrap(),
        AssignmentClaim {
            ticket_id: TICKET_ID.to_string(),
            attempt_id: 2,
            nonce: 100,
        }
    );
    assert!(
        fs::read_dir(fixture.root.path().join(".lisa/signals"))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".claim.tmp.")),
        "successful publication must leave no claim temporary"
    );
}

#[test]
fn prior_attempt_is_rejected_with_named_reason_even_when_its_assignment_exists() {
    let fixture = Fixture::new(2);
    fixture.assignment(1, 90);
    fixture.assignment(2, 100);

    let output = fixture.run(1, 90);

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("[stale-attempt]"),
        "stderr did not name stale-attempt: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!fixture.claim_path().exists());
}

#[test]
fn wrong_nonce_under_current_lease_is_rejected_with_named_reason() {
    let fixture = Fixture::new(2);
    fixture.assignment(2, 100);

    let output = fixture.run(2, 101);

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("[wrong-nonce]"),
        "stderr did not name wrong-nonce: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!fixture.claim_path().exists());
}
