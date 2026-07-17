//! Explicit operator disposition of a pending first-responder proposal.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use lisa_core::disposition::{parse_review_disposition, RemedyOwner, ReviewDisposition};
use lisa_core::parking::latest_park_attempt_leases;
use lisa_core::provenance::{
    append_proposal_action_record, system_time_to_epoch, ProposalAction, ProposalActionRecord,
    ProposalRecordType, SCHEMA_VERSION,
};
use lisa_core::ticket;
use lisa_core::triage::{
    read_stored_proposal, write_stored_proposal, PreparedStep, ProposalState, TRIAGE_PROPOSAL_FILE,
};
use lisa_core::types::TicketStatus;

use crate::completion_seal;
use crate::config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorProposalAction {
    Apply,
    Dismiss,
}

const APPLY_STEP_PREFIX: &str = "Applying proposal step:";

#[derive(Debug, PartialEq, Eq)]
struct ApplyFailure {
    applied_steps: Vec<String>,
    failed_step: String,
    reason: String,
}

pub fn run_proposal_action(
    root: &Path,
    ticket_id: &str,
    action: OperatorProposalAction,
) -> Result<String, String> {
    let stdout = io::stdout();
    run_proposal_action_with_writer(root, ticket_id, action, &mut stdout.lock())
}

fn run_proposal_action_with_writer(
    root: &Path,
    ticket_id: &str,
    action: OperatorProposalAction,
    output: &mut impl Write,
) -> Result<String, String> {
    let validation = config::load_config(root)?;
    let resolved = config::resolve_config(&validation.config, None, None);
    let tickets = ticket::scan_tickets(root.join(&resolved.ticket_dir))
        .map_err(|error| format!("Could not read the ticket board: {error}"))?;
    let ticket = tickets
        .iter()
        .find(|ticket| ticket.id == ticket_id)
        .ok_or_else(|| format!("I couldn't find {ticket_id}."))?;
    if ticket.status != TicketStatus::Blocked {
        return Err(format!("{ticket_id} isn't waiting."));
    }

    let ticket_work = root.join(&resolved.work_dir).join(ticket_id);
    let disposition = parse_review_disposition(ticket_work.join("review-disposition.json"));
    if !matches!(
        disposition,
        ReviewDisposition::Block {
            remedy_owner: RemedyOwner::Operator,
            ..
        }
    ) {
        return Err(format!(
            "{ticket_id} has no operator-owned proposal to settle."
        ));
    }
    let proposal_path = ticket_work.join(TRIAGE_PROPOSAL_FILE);
    let mut stored = read_stored_proposal(&proposal_path)?
        .ok_or_else(|| format!("{ticket_id} has no proposal."))?;
    let provenance_path = root.join(".lisa/provenance.jsonl");
    if stored.ticket_id != ticket_id
        || stored.state != ProposalState::Pending
        || latest_park_attempt_leases(&provenance_path).get(ticket_id)
            != Some(&stored.source_attempt_lease)
    {
        return Err(format!("{ticket_id} has no pending proposal."));
    }

    match action {
        OperatorProposalAction::Apply => {
            let attempt = proposal_action_record(
                root,
                resolved.completion_mode,
                ticket_id,
                &stored,
                ProposalAction::Attempted,
                Some(stored.proposal.clone()),
                Some(stored.proposal.prepared_steps.len()),
                Vec::new(),
                None,
                None,
            );
            append_proposal_action_record(&provenance_path, &attempt)
                .map_err(|error| format!("Could not record the operator apply attempt: {error}"))?;

            match apply_steps(root, &stored.proposal.prepared_steps, output) {
                Ok(applied_steps) => {
                    stored.state = ProposalState::Applied;
                    let outcome = proposal_action_record(
                        root,
                        resolved.completion_mode,
                        ticket_id,
                        &stored,
                        ProposalAction::Applied,
                        None,
                        None,
                        applied_steps,
                        None,
                        None,
                    );
                    persist_apply_outcome(&proposal_path, &stored, &provenance_path, &outcome)?;
                    ticket::update_ticket_status(&ticket.file_path, TicketStatus::Open)
                        .map_err(|error| format!("Could not reopen {ticket_id}: {error}"))?;
                    Ok(format!("Applied the proposal; {ticket_id} can run again."))
                }
                Err(failure) => {
                    stored.state = ProposalState::Failed;
                    let outcome = proposal_action_record(
                        root,
                        resolved.completion_mode,
                        ticket_id,
                        &stored,
                        ProposalAction::Failed,
                        None,
                        None,
                        failure.applied_steps,
                        Some(failure.failed_step),
                        Some(failure.reason.clone()),
                    );
                    match persist_apply_outcome(&proposal_path, &stored, &provenance_path, &outcome)
                    {
                        Ok(()) => Err(failure.reason),
                        Err(record_error) => Err(format!("{} {record_error}", failure.reason)),
                    }
                }
            }
        }
        OperatorProposalAction::Dismiss => {
            stored.state = ProposalState::Dismissed;
            let record = proposal_action_record(
                root,
                resolved.completion_mode,
                ticket_id,
                &stored,
                ProposalAction::Dismissed,
                None,
                None,
                Vec::new(),
                None,
                None,
            );
            append_proposal_action_record(&provenance_path, &record)
                .map_err(|error| format!("Could not record the operator action: {error}"))?;
            write_stored_proposal(&proposal_path, &stored)?;
            Ok(format!(
                "Dismissed the proposal; {ticket_id} remains waiting on the original review."
            ))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn proposal_action_record(
    root: &Path,
    completion_mode: lisa_core::completion::CompletionSealMode,
    ticket_id: &str,
    stored: &lisa_core::triage::StoredTriageProposal,
    action: ProposalAction,
    proposal: Option<lisa_core::triage::TriageProposal>,
    step_count: Option<usize>,
    applied_steps: Vec<String>,
    failed_step: Option<String>,
    failure_reason: Option<String>,
) -> ProposalActionRecord {
    ProposalActionRecord {
        schema_version: SCHEMA_VERSION,
        seal: completion_seal::resolve_for_inspection(root, completion_mode),
        record_type: ProposalRecordType::ProposalAction,
        ticket_id: ticket_id.to_string(),
        source_attempt_lease: stored.source_attempt_lease.clone(),
        action,
        actor: "operator".to_string(),
        proposal,
        step_count,
        applied_steps,
        failed_step,
        failure_reason,
        occurred_at: system_time_to_epoch(std::time::SystemTime::now()),
    }
}

fn persist_apply_outcome(
    proposal_path: &Path,
    stored: &lisa_core::triage::StoredTriageProposal,
    provenance_path: &Path,
    outcome: &ProposalActionRecord,
) -> Result<(), String> {
    let sidecar_result = write_stored_proposal(proposal_path, stored);
    let provenance_result = append_proposal_action_record(provenance_path, outcome)
        .map_err(|error| format!("Could not record the operator apply outcome: {error}"));
    match (sidecar_result, provenance_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(sidecar), Ok(())) => Err(format!("Could not update proposal state: {sidecar}")),
        (Ok(()), Err(provenance)) => Err(provenance),
        (Err(sidecar), Err(provenance)) => Err(format!(
            "Could not update proposal state: {sidecar} {provenance}"
        )),
    }
}

fn apply_steps(
    root: &Path,
    steps: &[PreparedStep],
    output: &mut impl Write,
) -> Result<Vec<String>, ApplyFailure> {
    let mut applied_steps = Vec::new();
    for step in steps {
        let failed_step = step.description().to_string();
        if let Err(error) = writeln!(output, "{APPLY_STEP_PREFIX} {failed_step}") {
            return Err(ApplyFailure {
                applied_steps,
                failed_step,
                reason: format!("Could not announce prepared step: {error}"),
            });
        }
        if let Err(error) = output.flush() {
            return Err(ApplyFailure {
                applied_steps,
                failed_step,
                reason: format!("Could not announce prepared step: {error}"),
            });
        }

        let result: Result<(), String> = (|| match step {
            PreparedStep::Command { command, .. } => {
                let status = Command::new("/bin/sh")
                    .args(["-c", command])
                    .current_dir(root)
                    .status()
                    .map_err(|error| format!("Could not run prepared command: {error}"))?;
                if !status.success() {
                    Err(format!("Prepared command failed with {status}: {command}"))
                } else {
                    Ok(())
                }
            }
            PreparedStep::FileEdit { path, old, new, .. } => {
                let destination = root.join(path);
                let body = fs::read_to_string(&destination)
                    .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
                let matches = body.matches(old).count();
                if matches != 1 {
                    return Err(format!(
                        "Prepared edit for {} expected its old text exactly once; found {matches}.",
                        path.display()
                    ));
                }
                let updated = body.replacen(old, new, 1);
                atomic_write(&destination, updated.as_bytes())
            }
        })();
        if let Err(reason) = result {
            return Err(ApplyFailure {
                applied_steps,
                failed_step,
                reason,
            });
        }
        applied_steps.push(step.description().to_string());
    }
    Ok(applied_steps)
}

fn atomic_write(path: &Path, body: &[u8]) -> Result<(), String> {
    let temporary = PathBuf::from(format!(
        "{}.proposal-tmp-{}",
        path.display(),
        std::process::id()
    ));
    fs::write(&temporary, body)
        .map_err(|error| format!("Could not write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("Could not publish {}: {error}", path.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lisa_core::completion::CompletionSeal;
    use lisa_core::disposition::RemedyOwner;
    use lisa_core::provenance::{
        append_parking_transition_record, ParkingTransitionRecord, ParkingTransitionType,
        ProvenanceLedgerRecord,
    };
    use lisa_core::triage::{StoredTriageProposal, TriageProposal};
    use lisa_core::types::AttemptLease;

    const TICKET_ID: &str = "T-046-06-03";

    fn append_park(root: &Path, attempt_id: u64) {
        append_parking_transition_record(
            &root.join(".lisa/provenance.jsonl"),
            &ParkingTransitionRecord {
                schema_version: SCHEMA_VERSION,
                seal: CompletionSeal::Commit,
                record_type: ParkingTransitionType::Park,
                ticket_id: TICKET_ID.to_string(),
                attempt_lease: AttemptLease {
                    ticket_id: TICKET_ID.to_string(),
                    attempt_id,
                },
                remedy_owner: RemedyOwner::Operator,
                retry_count: None,
                retry_limit: None,
                recheck_eligible: false,
                started_at: attempt_id,
                ended_at: attempt_id,
                wall_clock_secs: 0,
            },
        )
        .unwrap();
    }

    fn proposal_records(root: &Path) -> Vec<ProposalActionRecord> {
        fs::read_to_string(root.join(".lisa/provenance.jsonl"))
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<ProvenanceLedgerRecord>(line).ok())
            .filter_map(|record| match record {
                ProvenanceLedgerRecord::ProposalAction(record) => Some(record),
                _ => None,
            })
            .collect()
    }

    fn setup(action_state: ProposalState) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work/T-046-06-03")).unwrap();
        fs::write(
            dir.path().join("docs/active/tickets/T-046-06-03.md"),
            "---\nid: T-046-06-03\ntitle: field\ntype: task\nstatus: blocked\npriority: high\nphase: review\n---\n\nGate: approximately 200 MiB.\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("docs/active/work/T-046-06-03/review-disposition.json"),
            r#"{"disposition":"block","reason":"criteria versus evidence","remedy_owner":"operator","ask":"Choose the amendment."}"#,
        )
        .unwrap();
        let stored = StoredTriageProposal {
            ticket_id: "T-046-06-03".to_string(),
            source_attempt_lease: AttemptLease {
                ticket_id: "T-046-06-03".to_string(),
                attempt_id: 1,
            },
            state: action_state,
            proposal: TriageProposal {
                summary: "The written criterion conflicts with the measured evidence.".to_string(),
                recommendation: "Amend the stale criterion.".to_string(),
                prepared_steps: vec![PreparedStep::FileEdit {
                    description: "Use the calibrated bound.".to_string(),
                    path: PathBuf::from("docs/active/tickets/T-046-06-03.md"),
                    old: "approximately 200 MiB".to_string(),
                    new: "the calibrated 300 MiB bound".to_string(),
                }],
            },
        };
        write_stored_proposal(
            &dir.path()
                .join("docs/active/work/T-046-06-03/triage-proposal.json"),
            &stored,
        )
        .unwrap();
        append_park(dir.path(), 1);
        dir
    }

    #[test]
    fn apply_executes_prepared_edit_records_and_reopens() {
        let dir = setup(ProposalState::Pending);
        let mut output = Vec::new();
        let message = run_proposal_action_with_writer(
            dir.path(),
            TICKET_ID,
            OperatorProposalAction::Apply,
            &mut output,
        )
        .unwrap();
        assert!(message.contains("can run again"));
        let ticket =
            fs::read_to_string(dir.path().join("docs/active/tickets/T-046-06-03.md")).unwrap();
        assert!(ticket.contains("status: open"));
        assert!(ticket.contains("calibrated 300 MiB"));
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Applying proposal step: Use the calibrated bound.\n"
        );
        let records = proposal_records(dir.path());
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].action, ProposalAction::Attempted);
        assert_eq!(records[0].actor, "operator");
        assert_eq!(records[0].step_count, Some(1));
        assert!(records[0].proposal.is_some());
        assert_eq!(records[1].action, ProposalAction::Applied);
        assert_eq!(records[1].applied_steps, vec!["Use the calibrated bound."]);
        let stored = read_stored_proposal(
            &dir.path()
                .join("docs/active/work/T-046-06-03/triage-proposal.json"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(stored.state, ProposalState::Applied);
    }

    #[test]
    fn mid_list_failure_records_landed_and_failed_steps_and_leaves_failed_sidecar() {
        let dir = setup(ProposalState::Pending);
        fs::write(dir.path().join("prepared.txt"), "original").unwrap();
        let proposal_path = dir
            .path()
            .join("docs/active/work/T-046-06-03/triage-proposal.json");
        let mut stored = read_stored_proposal(&proposal_path).unwrap().unwrap();
        stored.proposal.prepared_steps = vec![
            PreparedStep::Command {
                description: "Invalidate the prepared edit.".to_string(),
                command: "printf changed > prepared.txt".to_string(),
            },
            PreparedStep::FileEdit {
                description: "Apply the now-stale edit.".to_string(),
                path: PathBuf::from("prepared.txt"),
                old: "original".to_string(),
                new: "updated".to_string(),
            },
        ];
        write_stored_proposal(&proposal_path, &stored).unwrap();
        let mut output = Vec::new();

        let error = run_proposal_action_with_writer(
            dir.path(),
            TICKET_ID,
            OperatorProposalAction::Apply,
            &mut output,
        )
        .unwrap_err();

        assert!(error.contains("expected its old text exactly once; found 0"));
        assert_eq!(
            fs::read_to_string(dir.path().join("prepared.txt")).unwrap(),
            "changed"
        );
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Applying proposal step: Invalidate the prepared edit.\nApplying proposal step: Apply the now-stale edit.\n"
        );
        let records = proposal_records(dir.path());
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].action, ProposalAction::Attempted);
        assert_eq!(records[0].step_count, Some(2));
        assert_eq!(records[1].action, ProposalAction::Failed);
        assert_eq!(
            records[1].applied_steps,
            vec!["Invalidate the prepared edit."]
        );
        assert_eq!(
            records[1].failed_step.as_deref(),
            Some("Apply the now-stale edit.")
        );
        assert!(records[1]
            .failure_reason
            .as_deref()
            .unwrap()
            .contains("found 0"));
        let stored = read_stored_proposal(&proposal_path).unwrap().unwrap();
        assert_eq!(stored.state, ProposalState::Failed);
        let ticket =
            fs::read_to_string(dir.path().join("docs/active/tickets/T-046-06-03.md")).unwrap();
        assert!(ticket.contains("status: blocked"));
    }

    #[test]
    fn stale_proposal_lease_is_rejected_before_recording_or_mutation() {
        let dir = setup(ProposalState::Pending);
        append_park(dir.path(), 2);

        let error =
            run_proposal_action(dir.path(), TICKET_ID, OperatorProposalAction::Apply).unwrap_err();

        assert_eq!(error, "T-046-06-03 has no pending proposal.");
        assert!(proposal_records(dir.path()).is_empty());
        let ticket =
            fs::read_to_string(dir.path().join("docs/active/tickets/T-046-06-03.md")).unwrap();
        assert!(ticket.contains("approximately 200 MiB"));
    }

    #[test]
    fn dismiss_records_and_keeps_original_park() {
        let dir = setup(ProposalState::Pending);
        run_proposal_action(dir.path(), TICKET_ID, OperatorProposalAction::Dismiss).unwrap();
        let ticket =
            fs::read_to_string(dir.path().join("docs/active/tickets/T-046-06-03.md")).unwrap();
        assert!(ticket.contains("status: blocked"));
        let stored = read_stored_proposal(
            &dir.path()
                .join("docs/active/work/T-046-06-03/triage-proposal.json"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(stored.state, ProposalState::Dismissed);
        let ledger = fs::read_to_string(dir.path().join(".lisa/provenance.jsonl")).unwrap();
        assert!(ledger.contains("\"action\":\"dismissed\""));
    }
}
