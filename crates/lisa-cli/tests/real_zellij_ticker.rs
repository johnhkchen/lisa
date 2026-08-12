use std::path::PathBuf;
use std::process::Command;

/// The plugin pane's title, read back out of a real Zellij at three moments
/// (T-062-01-03). Ignored by default: it needs a real Zellij and the built
/// WASM, like the other real-Zellij boundaries in this directory.
#[test]
#[ignore = "requires real Zellij, zsh, jq, python3, and the wasm32-wasip1 target"]
fn real_zellij_ticker() {
    let harness =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real_zellij_ticker.sh");
    let output = Command::new("bash")
        .arg(&harness)
        .env("LISA_BIN", env!("CARGO_BIN_EXE_lisa"))
        .output()
        .expect("run real-Zellij ticker harness");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "real-Zellij ticker harness failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("real-zellij-ticker: PASS"),
        "harness did not print its completion receipt\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
