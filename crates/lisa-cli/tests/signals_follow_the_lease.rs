//! The field failure of S-061-01, reproduced end to end and then fixed.
//!
//! One desk, two projects. A loop runs in project A and leases a pane to an
//! agent. The agent's ticket sends it to read project B — ordinary work, and the
//! thing Lisa should never need to forbid. Every lifecycle hook then fires with
//! its working directory inside B.
//!
//! Before the fix, `SIGNAL_DIR=".lisa/signals"` resolved against that working
//! directory: A lost the liveness it was waiting for, and B gained a fresh
//! `pane-1.*` file from a pane numbering it does not share — enough for B's own
//! launcher to refuse a run on evidence that was never its own. `mkdir -p` made
//! the second half silent, creating the directory to hold it wherever the agent
//! happened to be.
//!
//! The hooks are not read here, they are run: `lisa init` installs them into a
//! real project, `/bin/sh` executes the installed files from inside a different
//! repository, and the assertions are about which tree the files landed in.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const PANE: &str = "1";
const TICKET: &str = "T-061-01-01";
const ATTEMPT: &str = "1";
const LEASE: &str = r#"{"ticket_id":"T-061-01-01","attempt_id":1}"#;

/// `lisa init` a project at `root`, exactly as an operator would.
fn init_project(root: &Path, home: &Path) {
    fs::create_dir_all(root).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["init", "--no-history", "--path"])
        .arg(root)
        .env("HOME", home)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "lisa init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Run one installed hook of the leased project `project`, from `cwd`, with the
/// pane environment a launch line exports. `project_env` is what the hook sees
/// as `$LISA_PROJECT`; `None` is a pane launched before Lisa exported one, and
/// an operator's own session.
fn run_hook(
    project: &Path,
    hook: &str,
    cwd: &Path,
    project_env: Option<&Path>,
    payload: &str,
) -> bool {
    let mut command = Command::new("/bin/sh");
    command
        .arg(project.join(".lisa/hooks").join(hook))
        .current_dir(cwd)
        .env_remove("LISA_PROJECT")
        .env("LISA_PANE_ID", PANE)
        .env("LISA_TICKET_ID", TICKET)
        .env("LISA_ATTEMPT_ID", ATTEMPT)
        .env("LISA_BIN", env!("CARGO_BIN_EXE_lisa"))
        .stdin(Stdio::piped());
    if let Some(project_env) = project_env {
        command.env("LISA_PROJECT", project_env);
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    let status = child.wait().unwrap();
    assert!(
        status.success(),
        "{hook} must never fail its caller's turn (exit {status})"
    );
    true
}

/// Every signal file present under a project's `.lisa/signals`, sorted. The
/// scheduler-owned `.lease` marker is excluded: it is written by the plugin, not
/// by a hook, and the fixture places it deliberately.
fn signals(root: &Path) -> Vec<String> {
    let dir = root.join(".lisa/signals");
    let mut names: Vec<String> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name != "pane-1.lease")
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
}

/// The five hooks a Claude or Codex session fires during ordinary work, with the
/// payload each one's event arrives with.
fn lifecycle_hooks() -> Vec<(&'static str, &'static str)> {
    vec![
        ("on-start.sh", ""),
        (
            "on-ack.sh",
            r#"{"prompt":"read the other repository","session_id":"s-1"}"#,
        ),
        ("on-heartbeat.sh", ""),
        ("on-idle.sh", ""),
        (
            "on-stop.sh",
            r#"{"session_id":"s-1","transcript_path":"/nonexistent/transcript.jsonl"}"#,
        ),
    ]
}

struct Desk {
    _temp: tempfile::TempDir,
    /// The project whose loop leased the pane.
    a: PathBuf,
    /// A second Lisa project on the same desk. The agent walks into it and
    /// nothing about it is unusual: it is a working board with its own signals.
    b: PathBuf,
    /// A directory that is not a Lisa project at all.
    plain: PathBuf,
}

fn desk() -> Desk {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let a = temp.path().join("steer");
    let b = temp.path().join("screen-design");
    let plain = temp.path().join("some-library");
    init_project(&a, &home);
    init_project(&b, &home);
    fs::create_dir_all(&plain).unwrap();
    fs::write(a.join(".lisa/signals/pane-1.lease"), LEASE).unwrap();
    Desk {
        _temp: temp,
        a,
        b,
        plain,
    }
}

/// The reproduction, and the fix in the same run: an agent standing in another
/// repository signals into the project its pane was leased from.
#[test]
fn an_agent_in_another_repository_signals_into_the_project_it_was_leased_from() {
    let desk = desk();
    let before_b = signals(&desk.b);

    for (hook, payload) in lifecycle_hooks() {
        run_hook(&desk.a, hook, &desk.b, Some(&desk.a), payload);
    }

    assert_eq!(
        signals(&desk.a),
        vec![
            "pane-1.ack",
            "pane-1.alive",
            "pane-1.heartbeat",
            "pane-1.idle",
            "pane-1.started",
            "pane-1.stopped",
        ],
        "the leased project gets every signal its pane produced"
    );
    assert_eq!(
        signals(&desk.b),
        before_b,
        "the repository the agent walked into gains nothing"
    );

    // The heartbeat still proves identity against the marker in the project
    // that owns the lease — the marker in B (there is none) is not consulted.
    assert_eq!(
        fs::read_to_string(desk.a.join(".lisa/signals/pane-1.heartbeat")).unwrap(),
        LEASE
    );
    assert_eq!(
        fs::read_to_string(desk.a.join(".lisa/signals/pane-1.started")).unwrap(),
        LEASE
    );

    // The Stop hook's usage ledger follows the same lease, for the same reason.
    assert!(desk.a.join(".lisa/claude/no-captures.jsonl").exists());
    assert!(!desk.b.join(".lisa/claude").exists());
}

/// The half that made it silent. A hook that cannot name its lease writes
/// nothing and creates nothing — in particular it does not `mkdir -p` a signals
/// directory into a repository Lisa was never pointed at.
#[test]
fn a_hook_that_cannot_name_its_lease_writes_nothing_anywhere() {
    let desk = desk();
    let before_b = signals(&desk.b);

    for (hook, payload) in lifecycle_hooks() {
        run_hook(&desk.a, hook, &desk.plain, None, payload);
        run_hook(&desk.a, hook, &desk.b, None, payload);
    }

    assert!(
        signals(&desk.a).is_empty(),
        "a pane that cannot name its project publishes nothing"
    );
    assert_eq!(signals(&desk.b), before_b);
    assert!(
        !desk.plain.join(".lisa").exists(),
        "no signals directory may appear in a directory Lisa does not manage"
    );
}

/// An operator's own session — no pane, no lease — stays silent exactly as
/// `on-stop.sh` has always said it should, wherever it is standing.
#[test]
fn an_operators_own_session_still_writes_nothing() {
    let desk = desk();

    for (hook, payload) in lifecycle_hooks() {
        let mut child = Command::new("/bin/sh")
            .arg(desk.a.join(".lisa/hooks").join(hook))
            .current_dir(&desk.a)
            .env_remove("LISA_PANE_ID")
            .env_remove("LISA_PROJECT")
            .env_remove("LISA_TICKET_ID")
            .env_remove("LISA_ATTEMPT_ID")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(payload.as_bytes())
            .unwrap();
        assert!(child.wait().unwrap().success());
    }

    assert!(signals(&desk.a).is_empty());
    assert!(!desk.a.join(".lisa/claude").exists());
}

/// The desk this has always worked on, unchanged: one loop, one project, the
/// agent standing in the project it was leased from. This is why the relative
/// path survived so long — cwd and lease were the same directory.
#[test]
fn a_single_project_desk_behaves_exactly_as_before() {
    let desk = desk();

    for (hook, payload) in lifecycle_hooks() {
        run_hook(&desk.a, hook, &desk.a, Some(&desk.a), payload);
    }

    assert_eq!(
        signals(&desk.a),
        vec![
            "pane-1.ack",
            "pane-1.alive",
            "pane-1.heartbeat",
            "pane-1.idle",
            "pane-1.started",
            "pane-1.stopped",
        ]
    );
    assert_eq!(
        fs::read_to_string(desk.a.join(".lisa/signals/pane-1.ack")).unwrap(),
        r#"{"prompt":"read the other repository","session_id":"s-1"}"#,
        "the ack hook still preserves its payload byte for byte"
    );
}

/// Everything above runs the script directly. A client does not: it runs the
/// **binding** from `.claude/settings.local.json`, in the agent's working
/// directory. A binding that names `.lisa/hooks/on-stop.sh` therefore reaches
/// whatever is under the agent's feet — the other board's copy of the script,
/// or, in a directory that is not a Lisa project, nothing at all. Both are
/// driven here, through the bindings `lisa init` actually installed.
#[test]
fn the_installed_bindings_reach_this_projects_hooks_from_anywhere() {
    let desk = desk();
    let settings: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(desk.a.join(".claude/settings.local.json")).unwrap(),
    )
    .unwrap();
    let binding = |script: &str| -> String {
        settings["hooks"]
            .as_object()
            .unwrap()
            .values()
            .flat_map(|entries| entries.as_array().unwrap())
            .flat_map(|entry| entry["hooks"].as_array().unwrap())
            .filter_map(|hook| hook["command"].as_str())
            .find(|command| command.contains(script))
            .unwrap_or_else(|| panic!("{script} is bound"))
            .to_string()
    };

    for cwd in [&desk.b, &desk.plain] {
        for (script, payload) in lifecycle_hooks() {
            let mut child = Command::new("/bin/sh")
                .args(["-c", &binding(script)])
                .current_dir(cwd)
                .env("LISA_PROJECT", &desk.a)
                .env("LISA_PANE_ID", PANE)
                .env("LISA_TICKET_ID", TICKET)
                .env("LISA_ATTEMPT_ID", ATTEMPT)
                .env("LISA_BIN", env!("CARGO_BIN_EXE_lisa"))
                .stdin(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(payload.as_bytes())
                .unwrap();
            child.wait().unwrap();
        }
    }

    assert_eq!(
        signals(&desk.a),
        vec![
            "pane-1.ack",
            "pane-1.alive",
            "pane-1.heartbeat",
            "pane-1.idle",
            "pane-1.started",
            "pane-1.stopped",
        ],
        "a binding run from a stranger's tree still reaches this project's hooks"
    );
    assert!(!desk.plain.join(".lisa").exists());
}

/// The `AskUserQuestion` binding writes a signal too, from inside
/// `.claude/settings.local.json` rather than from a script. It obeys the same
/// rule: the pane's question parks the pane in the project that leased it.
#[test]
fn the_question_binding_parks_the_pane_in_the_leased_project() {
    let desk = desk();
    let settings: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(desk.a.join(".claude/settings.local.json")).unwrap(),
    )
    .unwrap();
    let command = settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .expect("the AskUserQuestion binding is installed")
        .to_string();

    let mut child = Command::new("/bin/sh")
        .args(["-c", &command])
        .current_dir(&desk.b)
        .env("LISA_PROJECT", &desk.a)
        .env("LISA_PANE_ID", PANE)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"tool_input":{"questions":[{"question":"which desk?","header":"Desk"}]}}"#)
        .unwrap();
    assert!(child.wait().unwrap().success());

    assert!(desk.a.join(".lisa/signals/pane-1.awaiting").exists());
    assert!(!desk.b.join(".lisa/signals/pane-1.awaiting").exists());
    assert!(desk.a.join(".lisa/run-events.jsonl").exists());
    assert!(!desk.b.join(".lisa/run-events.jsonl").exists());
}
