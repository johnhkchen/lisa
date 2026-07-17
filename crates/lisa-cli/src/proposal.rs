//! Explicit operator disposition of a pending first-responder proposal.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lisa_core::disposition::{parse_review_disposition, RemedyOwner, ReviewDisposition};
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

pub fn run_proposal_action(
    root: &Path,
    ticket_id: &str,
    action: OperatorProposalAction,
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
    if stored.ticket_id != ticket_id || stored.state != ProposalState::Pending {
        return Err(format!("{ticket_id} has no pending proposal."));
    }

    let provenance_action = match action {
        OperatorProposalAction::Apply => {
            validate_apply(root, &stored.proposal.prepared_steps)?;
            apply_steps(root, &stored.proposal.prepared_steps)?;
            stored.state = ProposalState::Applied;
            ProposalAction::Applied
        }
        OperatorProposalAction::Dismiss => {
            stored.state = ProposalState::Dismissed;
            ProposalAction::Dismissed
        }
    };
    let now = system_time_to_epoch(std::time::SystemTime::now());
    let record = ProposalActionRecord {
        schema_version: SCHEMA_VERSION,
        seal: completion_seal::resolve_for_inspection(root, resolved.completion_mode),
        record_type: ProposalRecordType::ProposalAction,
        ticket_id: ticket_id.to_string(),
        source_attempt_lease: stored.source_attempt_lease.clone(),
        action: provenance_action,
        actor: "operator".to_string(),
        proposal: None,
        occurred_at: now,
    };
    append_proposal_action_record(&root.join(".lisa/provenance.jsonl"), &record)
        .map_err(|error| format!("Could not record the operator action: {error}"))?;
    write_stored_proposal(&proposal_path, &stored)?;

    match action {
        OperatorProposalAction::Apply => {
            ticket::update_ticket_status(&ticket.file_path, TicketStatus::Open)
                .map_err(|error| format!("Could not reopen {ticket_id}: {error}"))?;
            Ok(format!("Applied the proposal; {ticket_id} can run again."))
        }
        OperatorProposalAction::Dismiss => Ok(format!(
            "Dismissed the proposal; {ticket_id} remains waiting on the original review."
        )),
    }
}

fn validate_apply(root: &Path, steps: &[PreparedStep]) -> Result<(), String> {
    for step in steps {
        step.validate()?;
        if let PreparedStep::FileEdit { path, old, .. } = step {
            let body = fs::read_to_string(root.join(path))
                .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
            let matches = body.matches(old).count();
            if matches != 1 {
                return Err(format!(
                    "Prepared edit for {} expected its old text exactly once; found {matches}.",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn apply_steps(root: &Path, steps: &[PreparedStep]) -> Result<(), String> {
    for step in steps {
        match step {
            PreparedStep::Command { command, .. } => {
                let status = Command::new("/bin/sh")
                    .args(["-c", command])
                    .current_dir(root)
                    .status()
                    .map_err(|error| format!("Could not run prepared command: {error}"))?;
                if !status.success() {
                    return Err(format!("Prepared command failed with {status}: {command}"));
                }
            }
            PreparedStep::FileEdit { path, old, new, .. } => {
                let destination = root.join(path);
                let body = fs::read_to_string(&destination)
                    .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
                let updated = body.replacen(old, new, 1);
                atomic_write(&destination, updated.as_bytes())?;
            }
        }
    }
    Ok(())
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
    use lisa_core::triage::{StoredTriageProposal, TriageProposal};
    use lisa_core::types::AttemptLease;

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
        dir
    }

    #[test]
    fn apply_executes_prepared_edit_records_and_reopens() {
        let dir = setup(ProposalState::Pending);
        let message =
            run_proposal_action(dir.path(), "T-046-06-03", OperatorProposalAction::Apply).unwrap();
        assert!(message.contains("can run again"));
        let ticket =
            fs::read_to_string(dir.path().join("docs/active/tickets/T-046-06-03.md")).unwrap();
        assert!(ticket.contains("status: open"));
        assert!(ticket.contains("calibrated 300 MiB"));
        let ledger = fs::read_to_string(dir.path().join(".lisa/provenance.jsonl")).unwrap();
        assert!(ledger.contains("\"action\":\"applied\""));
        assert!(ledger.contains("\"actor\":\"operator\""));
    }

    #[test]
    fn dismiss_records_and_keeps_original_park() {
        let dir = setup(ProposalState::Pending);
        run_proposal_action(dir.path(), "T-046-06-03", OperatorProposalAction::Dismiss).unwrap();
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
