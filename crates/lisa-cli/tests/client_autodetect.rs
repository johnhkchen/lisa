#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

const CODEX_ONLY_ANNOUNCEMENT: &str = "Driving Codex — it's the agent installed here.";
const CLAUDE_ONLY_ANNOUNCEMENT: &str = "Driving Claude — it's the agent installed here.";
const BOTH_ANNOUNCEMENT: &str =
    "Driving Claude — both agents are installed; claude is the default.";
const NEITHER_ANNOUNCEMENT: &str =
    "Driving Claude — neither agent is installed; claude is the default.";
const CLAUDE_INSTALL_REMEDY: &str = "claude       not found
    Install: https://docs.anthropic.com/en/docs/claude-code";

#[derive(Clone, Copy)]
struct InstalledAgents {
    claude: bool,
    codex: bool,
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn run_fixture(
    command: &str,
    extra_args: &[&str],
    installed: InstalledAgents,
    configured_client: Option<&str>,
) -> Output {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    let home = temp.path().join("home");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&home).unwrap();

    fs::write(root.join("CLAUDE.md"), "# Client autodetect fixture\n").unwrap();
    fs::write(root.join(".codex/hooks.json"), "{}\n").unwrap();

    let zellij = bin.join("zellij");
    let agent_config = configured_client
        .map(|client| format!("\n[agent]\nclient = {client:?}\n"))
        .unwrap_or_default();
    fs::write(
        root.join(".lisa.toml"),
        format!(
            "version = {:?}\n\n[runtime]\nzellij = {:?}\n\n[guards]\ncompletion = \"journal\"\n{}",
            env!("CARGO_PKG_VERSION"),
            zellij.to_str().unwrap(),
            agent_config,
        ),
    )
    .unwrap();

    write_executable(
        &bin.join("git"),
        "#!/bin/sh\nprintf '%s\\n' 'git version 2.50.0'\nexit 0\n",
    );
    write_executable(
        &zellij,
        "#!/bin/sh\nif [ \"${1:-}\" = \"--version\" ]; then\n  printf '%s\\n' 'zellij 0.44.3'\nfi\nexit 0\n",
    );
    if installed.claude {
        write_executable(
            &bin.join("claude"),
            "#!/bin/sh\nprintf '%s\\n' 'claude 1.0.0'\nexit 0\n",
        );
    }
    if installed.codex {
        write_executable(
            &bin.join("codex"),
            "#!/bin/sh\nprintf '%s\\n' 'codex-cli 1.0.0'\nexit 0\n",
        );
    }

    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg(command)
        .args(extra_args)
        .args(["--path", root.to_str().unwrap()])
        .env("PATH", &bin)
        .env("HOME", &home)
        .output()
        .unwrap()
}

#[test]
fn codex_only_resolves_codex_and_doctor_is_green() {
    let output = run_fixture(
        "doctor",
        &[],
        InstalledAgents {
            claude: false,
            codex: true,
        },
        None,
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success(), "doctor failed:\n{stdout}");
    assert!(stdout.contains(CODEX_ONLY_ANNOUNCEMENT));
    assert!(stdout.contains("codex-cli 1.0.0"));
    assert!(!stdout.contains("docs.anthropic.com"));
}

#[test]
fn claude_only_resolves_claude_and_doctor_is_green() {
    let output = run_fixture(
        "doctor",
        &[],
        InstalledAgents {
            claude: true,
            codex: false,
        },
        None,
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success(), "doctor failed:\n{stdout}");
    assert!(stdout.contains(CLAUDE_ONLY_ANNOUNCEMENT));
    assert!(stdout.contains("claude 1.0.0"));
    assert!(!stdout.contains("Checking Codex trust"));
}

#[test]
fn both_agents_resolve_claude_and_announce_the_default() {
    let output = run_fixture(
        "doctor",
        &[],
        InstalledAgents {
            claude: true,
            codex: true,
        },
        None,
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success(), "doctor failed:\n{stdout}");
    assert!(stdout.contains(BOTH_ANNOUNCEMENT));
    assert!(stdout.contains("claude 1.0.0"));
    assert!(!stdout.contains("Checking Codex trust"));
}

#[test]
fn neither_agent_keeps_the_existing_claude_install_remedy() {
    let output = run_fixture(
        "doctor",
        &[],
        InstalledAgents {
            claude: false,
            codex: false,
        },
        None,
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(!output.status.success());
    assert!(stdout.contains(NEITHER_ANNOUNCEMENT));
    assert!(
        stdout.contains(CLAUDE_INSTALL_REMEDY),
        "missing byte-pinned Claude remedy:\n{stdout}"
    );
}

#[test]
fn explicit_config_beats_claude_only_detection() {
    let output = run_fixture(
        "doctor",
        &[],
        InstalledAgents {
            claude: true,
            codex: false,
        },
        Some("codex"),
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(!output.status.success());
    assert!(stdout.contains("Driving Codex — selected in .lisa.toml."));
    assert!(stdout.contains("codex        not found"));
    assert!(!stdout.contains(CLAUDE_ONLY_ANNOUNCEMENT));
}

#[test]
fn client_flag_beats_codex_only_detection() {
    let output = run_fixture(
        "loop",
        &["--client", "claude"],
        InstalledAgents {
            claude: false,
            codex: true,
        },
        None,
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(!output.status.success());
    assert!(stdout.contains("Driving Claude — selected by --client."));
    assert!(stderr.contains("Dependency preflight failed for claude"));
    assert!(!stdout.contains(CODEX_ONLY_ANNOUNCEMENT));
}

#[test]
fn loop_start_announces_the_detected_client() {
    let output = run_fixture(
        "loop",
        &[],
        InstalledAgents {
            claude: false,
            codex: true,
        },
        None,
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(
        stdout.contains(CODEX_ONLY_ANNOUNCEMENT),
        "loop did not announce its detected client:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    if !output.status.success() {
        assert!(
            stderr.contains("WASM plugin not embedded"),
            "unexpected loop failure:\n{stderr}"
        );
    }
}
