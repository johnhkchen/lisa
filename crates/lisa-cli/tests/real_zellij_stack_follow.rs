use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "requires real Zellij, zsh, python3, jq, and the wasm32-wasip1 target"]
fn real_zellij_stack_follow() {
    let harness = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/real_zellij_stack_follow.sh");
    let output = Command::new("bash")
        .arg(&harness)
        .env("LISA_BIN", env!("CARGO_BIN_EXE_lisa"))
        .output()
        .expect("run real-Zellij stack-follow harness");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "real-Zellij stack harness failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("real-zellij-stack-follow: PASS"),
        "harness did not print its completion receipt\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
