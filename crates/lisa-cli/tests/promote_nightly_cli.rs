//! `lisa promote-nightly` end to end (T-069-01-03).
//!
//! The decision rules are unit-tested in `src/promote.rs` against a release
//! list. This file pins the command a scheduled job actually runs: the JSON it
//! branches on, the pointer file it writes, and — the criterion that matters
//! most for a tap nobody wants to read during an incident — that a run with
//! nothing to do leaves the file exactly as it found it.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// A fixed instant, so the fixture below reads as ages rather than dates.
const NOW: i64 = 1_786_000_000;
const HOUR: i64 = 3600;
const DAY: i64 = 24 * HOUR;

/// The asset set a finished release carries: the four Debian packages the apt
/// suites pool and the formula the tap renders.
fn assets() -> String {
    [
        "lisa-amd64.deb",
        "lisa-arm64.deb",
        "lisa-runtime-zellij-amd64.deb",
        "lisa-runtime-zellij-arm64.deb",
        "lisa.rb",
    ]
    .map(|name| format!("{{\"name\": \"{name}\", \"state\": \"uploaded\"}}"))
    .join(", ")
}

/// One releases-API entry, `age` seconds before [`NOW`].
fn release(tag: &str, age: i64, assets: &str) -> String {
    format!(
        "{{\"tag_name\": \"{tag}\", \"draft\": false, \"published_at\": \"{}\", \
          \"assets\": [{assets}]}}",
        rfc3339(NOW - age)
    )
}

/// Epoch seconds as the UTC grammar the releases API publishes.
fn rfc3339(epoch: i64) -> String {
    // 2026-08-14T09:12:33Z-shaped, computed the plain way: the fixture only
    // needs instants inside one century.
    let days = epoch.div_euclid(86_400);
    let seconds = epoch.rem_euclid(86_400);
    let (mut year, mut day_of_year) = (1970, days);
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let length = if leap { 366 } else { 365 };
        if day_of_year < length {
            break;
        }
        day_of_year -= length;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for length in months {
        if day_of_year < length {
            break;
        }
        day_of_year -= length;
        month += 1;
    }
    format!(
        "{year:04}-{month:02}-{:02}T{:02}:{:02}:{:02}Z",
        day_of_year + 1,
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60,
    )
}

/// The live list, measured 2026-08-14: v0.4.4 is the newest stable, and
/// v0.5.0-rc.2 is the newest release of any kind and five days old, so it has
/// long since cleared the window.
fn measured_releases() -> String {
    let assets = assets();
    format!(
        "[{}, {}, {}]",
        release("v0.5.0-rc.2", 5 * DAY, &assets),
        release("v0.4.4", 26 * DAY, &assets),
        release("v0.4.3", 40 * DAY, &assets),
    )
}

/// A world where a hotfix landed twenty minutes after the release it replaces,
/// and neither has cleared the window yet.
fn hotfixed_releases() -> String {
    let assets = assets();
    format!(
        "[{}, {}, {}]",
        release("v0.5.0-rc.3", 20 * 60, &assets),
        release("v0.5.0-rc.2", 40 * 60, &assets),
        release("v0.4.4", 26 * DAY, &assets),
    )
}

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new(pointer: &str, releases: &str) -> Self {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("nightly-tag.txt"), format!("{pointer}\n")).unwrap();
        std::fs::write(dir.path().join("releases.json"), releases).unwrap();
        Self { dir }
    }

    fn pointer(&self) -> std::path::PathBuf {
        self.dir.path().join("nightly-tag.txt")
    }

    fn promote(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_lisa"))
            .arg("promote-nightly")
            .arg("--releases")
            .arg(self.dir.path().join("releases.json"))
            .arg("--pointer")
            .arg(self.pointer())
            .arg("--now")
            .arg(NOW.to_string())
            .args(args)
            .output()
            .expect("run lisa promote-nightly")
    }

    fn pointer_says(&self) -> String {
        std::fs::read_to_string(self.pointer()).unwrap()
    }
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn field(json: &str, key: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(json.trim()).expect(json);
    match &value[key] {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// When a file was last written, for asserting that it was not.
fn touched_at(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

#[test]
fn a_soaked_release_is_promoted_and_the_pointer_says_so() {
    let fixture = Fixture::new("stable", &measured_releases());

    let output = fixture.promote(&["--json", "--write"]);
    let json = stdout_of(&output);
    assert!(output.status.success(), "{json}");

    assert_eq!(field(&json, "action"), "promote");
    assert_eq!(field(&json, "target"), "v0.5.0-rc.2");
    assert_eq!(field(&json, "canary"), "v0.5.0-rc.2");
    assert_eq!(field(&json, "soak_hours"), "24");
    assert_eq!(field(&json, "wrote"), "true");
    assert_eq!(fixture.pointer_says(), "v0.5.0-rc.2\n");
}

#[test]
fn a_promotion_with_nothing_to_do_does_not_touch_the_pointer() {
    let fixture = Fixture::new("v0.5.0-rc.2", &measured_releases());
    let before = touched_at(&fixture.pointer());

    let json = stdout_of(&fixture.promote(&["--json", "--write"]));

    assert_eq!(field(&json, "action"), "unchanged");
    assert_eq!(field(&json, "wrote"), "false");
    assert_eq!(fixture.pointer_says(), "v0.5.0-rc.2\n");
    assert_eq!(
        touched_at(&fixture.pointer()),
        before,
        "a run with nothing to do must leave no commit behind it"
    );
}

#[test]
fn two_releases_inside_one_window_promote_neither() {
    let fixture = Fixture::new("stable", &hotfixed_releases());

    let json = stdout_of(&fixture.promote(&["--json", "--write"]));

    assert_eq!(field(&json, "action"), "unchanged");
    assert_eq!(field(&json, "target"), "stable");
    assert!(
        field(&json, "reason").contains("superseded"),
        "the older release is superseded, not merely un-soaked: {json}"
    );
    assert_eq!(fixture.pointer_says(), "stable\n");
}

#[test]
fn a_pointer_whose_release_was_pulled_is_retired() {
    // The release list no longer carries rc.1 at all, and nothing newer has
    // soaked: nightly goes back to stable rather than naming a release the
    // publish would fail closed on.
    let assets = assets();
    let releases = format!(
        "[{}, {}]",
        release("v0.5.0-rc.3", 2 * HOUR, &assets),
        release("v0.4.4", 26 * DAY, &assets),
    );
    let fixture = Fixture::new("v0.5.0-rc.1", &releases);

    let json = stdout_of(&fixture.promote(&["--json", "--write"]));

    assert_eq!(field(&json, "action"), "retire");
    assert_eq!(fixture.pointer_says(), "stable\n");
}

#[test]
fn a_release_still_uploading_is_not_promoted() {
    let releases = format!(
        "[{}, {}]",
        release(
            "v0.5.0-rc.3",
            2 * DAY,
            "{\"name\": \"lisa-amd64.deb\", \"state\": \"uploaded\"}"
        ),
        release("v0.4.4", 26 * DAY, &assets()),
    );
    let fixture = Fixture::new("stable", &releases);

    let json = stdout_of(&fixture.promote(&["--json", "--write"]));

    assert_eq!(field(&json, "action"), "unchanged");
    assert!(field(&json, "reason").contains("lisa.rb"), "{json}");
    assert_eq!(fixture.pointer_says(), "stable\n");
}

#[test]
fn a_decision_without_write_changes_nothing() {
    let fixture = Fixture::new("stable", &measured_releases());
    let before = touched_at(&fixture.pointer());

    let json = stdout_of(&fixture.promote(&["--json"]));

    assert_eq!(field(&json, "action"), "promote");
    assert_eq!(field(&json, "wrote"), "false");
    assert_eq!(fixture.pointer_says(), "stable\n");
    assert_eq!(touched_at(&fixture.pointer()), before);
}

#[test]
fn the_release_list_can_arrive_on_standard_input() {
    let fixture = Fixture::new("stable", &measured_releases());

    let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .arg("promote-nightly")
        .arg("--pointer")
        .arg(fixture.pointer())
        .arg("--now")
        .arg(NOW.to_string())
        .arg("--json")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(measured_releases().as_bytes())?;
            child.wait_with_output()
        })
        .expect("run lisa promote-nightly");

    let json = stdout_of(&output);
    assert!(output.status.success(), "{json}");
    assert_eq!(field(&json, "action"), "promote");
}

#[test]
fn a_pointer_nobody_can_parse_is_refused_rather_than_guessed_at() {
    let fixture = Fixture::new("latest", &measured_releases());

    let output = fixture.promote(&["--json", "--write"]);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("neither"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fixture.pointer_says(), "latest\n");
}

#[test]
fn the_human_report_names_the_release_and_the_reason() {
    let fixture = Fixture::new("stable", &measured_releases());

    let report = stdout_of(&fixture.promote(&["--write"]));

    assert!(report.contains("action:  promote"), "{report}");
    assert!(report.contains("nightly: v0.5.0-rc.2"), "{report}");
    assert!(report.contains("soak window"), "{report}");
    assert!(report.contains("late:"), "{report}");
}
