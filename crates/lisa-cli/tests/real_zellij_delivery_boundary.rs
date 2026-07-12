use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "requires real Zellij, zsh, script, jq, and the wasm32-wasip1 target"]
fn real_zellij_delivery_boundary() {
    let harness = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/real_zellij_delivery_boundary.sh");
    let output = Command::new("bash")
        .arg(&harness)
        .env("LISA_BIN", env!("CARGO_BIN_EXE_lisa"))
        .output()
        .expect("run real-Zellij delivery-boundary harness");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "real-Zellij harness failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("real-zellij-delivery-boundary: PASS"),
        "harness did not print its completion receipt\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
