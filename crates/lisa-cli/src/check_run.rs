//! The execution contract a recorded check runs under.
//!
//! One runner, three callers: `lisa unblock`, the automated world recheck, and
//! the reviewer-side `lisa check-disposition`. They share this module precisely
//! so they cannot drift into disagreeing about what a check can see, how long it
//! gets, or what its exit code means — a reviewer authoring a check reads one
//! contract, in `docs/knowledge/lisa-workflow.md`, and it describes this file.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use lisa_core::disposition::resolve_check_budget_secs;

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_CAPTURE_BYTES: u64 = 8 * 1024;
const MAX_OBSERVATION_CHARS: usize = 240;
/// How many sanitized lines per stream a decline puts on screen. This bounds
/// what is *shown*; [`MAX_CAPTURE_BYTES`] still bounds what is captured, and an
/// 8 KiB capture printed whole is a wall rather than a report.
const MAX_OBSERVED_LINES: usize = 10;

/// How a check ended.
///
/// [`CheckResult::Failed`] and [`CheckResult::Inconclusive`] are different facts
/// about the world, not different wordings of one: a check that ran and said no
/// is evidence about the remedy, and a check that could not look is evidence
/// about nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckResult {
    Passed,
    Failed,
    Inconclusive,
    TimedOut,
}

/// One check run, and every fact needed to report it honestly.
///
/// The classification alone cannot be reported: attributing a finding means
/// naming what ran, where, for how long, and what it exited with, and all of
/// those live inside [`run_check`]. They are carried out rather than recomputed —
/// in particular [`CheckRun::directory`] is the value that was handed to
/// `current_dir` and [`CheckRun::budget`] is the budget actually enforced, so
/// both stay true when a later caller changes either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckRun {
    pub(crate) result: CheckResult,
    /// The check string exactly as the disposition recorded it.
    pub(crate) check: String,
    /// The directory the check actually ran in.
    pub(crate) directory: PathBuf,
    /// The time budget this run was actually held to.
    pub(crate) budget: Duration,
    /// `None` when the check was stopped rather than exiting on its own.
    pub(crate) exit_code: Option<i32>,
    /// Sanitized non-empty lines, capped for display.
    pub(crate) stdout: Vec<String>,
    pub(crate) stderr: Vec<String>,
    /// Lines this stream had beyond the display cap.
    pub(crate) stdout_dropped: usize,
    pub(crate) stderr_dropped: usize,
}

/// The time a check with this declared budget actually gets.
pub(crate) fn budget_for(declared: Option<u64>) -> Duration {
    Duration::from_secs(resolve_check_budget_secs(declared))
}

/// A budget in the units the person who declared it was thinking in.
///
/// A reviewer writes `"check_timeout_secs": 1500` while thinking "twenty-five
/// minutes", and reads the expiry sentence in the same units.
pub(crate) fn format_budget(budget: Duration) -> String {
    let seconds = budget.as_secs();
    let plural = |value: u64, unit: &str| {
        if value == 1 {
            format!("1 {unit}")
        } else {
            format!("{value} {unit}s")
        }
    };
    match (seconds / 60, seconds % 60) {
        (0, seconds) => plural(seconds, "second"),
        (minutes, 0) => plural(minutes, "minute"),
        (minutes, seconds) => format!(
            "{} {}",
            plural(minutes, "minute"),
            plural(seconds, "second")
        ),
    }
}

/// Run one recorded check against the project itself.
///
/// The check runs in `root` — the tree the operator changed, and the only tree
/// whose state they can act on. It sees every file that is there: tracked,
/// untracked, and gitignored alike. A relative path in a check therefore
/// resolves the way it would in the operator's own shell, which is the whole
/// point; checks used to run against a `git ls-files --exclude-standard` copy,
/// where every build output and every fetched dependency was missing by
/// construction and a check that read one reported a failure that was not true.
///
/// What the check gets: the project root as its working directory, a null
/// stdin, a disposable `TMPDIR`, a time budget enforced against the whole
/// process group, and captured output. What it does not get is protection from
/// its own writes, and Lisa no longer judges whether it wrote. That is
/// deliberate rather than overlooked: a before/after fingerprint of a live tree
/// cannot tell this check's writes from a concurrent agent thread's — the
/// scheduler fires `recheck-world` while sessions are editing the same files —
/// and reporting another writer's changes as the check's would be the same kind
/// of false verdict. The read-only requirement lives in the check contract that
/// the reviewer writes against, and `lisa check-disposition` is where an
/// unrunnable check is caught while that reviewer can still fix it.
pub(crate) fn run_check(root: &Path, check: &str, budget: Duration) -> Result<CheckRun, String> {
    let scratch =
        tempfile::tempdir().map_err(|error| format!("Could not prepare a safe check: {error}"))?;
    let mut stdout =
        tempfile::tempfile().map_err(|error| format!("Could not prepare check output: {error}"))?;
    let mut stderr =
        tempfile::tempfile().map_err(|error| format!("Could not prepare check output: {error}"))?;

    // Cloned out rather than read back later: this is the directory the check is
    // about to be given, and it is what the report names.
    let directory = root.to_path_buf();

    let mut command = Command::new("/bin/sh");
    command
        .arg("-c")
        .arg(check)
        .current_dir(&directory)
        .env("TMPDIR", scratch.path())
        .env("TMP", scratch.path())
        .env("TEMP", scratch.path())
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout.try_clone().map_err(|error| {
            format!("Could not prepare check output: {error}")
        })?))
        .stderr(Stdio::from(stderr.try_clone().map_err(|error| {
            format!("Could not prepare check output: {error}")
        })?));

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start the check: {error}"))?;
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child
            .try_wait()
            .map_err(|error| format!("Could not observe the check: {error}"))?
        {
            Some(status) => break status,
            None if started.elapsed() >= budget => {
                timed_out = true;
                terminate_check(&mut child);
                break child
                    .wait()
                    .map_err(|error| format!("Could not stop the check: {error}"))?;
            }
            None => thread::sleep(POLL_INTERVAL.min(budget)),
        }
    };

    // Read on every path, including the two that used to return early: a check
    // that timed out or wrote to the tree has still usually said something, and
    // an operator staring at a decline needs it.
    let stdout = read_capture(&mut stdout)
        .map_err(|error| format!("Could not read check output: {error}"))?;
    let stderr = read_capture(&mut stderr)
        .map_err(|error| format!("Could not read check output: {error}"))?;
    let (stdout_lines, stdout_dropped) = observed_lines(&stdout);
    let (stderr_lines, stderr_dropped) = observed_lines(&stderr);

    let (result, exit_code) = if timed_out {
        (CheckResult::TimedOut, None)
    } else {
        classify_exit(status.code())
    };

    Ok(CheckRun {
        result,
        check: check.to_string(),
        directory,
        budget,
        exit_code,
        stdout: stdout_lines,
        stderr: stderr_lines,
        stdout_dropped,
        stderr_dropped,
    })
}

/// Split "the check looked and said no" from "the check could not look".
///
/// The distinguished codes are the ones that mean the check never reached its
/// question: 2 is the long-standing "trouble, not a verdict" code (grep, diff,
/// and the field script this ticket comes from), and 126/127 are what
/// `/bin/sh -c` itself returns when the recorded command is not executable or
/// not found. A check killed by a signal concluded nothing either. None of those
/// is evidence that the operator's remedy was not done.
fn classify_exit(code: Option<i32>) -> (CheckResult, Option<i32>) {
    match code {
        Some(0) => (CheckResult::Passed, Some(0)),
        Some(code @ (2 | 126 | 127)) => (CheckResult::Inconclusive, Some(code)),
        Some(code) => (CheckResult::Failed, Some(code)),
        None => (CheckResult::Inconclusive, None),
    }
}

#[cfg(unix)]
fn terminate_check(child: &mut std::process::Child) {
    // The shell starts as the leader of its own process group, so a negative
    // PID reaches descendants as well as the wrapper process.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn terminate_check(child: &mut std::process::Child) {
    let _ = child.kill();
}

fn read_capture(file: &mut File) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take(MAX_CAPTURE_BYTES).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// One captured stream as display lines, plus how many were left off.
///
/// Every line goes through [`sanitize_observation`], so escape sequences and
/// control characters never reach the terminal no matter which stream they came
/// from; lines that sanitize to nothing are dropped rather than shown blank.
pub(crate) fn observed_lines(bytes: &[u8]) -> (Vec<String>, usize) {
    let decoded = String::from_utf8_lossy(bytes);
    let mut lines = Vec::new();
    let mut dropped = 0;
    for line in decoded.lines() {
        let line = sanitize_observation(line);
        if line.is_empty() {
            continue;
        }
        if lines.len() < MAX_OBSERVED_LINES {
            lines.push(line);
        } else {
            dropped += 1;
        }
    }
    (lines, dropped)
}

pub(crate) fn sanitize_observation(line: &str) -> String {
    let mut sanitized = String::new();
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' && characters.peek() == Some(&'[') {
            characters.next();
            for sequence_character in characters.by_ref() {
                if ('@'..='~').contains(&sequence_character) {
                    break;
                }
            }
        } else if character == '\t' {
            sanitized.push(' ');
        } else if !character.is_control() {
            sanitized.push(character);
        }
    }
    sanitized
        .trim()
        .chars()
        .take(MAX_OBSERVATION_CHARS)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use lisa_core::disposition::{DEFAULT_CHECK_BUDGET_SECS, MAX_CHECK_BUDGET_SECS};

    use super::*;

    fn run(root: &Path, check: &str) -> CheckRun {
        run_check(root, check, Duration::from_secs(5)).unwrap()
    }

    #[test]
    fn passing_and_failing_checks_carry_the_command_directory_and_code() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("ready"), "yes").unwrap();

        let passed = run(root.path(), "test -f ready");
        assert_eq!(passed.result, CheckResult::Passed);
        assert_eq!(passed.exit_code, Some(0));

        let failed = run(
            root.path(),
            "printf 'the key link still returns 404\\nextra detail\\n' >&2; \
             printf 'sampled 40 of 40\\n'; exit 1",
        );
        assert_eq!(failed.result, CheckResult::Failed);
        assert_eq!(failed.exit_code, Some(1));
        assert_eq!(
            failed.check,
            "printf 'the key link still returns 404\\nextra detail\\n' >&2; \
             printf 'sampled 40 of 40\\n'; exit 1"
        );
        assert_eq!(
            failed.stderr,
            vec!["the key link still returns 404", "extra detail"]
        );
        assert_eq!(failed.stdout, vec!["sampled 40 of 40"]);
    }

    /// The reported directory is the one the check itself observed — so this
    /// keeps holding if a later ticket moves where checks run.
    #[test]
    fn the_reported_directory_is_the_one_the_check_observed() {
        let root = tempfile::tempdir().unwrap();

        let observed = run(root.path(), "pwd -P; exit 1");

        assert_eq!(observed.stdout.len(), 1);
        let seen = &observed.stdout[0];
        let reported = observed.directory.display().to_string();
        // The directory is gone by now (the fixture is disposable), so neither
        // side can be canonicalized after the fact. `pwd -P` resolves symlinks
        // and the reported path does not, which on macOS is exactly the
        // `/private` prefix — the same directory, spelled physically.
        assert!(
            *seen == reported || seen.ends_with(&reported),
            "check saw {seen}, report named {reported}"
        );
    }

    /// "Could not look" is not "did not pass", and the line is the exit code.
    #[test]
    fn exit_two_and_shell_failures_are_inconclusive_not_a_verdict() {
        let root = tempfile::tempdir().unwrap();

        for (check, code) in [
            ("exit 2", 2),
            ("exit 126", 126),
            ("./definitely-not-here", 127),
        ] {
            let inconclusive = run(root.path(), check);
            assert_eq!(
                inconclusive.result,
                CheckResult::Inconclusive,
                "{check} must not read as a verdict"
            );
            assert_eq!(inconclusive.exit_code, Some(code));
        }

        for check in ["exit 1", "exit 3"] {
            assert_eq!(
                run(root.path(), check).result,
                CheckResult::Failed,
                "{check}"
            );
        }
    }

    /// The root cause, at the unit level: a check reads the project it was
    /// given, gitignored build output included.
    ///
    /// `out/` here is exactly the shape `--exclude-standard` used to drop —
    /// present on disk, absent from `git ls-files` — and the check reads it and
    /// a tracked source file in one run, from one working directory.
    #[test]
    fn a_check_reads_the_project_it_runs_in() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("out")).unwrap();
        fs::write(root.path().join("out/marker"), "built").unwrap();
        fs::write(root.path().join(".gitignore"), "out/\n").unwrap();
        fs::write(root.path().join("tracked.txt"), "source").unwrap();

        let seen = run(root.path(), "test -f out/marker && test -f tracked.txt");

        assert_eq!(seen.result, CheckResult::Passed);
        assert_eq!(seen.exit_code, Some(0));
    }

    /// The check runs in the project, not beside it.
    #[test]
    fn the_check_runs_in_the_project_root() {
        let root = tempfile::tempdir().unwrap();

        let ran = run(root.path(), "exit 1");

        assert_eq!(ran.directory, root.path());
    }

    /// The documented write behaviour, pinned.
    ///
    /// The contract says a check must only look. Lisa does not enforce that —
    /// it cannot, under a scheduler that edits the same tree concurrently — and
    /// this is what "does not enforce" means concretely: a writing check is run
    /// like any other, judged only by its exit code, and its writes land in the
    /// project. No result variant reports the write, so no operator is ever told
    /// a check "changed files" when another thread did.
    #[test]
    fn a_writing_check_is_judged_by_its_exit_code_alone() {
        let root = tempfile::tempdir().unwrap();

        let wrote = run(
            root.path(),
            "printf 'built\\n' > artifact; test -f artifact",
        );

        assert_eq!(wrote.result, CheckResult::Passed);
        assert_eq!(wrote.exit_code, Some(0));
        assert_eq!(
            fs::read_to_string(root.path().join("artifact")).unwrap(),
            "built\n",
            "the write reaches the project rather than a copy of it"
        );

        // And a writing check that fails is a failure, on its exit code, not on
        // the fact that it wrote.
        let wrote_then_failed = run(root.path(), "printf 'x\\n' > second; exit 1");
        assert_eq!(wrote_then_failed.result, CheckResult::Failed);
        assert!(root.path().join("second").exists());
    }

    #[test]
    fn timeout_is_bounded_and_kills_the_shell_group() {
        let root = tempfile::tempdir().unwrap();
        let started = Instant::now();

        let timed_out =
            run_check(root.path(), "sleep 5 & wait", Duration::from_millis(60)).unwrap();

        assert_eq!(timed_out.result, CheckResult::TimedOut);
        assert_eq!(timed_out.budget, Duration::from_millis(60));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    /// A check inside its budget finishes on its own; the same check with a
    /// smaller budget is stopped. The run reports the budget it was held to.
    #[test]
    fn a_declared_budget_is_the_budget_enforced() {
        let root = tempfile::tempdir().unwrap();

        let inside = run_check(root.path(), "sleep 0.05", Duration::from_secs(2)).unwrap();
        assert_eq!(inside.result, CheckResult::Passed);
        assert_eq!(inside.budget, Duration::from_secs(2));

        let outside = run_check(root.path(), "sleep 2", Duration::from_millis(50)).unwrap();
        assert_eq!(outside.result, CheckResult::TimedOut);
        assert_eq!(outside.budget, Duration::from_millis(50));
    }

    #[test]
    fn budget_resolution_defaults_and_clamps_to_the_documented_bounds() {
        assert_eq!(
            budget_for(None),
            Duration::from_secs(DEFAULT_CHECK_BUDGET_SECS)
        );
        assert_eq!(budget_for(Some(1500)), Duration::from_secs(1500));
        assert_eq!(
            budget_for(Some(u64::MAX)),
            Duration::from_secs(MAX_CHECK_BUDGET_SECS)
        );
    }

    #[test]
    fn budgets_read_in_the_units_they_were_written_in() {
        for (seconds, expected) in [
            (1, "1 second"),
            (5, "5 seconds"),
            (59, "59 seconds"),
            (60, "1 minute"),
            (90, "1 minute 30 seconds"),
            (1500, "25 minutes"),
            (1800, "30 minutes"),
        ] {
            assert_eq!(format_budget(Duration::from_secs(seconds)), expected);
        }
    }

    #[test]
    fn observed_lines_strip_controls_fold_tabs_and_cap_length_and_count() {
        let long = "x".repeat(MAX_OBSERVATION_CHARS + 20);
        let stream = format!("\x1b[31m  observed\t{long}  \n\n\x1b[0m\n");

        let (lines, dropped) = observed_lines(stream.as_bytes());

        assert_eq!(lines.len(), 1, "blank and escape-only lines are dropped");
        assert_eq!(dropped, 0);
        assert!(lines[0].starts_with("observed "));
        assert_eq!(lines[0].chars().count(), MAX_OBSERVATION_CHARS);
        assert!(!lines[0].contains('\n') && !lines[0].contains('\t'));
        assert!(!lines[0].contains('\u{1b}'));

        let many: String = (0..MAX_OBSERVED_LINES + 4)
            .map(|index| format!("line {index}\n"))
            .collect();
        let (capped, dropped) = observed_lines(many.as_bytes());
        assert_eq!(capped.len(), MAX_OBSERVED_LINES);
        assert_eq!(dropped, 4);
    }
}
