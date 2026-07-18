//! Durable append-only completion aggregate journal.
//!
//! Records are logically appended as compact JSONL. Each append validates and
//! folds the complete prior history, adds one record in memory, then publishes
//! the complete new bytes through [`RustPublication`]. Readers therefore see
//! either the prior complete history or the new complete history, never a torn
//! final transition.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use lisa_core::completion::{
    reduce, AttemptId, CompletionContentHash, CompletionDeadline, CompletionEvent,
    CompletionGenerationId, CompletionId, CompletionSeal, CompletionSealReceipt, CompletionState,
    CorrelationId, LaunchFailure, Retryability,
};
use lisa_core::disposition::DispositionNote;
use lisa_core::ticket;
use lisa_core::types::{Phase, TicketId, TicketStatus};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::publication::{
    publication_nonce, PublicationErrors, PublicationPath, RustPublication, TemporaryName,
};

const LEGACY_SCHEMA_VERSION: u32 = 1;
const SCHEMA_VERSION: u32 = 5;
const TEMPORARY_PREFIX: &str = ".completion-journal.jsonl.tmp.";
const TICKET_TEMPORARY_PREFIX: &str = ".journal-completion-ticket.tmp.";

/// One durable completion transition requested by the plugin adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompletionJournalTransition {
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
pub(crate) enum CompletionFailureClass {
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
pub(crate) enum FailureConsequence {
    RetryScheduled,
    RetryExhausted,
    Park,
}

/// Latest typed state reconstructed for one completion/ticket aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionJournalAggregate {
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
}

impl CompletionJournalAggregate {
    pub(crate) fn completion_key(&self) -> &CompletionGenerationId {
        &self.completion_key
    }

    #[cfg(test)]
    pub(crate) fn seal(&self) -> CompletionSeal {
        self.seal
    }

    pub(crate) fn state(&self) -> &CompletionState {
        &self.state
    }

    pub(crate) fn prior_phase(&self) -> Phase {
        self.prior_phase
    }

    pub(crate) fn prior_status(&self) -> TicketStatus {
        self.prior_status
    }

    pub(crate) fn failure_count(&self) -> u8 {
        self.failure_count
    }

    #[cfg(test)]
    pub(crate) fn failure_limit(&self) -> Option<u8> {
        self.failure_limit
    }

    pub(crate) fn retries_exhausted(&self) -> bool {
        self.retries_exhausted
    }

    #[cfg(test)]
    pub(crate) fn confirmed_commit_id(&self) -> Option<&str> {
        self.confirmed_receipt
            .as_ref()
            .and_then(CompletionSealReceipt::commit_id)
    }

    #[cfg(test)]
    pub(crate) fn confirmed_receipt(&self) -> Option<&CompletionSealReceipt> {
        self.confirmed_receipt.as_ref()
    }

    pub(crate) fn completion_note(&self) -> Option<&DispositionNote> {
        self.completion_note.as_ref()
    }

    pub(crate) fn masks_durable_done(&self) -> bool {
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

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn completion_content_path(project_root: &Path, path: &Path) -> Result<String, String> {
    // In the WASM sandbox, ticket and work paths are already project-relative
    // (the plugin's cwd is the /host project mount) while `project_root` is
    // the absolute HOST path kept for host-side run_command — stripping one
    // against the other can never succeed there. A relative input is accepted
    // as already project-relative; the traversal guard below still applies.
    // (2026-07-18 rc.3 field stall: every journal seal failed this strip.)
    let relative = match path.strip_prefix(project_root) {
        Ok(relative) => relative,
        Err(_) if path.is_relative() => path,
        Err(_) => {
            return Err(format!(
                "completion content path {} is outside project root {}",
                path.display(),
                project_root.display()
            ));
        }
    };
    if relative.as_os_str().is_empty() {
        return Err(format!(
            "completion content path {} must name a file below project root {}",
            path.display(),
            project_root.display()
        ));
    }
    if relative
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!(
            "completion content path {} escapes the project root",
            path.display()
        ));
    }
    Ok(relative.display().to_string())
}

fn content_hash(
    project_root: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<CompletionContentHash, String> {
    CompletionContentHash::new(completion_content_path(project_root, path)?, sha256(bytes))
}

fn collect_work_hashes(
    project_root: &Path,
    directory: &Path,
    hashes: &mut Vec<CompletionContentHash>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "cannot enumerate completion artifact directory {}: {error}",
            directory.display()
        )
    })?;
    let mut entries = entries
        .map(|entry| {
            entry.map_err(|error| {
                format!(
                    "cannot inspect completion artifact entry under {}: {error}",
                    directory.display()
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "cannot inspect completion artifact {}: {error}",
                path.display()
            )
        })?;
        if file_type.is_dir() {
            collect_work_hashes(project_root, &path, hashes)?;
        } else if file_type.is_file() || file_type.is_symlink() {
            let bytes = fs::read(&path).map_err(|error| {
                format!(
                    "cannot read and hash completion artifact {}: {error}",
                    path.display()
                )
            })?;
            hashes.push(content_hash(project_root, &path, &bytes)?);
        } else {
            return Err(format!(
                "cannot hash unsupported completion artifact {}: expected a regular file",
                path.display()
            ));
        }
    }
    Ok(())
}

fn prepare_done_ticket(ticket_file: &Path) -> Result<Vec<u8>, String> {
    let original = fs::read(ticket_file).map_err(|error| {
        format!(
            "cannot read completion ticket {}: {error}",
            ticket_file.display()
        )
    })?;
    let parent = ticket_file.parent().unwrap_or_else(|| Path::new(""));
    let prepared_path = parent.join(format!("{TICKET_TEMPORARY_PREFIX}{}", publication_nonce()));
    fs::write(&prepared_path, original).map_err(|error| {
        format!(
            "cannot write completion ticket preparation {}: {error}",
            prepared_path.display()
        )
    })?;

    let prepared = (|| {
        ticket::update_ticket_done(&prepared_path).map_err(|error| {
            format!(
                "cannot prepare completion ticket {}: {error}",
                ticket_file.display()
            )
        })?;
        fs::read(&prepared_path).map_err(|error| {
            format!(
                "cannot read prepared completion ticket {}: {error}",
                prepared_path.display()
            )
        })
    })();
    let cleanup = fs::remove_file(&prepared_path).map_err(|error| {
        format!(
            "cannot remove completion ticket preparation {}: {error}",
            prepared_path.display()
        )
    });
    match (prepared, cleanup) {
        (Ok(bytes), Ok(())) => Ok(bytes),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(primary), Err(cleanup)) => Err(format!("{primary}; cleanup also failed: {cleanup}")),
    }
}

fn complete_with_journal_seal_and_publish<F>(
    project_root: &Path,
    ticket_file: &Path,
    work_dir: &Path,
    publish: F,
) -> Result<CompletionSealReceipt, String>
where
    F: FnOnce(&Path, &[u8]) -> Result<(), String>,
{
    let done_ticket = prepare_done_ticket(ticket_file)?;
    let mut hashes = vec![content_hash(project_root, ticket_file, &done_ticket)?];
    collect_work_hashes(project_root, work_dir, &mut hashes)?;
    hashes.sort_by(|left, right| left.path().cmp(right.path()));
    let receipt = CompletionSealReceipt::journal(hashes)?;
    publish(ticket_file, &done_ticket)?;
    Ok(receipt)
}

/// Hash every retained completion artifact and atomically publish Done ticket bytes.
pub(crate) fn complete_with_journal_seal(
    project_root: &Path,
    ticket_file: &Path,
    work_dir: &Path,
) -> Result<CompletionSealReceipt, String> {
    complete_with_journal_seal_and_publish(
        project_root,
        ticket_file,
        work_dir,
        |destination, body| {
            RustPublication {
                path: PublicationPath {
                    destination: destination.to_path_buf(),
                    temporary_name: TemporaryName::Nonce {
                        prefix: TICKET_TEMPORARY_PREFIX.to_string(),
                    },
                },
                body,
                errors: PublicationErrors {
                    write: "cannot write completed ticket temporary",
                    publish: "cannot publish completed ticket",
                },
            }
            .publish()
            .map(|_| ())
        },
    )
}

/// Reconstruct every aggregate from a complete journal.
pub(crate) fn load(path: &Path) -> Result<HashMap<TicketId, CompletionJournalAggregate>, String> {
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
pub(crate) fn append_with_seal(
    path: &Path,
    seal: CompletionSeal,
    transition: CompletionJournalTransition,
) -> Result<CompletionJournalAggregate, String> {
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
    RustPublication {
        path: PublicationPath {
            destination: path.to_path_buf(),
            temporary_name: TemporaryName::Nonce {
                prefix: TEMPORARY_PREFIX.to_string(),
            },
        },
        body: &bytes,
        errors: PublicationErrors {
            write: "cannot write completion journal temporary",
            publish: "cannot publish completion journal",
        },
    }
    .publish()?;

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

    #[test]
    fn content_path_accepts_project_relative_inputs_from_the_sandbox() {
        // Field shape (2026-07-18 rc.3 stall): the plugin's cwd is the /host
        // project mount, so seal inputs are project-relative while
        // project_root is the absolute HOST path — the strip can never match.
        let root = Path::new("/home/tester/demo");
        assert_eq!(
            completion_content_path(root, Path::new("docs/active/tickets/T-001.md")).unwrap(),
            "docs/active/tickets/T-001.md"
        );
        assert_eq!(
            completion_content_path(root, Path::new("/home/tester/demo/review.md")).unwrap(),
            "review.md"
        );
        assert!(completion_content_path(root, Path::new("../escape.md"))
            .unwrap_err()
            .contains("escapes the project root"));
        assert!(completion_content_path(root, Path::new("/etc/passwd"))
            .unwrap_err()
            .contains("outside project root"));
    }

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
    fn repo_less_journal_seal_hashes_final_ticket_and_every_nested_artifact() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        let ticket = project.join("tickets/T-JOURNAL.md");
        let work = project.join("work/T-JOURNAL");
        fs::create_dir_all(work.join("nested")).unwrap();
        let original = b"---\nid: T-JOURNAL\nstatus: open\nphase: review\n---\nBody\n";
        fs::create_dir_all(ticket.parent().unwrap()).unwrap();
        fs::write(&ticket, original).unwrap();
        fs::write(work.join("review.md"), b"# Review\n").unwrap();
        fs::write(work.join("nested/evidence.bin"), [0, 1, 2, 255]).unwrap();

        let receipt = complete_with_journal_seal(project, &ticket, &work).unwrap();

        assert!(!project.join(".git").exists());
        let done = fs::read(&ticket).unwrap();
        let done_text = String::from_utf8_lossy(&done);
        assert!(done_text.contains("status: done"));
        assert!(done_text.contains("phase: done"));
        assert_eq!(receipt.seal(), CompletionSeal::Journal);
        let hashes = receipt.content_hashes();
        assert_eq!(hashes.len(), 3);
        assert_eq!(hashes[0].path(), "tickets/T-JOURNAL.md");
        assert_eq!(hashes[0].sha256(), sha256(&done));
        assert_eq!(hashes[1].path(), "work/T-JOURNAL/nested/evidence.bin");
        assert_eq!(
            hashes[1].sha256(),
            sha256(&fs::read(work.join("nested/evidence.bin")).unwrap())
        );
        assert_eq!(hashes[2].path(), "work/T-JOURNAL/review.md");
        assert_eq!(
            hashes[2].sha256(),
            sha256(&fs::read(work.join("review.md")).unwrap())
        );

        let sealed_review_hash = hashes[2].sha256().to_string();
        fs::write(work.join("review.md"), b"# Mutated after seal\n").unwrap();
        assert_ne!(
            sealed_review_hash,
            sha256(&fs::read(work.join("review.md")).unwrap()),
            "post-seal mutation must make the recorded content hash detectably stale"
        );
        assert_eq!(
            fs::read_dir(ticket.parent().unwrap()).unwrap().count(),
            1,
            "journal completion must remove every sibling temporary"
        );
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_journal_artifact_names_the_path_and_preserves_review_ticket() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        let ticket = project.join("tickets/T-UNREADABLE.md");
        let work = project.join("work/T-UNREADABLE");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(ticket.parent().unwrap()).unwrap();
        let original = b"---\nid: T-UNREADABLE\nstatus: open\nphase: review\n---\nBody\n";
        fs::write(&ticket, original).unwrap();
        let unreadable = work.join("missing-evidence.md");
        symlink("does-not-exist", &unreadable).unwrap();

        let error = complete_with_journal_seal(project, &ticket, &work).unwrap_err();

        assert!(error.contains("cannot read and hash completion artifact"));
        assert!(error.contains("missing-evidence.md"));
        assert_eq!(fs::read(&ticket).unwrap(), original);
        assert_eq!(
            fs::read_dir(ticket.parent().unwrap()).unwrap().count(),
            1,
            "failed hashing must clean the prepared ticket sibling"
        );
    }

    #[test]
    fn interrupted_ticket_publication_preserves_exact_review_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        let ticket = project.join("tickets/T-INTERRUPTED.md");
        let work = project.join("work/T-INTERRUPTED");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(ticket.parent().unwrap()).unwrap();
        let original = b"---\nid: T-INTERRUPTED\nstatus: open\nphase: review\n---\nBody\n";
        fs::write(&ticket, original).unwrap();
        fs::write(work.join("review.md"), b"# Review\n").unwrap();

        let error = complete_with_journal_seal_and_publish(
            project,
            &ticket,
            &work,
            |destination, prepared| {
                assert_eq!(fs::read(destination).unwrap(), original);
                assert!(String::from_utf8_lossy(prepared).contains("status: done"));
                Err("hostile interruption before atomic rename".to_string())
            },
        )
        .unwrap_err();

        assert_eq!(error, "hostile interruption before atomic rename");
        assert_eq!(fs::read(&ticket).unwrap(), original);
        assert_eq!(fs::read_dir(ticket.parent().unwrap()).unwrap().count(), 1);
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
