#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn run_with_zellij_version(command: &str, version_output: &str) -> Output {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    let home = temp.path().join("home");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&home).unwrap();

    fs::write(root.join("CLAUDE.md"), "# Stubbed Zellij preflight\n").unwrap();
    fs::write(
        root.join(".lisa.toml"),
        format!(
            "version = {:?}\n\n[agent]\nclient = \"claude\"\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();

    let git = Command::new("git")
        .args(["init", "--quiet"])
        .arg(&root)
        .status()
        .unwrap();
    assert!(git.success());

    write_executable(
        &bin.join("zellij"),
        &format!(
            "#!/bin/sh\nif [ \"${{1:-}}\" = \"--version\" ]; then\n  printf '%s\\n' {:?}\n  exit 0\nfi\nexit 0\n",
            version_output
        ),
    );
    write_executable(
        &bin.join("claude"),
        "#!/bin/sh\nif [ \"${1:-}\" = \"--version\" ]; then\n  printf '%s\\n' 'claude 1.0.0'\nfi\nexit 0\n",
    );

    let original_path = env::var_os("PATH").unwrap_or_default();
    let path =
        env::join_paths(std::iter::once(bin).chain(env::split_paths(&original_path))).unwrap();

    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg(command)
        .args(["--path", root.to_str().unwrap()])
        .env("PATH", path)
        .env("HOME", home)
        .output()
        .unwrap()
}

fn assert_supported_loop_preflight(version: &str) {
    let output = run_with_zellij_version("loop", version);
    if output.status.success() {
        return;
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("WASM plugin not embedded"),
        "supported Zellij {version} did not pass dependency preflight:\n{stderr}"
    );
    assert!(!stderr.contains("Dependency preflight failed"));
}

#[test]
fn loop_refuses_zellij_0401_with_floor_and_remedy() {
    let output = run_with_zellij_version("loop", "zellij 0.40.1");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Dependency preflight failed"));
    assert!(stderr.contains("detected Zellij 0.40.1"));
    assert!(stderr.contains("supported range >= 0.43.0"));
    assert!(stderr.contains("prebuilt static binaries"));
    assert!(stderr.contains("https://github.com/zellij-org/zellij/releases"));
}

#[test]
fn loop_preflight_accepts_zellij_043_and_044() {
    assert_supported_loop_preflight("zellij 0.43.9");
    assert_supported_loop_preflight("zellij 0.44.3");
}

#[test]
fn doctor_reports_detected_version_and_supported_range_on_success() {
    let output = run_with_zellij_version("doctor", "zellij 0.44.3");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("zellij"));
    assert!(stdout.contains("detected 0.44.3, supported >= 0.43.0"));
    assert!(stdout.contains("OK"));
}

#[test]
fn doctor_names_unparseable_zellij_output_as_unsupported() {
    let output = run_with_zellij_version("doctor", "zellij mystery-version");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!output.status.success());
    assert!(stdout.contains("zellij"));
    assert!(stdout.contains("unsupported"));
    assert!(stdout.contains("unparseable Zellij version output \"zellij mystery-version\""));
    assert!(stdout.contains("supported range >= 0.43.0"));
    assert!(stdout.contains("prebuilt static binaries"));
}
