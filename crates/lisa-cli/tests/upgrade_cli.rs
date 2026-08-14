//! `lisa upgrade` end to end, against a fixed release list (T-068-01-01).
//!
//! The list served here is the one that started the story, measured 2026-08-14:
//! v0.4.4 is the newest release that is not a prerelease and the version every
//! curl-installed machine was frozen on, v0.5.0-rc.2 is the newest tag and what
//! Homebrew carried, and a third tag exists that no channel but `canary` may
//! take because it has not soaked.
//!
//! Nothing here installs anything: every case either stops at `--dry-run`, or
//! stops earlier because the channel is not moving, or fails before it can
//! reach an artifact. The pure resolution rules are unit-tested in
//! `src/channel.rs`; this file pins the command's own behaviour — what it says
//! before it moves, what it records, and what it does with no network.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// The measured release list. Publish dates are the real ones; the third tag is
/// dated far enough ahead that it can never have soaked, whatever day the suite
/// runs on.
const RELEASE_LIST: &str = r#"[
  {"tag_name": "v0.9.0-rc.1", "published_at": "2099-01-01T00:00:00Z", "draft": false},
  {"tag_name": "v0.5.0-rc.2", "published_at": "2026-08-09T00:00:00Z", "draft": false},
  {"tag_name": "v0.4.4", "published_at": "2026-07-19T00:00:00Z", "draft": false},
  {"tag_name": "v0.4.3", "published_at": "2026-07-05T00:00:00Z", "draft": false}
]"#;

/// A local stand-in for the GitHub releases API, serving one fixed body.
struct ReleaseServer {
    url: String,
}

impl ReleaseServer {
    fn start(body: &'static str) -> Self {
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
    // Read past the request head; the body is fixed, so the request is ignored.
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

/// Run `lisa upgrade` with a throwaway machine config and a given release list.
fn upgrade(config_dir: &Path, releases_url: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("upgrade")
        .args(args)
        .env("LISA_CONFIG_DIR", config_dir)
        .env("LISA_RELEASES_URL", releases_url)
        .output()
        .expect("run lisa upgrade")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The version this build is, which is the version every message moves *from*.
const INSTALLED: &str = env!("CARGO_PKG_VERSION");

#[test]
fn stable_resolves_to_the_newest_release_that_is_not_a_prerelease() {
    let server = ReleaseServer::start(RELEASE_LIST);
    let config = TempDir::new().unwrap();

    let output = upgrade(
        config.path(),
        &server.url,
        &["--channel", "stable", "--dry-run"],
    );

    let stdout = stdout_of(&output);
    assert!(output.status.success(), "{stdout}{}", stderr_of(&output));
    assert!(
        stdout.contains(&format!("Moving lisa {INSTALLED} → 0.4.4 [v0.4.4]")),
        "an upgrade must name both versions before it moves:\n{stdout}"
    );
    assert!(
        stdout.contains("--dry-run: nothing was downloaded"),
        "{stdout}"
    );
}

#[test]
fn canary_resolves_to_the_newest_tag_prerelease_or_not() {
    let server = ReleaseServer::start(RELEASE_LIST);
    let config = TempDir::new().unwrap();

    let stdout = stdout_of(&upgrade(
        config.path(),
        &server.url,
        &["--channel", "canary", "--dry-run"],
    ));

    assert!(
        stdout.contains(&format!(
            "Moving lisa {INSTALLED} → 0.9.0-rc.1 [v0.9.0-rc.1]"
        )),
        "canary takes the newest tag even though it has not soaked:\n{stdout}"
    );
}

#[test]
fn nightly_holds_where_it_is_until_the_newest_tag_has_soaked() {
    let server = ReleaseServer::start(RELEASE_LIST);
    let config = TempDir::new().unwrap();

    let output = upgrade(
        config.path(),
        &server.url,
        &["--channel", "nightly", "--dry-run"],
    );
    let stdout = stdout_of(&output);

    assert!(output.status.success(), "{stdout}{}", stderr_of(&output));
    assert!(
        stdout.contains("not moving this cycle"),
        "nightly must not take an unsoaked tag:\n{stdout}"
    );
    assert!(stdout.contains("v0.9.0-rc.1"), "{stdout}");
    assert!(
        stdout.contains("superseded"),
        "the hold has to say why the older, soaked tags are not the answer either:\n{stdout}"
    );
    assert!(
        stdout.contains(&format!("lisa {INSTALLED} stays in place")),
        "{stdout}"
    );
}

#[test]
fn setting_a_channel_and_upgrading_is_one_command() {
    let server = ReleaseServer::start(RELEASE_LIST);
    let config = TempDir::new().unwrap();
    let config_file = config.path().join("config.toml");

    // A dry run says what it would record and records nothing.
    let stdout = stdout_of(&upgrade(
        config.path(),
        &server.url,
        &["--channel", "nightly", "--dry-run"],
    ));
    assert!(
        stdout.contains("Would put this machine on channel nightly"),
        "{stdout}"
    );
    assert!(
        !config_file.exists(),
        "a dry run wrote {}",
        config_file.display()
    );

    // The real run records the channel and then acts on it, in one command.
    // Nightly is holding here, so this stops before any artifact is fetched.
    let output = upgrade(config.path(), &server.url, &["--channel", "nightly"]);
    let stdout = stdout_of(&output);
    assert!(output.status.success(), "{stdout}{}", stderr_of(&output));
    assert!(
        stdout.contains("This machine is on channel nightly"),
        "{stdout}"
    );

    let recorded = std::fs::read_to_string(&config_file).expect("the channel was recorded");
    assert!(recorded.contains("channel = \"nightly\""), "{recorded}");
    assert!(recorded.contains("soak_hours = 24"), "{recorded}");

    // And the recorded channel is what a bare `lisa upgrade` then acts on.
    let stdout = stdout_of(&upgrade(config.path(), &server.url, &["--dry-run"]));
    assert!(stdout.contains("Channel nightly"), "{stdout}");
}

#[test]
fn a_machine_that_has_never_chosen_is_treated_as_stable_and_says_so() {
    let server = ReleaseServer::start(RELEASE_LIST);
    let config = TempDir::new().unwrap();

    let stdout = stdout_of(&upgrade(config.path(), &server.url, &["--dry-run"]));

    assert!(
        stdout.contains("No channel is set"),
        "an unset channel must read as unset, not as a silent stable:\n{stdout}"
    );
    assert!(stdout.contains("treated as stable"), "{stdout}");
    assert!(stdout.contains("→ 0.4.4 [v0.4.4]"), "{stdout}");
}

#[test]
fn a_tag_pins_to_an_exact_release_and_an_unknown_one_is_refused() {
    let server = ReleaseServer::start(RELEASE_LIST);
    let config = TempDir::new().unwrap();

    let stdout = stdout_of(&upgrade(
        config.path(),
        &server.url,
        &["--tag", "v0.4.3", "--dry-run"],
    ));
    assert!(stdout.contains("Pinning to v0.4.3"), "{stdout}");
    assert!(
        stdout.contains(&format!("Moving lisa {INSTALLED} → 0.4.3 [v0.4.3]")),
        "{stdout}"
    );
    assert!(
        stdout.contains("a move back to an older release"),
        "a rollback should say that it is one:\n{stdout}"
    );

    let output = upgrade(
        config.path(),
        &server.url,
        &["--tag", "v9.9.9", "--dry-run"],
    );
    assert!(!output.status.success(), "{}", stdout_of(&output));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("no release is tagged v9.9.9"), "{stderr}");
    assert!(stderr.contains("v0.9.0-rc.1"), "{stderr}");
}

#[test]
fn with_no_network_it_fails_loudly_and_changes_nothing() {
    let config = TempDir::new().unwrap();
    // Port 1 is reserved and never listening, so this is a real failed reach
    // rather than a mocked one.
    let output = upgrade(config.path(), "http://127.0.0.1:1/releases", &["--dry-run"]);

    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("cannot read the release list"), "{stderr}");
    assert!(
        stderr.contains(&format!("lisa {INSTALLED}")) && stderr.contains("is unchanged"),
        "a failed reach must say the installed Lisa is untouched:\n{stderr}"
    );
}

/// The guard T-068-01-03 added: a machine with a run on it keeps the binary
/// that run is calling. This is the one case that gets as far as being ready to
/// download something, so it is run without `--dry-run` — and it still fetches
/// no artifact, because the refusal comes first.
#[test]
fn an_upgrade_does_not_land_under_a_live_run() {
    let server = ReleaseServer::start(RELEASE_LIST);
    let config = TempDir::new().unwrap();

    let zellij = TempDir::new().unwrap();
    let script = zellij.path().join("zellij");
    std::fs::write(
        &script,
        "#!/bin/sh\necho 'board [Created 34m ago] (current)'\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["upgrade", "--channel", "stable"])
        .env("LISA_CONFIG_DIR", config.path())
        .env("LISA_RELEASES_URL", &server.url)
        .env("PATH", format!("{}:/usr/bin:/bin", zellij.path().display()))
        .output()
        .expect("run lisa upgrade");

    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("not moving lisa while this machine is working"),
        "{stderr}"
    );
    assert!(stderr.contains("board"), "the run is named: {stderr}");
    assert!(
        stderr.contains("lisa upgrade --anyway"),
        "the way past it is named too: {stderr}"
    );
    assert!(
        stderr.contains(&format!("lisa {INSTALLED}")) && stderr.contains("is unchanged"),
        "{stderr}"
    );
}

#[test]
fn an_unknown_channel_names_the_three_that_exist() {
    let config = TempDir::new().unwrap();
    let output = upgrade(
        config.path(),
        "http://127.0.0.1:1/releases",
        &["--channel", "beta"],
    );

    assert!(!output.status.success());
    let stderr = stderr_of(&output);
    assert!(stderr.contains("canary, nightly and stable"), "{stderr}");
}
