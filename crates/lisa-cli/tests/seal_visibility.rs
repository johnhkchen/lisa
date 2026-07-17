#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn lisa_command(root: &Path, home: &Path, bin: &Path, subcommand: &str) -> Command {
    let mut paths = vec![bin.to_path_buf()];
    paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));

    let mut command = Command::new(env!("CARGO_BIN_EXE_lisa"));
    command
        .env("PATH", env::join_paths(paths).unwrap())
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .arg(subcommand)
        .arg("--path")
        .arg(root);
    command
}

#[derive(Clone, Copy)]
enum RepositoryFixture {
    Absent,
    UnbornMissingIdentity,
    BornMissingIdentity,
    UnbornWithIdentity,
    WithIdentity,
}

fn run_fixture(command: &str, completion: &str, repository: RepositoryFixture) -> Output {
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

    if !matches!(repository, RepositoryFixture::Absent) {
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .arg(&root)
            .status()
            .unwrap();
        assert!(status.success());
    }
    if matches!(
        repository,
        RepositoryFixture::UnbornWithIdentity | RepositoryFixture::WithIdentity
    ) {
        for args in [
            ["config", "user.name", "Seal Fixture"],
            ["config", "user.email", "seal-fixture@example.invalid"],
        ] {
            let status = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success());
        }
    }
    if matches!(repository, RepositoryFixture::BornMissingIdentity) {
        let status = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["commit", "--quiet", "--allow-empty", "-m", "fixture root"])
            .env("GIT_AUTHOR_NAME", "Fixture Bootstrap")
            .env("GIT_AUTHOR_EMAIL", "bootstrap@example.invalid")
            .env("GIT_COMMITTER_NAME", "Fixture Bootstrap")
            .env("GIT_COMMITTER_EMAIL", "bootstrap@example.invalid")
            .status()
            .unwrap();
        assert!(status.success());
    }
    if matches!(repository, RepositoryFixture::WithIdentity) {
        let status = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["commit", "--quiet", "--allow-empty", "-m", "fixture root"])
            .status()
            .unwrap();
        assert!(status.success());
    }

    write_executable(
        &bin.join("zellij"),
        "#!/bin/sh\nif [ \"${1:-}\" = \"--version\" ]; then printf '%s\\n' 'zellij 0.44.3'; fi\nexit 0\n",
    );
    write_executable(
        &bin.join("claude"),
        "#!/bin/sh\nif [ \"${1:-}\" = \"--version\" ]; then printf '%s\\n' 'claude 1.0.0'; fi\nexit 0\n",
    );

    lisa_command(&root, &home, &bin, command).output().unwrap()
}

fn cure_fixture(name: &str) -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(name);
    let bin = temp.path().join("bin");
    let home = temp.path().join("home");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::write(root.join("CLAUDE.md"), "# Remedy cure fixture\n").unwrap();
    fs::write(
        root.join("docs/active/tickets/T-FIXTURE.md"),
        "---\nid: T-FIXTURE\ntitle: remedy cure fixture\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\nFixture\n",
    )
    .unwrap();
    fs::write(
        root.join(".lisa.toml"),
        format!(
            "version = {:?}\n\n[runtime]\nzellij = \"system\"\n\n[agent]\nclient = \"claude\"\n\n[guards]\ncompletion = \"auto\"\n",
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
    (temp, root, bin, home)
}

fn assert_completion_cured(root: &Path, home: &Path, bin: &Path, message: &str) {
    fs::write(root.join("completion-marker.txt"), "finished\n").unwrap();
    let completion = lisa_command(root, home, bin, "commit-ticket")
        .args([
            "--ticket-id",
            "T-FIXTURE",
            "--message",
            message,
            "--include",
            "completion-marker.txt",
        ])
        .output()
        .unwrap();
    assert!(
        completion.status.success(),
        "completion failed:\n{}",
        String::from_utf8_lossy(&completion.stderr)
    );

    let after = lisa_command(root, home, bin, "doctor").output().unwrap();
    let after_stdout = String::from_utf8(after.stdout).unwrap();
    assert!(after.status.success(), "doctor failed:\n{after_stdout}");
    assert!(after_stdout.contains("completion seal: commit-sealed"));
}

fn assert_seal_line(command: &str, mode: &str, repository: RepositoryFixture, expected: &str) {
    let output = run_fixture(command, mode, repository);
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
    assert_seal_line(
        "doctor",
        "commit",
        RepositoryFixture::WithIdentity,
        "completion seal: commit-sealed — finished work lands as history",
    );
    assert_seal_line(
        "doctor",
        "journal",
        RepositoryFixture::Absent,
        "completion seal: journal-only — finished work is recorded but not undoable",
    );
    for mode in ["commit", "journal"] {
        assert_seal_line(
            "status",
            mode,
            RepositoryFixture::Absent,
            if mode == "commit" {
                "completion seal: commit-sealed — finished work lands as history"
            } else {
                "completion seal: journal-only — finished work is recorded but not undoable"
            },
        );
    }
}

const IDENTITY_REASON: &str =
    "no commit identity is configured (git config user.email did not resolve)";
const IDENTITY_CONFIG_REMEDIES: &str = "Configure your own identity:
  git config user.name \"You\"
  git config user.email you@example.com";
const IDENTITY_INIT_REMEDY: &str = "Or rerun `lisa init` and accept the history offer.";
const REPOSITORY_REMEDY: &str = "Run `lisa init` to create project history, then retry.";
const TRANSACTION_HISTORY_REMEDY: &str = "Run `lisa init` and accept the history offer to create the missing project-history dependency, then retry.";
const TRANSACTION_REMEDY: &str =
    "Repair the named commit-transaction dependency, then rerun `lisa doctor`.";

#[test]
fn doctor_auto_names_unborn_missing_identity_and_both_valid_remedies_verbatim() {
    let output = run_fixture("doctor", "auto", RepositoryFixture::UnbornMissingIdentity);
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success(), "doctor failed:\n{stdout}");
    assert!(stdout
        .contains("completion seal: journal-only — finished work is recorded but not undoable"));
    assert!(stdout.contains(IDENTITY_REASON));
    assert!(stdout.contains(IDENTITY_CONFIG_REMEDIES));
    assert!(stdout.contains(IDENTITY_INIT_REMEDY));
    assert!(!stdout.contains(REPOSITORY_REMEDY));
    assert!(!stdout.contains(TRANSACTION_HISTORY_REMEDY));
    assert!(!stdout.contains(TRANSACTION_REMEDY));
}

#[test]
fn doctor_auto_born_missing_identity_prints_only_config_commands_verbatim() {
    let output = run_fixture("doctor", "auto", RepositoryFixture::BornMissingIdentity);
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success(), "doctor failed:\n{stdout}");
    assert!(stdout.contains(IDENTITY_REASON));
    assert!(stdout.contains(IDENTITY_CONFIG_REMEDIES));
    assert!(!stdout.contains(IDENTITY_INIT_REMEDY));
    assert!(!stdout.contains(REPOSITORY_REMEDY));
    assert!(!stdout.contains(TRANSACTION_HISTORY_REMEDY));
    assert!(!stdout.contains(TRANSACTION_REMEDY));
}

#[test]
fn doctor_auto_is_silent_about_identity_when_repository_can_commit() {
    let output = run_fixture("doctor", "auto", RepositoryFixture::WithIdentity);
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success(), "doctor failed:\n{stdout}");
    assert!(stdout.contains("completion seal: commit-sealed — finished work lands as history"));
    assert!(!stdout.contains(IDENTITY_REASON));
    assert!(!stdout.contains(IDENTITY_CONFIG_REMEDIES));
    assert!(!stdout.contains(IDENTITY_INIT_REMEDY));
    assert!(!stdout.contains(TRANSACTION_HISTORY_REMEDY));
}

#[test]
fn doctor_auto_without_repository_defers_to_journal_seal_line() {
    let output = run_fixture("doctor", "auto", RepositoryFixture::Absent);
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success(), "doctor failed:\n{stdout}");
    assert!(stdout
        .contains("completion seal: journal-only — finished work is recorded but not undoable"));
    assert!(!stdout.contains(IDENTITY_REASON));
    assert!(!stdout.contains(IDENTITY_CONFIG_REMEDIES));
    assert!(stdout.contains(REPOSITORY_REMEDY));
    assert!(!stdout.contains(IDENTITY_INIT_REMEDY));
    assert!(!stdout.contains(TRANSACTION_HISTORY_REMEDY));
    assert!(!stdout.contains(TRANSACTION_REMEDY));
}

#[test]
fn doctor_explicit_commit_uses_contextual_missing_identity_hard_failure() {
    let output = run_fixture("doctor", "commit", RepositoryFixture::BornMissingIdentity);
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(!output.status.success());
    assert!(stdout.contains("Completion seal preflight failed"));
    assert!(stdout.contains("[guards].completion = \"commit\""));
    assert!(stdout.contains(IDENTITY_REASON));
    assert!(stdout.contains(IDENTITY_CONFIG_REMEDIES));
    assert!(!stdout.contains(IDENTITY_INIT_REMEDY));
    assert!(!stdout.contains(REPOSITORY_REMEDY));
    assert!(!stdout.contains(TRANSACTION_HISTORY_REMEDY));
    assert!(!stdout.contains(TRANSACTION_REMEDY));
}

#[test]
fn doctor_auto_transaction_failure_prints_only_dependency_remedy() {
    let output = run_fixture("doctor", "auto", RepositoryFixture::UnbornWithIdentity);
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success(), "doctor failed:\n{stdout}");
    assert!(stdout.contains("the commit transaction path is unavailable"));
    assert!(stdout.contains(TRANSACTION_HISTORY_REMEDY));
    assert!(!stdout.contains(TRANSACTION_REMEDY));
    assert!(!stdout.contains(IDENTITY_CONFIG_REMEDIES));
    assert!(!stdout.contains(IDENTITY_INIT_REMEDY));
    assert!(!stdout.contains(REPOSITORY_REMEDY));
}

#[test]
fn born_identity_commands_printed_by_doctor_cure_a_completion_commit() {
    let (_temp, root, bin, home) = cure_fixture("identity-cure");
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .arg(&root)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["commit", "--quiet", "--allow-empty", "-m", "fixture root"])
        .env("GIT_AUTHOR_NAME", "Fixture Bootstrap")
        .env("GIT_AUTHOR_EMAIL", "bootstrap@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture Bootstrap")
        .env("GIT_COMMITTER_EMAIL", "bootstrap@example.invalid")
        .status()
        .unwrap()
        .success());

    let before = lisa_command(&root, &home, &bin, "doctor").output().unwrap();
    let before_stdout = String::from_utf8(before.stdout).unwrap();
    assert!(before_stdout.contains(IDENTITY_REASON));
    assert!(before_stdout.contains(IDENTITY_CONFIG_REMEDIES));
    assert!(!before_stdout.contains(IDENTITY_INIT_REMEDY));

    for args in [
        ["config", "user.name", "You"],
        ["config", "user.email", "you@example.com"],
    ] {
        assert!(Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .env("HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .unwrap()
            .success());
    }

    assert_completion_cured(&root, &home, &bin, "Complete identity cure fixture");
}

#[test]
fn repository_remedy_printed_by_doctor_cures_a_completion_commit() {
    let (_temp, root, bin, home) = cure_fixture("repository-cure");
    let before = lisa_command(&root, &home, &bin, "doctor").output().unwrap();
    let before_stdout = String::from_utf8(before.stdout).unwrap();
    assert!(before_stdout.contains("Reason: no repository is present."));
    assert!(before_stdout.contains(REPOSITORY_REMEDY));
    assert!(!before_stdout.contains(IDENTITY_CONFIG_REMEDIES));

    let init = lisa_command(&root, &home, &bin, "init").output().unwrap();
    assert!(
        init.status.success(),
        "init failed:\n{}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert_completion_cured(&root, &home, &bin, "Complete repository cure fixture");
}

#[test]
fn unborn_transaction_remedy_printed_by_doctor_cures_a_completion_commit() {
    let (_temp, root, bin, home) = cure_fixture("transaction-cure");
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .arg(&root)
        .status()
        .unwrap()
        .success());
    for args in [
        ["config", "user.name", "Seal Fixture"],
        ["config", "user.email", "seal-fixture@example.invalid"],
    ] {
        assert!(Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .status()
            .unwrap()
            .success());
    }

    let before = lisa_command(&root, &home, &bin, "doctor").output().unwrap();
    let before_stdout = String::from_utf8(before.stdout).unwrap();
    assert!(before_stdout.contains("the commit transaction path is unavailable"));
    assert!(before_stdout.contains(TRANSACTION_HISTORY_REMEDY));
    assert!(!before_stdout.contains(IDENTITY_CONFIG_REMEDIES));

    let init = lisa_command(&root, &home, &bin, "init")
        .arg("--with-history")
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "init failed:\n{}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert_completion_cured(&root, &home, &bin, "Complete transaction cure fixture");
}
