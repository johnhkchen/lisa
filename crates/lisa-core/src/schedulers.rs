//! One file per scheduler, so a board can say how many are on it.
//!
//! [`crate::liveness`] answers *is a scheduler running* with a single file that
//! every scheduler rewrites. That is enough for one reader and wrong for two:
//! three schedulers stamping `.lisa/scheduler.alive` produce exactly the same
//! bytes one healthy scheduler does, so a board with three runs on it reads as
//! a board with one. Every recovery Lisa offers keys on that file, so the state
//! that most needs recovering is the state that locks recovery out.
//!
//! This registry answers the question that actually matters — *how many, and
//! which* — by giving each scheduler its own file, named after itself:
//!
//! ```text
//! .lisa/schedulers/lisa-9c1f4b0a.alive          the scheduler's own stamp
//! .lisa/schedulers/lisa-9c1f4b0a.consumed.jsonl what it took out of .lisa/signals/
//! ```
//!
//! A file per writer is what makes the count real. Nothing merges, nothing
//! races, and a scheduler that dies leaves a record dated the moment it stopped
//! rather than a shared file somebody else keeps fresh.
//!
//! The record carries enough identity to *act* on: the Zellij session name,
//! because `zellij kill-session <name>` is the one thing measured to stop a
//! scheduler whose client is gone, and the Zellij server pid, because that is
//! what `lsof` and `ps` speak. Neither is inferable from the outside — the
//! plugin lives inside the server that owns both — so the scheduler writes them
//! down while it can.
//!
//! ## The consumption ledger
//!
//! `.lisa/signals/` is single-consumer by design and that contract is not
//! changed here: a scheduler still reads a signal and removes it. What is added
//! is a receipt. Before removing a *decisive* signal — the ones whose loss
//! changes what happens to a pane — the consuming scheduler appends one line
//! naming itself, so a signal that vanished into another scheduler is
//! distinguishable from one that was never written and from one that was acted
//! on here. Heartbeats and liveness pings are deliberately not recorded: they
//! arrive every few seconds, they are replaced by the next one, and a ledger
//! that grows with them would be unreadable by the time anyone needed it.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The registry directory, relative to the project root.
pub const SCHEDULER_DIR: &str = ".lisa/schedulers";

/// The registry directory's own name, for a writer that already holds `.lisa/`.
pub const SCHEDULER_DIR_NAME: &str = "schedulers";

/// Suffix of a scheduler's own stamp file.
pub const RECORD_SUFFIX: &str = ".alive";

/// Suffix of a scheduler's consumption ledger.
pub const CONSUMED_SUFFIX: &str = ".consumed.jsonl";

/// The shortest window Lisa will call a stamp fresh in, whatever a project
/// configures. Mirrors the floor [`crate::liveness`] readers already apply: a
/// tick that runs late must never read as a scheduler that stopped.
pub const MIN_LIVE_WINDOW_SECS: u64 = 60;

/// How long a record whose scheduler never came back is kept before a starting
/// scheduler sweeps it.
///
/// Long enough that yesterday evening's zombie is still on the list this
/// morning — the exact case this registry exists for — and short enough that a
/// board does not accumulate a year of dead runs.
pub const FORGET_STALE_AFTER_SECS: u64 = 7 * 24 * 3600;

/// One scheduler's statement of itself: that it is running, and which one it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerRecord {
    /// The shape of this record. Bumped only if a field's meaning changes.
    pub schema_version: u32,
    /// Filename-safe identity, unique to this scheduler on this board.
    pub scheduler_id: String,
    /// The Zellij session this scheduler's plugin runs in, when the launcher
    /// named one. `zellij kill-session <name>` is the supported way to stop it.
    #[serde(default)]
    pub session_name: Option<String>,
    /// The Zellij *server* pid — the process that outlives the client and keeps
    /// scheduling. Present for `lsof`/`ps`, not as a way to stop it: killing it
    /// was measured not to work.
    #[serde(default)]
    pub zellij_pid: Option<u32>,
    /// Unix seconds at which this scheduler started.
    pub started_at: u64,
    /// Unix seconds of its most recent stamp.
    pub stamped_at: u64,
    /// How often it intends to stamp, so a reader can tell a late stamp from a
    /// stopped one without hard-coding Lisa's poll cadence.
    pub poll_interval_secs: u64,
}

impl SchedulerRecord {
    /// The version this build writes and understands.
    pub const SCHEMA_VERSION: u32 = 1;

    /// A record for a scheduler that has just started and just stamped.
    pub fn new(
        scheduler_id: impl Into<String>,
        session_name: Option<String>,
        zellij_pid: Option<u32>,
        started_at: u64,
        stamped_at: u64,
        poll_interval_secs: u64,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            scheduler_id: scheduler_id.into(),
            session_name,
            zellij_pid,
            started_at,
            stamped_at,
            poll_interval_secs,
        }
    }

    /// How long ago this scheduler stamped, or `None` for a stamp dated ahead
    /// of the reader's clock — which is not evidence of anything.
    pub fn age_secs(&self, now: u64) -> Option<u64> {
        now.checked_sub(self.stamped_at)
    }

    /// Whether this scheduler stamped recently enough to still be here.
    pub fn is_live(&self, now: u64, window_secs: u64) -> bool {
        self.age_secs(now).is_some_and(|age| age <= window_secs)
    }

    /// The window this record's own cadence implies, widened by whatever the
    /// project's wind-down asks for and never below the floor.
    pub fn live_window_secs(&self, wind_down_secs: u64) -> u64 {
        live_window_secs(self.poll_interval_secs, wind_down_secs)
    }

    /// The identity an operator reads: the session name when there is one,
    /// because that is the name every Zellij command takes, and the id
    /// otherwise.
    pub fn label(&self) -> String {
        match &self.session_name {
            Some(session) => format!("{session} ({})", self.scheduler_id),
            None => self.scheduler_id.clone(),
        }
    }

    /// The exact command that stops this scheduler, when Lisa knows enough to
    /// name one.
    pub fn stop_command(&self) -> Option<String> {
        self.session_name
            .as_ref()
            .map(|session| format!("zellij kill-session {session}"))
    }
}

/// The window a stamp counts as fresh in: the project's wind-down, never
/// shorter than six poll intervals, never shorter than the floor.
pub fn live_window_secs(poll_interval_secs: u64, wind_down_secs: u64) -> u64 {
    wind_down_secs
        .max(poll_interval_secs.saturating_mul(6))
        .max(MIN_LIVE_WINDOW_SECS)
}

/// The registry directory under `root`.
pub fn roster_dir(root: &Path) -> PathBuf {
    root.join(SCHEDULER_DIR)
}

/// This scheduler's own stamp file inside `dir`.
pub fn record_path(dir: &Path, scheduler_id: &str) -> PathBuf {
    dir.join(format!("{scheduler_id}{RECORD_SUFFIX}"))
}

/// This scheduler's consumption ledger inside `dir`.
pub fn consumed_path(dir: &Path, scheduler_id: &str) -> PathBuf {
    dir.join(format!("{scheduler_id}{CONSUMED_SUFFIX}"))
}

/// Every record on this board, oldest run first.
///
/// A record Lisa cannot parse is skipped rather than guessed at: the whole
/// value of this directory is that each file speaks for exactly one scheduler,
/// and half a file speaks for none.
pub fn read_roster(root: &Path) -> Vec<SchedulerRecord> {
    read_roster_dir(&roster_dir(root))
}

/// Every record in an explicit registry directory, oldest run first.
pub fn read_roster_dir(dir: &Path) -> Vec<SchedulerRecord> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut records: Vec<SchedulerRecord> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(RECORD_SUFFIX))
        })
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|body| serde_json::from_str::<SchedulerRecord>(&body).ok())
        .collect();
    records.sort_by(|left, right| {
        left.started_at
            .cmp(&right.started_at)
            .then_with(|| left.scheduler_id.cmp(&right.scheduler_id))
    });
    records
}

/// Write one scheduler's record, creating the registry directory if needed.
///
/// Best-effort by design and never a fence: a stamp that cannot be written
/// costs an operator a diagnosis, and the next tick tries again. Failing a
/// scheduler over its own bookkeeping would turn a reporting aid into an
/// outage.
pub fn write_record(dir: &Path, record: &SchedulerRecord) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let body = serde_json::to_vec(record)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    std::fs::write(record_path(dir, &record.scheduler_id), body)
}

/// Remove a scheduler's record and ledger. The scheduler that owns it cannot
/// withdraw it — a run that is gone is exactly the run that cannot — so this is
/// for whoever established that it is gone.
pub fn forget(dir: &Path, scheduler_id: &str) {
    let _ = std::fs::remove_file(record_path(dir, scheduler_id));
    let _ = std::fs::remove_file(consumed_path(dir, scheduler_id));
}

/// Drop records for schedulers that stopped longer ago than
/// [`FORGET_STALE_AFTER_SECS`], and report which ids went.
///
/// Deliberately conservative about the recent past: a stale-looking record from
/// last night is the evidence this registry exists to preserve.
pub fn forget_long_dead(dir: &Path, now: u64) -> Vec<String> {
    let mut forgotten = Vec::new();
    for record in read_roster_dir(dir) {
        if record
            .age_secs(now)
            .is_some_and(|age| age > FORGET_STALE_AFTER_SECS)
        {
            forget(dir, &record.scheduler_id);
            forgotten.push(record.scheduler_id);
        }
    }
    forgotten
}

/// A filename-safe id for a scheduler, derived from what it knows about itself.
///
/// The visible half is the session name, because that is what an operator is
/// going to type; the hashed half separates two runs of the same session name
/// and two anonymous schedulers started a moment apart.
pub fn scheduler_id(session_name: Option<&str>, started_at_nanos: u128, salt: u64) -> String {
    let base = session_name
        .map(sanitize_id_base)
        .filter(|base| !base.is_empty())
        .unwrap_or_else(|| "scheduler".to_string());
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in base
        .as_bytes()
        .iter()
        .chain(started_at_nanos.to_le_bytes().iter())
        .chain(salt.to_le_bytes().iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{base}-{:08x}", hash as u32)
}

/// Reduce a session name to characters safe in a filename and in a shell word.
fn sanitize_id_base(session_name: &str) -> String {
    let mut base = String::new();
    for ch in session_name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            base.push(ch);
        } else if !base.ends_with('-') {
            base.push('-');
        }
    }
    base.trim_matches('-').chars().take(32).collect()
}

// ---------------------------------------------------------------------------
// The consumption ledger
// ---------------------------------------------------------------------------

/// One receipt: this scheduler took this signal out of the shared directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalReceipt {
    /// The shape of this line. Bumped only if a field's meaning changes.
    pub schema_version: u32,
    /// Unix seconds at which the signal was consumed.
    pub at: u64,
    /// Which scheduler consumed it.
    pub scheduler_id: String,
    /// The signal's exact file name, e.g. `pane-1.started`.
    pub signal: String,
    /// The pane the signal named, when the name carried one.
    #[serde(default)]
    pub pane_id: Option<u32>,
}

impl SignalReceipt {
    /// The version this build writes and understands.
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn new(at: u64, scheduler_id: impl Into<String>, signal: impl Into<String>) -> Self {
        let signal = signal.into();
        let pane_id = pane_id_of(&signal);
        Self {
            schema_version: Self::SCHEMA_VERSION,
            at,
            scheduler_id: scheduler_id.into(),
            signal,
            pane_id,
        }
    }
}

/// The pane a `pane-<n>.<kind>` file name refers to.
fn pane_id_of(signal: &str) -> Option<u32> {
    signal
        .strip_prefix("pane-")?
        .split('.')
        .next()?
        .parse()
        .ok()
}

/// Append one receipt to the consuming scheduler's own ledger.
///
/// Per-scheduler files rather than a shared log: the point of the whole
/// registry is that no two schedulers write the same bytes, and a shared
/// append-only file from three processes is the mistake this module exists to
/// undo.
pub fn append_receipt(dir: &Path, receipt: &SignalReceipt) -> std::io::Result<()> {
    use std::io::Write;

    std::fs::create_dir_all(dir)?;
    let mut line = serde_json::to_vec(receipt)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    line.push(b'\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(consumed_path(dir, &receipt.scheduler_id))?;
    file.write_all(&line)
}

/// Every receipt on this board, oldest first.
pub fn read_receipts(dir: &Path) -> Vec<SignalReceipt> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut receipts: Vec<SignalReceipt> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(CONSUMED_SUFFIX))
        })
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .flat_map(|body| {
            body.lines()
                .filter_map(|line| serde_json::from_str::<SignalReceipt>(line).ok())
                .collect::<Vec<_>>()
        })
        .collect();
    receipts.sort_by(|left, right| left.at.cmp(&right.at));
    receipts
}

/// The most recent receipt for one exact signal name taken by somebody other
/// than `scheduler_id`.
///
/// This is the question a scheduler asks before deciding that a startup it
/// never saw did not happen: *did another scheduler eat the proof?*
pub fn taken_by_another(
    dir: &Path,
    scheduler_id: &str,
    signal: &str,
    since: u64,
) -> Option<SignalReceipt> {
    read_receipts(dir)
        .into_iter()
        .filter(|receipt| {
            receipt.signal == signal && receipt.scheduler_id != scheduler_id && receipt.at >= since
        })
        .next_back()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, started_at: u64, stamped_at: u64) -> SchedulerRecord {
        SchedulerRecord::new(
            id,
            Some("lisa".to_string()),
            Some(9450),
            started_at,
            stamped_at,
            5,
        )
    }

    #[test]
    fn a_record_round_trips_through_the_shape_both_sides_read() {
        let record = record("lisa-9c1f4b0a", 1_786_487_000, 1_786_487_257);
        let encoded = serde_json::to_string(&record).unwrap();
        assert_eq!(
            serde_json::from_str::<SchedulerRecord>(&encoded).unwrap(),
            record
        );
        assert!(encoded.contains("\"session_name\":\"lisa\""));
    }

    /// The whole point: three schedulers are three files, and the count is
    /// readable without asking any of them.
    #[test]
    fn three_schedulers_on_one_board_read_as_three() {
        let dir = tempfile::tempdir().unwrap();
        for (index, id) in ["blossoming-cymbal", "inventive-triceratops", "fascinating-drum"]
            .iter()
            .enumerate()
        {
            write_record(
                dir.path(),
                &SchedulerRecord::new(
                    *id,
                    Some((*id).to_string()),
                    Some(9450 + index as u32),
                    1_000 + index as u64,
                    2_000,
                    5,
                ),
            )
            .unwrap();
        }

        let roster = read_roster_dir(dir.path());
        assert_eq!(roster.len(), 3);
        assert_eq!(roster[0].scheduler_id, "blossoming-cymbal");
        assert!(roster.iter().all(|record| record.is_live(2_010, 60)));
        assert_eq!(
            roster[2].stop_command().unwrap(),
            "zellij kill-session fascinating-drum"
        );
    }

    #[test]
    fn a_record_that_stopped_stamping_reads_as_stale_while_a_fresh_one_reads_as_live() {
        let now = 100_000;
        let live = record("live", 0, now - 4);
        let dead = record("dead", 0, now - 17 * 3600 - 4);

        assert!(live.is_live(now, live.live_window_secs(300)));
        assert!(!dead.is_live(now, dead.live_window_secs(300)));
        assert_eq!(dead.age_secs(now), Some(17 * 3600 + 4));
    }

    #[test]
    fn a_stamp_dated_ahead_of_the_clock_is_never_read_as_live() {
        let ahead = record("ahead", 0, 5_000);
        assert_eq!(ahead.age_secs(4_000), None);
        assert!(!ahead.is_live(4_000, 3_600));
    }

    #[test]
    fn the_window_never_shrinks_below_the_floor_or_six_ticks() {
        assert_eq!(live_window_secs(5, 5), MIN_LIVE_WINDOW_SECS);
        assert_eq!(live_window_secs(30, 5), 180);
        assert_eq!(live_window_secs(5, 900), 900);
    }

    #[test]
    fn an_unreadable_record_is_skipped_rather_than_guessed_at() {
        let dir = tempfile::tempdir().unwrap();
        write_record(dir.path(), &record("good", 0, 10)).unwrap();
        std::fs::write(dir.path().join("torn.alive"), "{\"scheduler_id\":").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "mine").unwrap();

        let roster = read_roster_dir(dir.path());
        assert_eq!(roster.len(), 1);
        assert_eq!(roster[0].scheduler_id, "good");
    }

    #[test]
    fn ids_separate_two_runs_of_one_session_name() {
        let first = scheduler_id(Some("lisa"), 1_786_487_000_000_000_000, 7);
        let second = scheduler_id(Some("lisa"), 1_786_487_000_000_000_001, 7);
        assert_ne!(first, second);
        assert!(first.starts_with("lisa-"));
        assert_eq!(
            scheduler_id(Some("repos/lisa 2"), 1, 1).split('-').next(),
            Some("repos")
        );
        assert!(scheduler_id(None, 1, 1).starts_with("scheduler-"));
    }

    #[test]
    fn last_nights_record_survives_the_sweep_and_last_years_does_not() {
        let dir = tempfile::tempdir().unwrap();
        let now = 10 * FORGET_STALE_AFTER_SECS;
        write_record(dir.path(), &record("overnight", 0, now - 17 * 3600)).unwrap();
        write_record(dir.path(), &record("ancient", 0, now - 30 * 24 * 3600)).unwrap();

        assert_eq!(forget_long_dead(dir.path(), now), vec!["ancient"]);
        let remaining = read_roster_dir(dir.path());
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].scheduler_id, "overnight");
    }

    #[test]
    fn forgetting_a_scheduler_takes_its_ledger_with_it() {
        let dir = tempfile::tempdir().unwrap();
        write_record(dir.path(), &record("gone", 0, 10)).unwrap();
        append_receipt(dir.path(), &SignalReceipt::new(10, "gone", "pane-1.started")).unwrap();

        forget(dir.path(), "gone");

        assert!(read_roster_dir(dir.path()).is_empty());
        assert!(read_receipts(dir.path()).is_empty());
    }

    /// The incident, in one assertion: the live scheduler never saw
    /// `pane-1.started`, and the ledger says where it went.
    #[test]
    fn a_signal_that_vanished_into_another_scheduler_is_named() {
        let dir = tempfile::tempdir().unwrap();
        append_receipt(
            dir.path(),
            &SignalReceipt::new(1_200, "fascinating-drum", "pane-1.started"),
        )
        .unwrap();
        append_receipt(
            dir.path(),
            &SignalReceipt::new(1_300, "blossoming-cymbal", "pane-2.started"),
        )
        .unwrap();

        let thief = taken_by_another(dir.path(), "blossoming-cymbal", "pane-1.started", 0).unwrap();
        assert_eq!(thief.scheduler_id, "fascinating-drum");
        assert_eq!(thief.pane_id, Some(1));
        assert!(
            taken_by_another(dir.path(), "blossoming-cymbal", "pane-2.started", 0).is_none(),
            "a signal this scheduler consumed itself is not a signal that vanished"
        );
        assert!(
            taken_by_another(dir.path(), "blossoming-cymbal", "pane-1.started", 1_250).is_none(),
            "a receipt older than the question does not answer it"
        );
    }

    #[test]
    fn receipts_from_every_scheduler_read_back_in_time_order() {
        let dir = tempfile::tempdir().unwrap();
        append_receipt(dir.path(), &SignalReceipt::new(300, "b", "pane-0.claim")).unwrap();
        append_receipt(dir.path(), &SignalReceipt::new(100, "a", "pane-0.started")).unwrap();
        append_receipt(dir.path(), &SignalReceipt::new(200, "a", "pane-0.stopped")).unwrap();

        let receipts = read_receipts(dir.path());
        assert_eq!(
            receipts
                .iter()
                .map(|receipt| receipt.signal.as_str())
                .collect::<Vec<_>>(),
            vec!["pane-0.started", "pane-0.stopped", "pane-0.claim"]
        );
    }
}
