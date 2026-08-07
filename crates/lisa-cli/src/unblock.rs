//! Verify an optional parked-remedy check, then restore ordinary scheduling.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use lisa_core::completion_journal::MAX_ACTION_REQUIRED_GENERATIONS;
use lisa_core::parking::collect_parked_remedies;
use lisa_core::provenance::{
    append_check_override_record, system_time_to_epoch, CheckOverrideOutcome, CheckOverrideRecord,
    CheckOverrideType, SCHEMA_VERSION,
};
use lisa_core::ticket;
use lisa_core::types::TicketStatus;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::completion_seal;
use crate::config;
use lisa_core::disposition::RemedyOwner;

const CHECK_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_CAPTURE_BYTES: u64 = 8 * 1024;
const MAX_OBSERVATION_CHARS: usize = 240;
/// How many sanitized lines per stream a decline puts on screen. This bounds
/// what is *shown*; [`MAX_CAPTURE_BYTES`] still bounds what is captured, and an
/// 8 KiB capture printed whole is a wall rather than a report.
const MAX_OBSERVED_LINES: usize = 10;
const PROVENANCE_PATH: &str = ".lisa/provenance.jsonl";
/// Who a forced unblock is attributed to. `lisa unblock` is an operator command.
const OPERATOR_ACTOR: &str = "operator";

/// The lead sentence for a check that ran and reported no.
///
/// Deliberately Lisa's own words. The check's words appear only under the
/// attribution label below, because a sentence written by the project's script
/// is evidence, not Lisa's verdict.
const DECLINE_FAILED: &str = "That didn't work yet — the check ran and did not pass.";
/// The lead sentence for a check that never got to look.
///
/// It must not read as a judgement on the operator's work, and it always names
/// the way through — the override is printed under every decline.
const DECLINE_INCONCLUSIVE: &str =
    "Lisa can't tell yet — the check stopped before it could look, so this isn't a judgement on \
     your work.";
const DECLINE_CHANGED_FILES: &str = "That didn't work yet — it tried to change project files.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnblockOutcome {
    Reopened(String),
    Declined(String),
}

/// How a check ended.
///
/// [`CheckResult::Failed`] and [`CheckResult::Inconclusive`] are different facts
/// about the world, not different wordings of one: a check that ran and said no
/// is evidence about the remedy, and a check that could not look is evidence
/// about nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckResult {
    Passed,
    Failed,
    Inconclusive,
    TimedOut,
    ChangedFiles,
}

/// One check run, and every fact needed to report it honestly.
///
/// The classification alone cannot be reported: attributing a finding means
/// naming what ran, where, and what it exited with, and all of those live inside
/// [`run_check`]. They are carried out rather than recomputed — in particular
/// [`CheckRun::directory`] is the value that was handed to `current_dir`, so it
/// stays true if that directory ever changes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckRun {
    result: CheckResult,
    /// The check string exactly as the disposition recorded it.
    check: String,
    /// The directory the check actually ran in.
    directory: PathBuf,
    /// `None` when the check was stopped rather than exiting on its own.
    exit_code: Option<i32>,
    /// Sanitized non-empty lines, capped for display.
    stdout: Vec<String>,
    stderr: Vec<String>,
    /// Lines this stream had beyond the display cap.
    stdout_dropped: usize,
    stderr_dropped: usize,
}

/// Verify one parked ticket and restore `status: open` only when safe.
///
/// `override_check` covers exactly one gate: a check that declined. An operator
/// who has performed the ask and verified it themselves is never held by a check
/// they cannot satisfy — and the override is recorded, so a forced unblock stays
/// distinguishable afterwards from a check that passed. Every other decline
/// (unknown ticket, a ticket that is not waiting, a missing remedy, a completion
/// Lisa stopped recording) is untouched by it, because none of those is a gate
/// having done the ask would clear.
pub fn run_unblock(
    root: &Path,
    ticket_id: &str,
    override_check: bool,
) -> Result<UnblockOutcome, String> {
    let validation = config::load_config(root)?;
    let resolved = config::resolve_config(&validation.config, None, None);
    let ticket_dir = root.join(&resolved.ticket_dir);
    let work_dir = root.join(&resolved.work_dir);
    let tickets = ticket::scan_tickets(&ticket_dir)
        .map_err(|error| format!("Could not read the ticket board: {error}"))?;
    let Some(ticket) = tickets.iter().find(|ticket| ticket.id == ticket_id) else {
        return Ok(UnblockOutcome::Declined(format!(
            "I couldn't find {ticket_id}."
        )));
    };
    if ticket.status != TicketStatus::Blocked {
        return Ok(UnblockOutcome::Declined(format!(
            "{ticket_id} isn't waiting."
        )));
    }
    if let Some(decline) = recording_failure_decline(root, ticket_id) {
        return Ok(UnblockOutcome::Declined(decline));
    }

    let mut remedies = collect_parked_remedies(
        std::iter::once(ticket),
        &work_dir,
        &root.join(PROVENANCE_PATH),
    );
    let Some(remedy) = remedies.pop() else {
        return Ok(UnblockOutcome::Declined(format!(
            "I couldn't find what {ticket_id} is waiting for."
        )));
    };

    let mut overrode = false;
    if let Some(check) = remedy.check {
        let run = run_check(root, &check, CHECK_TIMEOUT)?;
        if run.result != CheckResult::Passed {
            if !override_check {
                return Ok(UnblockOutcome::Declined(decline_report(ticket_id, &run)));
            }
            // The receipt lands before the flip. An override that left no trace
            // would be indistinguishable afterwards from a check that passed,
            // which is the whole point of recording it; the other order can
            // reach that state, this one cannot.
            record_check_override(root, &resolved, ticket_id, &run)?;
            overrode = true;
        }
    }

    ticket::update_ticket_status(&ticket.file_path, TicketStatus::Open)
        .map_err(|error| format!("Could not let {ticket_id} run again: {error}"))?;
    Ok(UnblockOutcome::Reopened(if overrode {
        format!("{ticket_id} can run again — you overrode its check.")
    } else {
        format!("{ticket_id} can run again.")
    }))
}

/// File the receipt for a check an operator decided over.
fn record_check_override(
    root: &Path,
    resolved: &config::ResolvedConfig,
    ticket_id: &str,
    run: &CheckRun,
) -> Result<(), String> {
    let mut observed = run.stderr.clone();
    observed.extend(run.stdout.iter().cloned());
    let record = CheckOverrideRecord {
        schema_version: SCHEMA_VERSION,
        seal: completion_seal::resolve_for_inspection(root, resolved.completion_mode),
        record_type: CheckOverrideType::CheckOverride,
        ticket_id: ticket_id.to_string(),
        actor: OPERATOR_ACTOR.to_string(),
        check: run.check.clone(),
        directory: run.directory.display().to_string(),
        result: override_outcome(run.result),
        exit_code: run.exit_code,
        observed,
        occurred_at: system_time_to_epoch(SystemTime::now()),
    };
    append_check_override_record(&root.join(PROVENANCE_PATH), &record)
        .map_err(|error| format!("Could not record the override: {error}"))
}

fn override_outcome(result: CheckResult) -> CheckOverrideOutcome {
    match result {
        CheckResult::Passed => unreachable!("a passing check is never overridden"),
        CheckResult::Failed => CheckOverrideOutcome::Failed,
        CheckResult::Inconclusive => CheckOverrideOutcome::Inconclusive,
        CheckResult::TimedOut => CheckOverrideOutcome::TimedOut,
        CheckResult::ChangedFiles => CheckOverrideOutcome::ChangedFiles,
    }
}

/// Verify every observable world-owned parked remedy and reopen only passes.
///
/// This is deliberately stricter than [`run_unblock`]: automation cannot act
/// for an operator or agent, and a world-owned remedy without a check has no
/// positive evidence that external reality changed. Ordinary failed, timed
/// out, or mutating checks are expected no-op observations rather than command
/// failures.
pub(crate) fn run_world_rechecks(root: &Path) -> Result<Vec<String>, String> {
    let validation = config::load_config(root)?;
    let resolved = config::resolve_config(&validation.config, None, None);
    let ticket_dir = root.join(&resolved.ticket_dir);
    let work_dir = root.join(&resolved.work_dir);
    let tickets = ticket::scan_tickets(&ticket_dir)
        .map_err(|error| format!("Could not read the ticket board: {error}"))?;
    let remedies = collect_parked_remedies(tickets.iter(), &work_dir, &root.join(PROVENANCE_PATH));
    let mut reopened = Vec::new();

    for remedy in remedies {
        if remedy.remedy_owner != RemedyOwner::World {
            continue;
        }
        let Some(check) = remedy.check else {
            continue;
        };
        let Some(ticket) = tickets.iter().find(|ticket| ticket.id == remedy.ticket_id) else {
            continue;
        };

        match run_check(root, &check, CHECK_TIMEOUT)
            .map_err(|error| format!("Could not recheck {}: {error}", remedy.ticket_id))?
            .result
        {
            CheckResult::Passed => {
                ticket::update_ticket_status(&ticket.file_path, TicketStatus::Open).map_err(
                    |error| format!("Could not let {} run again: {error}", remedy.ticket_id),
                )?;
                reopened.push(remedy.ticket_id);
            }
            // Automation never acts on a non-pass, and it gains no override:
            // an operator can say "I checked this myself", a timer cannot.
            CheckResult::Failed
            | CheckResult::Inconclusive
            | CheckResult::TimedOut
            | CheckResult::ChangedFiles => {}
        }
    }

    Ok(reopened)
}

/// The one case unblock now steps aside for.
///
/// Unblock's meaning is unchanged: verify what changed, and let a waiting
/// ticket run again. But a completion Lisa has stopped trying to record is not
/// waiting on the review, and reopening it would hand it a session to fail in
/// again. That is a case unblock cannot fix and `already-done` can, so it says
/// so rather than reporting success and changing nothing that matters.
///
/// A ticket with no journal record, or one still under the bound, is untouched
/// by this — which is every ordinary parked ticket.
fn recording_failure_decline(root: &Path, ticket_id: &str) -> Option<String> {
    let journal = root.join(lisa_core::completion_journal::COMPLETION_JOURNAL_RELATIVE_PATH);
    // A journal Lisa cannot read is not evidence of anything; unblock keeps its
    // ordinary behavior rather than refusing on a file it failed to parse.
    let aggregates = lisa_core::completion_journal::load(&journal).ok()?;
    let aggregate = aggregates.get(ticket_id)?;
    (aggregate.action_required_generations() >= MAX_ACTION_REQUIRED_GENERATIONS).then(|| {
        format!(
            "{ticket_id} is waiting because Lisa could not record its finished work, not because \
             of the review. If that work is already saved in history, run: `lisa already-done \
             {ticket_id}`."
        )
    })
}

/// The lead sentence for a check that outlived its budget.
///
/// Formatted from the budget actually in force rather than spelled out, so the
/// sentence cannot drift from [`CHECK_TIMEOUT`].
fn decline_timed_out() -> String {
    format!(
        "That didn't work yet — it took longer than {} seconds.",
        CHECK_TIMEOUT.as_secs()
    )
}

fn decline_header(result: CheckResult) -> String {
    match result {
        CheckResult::Passed => unreachable!("passing checks do not decline"),
        CheckResult::Failed => DECLINE_FAILED.to_string(),
        CheckResult::Inconclusive => DECLINE_INCONCLUSIVE.to_string(),
        CheckResult::TimedOut => decline_timed_out(),
        CheckResult::ChangedFiles => DECLINE_CHANGED_FILES.to_string(),
    }
}

/// What the check exited with, or plainly why there is no code to report.
fn exit_code_line(run: &CheckRun) -> String {
    match (run.result, run.exit_code) {
        (CheckResult::TimedOut, _) => format!(
            "none — Lisa stopped it after {} seconds",
            CHECK_TIMEOUT.as_secs()
        ),
        (_, Some(code)) => code.to_string(),
        (_, None) => "none — the check was stopped".to_string(),
    }
}

/// The whole decline: Lisa's finding, then the check's work, then a way through.
///
/// Everything the check said is shown under its own label. Nothing it said ever
/// reaches the header line — the field failure this ticket exists for was a
/// project script's sentence relayed as Lisa's verdict.
fn decline_report(ticket_id: &str, run: &CheckRun) -> String {
    let mut report = decline_header(run.result);
    report.push_str("\n\n");
    // A recorded check may span lines; folding them to spaces before sanitizing
    // keeps the command readable instead of running its words together.
    report.push_str(&format!(
        "  what ran:  {}\n",
        sanitize_observation(&run.check.replace(['\n', '\r'], " "))
    ));
    report.push_str(&format!(
        "  ran in:    {}\n",
        sanitize_observation(&run.directory.display().to_string())
    ));
    report.push_str(&format!("  exit code: {}\n", exit_code_line(run)));

    let mut printed_anything = false;
    for (label, lines, dropped) in [
        ("stderr", &run.stderr, run.stderr_dropped),
        ("stdout", &run.stdout, run.stdout_dropped),
    ] {
        if lines.is_empty() {
            continue;
        }
        printed_anything = true;
        report.push_str(&format!("\n  the check wrote to {label}:\n"));
        for line in lines {
            report.push_str(&format!("    {line}\n"));
        }
        if dropped > 0 {
            report.push_str(&format!("    … ({dropped} more lines)\n"));
        }
    }
    if !printed_anything {
        report.push_str("\n  the check printed nothing.\n");
    }

    report.push_str(&format!(
        "\nIf you have done this and checked it yourself, run:\n  lisa unblock {ticket_id} --override-check"
    ));
    report
}

fn run_check(root: &Path, check: &str, timeout: Duration) -> Result<CheckRun, String> {
    let snapshot = ReadOnlySnapshot::new(root)
        .map_err(|error| format!("Could not prepare a safe check: {error}"))?;
    let before = fingerprint_tree(snapshot.path())
        .map_err(|error| format!("Could not prepare a safe check: {error}"))?;
    let scratch =
        tempfile::tempdir().map_err(|error| format!("Could not prepare a safe check: {error}"))?;
    let mut stdout =
        tempfile::tempfile().map_err(|error| format!("Could not prepare check output: {error}"))?;
    let mut stderr =
        tempfile::tempfile().map_err(|error| format!("Could not prepare check output: {error}"))?;

    // Cloned out rather than read back later: this is the directory the check is
    // about to be given, and it is what the report names.
    let directory = snapshot.path().to_path_buf();

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
            None if started.elapsed() >= timeout => {
                timed_out = true;
                terminate_check(&mut child);
                break child
                    .wait()
                    .map_err(|error| format!("Could not stop the check: {error}"))?;
            }
            None => thread::sleep(POLL_INTERVAL.min(timeout)),
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
        let after = fingerprint_tree(snapshot.path());
        if after.as_ref().ok() != Some(&before) {
            (CheckResult::ChangedFiles, status.code())
        } else {
            classify_exit(status.code())
        }
    };

    Ok(CheckRun {
        result,
        check: check.to_string(),
        directory,
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
fn observed_lines(bytes: &[u8]) -> (Vec<String>, usize) {
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

fn sanitize_observation(line: &str) -> String {
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

struct ReadOnlySnapshot {
    directory: TempDir,
}

impl ReadOnlySnapshot {
    fn new(root: &Path) -> io::Result<Self> {
        let directory = tempfile::tempdir()?;
        snapshot_project(root, directory.path())?;
        set_tree_read_only(directory.path(), true)?;
        Ok(Self { directory })
    }

    fn path(&self) -> &Path {
        self.directory.path()
    }
}

impl Drop for ReadOnlySnapshot {
    fn drop(&mut self) {
        let _ = set_tree_read_only(self.directory.path(), false);
    }
}

fn snapshot_project(root: &Path, destination: &Path) -> io::Result<()> {
    if let Some(paths) = git_visible_paths(root) {
        let canonical_root = root.canonicalize()?;
        for relative in paths {
            copy_visible_path(&canonical_root, &relative, destination)?;
        }
        return Ok(());
    }

    copy_small_tree(root, root, destination)
}

fn git_visible_paths(root: &Path) -> Option<Vec<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let mut paths = BTreeSet::new();
    for bytes in output.stdout.split(|byte| *byte == 0) {
        if bytes.is_empty() {
            continue;
        }
        let relative = PathBuf::from(os_string_from_bytes(bytes));
        if is_safe_relative(&relative) {
            paths.insert(relative);
        }
    }
    Some(paths.into_iter().collect())
}

#[cfg(unix)]
fn os_string_from_bytes(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}

#[cfg(not(unix))]
fn os_string_from_bytes(bytes: &[u8]) -> OsString {
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}

fn is_safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn copy_visible_path(root: &Path, relative: &Path, destination: &Path) -> io::Result<()> {
    if !is_safe_relative(relative) {
        return Ok(());
    }
    let source = root.join(relative);
    let target = destination.join(relative);
    copy_entry(root, &source, &target)
}

fn copy_entry(root: &Path, source: &Path, target: &Path) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() {
        let resolved = match source.canonicalize() {
            Ok(resolved) if resolved.starts_with(root) => resolved,
            _ => return Ok(()),
        };
        if fs::metadata(&resolved)?.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(resolved, target)?;
        }
        return Ok(());
    }
    if metadata.is_dir() {
        fs::create_dir_all(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_entry(root, &entry.path(), &target.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if metadata.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, target)?;
    }
    Ok(())
}

fn copy_small_tree(root: &Path, source: &Path, destination: &Path) -> io::Result<()> {
    let relative = source.strip_prefix(root).unwrap_or(Path::new(""));
    if should_skip(relative) {
        return Ok(());
    }
    let target = destination.join(relative);
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        let canonical_root = root.canonicalize()?;
        let resolved = match source.canonicalize() {
            Ok(resolved) if resolved.starts_with(&canonical_root) => resolved,
            _ => return Ok(()),
        };
        return copy_entry(&canonical_root, &resolved, &target);
    }
    if metadata.is_dir() {
        fs::create_dir_all(&target)?;
        for entry in fs::read_dir(source)? {
            copy_small_tree(root, &entry?.path(), destination)?;
        }
    } else if metadata.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, target)?;
    }
    Ok(())
}

fn should_skip(relative: &Path) -> bool {
    let mut components = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        });
    let first = components.next();
    let second = components.next();
    matches!(
        first.and_then(|value| value.to_str()),
        Some(".git" | "target" | "node_modules")
    ) || (first.and_then(|value| value.to_str()) == Some(".lisa")
        && second.and_then(|value| value.to_str()) == Some("attempts"))
}

fn set_tree_read_only(path: &Path, read_only: bool) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            set_tree_read_only(&entry?.path(), read_only)?;
        }
    }

    let mut permissions = metadata.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = permissions.mode();
        permissions.set_mode(if read_only {
            mode & !0o222
        } else {
            mode | 0o200
        });
    }
    #[cfg(not(unix))]
    permissions.set_readonly(read_only);
    fs::set_permissions(path, permissions)
}

fn fingerprint_tree(root: &Path) -> io::Result<Vec<u8>> {
    let mut entries = Vec::new();
    collect_entries(root, root, &mut entries)?;
    entries.sort();

    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    for relative in entries {
        let path = root.join(&relative);
        let metadata = fs::symlink_metadata(&path)?;
        hash.update(path_bytes(&relative));
        hash.update([0]);
        hash.update(if metadata.is_dir() { b"d" } else { b"f" });
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            hash.update(metadata.permissions().mode().to_le_bytes());
        }
        if metadata.is_file() {
            let mut file = File::open(path)?;
            loop {
                let read = file.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                hash.update(&buffer[..read]);
            }
        }
        hash.update([0xff]);
    }
    Ok(hash.finalize().to_vec())
}

fn collect_entries(root: &Path, path: &Path, entries: &mut Vec<PathBuf>) -> io::Result<()> {
    if path != root {
        entries.push(path.strip_prefix(root).unwrap().to_path_buf());
    }
    if fs::symlink_metadata(path)?.is_dir() {
        for entry in fs::read_dir(path)? {
            collect_entries(root, &entry?.path(), entries)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The field sentence from the 0.4.4 `tabular-recipes` run, verbatim.
    const FIELD_LINE: &str = "No build at dist/. Run: npm run build";

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

        let silent = run(root.path(), "exit 1");
        assert_eq!(silent.result, CheckResult::Failed);
        assert!(silent.stdout.is_empty() && silent.stderr.is_empty());
        assert!(decline_report("T-1", &silent).contains("the check printed nothing."));
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
        // The directory is gone by now (the snapshot is disposable), so neither
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

    #[test]
    fn timeout_is_bounded_and_kills_the_shell_group() {
        let root = tempfile::tempdir().unwrap();
        let started = Instant::now();

        let timed_out =
            run_check(root.path(), "sleep 5 & wait", Duration::from_millis(60)).unwrap();

        assert_eq!(timed_out.result, CheckResult::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(
            decline_header(CheckResult::TimedOut),
            "That didn't work yet — it took longer than 5 seconds."
        );
        assert!(decline_report("T-1", &timed_out).contains("exit code: none — Lisa stopped it"));
    }

    #[test]
    fn relative_write_never_reaches_live_project_and_cannot_pass() {
        let root = tempfile::tempdir().unwrap();
        let live_sentinel = root.path().join("must-not-exist");

        let result = run(root.path(), "touch must-not-exist");

        assert_ne!(result.result, CheckResult::Passed);
        assert!(!live_sentinel.exists());
    }

    #[test]
    fn mutation_inside_disposable_state_is_detected_even_after_chmod() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("fixture"), "before").unwrap();

        let changed = run(root.path(), "chmod u+w fixture && printf after > fixture");

        assert_eq!(changed.result, CheckResult::ChangedFiles);
        assert_eq!(
            fs::read_to_string(root.path().join("fixture")).unwrap(),
            "before"
        );
        assert_eq!(
            decline_header(CheckResult::ChangedFiles),
            "That didn't work yet — it tried to change project files."
        );
    }

    /// Every non-passing result has its own sentence, none of them is a
    /// judgement the check did not make, and each names the way through.
    #[test]
    fn every_decline_header_is_distinct_and_names_the_way_through() {
        let root = tempfile::tempdir().unwrap();
        let headers: Vec<String> = [
            CheckResult::Failed,
            CheckResult::Inconclusive,
            CheckResult::TimedOut,
            CheckResult::ChangedFiles,
        ]
        .into_iter()
        .map(decline_header)
        .collect();

        let mut unique = headers.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), headers.len(), "{headers:?}");

        for check in ["exit 1", "exit 2", "chmod u+w . 2>/dev/null; touch wrote"] {
            let report = decline_report("T-010-03", &run(root.path(), check));
            assert!(
                report.ends_with("lisa unblock T-010-03 --override-check"),
                "{report}"
            );
        }
    }

    /// The field failure, at the unit level: the script's sentence is shown as
    /// the script's, never as Lisa's finding.
    #[test]
    fn the_field_line_is_reported_not_asserted_as_lisas_verdict() {
        let root = tempfile::tempdir().unwrap();

        let inconclusive = run(
            root.path(),
            &format!("printf '{FIELD_LINE}\\n' >&2; exit 2"),
        );
        let report = decline_report("T-010-03", &inconclusive);
        let header = report.lines().next().unwrap();

        assert_eq!(inconclusive.result, CheckResult::Inconclusive);
        assert_eq!(header, DECLINE_INCONCLUSIVE);
        assert!(!header.contains("No build at dist/"), "{header}");
        assert!(!report.contains(&format!("That didn't work yet — {FIELD_LINE}")));
        assert!(report.contains("  the check wrote to stderr:\n    No build at dist/"));
        assert!(report.contains("exit code: 2"), "{report}");
        assert!(report.contains("  what ran:  printf"), "{report}");
        assert!(
            report.contains(&format!(
                "  ran in:    {}",
                inconclusive.directory.display()
            )),
            "{report}"
        );
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
