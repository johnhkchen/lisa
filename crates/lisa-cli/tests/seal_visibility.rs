#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn run_fixture(command: &str, completion: &str) -> Output {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    let home = temp.path().join("home");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&home).unwrap();

    fs::write(root.join("CLAUDE.md"), "# Seal visibility fixture\n").unwrap();
    fs::write(
        root.join("docs/active/tickets/T-FIXTURE.md"),
        "---\nid: T-FIXTURE\ntitle: seal fixture\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\nFixture\n",
    )
    .unwrap();
    fs::write(
        root.join(".lisa.toml"),
        format!(
            "version = {:?}\n\n[runtime]\nzellij = \"system\"\n\n[agent]\nclient = \"claude\"\n\n[guards]\ncompletion = {completion:?}\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();

    write_executable(
        &bin.join("zellij"),
        "#!/bin/sh\nif [ \"${1:-}\" = \"--version\" ]; then printf '%s\\n' 'zellij 0.44.3'; fi\nexit 0\n",
    );
    write_executable(
        &bin.join("claude"),
        "#!/bin/sh\nif [ \"${1:-}\" = \"--version\" ]; then printf '%s\\n' 'claude 1.0.0'; fi\nexit 0\n",
    );

    let mut paths = vec![bin];
    paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));

    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg(command)
        .args(["--path", root.to_str().unwrap()])
        .env("PATH", env::join_paths(paths).unwrap())
        .env("HOME", home)
        .output()
        .unwrap()
}

fn assert_seal_line(command: &str, mode: &str, expected: &str) {
    let output = run_fixture(command, mode);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "{command} failed:\n{stdout}");
    let line = stdout
        .lines()
        .find(|line| line.contains("completion seal:"))
        .expect("command output must include a completion-seal line")
        .trim();
    assert_eq!(line, expected);
    if mode == "journal" {
        assert!(!line.to_ascii_lowercase().contains("git"));
    }
}

#[test]
fn doctor_and_status_fixture_show_each_resolved_seal_in_plain_language() {
    for command in ["doctor", "status"] {
        assert_seal_line(
            command,
            "commit",
            "completion seal: commit-sealed — finished work lands as history",
        );
        assert_seal_line(
            command,
            "journal",
            "completion seal: journal-only — finished work is recorded but not undoable",
        );
    }
}
