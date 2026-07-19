//! Pane-time ownership over durable scheduler provenance plus live occupancy.
//!
//! Terminal execution records retain the physical pane, ticket, attempt, and
//! start time after a thread leaves scheduler memory. A capture is attributed by
//! **pane reign**: on a pane, each occupant reigns from its `started_at` until
//! the next occupant starts, so a capture belongs to the occupant with the
//! greatest `started_at` at or before the capture time.
//!
//! This matters because rest-before-retire lands a session's final capture
//! *after* its terminal `ended_at`; a closed `[started_at, ended_at]` window
//! could never cover it, but the reign extends past `ended_at` until the pane's
//! next occupant. A reign that is still open must not swallow a capture belonging
//! to a currently live but not-yet-recorded successor, so live threads are folded
//! in as pending occupants and resolve to [`ReignOutcome::Pending`].

use lisa_core::provenance::ProvenanceRecord;

/// What a pane occupant is: a durable, attributable terminal record, or a live
/// thread that has not yet produced one.
pub(crate) enum ReignSource<'a> {
    /// A durable terminal execution record — attributable now.
    Completed(&'a ProvenanceRecord),
    /// An in-flight thread occupying the pane — attribution must wait for it to
    /// complete and land its own record.
    Live { ticket_id: &'a str },
}

/// One pane occupant in reign resolution.
pub(crate) struct Reign<'a> {
    pub(crate) pane_id: u32,
    pub(crate) started_at: u64,
    pub(crate) source: ReignSource<'a>,
}

/// The result of resolving a capture against a pane's reigns.
pub(crate) enum ReignOutcome<'a> {
    /// The capture belongs to this completed record's ticket/attempt.
    Attributed(&'a ProvenanceRecord),
    /// The winning reign is a live thread; attribution is not yet possible.
    Pending,
    /// No occupant started at or before the capture, or the evidence is
    /// ambiguous — the capture is unowned and quarantines.
    Unowned,
}

/// Resolve which occupant reigned over `pane_id` at `captured_at`.
///
/// The winner is the occupant with the greatest `started_at ≤ captured_at`.
/// If two *different* tickets tie on that greatest `started_at`, the evidence is
/// ambiguous and this fails closed to [`ReignOutcome::Unowned`] rather than
/// depending on record order. A live winner yields [`ReignOutcome::Pending`].
pub(crate) fn reign_owner_at<'a>(
    reigns: &'a [Reign<'a>],
    pane_id: u32,
    captured_at: u64,
) -> ReignOutcome<'a> {
    let mut best_started_at: Option<u64> = None;
    let mut winner: Option<&'a Reign<'a>> = None;
    let mut ambiguous = false;

    for reign in reigns {
        if reign.pane_id != pane_id || reign.started_at > captured_at {
            continue;
        }
        match best_started_at {
            Some(best) if reign.started_at < best => {}
            Some(best) if reign.started_at == best => {
                if ticket_of(reign) != winner.map(ticket_of).unwrap_or_default() {
                    // Two different tickets tie on the winning start → ambiguous.
                    ambiguous = true;
                } else if matches!(reign.source, ReignSource::Completed(_)) {
                    // Same ticket: a durable record can attribute now, so prefer
                    // it over a still-live occupant (a resting session shows both
                    // a completed row and its live thread at the same start).
                    winner = Some(reign);
                }
            }
            _ => {
                best_started_at = Some(reign.started_at);
                winner = Some(reign);
                ambiguous = false;
            }
        }
    }

    if ambiguous {
        return ReignOutcome::Unowned;
    }
    match winner {
        None => ReignOutcome::Unowned,
        Some(reign) => match &reign.source {
            ReignSource::Completed(record) => ReignOutcome::Attributed(record),
            ReignSource::Live { .. } => ReignOutcome::Pending,
        },
    }
}

fn ticket_of<'a>(reign: &'a Reign<'a>) -> &'a str {
    match &reign.source {
        ReignSource::Completed(record) => record.ticket_id.as_str(),
        ReignSource::Live { ticket_id } => ticket_id,
    }
}

#[cfg(test)]
mod tests {
    use lisa_core::completion::CompletionSeal;
    use lisa_core::provenance::{ProvenanceRecord, Route, RunOutcome, SCHEMA_VERSION};
    use lisa_core::types::AttemptLease;

    use super::{reign_owner_at, Reign, ReignOutcome, ReignSource};

    fn record(
        ticket_id: &str,
        attempt_id: u64,
        pane_id: u32,
        started_at: u64,
        ended_at: u64,
    ) -> ProvenanceRecord {
        let route = Route {
            method: "claude".to_string(),
            provider: "anthropic".to_string(),
            model: None,
        };
        ProvenanceRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            completion_note: None,
            ticket_id: ticket_id.to_string(),
            attempt_lease: AttemptLease {
                ticket_id: ticket_id.to_string(),
                attempt_id,
            },
            outcome: RunOutcome::Done,
            authoritative: true,
            fenced: false,
            requested: route.clone(),
            actual: route,
            started_at,
            ended_at,
            wall_clock_secs: ended_at.saturating_sub(started_at),
            tokens_in: None,
            tokens_out: None,
            cost_usd: None,
            concurrency_at_spawn: 0,
            pane_id,
        }
    }

    fn completed<'a>(record: &'a ProvenanceRecord) -> Reign<'a> {
        Reign {
            pane_id: record.pane_id,
            started_at: record.started_at,
            source: ReignSource::Completed(record),
        }
    }

    fn live<'a>(pane_id: u32, started_at: u64, ticket_id: &'a str) -> Reign<'a> {
        Reign {
            pane_id,
            started_at,
            source: ReignSource::Live { ticket_id },
        }
    }

    fn owner(outcome: ReignOutcome<'_>) -> Option<&str> {
        match outcome {
            ReignOutcome::Attributed(record) => Some(record.ticket_id.as_str()),
            _ => None,
        }
    }

    #[test]
    fn reign_attributes_post_ended_at_rest_capture_to_the_owner() {
        // A ran [100, 199]; its rest capture lands at 250, after ended_at, before
        // any successor. The reign still belongs to A.
        let a = record("T-A", 1, 7, 100, 199);
        let reigns = [completed(&a)];
        assert_eq!(owner(reign_owner_at(&reigns, 7, 250)), Some("T-A"));
        assert_eq!(owner(reign_owner_at(&reigns, 7, 150)), Some("T-A"));
        assert_eq!(owner(reign_owner_at(&reigns, 7, 100)), Some("T-A"));
    }

    #[test]
    fn reign_splits_a_recycled_pane_by_successor_start() {
        let a = record("T-A", 1, 7, 100, 199);
        let b = record("T-B", 1, 7, 300, 399);
        let reigns = [completed(&a), completed(&b)];
        // A's reign runs until B starts, covering the gap after A's ended_at.
        assert_eq!(owner(reign_owner_at(&reigns, 7, 250)), Some("T-A"));
        assert_eq!(owner(reign_owner_at(&reigns, 7, 299)), Some("T-A"));
        // From B's start onward the pane is B's.
        assert_eq!(owner(reign_owner_at(&reigns, 7, 300)), Some("T-B"));
        assert_eq!(owner(reign_owner_at(&reigns, 7, 450)), Some("T-B"));
    }

    #[test]
    fn reign_defers_to_a_live_successor() {
        // A completed at 199; B is live from 300 with no record yet. A capture in
        // B's live reign must be Pending, not misattributed to A.
        let a = record("T-A", 1, 7, 100, 199);
        let reigns = [completed(&a), live(7, 300, "T-B")];
        assert!(matches!(
            reign_owner_at(&reigns, 7, 350),
            ReignOutcome::Pending
        ));
        // Before B started, the capture is still A's.
        assert_eq!(owner(reign_owner_at(&reigns, 7, 250)), Some("T-A"));
    }

    #[test]
    fn reign_is_unowned_before_any_occupant_and_on_other_panes() {
        let a = record("T-A", 1, 7, 100, 199);
        let reigns = [completed(&a)];
        assert!(matches!(
            reign_owner_at(&reigns, 7, 50),
            ReignOutcome::Unowned
        ));
        assert!(matches!(
            reign_owner_at(&reigns, 8, 150),
            ReignOutcome::Unowned
        ));
    }

    #[test]
    fn reign_accepts_duplicate_identity_retries_but_rejects_conflicting_ties() {
        // Two attempts of the same ticket sharing the winning start → still owned.
        let a1 = record("T-A", 1, 7, 200, 300);
        let a2 = record("T-A", 2, 7, 200, 320);
        let same = [completed(&a1), completed(&a2)];
        assert_eq!(owner(reign_owner_at(&same, 7, 250)), Some("T-A"));

        // Two different tickets tie on the winning start → fail closed.
        let a = record("T-A", 1, 7, 200, 300);
        let b = record("T-B", 1, 7, 200, 300);
        let conflict = [completed(&a), completed(&b)];
        assert!(matches!(
            reign_owner_at(&conflict, 7, 250),
            ReignOutcome::Unowned
        ));
        let reversed = [completed(&b), completed(&a)];
        assert!(matches!(
            reign_owner_at(&reversed, 7, 250),
            ReignOutcome::Unowned
        ));
    }

    #[test]
    fn reign_prefers_a_completed_record_over_the_same_tickets_live_thread() {
        // A resting session: its Done row and its still-live thread share a
        // start. Attribution must resolve to the durable record regardless of
        // reign order, not stall at Pending.
        let a = record("T-A", 1, 7, 100, 199);
        let completed_first = [completed(&a), live(7, 100, "T-A")];
        assert_eq!(owner(reign_owner_at(&completed_first, 7, 250)), Some("T-A"));
        let live_first = [live(7, 100, "T-A"), completed(&a)];
        assert_eq!(owner(reign_owner_at(&live_first, 7, 250)), Some("T-A"));
    }

    #[test]
    fn reign_attribution_carries_the_owning_attempt() {
        let a = record("T-A", 5, 7, 100, 199);
        let reigns = [completed(&a)];
        match reign_owner_at(&reigns, 7, 150) {
            ReignOutcome::Attributed(record) => {
                assert_eq!(record.attempt_lease.attempt_id, 5);
                assert_eq!(record.ticket_id, "T-A");
            }
            _ => panic!("expected attribution"),
        }
    }
}
