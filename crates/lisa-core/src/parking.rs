//! Discovery of durable parked Review remedies.
//!
//! Ticket status remains scheduling authority. The canonical Review
//! disposition carries the human ask and optional verification check after the
//! scheduler has released the attempt that produced it.

use std::path::Path;

use crate::disposition::{parse_review_disposition, RemedyOwner, ReviewDisposition};
use crate::types::{Ticket, TicketStatus};

/// The small remedy projection needed by status, dashboard, and unblock UX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParkedRemedy {
    pub ticket_id: String,
    pub remedy_owner: RemedyOwner,
    pub ask: String,
    pub check: Option<String>,
}

/// Collect valid canonical remedies for tickets durably parked by status.
///
/// Missing, invalid, or passing dispositions do not manufacture remedy data.
/// The ticket remains visible as blocked through ordinary board rendering.
pub fn collect_parked_remedies<'a>(
    tickets: impl IntoIterator<Item = &'a Ticket>,
    work_dir: &Path,
) -> Vec<ParkedRemedy> {
    let mut remedies: Vec<_> = tickets
        .into_iter()
        .filter(|ticket| ticket.status == TicketStatus::Blocked)
        .filter_map(|ticket| {
            let disposition =
                parse_review_disposition(work_dir.join(&ticket.id).join("review-disposition.json"));
            let ReviewDisposition::Block {
                remedy_owner,
                ask,
                check,
                ..
            } = disposition
            else {
                return None;
            };
            Some(ParkedRemedy {
                ticket_id: ticket.id.clone(),
                remedy_owner,
                ask,
                check,
            })
        })
        .collect();
    remedies.sort_by(|left, right| left.ticket_id.cmp(&right.ticket_id));
    remedies
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn ticket(id: &str, status: TicketStatus) -> Ticket {
        let mut ticket = Ticket::new(id, "fixture");
        ticket.status = status;
        ticket
    }

    fn write_disposition(work_dir: &Path, ticket_id: &str, document: &str) {
        let ticket_work = work_dir.join(ticket_id);
        fs::create_dir_all(&ticket_work).unwrap();
        fs::write(ticket_work.join("review-disposition.json"), document).unwrap();
    }

    #[test]
    fn collects_structured_operator_and_world_remedies_in_ticket_order() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        write_disposition(
            &work,
            "T-002",
            r#"{"disposition":"block","reason":"release missing","remedy_owner":"world","ask":"Wait for the release link.","check":"test -f release"}"#,
        );
        write_disposition(
            &work,
            "T-001",
            r#"{"disposition":"block","reason":"manual test missing","remedy_owner":"operator","ask":"Run the checkout test.","steps":["Open checkout"]}"#,
        );
        let tickets = vec![
            ticket("T-002", TicketStatus::Blocked),
            ticket("T-001", TicketStatus::Blocked),
        ];

        assert_eq!(
            collect_parked_remedies(&tickets, &work),
            vec![
                ParkedRemedy {
                    ticket_id: "T-001".to_string(),
                    remedy_owner: RemedyOwner::Operator,
                    ask: "Run the checkout test.".to_string(),
                    check: None,
                },
                ParkedRemedy {
                    ticket_id: "T-002".to_string(),
                    remedy_owner: RemedyOwner::World,
                    ask: "Wait for the release link.".to_string(),
                    check: Some("test -f release".to_string()),
                },
            ]
        );
    }

    #[test]
    fn legacy_block_uses_the_parser_operator_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        write_disposition(
            &work,
            "T-LEGACY",
            r#"{"disposition":"block","reason":"  Run the old test.  "}"#,
        );
        let tickets = vec![ticket("T-LEGACY", TicketStatus::Blocked)];

        assert_eq!(
            collect_parked_remedies(&tickets, &work),
            vec![ParkedRemedy {
                ticket_id: "T-LEGACY".to_string(),
                remedy_owner: RemedyOwner::Operator,
                ask: "  Run the old test.  ".to_string(),
                check: None,
            }]
        );
    }

    #[test]
    fn excludes_open_tickets_and_non_block_dispositions() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        write_disposition(
            &work,
            "T-OPEN",
            r#"{"disposition":"block","reason":"hidden","remedy_owner":"operator","ask":"Do not show this."}"#,
        );
        write_disposition(&work, "T-PASS", r#"{"disposition":"pass","reason":null}"#);
        write_disposition(&work, "T-BAD", "not json");
        let tickets = vec![
            ticket("T-OPEN", TicketStatus::Open),
            ticket("T-PASS", TicketStatus::Blocked),
            ticket("T-BAD", TicketStatus::Blocked),
            ticket("T-MISSING", TicketStatus::Blocked),
        ];

        assert!(collect_parked_remedies(&tickets, &work).is_empty());
    }
}
