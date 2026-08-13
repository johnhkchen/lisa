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
    run_with_zellij_version_and_path_and_home(command, version_output, include_host_path, true)
}

fn run_with_zellij_version_and_path_and_home(
    command: &str,
    version_output: &str,
    include_host_path: bool,
    seed_claude_acceptance: bool,
) -> Output {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    let home = temp.path().join("home");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&home).unwrap();
    // These fixtures test Zellij/seal preflights, so by default the Claude
    // one-time confirmation is declared already accepted; the refusal path
    // has its own dedicated test.
    if seed_claude_acceptance {
        fs::write(
            home.join(".claude.json"),
            "{\"bypassPermissionsModeAccepted\": true}\n",
        )
        .unwrap();
    }

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
fn doctor_auto_without_git_passes_journal_sealed() {
    // 2026-07-17 rc.1 field regression: auto seal + no usable git resolves
    // journal, and doctor must pass instead of demanding a git install.
    let output = run_with_zellij_version_and_path("doctor", "zellij 0.44.3", false);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "auto doctor failed without git:\n{stdout}"
    );
    assert!(stdout
        .contains("completion seal: journal-only — finished work is recorded but not undoable"));
    assert!(!stdout.contains("sudo apt install git"));
}

#[test]
fn doctor_explicit_commit_without_git_fails_with_apt_remedy() {
    // The operator chose commit sealing; a machine without git is then a
    // hard, named failure — git stays a required dependency with its remedy.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    let home = temp.path().join("home");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::write(
        root.join(".lisa.toml"),
        format!(
            "version = {:?}\n\n[runtime]\nzellij = \"system\"\n\n[agent]\nclient = \"claude\"\n\n[guards]\ncompletion = \"commit\"\n",
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

    let mut command = Command::new(env!("CARGO_BIN_EXE_lisa"));
    command
        .env("PATH", &bin)
        .env("HOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(["doctor", "--path"])
        .arg(&root);
    let output = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "explicit commit must fail without git:\n{stdout}"
    );
    assert!(stdout.contains("git"));
    assert!(stdout.contains("sudo apt install git"));
}

#[test]
fn loop_refuses_claude_without_first_run_confirmation() {
    // Field stall, 2026-07-18: a fresh machine's loop spawned two Claude panes
    // that froze at the bypass-permissions dialog. The loop must refuse with
    // the one-command remedy instead — and Lisa never accepts it for you.
    let output = run_with_zellij_version_and_path_and_home(
        "loop",
        "zellij 0.44.3",
        false,
        false, // do NOT seed the acceptance
    );
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "loop must refuse: {stderr}");
    assert!(stderr.contains("one-time confirmation"));
    assert!(stderr.contains("claude --dangerously-skip-permissions"));
    assert!(stderr.contains("Lisa never accepts it for you"));
    assert!(stderr.contains("Then run `lisa loop` again."));
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

#[test]
fn loop_reports_a_session_named_after_the_project() {
    // Unnamed, Zellij invents an animal, and that animal is what
    // `zellij list-sessions`, the status bar, and the terminal tab show. The
    // fixture project directory is named `project`, so the report is too.
    let output = run_with_zellij_version_and_path("loop", "zellij 0.44.3", false);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "loop failed: {stderr}");
    assert!(
        stdout.contains("Session: project"),
        "startup report did not name the session after the project:\n{stdout}"
    );
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

/// A run that finished its board and stayed resident, at the boundary an
/// operator actually stands at. On 2026-08-12 this exact sequence — a live
/// session, a scheduler with nothing left to stamp about — printed
/// `Session: lisa-2` and started a second scheduler on one board. The session
/// alone has to stop it, with no registry entry anywhere and no stamp to read.
#[test]
fn loop_refuses_a_board_whose_session_is_still_running() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    let home = temp.path().join("home");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join(".claude.json"),
        "{\"bypassPermissionsModeAccepted\": true}\n",
    )
    .unwrap();
    fs::write(
        root.join(".lisa.toml"),
        format!(
            "version = {:?}\n\n[runtime]\nzellij = \"system\"\n\n[agent]\nclient = \"claude\"\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();

    // A Zellij holding one dead session and one running one, both named for
    // this project directory — the shape `zellij list-sessions` printed that
    // morning.
    write_executable(
        &bin.join("zellij"),
        "#!/bin/sh\n\
         if [ \"${1:-}\" = \"--version\" ]; then printf '%s\\n' 'zellij 0.44.3'; exit 0; fi\n\
         if [ \"${1:-}\" = \"list-sessions\" ]; then\n\
         printf '%s\\n' 'project [Created 19h ago] (EXITED - attach to resurrect)'\n\
         printf '%s\\n' 'project-2 [Created 4h ago]'\n\
         printf '%s\\n' 'unrelated-panda [Created 1h ago]'\n\
         exit 0\n\
         fi\n\
         printf '%s\\n' 'zellij was started' > \"$(dirname \"$0\")/started\"\n\
         exit 0\n",
    );
    write_executable(
        &bin.join("claude"),
        "#!/bin/sh\nif [ \"${1:-}\" = \"--version\" ]; then printf '%s\\n' 'claude 1.0.0'; fi\nexit 0\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("loop")
        .args(["--path", root.to_str().unwrap()])
        .env("PATH", &bin)
        .env("HOME", &home)
        .env_remove("ZELLIJ_SESSION_NAME")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "loop must refuse:\n{stdout}");
    assert!(
        stderr.contains("There is a run already running on this board"),
        "loop did not refuse the running session:\n{stderr}"
    );
    assert!(stderr.contains("project-2 — a Zellij session still open on this board"));
    assert!(stderr.contains("zellij attach project-2"));
    assert!(stderr.contains("zellij kill-session project-2"));
    assert!(
        !stdout.contains("Session: project-3"),
        "the numbering must never get its turn:\n{stdout}"
    );
    assert!(
        !bin.join("started").exists(),
        "a refused loop must not have launched Zellij"
    );
}

/// The other half of the same fact: a session that only *exited* under this
/// board's name is a crashed run, not a second scheduler. It costs the start a
/// number, exactly as it always did.
#[test]
fn loop_starts_past_an_exited_session_of_the_same_name() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    let home = temp.path().join("home");
    fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join(".claude.json"),
        "{\"bypassPermissionsModeAccepted\": true}\n",
    )
    .unwrap();
    fs::write(
        root.join(".lisa.toml"),
        format!(
            "version = {:?}\n\n[runtime]\nzellij = \"system\"\n\n[agent]\nclient = \"claude\"\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    write_executable(
        &bin.join("zellij"),
        "#!/bin/sh\n\
         if [ \"${1:-}\" = \"--version\" ]; then printf '%s\\n' 'zellij 0.44.3'; exit 0; fi\n\
         if [ \"${1:-}\" = \"list-sessions\" ]; then\n\
         printf '%s\\n' 'project [Created 19h ago] (EXITED - attach to resurrect)'\n\
         exit 0\n\
         fi\n\
         exit 0\n",
    );
    write_executable(
        &bin.join("claude"),
        "#!/bin/sh\nif [ \"${1:-}\" = \"--version\" ]; then printf '%s\\n' 'claude 1.0.0'; fi\nexit 0\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("loop")
        .args(["--path", root.to_str().unwrap()])
        .env("PATH", &bin)
        .env("HOME", &home)
        .env_remove("ZELLIJ_SESSION_NAME")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "loop must start: {stderr}");
    assert!(
        stdout.contains("Session: project-2"),
        "a dead session still costs a number and not the start:\n{stdout}"
    );
}
