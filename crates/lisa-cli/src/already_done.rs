//! The way out of a completion that will not record itself.
//!
//! When a completion ends `rejected` / `action-required` but the work it was
//! trying to seal is already in history, the board and the journal disagree and
//! nothing on the board can be edited to make them agree. This is the one
//! command that settles it, and the shape of it is deliberate in three ways.
//!
//! **It runs with no loop.** The field escape came *after* killing a loop that
//! would not stop re-attempting; a route that needs a healthy running plugin is
//! not a route out. So the journal transitions are written here rather than
//! requested from the adapter.
//!
//! **It is proved by the key, never by the absence of a diff.** A commit
//! reachable from HEAD carrying this ticket's `Lisa-Completion-Key` is the only
//! accepted evidence. No such commit, no recovery — an operator's word is not
//! a receipt.
//!
//! **It writes a new generation, not a confirmation of the rejected one.** The
//! reducer refuses `CommandSucceeded` from `Rejected`, and a hand-appended row
//! that ignores that has bricked a real board: the plugin's fail-closed load
//! errors and all scheduling stops. Every append here goes through
//! `lisa_core::completion_journal`, which folds the whole history first, so
//! this command fails loudly on a journal it cannot replay instead of growing
//! one nobody can.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lisa_core::completion::{
    completion_key_ticket_prefix, AttemptId, CompletionDeadline, CompletionGenerationId,
    CompletionId, CompletionSeal, CompletionSealReceipt, CompletionState, CorrelationId,
    Retryability,
};
use lisa_core::completion_journal::{
    append_with_seal_using, load, CompletionJournalAggregate, CompletionJournalTransition,
};
use lisa_core::ticket;
use lisa_core::types::{Phase, TicketStatus};

/// Everything the command reads, resolved by the caller.
///
/// Deliberately explicit paths rather than a project root plus config: this
/// module is exported for tests in another crate, and taking the configuration
/// loader with it would drag the whole binary's module tree along.
pub struct AlreadyDoneRequest<'a> {
    /// Repository root the completion commit is searched from.
    pub project_root: &'a Path,
    /// Directory holding the ticket board.
    pub ticket_dir: &'a Path,
    /// The project's completion journal.
    pub journal_path: &'a Path,
}

/// What the command did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlreadyDoneOutcome {
    /// The ticket now reads Done and the journal records it terminal.
    Recovered {
        ticket_id: String,
        commit_id: String,
        /// True when the ticket file on disk had to be rewritten to Done, and
        /// is therefore an uncommitted change the operator still owns.
        ticket_file_rewritten: bool,
    },
    /// Nothing changed, for a reason a person can read.
    Declined(String),
}

/// Finish a ticket whose work is already recorded in history.
pub fn run_already_done(
    request: AlreadyDoneRequest<'_>,
    ticket_id: &str,
) -> Result<AlreadyDoneOutcome, String> {
    let tickets = ticket::scan_tickets(request.ticket_dir)
        .map_err(|error| format!("Could not read the ticket board: {error}"))?;
    let Some(board_ticket) = tickets.iter().find(|listed| listed.id == ticket_id) else {
        return Ok(declined(format!("I couldn't find {ticket_id}.")));
    };

    let aggregates = load(request.journal_path)?;
    let Some(aggregate) = aggregates.get(ticket_id) else {
        return Ok(declined(format!(
            "Lisa has no record of trying to finish {ticket_id}, so there is nothing to settle."
        )));
    };
    let start = match recovery_start(ticket_id, aggregate) {
        Ok(start) => start,
        Err(decline) => return Ok(declined(decline)),
    };

    let completion_id = CompletionId::new(ticket_id);
    let Some(commit_id) = find_sealed_commit(request.project_root, &completion_id)? else {
        return Ok(declined(format!(
            "I can't find {ticket_id}'s finished work in this repository's history. Nothing changed."
        )));
    };

    let (recovery_key, correlation) = match &start {
        RecoveryStart::Fresh => {
            let key = CompletionGenerationId::new(
                completion_id,
                AttemptId::new(OPERATOR_ATTEMPT),
                aggregate.completion_key().generation().saturating_add(1),
            );
            let correlation = CorrelationId::new(key.to_string());
            (key, correlation)
        }
        // An interrupted earlier run left its own generation part-written.
        // Finishing it is the only move: its key already owns the aggregate,
        // and a fresh one would be refused by the fold.
        RecoveryStart::Resume { correlation } => (
            aggregate.completion_key().clone(),
            correlation
                .clone()
                .unwrap_or_else(|| CorrelationId::new(aggregate.completion_key().to_string())),
        ),
    };
    let receipt = CompletionSealReceipt::commit(commit_id.clone())?;
    let prior_phase = aggregate.prior_phase();
    let prior_status = aggregate.prior_status();

    // A new generation is the only legal road from Rejected to Confirmed. Each
    // append re-folds, so an interruption between them leaves a state the
    // plugin can still replay — and one this command can pick back up.
    let pending: &[CompletionJournalTransition] = &[
        CompletionJournalTransition::Requested {
            key: recovery_key.clone(),
            prior_phase,
            prior_status,
            note: None,
        },
        CompletionJournalTransition::CommandInFlight {
            key: recovery_key.clone(),
            correlation: correlation.clone(),
            deadline: CompletionDeadline::from_unix_millis(0),
        },
        CompletionJournalTransition::Confirmed {
            key: recovery_key.clone(),
            correlation: correlation.clone(),
            receipt: receipt.clone(),
            note: None,
        },
    ];
    for transition in &pending[start.already_recorded()..] {
        append_with_seal_using(
            request.journal_path,
            CompletionSeal::Commit,
            transition.clone(),
            atomic_write,
        )?;
    }

    let ticket_file_rewritten =
        board_ticket.phase != Phase::Done || board_ticket.status != TicketStatus::Done;
    if ticket_file_rewritten {
        ticket::update_ticket_done(&board_ticket.file_path)
            .map_err(|error| format!("Could not mark {ticket_id} done: {error}"))?;
    }

    Ok(AlreadyDoneOutcome::Recovered {
        ticket_id: ticket_id.to_string(),
        commit_id,
        ticket_file_rewritten,
    })
}

/// The attempt identity a person's own settlement is recorded under.
const OPERATOR_ATTEMPT: &str = "operator";

fn declined(message: String) -> AlreadyDoneOutcome {
    AlreadyDoneOutcome::Declined(message)
}

/// Where in the three-record sequence this run has to start.
enum RecoveryStart {
    /// Nothing of a recovery generation exists yet.
    Fresh,
    /// An earlier run of this command died part-way through.
    Resume { correlation: Option<CorrelationId> },
}

impl RecoveryStart {
    /// How many of the three transitions are already on disk.
    fn already_recorded(&self) -> usize {
        match self {
            Self::Fresh => 0,
            Self::Resume { correlation: None } => 1,
            Self::Resume { .. } => 2,
        }
    }
}

/// Decide whether this completion can be settled, and from where.
///
/// The resume arm matters more than it looks. Three appends means two windows
/// where a killed process leaves a half-written recovery generation, and a
/// half-written one reads as `Requested` — which masks the board's Done and,
/// without this, would be refused as "still working on it". That would be a
/// fresh dead end inside the command built to remove dead ends.
fn recovery_start(
    ticket_id: &str,
    aggregate: &CompletionJournalAggregate,
) -> Result<RecoveryStart, String> {
    if aggregate.seal() != CompletionSeal::Commit {
        return Err(format!(
            "This project records finished work in the journal, not in commits, so there is no \
             commit for {ticket_id} to find."
        ));
    }
    let is_operator_generation =
        aggregate.completion_key().attempt_id().as_str() == OPERATOR_ATTEMPT;
    match aggregate.state() {
        CompletionState::Rejected {
            retryability: Retryability::ActionRequired,
            ..
        } => Ok(RecoveryStart::Fresh),
        CompletionState::Requested if is_operator_generation => {
            Ok(RecoveryStart::Resume { correlation: None })
        }
        CompletionState::CommandInFlight { correlation, .. } if is_operator_generation => {
            Ok(RecoveryStart::Resume {
                correlation: Some(correlation.clone()),
            })
        }
        CompletionState::Confirmed => Err(format!("{ticket_id} is already finished.")),
        _ => Err(format!(
            "{ticket_id} isn't stuck — Lisa is still working on finishing it."
        )),
    }
}

/// The commit that proves some completion for this ticket already sealed.
///
/// Two steps for the same reason the transaction's own discovery uses two:
/// `--grep` matches anywhere in the message, so every candidate is re-read and
/// checked line-by-line before it counts.
fn find_sealed_commit(
    project_root: &Path,
    completion_id: &CompletionId,
) -> Result<Option<String>, String> {
    if head_is_unborn(project_root)? {
        return Ok(None);
    }
    let prefix = completion_key_ticket_prefix(completion_id);
    let candidates = git(
        project_root,
        "look for this ticket's finished work",
        [
            OsStr::new("log"),
            OsStr::new("--format=%H"),
            OsStr::new("--fixed-strings"),
            OsStr::new("--grep"),
            OsStr::new(&prefix),
        ],
    )?;

    for commit_id in candidates.lines().filter(|line| !line.is_empty()) {
        let message = git(
            project_root,
            "read a candidate commit",
            [
                OsStr::new("show"),
                OsStr::new("-s"),
                OsStr::new("--format=%B"),
                OsStr::new(commit_id),
            ],
        )?;
        if message.lines().any(|line| line.starts_with(&prefix)) {
            return Ok(Some(commit_id.to_string()));
        }
    }
    Ok(None)
}

fn head_is_unborn(project_root: &Path) -> Result<bool, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .output()
        .map_err(|error| format!("Could not run git in {}: {error}", project_root.display()))?;
    Ok(!output.status.success())
}

fn git<I>(project_root: &Path, label: &str, args: I) -> Result<String, String>
where
    I: IntoIterator,
    I::Item: AsRef<OsStr>,
{
    let mut command = Command::new("git");
    command.arg("-C").arg(project_root);
    let args: Vec<OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
    command.args(&args);
    let output = command
        .output()
        .map_err(|error| format!("Could not {label}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Could not {label}: git exited {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("Could not {label}: {error}"))
}

fn atomic_write(path: &Path, body: &[u8]) -> Result<(), String> {
    let temporary = PathBuf::from(format!(
        "{}.already-done-tmp-{}",
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
