//! `lisa spend --guard` exercised as a real process — the ticket's own
//! reproduction recipe (T-072-01-02): set the allowance low, run a board past
//! it, and have the desk do the chosen thing ("it stops, but never starts or
//! changes") and say so.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn write_capture(dir: &Path, input: u64, output: u64) {
    let client_dir = dir.join(".lisa").join("claude");
    fs::create_dir_all(&client_dir).unwrap();
    let when = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let line = format!(
        r#"{{"pane_id":0,"session_id":"s1","captured_at":{when},"input_tokens":{input},"output_tokens":{output},"client":"claude","model":"claude-opus-5"}}"#
    );
    fs::write(client_dir.join("captures.jsonl"), line + "\n").unwrap();
}

fn write_config(dir: &Path, body: &str) {
    fs::write(dir.join(".lisa.toml"), body).unwrap();
}

fn write_executable(path: &Path, script: &str) {
    fs::write(path, script).unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

/// A fixture `rail` that answers `desk --hosts --json` with this one project
/// (so the guard reads a clean, fully-reachable desk) and logs every
/// `rail tell` invocation's argv, one line per call, to `tell_log`.
fn write_rail_fixture(bin_dir: &Path, project: &Path, tell_log: &Path) {
    write_executable(
        &bin_dir.join("rail"),
        &format!(
            r#"#!/bin/sh
if [ "$1" = "desk" ]; then
  cat <<JSON
{{"ok":true,"data":{{"hosts":[{{"name":"here","reach":"","local":true,"used":["{}"]}}]}}}}
JSON
elif [ "$1" = "tell" ]; then
  echo "$@" >> "{}"
fi
"#,
            project.display(),
            tell_log.display()
        ),
    );
}

fn run_guard(project: &Path, fixture_bin: Option<&Path>) -> Output {
    let path_var = match fixture_bin {
        Some(bin) => format!("{}:/usr/bin:/bin", bin.display()),
        None => "/usr/bin:/bin".to_string(),
    };
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("spend")
        .arg("--guard")
        .arg("--path")
        .arg(project)
        .env("PATH", path_var)
        .env_remove("ZELLIJ_SESSION_NAME")
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "lisa spend --guard failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn a_low_priority_board_over_its_allowance_stops_and_tells_rail() {
    let scratch = tempfile::tempdir().unwrap();
    let project = scratch.path().join("board");
    fs::create_dir_all(&project).unwrap();
    write_capture(&project, 900, 50); // 950 tokens, 95% of 1000
    write_config(
        &project,
        "[scheduling]\npriority = \"low\"\nweekly_token_allowance = 1000\n",
    );

    let fixture_bin = scratch.path().join("fixture-bin");
    fs::create_dir_all(&fixture_bin).unwrap();
    let tell_log = scratch.path().join("tell.log");
    write_rail_fixture(&fixture_bin, &project, &tell_log);

    let output = run_guard(&project, Some(&fixture_bin));
    let text = stdout(&output);

    assert!(
        text.contains("950 of 1000 tokens spent this week (95%)"),
        "{text}"
    );
    assert!(text.contains("Stopping its loop."), "{text}");
    assert!(
        text.contains("No scheduler has recorded itself here; nothing to stop."),
        "{text}"
    );
    assert!(text.contains("Told rail (loop-degraded):"), "{text}");

    let logged = fs::read_to_string(&tell_log).unwrap_or_default();
    assert!(logged.contains("loop-degraded"), "{logged}");
    assert!(logged.contains("low-priority board"), "{logged}");
    assert!(
        logged.contains("Lisa found nothing running to stop"),
        "{logged}"
    );
}

#[test]
fn a_board_with_no_configured_priority_is_never_stopped() {
    let scratch = tempfile::tempdir().unwrap();
    let project = scratch.path().join("board");
    fs::create_dir_all(&project).unwrap();
    write_capture(&project, 900, 50); // 95% of 1000, same as above
    write_config(&project, "[scheduling]\nweekly_token_allowance = 1000\n");

    let fixture_bin = scratch.path().join("fixture-bin");
    fs::create_dir_all(&fixture_bin).unwrap();
    let tell_log = scratch.path().join("tell.log");
    write_rail_fixture(&fixture_bin, &project, &tell_log);

    let output = run_guard(&project, Some(&fixture_bin));
    let text = stdout(&output);

    assert!(text.contains("[scheduling].priority is medium"), "{text}");
    assert!(text.contains("Nothing to do."), "{text}");
    assert!(!text.contains("Stopping its loop."), "{text}");
    // Not eligible means the guard never even reaches the point of telling
    // rail — nothing acted, nothing to report.
    assert!(!tell_log.exists() || fs::read_to_string(&tell_log).unwrap().is_empty());
}

#[test]
fn frontloaded_spend_under_the_threshold_is_reported_not_acted_on() {
    let scratch = tempfile::tempdir().unwrap();
    let project = scratch.path().join("board");
    fs::create_dir_all(&project).unwrap();
    write_capture(&project, 700, 100); // 800 tokens, 80% of 1000 — under 90%
    write_config(
        &project,
        "[scheduling]\npriority = \"low\"\nweekly_token_allowance = 1000\n",
    );

    let fixture_bin = scratch.path().join("fixture-bin");
    fs::create_dir_all(&fixture_bin).unwrap();
    let tell_log = scratch.path().join("tell.log");
    write_rail_fixture(&fixture_bin, &project, &tell_log);

    let output = run_guard(&project, Some(&fixture_bin));
    let text = stdout(&output);

    assert!(text.contains("under the 90% mark"), "{text}");
    assert!(text.contains("spending early is not an error"), "{text}");
    assert!(!text.contains("Stopping its loop."), "{text}");
}

#[test]
fn no_configured_allowance_leaves_the_guard_inert_however_much_was_spent() {
    let scratch = tempfile::tempdir().unwrap();
    let project = scratch.path().join("board");
    fs::create_dir_all(&project).unwrap();
    write_capture(&project, 10_000, 10_000);
    write_config(&project, "[scheduling]\npriority = \"low\"\n");

    let fixture_bin = scratch.path().join("fixture-bin");
    fs::create_dir_all(&fixture_bin).unwrap();
    let tell_log = scratch.path().join("tell.log");
    write_rail_fixture(&fixture_bin, &project, &tell_log);

    let output = run_guard(&project, Some(&fixture_bin));
    let text = stdout(&output);

    assert!(text.contains("weekly_token_allowance is not set"), "{text}");
    assert!(!text.contains("Stopping its loop."), "{text}");
}

#[test]
fn an_unreachable_machine_refuses_to_act_even_over_threshold() {
    let scratch = tempfile::tempdir().unwrap();
    let project = scratch.path().join("board");
    fs::create_dir_all(&project).unwrap();
    write_capture(&project, 900, 50); // 95% of 1000, alone
    write_config(
        &project,
        "[scheduling]\npriority = \"low\"\nweekly_token_allowance = 1000\n",
    );

    let fixture_bin = scratch.path().join("fixture-bin");
    fs::create_dir_all(&fixture_bin).unwrap();
    let tell_log = scratch.path().join("tell.log");
    write_executable(
        &fixture_bin.join("rail"),
        &format!(
            r#"#!/bin/sh
if [ "$1" = "desk" ]; then
  cat <<JSON
{{"ok":true,"data":{{"hosts":[
  {{"name":"here","reach":"","local":true,"used":["{}"]}},
  {{"name":"ghost","reach":"fake-ssh ghost","local":false,"used":["/nowhere"]}}
]}}}}
JSON
elif [ "$1" = "tell" ]; then
  echo "$@" >> "{}"
fi
"#,
            project.display(),
            tell_log.display()
        ),
    );
    write_executable(
        &fixture_bin.join("fake-ssh"),
        "#!/bin/sh\necho \"connection refused\" >&2\nexit 255\n",
    );

    let output = run_guard(&project, Some(&fixture_bin));
    let text = stdout(&output);

    assert!(text.contains("could not be reached this pass"), "{text}");
    assert!(!text.contains("Stopping its loop."), "{text}");
    assert!(!tell_log.exists() || fs::read_to_string(&tell_log).unwrap().is_empty());
}
