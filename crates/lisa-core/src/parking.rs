//! Discovery of durable parked Review remedies.
//!
//! Ticket status remains scheduling authority. The canonical Review
//! disposition carries the human ask and optional verification check after the
//! scheduler has released the attempt that produced it.

use std::path::Path;

use crate::disposition::{parse_review_disposition, RemedyOwner, ReviewDisposition};
use crate::triage::{read_stored_proposal, ProposalState, TriageProposal, TRIAGE_PROPOSAL_FILE};
use crate::types::{Ticket, TicketStatus};

/// Plain lead shown when an older block has no structured human ask.
pub const LEGACY_BLOCK_ASK: &str = "This ticket needs a decision from you. The reviewer's note is below — you can paste it to your coding agent.";

/// Correction shown when a block ask describes state but gives no human move.
pub const BLOCK_ASK_ACTION_FIX: &str =
    "lead ask with a plain action, for example: Run the test. Choose an option. Publish the release.";

/// Correction shown when technical detail displaces the block ask's plain lead.
pub const BLOCK_ASK_LEAD_FIX: &str = "make ask's first sentence one short line (160 characters or fewer); move technical detail to reason or steps";

const MAX_BLOCK_ASK_LEAD_CHARS: usize = 160;

/// Enforce the minimum authoring floor for a block's operator-facing ask.
///
/// Rendering still supplies [`LEGACY_BLOCK_ASK`] for unchecked historical
/// blocks. This check is for newly authored structured blocks: their first
/// sentence must stay short, single-line, and contain an action a bystander can
/// take. Follow-up context may remain after that first sentence.
pub fn validate_block_ask(ask: &str) -> Result<(), &'static str> {
    let trimmed = ask.trim();
    if trimmed.is_empty() || trimmed.contains(['\n', '\r']) {
        return Err(BLOCK_ASK_LEAD_FIX);
    }

    let lead_end = trimmed
        .char_indices()
        .find_map(|(index, character)| {
            matches!(character, '.' | '!' | '?').then_some(index + character.len_utf8())
        })
        .unwrap_or(trimmed.len());
    let lead = &trimmed[..lead_end];
    if lead.chars().count() > MAX_BLOCK_ASK_LEAD_CHARS {
        return Err(BLOCK_ASK_LEAD_FIX);
    }

    const ACTION_WORDS: &[&str] = &[
        "add", "amend", "apply", "approve", "attach", "choose", "complete", "confirm", "contact",
        "create", "decide", "deploy", "fix", "grant", "install", "open", "provide", "publish",
        "record", "remove", "replace", "rerun", "retry", "review", "run", "select", "send", "set",
        "share", "sign", "update", "upload", "wait",
    ];
    let has_action = lead
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .filter(|word| !word.is_empty())
        .any(|word| {
            ACTION_WORDS
                .iter()
                .any(|action| word.eq_ignore_ascii_case(action))
        });
    if !has_action {
        return Err(BLOCK_ASK_ACTION_FIX);
    }

    Ok(())
}

/// The small remedy projection needed by status, dashboard, and unblock UX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParkedRemedy {
    pub ticket_id: String,
    pub remedy_owner: RemedyOwner,
    pub ask: String,
    pub reason: String,
    pub check: Option<String>,
    pub proposal: Option<TriageProposal>,
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
                reason,
                remedy_owner,
                ask,
                check,
                unstructured,
                ..
            } = disposition
            else {
                return None;
            };
            let ask = if unstructured {
                LEGACY_BLOCK_ASK.to_string()
            } else {
                ask
            };
            let proposal =
                read_stored_proposal(&work_dir.join(&ticket.id).join(TRIAGE_PROPOSAL_FILE))
                    .ok()
                    .flatten()
                    .filter(|stored| {
                        stored.ticket_id == ticket.id && stored.state == ProposalState::Pending
                    })
                    .map(|stored| stored.proposal);
            Some(ParkedRemedy {
                ticket_id: ticket.id.clone(),
                remedy_owner,
                ask,
                reason,
                check,
                proposal,
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

    const FIELD_ASK: &str = "The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant bypassed the old binary through managed mode instead of recording the required recovery through Lisa's error strings; John must either provide conforming reruns or explicitly amend both acceptance requirements before Review can pass.";

    #[test]
    fn ask_floor_strings_are_one_shared_pinned_source() {
        assert_eq!(
            LEGACY_BLOCK_ASK,
            "This ticket needs a decision from you. The reviewer's note is below — you can paste it to your coding agent."
        );
        assert_eq!(
            BLOCK_ASK_ACTION_FIX,
            "lead ask with a plain action, for example: Run the test. Choose an option. Publish the release."
        );
        assert_eq!(
            BLOCK_ASK_LEAD_FIX,
            "make ask's first sentence one short line (160 characters or fewer); move technical detail to reason or steps"
        );
    }

    #[test]
    fn ask_floor_accepts_plain_actions_and_the_workflow_example() {
        for ask in [
            "Run the checkout test.",
            "Choose whether to amend the criterion.",
            "Publish the release.",
            "Wait for the release link.",
            "Contact the repository owner.",
            "Lisa needs the release published; run: just release. Lisa will notice on its own once it's live.",
        ] {
            assert_eq!(validate_block_ask(ask), Ok(()), "rejected {ask:?}");
        }
    }

    #[test]
    fn ask_floor_rejects_observations_multiline_text_and_field_jargon_wall() {
        assert_eq!(
            validate_block_ask("The release artifact is unavailable."),
            Err(BLOCK_ASK_ACTION_FIX)
        );
        assert_eq!(
            validate_block_ask("Run the test.\nThen inspect the output."),
            Err(BLOCK_ASK_LEAD_FIX)
        );
        assert_eq!(validate_block_ask(FIELD_ASK), Err(BLOCK_ASK_LEAD_FIX));
    }

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
                    reason: "manual test missing".to_string(),
                    check: None,
                    proposal: None,
                },
                ParkedRemedy {
                    ticket_id: "T-002".to_string(),
                    remedy_owner: RemedyOwner::World,
                    ask: "Wait for the release link.".to_string(),
                    reason: "release missing".to_string(),
                    check: Some("test -f release".to_string()),
                    proposal: None,
                },
            ]
        );
    }

    #[test]
    fn legacy_block_gets_a_plain_ask_and_preserves_the_raw_reason() {
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
                ask: LEGACY_BLOCK_ASK.to_string(),
                reason: "  Run the old test.  ".to_string(),
                check: None,
                proposal: None,
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

    #[test]
    fn projects_only_a_matching_pending_proposal() {
        use crate::triage::{
            write_stored_proposal, PreparedStep, ProposalState, StoredTriageProposal,
            TriageProposal,
        };
        use crate::types::AttemptLease;

        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        write_disposition(
            &work,
            "T-PROPOSAL",
            r#"{"disposition":"block","reason":"criteria mismatch","remedy_owner":"operator","ask":"Choose an amendment."}"#,
        );
        let proposal = TriageProposal {
            summary: "The criterion conflicts with its cited evidence.".to_string(),
            recommendation: "Amend the stale criterion.".to_string(),
            prepared_steps: vec![PreparedStep::Command {
                description: "Apply the prepared amendment.".to_string(),
                command: "git apply amendment.patch".to_string(),
            }],
        };
        let path = work.join("T-PROPOSAL").join(TRIAGE_PROPOSAL_FILE);
        let mut stored = StoredTriageProposal {
            ticket_id: "T-PROPOSAL".to_string(),
            source_attempt_lease: AttemptLease {
                ticket_id: "T-PROPOSAL".to_string(),
                attempt_id: 3,
            },
            state: ProposalState::Pending,
            proposal: proposal.clone(),
        };
        write_stored_proposal(&path, &stored).unwrap();

        let remedies =
            collect_parked_remedies([&ticket("T-PROPOSAL", TicketStatus::Blocked)], &work);
        assert_eq!(remedies[0].proposal, Some(proposal));

        stored.state = ProposalState::Dismissed;
        write_stored_proposal(&path, &stored).unwrap();
        let remedies =
            collect_parked_remedies([&ticket("T-PROPOSAL", TicketStatus::Blocked)], &work);
        assert_eq!(remedies[0].proposal, None);
    }
}
