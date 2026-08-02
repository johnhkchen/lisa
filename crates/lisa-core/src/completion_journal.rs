//! Durable append-only completion aggregate journal.
//!
//! Records are logically appended as compact JSONL. Each append validates and
//! folds the complete prior history, adds one record in memory, then hands the
//! complete new bytes to a caller-supplied atomic publisher. Readers therefore
//! see either the prior complete history or the new complete history, never a
//! torn final transition.
//!
//! The publisher is a parameter rather than a fixed mechanism because two
//! crates write this file: the plugin's effect adapter, which publishes through
//! its own sibling-temporary machinery, and the operator recovery command,
//! which runs with no plugin at all. Both go through the same fold, so neither
//! can append a record the other cannot replay.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::completion::{
    reduce, AttemptId, CompletionContentHash, CompletionDeadline, CompletionEvent,
    CompletionGenerationId, CompletionId, CompletionSeal, CompletionSealReceipt, CompletionState,
    CorrelationId, LaunchFailure, Retryability,
};
use crate::disposition::DispositionNote;
use crate::types::{Phase, TicketId, TicketStatus};

const LEGACY_SCHEMA_VERSION: u32 = 1;
const SCHEMA_VERSION: u32 = 5;

/// Where a project keeps its completion journal, relative to the project root.
pub const COMPLETION_JOURNAL_RELATIVE_PATH: &str = ".lisa/completion-journal.jsonl";

/// Maximum generations of one completion that may end action-required before
/// it stops being re-armed.
///
/// Lives beside the counter it bounds because two crates read it: the scheduler
/// decides whether to re-arm, and `lisa unblock` decides whether reopening the
/// ticket could possibly help.
pub const MAX_ACTION_REQUIRED_GENERATIONS: u8 = 2;

/// One durable completion transition requested by the plugin adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionJournalTransition {
    Requested {
        key: CompletionGenerationId,
        prior_phase: Phase,
        prior_status: TicketStatus,
        note: Option<DispositionNote>,
    },
    CommandInFlight {
        key: CompletionGenerationId,
        correlation: CorrelationId,
        deadline: CompletionDeadline,
    },
    FailureObserved {
        key: CompletionGenerationId,
        correlation: CorrelationId,
        reason: String,
        class: CompletionFailureClass,
        failure_count: u8,
        failure_limit: u8,
        consequence: FailureConsequence,
    },
    Rejected {
        key: CompletionGenerationId,
        correlation: Option<CorrelationId>,
        reason: String,
        retryability: Retryability,
    },
    Confirmed {
        key: CompletionGenerationId,
        correlation: CorrelationId,
        receipt: CompletionSealReceipt,
        note: Option<DispositionNote>,
    },
}

impl CompletionJournalTransition {
    fn key(&self) -> &CompletionGenerationId {
        match self {
            Self::Requested { key, .. }
            | Self::CommandInFlight { key, .. }
            | Self::FailureObserved { key, .. }
            | Self::Rejected { key, .. }
            | Self::Confirmed { key, .. } => key,
        }
    }
}

/// Conservative adapter classification retained with every failed command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompletionFailureClass {
    OperatorHistoryOrIdentity,
    OperatorRepositoryUnwritable,
    OperatorStaleLock,
    TransientContention,
    Unrecognized,
    DeadlineExpired,
}

/// Scheduler consequence paired with one bounded failed command observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailureConsequence {
    RetryScheduled,
    RetryExhausted,
    Park,
}

/// Latest typed state reconstructed for one completion/ticket aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionJournalAggregate {
    completion_key: CompletionGenerationId,
    seal: CompletionSeal,
    state: CompletionState,
    prior_phase: Phase,
    prior_status: TicketStatus,
    confirmed_receipt: Option<CompletionSealReceipt>,
    completion_note: Option<DispositionNote>,
    failure_count: u8,
    failure_limit: Option<u8>,
    retries_exhausted: bool,
    action_required_generations: u8,
}

impl CompletionJournalAggregate {
    pub fn completion_key(&self) -> &CompletionGenerationId {
        &self.completion_key
    }

    pub fn seal(&self) -> CompletionSeal {
        self.seal
    }

    pub fn state(&self) -> &CompletionState {
        &self.state
    }

    pub fn prior_phase(&self) -> Phase {
        self.prior_phase
    }

    pub fn prior_status(&self) -> TicketStatus {
        self.prior_status
    }

    pub fn failure_count(&self) -> u8 {
        self.failure_count
    }

    pub fn failure_limit(&self) -> Option<u8> {
        self.failure_limit
    }

    pub fn retries_exhausted(&self) -> bool {
        self.retries_exhausted
    }

    /// How many generations of this completion have ended action-required.
    ///
    /// `failure_count` is bounded *within* one generation and resets whenever a
    /// new key starts. That is why re-attempting was bounded per attempt and
    /// unbounded across loop starts: each unpark minted a fresh generation with
    /// a fresh budget. This counter is the one that survives the reset, so a
    /// recovery that keeps failing can end somewhere instead of nowhere.
    ///
    /// Derived from the records already on disk — no record type and no schema
    /// version changes, and an old journal folds to the same number.
    pub fn action_required_generations(&self) -> u8 {
        self.action_required_generations
    }

    pub fn confirmed_commit_id(&self) -> Option<&str> {
        self.confirmed_receipt
            .as_ref()
            .and_then(CompletionSealReceipt::commit_id)
    }

    pub fn confirmed_receipt(&self) -> Option<&CompletionSealReceipt> {
        self.confirmed_receipt.as_ref()
    }

    pub fn completion_note(&self) -> Option<&DispositionNote> {
        self.completion_note.as_ref()
    }

    pub fn masks_durable_done(&self) -> bool {
        matches!(
            self.state,
            CompletionState::Requested
                | CompletionState::CommandInFlight { .. }
                | CompletionState::Rejected {
                    retryability: Retryability::ActionRequired,
                    ..
                }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct JournalRecord {
    schema_version: u32,
    #[serde(default)]
    seal: CompletionSeal,
    #[serde(flatten)]
    body: JournalRecordBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
enum JournalRecordBody {
    Requested {
        completion_id: String,
        attempt_id: String,
        generation: u64,
        prior_phase: Phase,
        prior_status: TicketStatus,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<DispositionNote>,
    },
    CommandInFlight {
        completion_id: String,
        attempt_id: String,
        generation: u64,
        correlation_id: String,
        #[serde(default)]
        reconciliation_deadline_unix_ms: Option<u64>,
    },
    FailureObserved {
        completion_id: String,
        attempt_id: String,
        generation: u64,
        correlation_id: String,
        reason: String,
        class: CompletionFailureClass,
        failure_count: u8,
        failure_limit: u8,
        consequence: FailureConsequence,
    },
    Rejected {
        completion_id: String,
        attempt_id: String,
        generation: u64,
        correlation_id: Option<String>,
        reason: String,
        retryability: JournalRetryability,
    },
    Confirmed {
        completion_id: String,
        attempt_id: String,
        generation: u64,
        correlation_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        commit_id: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content_hashes: Vec<CompletionContentHash>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<DispositionNote>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum JournalRetryability {
    Retryable,
    ActionRequired,
}

impl From<Retryability> for JournalRetryability {
    fn from(value: Retryability) -> Self {
        match value {
            Retryability::Retryable => Self::Retryable,
            Retryability::ActionRequired => Self::ActionRequired,
        }
    }
}

impl From<JournalRetryability> for Retryability {
    fn from(value: JournalRetryability) -> Self {
        match value {
            JournalRetryability::Retryable => Self::Retryable,
            JournalRetryability::ActionRequired => Self::ActionRequired,
        }
    }
}

impl JournalRecord {
    fn from_transition(seal: CompletionSeal, transition: &CompletionJournalTransition) -> Self {
        let body = match transition {
            CompletionJournalTransition::Requested {
                key,
                prior_phase,
                prior_status,
                note,
            } => JournalRecordBody::Requested {
                completion_id: key.completion_id().to_string(),
                attempt_id: key.attempt_id().to_string(),
                generation: key.generation(),
                prior_phase: *prior_phase,
                prior_status: *prior_status,
                note: note.clone(),
            },
            CompletionJournalTransition::CommandInFlight {
                key,
                correlation,
                deadline,
            } => JournalRecordBody::CommandInFlight {
                completion_id: key.completion_id().to_string(),
                attempt_id: key.attempt_id().to_string(),
                generation: key.generation(),
                correlation_id: correlation.to_string(),
                reconciliation_deadline_unix_ms: Some(deadline.unix_millis()),
            },
            CompletionJournalTransition::FailureObserved {
                key,
                correlation,
                reason,
                class,
                failure_count,
                failure_limit,
                consequence,
            } => JournalRecordBody::FailureObserved {
                completion_id: key.completion_id().to_string(),
                attempt_id: key.attempt_id().to_string(),
                generation: key.generation(),
                correlation_id: correlation.to_string(),
                reason: reason.clone(),
                class: *class,
                failure_count: *failure_count,
                failure_limit: *failure_limit,
                consequence: *consequence,
            },
            CompletionJournalTransition::Rejected {
                key,
                correlation,
                reason,
                retryability,
            } => JournalRecordBody::Rejected {
                completion_id: key.completion_id().to_string(),
                attempt_id: key.attempt_id().to_string(),
                generation: key.generation(),
                correlation_id: correlation.as_ref().map(ToString::to_string),
                reason: reason.clone(),
                retryability: (*retryability).into(),
            },
            CompletionJournalTransition::Confirmed {
                key,
                correlation,
                receipt,
                note,
            } => {
                let (commit_id, content_hashes) = match receipt {
                    CompletionSealReceipt::Commit { commit_id } => {
                        (Some(commit_id.clone()), Vec::new())
                    }
                    CompletionSealReceipt::Journal { content_hashes } => {
                        (None, content_hashes.clone())
                    }
                };
                JournalRecordBody::Confirmed {
                    completion_id: key.completion_id().to_string(),
                    attempt_id: key.attempt_id().to_string(),
                    generation: key.generation(),
                    correlation_id: correlation.to_string(),
                    commit_id,
                    content_hashes,
                    note: note.clone(),
                }
            }
        };
        Self {
            schema_version: SCHEMA_VERSION,
            seal,
            body,
        }
    }

    fn into_transition(self) -> Result<(CompletionSeal, CompletionJournalTransition), String> {
        if !(LEGACY_SCHEMA_VERSION..=SCHEMA_VERSION).contains(&self.schema_version) {
            return Err(format!(
                "unsupported completion journal schema version {} (expected {LEGACY_SCHEMA_VERSION} through {SCHEMA_VERSION})",
                self.schema_version
            ));
        }
        let transition = match self.body {
            JournalRecordBody::Requested {
                completion_id,
                attempt_id,
                generation,
                prior_phase,
                prior_status,
                note,
            } => CompletionJournalTransition::Requested {
                key: generation_key(completion_id, attempt_id, generation),
                prior_phase,
                prior_status,
                note,
            },
            JournalRecordBody::CommandInFlight {
                completion_id,
                attempt_id,
                generation,
                correlation_id,
                reconciliation_deadline_unix_ms,
            } => CompletionJournalTransition::CommandInFlight {
                key: generation_key(completion_id, attempt_id, generation),
                correlation: CorrelationId::new(correlation_id),
                deadline: CompletionDeadline::from_unix_millis(
                    reconciliation_deadline_unix_ms.unwrap_or(0),
                ),
            },
            JournalRecordBody::FailureObserved {
                completion_id,
                attempt_id,
                generation,
                correlation_id,
                reason,
                class,
                failure_count,
                failure_limit,
                consequence,
            } => CompletionJournalTransition::FailureObserved {
                key: generation_key(completion_id, attempt_id, generation),
                correlation: CorrelationId::new(correlation_id),
                reason,
                class,
                failure_count,
                failure_limit,
                consequence,
            },
            JournalRecordBody::Rejected {
                completion_id,
                attempt_id,
                generation,
                correlation_id,
                reason,
                retryability,
            } => CompletionJournalTransition::Rejected {
                key: generation_key(completion_id, attempt_id, generation),
                correlation: correlation_id.map(CorrelationId::new),
                reason,
                retryability: retryability.into(),
            },
            JournalRecordBody::Confirmed {
                completion_id,
                attempt_id,
                generation,
                correlation_id,
                commit_id,
                content_hashes,
                note,
            } => CompletionJournalTransition::Confirmed {
                key: generation_key(completion_id, attempt_id, generation),
                correlation: CorrelationId::new(correlation_id),
                receipt: match self.seal {
                    CompletionSeal::Commit => {
                        if !content_hashes.is_empty() {
                            return Err("commit-sealed confirmation must not carry content hashes"
                                .to_string());
                        }
                        CompletionSealReceipt::commit(commit_id.ok_or_else(|| {
                            "commit-sealed confirmation requires a commit id".to_string()
                        })?)?
                    }
                    CompletionSeal::Journal => {
                        if commit_id.is_some() {
                            return Err("journal-sealed confirmation must not carry a commit id"
                                .to_string());
                        }
                        CompletionSealReceipt::journal(content_hashes)?
                    }
                },
                note,
            },
        };
        Ok((self.seal, transition))
    }
}

fn generation_key(
    completion_id: String,
    attempt_id: String,
    generation: u64,
) -> CompletionGenerationId {
    CompletionGenerationId::new(
        CompletionId::new(completion_id),
        AttemptId::new(attempt_id),
        generation,
    )
}
pub fn load(path: &Path) -> Result<HashMap<TicketId, CompletionJournalAggregate>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(error) => {
            return Err(format!(
                "cannot read completion journal {}: {error}",
                path.display()
            ));
        }
    };
    fold_bytes(&bytes)
}

/// Atomically append one validated transition and return its new aggregate.
///
/// The whole prior history is folded before a single byte is handed to
/// `publish`, so an unreplayable journal fails here rather than growing a
/// record the plugin's fail-closed load would choke on. `publish` receives the
/// destination and the complete new file bytes.
pub fn append_with_seal_using<F>(
    path: &Path,
    seal: CompletionSeal,
    transition: CompletionJournalTransition,
    publish: F,
) -> Result<CompletionJournalAggregate, String>
where
    F: FnOnce(&Path, &[u8]) -> Result<(), String>,
{
    let mut bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            return Err(format!(
                "cannot read completion journal {}: {error}",
                path.display()
            ));
        }
    };
    let mut aggregates = fold_bytes(&bytes)?;
    let record = JournalRecord::from_transition(seal, &transition);
    let ticket_id = transition.key().completion_id().to_string();
    let aggregate = apply_transition(&mut aggregates, seal, transition)?;

    let mut line = serde_json::to_vec(&record)
        .map_err(|error| format!("cannot serialize completion journal transition: {error}"))?;
    line.push(b'\n');
    bytes.extend_from_slice(&line);

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "cannot create completion journal directory {}: {error}",
                    parent.display()
                )
            })?;
        }
    }
    publish(path, &bytes)?;

    debug_assert_eq!(ticket_id, aggregate.completion_key.completion_id().as_str());
    Ok(aggregate)
}

#[cfg(test)]
fn append(
    path: &Path,
    transition: CompletionJournalTransition,
) -> Result<CompletionJournalAggregate, String> {
    append_with_seal(path, CompletionSeal::Commit, transition)
}

/// Test-only publisher matching the plugin's sibling-temporary contract.
#[cfg(test)]
fn append_with_seal(
    path: &Path,
    seal: CompletionSeal,
    transition: CompletionJournalTransition,
) -> Result<CompletionJournalAggregate, String> {
    append_with_seal_using(path, seal, transition, |destination, body| {
        let temporary = destination
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(".completion-journal.jsonl.test.tmp");
        fs::write(&temporary, body).map_err(|error| format!("cannot write journal: {error}"))?;
        fs::rename(&temporary, destination)
            .map_err(|error| format!("cannot publish journal: {error}"))
    })
}

fn fold_bytes(bytes: &[u8]) -> Result<HashMap<TicketId, CompletionJournalAggregate>, String> {
    let mut aggregates = HashMap::new();
    if bytes.is_empty() {
        return Ok(aggregates);
    }
    if !bytes.ends_with(b"\n") {
        return Err("completion journal has a torn final record (missing newline)".to_string());
    }

    for (index, line) in bytes[..bytes.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        let line_number = index + 1;
        if line.is_empty() {
            return Err(format!(
                "completion journal line {line_number} is unexpectedly empty"
            ));
        }
        let record: JournalRecord = serde_json::from_slice(line).map_err(|error| {
            format!("cannot parse completion journal line {line_number}: {error}")
        })?;
        let (seal, transition) = record
            .into_transition()
            .map_err(|error| format!("invalid completion journal line {line_number}: {error}"))?;
        apply_transition(&mut aggregates, seal, transition)
            .map_err(|error| format!("invalid completion journal line {line_number}: {error}"))?;
    }
    Ok(aggregates)
}

fn apply_transition(
    aggregates: &mut HashMap<TicketId, CompletionJournalAggregate>,
    seal: CompletionSeal,
    transition: CompletionJournalTransition,
) -> Result<CompletionJournalAggregate, String> {
    let ticket_id = transition.key().completion_id().to_string();
    let next = match transition {
        CompletionJournalTransition::Requested {
            key,
            prior_phase,
            prior_status,
            note,
        } => {
            if let Some(aggregate) = aggregates.get(&ticket_id) {
                if aggregate.completion_key == key && aggregate.seal != seal {
                    return Err(format!(
                        "completion {ticket_id} seal mismatch: expected {}, got {seal}",
                        aggregate.seal
                    ));
                }
            }
            let state = match aggregates.get(&ticket_id) {
                Some(aggregate)
                    if aggregate.completion_key != key
                        && matches!(
                            aggregate.state,
                            CompletionState::Rejected { .. } | CompletionState::Confirmed
                        ) =>
                {
                    CompletionState::Eligible
                }
                Some(aggregate) => aggregate.state.clone(),
                None => CompletionState::Eligible,
            };
            let reduced = reduce(
                state,
                CompletionEvent::Request {
                    attempt_id: key.attempt_id().clone(),
                    completion_id: key.completion_id().clone(),
                },
            )
            .map_err(|error| format!("requested transition rejected for {ticket_id}: {error}"))?;
            if !matches!(reduced.state, CompletionState::Requested) {
                return Err(format!(
                    "requested transition for {ticket_id} produced unexpected state {:?}",
                    reduced.state
                ));
            }
            CompletionJournalAggregate {
                completion_key: key,
                seal,
                state: reduced.state,
                prior_phase,
                prior_status,
                confirmed_receipt: None,
                completion_note: note,
                failure_count: 0,
                failure_limit: None,
                retries_exhausted: false,
                // The per-generation budget resets here — deliberately, a new
                // generation is a new command. What must not reset is how many
                // generations have already given up, or recovery is unbounded
                // by construction.
                action_required_generations: aggregates
                    .get(&ticket_id)
                    .map(|prior| prior.action_required_generations)
                    .unwrap_or(0),
            }
        }
        CompletionJournalTransition::CommandInFlight {
            key,
            correlation,
            deadline,
        } => {
            let mut aggregate = matching_aggregate(aggregates, &ticket_id, &key, seal)?;
            let reduced = reduce(
                aggregate.state.clone(),
                CompletionEvent::CommandLaunched {
                    correlation,
                    deadline,
                },
            )
            .map_err(|error| {
                format!("command-in-flight transition rejected for {ticket_id}: {error}")
            })?;
            if !matches!(reduced.state, CompletionState::CommandInFlight { .. }) {
                return Err(format!(
                    "command-in-flight transition for {ticket_id} produced unexpected state {:?}",
                    reduced.state
                ));
            }
            aggregate.state = reduced.state;
            aggregate.confirmed_receipt = None;
            aggregate
        }
        CompletionJournalTransition::FailureObserved {
            key,
            correlation,
            reason,
            class: _,
            failure_count,
            failure_limit,
            consequence,
        } => {
            let mut aggregate = matching_aggregate(aggregates, &ticket_id, &key, seal)?;
            let CompletionState::CommandInFlight {
                correlation: expected,
                ..
            } = &aggregate.state
            else {
                return Err(format!(
                    "cannot observe a command failure for completion {ticket_id} from state {:?}",
                    aggregate.state
                ));
            };
            if expected != &correlation {
                return Err(format!(
                    "completion failure correlation mismatch for {ticket_id}: expected {expected}, got {correlation}"
                ));
            }
            if reason.trim().is_empty() {
                return Err(format!(
                    "completion failure observation for {ticket_id} requires a reason"
                ));
            }
            if failure_limit == 0 {
                return Err(format!(
                    "completion failure observation for {ticket_id} requires a positive limit"
                ));
            }
            let expected_count = aggregate.failure_count.saturating_add(1);
            if failure_count != expected_count {
                return Err(format!(
                    "completion failure count for {ticket_id} must be {expected_count}, got {failure_count}"
                ));
            }
            if aggregate
                .failure_limit
                .is_some_and(|prior| prior != failure_limit)
            {
                return Err(format!(
                    "completion failure limit changed for {ticket_id}: expected {}, got {failure_limit}",
                    aggregate.failure_limit.unwrap_or_default()
                ));
            }
            if failure_count > failure_limit {
                return Err(format!(
                    "completion failure count {failure_count} exceeds limit {failure_limit} for {ticket_id}"
                ));
            }
            match consequence {
                FailureConsequence::RetryScheduled if failure_count >= failure_limit => {
                    return Err(format!(
                        "completion retry for {ticket_id} cannot be scheduled at exhausted count {failure_count}/{failure_limit}"
                    ));
                }
                FailureConsequence::RetryExhausted if failure_count != failure_limit => {
                    return Err(format!(
                        "completion retries for {ticket_id} can only be exhausted at {failure_limit}, got {failure_count}"
                    ));
                }
                _ => {}
            }
            aggregate.failure_count = failure_count;
            aggregate.failure_limit = Some(failure_limit);
            aggregate.retries_exhausted = matches!(
                consequence,
                FailureConsequence::RetryExhausted | FailureConsequence::Park
            );
            aggregate
        }
        CompletionJournalTransition::Rejected {
            key,
            correlation,
            reason,
            retryability,
        } => {
            let mut aggregate = matching_aggregate(aggregates, &ticket_id, &key, seal)?;
            let event = match &aggregate.state {
                CompletionState::Requested => {
                    if correlation.is_some() {
                        return Err(format!(
                            "requested aggregate {ticket_id} cannot reject a correlated command"
                        ));
                    }
                    if retryability != Retryability::Retryable {
                        return Err(format!(
                            "pre-launch rejection for {ticket_id} must be retryable"
                        ));
                    }
                    CompletionEvent::CommandLaunchFailed {
                        source: LaunchFailure::new(reason),
                    }
                }
                CompletionState::CommandInFlight { .. } => CompletionEvent::CommandFailed {
                    correlation: correlation.ok_or_else(|| {
                        format!("in-flight rejection for {ticket_id} requires a correlation")
                    })?,
                    source: LaunchFailure::new(reason),
                    retryability,
                },
                state => {
                    return Err(format!(
                        "cannot reject completion {ticket_id} from state {state:?}"
                    ));
                }
            };
            let reduced = reduce(aggregate.state.clone(), event)
                .map_err(|error| format!("rejected transition refused for {ticket_id}: {error}"))?;
            if !matches!(reduced.state, CompletionState::Rejected { .. }) {
                return Err(format!(
                    "rejected transition for {ticket_id} produced unexpected state {:?}",
                    reduced.state
                ));
            }
            if matches!(
                reduced.state,
                CompletionState::Rejected {
                    retryability: Retryability::ActionRequired,
                    ..
                }
            ) {
                aggregate.action_required_generations =
                    aggregate.action_required_generations.saturating_add(1);
            }
            aggregate.state = reduced.state;
            aggregate.confirmed_receipt = None;
            aggregate
        }
        CompletionJournalTransition::Confirmed {
            key,
            correlation,
            receipt,
            note,
        } => {
            if receipt.seal() != seal {
                return Err(format!(
                    "confirmed transition for {ticket_id} carries {} evidence under {seal} seal",
                    receipt.seal()
                ));
            }
            let mut aggregate = matching_aggregate(aggregates, &ticket_id, &key, seal)?;
            let reduced = reduce(
                aggregate.state.clone(),
                CompletionEvent::CommandSucceeded { correlation },
            )
            .map_err(|error| format!("confirmed transition rejected for {ticket_id}: {error}"))?;
            if !matches!(reduced.state, CompletionState::Confirmed) {
                return Err(format!(
                    "confirmed transition for {ticket_id} produced unexpected state {:?}",
                    reduced.state
                ));
            }
            aggregate.state = reduced.state;
            aggregate.confirmed_receipt = Some(receipt);
            if note != aggregate.completion_note {
                return Err(format!(
                    "confirmed transition for {ticket_id} changed its admitted completion note"
                ));
            }
            aggregate
        }
    };

    aggregates.insert(ticket_id, next.clone());
    Ok(next)
}

fn matching_aggregate(
    aggregates: &HashMap<TicketId, CompletionJournalAggregate>,
    ticket_id: &str,
    key: &CompletionGenerationId,
    seal: CompletionSeal,
) -> Result<CompletionJournalAggregate, String> {
    let aggregate = aggregates
        .get(ticket_id)
        .cloned()
        .ok_or_else(|| format!("completion {ticket_id} has no requested aggregate"))?;
    if aggregate.completion_key != *key {
        return Err(format!(
            "completion {ticket_id} generation key mismatch: expected {}, got {key}",
            aggregate.completion_key
        ));
    }
    if aggregate.seal != seal {
        return Err(format!(
            "completion {ticket_id} seal mismatch: expected {}, got {seal}",
            aggregate.seal
        ));
    }
    Ok(aggregate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(ticket: &str, attempt: &str, generation: u64) -> CompletionGenerationId {
        CompletionGenerationId::new(
            CompletionId::new(ticket),
            AttemptId::new(attempt),
            generation,
        )
    }

    fn correlation(value: &str) -> CorrelationId {
        CorrelationId::new(value)
    }

    fn deadline(value: u64) -> CompletionDeadline {
        CompletionDeadline::from_unix_millis(value)
    }

    #[test]
    fn journal_confirmation_row_carries_only_journal_hash_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        let generation = key("T-HASHED", "1", 1);
        let command = correlation("journal-command");
        append_with_seal(
            &path,
            CompletionSeal::Journal,
            CompletionJournalTransition::Requested {
                key: generation.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Open,
                note: None,
            },
        )
        .unwrap();
        append_with_seal(
            &path,
            CompletionSeal::Journal,
            CompletionJournalTransition::CommandInFlight {
                key: generation.clone(),
                correlation: command.clone(),
                deadline: deadline(42),
            },
        )
        .unwrap();
        let receipt = CompletionSealReceipt::journal(vec![
            CompletionContentHash::new("tickets/T-HASHED.md", "a".repeat(64)).unwrap(),
            CompletionContentHash::new("work/T-HASHED/review.md", "b".repeat(64)).unwrap(),
        ])
        .unwrap();
        let confirmed = append_with_seal(
            &path,
            CompletionSeal::Journal,
            CompletionJournalTransition::Confirmed {
                key: generation,
                correlation: command,
                receipt: receipt.clone(),
                note: None,
            },
        )
        .unwrap();

        assert_eq!(confirmed.confirmed_receipt(), Some(&receipt));
        assert_eq!(load(&path).unwrap()["T-HASHED"], confirmed);
        let confirmed_line = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .last()
            .unwrap()
            .to_string();
        assert!(confirmed_line.contains("\"schema_version\":5"));
        assert!(confirmed_line.contains("\"seal\":\"journal\""));
        assert!(confirmed_line.contains("\"content_hashes\""));
        assert!(confirmed_line.contains("tickets/T-HASHED.md"));
        assert!(!confirmed_line.contains("\"commit_id\""));
    }

    #[test]
    fn admitted_note_is_stable_across_request_confirmation_and_reload() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        let generation = key("T-046-06-03", "1", 1);
        let command = correlation("note-command");
        let note = DispositionNote::new(
            "approximately 200 MiB",
            "docs/active/work/T-046-06-03/cbt-0716-210943-closing-codex/run-record.md",
            "The 225 MiB measurement supports completion while the written gate is stale.",
        )
        .unwrap();
        append(
            &path,
            CompletionJournalTransition::Requested {
                key: generation.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Open,
                note: Some(note.clone()),
            },
        )
        .unwrap();
        append(
            &path,
            CompletionJournalTransition::CommandInFlight {
                key: generation.clone(),
                correlation: command.clone(),
                deadline: deadline(42),
            },
        )
        .unwrap();
        let confirmed = append(
            &path,
            CompletionJournalTransition::Confirmed {
                key: generation,
                correlation: command,
                receipt: CompletionSealReceipt::commit("a".repeat(40)).unwrap(),
                note: Some(note.clone()),
            },
        )
        .unwrap();

        assert_eq!(confirmed.completion_note(), Some(&note));
        assert_eq!(load(&path).unwrap()["T-046-06-03"], confirmed);
        let body = fs::read_to_string(&path).unwrap();
        assert_eq!(body.matches("\"completion_note\"").count(), 0);
        assert_eq!(body.matches("\"note\"").count(), 2);
        assert_eq!(body.matches("\"criterion_quote\"").count(), 2);
        assert_eq!(body.matches("\"evidence_citation\"").count(), 2);
    }

    #[test]
    fn schema_three_commit_confirmation_remains_readable_and_invalid_journal_receipt_fails_closed()
    {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        fs::write(
            &path,
            concat!(
                "{\"schema_version\":3,\"seal\":\"commit\",\"state\":\"requested\",\"completion_id\":\"T-SCHEMA3\",\"attempt_id\":\"1\",\"generation\":1,\"prior_phase\":\"review\",\"prior_status\":\"open\"}\n",
                "{\"schema_version\":3,\"seal\":\"commit\",\"state\":\"command-in-flight\",\"completion_id\":\"T-SCHEMA3\",\"attempt_id\":\"1\",\"generation\":1,\"correlation_id\":\"schema3-command\",\"reconciliation_deadline_unix_ms\":42}\n",
                "{\"schema_version\":3,\"seal\":\"commit\",\"state\":\"confirmed\",\"completion_id\":\"T-SCHEMA3\",\"attempt_id\":\"1\",\"generation\":1,\"correlation_id\":\"schema3-command\",\"commit_id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\n",
            ),
        )
        .unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(
            loaded["T-SCHEMA3"].confirmed_commit_id(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );

        let invalid = temp.path().join("invalid-journal-receipt.jsonl");
        fs::write(
            &invalid,
            concat!(
                "{\"schema_version\":4,\"seal\":\"journal\",\"state\":\"requested\",\"completion_id\":\"T-INVALID\",\"attempt_id\":\"1\",\"generation\":1,\"prior_phase\":\"review\",\"prior_status\":\"open\"}\n",
                "{\"schema_version\":4,\"seal\":\"journal\",\"state\":\"command-in-flight\",\"completion_id\":\"T-INVALID\",\"attempt_id\":\"1\",\"generation\":1,\"correlation_id\":\"invalid-command\",\"reconciliation_deadline_unix_ms\":42}\n",
                "{\"schema_version\":4,\"seal\":\"journal\",\"state\":\"confirmed\",\"completion_id\":\"T-INVALID\",\"attempt_id\":\"1\",\"generation\":1,\"correlation_id\":\"invalid-command\"}\n",
            ),
        )
        .unwrap();
        let error = load(&invalid).unwrap_err();
        assert!(error.contains("journal completion receipt requires content hashes"));
    }

    #[test]
    fn requested_in_flight_and_confirmed_reconstruct_after_each_restart() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("journal path ' ; $() `x`");
        let path = directory.join("completion-journal.jsonl");
        let generation = key("T-042-02-02", "attempt ' 7", 3);
        let command = correlation("v1:correlation ' ; $()");

        let requested = append(
            &path,
            CompletionJournalTransition::Requested {
                key: generation.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::InProgress,
                note: None,
            },
        )
        .unwrap();
        assert_eq!(requested.completion_key(), &generation);
        assert_eq!(requested.state(), &CompletionState::Requested);
        assert_eq!(requested.prior_phase(), Phase::Review);
        assert_eq!(requested.prior_status(), TicketStatus::InProgress);
        assert!(requested.masks_durable_done());
        assert_eq!(load(&path).unwrap().get("T-042-02-02"), Some(&requested));

        let in_flight = append(
            &path,
            CompletionJournalTransition::CommandInFlight {
                key: generation.clone(),
                correlation: command.clone(),
                deadline: deadline(42),
            },
        )
        .unwrap();
        assert_eq!(
            in_flight.state(),
            &CompletionState::CommandInFlight {
                correlation: command.clone(),
                deadline: deadline(42),
            }
        );
        assert!(in_flight.masks_durable_done());
        assert_eq!(load(&path).unwrap().get("T-042-02-02"), Some(&in_flight));

        let commit_id = "a".repeat(40);
        let confirmed = append(
            &path,
            CompletionJournalTransition::Confirmed {
                key: generation,
                correlation: command,
                receipt: CompletionSealReceipt::commit(commit_id.clone()).unwrap(),
                note: None,
            },
        )
        .unwrap();
        assert_eq!(confirmed.state(), &CompletionState::Confirmed);
        assert_eq!(confirmed.confirmed_commit_id(), Some(commit_id.as_str()));
        assert!(!confirmed.masks_durable_done());
        assert_eq!(load(&path).unwrap().get("T-042-02-02"), Some(&confirmed));

        let body = fs::read_to_string(&path).unwrap();
        assert_eq!(body.lines().count(), 3);
        assert_eq!(body.matches("\"schema_version\":5").count(), 3);
        assert_eq!(body.matches("\"seal\":\"commit\"").count(), 3);
        assert_eq!(body.matches("\"state\":\"requested\"").count(), 1);
        assert_eq!(body.matches("\"state\":\"command-in-flight\"").count(), 1);
        assert_eq!(
            body.matches("\"reconciliation_deadline_unix_ms\":42")
                .count(),
            1
        );
        assert_eq!(body.matches("\"state\":\"confirmed\"").count(), 1);
        assert!(body.ends_with('\n'));
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    }

    #[test]
    fn failed_command_observations_are_bounded_durable_and_restart_safe() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        let generation = key("T-FAIL", "9", 1);
        let command = correlation("bounded-command");
        append(
            &path,
            CompletionJournalTransition::Requested {
                key: generation.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Open,
                note: None,
            },
        )
        .unwrap();
        append(
            &path,
            CompletionJournalTransition::CommandInFlight {
                key: generation.clone(),
                correlation: command.clone(),
                deadline: deadline(60_000),
            },
        )
        .unwrap();

        let retry = append(
            &path,
            CompletionJournalTransition::FailureObserved {
                key: generation.clone(),
                correlation: command.clone(),
                reason: "index is busy".to_string(),
                class: CompletionFailureClass::TransientContention,
                failure_count: 1,
                failure_limit: 2,
                consequence: FailureConsequence::RetryScheduled,
            },
        )
        .unwrap();
        assert_eq!(retry.failure_count(), 1);
        assert_eq!(retry.failure_limit(), Some(2));
        assert!(!retry.retries_exhausted());
        assert!(matches!(
            retry.state(),
            CompletionState::CommandInFlight { .. }
        ));

        let exhausted = append(
            &path,
            CompletionJournalTransition::FailureObserved {
                key: generation,
                correlation: command,
                reason: "index is still busy".to_string(),
                class: CompletionFailureClass::TransientContention,
                failure_count: 2,
                failure_limit: 2,
                consequence: FailureConsequence::RetryExhausted,
            },
        )
        .unwrap();
        assert_eq!(exhausted.failure_count(), 2);
        assert_eq!(exhausted.failure_limit(), Some(2));
        assert!(exhausted.retries_exhausted());
        assert_eq!(load(&path).unwrap().get("T-FAIL"), Some(&exhausted));

        let body = fs::read_to_string(&path).unwrap();
        assert_eq!(body.matches("\"state\":\"failure-observed\"").count(), 2);
        assert!(body.contains("\"failure_count\":1,\"failure_limit\":2"));
        assert!(body.contains("\"failure_count\":2,\"failure_limit\":2"));
        assert!(body.contains("\"consequence\":\"retry-exhausted\""));
    }

    #[test]
    fn failure_observation_rejects_skips_limit_changes_and_overrun() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        let generation = key("T-BAD-FAIL", "1", 1);
        let command = correlation("command");
        append(
            &path,
            CompletionJournalTransition::Requested {
                key: generation.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Open,
                note: None,
            },
        )
        .unwrap();
        append(
            &path,
            CompletionJournalTransition::CommandInFlight {
                key: generation.clone(),
                correlation: command.clone(),
                deadline: deadline(60_000),
            },
        )
        .unwrap();
        let before = fs::read(&path).unwrap();

        for (count, limit, consequence) in [
            (2, 2, FailureConsequence::RetryExhausted),
            (1, 1, FailureConsequence::RetryScheduled),
            (3, 2, FailureConsequence::Park),
        ] {
            assert!(append(
                &path,
                CompletionJournalTransition::FailureObserved {
                    key: generation.clone(),
                    correlation: command.clone(),
                    reason: "invalid sequence".to_string(),
                    class: CompletionFailureClass::Unrecognized,
                    failure_count: count,
                    failure_limit: limit,
                    consequence,
                },
            )
            .is_err());
            assert_eq!(fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn retryable_rejection_can_start_another_request_generation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        let first = key("T-RETRY", "4", 1);
        let command = correlation("first-command");

        append(
            &path,
            CompletionJournalTransition::Requested {
                key: first.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Open,
                note: None,
            },
        )
        .unwrap();
        append(
            &path,
            CompletionJournalTransition::CommandInFlight {
                key: first.clone(),
                correlation: command.clone(),
                deadline: deadline(42),
            },
        )
        .unwrap();
        let rejected = append(
            &path,
            CompletionJournalTransition::Rejected {
                key: first,
                correlation: Some(command),
                reason: "host command returned 1".to_string(),
                retryability: Retryability::Retryable,
            },
        )
        .unwrap();
        assert!(matches!(
            rejected.state(),
            CompletionState::Rejected {
                retryability: Retryability::Retryable,
                ..
            }
        ));
        assert!(!rejected.masks_durable_done());

        let second = key("T-RETRY", "4", 2);
        let requested = append(
            &path,
            CompletionJournalTransition::Requested {
                key: second.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Open,
                note: None,
            },
        )
        .unwrap();
        assert_eq!(requested.completion_key(), &second);
        assert_eq!(requested.state(), &CompletionState::Requested);
        assert_eq!(load(&path).unwrap().get("T-RETRY"), Some(&requested));
    }

    #[test]
    fn action_required_generations_survive_a_new_key_and_a_retryable_one_does_not_count() {
        // The counter that makes recovery boundable. `failure_count` resets on
        // every new key by design; this one must not, or the field's unpark →
        // re-attempt → park cycle has no end.
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");

        let mut fail_one_generation = |generation: u64, retryability| {
            let generation_key = key("T-BOUND", "operator", generation);
            let command = correlation(&format!("command-{generation}"));
            append(
                &path,
                CompletionJournalTransition::Requested {
                    key: generation_key.clone(),
                    prior_phase: Phase::Review,
                    prior_status: TicketStatus::Open,
                    note: None,
                },
            )
            .unwrap();
            append(
                &path,
                CompletionJournalTransition::CommandInFlight {
                    key: generation_key.clone(),
                    correlation: command.clone(),
                    deadline: deadline(42),
                },
            )
            .unwrap();
            append(
                &path,
                CompletionJournalTransition::Rejected {
                    key: generation_key,
                    correlation: Some(command),
                    reason: "no changes in the requested include paths".to_string(),
                    retryability,
                },
            )
            .unwrap()
        };

        // A retryable rejection is not a generation giving up.
        let retryable = fail_one_generation(1, Retryability::Retryable);
        assert_eq!(retryable.action_required_generations(), 0);

        let first = fail_one_generation(2, Retryability::ActionRequired);
        assert_eq!(first.action_required_generations(), 1);
        let second = fail_one_generation(3, Retryability::ActionRequired);
        assert_eq!(second.action_required_generations(), 2);

        // Durable: a restart folds to the same number, and a fresh generation
        // carries it forward rather than starting the budget over.
        assert_eq!(
            load(&path).unwrap()["T-BOUND"].action_required_generations(),
            2
        );
        let reopened = append(
            &path,
            CompletionJournalTransition::Requested {
                key: key("T-BOUND", "operator", 4),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Open,
                note: None,
            },
        )
        .unwrap();
        assert_eq!(reopened.action_required_generations(), 2);
        assert_eq!(reopened.failure_count(), 0);
    }

    #[test]
    fn a_new_attempt_key_can_start_after_a_confirmed_ticket_is_reset() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        let first = key("T-RESET", "1", 1);
        let command = correlation("first-command");
        append(
            &path,
            CompletionJournalTransition::Requested {
                key: first.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Review,
                note: None,
            },
        )
        .unwrap();
        append(
            &path,
            CompletionJournalTransition::CommandInFlight {
                key: first.clone(),
                correlation: command.clone(),
                deadline: deadline(42),
            },
        )
        .unwrap();
        append(
            &path,
            CompletionJournalTransition::Confirmed {
                key: first,
                correlation: command,
                receipt: CompletionSealReceipt::commit("c".repeat(40)).unwrap(),
                note: None,
            },
        )
        .unwrap();

        let reset_attempt = key("T-RESET", "2", 1);
        let requested = append(
            &path,
            CompletionJournalTransition::Requested {
                key: reset_attempt.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Review,
                note: None,
            },
        )
        .unwrap();
        assert_eq!(requested.completion_key(), &reset_attempt);
        assert_eq!(requested.state(), &CompletionState::Requested);
        assert_eq!(load(&path).unwrap().get("T-RESET"), Some(&requested));
    }

    #[test]
    fn append_publishes_only_after_the_whole_history_folds() {
        // The property the shared writer exists for. A journal the plugin's
        // fail-closed load would refuse must stop an append before any byte
        // reaches the file — otherwise a second writer could grow the exact
        // unreplayable record that fences all scheduling.
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        let unreplayable = concat!(
            "{\"schema_version\":5,\"seal\":\"commit\",\"state\":\"requested\",\"completion_id\":\"T-BRICK\",\"attempt_id\":\"1\",\"generation\":1,\"prior_phase\":\"review\",\"prior_status\":\"open\"}\n",
            "{\"schema_version\":5,\"seal\":\"commit\",\"state\":\"confirmed\",\"completion_id\":\"T-BRICK\",\"attempt_id\":\"1\",\"generation\":1,\"correlation_id\":\"c\",\"commit_id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}\n",
        );
        fs::write(&path, unreplayable).unwrap();
        let mut published = false;

        let error = append_with_seal_using(
            &path,
            CompletionSeal::Commit,
            CompletionJournalTransition::Requested {
                key: key("T-OTHER", "1", 1),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Open,
                note: None,
            },
            |_, _| {
                published = true;
                Ok(())
            },
        )
        .unwrap_err();

        assert!(error.contains("confirmed transition rejected"), "{error}");
        assert!(
            !published,
            "an unfoldable journal must not reach the writer"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), unreplayable);
    }

    #[test]
    fn torn_malformed_empty_and_unknown_records_fail_closed() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");

        fs::write(&path, br#"{"schema_version":1}"#).unwrap();
        assert!(load(&path).unwrap_err().contains("torn final record"));

        fs::write(&path, b"not json\n").unwrap();
        assert!(load(&path)
            .unwrap_err()
            .contains("cannot parse completion journal line 1"));

        fs::write(&path, b"\n").unwrap();
        assert!(load(&path)
            .unwrap_err()
            .contains("line 1 is unexpectedly empty"));

        fs::write(
            &path,
            b"{\"schema_version\":99,\"state\":\"requested\",\"completion_id\":\"T\",\"attempt_id\":\"1\",\"generation\":1,\"prior_phase\":\"review\",\"prior_status\":\"open\"}\n",
        )
        .unwrap();
        assert!(load(&path)
            .unwrap_err()
            .contains("unsupported completion journal schema version 99"));
    }

    #[test]
    fn invalid_key_correlation_and_order_leave_prior_bytes_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        let expected = key("T-STRICT", "1", 1);
        append(
            &path,
            CompletionJournalTransition::Requested {
                key: expected.clone(),
                prior_phase: Phase::Implement,
                prior_status: TicketStatus::InProgress,
                note: None,
            },
        )
        .unwrap();
        let requested_bytes = fs::read(&path).unwrap();

        let error = append(
            &path,
            CompletionJournalTransition::CommandInFlight {
                key: key("T-STRICT", "1", 2),
                correlation: correlation("command-a"),
                deadline: deadline(42),
            },
        )
        .unwrap_err();
        assert!(error.contains("generation key mismatch"));
        assert_eq!(fs::read(&path).unwrap(), requested_bytes);

        append(
            &path,
            CompletionJournalTransition::CommandInFlight {
                key: expected.clone(),
                correlation: correlation("command-a"),
                deadline: deadline(42),
            },
        )
        .unwrap();
        let in_flight_bytes = fs::read(&path).unwrap();

        let error = append(
            &path,
            CompletionJournalTransition::Confirmed {
                key: expected.clone(),
                correlation: correlation("command-b"),
                receipt: CompletionSealReceipt::commit("b".repeat(40)).unwrap(),
                note: None,
            },
        )
        .unwrap_err();
        assert!(error.contains("correlation mismatch"));
        assert_eq!(fs::read(&path).unwrap(), in_flight_bytes);

        let error = append(
            &path,
            CompletionJournalTransition::Requested {
                key: expected,
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Open,
                note: None,
            },
        )
        .unwrap_err();
        assert!(error.contains("already pending"));
        assert_eq!(fs::read(&path).unwrap(), in_flight_bytes);
    }

    #[test]
    fn legacy_in_flight_without_deadline_loads_expired_and_action_required_masks_done() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        fs::write(
            &path,
            concat!(
                "{\"schema_version\":1,\"state\":\"requested\",\"completion_id\":\"T-LEGACY\",\"attempt_id\":\"1\",\"generation\":1,\"prior_phase\":\"review\",\"prior_status\":\"review\"}\n",
                "{\"schema_version\":1,\"state\":\"command-in-flight\",\"completion_id\":\"T-LEGACY\",\"attempt_id\":\"1\",\"generation\":1,\"correlation_id\":\"legacy-command\"}\n",
            ),
        )
        .unwrap();

        let aggregate = load(&path).unwrap().remove("T-LEGACY").unwrap();
        assert_eq!(aggregate.seal(), CompletionSeal::Commit);
        assert_eq!(
            aggregate.state(),
            &CompletionState::CommandInFlight {
                correlation: correlation("legacy-command"),
                deadline: deadline(0),
            }
        );

        let rejected = append(
            &path,
            CompletionJournalTransition::Rejected {
                key: key("T-LEGACY", "1", 1),
                correlation: Some(correlation("legacy-command")),
                reason: "reconciliation deadline exceeded".to_string(),
                retryability: Retryability::ActionRequired,
            },
        )
        .unwrap();
        assert!(matches!(
            rejected.state(),
            CompletionState::Rejected {
                retryability: Retryability::ActionRequired,
                ..
            }
        ));
        assert!(rejected.masks_durable_done());
    }

    #[test]
    fn new_rows_carry_the_pinned_seal_and_mixed_generations_fail_closed() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion-journal.jsonl");
        let generation = key("T-SEAL", "1", 1);

        let requested = append_with_seal(
            &path,
            CompletionSeal::Journal,
            CompletionJournalTransition::Requested {
                key: generation.clone(),
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Review,
                note: None,
            },
        )
        .unwrap();
        assert_eq!(requested.seal(), CompletionSeal::Journal);
        let requested_bytes = fs::read(&path).unwrap();
        let requested_json = String::from_utf8_lossy(&requested_bytes);
        assert!(requested_json.contains("\"schema_version\":5"));
        assert!(requested_json.contains("\"seal\":\"journal\""));

        let error = append_with_seal(
            &path,
            CompletionSeal::Commit,
            CompletionJournalTransition::CommandInFlight {
                key: generation,
                correlation: correlation("mixed-tier-command"),
                deadline: deadline(42),
            },
        )
        .unwrap_err();
        assert!(error.contains("seal mismatch"));
        assert_eq!(fs::read(&path).unwrap(), requested_bytes);
    }
}
