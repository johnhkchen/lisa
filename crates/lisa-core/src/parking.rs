//! Discovery of durable parked Review remedies.
//!
//! Ticket status remains scheduling authority. The canonical Review
//! disposition carries the human ask and optional verification check after the
//! scheduler has released the attempt that produced it.

use std::collections::HashMap;
use std::path::Path;

use crate::disposition::{parse_review_disposition, RemedyOwner, ReviewDisposition};
use crate::provenance::{ParkingTransitionRecord, ParkingTransitionType, ProvenanceLedgerRecord};
use crate::triage::{read_stored_proposal, ProposalState, TriageProposal, TRIAGE_PROPOSAL_FILE};
use crate::types::{AttemptLease, Ticket, TicketStatus};

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
    /// The block's own prepared steps, empty when it supplied none. "No steps"
    /// and "an empty step list" are the same fact to a reader, so the option is
    /// flattened here rather than carried forward.
    pub steps: Vec<String>,
    pub check: Option<String>,
    pub proposal: Option<TriageProposal>,
}

/// Read the latest current Park record per ticket from the mixed provenance ledger.
///
/// One walk serves every current-park projection. Park inserts, Unpark removes:
/// a ticket that has been released is not currently parked, whatever earlier
/// rows say. Missing or malformed ledger data simply cannot establish a park.
fn latest_park_records(ledger_path: &Path) -> HashMap<String, ParkingTransitionRecord> {
    let Ok(ledger) = std::fs::read_to_string(ledger_path) else {
        return HashMap::new();
    };
    let mut latest = HashMap::new();
    for record in ledger
        .lines()
        .filter_map(|line| serde_json::from_str::<ProvenanceLedgerRecord>(line).ok())
    {
        if let ProvenanceLedgerRecord::ParkingTransition(record) = record {
            if record.record_type == ParkingTransitionType::Park {
                latest.insert(record.ticket_id.clone(), record);
            } else {
                latest.remove(&record.ticket_id);
            }
        }
    }
    latest
}

/// Read the latest current Park lease per ticket from the mixed provenance ledger.
///
/// This is an observational correlation aid, not scheduling authority. Missing or
/// malformed ledger data simply cannot establish a current Park lease.
pub fn latest_park_attempt_leases(ledger_path: &Path) -> HashMap<String, AttemptLease> {
    latest_park_records(ledger_path)
        .into_iter()
        .map(|(ticket_id, record)| (ticket_id, record.attempt_lease))
        .collect()
}

/// Read when each currently parked ticket was parked, in epoch seconds.
///
/// The durable answer to "how long has this been waiting". A ticket parks by
/// having its thread removed, so no in-memory clock survives the transition;
/// this row is the only stamp that does. A ticket whose park row was never
/// written — the ledger refuses rows it cannot attribute to a current attempt —
/// is simply absent, and its age renders as unknown rather than as a guess.
pub fn latest_park_stamps(ledger_path: &Path) -> HashMap<String, u64> {
    latest_park_records(ledger_path)
        .into_iter()
        .map(|(ticket_id, record)| (ticket_id, record.ended_at))
        .collect()
}

/// Collect valid canonical remedies for tickets durably parked by status.
///
/// Missing, invalid, or passing dispositions do not manufacture remedy data.
/// The ticket remains visible as blocked through ordinary board rendering.
pub fn collect_parked_remedies<'a>(
    tickets: impl IntoIterator<Item = &'a Ticket>,
    work_dir: &Path,
    ledger_path: &Path,
) -> Vec<ParkedRemedy> {
    let park_attempts = latest_park_attempt_leases(ledger_path);
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
                steps,
                check,
                unstructured,
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
                        stored.ticket_id == ticket.id
                            && stored.state == ProposalState::Pending
                            && park_attempts.get(&ticket.id) == Some(&stored.source_attempt_lease)
                    })
                    .map(|stored| stored.proposal);
            Some(ParkedRemedy {
                ticket_id: ticket.id.clone(),
                remedy_owner,
                ask,
                reason,
                steps: steps.unwrap_or_default(),
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
    use crate::completion::CompletionSeal;
    use crate::provenance::{
        append_parking_transition_record, ParkingTransitionRecord, SCHEMA_VERSION,
    };

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

    fn write_parking_transition(
        ledger: &Path,
        ticket_id: &str,
        attempt_id: u64,
        record_type: ParkingTransitionType,
    ) {
        append_parking_transition_record(
            ledger,
            &ParkingTransitionRecord {
                schema_version: SCHEMA_VERSION,
                seal: CompletionSeal::Commit,
                record_type,
                ticket_id: ticket_id.to_string(),
                attempt_lease: AttemptLease {
                    ticket_id: ticket_id.to_string(),
                    attempt_id,
                },
                remedy_owner: RemedyOwner::Operator,
                retry_count: None,
                retry_limit: None,
                recheck_eligible: false,
                started_at: 1,
                ended_at: 1,
                wall_clock_secs: 0,
            },
        )
        .unwrap();
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
            collect_parked_remedies(&tickets, &work, &dir.path().join("provenance.jsonl")),
            vec![
                ParkedRemedy {
                    ticket_id: "T-001".to_string(),
                    remedy_owner: RemedyOwner::Operator,
                    ask: "Run the checkout test.".to_string(),
                    reason: "manual test missing".to_string(),
                    steps: vec!["Open checkout".to_string()],
                    check: None,
                    proposal: None,
                },
                ParkedRemedy {
                    ticket_id: "T-002".to_string(),
                    remedy_owner: RemedyOwner::World,
                    ask: "Wait for the release link.".to_string(),
                    reason: "release missing".to_string(),
                    steps: Vec::new(),
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
            collect_parked_remedies(&tickets, &work, &dir.path().join("provenance.jsonl")),
            vec![ParkedRemedy {
                ticket_id: "T-LEGACY".to_string(),
                remedy_owner: RemedyOwner::Operator,
                ask: LEGACY_BLOCK_ASK.to_string(),
                reason: "  Run the old test.  ".to_string(),
                steps: Vec::new(),
                check: None,
                proposal: None,
            }]
        );
    }

    #[test]
    fn a_blocks_prepared_steps_reach_the_projection() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        write_disposition(
            &work,
            "T-STEPS",
            r#"{"disposition":"block","reason":"signing identity missing","remedy_owner":"operator","ask":"Sign into Xcode.","steps":["Open Xcode","Add an Apple ID"],"check":"security find-identity -p codesigning"}"#,
        );
        let ledger = dir.path().join("provenance.jsonl");

        let remedies =
            collect_parked_remedies([&ticket("T-STEPS", TicketStatus::Blocked)], &work, &ledger);

        assert_eq!(remedies[0].steps, vec!["Open Xcode", "Add an Apple ID"]);
        assert_eq!(
            remedies[0].check.as_deref(),
            Some("security find-identity -p codesigning")
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

        assert!(
            collect_parked_remedies(&tickets, &work, &dir.path().join("provenance.jsonl"))
                .is_empty()
        );
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
        let ledger = dir.path().join("provenance.jsonl");
        write_parking_transition(&ledger, "T-PROPOSAL", 3, ParkingTransitionType::Park);

        let remedies = collect_parked_remedies(
            [&ticket("T-PROPOSAL", TicketStatus::Blocked)],
            &work,
            &ledger,
        );
        assert_eq!(remedies[0].proposal, Some(proposal));

        stored.source_attempt_lease.attempt_id = 2;
        write_stored_proposal(&path, &stored).unwrap();
        let remedies = collect_parked_remedies(
            [&ticket("T-PROPOSAL", TicketStatus::Blocked)],
            &work,
            &ledger,
        );
        assert_eq!(remedies[0].proposal, None);

        stored.source_attempt_lease.attempt_id = 3;
        stored.state = ProposalState::Dismissed;
        write_stored_proposal(&path, &stored).unwrap();
        let remedies = collect_parked_remedies(
            [&ticket("T-PROPOSAL", TicketStatus::Blocked)],
            &work,
            &ledger,
        );
        assert_eq!(remedies[0].proposal, None);
    }

    #[test]
    fn latest_unpark_removes_the_current_park_lease() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("provenance.jsonl");
        write_parking_transition(&ledger, "T-PROPOSAL", 3, ParkingTransitionType::Park);
        assert_eq!(
            latest_park_attempt_leases(&ledger)["T-PROPOSAL"].attempt_id,
            3
        );

        write_parking_transition(&ledger, "T-PROPOSAL", 3, ParkingTransitionType::Unpark);

        assert!(latest_park_attempt_leases(&ledger).is_empty());
    }

    /// The durable park stamp follows the lease exactly: a ticket that is
    /// currently parked has one, and a released ticket has none to render.
    #[test]
    fn park_stamps_track_the_current_park_and_clear_on_unpark() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("provenance.jsonl");
        assert!(latest_park_stamps(&ledger).is_empty());

        write_parking_transition(&ledger, "T-PARKED", 3, ParkingTransitionType::Park);
        assert_eq!(latest_park_stamps(&ledger).get("T-PARKED"), Some(&1));

        write_parking_transition(&ledger, "T-PARKED", 3, ParkingTransitionType::Unpark);
        assert!(latest_park_stamps(&ledger).is_empty());
    }
}
