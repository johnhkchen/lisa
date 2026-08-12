//! A pane whose agent will not leave, and the attempt that was never made.
//!
//! Lisa's only evidence that a provider left its pane is silence. That is
//! usually enough, and where it is not — the pane keeps firing provider hooks
//! after `/exit` — the scheduler waits. What it used to do at the end of that
//! wait was launch anyway, on the reasoning that a dying provider must not
//! strand a seat forever. The reasoning is right and the action was not: the
//! launch line goes to whatever is reading the pane, and what is reading the
//! pane is the agent that never left. It arrives as chat.
//!
//! That is the field failure of `T-062-01-04`. The agent in the pane read the
//! next attempt's launcher as a message and answered it in words:
//!
//! > *"Not runnable from inside this session. It launches an interactive
//! > `claude` TUI … If I run it through Bash it either hangs with no terminal
//! > or nests a Claude inside this one."*
//!
//! It was right, the session was recorded failed, and the scheduler tried
//! again into the same pane. Three tickets, and a counter that reached four
//! with nothing to show but four assignment files.
//!
//! So the ceiling releases the **ticket** instead of the pane. This module
//! holds what that leaves behind: which pane is held, which ticket had to move,
//! and the durable note that an attempt number was spent without a session ever
//! starting — so `attempt 4` cannot be read as four tries.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use lisa_core::types::TicketId;
use serde::{Deserialize, Serialize};

/// A pane that did not come back, and what Lisa did about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WedgedSeat {
    /// The ticket that was waiting for this pane and had to be released.
    pub(crate) ticket_id: Option<TicketId>,
    /// The attempt generation that was never started here.
    pub(crate) attempt_id: Option<u64>,
    /// When Lisa concluded the pane was held.
    pub(crate) since: SystemTime,
}

impl WedgedSeat {
    /// True once the pane has been silent long enough to be believed departed.
    ///
    /// The ordinary exit path treats one quiet grace as departure evidence.
    /// A pane that has already proven it can outlive an `/exit` gets the whole
    /// ceiling of silence instead — the same evidence, held to a higher bar,
    /// rather than a new kind of evidence nobody can check.
    pub(crate) fn is_released(
        &self,
        now: SystemTime,
        last_provider_signal: Option<SystemTime>,
        release_after: Duration,
    ) -> bool {
        if now.duration_since(self.since).unwrap_or_default() < release_after {
            return false;
        }
        match last_provider_signal {
            None => true,
            Some(last) => now.duration_since(last).unwrap_or_default() >= release_after,
        }
    }
}

/// The dashboard line for one held pane.
///
/// Names the pane, because the pane is the fact Lisa actually knows and the
/// operator can look at. `"Session failed"` — printed three times during the
/// field failure — sent the operator to transcripts that were never written.
pub(crate) fn alert_detail(pane_id: u32, seat: &WedgedSeat) -> String {
    match &seat.ticket_id {
        Some(ticket_id) => format!(
            "Pane {} is still held by its previous agent; {} moved to another seat",
            pane_id, ticket_id
        ),
        None => format!("Pane {} is still held by its previous agent", pane_id),
    }
}

/// What an operator can do about it, in the order they should try it.
pub(crate) fn suggested_actions(pane_id: u32) -> Vec<String> {
    vec![
        format!("Look at pane {} and end the agent there", pane_id),
        "lisa reset-ticket <ticket>".to_string(),
    ]
}

/// The durable note that one attempt number bought no session at all.
///
/// Attempt generations are minted monotonically and never reused — walking a
/// counter backwards would break the fencing that keeps two attempts from ever
/// holding the same ticket. So the counter still climbs, and this record is
/// what keeps the climb honest: a reader of `.lisa/attempts/<ticket>/` can tell
/// a try from a number that was spent before anything ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct VoidAttempt {
    /// The shape of this record.
    pub(crate) schema_version: u32,
    /// The ticket whose attempt number was spent.
    pub(crate) ticket_id: TicketId,
    /// The generation that never started.
    pub(crate) attempt_id: u64,
    /// The pane it would have started in.
    pub(crate) pane_id: u32,
    /// Unix seconds at which Lisa gave up on this attempt.
    pub(crate) voided_at: u64,
    /// Plain sentence for whoever reads this directory later.
    pub(crate) reason: String,
}

impl VoidAttempt {
    pub(crate) const SCHEMA_VERSION: u32 = 1;
    /// File name inside the attempt directory.
    pub(crate) const FILE_NAME: &'static str = "void.json";

    /// A record for an attempt refused because its pane was held.
    pub(crate) fn held_pane(
        ticket_id: TicketId,
        attempt_id: u64,
        pane_id: u32,
        voided_at: u64,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            ticket_id,
            attempt_id,
            pane_id,
            voided_at,
            reason: format!(
                "No session ever started: pane {pane_id} was still held by its previous agent, \
                 so nothing was launched and this attempt number was never used."
            ),
        }
    }
}

/// Where a void record lives: beside the attempt's own work directory, so the
/// evidence and the absence of evidence sit in the same place.
pub(crate) fn void_record_path(attempt_root: &Path, ticket_id: &str, attempt_id: u64) -> PathBuf {
    attempt_root
        .join(ticket_id)
        .join(attempt_id.to_string())
        .join(VoidAttempt::FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn seat() -> WedgedSeat {
        WedgedSeat {
            ticket_id: Some("T-062-01-03".to_string()),
            attempt_id: Some(4),
            since: at(1_000),
        }
    }

    #[test]
    fn a_pane_that_is_still_talking_is_not_released() {
        let seat = seat();
        let release_after = Duration::from_secs(90);
        // Long enough since the wedge, but the provider spoke a moment ago.
        assert!(!seat.is_released(at(2_000), Some(at(1_990)), release_after));
        // Quiet for the full window: believed departed.
        assert!(seat.is_released(at(2_000), Some(at(1_900)), release_after));
        // Never heard from at all, and the window has passed.
        assert!(seat.is_released(at(2_000), None, release_after));
    }

    #[test]
    fn a_fresh_wedge_is_never_released_early_however_quiet_it_is() {
        let seat = seat();
        let release_after = Duration::from_secs(90);
        assert!(!seat.is_released(at(1_010), None, release_after));
        assert!(seat.is_released(at(1_090), None, release_after));
    }

    #[test]
    fn the_alert_names_the_pane_and_the_ticket_that_moved() {
        let detail = alert_detail(3, &seat());
        assert!(detail.contains("Pane 3"));
        assert!(detail.contains("T-062-01-03"));
        assert!(!detail.contains("Session failed"));

        let unassigned = WedgedSeat {
            ticket_id: None,
            ..seat()
        };
        assert_eq!(
            alert_detail(3, &unassigned),
            "Pane 3 is still held by its previous agent"
        );
    }

    #[test]
    fn the_void_record_says_no_session_ever_started() {
        let record = VoidAttempt::held_pane("T-062-01-03".to_string(), 4, 3, 1_786_500_000);
        assert_eq!(record.schema_version, VoidAttempt::SCHEMA_VERSION);
        assert!(record.reason.contains("No session ever started"));
        assert!(record.reason.contains("pane 3"));

        let encoded = serde_json::to_string(&record).unwrap();
        assert_eq!(
            serde_json::from_str::<VoidAttempt>(&encoded).unwrap(),
            record
        );
    }

    #[test]
    fn the_void_record_sits_in_the_attempt_it_describes() {
        assert_eq!(
            void_record_path(Path::new("/repo/.lisa/attempts"), "T-062-01-03", 4),
            PathBuf::from("/repo/.lisa/attempts/T-062-01-03/4/void.json")
        );
    }
}
