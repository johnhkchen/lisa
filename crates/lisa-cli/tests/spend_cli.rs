//! `lisa spend` exercised as a real process, against real `.lisa/<client>/captures.jsonl`
//! files and fixture `rail`/reach binaries on `PATH` — the ticket's own reproduction
//! recipe: run two boards on two models and have the answer break down by both.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn write_capture(dir: &Path, client: &str, lines: &[String]) {
    let client_dir = dir.join(".lisa").join(client);
    fs::create_dir_all(&client_dir).unwrap();
    fs::write(client_dir.join("captures.jsonl"), lines.join("\n") + "\n").unwrap();
}

fn capture_line(
    session_id: &str,
    captured_at: u64,
    model: Option<&str>,
    input: u64,
    output: u64,
) -> String {
    let model_field = model
        .map(|m| format!(r#","model":"{m}""#))
        .unwrap_or_default();
    format!(
        r#"{{"pane_id":0,"session_id":"{session_id}","captured_at":{captured_at},"input_tokens":{input},"output_tokens":{output},"client":"claude"{model_field}}}"#
    )
}

fn write_executable(path: &Path, script: &str) {
    fs::write(path, script).unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn run_spend(project: &Path, fixture_bin: Option<&Path>, extra_args: &[&str]) -> Output {
    let path_var = match fixture_bin {
        Some(bin) => format!("{}:/usr/bin:/bin", bin.display()),
        None => "/usr/bin:/bin".to_string(),
    };
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("spend")
        .arg("--path")
        .arg(project)
        .args(extra_args)
        .env("PATH", path_var)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "lisa spend failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn without_rail_on_path_it_falls_back_to_this_project_and_says_so() {
    let project = tempfile::tempdir().unwrap();
    write_capture(
        project.path(),
        "claude",
        &[capture_line("s1", now(), Some("claude-opus-5"), 1000, 100)],
    );

    let output = run_spend(project.path(), None, &[]);
    let text = stdout(&output);

    assert!(text.contains("rail` was not available"));
    assert!(text.contains("here: ok"));
    assert!(text.contains("claude-opus-5: 1000 in / 100 out (1 turn)"));
}

#[test]
fn two_boards_two_models_two_machines_break_down_by_both() {
    let scratch = tempfile::tempdir().unwrap();
    let board_a = scratch.path().join("board-a");
    let board_b = scratch.path().join("board-b");
    fs::create_dir_all(&board_a).unwrap();
    fs::create_dir_all(&board_b).unwrap();

    let when = now();
    write_capture(
        &board_a,
        "claude",
        &[capture_line("a1", when, Some("claude-opus-5"), 1000, 100)],
    );
    write_capture(
        &board_b,
        "claude",
        &[capture_line("b1", when, Some("claude-sonnet-5"), 300, 30)],
    );

    let fixture_bin = scratch.path().join("fixture-bin");
    fs::create_dir_all(&fixture_bin).unwrap();

    write_executable(
        &fixture_bin.join("rail"),
        &format!(
            r#"#!/bin/sh
cat <<JSON
{{"ok":true,"data":{{"hosts":[
  {{"name":"here","reach":"","local":true,"used":["{}"]}},
  {{"name":"mini","reach":"fake-ssh mini","local":false,"used":["{}"]}}
]}}}}
JSON
"#,
            board_a.display(),
            board_b.display()
        ),
    );

    // A faithful-enough ssh stand-in: real ssh joins argv with spaces and
    // hands the result to a shell on the far side. This does the same, minus
    // the network, so `echo MARKER; cat f1 f2` is interpreted once, remotely.
    write_executable(
        &fixture_bin.join("fake-ssh"),
        "#!/bin/sh\nshift\nexec sh -c \"$*\"\n",
    );

    let output = run_spend(&scratch.path().join("board-a"), Some(&fixture_bin), &[]);
    let text = stdout(&output);

    assert!(text.contains("here: ok"));
    assert!(text.contains("mini (fake-ssh mini): ok"));
    assert!(text.contains("claude-opus-5: 1000 in / 100 out (1 turn)"));
    assert!(text.contains("claude-sonnet-5: 300 in / 30 out (1 turn)"));
    assert!(text.contains("here: 1000 in / 100 out (1 turn)"));
    assert!(text.contains("mini (fake-ssh mini): 300 in / 30 out (1 turn)"));
    assert!(text.contains("total: 1300 in / 130 out (2 turns)"));
}

#[test]
fn an_unreachable_machine_is_named_with_its_reason_and_left_out_of_the_totals() {
    let scratch = tempfile::tempdir().unwrap();
    let board_a = scratch.path().join("board-a");
    fs::create_dir_all(&board_a).unwrap();
    write_capture(
        &board_a,
        "claude",
        &[capture_line("a1", now(), Some("claude-opus-5"), 5, 1)],
    );

    let fixture_bin = scratch.path().join("fixture-bin");
    fs::create_dir_all(&fixture_bin).unwrap();

    write_executable(
        &fixture_bin.join("rail"),
        &format!(
            r#"#!/bin/sh
cat <<JSON
{{"ok":true,"data":{{"hosts":[
  {{"name":"here","reach":"","local":true,"used":["{}"]}},
  {{"name":"ghost","reach":"fake-ssh ghost","local":false,"used":["/nowhere"]}}
]}}}}
JSON
"#,
            board_a.display()
        ),
    );
    write_executable(
        &fixture_bin.join("fake-ssh"),
        "#!/bin/sh\necho \"ssh: connect to host ghost port 22: Connection refused\" >&2\nexit 255\n",
    );

    let output = run_spend(&board_a, Some(&fixture_bin), &[]);
    let text = stdout(&output);

    assert!(text.contains("ghost (fake-ssh ghost): unreachable — ssh: connect to host ghost port 22: Connection refused"));
    assert!(text.contains("could not be reached and is not counted above"));
    // Only the reachable host's tokens are in the total.
    assert!(text.contains("total: 5 in / 1 out (1 turn)"));
}

#[test]
fn a_host_that_never_answers_is_abandoned_at_its_timeout_not_hung_on_forever() {
    let scratch = tempfile::tempdir().unwrap();
    let board_a = scratch.path().join("board-a");
    fs::create_dir_all(&board_a).unwrap();
    write_capture(
        &board_a,
        "claude",
        &[capture_line("a1", now(), Some("claude-opus-5"), 5, 1)],
    );

    let fixture_bin = scratch.path().join("fixture-bin");
    fs::create_dir_all(&fixture_bin).unwrap();
    write_executable(
        &fixture_bin.join("rail"),
        &format!(
            r#"#!/bin/sh
cat <<JSON
{{"ok":true,"data":{{"hosts":[
  {{"name":"here","reach":"","local":true,"used":["{}"]}},
  {{"name":"ghost","reach":"fake-ssh ghost","local":false,"used":["/nowhere"]}}
]}}}}
JSON
"#,
            board_a.display()
        ),
    );
    write_executable(&fixture_bin.join("fake-ssh"), "#!/bin/sh\nsleep 30\n");

    let output = run_spend(&board_a, Some(&fixture_bin), &["--reach-timeout-secs", "1"]);
    let text = stdout(&output);

    assert!(text.contains("ghost (fake-ssh ghost): unreachable — timed out after 1s"));
}
