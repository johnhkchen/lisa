use std::path::PathBuf;
use std::process::Command;

#[test]
fn six_ticket_provider_contract_preserves_foreign_index_and_dependency_gate() {
    let harness = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/active/work/T-031-03/harness/run.sh");
    let lisa = env!("CARGO_BIN_EXE_lisa");

    let output = Command::new("bash")
        .arg(&harness)
        .env("LISA_BIN", lisa)
        .output()
        .expect("run T-031-03 provider-contract harness");

    assert!(
        output.status.success(),
        "provider-contract harness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("PASS: six-ticket atomic provider contract"),
        "harness did not print its completion receipt: {}",
        String::from_utf8_lossy(&output.stdout),
    );
}
