//! The scheduler's own statement that it is still running.
//!
//! A pane lease marker records a *placement*. It says nothing about whether the
//! run that made the placement still exists, and a run that dies without
//! shutting down cannot withdraw one — the process that would have done it is
//! the one that died. On disk, a lease held by a live but slow session and a
//! lease left by a dead run are byte-identical.
//!
//! So the scheduler says so itself. Every poll tick the plugin rewrites
//! [`SCHEDULER_ALIVE_FILE`] with the moment it happened. The stamp stops the
//! instant the process stops, however it stops — crash, kill, machine swap, a
//! terminal closed on it — because nothing but a running scheduler ever writes
//! it. A reader that finds a stamp from four seconds ago knows a scheduler is
//! there; one that finds a stamp from yesterday knows the run that placed
//! today's seats is gone.
//!
//! This is deliberately **not** a pane signal. It lives beside `.lisa/signals/`
//! rather than in it, so that "the signal directory has gone quiet" stays an
//! independent fact that the stamp cannot contaminate — the two together are
//! what let a consumer say a seat is abandoned rather than merely idle. It is
//! also not part of the single-consumer signal contract: nothing consumes it,
//! and reading it destroys nothing.

use serde::{Deserialize, Serialize};

/// Where the scheduler stamps itself, relative to the project root.
pub const SCHEDULER_ALIVE_FILE: &str = ".lisa/scheduler.alive";

/// The stamp's file name alone, for a writer that already holds `.lisa/`.
pub const SCHEDULER_ALIVE_NAME: &str = "scheduler.alive";

/// The scheduler's most recent "I am running" stamp.
///
/// `stamped_at` is the authority. The file's mtime says the same thing, but a
/// copied or restored tree can carry an mtime the writer never chose, so the
/// number written inside is what a reader trusts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerAlive {
    /// The shape of this record. Bumped only if a field's meaning changes.
    pub schema_version: u32,
    /// Unix seconds at which the scheduler wrote this stamp.
    pub stamped_at: u64,
    /// How often the scheduler intends to rewrite it, so a reader can tell a
    /// late stamp from a stopped one without hard-coding Lisa's poll cadence.
    pub poll_interval_secs: u64,
}

impl SchedulerAlive {
    /// The version this build writes and understands.
    pub const SCHEMA_VERSION: u32 = 1;

    /// A stamp for `stamped_at`, from a scheduler polling every
    /// `poll_interval_secs`.
    pub fn new(stamped_at: u64, poll_interval_secs: u64) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            stamped_at,
            poll_interval_secs,
        }
    }

    /// How long ago this stamp was written, in seconds, or `None` when it
    /// carries a future time.
    ///
    /// A future stamp is not treated as "very recent" and not treated as stale
    /// either: a clock that moved is a thing Lisa cannot reason about, and the
    /// callers of this turn `None` into *keep the seat*.
    pub fn age_secs(&self, now: u64) -> Option<u64> {
        now.checked_sub(self.stamped_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamp_round_trips_through_the_shape_both_sides_read() {
        let stamp = SchedulerAlive::new(1_786_487_257, 5);
        let encoded = serde_json::to_string(&stamp).unwrap();
        assert_eq!(
            serde_json::from_str::<SchedulerAlive>(&encoded).unwrap(),
            stamp
        );
        assert!(encoded.contains("\"stamped_at\":1786487257"));
    }

    #[test]
    fn age_is_none_when_the_stamp_is_in_the_future() {
        let stamp = SchedulerAlive::new(2_000, 5);
        assert_eq!(stamp.age_secs(2_090), Some(90));
        assert_eq!(stamp.age_secs(2_000), Some(0));
        assert_eq!(stamp.age_secs(1_999), None);
    }
}
