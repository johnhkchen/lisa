//! Black-box regression for reconstructing a pre-ownership failure from the
//! retained ledger, without project tickets, scheduler state, or a live pane.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn status_reconstructs_preownership_failure_from_ledger_fixture_without_a_pane() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/preownership-ledger.jsonl");
    let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args([
            "status",
            "--ticket",
            "T-040-02-01",
            "--ledger",
            fixture.to_str().unwrap(),
        ])
        .output()
        .expect("the lisa binary should run");

    assert!(
        output.status.success(),
        "lisa status failed with {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "Pre-ownership failures for T-040-02-01 (1):\n",
            "Attempt 7 (pane 12)\n",
            "  state: delivery-failed\n",
            "  reason: provider did not acknowledge the bounded chat assignment\n",
            "  provider: openai\n",
            "  started_at: 1752000000\n",
            "  ended_at: 1752000030\n",
            "  wall_clock_secs: 30\n",
        )
    );
}
