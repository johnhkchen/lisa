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
//! ## What the shared stamp cannot say, and what replaces it
//!
//! `.lisa/scheduler.alive` is one file with as many writers as there are
//! schedulers. Three of them keep it fresh exactly as convincingly as one, so
//! "a scheduler stamped 4s ago" survived seventeen hours of a board being held
//! by two runs nobody could see, and both recoveries Lisa offers refused on
//! that sentence. The refusals were correct about the file and wrong about the
//! world (S-063-01).
//!
//! So the verdict here is built from evidence that can be attributed:
//!
//! - **Who is here** comes from [`lisa_core::schedulers`] — one file per
//!   scheduler, each naming its session and its Zellij server. A count is a
//!   count, and every refusal below can name the run it is protecting and the
//!   command that ends it.
//! - **Whether anything is being worked** comes from `.lisa/signals/`, which
//!   only a live pane or a live plugin moves. That is the difference between
//!   [`RunLiveness::Working`] and [`RunLiveness::Stamping`]: a scheduler that
//!   stamps proves a process exists, not that a ticket is progressing.
//! - **The shared stamp remains the fallback** for a board whose scheduler
//!   predates the registry, and nothing more than that.
//!
//! Both states still keep the seats — a stamping scheduler can dispatch at any
//! moment — but they are no longer the same sentence, and the stamping one now
//! ends at a command instead of at a wall.
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
    /// Something moved in `.lisa/signals/` inside the window: a pane wrote, or
    /// a plugin consumed. A ticket is being worked.
    Working,
    /// A scheduler is stamping, but nothing has moved in `.lisa/signals/` for
    /// longer than the window. A process exists; no work is visible. That is a
    /// run waiting on a question — or a scheduler nobody meant to leave behind.
    Stamping,
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
    /// The schedulers stamping this board right now, named. Empty when the
    /// registry has nothing live in it — including every board whose scheduler
    /// predates the registry, where the shared stamp is all there is.
    pub(crate) schedulers: Vec<lisa_core::schedulers::SchedulerRecord>,
    /// Whether the shared [`SCHEDULER_ALIVE_FILE`] stamp is fresh.
    ///
    /// The registry above answers *which* schedulers, and answers nothing at
    /// all on a board whose scheduler predates it. This bool is the other half
    /// of "is one here", and it is kept separately because [`RunLiveness`]
    /// folds the two together: `Working` is decided by the signal directory
    /// moving, which a freshly created `.lisa/signals/` does on a board nobody
    /// has ever run.
    pub(crate) stamped: bool,
}

impl RunReport {
    fn working(
        evidence: String,
        schedulers: Vec<lisa_core::schedulers::SchedulerRecord>,
        stamped: bool,
    ) -> Self {
        Self {
            liveness: RunLiveness::Working,
            evidence,
            schedulers,
            stamped,
        }
    }

    fn stamping(
        evidence: String,
        schedulers: Vec<lisa_core::schedulers::SchedulerRecord>,
        stamped: bool,
    ) -> Self {
        Self {
            liveness: RunLiveness::Stamping,
            evidence,
            schedulers,
            stamped,
        }
    }

    fn unclear(evidence: String) -> Self {
        Self {
            liveness: RunLiveness::Unclear,
            evidence,
            schedulers: Vec::new(),
            stamped: false,
        }
    }

    /// True when Lisa is prepared to say the seats on disk are held by nobody.
    pub(crate) fn seats_are_abandoned(&self) -> bool {
        self.liveness == RunLiveness::Ended
    }

    /// True while anything is here to hold the seats — whether it is working or
    /// only stamping.
    pub(crate) fn holds_the_board(&self) -> bool {
        matches!(self.liveness, RunLiveness::Working | RunLiveness::Stamping)
    }

    /// Whether Lisa has evidence of a *scheduler* on this board, as opposed to
    /// evidence that something once touched the signal directory.
    ///
    /// Two independent sources, because either alone has a board it cannot
    /// speak for: the registry names schedulers and says nothing about a build
    /// that predates it, and the shared stamp survives that build but cannot
    /// count past one.
    fn a_scheduler_is_here(&self) -> bool {
        !self.schedulers.is_empty() || self.stamped
    }

    /// Where this run is, for a program that has to decide something.
    ///
    /// The prose above tells an operator; this tells `rail up` whether a loop
    /// already has a strip beside it, and a phone over SSH which session to
    /// attach to. Both had to infer it from panes, or guess, because the
    /// envelope carried the board and not the run.
    ///
    /// Three states rather than a bool, because "no run" and "a run Lisa cannot
    /// place" are different answers and silence was both:
    ///
    /// - **`working`** — a scheduler is here and something moved in
    ///   `.lisa/signals/` inside the window.
    /// - **`idle`** — a scheduler is here and nothing is moving. A run that has
    ///   finished every ticket is exactly this: still resident, still holding
    ///   the board, not working. Reporting it as absent is what started a second
    ///   scheduler here on 2026-08-12.
    /// - **`none`** — Lisa has no evidence of a scheduler. Signals moving with
    ///   nobody stamping is *not* a run: a `.lisa/signals/` that `lisa init`
    ///   created a moment ago moves exactly that way.
    /// - **`unknown`** — Lisa could not look. A clock before the epoch, a signal
    ///   directory it cannot read.
    ///
    /// The session name is the whole point of the payload: it is what
    /// `zellij attach` takes, and it means the same thing on the other end of
    /// `gh codespace ssh`. Nothing here is a pid or a socket path.
    pub(crate) fn location(&self) -> crate::json_output::RunLocationView {
        let mut sessions: Vec<String> = self
            .schedulers
            .iter()
            .filter_map(|record| record.session_name.clone())
            .collect();
        sessions.sort();
        sessions.dedup();

        let state = match self.liveness {
            RunLiveness::Unclear => "unknown",
            _ if !self.a_scheduler_is_here() => "none",
            RunLiveness::Working => "working",
            RunLiveness::Stamping | RunLiveness::Ended => "idle",
        };

        // One session is a place to go; two is a contested board, where naming
        // either one as *the* run would send a caller to an arbitrary half of
        // the problem. `sessions` still lists both, and `schedulers` has the
        // rest of each one.
        let session = match sessions.as_slice() {
            [only] => Some(only.clone()),
            _ => None,
        };

        crate::json_output::RunLocationView {
            state,
            attach_command: session
                .as_ref()
                .map(|session| format!("zellij attach {session}")),
            session,
            sessions,
        }
    }

    /// One sentence naming the schedulers on this board and how to stop them,
    /// for a refusal that would otherwise be a dead end.
    ///
    /// `None` when the registry named nobody: there is nothing honest to add,
    /// and inventing a command an operator cannot run is worse than silence.
    pub(crate) fn how_to_stop(&self) -> Option<String> {
        if self.schedulers.is_empty() {
            return None;
        }
        let named: Vec<String> = self
            .schedulers
            .iter()
            .map(|record| match record.stop_command() {
                // Both commands, because the first one has a way of being
                // unrunnable at exactly the moment it is needed: a session that
                // ended without its scheduler noticing cannot be killed, and an
                // operator standing at `(the session does not exist)` needs the
                // next sentence to be here rather than in a story (S-070-01).
                Some(command) => format!(
                    "{} — stop it with `{command}`, or with `lisa schedulers --stop {}` if that \
                     session is already gone",
                    record.label(),
                    record.scheduler_id
                ),
                None => format!(
                    "{} — Lisa was never told its session name; `zellij list-sessions` has it, and \
                     `lisa schedulers --stop {}` ends it either way",
                    record.label(),
                    record.scheduler_id
                ),
            })
            .collect();
        Some(format!(
            "{} on this board: {}. `lisa schedulers` lists them.",
            match named.len() {
                1 => "One scheduler is running".to_string(),
                count => format!("{count} schedulers are running"),
            },
            named.join("; ")
        ))
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
    assess_run_with(root, resolved, now, &crate::presence::Machine::read(root))
}

/// The same decision, against a stated machine.
///
/// The machine is a parameter for the same reason the clock is: "the process
/// this record names is gone" is the judgement this file exists to make, and a
/// test that has to arrange a real dead Zellij to state it is a test of the
/// harness.
pub(crate) fn assess_run_with(
    root: &Path,
    resolved: &config::ResolvedConfig,
    now: u64,
    machine: &crate::presence::Machine,
) -> RunReport {
    let Roll { live, gone } = roll_call(root, resolved, now, machine);
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

    // Whether the shared stamp is fresh, decided once and carried on the
    // report. Every branch below that returns early does so before reaching the
    // stamp, and a caller asking "is a scheduler here" needs the answer on all
    // of them.
    let stamp_is_fresh = stamp
        .as_ref()
        .and_then(|stamp| stamp.age_secs(now))
        .is_some_and(|age| age <= window);

    // A fresh stamp written by a scheduler that is provably gone is not
    // evidence that anything is here. `.lisa/scheduler.alive` has one name on
    // it and as many writers as there are schedulers, so the only honest
    // reading of a fresh one is "whoever wrote this was alive then" — and when
    // every scheduler that could have written it has been asked about and
    // answered *gone*, the board's own stamp is the last thing holding a lock
    // nobody is behind. That was the second half of the S-070-01 deadlock: the
    // registry was cleared by hand and the shared file still fenced the board.
    let orphaned_stamp = live.is_empty() && !gone.is_empty();
    let stamped = stamp_is_fresh && !orphaned_stamp;

    // Work first, because work is the fact that matters and the only one a
    // second scheduler cannot forge on another's behalf: whoever consumed it,
    // something was written into that directory by a live pane or plugin.
    let quiet = signals_quiet_for(root, now);
    if let Some(quiet) = quiet.filter(|quiet| *quiet <= window) {
        return RunReport::working(
            format!(
                "Something wrote in {SIGNAL_DIR}/ {} ago, which only a running pane or plugin \
                 does.",
                humanize(quiet)
            ),
            live,
            stamped,
        );
    }

    // Nothing is moving. Anything still here is stamping and not visibly
    // working, and the registry is what says who that is.
    if !live.is_empty() {
        let quiet_for = match quiet {
            Some(quiet) => format!("nothing has moved in {SIGNAL_DIR}/ for {}", humanize(quiet)),
            None => format!("there is no {SIGNAL_DIR}/ to read"),
        };
        let evidence = format!(
            "{}, and {quiet_for}. A stamping scheduler is a process that exists, not a ticket \
             being worked.",
            match live.len() {
                1 => format!("{} is stamping this board", live[0].label()),
                _ => format!(
                    "{} schedulers are stamping this board ({})",
                    live.len(),
                    live.iter()
                        .map(|record| record.label())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        );
        return RunReport::stamping(evidence, live, stamped);
    }

    let stamp_age = match &stamp {
        Some(stamp) => match stamp.age_secs(now) {
            // No registry entry, but the shared stamp is fresh: a scheduler
            // from a build that predates the registry. It is here, and that is
            // all this file has ever been able to say about it — unless the
            // registry named its writer and this machine says that writer is
            // gone, in which case the stamp is a dead run's last word and the
            // decision falls through to the evidence below.
            Some(age) if age <= window && !orphaned_stamp => {
                return RunReport::stamping(
                    format!(
                        "Lisa's scheduler said it was running {} ago.",
                        humanize(age)
                    ),
                    Vec::new(),
                    stamped,
                )
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

    let Some(quiet) = quiet else {
        return RunReport::unclear(format!(
            "There is no {SIGNAL_DIR}/ to read, so there is nothing to say about it."
        ));
    };

    // What was asked, and what answered. A scheduler Lisa proved gone is named
    // with the proof, because "the stamp is old enough" and "pid 15340 is not a
    // process on this machine any more" are different sentences and only the
    // second one ends an argument.
    let seen = match (gone.first(), stamp_age) {
        (Some(gone), _) => format!(
            "{} stamped {}, and {}",
            gone.label,
            match gone.stamped_age {
                Some(age) => format!("{} ago", humanize(age)),
                None => "ahead of this machine's clock".to_string(),
            },
            gone.because
        ),
        (None, Some(age)) => format!(
            "Lisa's scheduler last said it was running {} ago",
            humanize(age)
        ),
        (None, None) => "Lisa has no record of a scheduler running here".to_string(),
    };
    RunReport {
        liveness: RunLiveness::Ended,
        evidence: format!(
            "{seen}, and nothing has changed in {SIGNAL_DIR}/ for {quiet_for}. Lisa waits \
             {waited} before believing that, because {window_reason}.",
            quiet_for = humanize(quiet),
            waited = humanize(window),
        ),
        schedulers: Vec::new(),
        stamped,
    }
}

/// A record whose stamp is fresh and whose process this machine says is gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoneScheduler {
    /// The name an operator reads.
    pub(crate) label: String,
    /// The id `lisa schedulers --stop` takes.
    pub(crate) scheduler_id: String,
    /// How long ago it stamped, or `None` for a stamp ahead of the clock.
    pub(crate) stamped_age: Option<u64>,
    /// What this machine was asked and what it answered.
    pub(crate) because: String,
}

/// The registry, read once and divided by what this machine says about it.
pub(crate) struct Roll {
    /// Stamping recently, and not provably gone. These hold the board.
    pub(crate) live: Vec<lisa_core::schedulers::SchedulerRecord>,
    /// Stamping recently, and provably gone. The state the whole ticket is
    /// about: a record that reads alive and a process that is not there.
    pub(crate) gone: Vec<GoneScheduler>,
}

/// Divide the roster into the runs that are here and the runs that only look
/// it.
///
/// A fresh stamp is where this starts and no longer where it ends. **A stamp
/// says a process was alive when it wrote; a pid is a question you can ask
/// now** — so every record whose stamp is fresh is put to
/// [`crate::presence`], and only the ones this machine cannot contradict count
/// as holding the board. An empty `live` still does not mean nobody is here: a
/// board whose scheduler predates the registry writes no record at all, which
/// is why the shared stamp stays in the decision above.
pub(crate) fn roll_call(
    root: &Path,
    resolved: &config::ResolvedConfig,
    now: u64,
    machine: &crate::presence::Machine,
) -> Roll {
    let mut roll = Roll {
        live: Vec::new(),
        gone: Vec::new(),
    };
    for record in lisa_core::schedulers::read_roster(root) {
        if !record.is_live(
            now,
            record
                .live_window_secs(resolved.wind_down_secs)
                .max(MIN_STAMPED_WINDOW_SECS),
        ) {
            // A stale stamp was already an answer before this ticket, and it
            // is the same answer now. Nothing is asked about its process:
            // there is nothing left for the question to change.
            continue;
        }
        let presence = machine.look(&record, now);
        if presence.is_gone() {
            roll.gone.push(GoneScheduler {
                label: record.label(),
                scheduler_id: record.scheduler_id.clone(),
                stamped_age: record.age_secs(now),
                because: presence.because().to_string(),
            });
        } else {
            roll.live.push(record);
        }
    }
    roll
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
    run_release_seats_with_writer(
        root,
        release,
        &mut out,
        now,
        &crate::presence::Machine::read(root),
    )
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
    machine: &crate::presence::Machine,
) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root.display()));
    }

    let resolved = match config::load_config(root) {
        Ok(validation) => config::resolve_config(&validation.config, None, None),
        Err(_) => config::ResolvedConfig::default(),
    };
    let report = assess_run_with(root, &resolved, now, machine);
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
                    RunLiveness::Working => "a run is working these seats",
                    RunLiveness::Stamping => "a scheduler is holding these seats",
                    _ => "Lisa cannot say these seats are free",
                },
                report.evidence
            ),
        )?;
        // A stamping scheduler is the state this refusal used to be a dead end
        // in: correct about the file, silent about which run to go and stop.
        if report.liveness == RunLiveness::Stamping {
            if let Some(how) = report.how_to_stop() {
                write_line(out, format_args!(""))?;
                write_line(out, format_args!("{how}"))?;
            }
        }
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

    /// The board as read on a machine that answers nothing about processes —
    /// the world every test here described before this ticket, and the one a
    /// reader falls back to when it cannot ask.
    fn assess(root: &Path, now: u64) -> RunReport {
        assess_on(root, now, crate::presence::Machine::unknown())
    }

    fn assess_on(root: &Path, now: u64, machine: crate::presence::Machine) -> RunReport {
        assess_run_with(root, &config::ResolvedConfig::default(), now, &machine)
    }

    /// The whole point: a fresh stamp means a run is here, whatever the seats
    /// look like and however long they have sat.
    #[test]
    fn a_scheduler_that_stamped_seconds_ago_holds_every_seat() {
        let dir = project();
        let now = later(86_400);
        stamp(dir.path(), now, 4);

        let report = assess(dir.path(), now);
        assert_eq!(report.liveness, RunLiveness::Stamping);
        assert!(report.evidence.contains("said it was running"));
        assert!(report.holds_the_board());
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

        let report = assess(dir.path(), now);
        assert_eq!(report.liveness, RunLiveness::Stamping);
        assert!(report.holds_the_board(), "a stamping run keeps its seats");
    }

    /// The other half: a scheduler whose stamp is stale but whose panes are
    /// still signalling is still a run.
    #[test]
    fn a_stale_stamp_alone_is_not_enough_while_signals_keep_moving() {
        let dir = project();
        let now = later(3);
        stamp(dir.path(), now, 86_400);

        let report = assess(dir.path(), now);
        assert_eq!(report.liveness, RunLiveness::Working);
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
            RunLiveness::Working,
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
        // Far enough out that the signal directory is quiet and the stamp is
        // the only thing left holding the seats.
        let now = later(70);
        stamp(dir.path(), now, 45);
        let resolved = config::ResolvedConfig {
            wind_down_secs: 5,
            ..config::ResolvedConfig::default()
        };

        assert_eq!(
            assess_run_with(
                dir.path(),
                &resolved,
                now,
                &crate::presence::Machine::unknown()
            )
            .liveness,
            RunLiveness::Stamping,
            "45s is inside the {MIN_STAMPED_WINDOW_SECS}s floor"
        );
    }

    /// Register a scheduler that stamped `age` seconds before `now`.
    fn register(root: &Path, id: &str, session: &str, now: u64, age: u64) {
        lisa_core::schedulers::write_record(
            &lisa_core::schedulers::roster_dir(root),
            &lisa_core::schedulers::SchedulerRecord::new(
                id,
                Some(session.to_string()),
                Some(30186),
                now - 17 * 3600,
                now - age,
                5,
            ),
        )
        .unwrap();
    }

    /// The sentence the day turned on. Three schedulers stamping a board where
    /// nothing has moved for hours is not "a run is working these seats", and
    /// the refusal has to name every one of them.
    #[test]
    fn schedulers_stamping_a_quiet_board_are_named_rather_than_summed_into_a_run() {
        let dir = project();
        let now = later(86_400);
        stamp(dir.path(), now, 4);
        register(dir.path(), "cymbal", "blossoming-cymbal", now, 3);
        register(dir.path(), "drum", "fascinating-drum", now, 4);

        let report = assess(dir.path(), now);

        assert_eq!(report.liveness, RunLiveness::Stamping);
        assert!(
            report.holds_the_board(),
            "seats stay while anything is here"
        );
        assert!(report.evidence.contains("2 schedulers are"));
        assert!(report.evidence.contains("blossoming-cymbal"));
        assert!(report.evidence.contains("fascinating-drum"));
        let how = report
            .how_to_stop()
            .expect("a named scheduler can be stopped");
        assert!(how.contains("zellij kill-session fascinating-drum"));
        assert!(how.contains("lisa schedulers"));
    }

    /// A machine that says the pid a record names is not a process any more.
    fn without_that_process() -> crate::presence::Machine {
        crate::presence::Machine::fixed(Some(Vec::new()), |_| crate::presence::Server::Missing)
    }

    /// The `renderer` board of 2026-08-14. The stamp is a minute old, the
    /// Zellij server it names is gone, and the seats are free — which is the
    /// answer every command on the day refused to give.
    #[test]
    fn a_fresh_stamp_from_a_scheduler_whose_server_is_gone_does_not_hold_the_seats() {
        let dir = project();
        let now = later(45 * 60);
        stamp(dir.path(), now, 87);
        register(dir.path(), "renderer-3-49ded6ab", "renderer-3", now, 87);

        let report = assess_on(dir.path(), now, without_that_process());

        assert_eq!(report.liveness, RunLiveness::Ended);
        assert!(report.seats_are_abandoned(), "{}", report.evidence);
        assert!(!report.holds_the_board());
        assert!(
            report.evidence.contains("renderer-3 (renderer-3-49ded6ab)"),
            "{}",
            report.evidence
        );
        assert!(
            report.evidence.contains("pid 30186"),
            "the proof is the pid, not the clock: {}",
            report.evidence
        );
        assert!(
            !report.stamped,
            "the shared stamp was written by that same dead run"
        );
    }

    /// The same board with the same stamp, on a machine that still holds the
    /// process. Nothing here may make it easier to release a seat somebody is
    /// working.
    #[test]
    fn the_same_stamp_with_the_server_still_up_keeps_every_seat() {
        let dir = project();
        let now = later(45 * 60);
        stamp(dir.path(), now, 87);
        register(dir.path(), "renderer-3-49ded6ab", "renderer-3", now, 87);

        let report = assess_on(
            dir.path(),
            now,
            crate::presence::Machine::fixed(Some(vec!["renderer-3".to_string()]), |_| {
                crate::presence::Server::Zellij {
                    up_secs: Some(20 * 3600),
                }
            }),
        );

        assert_eq!(report.liveness, RunLiveness::Stamping);
        assert!(report.holds_the_board());
        assert!(report.stamped);
    }

    /// A machine Lisa could not ask answers the way it always did: the stamp,
    /// and nothing else. Doubt keeps the seat.
    #[test]
    fn a_machine_that_cannot_be_asked_still_reads_the_stamp_alone() {
        let dir = project();
        let now = later(45 * 60);
        stamp(dir.path(), now, 87);
        register(dir.path(), "renderer-3-49ded6ab", "renderer-3", now, 87);

        let report = assess(dir.path(), now);

        assert_eq!(report.liveness, RunLiveness::Stamping);
        assert!(report.holds_the_board());
    }

    /// The refusal that used to be a dead end names both commands now, because
    /// the first one is exactly the one that cannot work once a session has
    /// ended without its scheduler noticing.
    #[test]
    fn the_refusal_names_a_remedy_that_still_works_when_the_session_is_gone() {
        let dir = project();
        let now = later(86_400);
        register(dir.path(), "renderer-3-49ded6ab", "renderer-3", now, 4);

        let how = assess(dir.path(), now)
            .how_to_stop()
            .expect("a named scheduler can be stopped");

        assert!(how.contains("zellij kill-session renderer-3"));
        assert!(how.contains("lisa schedulers --stop renderer-3-49ded6ab"));
        assert!(how.contains("already gone"));
    }

    /// The other half of the same distinction: panes writing means work, and
    /// work is not something a second scheduler can fake on another's behalf.
    #[test]
    fn a_board_whose_panes_are_writing_reads_as_working_not_merely_stamping() {
        let dir = project();
        let now = later(3);
        register(dir.path(), "cymbal", "blossoming-cymbal", now, 2);

        let report = assess(dir.path(), now);

        assert_eq!(report.liveness, RunLiveness::Working);
        assert_eq!(report.schedulers.len(), 1, "the run is still named");
    }

    /// A registry record left by a scheduler that stopped must not hold seats
    /// the shared stamp would also have released.
    #[test]
    fn a_registry_record_from_a_dead_scheduler_does_not_hold_the_seats() {
        let dir = project();
        let now = later(86_400);
        register(dir.path(), "gone", "lisa", now, 17 * 3600);

        let report = assess(dir.path(), now);

        assert_eq!(report.liveness, RunLiveness::Ended);
        assert!(report.seats_are_abandoned());
    }

    /// The refusal an operator reads while a scheduler holds a quiet board:
    /// still a refusal, no longer a dead end.
    #[test]
    fn a_stamping_scheduler_refuses_the_release_and_says_which_run_to_stop() {
        let dir = project();
        let now = later(86_400);
        register(dir.path(), "drum", "fascinating-drum", now, 4);

        let mut out = Vec::new();
        run_release_seats_with_writer(
            dir.path(),
            true,
            &mut out,
            now,
            &crate::presence::Machine::unknown(),
        )
        .unwrap();
        let printed = String::from_utf8(out).unwrap();

        assert!(printed.contains("a scheduler is holding these seats"));
        assert!(printed.contains("zellij kill-session fascinating-drum"));
        assert!(printed.contains("skip     .lisa/signals/pane-0.lease"));
        assert!(dir.path().join(SIGNAL_DIR).join("pane-0.lease").exists());
    }

    #[test]
    fn a_dry_run_prints_the_evidence_and_removes_nothing() {
        let dir = project();
        let now = later(86_400);
        stamp(dir.path(), now, 86_400);

        let mut out = Vec::new();
        run_release_seats_with_writer(
            dir.path(),
            false,
            &mut out,
            now,
            &crate::presence::Machine::unknown(),
        )
        .unwrap();
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
        run_release_seats_with_writer(
            dir.path(),
            true,
            &mut out,
            now,
            &crate::presence::Machine::unknown(),
        )
        .unwrap();
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
        run_release_seats_with_writer(
            dir.path(),
            true,
            &mut out,
            now,
            &crate::presence::Machine::unknown(),
        )
        .unwrap();
        let printed = String::from_utf8(out).unwrap();

        assert!(printed.contains("Nothing to release"));
        assert!(printed.contains("a run is working these seats"));
        assert!(printed.contains("skip     .lisa/signals/pane-0.lease"));
        assert!(dir.path().join(SIGNAL_DIR).join("pane-0.lease").exists());
    }

    #[test]
    fn a_project_with_no_seats_says_so_without_a_plan() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();

        let mut out = Vec::new();
        run_release_seats_with_writer(
            dir.path(),
            true,
            &mut out,
            later(0),
            &crate::presence::Machine::unknown(),
        )
        .unwrap();
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
