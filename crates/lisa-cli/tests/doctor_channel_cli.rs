//! `lisa doctor` reporting the gap between a box and its channel (T-068-01-02).
//!
//! The story this belongs to starts with a fleet that had been four weeks
//! behind for four weeks with nothing on any machine saying so. `doctor` is the
//! command that should have said it, and the one version it never reported was
//! Lisa's own.
//!
//! Every case here runs the real binary against a local stand-in for the GitHub
//! releases API and a throwaway machine config, so nothing touches the
//! operator's real channel and nothing reaches the network. The resolution
//! rules are unit-tested in `src/channel.rs`, the states in `src/freshness.rs`;
//! this file pins what an operator actually sees — the row, its remedy, and the
//! same fields under `--json`.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// The version this build is, which is the version every row reports as
/// installed.
const INSTALLED: &str = env!("CARGO_PKG_VERSION");

/// A release list where the newest tag of every kind is far above this build,
/// so the box running the suite is behind on any channel.
fn a_newer_release_exists() -> String {
    format!(
        r#"[
  {{"tag_name": "v99.1.0", "published_at": "2026-08-01T00:00:00Z", "draft": false}},
  {{"tag_name": "v{INSTALLED}", "published_at": "2026-07-19T00:00:00Z", "draft": false}}
]"#
    )
}

/// A release list whose newest tag is exactly this build: a machine on canary
/// that is level with its channel.
fn nothing_newer_exists() -> String {
    format!(
        r#"[
  {{"tag_name": "v{INSTALLED}", "published_at": "2026-07-19T00:00:00Z", "draft": false}},
  {{"tag_name": "v0.1.0", "published_at": "2026-01-01T00:00:00Z", "draft": false}}
]"#
    )
}

/// A local stand-in for the GitHub releases API, serving one fixed body.
struct ReleaseServer {
    url: String,
}

impl ReleaseServer {
    fn start(body: String) -> Self {
        let body: &'static str = Box::leak(body.into_boxed_str());
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a local port");
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                serve(stream, body);
            }
        });

        Self {
            url: format!("http://127.0.0.1:{port}/releases"),
        }
    }
}

fn serve(mut stream: TcpStream, body: &str) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone the connection"));
    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        if line == "\r\n" || line == "\n" {
            break;
        }
        line.clear();
    }

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// The smallest thing `lisa doctor` will look at: a project it recognises.
/// Nothing here is about the board — the row under test is about the machine.
fn project() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
    std::fs::write(
        dir.path().join(".lisa.toml"),
        format!("version = \"{INSTALLED}\"\n"),
    )
    .unwrap();
    dir
}

/// A machine that has recorded a channel.
fn machine_on(channel: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("config.toml"),
        format!("channel = \"{channel}\"\n"),
    )
    .unwrap();
    dir
}

fn doctor(project: &Path, config_dir: &Path, releases_url: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("doctor")
        .arg("--path")
        .arg(project)
        .args(args)
        .env("LISA_CONFIG_DIR", config_dir)
        .env("LISA_RELEASES_URL", releases_url)
        .output()
        .expect("run lisa doctor")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The `lisa` row and everything indented under it, which is where the version,
/// the channel and the remedy live.
fn lisa_row(stdout: &str) -> String {
    let mut lines = stdout
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("lisa "));
    let head = lines.next().unwrap_or_else(|| {
        panic!("doctor printed no lisa row:\n{stdout}");
    });
    let rest: Vec<&str> = lines.take_while(|line| line.starts_with("    ")).collect();
    format!("{head}\n{}", rest.join("\n"))
}

fn document(output: &Output) -> serde_json::Value {
    let stdout = stdout_of(output);
    assert_eq!(
        stdout.trim_end_matches('\n').lines().count(),
        1,
        "--json writes exactly one document: {stdout}"
    );
    serde_json::from_str(&stdout).expect("the document must parse")
}

#[test]
fn a_box_behind_its_channel_is_named_and_told_how_to_settle_it() {
    let server = ReleaseServer::start(a_newer_release_exists());
    let project = project();
    let machine = machine_on("stable");

    let row = lisa_row(&stdout_of(&doctor(
        project.path(),
        machine.path(),
        &server.url,
        &[],
    )));

    assert!(row.contains("behind"), "{row}");
    assert!(row.contains("channel stable"), "{row}");
    assert!(row.contains(&format!("installed {INSTALLED}")), "{row}");
    assert!(
        row.contains("stable resolves to v99.1.0"),
        "the row has to name the version the channel resolves to:\n{row}"
    );
    assert!(
        row.contains("lisa upgrade"),
        "a named gap always carries the command that settles it:\n{row}"
    );
}

#[test]
fn a_box_level_with_its_channel_is_ok_and_quiet() {
    let server = ReleaseServer::start(nothing_newer_exists());
    let project = project();
    let machine = machine_on("canary");

    let row = lisa_row(&stdout_of(&doctor(
        project.path(),
        machine.path(),
        &server.url,
        &[],
    )));

    assert!(row.contains("OK"), "{row}");
    assert!(!row.contains("behind"), "{row}");
    assert!(
        !row.contains("Remedy"),
        "a level box has nothing to do:\n{row}"
    );
    assert!(
        row.contains(&format!("canary resolves to v{INSTALLED}")),
        "{row}"
    );
}

#[test]
fn a_box_with_no_channel_reads_as_unset_rather_than_as_a_silent_stable() {
    let server = ReleaseServer::start(a_newer_release_exists());
    let project = project();
    // No config.toml: the state every machine in the fleet is in today.
    let machine = TempDir::new().unwrap();

    let row = lisa_row(&stdout_of(&doctor(
        project.path(),
        machine.path(),
        &server.url,
        &[],
    )));

    assert!(row.contains("channel unset"), "{row}");
    assert!(row.contains("treated as stable"), "{row}");
    assert!(
        row.contains("lisa upgrade --channel <name>"),
        "a machine with no channel is asked for one, not just moved:\n{row}"
    );
}

/// Port 1 is reserved and never listening, so this is a real failed reach.
#[test]
fn with_no_network_it_says_what_it_could_not_do_and_reports_what_is_installed() {
    let project = project();
    let machine = machine_on("nightly");

    let output = doctor(
        project.path(),
        machine.path(),
        "http://127.0.0.1:1/releases",
        &[],
    );
    let row = lisa_row(&stdout_of(&output));

    assert!(row.contains(&format!("installed {INSTALLED}")), "{row}");
    assert!(row.contains("could not be resolved"), "{row}");
    assert!(
        !row.contains("resolves to v"),
        "an unreachable list must not name a release:\n{row}"
    );
    assert!(
        !row.contains("OK"),
        "a machine that could not look has not been told it is level:\n{row}"
    );
}

#[test]
fn the_json_document_carries_the_same_fields_as_the_row() {
    let server = ReleaseServer::start(a_newer_release_exists());
    let project = project();
    let machine = machine_on("nightly");

    let output = doctor(project.path(), machine.path(), &server.url, &["--json"]);
    let document = document(&output);

    assert_eq!(document["command"], "doctor");
    assert_eq!(document["ok"], true);

    let lisa = &document["data"]["lisa"];
    assert_eq!(lisa["installed"], INSTALLED);
    assert_eq!(lisa["channel"], "nightly");
    assert_eq!(lisa["effective_channel"], "nightly");
    assert_eq!(lisa["state"], "behind");
    assert_eq!(lisa["resolved_tag"], "v99.1.0");
    assert_eq!(lisa["resolved_version"], "99.1.0");
    assert!(lisa["remedy"].as_str().unwrap().contains("lisa upgrade"));

    // The row is in `checks[]` too, so a script reading one array sees every
    // check the report shows, Lisa's own among them.
    let checks = document["data"]["checks"].as_array().unwrap();
    let row = checks
        .iter()
        .find(|check| check["name"] == "lisa")
        .expect("the lisa check is one of doctor's rows");
    assert_eq!(row["status"], "behind");
    assert_eq!(row["required"], false);
}

#[test]
fn the_json_document_reports_an_unset_channel_as_null_and_an_offline_lookup_as_unresolved() {
    let project = project();
    let machine = TempDir::new().unwrap();

    let output = doctor(
        project.path(),
        machine.path(),
        "http://127.0.0.1:1/releases",
        &["--json"],
    );
    let lisa = document(&output)["data"]["lisa"].clone();

    assert!(
        lisa["channel"].is_null(),
        "unset is null, never the channel it is treated as: {lisa}"
    );
    assert_eq!(lisa["effective_channel"], "stable");
    assert_eq!(lisa["state"], "unresolved");
    assert!(lisa["resolved_tag"].is_null());
    assert_eq!(lisa["installed"], INSTALLED);
}

/// A shape nobody can find is not an interface, so the fields are stated where
/// a consumer is told to look.
#[test]
fn the_guide_names_the_fields_a_fleet_is_asked_with() {
    let guide = stdout_of(
        &Command::new(env!("CARGO_BIN_EXE_lisa"))
            .arg("json-guide")
            .output()
            .expect("run lisa json-guide"),
    );

    for marker in [
        "lisa doctor --json",
        "effective_channel",
        "resolved_tag",
        "unresolved",
        "behind",
    ] {
        assert!(guide.contains(marker), "json-guide is missing {marker:?}");
    }
}

/// A machine that is behind still runs, so `doctor` still says so. Making drift
/// a failure would fail every box in the fleet the moment a release is cut,
/// including the desk that cut it.
#[test]
fn being_behind_does_not_turn_doctor_into_a_refusal() {
    let server = ReleaseServer::start(a_newer_release_exists());
    let project = project();
    let machine = machine_on("stable");

    let document = document(&doctor(
        project.path(),
        machine.path(),
        &server.url,
        &["--json"],
    ));

    // The dependency verdict is about this machine's tools, which the suite
    // host may or may not have; what must hold is that the lisa row is not
    // counted as a required failure.
    let lisa = document["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "lisa")
        .unwrap()
        .clone();
    assert_eq!(lisa["required"], false);
    assert_eq!(document["data"]["lisa"]["state"], "behind");
}
