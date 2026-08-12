//! `lisa release-seats` — the seats a run left behind when it died.
//!
//! ## The problem this answers
//!
//! A run that dies without shutting down cannot withdraw its pane lease
//! markers: the process that would have done it is the one that died. The
//! markers stay in `.lisa/signals/`, `lisa status` keeps reporting them in
//! `attempts[]`, and a consumer reading the documented contract keeps seeing
//! seats that are working. Nothing clears them, and `lisa clean` deliberately
//! will not — its one rule is about a *finished ticket's* litter, and a lease
//! for a ticket in `implement` is the opposite of that.
//!
//! ## Telling a dead run from a slow one
//!
//! On disk the two are identical, so the scheduler says which it is. Every poll
//! tick the plugin rewrites [`lisa_core::liveness::SCHEDULER_ALIVE_FILE`] with
//! the moment it happened, and that stamp stops the instant the process stops —
//! crash, kill, machine swap, closed terminal. Two facts have to agree before
//! Lisa will call a seat abandoned:
//!
//! 1. **No scheduler has stamped recently.** Nothing but a running scheduler
//!    writes that file.
//! 2. **`.lisa/signals/` has gone quiet.** A live run's panes write heartbeats
//!    into it and a live plugin consumes them, so both the files and the
//!    directory's own timestamp move while anything is happening.
//!
//! Either one alone would be wrong. A run detached into the background and
//! blocked on a question writes no signals for hours, and (1) keeps its seats;
//! a scheduler that cannot write its stamp still has panes signalling, and (2)
//! keeps them. Every other outcome is *unclear*, and unclear keeps the seat —
//! clearing a live one would put a second agent on a ticket somebody is
//! working, which is the expensive mistake.
//!
//! ## Why an explicit command
//!
//! The recovery already existed: run `lisa loop` again and it reassigns the
//! tickets and overwrites the leases. What did not exist was any way to *find
//! out*, and the only recovery was the exact thing the stale state was there to
//! prevent. So the diagnosis goes where an operator already looks — `lisa
//! status` — and the deletion is its own deliberate command, in the consent
//! shape [`crate::clean`] established: a bare run prints the list and the
//! evidence, and changes nothing.

use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lisa_core::liveness::{SchedulerAlive, SCHEDULER_ALIVE_FILE};

use crate::clean::{display_relative, plural, reachability, Reachability};
use crate::config;

/// Where the plugin publishes pane state, relative to the project root.
const SIGNAL_DIR: &str = ".lisa/signals";

/// The shortest silence Lisa will accept as evidence, whatever the project
/// configures.
///
/// A project may set `wind_down_secs` very low — this repository's own test
/// fixtures use `5` — and a window under a handful of poll intervals would call
/// a healthy scheduler dead the first time a tick ran late.
const MIN_STAMPED_WINDOW_SECS: u64 = 60;

/// The shortest silence Lisa will accept when there is no stamp at all.
///
/// Without a stamp the whole verdict rests on quiet, which a live run can
/// produce honestly, so the bar is much higher than the stamped one.
const MIN_UNSTAMPED_WINDOW_SECS: u64 = 900;

/// The session budget assumed when the project disables its own.
const ASSUMED_SESSION_BUDGET_SECS: u64 = 3600;

/// What Lisa can say about the run that placed the seats on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunLiveness {
    /// A scheduler stamped itself inside the window, or something in
    /// `.lisa/signals/` moved inside it. Either way a run is here.
    Running,
    /// Both tests agree: nothing has stamped and nothing has signalled.
    Ended,
    /// Neither running nor provably ended — a clock that disagrees, a signal
    /// directory Lisa cannot read. Seats stay.
    Unclear,
}

/// The verdict plus the one sentence that justifies it, in the operator's
/// words.
///
/// The sentence is built once and used by every renderer — the `status` prose,
/// the `--json` document, and this command's plan — so the three cannot end up
/// telling the operator different stories about the same evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunReport {
    pub(crate) liveness: RunLiveness,
    pub(crate) evidence: String,
}

impl RunReport {
    fn running(evidence: String) -> Self {
        Self {
            liveness: RunLiveness::Running,
            evidence,
        }
    }

    fn unclear(evidence: String) -> Self {
        Self {
            liveness: RunLiveness::Unclear,
            evidence,
        }
    }

    /// True when Lisa is prepared to say the seats on disk are held by nobody.
    pub(crate) fn seats_are_abandoned(&self) -> bool {
        self.liveness == RunLiveness::Ended
    }
}

/// Seconds since the epoch, or `None` when the clock is before it.
pub(crate) fn now_secs() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|since| since.as_secs())
}

/// A duration an operator can read without converting anything.
pub(crate) fn humanize(secs: u64) -> String {
    match secs {
        0..=90 => format!("{secs}s"),
        91..=5399 => format!("{}m", secs.div_ceil(60)),
        5400..=172_799 => format!("{}h", secs.div_ceil(3600)),
        _ => format!("{}d", secs.div_ceil(86_400)),
    }
}

/// The stamp on disk, or `None` when there is none Lisa can read.
///
/// An unparsable stamp reads as no stamp. That is the conservative direction:
/// it widens the window Lisa demands rather than narrowing it.
fn read_stamp(root: &Path) -> Option<SchedulerAlive> {
    let body = std::fs::read_to_string(root.join(SCHEDULER_ALIVE_FILE)).ok()?;
    serde_json::from_str(&body).ok()
}

/// How long ago anything in `.lisa/signals/` last changed.
///
/// The directory's own timestamp counts alongside its entries: a plugin
/// consuming a heartbeat *removes* a file, which leaves no fresh file behind
/// but does move the directory. `None` means the directory is not there to
/// read, which is not evidence of anything.
fn signals_quiet_for(root: &Path, now: u64) -> Option<u64> {
    let dir = root.join(SIGNAL_DIR);
    let mut newest = modified_secs(&dir)?;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(stamped) = entry.metadata().ok().and_then(|meta| {
                meta.modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|since| since.as_secs())
            }) {
                newest = newest.max(stamped);
            }
        }
    }
    // Saturating rather than checked: a timestamp in the future reads as zero
    // seconds ago, which keeps the seat, which is the safe direction.
    Some(now.saturating_sub(newest))
}

fn modified_secs(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|since| since.as_secs())
}

/// Decide what the run is, and say why in one sentence.
pub(crate) fn assess_run(root: &Path, resolved: &config::ResolvedConfig) -> RunReport {
    let Some(now) = now_secs() else {
        return RunReport::unclear(
            "This machine's clock is set before 1970, so Lisa cannot age anything.".to_string(),
        );
    };
    assess_run_at(root, resolved, now)
}

/// The whole decision, as a function of the tree and one clock reading.
///
/// The clock is a parameter so a test can state the passage of a day without
/// rewriting timestamps on disk — backdating files tests the test harness, and
/// this is the judgement worth testing.
pub(crate) fn assess_run_at(root: &Path, resolved: &config::ResolvedConfig, now: u64) -> RunReport {
    let stamp = read_stamp(root);
    let (window, window_reason) = match &stamp {
        Some(stamp) => (
            resolved
                .wind_down_secs
                .max(stamp.poll_interval_secs.saturating_mul(6))
                .max(MIN_STAMPED_WINDOW_SECS),
            format!(
                "Lisa's scheduler writes {SCHEDULER_ALIVE_FILE} every {}s while it runs",
                stamp.poll_interval_secs
            ),
        ),
        None => {
            let budget = if resolved.session_timeout_secs == 0 {
                ASSUMED_SESSION_BUDGET_SECS
            } else {
                resolved.session_timeout_secs
            };
            (
                budget.max(MIN_UNSTAMPED_WINDOW_SECS),
                format!(
                    "no scheduler has ever written {SCHEDULER_ALIVE_FILE} here, so Lisa waits out \
                     a whole session budget instead"
                ),
            )
        }
    };

    let stamp_age = match &stamp {
        Some(stamp) => match stamp.age_secs(now) {
            Some(age) if age <= window => {
                return RunReport::running(format!(
                    "Lisa's scheduler said it was running {} ago.",
                    humanize(age)
                ))
            }
            Some(age) => Some(age),
            None => {
                return RunReport::unclear(format!(
                    "The last scheduler stamp in {SCHEDULER_ALIVE_FILE} is dated ahead of this \
                     machine's clock, so Lisa cannot tell how old it is."
                ))
            }
        },
        None => None,
    };

    let Some(quiet) = signals_quiet_for(root, now) else {
        return RunReport::unclear(format!(
            "There is no {SIGNAL_DIR}/ to read, so there is nothing to say about it."
        ));
    };
    if quiet <= window {
        return RunReport::running(format!(
            "Something wrote in {SIGNAL_DIR}/ {} ago, which only a running pane or plugin does.",
            humanize(quiet)
        ));
    }

    let seen = match stamp_age {
        Some(age) => format!(
            "Lisa's scheduler last said it was running {} ago",
            humanize(age)
        ),
        None => "Lisa has no record of a scheduler running here".to_string(),
    };
    RunReport {
        liveness: RunLiveness::Ended,
        evidence: format!(
            "{seen}, and nothing has changed in {SIGNAL_DIR}/ for {quiet_for}. Lisa waits \
             {waited} before believing that, because {window_reason}.",
            quiet_for = humanize(quiet),
            waited = humanize(window),
        ),
    }
}

// ---------------------------------------------------------------------------
// The command
// ---------------------------------------------------------------------------

/// One line of the release plan.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SeatAction {
    remove: bool,
    path: PathBuf,
    reason: String,
}

impl SeatAction {
    fn line(&self, root: &Path) -> String {
        let path = display_relative(root, &self.path);
        if self.remove {
            format!("  release  {path} ({})", self.reason)
        } else {
            format!("  skip     {path} ({})", self.reason)
        }
    }
}

/// Every pane marker in `.lisa/signals/`, sorted, with the seat it names.
///
/// Only `pane-*` entries: nothing else in that directory belongs to a pane, and
/// a command that deletes should not be guessing at names it does not
/// recognise.
fn pane_marker_paths(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root.join(SIGNAL_DIR)) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("pane-"))
        })
        .collect();
    paths.sort();
    paths
}

/// What a marker names, for the plan line — the ticket when it is a lease, the
/// kind of marker otherwise.
fn marker_reason(path: &Path) -> String {
    let kind = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("marker");
    if kind == "lease" {
        if let Some(lease) = std::fs::read_to_string(path)
            .ok()
            .and_then(|body| serde_json::from_str::<lisa_core::types::AttemptLease>(&body).ok())
        {
            return format!(
                "the seat Lisa placed {} attempt {} in",
                lease.ticket_id, lease.attempt_id
            );
        }
        return "a seat marker Lisa cannot read".to_string();
    }
    format!("a {kind} marker from the same pane")
}

/// Everything `lisa release-seats` would do, computed before anything is
/// touched.
fn plan_release(root: &Path, report: &RunReport) -> Vec<SeatAction> {
    let markers = pane_marker_paths(root);
    if !report.seats_are_abandoned() {
        return markers
            .into_iter()
            .map(|path| SeatAction {
                remove: false,
                path,
                reason: "a run may still be holding it".to_string(),
            })
            .collect();
    }

    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    markers
        .into_iter()
        .map(|path| match reachability(root, &canonical_root, &path) {
            Reachability::Safe => SeatAction {
                remove: true,
                reason: marker_reason(&path),
                path,
            },
            Reachability::Refused(why) => SeatAction {
                remove: false,
                path,
                reason: format!("preserved: {why}"),
            },
        })
        .collect()
}

fn write_line(out: &mut impl Write, args: fmt::Arguments<'_>) -> Result<(), String> {
    writeln!(out, "{args}")
        .map_err(|error| format!("Failed to write release-seats output: {error}"))
}

/// Execute the command, writing operator-facing output to stdout.
pub fn run_release_seats(root: &Path, release: bool) -> Result<(), String> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let Some(now) = now_secs() else {
        return Err(
            "This machine's clock is set before 1970, so Lisa cannot tell how old a seat is."
                .to_string(),
        );
    };
    run_release_seats_with_writer(root, release, &mut out, now)
}

/// Print the plan, and carry it out only when `release` is true.
///
/// The plan is complete before the first deletion and the preview run returns
/// before the loop that deletes, so a released marker that was never printed is
/// not reachable from here — the same shape `lisa clean` uses, for the same
/// reason.
fn run_release_seats_with_writer(
    root: &Path,
    release: bool,
    out: &mut impl Write,
    now: u64,
) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root.display()));
    }

    let resolved = match config::load_config(root) {
        Ok(validation) => config::resolve_config(&validation.config, None, None),
        Err(_) => config::ResolvedConfig::default(),
    };
    let report = assess_run_at(root, &resolved, now);
    let plan = plan_release(root, &report);
    let removals: Vec<&SeatAction> = plan.iter().filter(|action| action.remove).collect();

    if plan.is_empty() {
        write_line(
            out,
            format_args!("No seats are held here. Nothing to release."),
        )?;
        return Ok(());
    }

    if removals.is_empty() {
        write_line(
            out,
            format_args!(
                "Nothing to release: {}. {}",
                match report.liveness {
                    RunLiveness::Running => "a run is holding these seats",
                    _ => "Lisa cannot say these seats are free",
                },
                report.evidence
            ),
        )?;
        write_line(out, format_args!(""))?;
        for action in &plan {
            write_line(out, format_args!("{}", action.line(root)))?;
        }
        return Ok(());
    }

    write_line(
        out,
        format_args!(
            "{} to release, held by a run that is no longer running. Lisa wrote all of it.",
            plural(removals.len(), "marker", "markers")
        ),
    )?;
    write_line(out, format_args!(""))?;
    write_line(out, format_args!("Evidence: {}", report.evidence))?;
    write_line(out, format_args!(""))?;
    write_line(out, format_args!("Planned actions:"))?;
    for action in &plan {
        write_line(out, format_args!("{}", action.line(root)))?;
    }
    write_line(out, format_args!(""))?;
    write_line(
        out,
        format_args!(
            "Never a candidate: a seat any run may still be holding, your board, your work, and \
             anything Lisa did not write."
        ),
    )?;
    write_line(out, format_args!(""))?;

    if !release {
        write_line(
            out,
            format_args!(
                "Dry run complete. No changes made. Add --release to carry this list out."
            ),
        )?;
        return Ok(());
    }

    let mut released = 0usize;
    let mut failures = Vec::new();
    for action in &removals {
        match std::fs::remove_file(&action.path) {
            Ok(()) => released += 1,
            Err(error) => failures.push(format!(
                "  {} ({error})",
                display_relative(root, &action.path)
            )),
        }
    }

    write_line(
        out,
        format_args!(
            "Released {}. The next run places its own seats; nothing else changed.",
            plural(released, "marker", "markers")
        ),
    )?;

    if !failures.is_empty() {
        write_line(out, format_args!(""))?;
        write_line(out, format_args!("Could not release:"))?;
        for failure in &failures {
            write_line(out, format_args!("{failure}"))?;
        }
        return Err(format!(
            "{} of {} planned releases did not happen; every one is listed above",
            failures.len(),
            removals.len()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A project holding two seats, both placed "now" in real time.
    ///
    /// Age is expressed by reading the clock later rather than by rewriting
    /// timestamps: [`later`] is how long after the seats were placed the
    /// question is being asked.
    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join(SIGNAL_DIR)).unwrap();
        fs::write(
            dir.path().join(SIGNAL_DIR).join("pane-0.lease"),
            r#"{"ticket_id":"T-008-08","attempt_id":1}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join(SIGNAL_DIR).join("pane-1.lease"),
            r#"{"ticket_id":"T-009-05","attempt_id":1}"#,
        )
        .unwrap();
        dir
    }

    /// The clock, `secs` after the fixture was written.
    fn later(secs: u64) -> u64 {
        now_secs().unwrap() + secs
    }

    /// Write a stamp dated `age` seconds before the moment `now` names.
    fn stamp(root: &Path, now: u64, age: u64) {
        let stamp = SchedulerAlive::new(now - age, 5);
        fs::create_dir_all(root.join(".lisa")).unwrap();
        fs::write(
            root.join(SCHEDULER_ALIVE_FILE),
            serde_json::to_vec(&stamp).unwrap(),
        )
        .unwrap();
    }

    fn assess(root: &Path, now: u64) -> RunReport {
        assess_run_at(root, &config::ResolvedConfig::default(), now)
    }

    /// The whole point: a fresh stamp means a run is here, whatever the seats
    /// look like and however long they have sat.
    #[test]
    fn a_scheduler_that_stamped_seconds_ago_holds_every_seat() {
        let dir = project();
        let now = later(86_400);
        stamp(dir.path(), now, 4);

        let report = assess(dir.path(), now);
        assert_eq!(report.liveness, RunLiveness::Running);
        assert!(report.evidence.contains("said it was running"));
        assert!(!report.seats_are_abandoned());
    }

    /// A run detached into the background and blocked on a question writes no
    /// signals for hours. Its stamp is the only thing keeping its seats, and it
    /// has to be enough.
    #[test]
    fn a_silent_but_stamping_run_keeps_its_seats() {
        let dir = project();
        let now = later(6 * 3600);
        stamp(dir.path(), now, 10);

        assert_eq!(assess(dir.path(), now).liveness, RunLiveness::Running);
    }

    /// The other half: a scheduler whose stamp is stale but whose panes are
    /// still signalling is still a run.
    #[test]
    fn a_stale_stamp_alone_is_not_enough_while_signals_keep_moving() {
        let dir = project();
        let now = later(3);
        stamp(dir.path(), now, 86_400);

        let report = assess(dir.path(), now);
        assert_eq!(report.liveness, RunLiveness::Running);
        assert!(report.evidence.contains(".lisa/signals/"));
    }

    #[test]
    fn both_facts_agreeing_is_what_makes_a_seat_abandoned() {
        let dir = project();
        let now = later(86_400);
        stamp(dir.path(), now, 86_400);

        let report = assess(dir.path(), now);
        assert_eq!(report.liveness, RunLiveness::Ended);
        assert!(report.seats_are_abandoned());
        assert!(report.evidence.contains("last said it was running 24h ago"));
        assert!(report.evidence.contains("nothing has changed"));
    }

    /// A project that has never run a stamping Lisa still recovers, but only
    /// after a silence long enough that no live session could produce it.
    #[test]
    fn with_no_stamp_at_all_lisa_waits_out_a_whole_session_budget() {
        let dir = project();
        assert_eq!(
            assess(dir.path(), later(20 * 60)).liveness,
            RunLiveness::Running,
            "20 minutes of quiet is inside the default hour-long budget"
        );

        let report = assess(dir.path(), later(2 * 3600));
        assert_eq!(report.liveness, RunLiveness::Ended);
        assert!(report.evidence.contains("no record of a scheduler"));
    }

    /// A stamp dated ahead of this machine's clock is not evidence of anything,
    /// and doubt keeps the seat.
    #[test]
    fn a_stamp_from_the_future_keeps_the_seats_rather_than_guessing() {
        let dir = project();
        let now = later(86_400);
        let ahead = SchedulerAlive::new(now + 3600, 5);
        fs::create_dir_all(dir.path().join(".lisa")).unwrap();
        fs::write(
            dir.path().join(SCHEDULER_ALIVE_FILE),
            serde_json::to_vec(&ahead).unwrap(),
        )
        .unwrap();

        let report = assess(dir.path(), now);
        assert_eq!(report.liveness, RunLiveness::Unclear);
        assert!(!report.seats_are_abandoned());
    }

    /// A very short configured wind-down cannot shrink the window below the
    /// floor — a tick that runs late must never read as a dead scheduler.
    #[test]
    fn a_tiny_configured_wind_down_cannot_shrink_the_window_below_the_floor() {
        let dir = project();
        let now = later(45);
        stamp(dir.path(), now, 45);
        let resolved = config::ResolvedConfig {
            wind_down_secs: 5,
            ..config::ResolvedConfig::default()
        };

        assert_eq!(
            assess_run_at(dir.path(), &resolved, now).liveness,
            RunLiveness::Running,
            "45s is inside the {MIN_STAMPED_WINDOW_SECS}s floor"
        );
    }

    #[test]
    fn a_dry_run_prints_the_evidence_and_removes_nothing() {
        let dir = project();
        let now = later(86_400);
        stamp(dir.path(), now, 86_400);

        let mut out = Vec::new();
        run_release_seats_with_writer(dir.path(), false, &mut out, now).unwrap();
        let printed = String::from_utf8(out).unwrap();

        assert!(printed.contains("2 markers to release"));
        assert!(printed.contains("Evidence:"));
        assert!(printed.contains("release  .lisa/signals/pane-0.lease"));
        assert!(printed.contains("the seat Lisa placed T-008-08 attempt 1 in"));
        assert!(printed.contains("Add --release to carry this list out."));
        assert!(dir.path().join(SIGNAL_DIR).join("pane-0.lease").exists());
    }

    #[test]
    fn releasing_removes_exactly_what_the_plan_named() {
        let dir = project();
        fs::write(dir.path().join(SIGNAL_DIR).join("pane-1.heartbeat"), "{}").unwrap();
        // An operator's own file, which is not a pane marker and is not Lisa's.
        fs::write(dir.path().join(SIGNAL_DIR).join("notes.txt"), "mine").unwrap();
        let now = later(86_400);
        stamp(dir.path(), now, 86_400);

        let mut out = Vec::new();
        run_release_seats_with_writer(dir.path(), true, &mut out, now).unwrap();
        let printed = String::from_utf8(out).unwrap();

        assert!(printed.contains("Released 3 markers"));
        assert!(!dir.path().join(SIGNAL_DIR).join("pane-0.lease").exists());
        assert!(!dir.path().join(SIGNAL_DIR).join("pane-1.lease").exists());
        assert!(!dir
            .path()
            .join(SIGNAL_DIR)
            .join("pane-1.heartbeat")
            .exists());
        assert!(
            dir.path().join(SIGNAL_DIR).join("notes.txt").exists(),
            "a file that is not a pane marker is never a candidate"
        );
        assert!(
            dir.path().join(SCHEDULER_ALIVE_FILE).exists(),
            "the stamp is the record of when the run was last seen, not litter"
        );
    }

    /// The command an operator runs while a run is live has to say no, and say
    /// which seats it declined to touch.
    #[test]
    fn a_live_run_makes_the_command_refuse_and_list_what_it_kept() {
        let dir = project();
        let now = later(10);
        stamp(dir.path(), now, 2);

        let mut out = Vec::new();
        run_release_seats_with_writer(dir.path(), true, &mut out, now).unwrap();
        let printed = String::from_utf8(out).unwrap();

        assert!(printed.contains("Nothing to release"));
        assert!(printed.contains("a run is holding these seats"));
        assert!(printed.contains("skip     .lisa/signals/pane-0.lease"));
        assert!(dir.path().join(SIGNAL_DIR).join("pane-0.lease").exists());
    }

    #[test]
    fn a_project_with_no_seats_says_so_without_a_plan() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();

        let mut out = Vec::new();
        run_release_seats_with_writer(dir.path(), true, &mut out, later(0)).unwrap();
        assert!(String::from_utf8(out)
            .unwrap()
            .contains("No seats are held here."));
    }

    #[test]
    fn durations_read_the_way_an_operator_would_say_them() {
        assert_eq!(humanize(4), "4s");
        assert_eq!(humanize(90), "90s");
        assert_eq!(humanize(600), "10m");
        assert_eq!(humanize(3600), "60m");
        assert_eq!(humanize(18 * 3600), "18h");
        assert_eq!(humanize(3 * 86_400), "3d");
    }
}
