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
    run_with_zellij_version_and_path(command, version_output, true)
}

fn run_with_zellij_version_and_path(
    command: &str,
    version_output: &str,
    include_host_path: bool,
) -> Output {
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
            "version = {:?}\n\n[runtime]\nzellij = \"system\"\n\n[agent]\nclient = \"claude\"\n",
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

    let mut paths = vec![bin];
    if include_host_path {
        let original_path = env::var_os("PATH").unwrap_or_default();
        paths.extend(env::split_paths(&original_path));
    }
    let path = env::join_paths(paths).unwrap();

    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg(command)
        .args(["--path", root.to_str().unwrap()])
        .env("PATH", path)
        .env("HOME", home)
        .output()
        .unwrap()
}

#[test]
fn doctor_names_missing_git_and_apt_remedy() {
    let output = run_with_zellij_version_and_path("doctor", "zellij 0.44.3", false);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!output.status.success());
    assert!(stdout.contains("git"));
    assert!(stdout.contains("not found"));
    assert!(stdout.contains("sudo apt install git"));
}

#[test]
fn loop_auto_without_git_uses_journal_instead_of_requiring_git() {
    let output = run_with_zellij_version_and_path("loop", "zellij 0.44.3", false);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "auto loop failed: {stderr}");
    assert!(stdout.contains("Lisa loop starting"));
    assert!(!stderr.contains("Dependency preflight failed"));
    assert!(!stderr.contains("sudo apt install git"));
    assert!(!stderr.contains("Failed to discover Git root"));
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

fn assert_runtime_remedy(output: &str) {
    assert!(
        output.contains("prebuilt static binaries") || output.contains("managed runtime"),
        "missing Zellij runtime remedy:\n{output}"
    );
}

#[test]
fn loop_refuses_zellij_0401_with_floor_and_remedy() {
    let output = run_with_zellij_version("loop", "zellij 0.40.1");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Zellij 0.40.1"));
    assert!(stderr.contains(">= 0.43.0"));
    assert_runtime_remedy(&stderr);
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
    assert!(stdout.contains("0.44.3"));
    assert!(stdout.contains("supported >= 0.43.0"));
    assert!(stdout.contains("OK"));
}

#[test]
fn doctor_names_unparseable_zellij_output_as_unsupported() {
    let output = run_with_zellij_version("doctor", "zellij mystery-version");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!output.status.success());
    assert!(stdout.contains("zellij"));
    assert!(stdout.contains("unsupported"));
    assert!(stdout.contains("unparseable"));
    assert!(stdout.contains("zellij mystery-version"));
    assert!(stdout.contains(">= 0.43.0"));
    assert_runtime_remedy(&stdout);
}
