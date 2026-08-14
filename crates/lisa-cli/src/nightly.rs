//! The unattended arrangement: a machine that moves on its own, says what it
//! did, and shouts when the move went wrong.
//!
//! [`crate::channel`] holds the rules, [`crate::upgrade`] is the command a
//! person runs, and [`crate::freshness`] is `doctor`'s report on the gap. This
//! module is the fourth part and the one that makes the other three worth
//! having on a box nobody is looking at: **a machine that waits to be upgraded
//! by hand drifts**, exactly the way the curl-installed machines drifted four
//! weeks behind without anyone deciding they should. Insurance you have to
//! remember to renew is not insurance.
//!
//! ## What one cycle does, and what it refuses to do
//!
//! 1. **Refuses to land mid-run.** [`crate::busy`] asks whether any Zellij
//!    session is still up on this machine. If one is, the cycle records
//!    `skipped` and ends; the schedule tries again within the hour. Being one
//!    release behind is cheaper than a run whose binary changed underneath it.
//! 2. **Moves, if the channel says to.** The nightly channel resolves to the
//!    newest tag once it has soaked; mid-soak the cycle records `waiting` and
//!    the machine stays exactly where it is.
//! 3. **Leaves the working version in place when anything fails.** The move is
//!    the release's own shell installer writing a new file into
//!    `~/.local/bin`. Nothing here deletes or truncates the running binary, so
//!    a failed download, a failed checksum or a failed installer ends with the
//!    Lisa that was working still being the Lisa on the box.
//! 4. **Checks the release against this machine's own work.** After a move it
//!    runs the newly installed binary's `doctor --json` against the project
//!    this box actually works (`nightly_project`). That is what catches a
//!    Homebrew Zellij that has drifted out of `SUPPORTED_ZELLIJ_RANGE` — the
//!    failure a macOS box can have and a Debian box, with its pinned Zellij,
//!    cannot.
//! 5. **Shouts when it fails.** stderr (which launchd files away), the system
//!    log, a desktop notification, and `alert_command` — the one that leaves
//!    the box. Every alarm carries the rollback: `lisa upgrade --tag <the tag
//!    that was working>`.
//!
//! ## Where the record lives
//!
//! Next to the channel, under the machine's own Lisa directory:
//!
//! ```text
//! <config dir>/nightly/health.json     the last cycle, whole
//! <config dir>/nightly/history.jsonl   one line per cycle, appended
//! <config dir>/nightly/launchd.out     what the job printed
//! <config dir>/nightly/launchd.err     what it printed when it went wrong
//! ```
//!
//! `lisa nightly status` reads the first of those and is the question to ask a
//! box: it fails when the last cycle failed, when the record has gone stale
//! (nothing ran, so the schedule itself is broken), and when the box has been
//! too busy to move for several nights running. A silence is a finding here,
//! not an absence of one.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::busy;
use crate::channel::{self, Channel, MachineConfig};
use crate::upgrade::{self, InstallMethod};

/// The launchd job's name. Reverse-DNS, the same identity
/// `directories::ProjectDirs` gives the config directory.
const LAUNCHD_LABEL: &str = "io.johnhkchen.lisa.nightly";

/// Environment override for `~/Library/LaunchAgents`, so a test can write a
/// throwaway one. Set, it also means `launchctl` is not called: a test asserts
/// the file, it does not load a job onto the machine running it.
const LAUNCH_AGENTS_DIR_ENV: &str = "LISA_LAUNCH_AGENTS_DIR";

/// Set to `off` to keep a cycle's local alarms — the system log and the desktop
/// notification — quiet. For a test, and for a box where the desktop alarm is
/// noise rather than signal.
const NOTIFY_ENV: &str = "LISA_NIGHTLY_NOTIFY";

/// When the job runs, and the two second chances behind it.
///
/// One time a night would be enough if the box were always idle at that time,
/// and it is not: this machine's job is running work. Three tries an hour apart
/// before breakfast means a box that was working at 04:30 still gets its move
/// before anyone looks at it, and a box that is working at all three keeps its
/// running loop, which is the trade this arrangement is built on.
const RUN_TIMES: [(u32, u32); 3] = [(4, 30), (5, 30), (6, 30)];

/// How long a record can go without a new cycle before the silence is the
/// finding. Longer than a day so a single missed night is not an alarm, short
/// enough that two are.
const STALE_AFTER_HOURS: i64 = 36;

/// How many cycles in a row may be skipped for a live run before `status` says
/// this machine never gets a chance to move.
const SKIPS_BEFORE_SAYING_SO: u64 = 3;

/// What a cycle did. One word, so a script can branch on it and a person can
/// read it.
mod outcome {
    /// The machine moved to a new release and it checked out.
    pub(super) const MOVED: &str = "moved";
    /// The machine is already on what its channel resolves to.
    pub(super) const LEVEL: &str = "level";
    /// The channel names no release this cycle — nightly, mid-soak.
    pub(super) const WAITING: &str = "waiting";
    /// A run was live, so nothing was touched.
    pub(super) const SKIPPED: &str = "skipped";
    /// Something went wrong, and this is the outcome that shouts.
    pub(super) const FAILED: &str = "failed";
}

/// One cycle, as it is written down.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Cycle {
    /// When the cycle finished, in seconds since the Unix epoch.
    pub(crate) at: i64,
    /// The same instant in UTC, because a person opens this file too.
    pub(crate) at_utc: String,
    /// One of [`outcome`].
    pub(crate) outcome: String,
    /// Whether this cycle is a finding. `false` is the thing to alert on.
    pub(crate) ok: bool,
    /// The sentence that says what happened.
    pub(crate) detail: String,
    /// The channel this machine recorded, or `null` when it has chosen none.
    pub(crate) channel: Option<String>,
    /// The channel actually acted on — the chosen one, or `stable`.
    pub(crate) effective_channel: String,
    /// The version that was installed when the cycle started.
    pub(crate) installed_before: String,
    /// The version installed when it ended, when the cycle moved.
    pub(crate) installed_after: Option<String>,
    /// The tag the channel resolved to, when it resolved to one.
    pub(crate) tag: Option<String>,
    /// The command that settles this, when there is one. On a failure that is
    /// the rollback.
    pub(crate) remedy: Option<String>,
    /// How many cycles in a row have now been skipped for a live run.
    pub(crate) consecutive_skips: u64,
    /// What was told, and how it went.
    #[serde(default)]
    pub(crate) alerts: Vec<String>,
}

/// Where the record lives: `<machine config dir>/nightly`.
pub(crate) fn state_dir() -> Result<PathBuf, String> {
    Ok(channel::config_dir()?.join("nightly"))
}

fn health_path(state: &Path) -> PathBuf {
    state.join("health.json")
}

fn history_path(state: &Path) -> PathBuf {
    state.join("history.jsonl")
}

/// Read the last cycle. A missing file is "nothing has run here", which is a
/// state with its own meaning and not an error.
pub(crate) fn last_cycle(state: &Path) -> Result<Option<Cycle>, String> {
    let path = health_path(state);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
}

/// Write the cycle down: the whole of the last one, and one appended line of
/// history so "how often does nightly actually catch something" is answerable
/// later without having watched.
fn record(state: &Path, cycle: &Cycle) -> Result<(), String> {
    std::fs::create_dir_all(state)
        .map_err(|error| format!("cannot create {}: {error}", state.display()))?;

    let body = serde_json::to_string_pretty(cycle)
        .map_err(|error| format!("cannot write the health record: {error}"))?;
    std::fs::write(health_path(state), format!("{body}\n"))
        .map_err(|error| format!("cannot write {}: {error}", health_path(state).display()))?;

    let line = serde_json::to_string(cycle)
        .map_err(|error| format!("cannot write the history line: {error}"))?;
    let mut history = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path(state))
        .map_err(|error| format!("cannot open {}: {error}", history_path(state).display()))?;
    writeln!(history, "{line}")
        .map_err(|error| format!("cannot write {}: {error}", history_path(state).display()))
}

/// The binary a cycle checks after it moves: the installer's, when it is there,
/// and otherwise whatever is running now.
fn lisa_to_check(home: Option<&Path>, running: &Path) -> PathBuf {
    match home {
        Some(home) if upgrade::installer_owned_path(home).exists() => {
            upgrade::installer_owned_path(home)
        }
        _ => running.to_path_buf(),
    }
}

/// Render a span of seconds as an operator reads an age.
fn ago(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds >= 172_800 {
        format!("{} days ago", seconds / 86_400)
    } else if seconds >= 86_400 {
        "a day ago".to_string()
    } else if seconds >= 3600 {
        format!("{}h ago", seconds / 3600)
    } else {
        format!("{}m ago", seconds / 60)
    }
}

/// Build a cycle record around what happened.
#[allow(clippy::too_many_arguments)]
fn cycle(
    at: i64,
    outcome: &str,
    ok: bool,
    detail: String,
    config: &MachineConfig,
    installed_before: &Version,
    installed_after: Option<String>,
    tag: Option<String>,
    remedy: Option<String>,
    consecutive_skips: u64,
) -> Cycle {
    Cycle {
        at,
        at_utc: channel::format_rfc3339_utc(at),
        outcome: outcome.to_string(),
        ok,
        detail,
        channel: config.channel.map(|channel| channel.as_str().to_string()),
        effective_channel: config.effective_channel().as_str().to_string(),
        installed_before: installed_before.to_string(),
        installed_after,
        tag,
        remedy,
        consecutive_skips,
        alerts: Vec::new(),
    }
}

/// Run one unattended cycle. The exit status is the verdict: `0` for a cycle
/// that did the right thing, including doing nothing, and `1` for one that
/// failed.
pub(crate) fn run_cycle() -> Result<i32, String> {
    let started = channel::now_unix();
    let state = state_dir()?;
    let config_path = channel::config_path()?;
    let config = channel::load_from(&config_path)?;
    let previous = last_cycle(&state)?;
    let skips_before = previous
        .as_ref()
        .filter(|cycle| cycle.outcome == outcome::SKIPPED)
        .map(|cycle| cycle.consecutive_skips)
        .unwrap_or(0);

    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot find the running lisa: {error}"))?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let method = upgrade::classify_install(&exe, home.as_deref());
    // The version this machine *has*, which on a box where the running lisa
    // came from somewhere else is not the version of the process asking.
    let installed = upgrade::installed_lisa(&exe, home.as_deref())?.version;

    println!(
        "lisa nightly, {} — channel {}, installed {installed}.",
        channel::format_rfc3339_utc(started),
        config.effective_channel(),
    );

    // A package-managed box cannot honour a channel at all, and a schedule
    // pointed at one will fail every night until someone says why.
    if matches!(method, InstallMethod::Homebrew | InstallMethod::Apt) {
        let detail = format!(
            "this machine's lisa is managed by a package manager ({}), which carries one \
             version and cannot follow a channel",
            exe.display()
        );
        return finish(
            &state,
            &config,
            cycle(
                channel::now_unix(),
                outcome::FAILED,
                false,
                detail,
                &config,
                &installed,
                None,
                None,
                Some("Move this machine onto the channel-aware install: lisa upgrade".to_string()),
                0,
            ),
        );
    }

    // Nothing lands under a live run.
    let busy = busy::look();
    if busy.is_busy() {
        let skips = skips_before + 1;
        let detail = format!("nothing was touched: {}", busy.describe());
        return finish(
            &state,
            &config,
            cycle(
                channel::now_unix(),
                outcome::SKIPPED,
                true,
                detail,
                &config,
                &installed,
                None,
                None,
                None,
                skips,
            ),
        );
    }

    let releases = match upgrade::fetch_releases() {
        Ok(releases) => releases,
        Err(error) => {
            return finish(
                &state,
                &config,
                cycle(
                    channel::now_unix(),
                    outcome::FAILED,
                    false,
                    format!("{error}. lisa {installed} is unchanged"),
                    &config,
                    &installed,
                    None,
                    None,
                    None,
                    0,
                ),
            )
        }
    };

    let selected = config.effective_channel();
    let resolution = channel::resolve(selected, &releases, channel::now_unix(), config.soak());
    let Some(target) = resolution.release().cloned() else {
        let reason = resolution
            .waiting_reason()
            .unwrap_or("the channel resolves to no release")
            .to_string();
        return finish(
            &state,
            &config,
            cycle(
                channel::now_unix(),
                outcome::WAITING,
                true,
                format!("channel {selected} is not moving this cycle: {reason}"),
                &config,
                &installed,
                None,
                None,
                None,
                0,
            ),
        );
    };

    if target.version == installed {
        return finish(
            &state,
            &config,
            cycle(
                channel::now_unix(),
                outcome::LEVEL,
                true,
                format!(
                    "lisa {installed} is already {}, so nothing moved",
                    target.tag
                ),
                &config,
                &installed,
                None,
                Some(target.tag.clone()),
                None,
                0,
            ),
        );
    }

    // The way back, worked out before the move so the alarm can carry it.
    let rollback = format!("lisa upgrade --tag v{installed}");
    println!(
        "Moving lisa {installed} → {} [{}].",
        target.version, target.tag
    );

    if let Err(error) = upgrade::install(&target) {
        return finish(
            &state,
            &config,
            cycle(
                channel::now_unix(),
                outcome::FAILED,
                false,
                format!("{error}. lisa {installed} is still in place and still works"),
                &config,
                &installed,
                None,
                Some(target.tag.clone()),
                Some(
                    "Nothing was replaced, so there is nothing to undo. The next cycle \
                     tries again; to look now: lisa upgrade --dry-run"
                        .to_string(),
                ),
                0,
            ),
        );
    }

    let lisa = lisa_to_check(home.as_deref(), &exe);
    match verify(&lisa, &target.version, config.nightly_project.as_deref()) {
        Ok(detail) => finish(
            &state,
            &config,
            cycle(
                channel::now_unix(),
                outcome::MOVED,
                true,
                format!(
                    "moved {installed} → {} [{}]. {detail}",
                    target.version, target.tag
                ),
                &config,
                &installed,
                Some(target.version.to_string()),
                Some(target.tag.clone()),
                None,
                0,
            ),
        ),
        Err(error) => finish(
            &state,
            &config,
            cycle(
                channel::now_unix(),
                outcome::FAILED,
                false,
                format!(
                    "lisa {} [{}] installed, and this machine is not working with it: {error}",
                    target.version, target.tag
                ),
                &config,
                &installed,
                Some(target.version.to_string()),
                Some(target.tag.clone()),
                Some(format!(
                    "Put this machine back on the release that worked:\n    {rollback}"
                )),
                0,
            ),
        ),
    }
}

/// Write the cycle down, say it, and raise the alarm when it is a finding.
fn finish(state: &Path, config: &MachineConfig, mut cycle: Cycle) -> Result<i32, String> {
    if !cycle.ok {
        cycle.alerts = raise_alarm(config, &cycle);
    }
    record(state, &cycle)?;

    let line = format!("{}: {}", cycle.outcome, cycle.detail);
    if cycle.ok {
        println!("{line}");
    } else {
        eprintln!("{line}");
    }
    if let Some(remedy) = &cycle.remedy {
        if cycle.ok {
            println!("{remedy}");
        } else {
            eprintln!("{remedy}");
        }
    }
    println!("Recorded in {}.", health_path(state).display());

    Ok(if cycle.ok { 0 } else { 1 })
}

/// Prove the release that just landed is one this machine can work with.
///
/// Two questions, in order. Did the swap take — is the binary on `PATH` the
/// version we asked for? And does this box's own board still come up clean
/// under it, which is where a Homebrew Zellij that drifted out of the supported
/// range shows up. Without a project to check against, only the first question
/// can be asked, and the answer says so rather than implying more.
fn verify(lisa: &Path, expected: &Version, project: Option<&Path>) -> Result<String, String> {
    let reported = Command::new(lisa)
        .arg("--version")
        .output()
        .map_err(|error| format!("cannot run {}: {error}", lisa.display()))?;
    let reported = String::from_utf8_lossy(&reported.stdout);
    let reported = reported
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("{} --version said {reported:?}", lisa.display()))?;
    if reported != expected.to_string() {
        return Err(format!(
            "{} reports {reported} after installing {expected}",
            lisa.display()
        ));
    }

    let Some(project) = project else {
        return Ok(format!(
            "{} reports {expected}. No project is set for the check, so nothing deeper was \
             asked — set one with: lisa nightly install --project <path>",
            lisa.display()
        ));
    };

    let doctor = Command::new(lisa)
        .arg("doctor")
        .arg("--json")
        .arg("--path")
        .arg(project)
        .output()
        .map_err(|error| format!("cannot run doctor at {}: {error}", project.display()))?;
    let body = String::from_utf8_lossy(&doctor.stdout);
    let document: serde_json::Value = serde_json::from_str(body.trim()).map_err(|error| {
        format!(
            "doctor at {} did not answer in JSON ({error}): {}",
            project.display(),
            String::from_utf8_lossy(&doctor.stderr).trim()
        )
    })?;

    if document["ok"] != serde_json::Value::Bool(true) {
        return Err(format!(
            "doctor at {} could not run: {}",
            project.display(),
            document["error"]["message"]
                .as_str()
                .unwrap_or("no reason given")
        ));
    }

    if document["data"]["verdict"] == serde_json::Value::String("passed".to_string()) {
        return Ok(format!(
            "doctor passes at {} under {expected}",
            project.display()
        ));
    }

    let failing: Vec<String> = document["data"]["checks"]
        .as_array()
        .map(|checks| {
            checks
                .iter()
                .filter(|check| {
                    matches!(
                        check["status"].as_str(),
                        Some("missing") | Some("unsupported")
                    )
                })
                .map(|check| {
                    format!(
                        "{} {}",
                        check["name"].as_str().unwrap_or("a check"),
                        check["detail"]
                            .as_str()
                            .or_else(|| check["remedy"].as_str())
                            .unwrap_or("is not usable")
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    Err(format!(
        "doctor fails at {} under {expected}: {}",
        project.display(),
        if failing.is_empty() {
            "see lisa doctor".to_string()
        } else {
            failing.join("; ")
        }
    ))
}

/// Tell someone. Four ways out, each one weaker than the last is loud:
/// stderr for the log launchd keeps, the system log for a box nobody is at, a
/// desktop notification for one somebody is, and the operator's own command for
/// the alarm that leaves the machine.
fn raise_alarm(config: &MachineConfig, cycle: &Cycle) -> Vec<String> {
    let mut told = Vec::new();
    let headline = format!("Lisa nightly {}: {}", cycle.outcome, cycle.detail);

    if std::env::var(NOTIFY_ENV).as_deref() != Ok("off") {
        if Command::new("logger")
            .args(["-t", "lisa-nightly", "-p", "user.err"])
            .arg(&headline)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
        {
            told.push("the system log".to_string());
        }

        if cfg!(target_os = "macos") {
            let script = format!(
                "display notification {} with title \"Lisa nightly\"",
                applescript_string(&cycle.detail)
            );
            if Command::new("osascript")
                .arg("-e")
                .arg(script)
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
            {
                told.push("a desktop notification".to_string());
            }
        }
    }

    match &config.alert_command {
        None => told.push(
            "nothing left this machine: no alert_command is set in the machine config".to_string(),
        ),
        Some(command) => told.push(run_alert_command(command, cycle)),
    }

    told
}

/// Quote a string for AppleScript, where the escapes are the same two that
/// matter in TOML and nothing else is safe to leave alone.
fn applescript_string(raw: &str) -> String {
    let mut quoted = String::with_capacity(raw.len() + 2);
    quoted.push('"');
    for ch in raw.chars() {
        match ch {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' | '\r' | '\t' => quoted.push(' '),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

/// Run the operator's own alarm, handing it the whole record on stdin.
fn run_alert_command(command: &str, cycle: &Cycle) -> String {
    let body = serde_json::to_string(cycle).unwrap_or_else(|_| "{}".to_string());

    let child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .env("LISA_NIGHTLY_OUTCOME", &cycle.outcome)
        .env("LISA_NIGHTLY_DETAIL", &cycle.detail)
        .stdin(Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(child) => child,
        Err(error) => return format!("alert_command could not be started: {error}"),
    };

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(body.as_bytes());
    }
    drop(child.stdin.take());

    match child.wait() {
        Ok(status) if status.success() => "alert_command ran".to_string(),
        Ok(status) => format!("alert_command exited {status}"),
        Err(error) => format!("alert_command could not be waited on: {error}"),
    }
}

/// What `lisa nightly status` found.
struct Standing {
    /// The line that says where this machine stands.
    headline: String,
    /// What to do about it, when there is something.
    remedy: Option<String>,
    /// Whether this is a finding.
    ok: bool,
    /// The last cycle, when there has been one.
    last: Option<Cycle>,
}

/// Read the record and say where this machine stands.
///
/// Three ways to fail, and only one of them is a failed upgrade: a schedule
/// that stopped running leaves a record that ages, and a box that is always
/// working when the job fires never moves at all. Both of those are silences,
/// and a silence that reads as health is how a fleet goes stale.
fn standing(last: Option<Cycle>, now: i64) -> Standing {
    let Some(cycle) = last else {
        return Standing {
            headline: "No nightly cycle has ever run on this machine.".to_string(),
            remedy: Some("Set the arrangement up: lisa nightly install".to_string()),
            ok: false,
            last: None,
        };
    };

    let age = now - cycle.at;
    if age > STALE_AFTER_HOURS * 3600 {
        return Standing {
            headline: format!(
                "The last nightly cycle was {} ({}), which is longer ago than a night. \
                 The schedule is not running.",
                ago(age),
                cycle.at_utc
            ),
            remedy: Some(format!(
                "Check the job and put it back: launchctl list | grep {LAUNCHD_LABEL}\n    \
                 lisa nightly install"
            )),
            ok: false,
            last: Some(cycle),
        };
    }

    if !cycle.ok {
        let remedy = cycle.remedy.clone();
        return Standing {
            headline: format!(
                "The last nightly cycle failed, {}: {}",
                ago(age),
                cycle.detail
            ),
            remedy,
            ok: false,
            last: Some(cycle),
        };
    }

    if cycle.consecutive_skips >= SKIPS_BEFORE_SAYING_SO {
        return Standing {
            headline: format!(
                "The last {} cycles were skipped because this machine was working. \
                 It is not moving at all.",
                cycle.consecutive_skips
            ),
            remedy: Some(
                "Move it by hand at a quiet moment, or give the schedule a window this \
                 machine is idle in: lisa upgrade"
                    .to_string(),
            ),
            ok: false,
            last: Some(cycle),
        };
    }

    Standing {
        headline: format!(
            "The last nightly cycle was {}: {} — {}",
            ago(age),
            cycle.outcome,
            cycle.detail
        ),
        remedy: None,
        ok: true,
        last: Some(cycle),
    }
}

/// Run `lisa nightly status`.
pub(crate) fn run_status(json: bool) -> Result<i32, String> {
    let state = state_dir()?;
    let config = channel::load_from(&channel::config_path()?).unwrap_or_default();
    let standing = standing(last_cycle(&state)?, channel::now_unix());
    let exit_code = i32::from(!standing.ok);

    if json {
        let data = serde_json::json!({
            "state": if standing.ok { "ok" } else { "finding" },
            "channel": config.channel.map(|channel| channel.as_str()),
            "effective_channel": config.effective_channel().as_str(),
            "detail": standing.headline,
            "remedy": standing.remedy,
            "last_cycle": standing.last,
            "record": health_path(&state).to_string_lossy(),
        });
        return Ok(crate::json_output::emit(
            "nightly-status",
            crate::json_output::Outcome::verdict(data, exit_code),
        ));
    }

    println!("{}", standing.headline);
    if let Some(remedy) = &standing.remedy {
        println!("Remedy: {remedy}");
    }
    println!("Record: {}", health_path(&state).display());
    Ok(exit_code)
}

/// What `lisa nightly install` was asked for.
pub(crate) struct InstallArgs {
    /// The project the check runs against after a move.
    pub(crate) project: Option<PathBuf>,
    /// The command that carries a failure off this machine.
    pub(crate) alert: Option<String>,
    /// Print the job and change nothing.
    pub(crate) dry_run: bool,
}

/// The launchd job, exactly as it is written to disk.
///
/// launchd hands a job a minimal `PATH` — `/usr/bin:/bin:/usr/sbin:/sbin` — and
/// this arrangement needs three things that are not on it: `lisa` itself, the
/// Homebrew `zellij` the busy check asks about, and whichever agent client the
/// board runs. So the job carries its own, written down here rather than
/// inherited from whichever shell happened to run the install.
fn launchd_job(lisa: &Path, home: &Path, state: &Path, times: &[(u32, u32)]) -> String {
    let mut schedule = String::new();
    for (hour, minute) in times {
        schedule.push_str(&format!(
            "\t\t<dict>\n\t\t\t<key>Hour</key>\n\t\t\t<integer>{hour}</integer>\n\
             \t\t\t<key>Minute</key>\n\t\t\t<integer>{minute}</integer>\n\t\t</dict>\n"
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{LAUNCHD_LABEL}</string>

	<!-- Written by `lisa nightly install`. One cycle: skip if this machine is
	     working, move if the nightly channel has a soaked release, check the
	     new one against this box's own board, shout if it does not hold up. -->
	<key>ProgramArguments</key>
	<array>
		<string>{lisa}</string>
		<string>nightly</string>
		<string>run</string>
	</array>

	<!-- Before anyone looks, with two second chances for a box that was busy. -->
	<key>StartCalendarInterval</key>
	<array>
{schedule}	</array>

	<!-- Never at load: an upgrade is not something a login should trigger. -->
	<key>RunAtLoad</key>
	<false/>

	<key>EnvironmentVariables</key>
	<dict>
		<key>PATH</key>
		<string>{home}/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
	</dict>

	<key>StandardOutPath</key>
	<string>{out}</string>
	<key>StandardErrorPath</key>
	<string>{err}</string>

	<key>ProcessType</key>
	<string>Background</string>
</dict>
</plist>
"#,
        lisa = lisa.display(),
        home = home.display(),
        out = state.join("launchd.out").display(),
        err = state.join("launchd.err").display(),
    )
}

/// The systemd timer the same arrangement would be on a Linux box, named rather
/// than half-built: this ticket puts one macOS machine on nightly, and a Linux
/// box that wants the same thing should get it written for it, not guessed.
const SYSTEMD_HINT: &str = "This arrangement is a launchd job, which is macOS. On a Linux box the \
     equivalent is a systemd user timer running `lisa nightly run` — `systemctl --user edit \
     --force --full lisa-nightly.timer` — and `lisa nightly run` and `lisa nightly status` work \
     there today.";

/// Run `lisa nightly install`.
pub(crate) fn run_install(args: InstallArgs) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot find the running lisa: {error}"))?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "cannot find this machine's home directory: HOME is not set".to_string())?;
    let method = upgrade::classify_install(&exe, Some(&home));

    if matches!(method, InstallMethod::Homebrew | InstallMethod::Apt) {
        return Err(format!(
            "this lisa is managed by a package manager ({}), which carries one version and \
             cannot follow a channel, so an unattended nightly upgrade has nothing to do. \
             Move onto the channel-aware install first: lisa upgrade",
            exe.display()
        ));
    }

    let lisa = lisa_to_check(Some(&home), &exe);
    let state = state_dir()?;
    let job = launchd_job(&lisa, &home, &state, &RUN_TIMES);

    if args.dry_run {
        println!("Would write {}:\n", plist_path(&home)?.display());
        print!("{job}");
        return Ok(());
    }

    if !cfg!(target_os = "macos") {
        return Err(SYSTEMD_HINT.to_string());
    }

    // The channel first: a nightly job on a machine that has not said it wants
    // nightly would quietly follow stable and look like it was working.
    let config_path = channel::config_path()?;
    let mut config = channel::load_from(&config_path)?;
    config.channel = Some(Channel::Nightly);
    if let Some(project) = args.project {
        let project = project
            .canonicalize()
            .map_err(|error| format!("cannot use {}: {error}", project.display()))?;
        // Checked here rather than every night at 04:30: a directory `doctor`
        // will not answer about is an alarm that fires forever and means
        // nothing.
        if !project.join(".lisa.toml").exists() && !project.join("docs/active/tickets").is_dir() {
            return Err(format!(
                "{} is not a board Lisa knows — no .lisa.toml and no docs/active/tickets/ in \
                 it, so the nightly check would have nothing to ask about. Name a project \
                 this machine actually works.",
                project.display()
            ));
        }
        config.nightly_project = Some(project);
    }
    if let Some(alert) = args.alert {
        config.alert_command = Some(alert);
    }
    channel::save_to(&config_path, &config)?;

    std::fs::create_dir_all(&state)
        .map_err(|error| format!("cannot create {}: {error}", state.display()))?;

    let plist = plist_path(&home)?;
    if let Some(parent) = plist.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(&plist, &job)
        .map_err(|error| format!("cannot write {}: {error}", plist.display()))?;

    if std::env::var_os(LAUNCH_AGENTS_DIR_ENV).is_none() {
        load_job(&plist)?;
    }

    println!("This machine is on channel nightly, and it upgrades itself.");
    println!("  schedule   {}, {}", plist.display(), times_in_words());
    println!("  runs       {} nightly run", lisa.display());
    println!(
        "  records    {}\n             {}",
        health_path(&state).display(),
        history_path(&state).display()
    );
    match &config.nightly_project {
        Some(project) => println!("  checks     lisa doctor at {}", project.display()),
        None => println!(
            "  checks     the installed version only — give it a board to check against \
             with: lisa nightly install --project <path>"
        ),
    }
    match &config.alert_command {
        Some(command) => println!("  shouts     {command}"),
        None => println!(
            "  shouts     on this box only — nothing leaves the machine until you set one \
             with: lisa nightly install --alert '<command>'"
        ),
    }
    println!("\nAsk it where it stands:  lisa nightly status");
    println!("Take it off the schedule: lisa nightly uninstall");
    Ok(())
}

/// Run `lisa nightly uninstall`. The schedule goes; the channel and the record
/// stay, because a machine that stops upgrading itself has not stopped being a
/// nightly machine, and last night's record is the thing you are most likely to
/// want after turning the job off.
pub(crate) fn run_uninstall() -> Result<(), String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "cannot find this machine's home directory: HOME is not set".to_string())?;
    let plist = plist_path(&home)?;

    if !plist.exists() {
        println!("No nightly job is installed here ({}).", plist.display());
        return Ok(());
    }

    if std::env::var_os(LAUNCH_AGENTS_DIR_ENV).is_none() {
        unload_job(&plist);
    }
    std::fs::remove_file(&plist)
        .map_err(|error| format!("cannot remove {}: {error}", plist.display()))?;

    println!(
        "Removed {}. This machine no longer upgrades itself.",
        plist.display()
    );
    println!(
        "Its channel and its record are untouched: lisa doctor still says where it stands, \
         and lisa upgrade still moves it."
    );
    Ok(())
}

/// Where the job file goes.
fn plist_path(home: &Path) -> Result<PathBuf, String> {
    let dir = match std::env::var_os(LAUNCH_AGENTS_DIR_ENV) {
        Some(dir) => PathBuf::from(dir),
        None => home.join("Library").join("LaunchAgents"),
    };
    Ok(dir.join(format!("{LAUNCHD_LABEL}.plist")))
}

/// The schedule, in the words the install prints.
fn times_in_words() -> String {
    RUN_TIMES
        .iter()
        .map(|(hour, minute)| format!("{hour:02}:{minute:02}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The user's launchd domain, `gui/<uid>`.
fn gui_domain() -> String {
    let uid = Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|uid| uid.trim().to_string())
        .unwrap_or_default();
    format!("gui/{uid}")
}

/// Put the job in, replacing one that is already there.
fn load_job(plist: &Path) -> Result<(), String> {
    let domain = gui_domain();
    // An older copy of the job under the same label refuses the bootstrap, and
    // there is nothing to say about removing one that was not there.
    let _ = Command::new("launchctl")
        .args(["bootout", &domain])
        .arg(plist)
        .output();

    let loaded = Command::new("launchctl")
        .args(["bootstrap", &domain])
        .arg(plist)
        .output()
        .map_err(|error| format!("cannot run launchctl: {error}"))?;

    if loaded.status.success() {
        Ok(())
    } else {
        Err(format!(
            "launchctl would not load {}: {}. The job file is written; load it with: \
             launchctl bootstrap {domain} {}",
            plist.display(),
            String::from_utf8_lossy(&loaded.stderr).trim(),
            plist.display(),
        ))
    }
}

/// Take the job out. A job that was not loaded is not an error to remove.
fn unload_job(plist: &Path) {
    let _ = Command::new("launchctl")
        .args(["bootout", &gui_domain()])
        .arg(plist)
        .output();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> MachineConfig {
        MachineConfig {
            channel: Some(Channel::Nightly),
            ..MachineConfig::default()
        }
    }

    fn a_cycle(outcome: &str, ok: bool, at: i64) -> Cycle {
        cycle(
            at,
            outcome,
            ok,
            "something happened".to_string(),
            &config(),
            &Version::parse("0.5.0").unwrap(),
            None,
            None,
            None,
            0,
        )
    }

    const NOW: i64 = 1_786_000_000;

    #[test]
    fn a_machine_that_has_never_run_a_cycle_is_a_finding_not_a_silence() {
        let standing = standing(None, NOW);
        assert!(!standing.ok);
        assert!(
            standing.headline.contains("has ever run"),
            "{}",
            standing.headline
        );
        assert!(standing.remedy.unwrap().contains("lisa nightly install"));
    }

    #[test]
    fn a_record_older_than_a_night_says_the_schedule_is_not_running() {
        let stale = a_cycle(outcome::LEVEL, true, NOW - 3 * 86_400);
        let standing = standing(Some(stale), NOW);
        assert!(!standing.ok);
        assert!(
            standing.headline.contains("The schedule is not running"),
            "{}",
            standing.headline
        );
        assert!(standing.remedy.unwrap().contains(LAUNCHD_LABEL));
    }

    #[test]
    fn a_failed_cycle_carries_its_own_remedy_forward() {
        let mut failed = a_cycle(outcome::FAILED, false, NOW - 3600);
        failed.detail = "doctor fails under 0.6.0".to_string();
        failed.remedy = Some("lisa upgrade --tag v0.5.0".to_string());

        let standing = standing(Some(failed), NOW);
        assert!(!standing.ok);
        assert!(standing.headline.contains("doctor fails under 0.6.0"));
        assert_eq!(
            standing.remedy.as_deref(),
            Some("lisa upgrade --tag v0.5.0")
        );
    }

    #[test]
    fn a_machine_that_is_always_working_is_a_machine_that_never_moves() {
        let mut skipped = a_cycle(outcome::SKIPPED, true, NOW - 3600);
        skipped.consecutive_skips = SKIPS_BEFORE_SAYING_SO;

        let standing = standing(Some(skipped), NOW);
        assert!(!standing.ok, "three skipped nights is a finding");
        assert!(
            standing.headline.contains("not moving at all"),
            "{}",
            standing.headline
        );
    }

    #[test]
    fn one_skipped_night_is_not_a_finding() {
        let mut skipped = a_cycle(outcome::SKIPPED, true, NOW - 3600);
        skipped.consecutive_skips = 1;
        assert!(standing(Some(skipped), NOW).ok);
    }

    #[test]
    fn a_recent_ordinary_cycle_is_quiet() {
        let standing = standing(Some(a_cycle(outcome::LEVEL, true, NOW - 7200)), NOW);
        assert!(standing.ok);
        assert!(standing.remedy.is_none());
        assert!(
            standing.headline.contains("2h ago"),
            "{}",
            standing.headline
        );
    }

    #[test]
    fn the_job_runs_lisa_by_absolute_path_and_carries_a_path_of_its_own() {
        let job = launchd_job(
            Path::new("/Users/someone/.local/bin/lisa"),
            Path::new("/Users/someone"),
            Path::new("/Users/someone/Library/Application Support/io.johnhkchen.lisa/nightly"),
            &RUN_TIMES,
        );

        assert!(job.contains("<string>/Users/someone/.local/bin/lisa</string>"));
        assert!(job.contains("<string>nightly</string>"));
        assert!(job.contains("<string>run</string>"));
        assert!(job.contains(&format!("<string>{LAUNCHD_LABEL}</string>")));
        // The three tries, and never at login.
        assert_eq!(job.matches("<key>Hour</key>").count(), RUN_TIMES.len());
        assert!(job.contains("<key>RunAtLoad</key>\n\t<false/>"));
        // Homebrew's zellij and the installer's lisa are both off launchd's PATH.
        assert!(job.contains("/Users/someone/.local/bin:/opt/homebrew/bin:"));
        assert!(job.contains("launchd.out"));
        assert!(job.contains("launchd.err"));
    }

    #[test]
    fn the_record_lands_next_to_the_channel_and_reads_back() {
        let temp = tempfile::tempdir().expect("temp dir");
        let state = temp.path().join("nightly");

        assert!(last_cycle(&state).unwrap().is_none());

        let written = a_cycle(outcome::MOVED, true, NOW);
        record(&state, &written).expect("record");
        record(&state, &a_cycle(outcome::LEVEL, true, NOW + 86_400)).expect("second record");

        let read = last_cycle(&state).unwrap().expect("a cycle");
        assert_eq!(read.outcome, outcome::LEVEL);
        assert_eq!(read.at_utc, channel::format_rfc3339_utc(NOW + 86_400));

        // History keeps both, one line each, oldest first.
        let history = std::fs::read_to_string(history_path(&state)).expect("history");
        let lines: Vec<&str> = history.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains(outcome::MOVED), "{}", lines[0]);
        assert!(lines[1].contains(outcome::LEVEL), "{}", lines[1]);
        assert_eq!(written.at_utc, channel::format_rfc3339_utc(NOW));
    }

    #[test]
    fn the_binary_checked_after_a_move_is_the_one_the_installer_writes() {
        let temp = tempfile::tempdir().expect("temp dir");
        let home = temp.path();
        let running = home.join("src/lisa/target/debug/lisa");

        // Nothing in ~/.local/bin yet: the running build is all there is.
        assert_eq!(lisa_to_check(Some(home), &running), running);

        let installed = upgrade::installer_owned_path(home);
        std::fs::create_dir_all(installed.parent().unwrap()).expect("bin dir");
        std::fs::write(&installed, "#!/bin/sh\n").expect("write lisa");
        assert_eq!(lisa_to_check(Some(home), &running), installed);
    }

    #[test]
    fn an_alarm_with_nowhere_to_go_says_so_rather_than_reading_as_sent() {
        std::env::set_var(NOTIFY_ENV, "off");
        let told = raise_alarm(&config(), &a_cycle(outcome::FAILED, false, NOW));
        std::env::remove_var(NOTIFY_ENV);

        assert_eq!(told.len(), 1);
        assert!(told[0].contains("no alert_command"), "{}", told[0]);
    }

    #[test]
    fn the_operators_own_alarm_gets_the_whole_record_on_stdin() {
        let temp = tempfile::tempdir().expect("temp dir");
        let landed = temp.path().join("alarm.json");
        let mut config = config();
        config.alert_command = Some(format!("cat > {}", landed.display()));

        std::env::set_var(NOTIFY_ENV, "off");
        let told = raise_alarm(&config, &a_cycle(outcome::FAILED, false, NOW));
        std::env::remove_var(NOTIFY_ENV);

        assert!(told.contains(&"alert_command ran".to_string()), "{told:?}");
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&landed).expect("alarm")).unwrap();
        assert_eq!(body["outcome"], outcome::FAILED);
        assert_eq!(body["ok"], false);
    }

    #[test]
    fn a_failing_alarm_is_reported_rather_than_swallowed() {
        let mut config = config();
        config.alert_command = Some("exit 3".to_string());

        std::env::set_var(NOTIFY_ENV, "off");
        let told = raise_alarm(&config, &a_cycle(outcome::FAILED, false, NOW));
        std::env::remove_var(NOTIFY_ENV);

        assert!(
            told.iter()
                .any(|line| line.contains("alert_command exited")),
            "{told:?}"
        );
    }

    /// A stand-in for the lisa that was just installed: a script that answers
    /// `--version` and `doctor --json` with whatever this test needs.
    fn fake_lisa(dir: &Path, version: &str, doctor: &str) -> PathBuf {
        let path = dir.join("lisa");
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = \"--version\" ]; then echo 'lisa {version}'; exit 0; fi\n\
                 cat <<'JSON'\n{doctor}\nJSON\n"
            ),
        )
        .expect("write the fake lisa");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("make it runnable");
        }
        path
    }

    #[test]
    fn a_cycle_with_no_project_checks_the_version_and_says_so() {
        let temp = tempfile::tempdir().expect("temp dir");
        let lisa = fake_lisa(temp.path(), "0.6.0", "{}");

        let detail = verify(&lisa, &Version::parse("0.6.0").unwrap(), None).expect("verified");
        assert!(detail.contains("0.6.0"), "{detail}");
        assert!(
            detail.contains("No project is set for the check"),
            "the check must not imply more than it asked: {detail}"
        );
    }

    #[test]
    fn a_release_that_did_not_actually_land_is_caught_before_it_is_called_a_move() {
        let temp = tempfile::tempdir().expect("temp dir");
        let lisa = fake_lisa(temp.path(), "0.5.0-rc.2", "{}");

        let error = verify(&lisa, &Version::parse("0.6.0").unwrap(), None).expect_err("caught");
        assert!(error.contains("reports 0.5.0-rc.2"), "{error}");
    }

    #[test]
    fn a_zellij_the_new_release_cannot_use_fails_the_cycle_by_name() {
        let temp = tempfile::tempdir().expect("temp dir");
        let project = temp.path().join("board");
        std::fs::create_dir_all(&project).expect("board");
        let lisa = fake_lisa(
            temp.path(),
            "0.6.0",
            r#"{"ok":true,"data":{"verdict":"failed","checks":[
                 {"name":"lisa","status":"ok","detail":"channel nightly"},
                 {"name":"zellij","status":"unsupported","detail":"Unsupported system Zellij 0.42.0; Lisa requires >= 0.43.0"}
               ]}}"#,
        );

        let error =
            verify(&lisa, &Version::parse("0.6.0").unwrap(), Some(&project)).expect_err("caught");
        assert!(error.contains("zellij"), "{error}");
        assert!(
            error.contains("0.43.0"),
            "the row's own words carry: {error}"
        );
    }

    #[test]
    fn a_board_that_comes_up_clean_under_the_new_release_is_the_pass() {
        let temp = tempfile::tempdir().expect("temp dir");
        let project = temp.path().join("board");
        std::fs::create_dir_all(&project).expect("board");
        let lisa = fake_lisa(
            temp.path(),
            "0.6.0",
            r#"{"ok":true,"data":{"verdict":"passed","checks":[]}}"#,
        );

        let detail =
            verify(&lisa, &Version::parse("0.6.0").unwrap(), Some(&project)).expect("verified");
        assert!(detail.contains("doctor passes"), "{detail}");
    }

    #[test]
    fn a_doctor_that_cannot_answer_in_json_is_a_failure_not_a_pass() {
        let temp = tempfile::tempdir().expect("temp dir");
        let project = temp.path().join("board");
        std::fs::create_dir_all(&project).expect("board");
        // An older lisa — a rollback target, say — has no `doctor --json`.
        let lisa = fake_lisa(
            temp.path(),
            "0.4.4",
            "error: unexpected argument '--json' found",
        );

        let error =
            verify(&lisa, &Version::parse("0.4.4").unwrap(), Some(&project)).expect_err("caught");
        assert!(error.contains("did not answer in JSON"), "{error}");
    }

    #[test]
    fn ages_read_the_way_a_person_says_them() {
        assert_eq!(ago(0), "0m ago");
        assert_eq!(ago(1800), "30m ago");
        assert_eq!(ago(7200), "2h ago");
        assert_eq!(ago(90_000), "a day ago");
        assert_eq!(ago(3 * 86_400), "3 days ago");
    }

    #[test]
    fn applescript_quoting_survives_a_quote_and_a_newline() {
        assert_eq!(
            applescript_string("doctor said \"no\"\nand stopped"),
            "\"doctor said \\\"no\\\" and stopped\""
        );
    }
}
