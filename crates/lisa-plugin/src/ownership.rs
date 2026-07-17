//! Pane-time ownership lookup over durable scheduler provenance.
//!
//! Terminal execution records retain the physical pane, ticket, and closed
//! start/end window after a thread leaves scheduler memory. Capture attribution
//! can combine persisted records with the current record being completed and
//! resolve ownership without trusting inherited environment or pane residency.

use lisa_core::provenance::ProvenanceRecord;

/// Return the unique ticket that owned `pane_id` at `captured_at`.
///
/// Provenance timestamps have epoch-second resolution, so both interval
/// endpoints are inclusive. Repeated matching records for the same ticket keep
/// the same answer. If different tickets overlap at the requested pane-time,
/// the evidence is ambiguous and the lookup fails closed with `None` rather
/// than depending on record order.
pub(crate) fn owner_at<'a>(
    records: impl IntoIterator<Item = &'a ProvenanceRecord>,
    pane_id: u32,
    captured_at: u64,
) -> Option<&'a str> {
    let mut owner = None;

    for record in records {
        if record.pane_id != pane_id
            || captured_at < record.started_at
            || captured_at > record.ended_at
        {
            continue;
        }

        match owner {
            None => owner = Some(record.ticket_id.as_str()),
            Some(ticket_id) if ticket_id == record.ticket_id => {}
            Some(_) => return None,
        }
    }

    owner
}

#[cfg(test)]
mod tests {
    use lisa_core::completion::CompletionSeal;
    use lisa_core::provenance::{ProvenanceRecord, Route, RunOutcome, SCHEMA_VERSION};
    use lisa_core::types::AttemptLease;

    use super::owner_at;

    fn interval(
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

    #[test]
    fn owner_at_resolves_each_ticket_window_on_a_recycled_pane() {
        let records = [
            interval("T-A", 1, 7, 100, 199),
            interval("T-B", 1, 7, 300, 399),
        ];

        assert_eq!(owner_at(&records, 7, 150), Some("T-A"));
        assert_eq!(owner_at(&records, 7, 350), Some("T-B"));

        assert_eq!(owner_at(&records, 7, 100), Some("T-A"));
        assert_eq!(owner_at(&records, 7, 199), Some("T-A"));
        assert_eq!(owner_at(&records, 7, 300), Some("T-B"));
        assert_eq!(owner_at(&records, 7, 399), Some("T-B"));

        assert_eq!(owner_at(&records, 7, 50), None);
        assert_eq!(owner_at(&records, 7, 250), None);
        assert_eq!(owner_at(&records, 7, 450), None);
        assert_eq!(owner_at(&records, 8, 150), None);
    }

    #[test]
    fn owner_at_accepts_duplicate_identity_but_rejects_conflicting_overlap() {
        let duplicate_owner = [
            interval("T-A", 1, 7, 100, 200),
            interval("T-A", 2, 7, 150, 250),
        ];
        assert_eq!(owner_at(&duplicate_owner, 7, 175), Some("T-A"));

        let conflicting_owner = [
            interval("T-A", 1, 7, 100, 200),
            interval("T-B", 1, 7, 150, 250),
        ];
        assert_eq!(owner_at(&conflicting_owner, 7, 175), None);
        assert_eq!(owner_at(conflicting_owner.iter().rev(), 7, 175), None);
    }
}
