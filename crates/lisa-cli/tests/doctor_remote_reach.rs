//! `lisa doctor` saying whether this board can reach its remote (T-073-01-01).
//!
//! The week this was written, an operator moved 98 commits by hand because the
//! session that made them could not push, and nothing in `doctor` had said so.
//! Every case here is one of the answers that row can give, run through the
//! real binary against local stand-ins, so nothing reaches the network and
//! nothing on the machine is touched.
//!
//! The classification rules are unit-tested in `src/remote_reach.rs`; this
//! file pins what an operator actually reads: the word, the road it names, and
//! that none of it stops a run.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const INSTALLED: &str = env!("CARGO_PKG_VERSION");

/// A board: a project Lisa recognises, with history and an identity, so the
/// only thing under test is the road out.
fn board() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
    std::fs::write(
        dir.path().join(".lisa.toml"),
        format!("version = \"{INSTALLED}\"\n"),
    )
    .unwrap();
    git(dir.path(), &["init", "--quiet", "-b", "main"]);
    git(dir.path(), &["config", "user.name", "Test Operator"]);
    git(dir.path(), &["config", "user.email", "test@example.com"]);
    git(
        dir.path(),
        &["commit", "--quiet", "--allow-empty", "-m", "start"],
    );
    dir
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        status.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

/// A remote that is really there and really accepts a push.
fn a_bare_repository() -> TempDir {
    let dir = TempDir::new().unwrap();
    let status = Command::new("git")
        .args(["init", "--bare", "--quiet"])
        .arg(dir.path())
        .status()
        .expect("run git init --bare");
    assert!(status.success());
    dir
}

/// A remote that answers, and says no: an HTTP server that refuses every
/// request the way a credential without write access does.
struct RefusingServer {
    url: String,
}

impl RefusingServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a local port");
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                refuse(stream);
            }
        });
        Self {
            url: format!("http://127.0.0.1:{port}/board.git"),
        }
    }
}

fn refuse(mut stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone the connection"));
    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        if line == "\r\n" || line == "\n" {
            break;
        }
        line.clear();
    }
    let _ = stream
        .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    let _ = stream.flush();
}

/// A port nothing is listening on: the road, not the credential, is missing.
fn a_closed_port() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a local port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}/board.git")
}

fn doctor(project: &Path) -> Output {
    let config_dir = TempDir::new().unwrap();
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("doctor")
        .arg("--path")
        .arg(project)
        .env("LISA_CONFIG_DIR", config_dir.path())
        // Nothing in this suite may consult the operator's own git config,
        // credential helpers or proxy — the answers under test are about the
        // board in front of it.
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("http_proxy", "")
        .env("https_proxy", "")
        .env("all_proxy", "")
        .output()
        .expect("run lisa doctor")
}

/// The remote section and everything under it.
fn remote_section(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let start = stdout
        .find("Checking whether work made here can reach its remote")
        .unwrap_or_else(|| panic!("doctor printed no remote section:\n{stdout}"));
    let rest = &stdout[start..];
    let end = rest[1..]
        .find("\nChecking ")
        .map(|at| at + 1)
        .unwrap_or(rest.len());
    rest[..end].to_string()
}

#[test]
fn a_board_that_can_push_says_so_and_names_the_road() {
    let board = board();
    let remote = a_bare_repository();
    git(
        board.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );

    let output = doctor(board.path());
    let section = remote_section(&output);

    assert!(section.contains("remote"), "{section}");
    assert!(
        section.contains("OK"),
        "a reachable board passes:\n{section}"
    );
    assert!(
        section.contains("push over a local path"),
        "the row names the road it went over:\n{section}"
    );
    assert!(
        section.contains("dry-run push"),
        "the row says what was measured:\n{section}"
    );
}

#[test]
fn a_refused_push_is_a_no_that_names_push_and_the_road() {
    let board = board();
    let server = RefusingServer::start();
    git(board.path(), &["remote", "add", "origin", &server.url]);

    let output = doctor(board.path());
    let section = remote_section(&output);

    assert!(
        section.contains("remote       no"),
        "a refused push is a no:\n{section}"
    );
    assert!(
        section.contains("was refused"),
        "the row says what was refused:\n{section}"
    );
    assert!(
        section.contains("over http"),
        "the row names the protocol:\n{section}"
    );
    assert!(
        !section.to_lowercase().contains("fetch"),
        "push is what was measured, so push is the word used:\n{section}"
    );
    assert!(
        section.contains("Remedy:"),
        "a no carries what to do about it:\n{section}"
    );
}

#[test]
fn a_board_whose_road_is_closed_does_not_block_a_run() {
    let board = board();
    let server = RefusingServer::start();
    git(board.path(), &["remote", "add", "origin", &server.url]);

    let output = doctor(board.path());

    assert!(
        output.status.success(),
        "doctor reports; the operator decides. A board that cannot push is not a failed doctor:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn no_answer_is_cannot_tell_and_never_a_no() {
    let board = board();
    git(board.path(), &["remote", "add", "origin", &a_closed_port()]);

    let output = doctor(board.path());
    let section = remote_section(&output);

    assert!(
        section.contains("cannot tell"),
        "a road that never answered is not a verdict on a credential:\n{section}"
    );
    assert!(
        !section.contains("remote       no"),
        "`cannot tell` is a distinct outcome from `no`:\n{section}"
    );
    assert!(
        !section.contains("Remedy:"),
        "there is nothing to fix about a network that did not answer:\n{section}"
    );
}

#[test]
fn a_board_with_no_remote_is_not_flagged() {
    let board = board();

    let output = doctor(board.path());
    let section = remote_section(&output);

    assert!(
        section.contains("skipped"),
        "plenty of work is local, and local is fine:\n{section}"
    );
    assert!(
        section.contains("fine way to run a board"),
        "the row says plainly that this is not a problem:\n{section}"
    );
    assert!(output.status.success(), "{section}");
}

#[test]
fn the_row_says_which_shell_it_measured() {
    let board = board();
    let remote = a_bare_repository();
    git(
        board.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );

    let section = remote_section(&doctor(board.path()));

    assert!(
        section.contains("Measured in this shell"),
        "a check that runs in a better shell than the work does has to say so:\n{section}"
    );
    assert!(
        section.contains("not for that pane"),
        "the row must not imply it measured the pane a run starts:\n{section}"
    );
}

#[test]
fn the_probe_writes_nothing_to_the_remote() {
    let board = board();
    let remote = a_bare_repository();
    git(
        board.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );

    doctor(board.path());

    let refs = Command::new("git")
        .arg("-C")
        .arg(remote.path())
        .args(["for-each-ref"])
        .output()
        .expect("read the remote's refs");
    assert!(
        String::from_utf8_lossy(&refs.stdout).trim().is_empty(),
        "doctor creates no branch and pushes nothing: {}",
        String::from_utf8_lossy(&refs.stdout)
    );
}

#[test]
fn a_script_reads_the_same_answer_a_person_does() {
    let board = board();
    let server = RefusingServer::start();
    git(board.path(), &["remote", "add", "origin", &server.url]);

    let config_dir = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["doctor", "--json", "--path"])
        .arg(board.path())
        .env("LISA_CONFIG_DIR", config_dir.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("http_proxy", "")
        .env("https_proxy", "")
        .env("all_proxy", "")
        .output()
        .expect("run lisa doctor --json");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let document: serde_json::Value = serde_json::from_str(&stdout).expect("the document parses");
    let checks = document["data"]["checks"].as_array().expect("checks");
    let remote = checks
        .iter()
        .find(|check| check["name"] == "remote")
        .unwrap_or_else(|| panic!("no remote row in the document: {stdout}"));

    assert_eq!(remote["status"], "unreachable", "{remote}");
    assert_eq!(remote["required"], false, "{remote}");
    assert_eq!(
        document["data"]["verdict"], "passed",
        "a board that cannot push is still a passing doctor: {stdout}"
    );
}
