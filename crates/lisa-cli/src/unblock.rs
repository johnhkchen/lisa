//! Verify an optional parked-remedy check, then restore ordinary scheduling.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use lisa_core::parking::collect_parked_remedies;
use lisa_core::ticket;
use lisa_core::types::TicketStatus;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::config;

const CHECK_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_CAPTURE_BYTES: u64 = 8 * 1024;
const MAX_OBSERVATION_CHARS: usize = 240;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnblockOutcome {
    Reopened(String),
    Declined(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckResult {
    Passed,
    Failed(String),
    TimedOut,
    ChangedFiles,
}

/// Verify one parked ticket and restore `status: open` only when safe.
pub fn run_unblock(root: &Path, ticket_id: &str) -> Result<UnblockOutcome, String> {
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

    let mut remedies = collect_parked_remedies(std::iter::once(ticket), &work_dir);
    let Some(remedy) = remedies.pop() else {
        return Ok(UnblockOutcome::Declined(format!(
            "I couldn't find what {ticket_id} is waiting for."
        )));
    };

    if let Some(check) = remedy.check {
        match run_check(root, &check, CHECK_TIMEOUT)? {
            CheckResult::Passed => {}
            result => return Ok(UnblockOutcome::Declined(decline_message(result))),
        }
    }

    ticket::update_ticket_status(&ticket.file_path, TicketStatus::Open)
        .map_err(|error| format!("Could not let {ticket_id} run again: {error}"))?;
    Ok(UnblockOutcome::Reopened(format!(
        "{ticket_id} can run again."
    )))
}

fn decline_message(result: CheckResult) -> String {
    match result {
        CheckResult::Passed => unreachable!("passing checks do not decline"),
        CheckResult::Failed(observation) => {
            format!("That didn't work yet — {observation}")
        }
        CheckResult::TimedOut => {
            "That didn't work yet — it took longer than 5 seconds.".to_string()
        }
        CheckResult::ChangedFiles => {
            "That didn't work yet — it tried to change project files.".to_string()
        }
    }
}

fn run_check(root: &Path, check: &str, timeout: Duration) -> Result<CheckResult, String> {
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

    let mut command = Command::new("/bin/sh");
    command
        .arg("-c")
        .arg(check)
        .current_dir(snapshot.path())
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

    if timed_out {
        return Ok(CheckResult::TimedOut);
    }

    let after = fingerprint_tree(snapshot.path());
    if after.as_ref().ok() != Some(&before) {
        return Ok(CheckResult::ChangedFiles);
    }

    let stdout = read_capture(&mut stdout)
        .map_err(|error| format!("Could not read check output: {error}"))?;
    let stderr = read_capture(&mut stderr)
        .map_err(|error| format!("Could not read check output: {error}"))?;
    if status.success() {
        Ok(CheckResult::Passed)
    } else {
        Ok(CheckResult::Failed(
            observed_line(&stderr, &stdout).unwrap_or_else(|| "it still isn't ready.".to_string()),
        ))
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

fn observed_line(stderr: &[u8], stdout: &[u8]) -> Option<String> {
    for bytes in [stderr, stdout] {
        let decoded = String::from_utf8_lossy(bytes);
        for line in decoded.lines() {
            let line = sanitize_observation(line);
            if !line.is_empty() {
                return Some(line);
            }
        }
    }
    None
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

    #[test]
    fn passing_and_failing_checks_report_one_plain_observation() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("ready"), "yes").unwrap();

        assert_eq!(
            run_check(root.path(), "test -f ready", Duration::from_secs(1)).unwrap(),
            CheckResult::Passed
        );
        assert_eq!(
            run_check(
                root.path(),
                "printf 'the key link still returns 404\\nextra detail\\n' >&2; exit 1",
                Duration::from_secs(1),
            )
            .unwrap(),
            CheckResult::Failed("the key link still returns 404".to_string())
        );
        assert_eq!(
            run_check(root.path(), "exit 1", Duration::from_secs(1)).unwrap(),
            CheckResult::Failed("it still isn't ready.".to_string())
        );
    }

    #[test]
    fn timeout_is_bounded_and_kills_the_shell_group() {
        let root = tempfile::tempdir().unwrap();
        let started = Instant::now();

        assert_eq!(
            run_check(root.path(), "sleep 5 & wait", Duration::from_millis(60),).unwrap(),
            CheckResult::TimedOut
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(
            decline_message(CheckResult::TimedOut),
            "That didn't work yet — it took longer than 5 seconds."
        );
    }

    #[test]
    fn relative_write_never_reaches_live_project_and_cannot_pass() {
        let root = tempfile::tempdir().unwrap();
        let live_sentinel = root.path().join("must-not-exist");

        let result =
            run_check(root.path(), "touch must-not-exist", Duration::from_secs(1)).unwrap();

        assert_ne!(result, CheckResult::Passed);
        assert!(!live_sentinel.exists());
    }

    #[test]
    fn mutation_inside_disposable_state_is_detected_even_after_chmod() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("fixture"), "before").unwrap();

        assert_eq!(
            run_check(
                root.path(),
                "chmod u+w fixture && printf after > fixture",
                Duration::from_secs(1),
            )
            .unwrap(),
            CheckResult::ChangedFiles
        );
        assert_eq!(
            fs::read_to_string(root.path().join("fixture")).unwrap(),
            "before"
        );
        assert_eq!(
            decline_message(CheckResult::ChangedFiles),
            "That didn't work yet — it tried to change project files."
        );
    }

    #[test]
    fn observation_prefers_stderr_removes_controls_and_caps_length() {
        let long = "x".repeat(MAX_OBSERVATION_CHARS + 20);
        let stderr = format!("\x1b[31m  observed\t{long}  \n");

        let observation = observed_line(stderr.as_bytes(), b"stdout fallback").unwrap();

        assert!(observation.starts_with("observed "));
        assert_eq!(observation.chars().count(), MAX_OBSERVATION_CHARS);
        assert!(!observation.contains('\n'));
        assert!(!observation.contains('\t'));
    }
}
