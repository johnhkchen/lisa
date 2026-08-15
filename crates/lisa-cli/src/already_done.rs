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
//! reachable from HEAD carrying this ticket's `Lisa-Completion-Key` is the
//! evidence that settles a completion without writing one. An operator's word
//! is never a receipt.
//!
//! **When the seal is the only thing missing, it writes the seal.** The
//! 2026-08-13 `renderer` board had five commits of finished work, published
//! review artifacts, and a completion whose *commit* had failed — so there was
//! no key anywhere and this command refused, on a ticket nothing else could
//! finish either. Refusing there was reading "already recorded" as "already
//! sealed". The seal is Lisa's own to write: the journal already holds Lisa's
//! decision that this ticket was completable, the published review is the
//! artifact it admitted, and the completion transaction is the same one the
//! loop would have run. So a rejected completion with no seal in history and a
//! published review is sealed here, through
//! [`lisa_cli::commit_transaction::complete_ticket`](crate::commit_transaction::complete_ticket)
//! — the identical transaction, under an `operator` generation key.
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
    Retryability, COMPLETION_KEY_PREFIX,
};
use lisa_core::completion_journal::{
    append_with_seal_using, load, CompletionJournalAggregate, CompletionJournalTransition,
};
use lisa_core::disposition::{parse_review_disposition, DispositionOrigin, ReviewDisposition};
use lisa_core::ticket;
use lisa_core::types::{Phase, TicketStatus};

use crate::commit_transaction::{complete_ticket, CompleteTicketRequest};

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
    /// Directory Lisa publishes admitted work artifacts into, one
    /// subdirectory per ticket. The published review is what the missing seal
    /// would commit.
    pub work_dir: &'a Path,
    /// The project's completion journal.
    pub journal_path: &'a Path,
}

/// Where the commit that settles this completion came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealSource {
    /// The seal was already in history; this command only recorded it.
    Adopted,
    /// The seal was missing and this command wrote it, through the same
    /// completion transaction the loop uses.
    Written,
}

/// What the command did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlreadyDoneOutcome {
    /// The ticket now reads Done and the journal records it terminal.
    Recovered {
        ticket_id: String,
        commit_id: String,
        /// True when the ticket file on disk had to be rewritten to Done, and
        /// is therefore an uncommitted change the operator still owns. False
        /// for a written seal, whose transaction commits that rewrite.
        ticket_file_rewritten: bool,
        seal: SealSource,
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
    let (recovery_key, correlation) = match &start {
        RecoveryStart::Fresh => {
            let key = CompletionGenerationId::new(
                completion_id.clone(),
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
    // Adopt the seal if it is there, and write it if it is not. The commit
    // comes first either way: a run killed between the commit and the journal
    // rows leaves history holding the key, which is exactly the state the
    // adopting arm was built to settle.
    let (commit_id, seal) = match find_sealed_commit(request.project_root, &completion_id)? {
        Some(commit_id) => (commit_id, SealSource::Adopted),
        None => match write_missing_seal(&request, board_ticket, &recovery_key)? {
            Ok(commit_id) => (commit_id, SealSource::Written),
            Err(decline) => return Ok(declined(decline)),
        },
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

    // A written seal committed the Done bytes itself, so there is nothing left
    // on disk for the operator to own.
    let ticket_file_rewritten = seal == SealSource::Adopted
        && (board_ticket.phase != Phase::Done || board_ticket.status != TicketStatus::Done);
    if ticket_file_rewritten {
        ticket::update_ticket_done(&board_ticket.file_path)
            .map_err(|error| format!("Could not mark {ticket_id} done: {error}"))?;
    }

    Ok(AlreadyDoneOutcome::Recovered {
        ticket_id: ticket_id.to_string(),
        commit_id,
        ticket_file_rewritten,
        seal,
    })
}

/// Write the completion commit Lisa failed to write, or say why not.
///
/// The outer `Result` is a real failure — the transaction ran and could not
/// finish. The inner `Err` is a decline: there is nothing here to seal, said
/// in the words of what was looked for.
///
/// Nothing is journaled before this succeeds. A transaction that fails here
/// restores the ticket's own bytes and leaves the board exactly as it was, so
/// a person can fix what it named and run the command again.
fn write_missing_seal(
    request: &AlreadyDoneRequest<'_>,
    board_ticket: &lisa_core::types::Ticket,
    recovery_key: &CompletionGenerationId,
) -> Result<Result<String, String>, String> {
    let ticket_id = board_ticket.id.as_str();
    let ticket_work_dir = request.work_dir.join(ticket_id);
    let review = ticket_work_dir.join("review.md");
    if head_is_unborn(request.project_root)? || !review.is_file() {
        return Ok(Err(no_evidence_decline(ticket_id, &review)));
    }
    if let Some(refusal) = reviewer_block(ticket_id, &ticket_work_dir) {
        return Ok(Err(refusal));
    }

    let ticket_file = repository_relative(request.project_root, &board_ticket.file_path)?;
    let work_dir = repository_relative(request.project_root, &ticket_work_dir)?;
    match complete_ticket(CompleteTicketRequest {
        repo_root: request.project_root.to_path_buf(),
        ticket_id: ticket_id.to_string(),
        message: format!("Complete {ticket_id}"),
        ticket_file,
        work_dir,
        completion_key: recovery_key.clone(),
    }) {
        Ok(result) => Ok(Ok(result.commit_id)),
        Err(error) => Err(format!(
            "I could not write {ticket_id}'s completion commit, so nothing changed: {error}\n\
             Fix what that names and run `lisa already-done {ticket_id}` again."
        )),
    }
}

/// The refusal that says what was looked for, because five commits naming the
/// ticket in their subject lines are not what this command reads.
fn no_evidence_decline(ticket_id: &str, review: &Path) -> String {
    format!(
        "I can't find {ticket_id}'s finished work, and there is nothing here for me to seal. \
         Nothing changed.\n\
         I look for two things, in order:\n  \
         1. a commit in this repository's history whose message carries the line \
         `{COMPLETION_KEY_PREFIX}v1:…` for {ticket_id} — the seal Lisa writes when it finishes a \
         ticket. A commit that only names {ticket_id} in its subject is not this.\n  \
         2. failing that, {ticket_id}'s published review at {}, which is what I would commit now \
         to write that seal myself.\n\
         Neither is here.",
        review.display()
    )
}

/// A reviewer's block is a judgement about the work and outranks this command.
/// Lisa's own recording-failure block is not — it is the very state this
/// command exists to clear.
fn reviewer_block(ticket_id: &str, ticket_work_dir: &Path) -> Option<String> {
    match parse_review_disposition(ticket_work_dir.join("review-disposition.json")) {
        ReviewDisposition::Block {
            origin: DispositionOrigin::Review,
            reason,
            ..
        } => Some(format!(
            "{ticket_id}'s review says its work is not finished, so I won't seal it: {reason}\n\
             Clear that first — `lisa unblock {ticket_id}` once the ask is done — then run this \
             again."
        )),
        _ => None,
    }
}

/// Express a path the board reported as one the commit transaction accepts.
fn repository_relative(project_root: &Path, path: &Path) -> Result<PathBuf, String> {
    path.strip_prefix(project_root)
        .map(Path::to_path_buf)
        .map_err(|_| {
            format!(
                "{} is outside the project at {}",
                path.display(),
                project_root.display()
            )
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
