//! The unattended arrangement end to end (T-068-01-03).
//!
//! Everything here runs the real binary against a local stand-in for the
//! releases API, a throwaway machine config directory, and a stand-in `zellij`
//! that says whether this machine is working. Nothing reaches the network,
//! nothing installs an artifact, and nothing touches the operator's own channel
//! or launch agents.
//!
//! The three properties the ticket turns on are here: a cycle never lands under
//! a live run, a tag that has not soaked is visible and *not* taken, and a
//! failed cycle is loud — it writes a record, exits non-zero, and hands the
//! whole record to the alarm the machine was given.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// The version this build is: what every cycle starts from.
const INSTALLED: &str = env!("CARGO_PKG_VERSION");

/// The newest tag in the fixtures below, as a tag string.
///
/// Derived from [`INSTALLED`] rather than written out. A literal newest tag
/// stops being the version under test the moment the workspace version is
/// bumped, and then a level machine reads as one release behind — so the suite
/// fails on exactly the operation it exists to protect. It did: `0.5.0-rc.2`
/// was hardcoded here and three tests broke on the `0.5.0-rc.3` bump.
fn newest_tag() -> String {
    format!("v{INSTALLED}")
}

/// A release list whose newest tag is the version under test, soaked long ago.
/// A machine on nightly is level with this list and has nothing to do.
fn level_list() -> String {
    format!(
        r#"[
  {{"tag_name": "{}", "published_at": "2026-08-09T00:00:00Z", "draft": false}},
  {{"tag_name": "v0.4.4", "published_at": "2026-07-19T00:00:00Z", "draft": false}}
]"#,
        newest_tag()
    )
}

/// The same list with a tag published far enough ahead that it can never have
/// soaked, whatever day the suite runs on. `nightly` must see it and refuse it.
fn unsoaked_list() -> String {
    format!(
        r#"[
  {{"tag_name": "v0.9.0-rc.1", "published_at": "2099-01-01T00:00:00Z", "draft": false}},
  {{"tag_name": "{}", "published_at": "2026-08-09T00:00:00Z", "draft": false}},
  {{"tag_name": "v0.4.4", "published_at": "2026-07-19T00:00:00Z", "draft": false}}
]"#,
        newest_tag()
    )
}

/// A local stand-in for the GitHub releases API, serving one fixed body.
struct ReleaseServer {
    url: String,
}

impl ReleaseServer {
    fn start(body: String) -> Self {
        // The serving thread outlives this call, and the fixtures are now built
        // at runtime from INSTALLED rather than being literals.
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

/// A `zellij` that reports exactly the sessions a test wants it to.
///
/// The machine running the suite has its own Zellij and quite possibly its own
/// running sessions, so every test puts one of these first on `PATH` rather
/// than asking the real one.
struct FakeZellij {
    _dir: TempDir,
    path: PathBuf,
}

impl FakeZellij {
    fn listing(listing: &str) -> Self {
        let dir = TempDir::new().unwrap();
        let script = dir.path().join("zellij");
        std::fs::write(
            &script,
            format!("#!/bin/sh\ncat <<'LIST'\n{listing}\nLIST\n"),
        )
        .expect("write the stand-in zellij");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let path = dir.path().to_path_buf();
        Self { _dir: dir, path }
    }

    /// A machine with nothing running on it.
    fn idle() -> Self {
        Self::listing("lisa [Created 1day ago] (EXITED - attach to resurrect)")
    }

    /// A machine in the middle of a run.
    fn working() -> Self {
        Self::listing("board [Created 34m ago] (current)")
    }

    /// A `PATH` with this Zellij on it and the real one nowhere near it.
    fn path_env(&self) -> String {
        format!("{}:/usr/bin:/bin", self.path.display())
    }
}

/// Run `lisa nightly …` on a machine assembled out of the parts above.
fn nightly(config: &Path, zellij: &FakeZellij, releases_url: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("nightly")
        .args(args)
        // A thrown-away HOME too: a cycle reads the version of the lisa in
        // `~/.local/bin`, and that must not be the one this machine happens to
        // have installed.
        .env("HOME", config)
        .env("LISA_CONFIG_DIR", config)
        .env("LISA_RELEASES_URL", releases_url)
        .env("PATH", zellij.path_env())
        // No system log entries and no desktop notifications from a test run.
        .env("LISA_NIGHTLY_NOTIFY", "off")
        .output()
        .expect("run lisa nightly")
}

fn write_config(config: &Path, body: &str) {
    std::fs::create_dir_all(config).unwrap();
    std::fs::write(config.join("config.toml"), body).unwrap();
}

fn health(config: &Path) -> serde_json::Value {
    let path = config.join("nightly").join("health.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("no record at {}: {error}", path.display()));
    serde_json::from_str(&raw).expect("the record is JSON")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_machine_level_with_its_channel_records_that_and_stays_quiet() {
    let server = ReleaseServer::start(level_list());
    let config = TempDir::new().unwrap();
    write_config(config.path(), "channel = \"nightly\"\n");

    let output = nightly(config.path(), &FakeZellij::idle(), &server.url, &["run"]);
    assert!(
        output.status.success(),
        "{}{}",
        stdout_of(&output),
        stderr_of(&output)
    );

    let record = health(config.path());
    assert_eq!(record["outcome"], "level");
    assert_eq!(record["ok"], true);
    assert_eq!(record["channel"], "nightly");
    assert_eq!(record["installed_before"], INSTALLED);
    assert_eq!(record["tag"], newest_tag());
    assert!(record["at_utc"].as_str().unwrap().ends_with('Z'));
}

#[test]
fn a_tag_that_has_not_soaked_is_seen_and_not_taken() {
    let server = ReleaseServer::start(unsoaked_list());
    let config = TempDir::new().unwrap();
    write_config(config.path(), "channel = \"nightly\"\n");

    let output = nightly(config.path(), &FakeZellij::idle(), &server.url, &["run"]);
    assert!(output.status.success(), "{}", stderr_of(&output));

    let record = health(config.path());
    assert_eq!(record["outcome"], "waiting");
    assert_eq!(record["ok"], true);
    let detail = record["detail"].as_str().unwrap();
    assert!(
        detail.contains("v0.9.0-rc.1") && detail.contains("soak window left"),
        "the newest tag must be named, and named as not taken: {detail}"
    );
    assert!(
        record["installed_after"].is_null(),
        "nothing moved: {record}"
    );
}

#[test]
fn a_cycle_never_lands_under_a_live_run() {
    // The releases URL is unreachable on purpose: a busy machine must stop
    // before it asks the world anything at all.
    let config = TempDir::new().unwrap();
    write_config(config.path(), "channel = \"nightly\"\n");

    let output = nightly(
        config.path(),
        &FakeZellij::working(),
        "http://127.0.0.1:1/releases",
        &["run"],
    );
    assert!(output.status.success(), "{}", stderr_of(&output));

    let record = health(config.path());
    assert_eq!(record["outcome"], "skipped");
    assert_eq!(record["ok"], true);
    assert_eq!(record["consecutive_skips"], 1);
    assert!(
        record["detail"].as_str().unwrap().contains("board"),
        "the session holding the machine is named: {record}"
    );
}

#[test]
fn a_machine_that_is_always_working_stops_reading_as_healthy() {
    let config = TempDir::new().unwrap();
    write_config(config.path(), "channel = \"nightly\"\n");
    let zellij = FakeZellij::working();

    for expected in 1..=3 {
        let output = nightly(
            config.path(),
            &zellij,
            "http://127.0.0.1:1/releases",
            &["run"],
        );
        assert!(output.status.success(), "{}", stderr_of(&output));
        assert_eq!(health(config.path())["consecutive_skips"], expected);
    }

    let status = nightly(
        config.path(),
        &zellij,
        "http://127.0.0.1:1/releases",
        &["status"],
    );
    assert_eq!(
        status.status.code(),
        Some(1),
        "three skipped nights is a finding, not a quiet success"
    );
    assert!(
        stdout_of(&status).contains("not moving at all"),
        "{}",
        stdout_of(&status)
    );
}

#[test]
fn a_cycle_that_cannot_read_the_release_list_fails_loudly_and_moves_nothing() {
    let config = TempDir::new().unwrap();
    let alarm = TempDir::new().unwrap();
    let landed = alarm.path().join("alarm.json");
    write_config(
        config.path(),
        &format!(
            "channel = \"nightly\"\nalert_command = \"cat > {}\"\n",
            landed.display()
        ),
    );

    let output = nightly(
        config.path(),
        &FakeZellij::idle(),
        "http://127.0.0.1:1/releases",
        &["run"],
    );
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert!(
        stderr_of(&output).contains("cannot read the release list"),
        "{}",
        stderr_of(&output)
    );

    let record = health(config.path());
    assert_eq!(record["outcome"], "failed");
    assert_eq!(record["ok"], false);
    assert!(record["detail"]
        .as_str()
        .unwrap()
        .contains(&format!("lisa {INSTALLED} is unchanged")));

    // The alarm left the machine, carrying the whole record.
    let sent: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&landed).expect("the alarm ran")).unwrap();
    assert_eq!(sent["outcome"], "failed");
    assert_eq!(sent["installed_before"], INSTALLED);
    assert!(record["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line.as_str().unwrap() == "alert_command ran"));
}

#[test]
fn every_cycle_leaves_a_line_behind_so_the_arrangement_can_be_judged_later() {
    let server = ReleaseServer::start(level_list());
    let config = TempDir::new().unwrap();
    write_config(config.path(), "channel = \"nightly\"\n");
    let zellij = FakeZellij::idle();

    nightly(config.path(), &zellij, &server.url, &["run"]);
    nightly(config.path(), &zellij, &server.url, &["run"]);

    let history =
        std::fs::read_to_string(config.path().join("nightly").join("history.jsonl")).unwrap();
    assert_eq!(history.lines().count(), 2, "{history}");
    for line in history.lines() {
        let cycle: serde_json::Value = serde_json::from_str(line).expect("one JSON object a line");
        assert_eq!(cycle["outcome"], "level");
    }
}

#[test]
fn a_box_nobody_has_set_up_says_so_rather_than_reading_as_healthy() {
    let config = TempDir::new().unwrap();
    let status = nightly(
        config.path(),
        &FakeZellij::idle(),
        "http://127.0.0.1:1/releases",
        &["status"],
    );

    assert_eq!(status.status.code(), Some(1));
    let stdout = stdout_of(&status);
    assert!(stdout.contains("No nightly cycle has ever run"), "{stdout}");
    assert!(stdout.contains("lisa nightly install"), "{stdout}");
}

#[test]
fn the_json_a_fleet_reads_says_what_the_prose_says() {
    let server = ReleaseServer::start(level_list());
    let config = TempDir::new().unwrap();
    write_config(config.path(), "channel = \"nightly\"\n");
    let zellij = FakeZellij::idle();
    nightly(config.path(), &zellij, &server.url, &["run"]);

    let prose = nightly(config.path(), &zellij, &server.url, &["status"]);
    let json = nightly(config.path(), &zellij, &server.url, &["status", "--json"]);
    assert_eq!(prose.status.code(), json.status.code());
    assert_eq!(json.status.code(), Some(0));

    let document: serde_json::Value = serde_json::from_str(stdout_of(&json).trim()).unwrap();
    assert_eq!(document["command"], "nightly-status");
    assert_eq!(document["ok"], true);
    let data = &document["data"];
    assert_eq!(data["state"], "ok");
    assert_eq!(data["channel"], "nightly");
    assert_eq!(data["effective_channel"], "nightly");
    assert_eq!(data["last_cycle"]["outcome"], "level");
    assert!(stdout_of(&prose).contains(data["detail"].as_str().unwrap()));
}

#[test]
fn the_guide_names_the_fields_a_script_would_read() {
    let guide = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("json-guide")
        .output()
        .expect("run lisa json-guide");
    let guide = stdout_of(&guide);

    assert!(guide.contains("lisa nightly status --json"), "{guide}");
    assert!(guide.contains("nightly-status"), "{guide}");
    for field in ["last_cycle", "consecutive_skips", "installed_before"] {
        assert!(guide.contains(field), "the guide never names {field}");
    }
    for outcome in ["moved", "level", "waiting", "skipped", "failed"] {
        assert!(guide.contains(outcome), "the guide never names {outcome}");
    }
}

#[test]
fn install_dry_run_prints_the_job_and_touches_nothing() {
    let config = TempDir::new().unwrap();
    let agents = TempDir::new().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["nightly", "install", "--dry-run"])
        .env("HOME", config.path())
        .env("LISA_CONFIG_DIR", config.path())
        .env("LISA_LAUNCH_AGENTS_DIR", agents.path())
        .output()
        .expect("run lisa nightly install --dry-run");

    let stdout = stdout_of(&output);
    assert!(output.status.success(), "{stdout}{}", stderr_of(&output));
    assert!(stdout.contains("io.johnhkchen.lisa.nightly"), "{stdout}");
    assert!(stdout.contains("<string>nightly</string>"), "{stdout}");
    assert!(stdout.contains("<string>run</string>"), "{stdout}");

    assert!(
        std::fs::read_dir(agents.path()).unwrap().next().is_none(),
        "a dry run installs no job"
    );
    assert!(
        !config.path().join("config.toml").exists(),
        "a dry run records no channel"
    );
}

/// A board the nightly check could never ask about is refused at install time,
/// not at 04:30 every morning for the rest of the machine's life.
#[cfg(target_os = "macos")]
#[test]
fn a_project_that_is_not_a_board_is_refused_before_it_becomes_a_nightly_alarm() {
    let config = TempDir::new().unwrap();
    let agents = TempDir::new().unwrap();
    let not_a_board = TempDir::new().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["nightly", "install", "--project"])
        .arg(not_a_board.path())
        .env("HOME", config.path())
        .env("LISA_CONFIG_DIR", config.path())
        .env("LISA_LAUNCH_AGENTS_DIR", agents.path())
        .output()
        .expect("run lisa nightly install");

    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    assert!(
        stderr_of(&output).contains("is not a board Lisa knows"),
        "{}",
        stderr_of(&output)
    );
    assert!(
        std::fs::read_dir(agents.path()).unwrap().next().is_none(),
        "a refused install leaves no job behind"
    );
}

/// The schedule itself is launchd, so this is the machine it is for. The job is
/// written and read back; `LISA_LAUNCH_AGENTS_DIR` also keeps `launchctl` out of
/// it, so the suite never loads a job onto the machine running it.
#[cfg(target_os = "macos")]
#[test]
fn install_puts_the_machine_on_nightly_and_uninstall_leaves_what_it_knows() {
    let config = TempDir::new().unwrap();
    let agents = TempDir::new().unwrap();
    let board = TempDir::new().unwrap();
    std::fs::write(board.path().join(".lisa.toml"), "version = \"0.5.0\"\n").unwrap();

    let install = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["nightly", "install", "--project"])
        .arg(board.path())
        .args(["--alert", "cat > /dev/null"])
        .env("HOME", config.path())
        .env("LISA_CONFIG_DIR", config.path())
        .env("LISA_LAUNCH_AGENTS_DIR", agents.path())
        .output()
        .expect("run lisa nightly install");
    assert!(install.status.success(), "{}", stderr_of(&install));

    let job = std::fs::read_to_string(agents.path().join("io.johnhkchen.lisa.nightly.plist"))
        .expect("the job file");
    assert!(job.contains("<key>StartCalendarInterval</key>"), "{job}");
    assert!(job.contains("<string>nightly</string>"), "{job}");
    assert!(
        job.contains("/opt/homebrew/bin"),
        "launchd's PATH is not enough: {job}"
    );

    let recorded = std::fs::read_to_string(config.path().join("config.toml")).unwrap();
    assert!(recorded.contains("channel = \"nightly\""), "{recorded}");
    assert!(recorded.contains("nightly_project = "), "{recorded}");
    assert!(recorded.contains("alert_command = "), "{recorded}");

    let uninstall = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["nightly", "uninstall"])
        .env("HOME", config.path())
        .env("LISA_CONFIG_DIR", config.path())
        .env("LISA_LAUNCH_AGENTS_DIR", agents.path())
        .output()
        .expect("run lisa nightly uninstall");
    assert!(uninstall.status.success(), "{}", stderr_of(&uninstall));
    assert!(
        !agents
            .path()
            .join("io.johnhkchen.lisa.nightly.plist")
            .exists(),
        "the schedule is gone"
    );
    assert!(
        std::fs::read_to_string(config.path().join("config.toml"))
            .unwrap()
            .contains("channel = \"nightly\""),
        "the channel and the record survive the job being removed"
    );
}
