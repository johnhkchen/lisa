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
    reduce, AttemptId, CompletionDeadline, CompletionEvent, CompletionGenerationId, CompletionId,
    CompletionSeal, CompletionState, CorrelationId, LaunchFailure, Retryability,
};
use lisa_core::types::{Phase, TicketId, TicketStatus};
use serde::{Deserialize, Serialize};

use crate::publication::{PublicationErrors, PublicationPath, RustPublication, TemporaryName};

const LEGACY_SCHEMA_VERSION: u32 = 1;
const SCHEMA_VERSION: u32 = 2;
const TEMPORARY_PREFIX: &str = ".completion-journal.jsonl.tmp.";

/// One durable completion transition requested by the plugin adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompletionJournalTransition {
    Requested {
        key: CompletionGenerationId,
        prior_phase: Phase,
        prior_status: TicketStatus,
    },
    CommandInFlight {
        key: CompletionGenerationId,
        correlation: CorrelationId,
        deadline: CompletionDeadline,
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
        commit_id: String,
    },
}

impl CompletionJournalTransition {
    fn key(&self) -> &CompletionGenerationId {
        match self {
            Self::Requested { key, .. }
            | Self::CommandInFlight { key, .. }
            | Self::Rejected { key, .. }
            | Self::Confirmed { key, .. } => key,
        }
    }
}

/// Latest typed state reconstructed for one completion/ticket aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionJournalAggregate {
    completion_key: CompletionGenerationId,
    seal: CompletionSeal,
    state: CompletionState,
    prior_phase: Phase,
    prior_status: TicketStatus,
    confirmed_commit_id: Option<String>,
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

    #[cfg(test)]
    pub(crate) fn confirmed_commit_id(&self) -> Option<&str> {
        self.confirmed_commit_id.as_deref()
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
    },
    CommandInFlight {
        completion_id: String,
        attempt_id: String,
        generation: u64,
        correlation_id: String,
        #[serde(default)]
        reconciliation_deadline_unix_ms: Option<u64>,
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
        commit_id: String,
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
            } => JournalRecordBody::Requested {
                completion_id: key.completion_id().to_string(),
                attempt_id: key.attempt_id().to_string(),
                generation: key.generation(),
                prior_phase: *prior_phase,
                prior_status: *prior_status,
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
                commit_id,
            } => JournalRecordBody::Confirmed {
                completion_id: key.completion_id().to_string(),
                attempt_id: key.attempt_id().to_string(),
                generation: key.generation(),
                correlation_id: correlation.to_string(),
                commit_id: commit_id.clone(),
            },
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
                "unsupported completion journal schema version {} (expected {LEGACY_SCHEMA_VERSION} or {SCHEMA_VERSION})",
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
            } => CompletionJournalTransition::Requested {
                key: generation_key(completion_id, attempt_id, generation),
                prior_phase,
                prior_status,
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
            } => CompletionJournalTransition::Confirmed {
                key: generation_key(completion_id, attempt_id, generation),
                correlation: CorrelationId::new(correlation_id),
                commit_id,
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
                confirmed_commit_id: None,
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
            aggregate.confirmed_commit_id = None;
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
            aggregate.confirmed_commit_id = None;
            aggregate
        }
        CompletionJournalTransition::Confirmed {
            key,
            correlation,
            commit_id,
        } => {
            if commit_id.trim().is_empty() {
                return Err(format!(
                    "confirmed transition for {ticket_id} requires a commit id"
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
            aggregate.confirmed_commit_id = Some(commit_id);
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
                commit_id: commit_id.clone(),
            },
        )
        .unwrap();
        assert_eq!(confirmed.state(), &CompletionState::Confirmed);
        assert_eq!(confirmed.confirmed_commit_id(), Some(commit_id.as_str()));
        assert!(!confirmed.masks_durable_done());
        assert_eq!(load(&path).unwrap().get("T-042-02-02"), Some(&confirmed));

        let body = fs::read_to_string(&path).unwrap();
        assert_eq!(body.lines().count(), 3);
        assert_eq!(body.matches("\"schema_version\":2").count(), 3);
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
                commit_id: "c".repeat(40),
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
                commit_id: "b".repeat(40),
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
            },
        )
        .unwrap();
        assert_eq!(requested.seal(), CompletionSeal::Journal);
        let requested_bytes = fs::read(&path).unwrap();
        let requested_json = String::from_utf8_lossy(&requested_bytes);
        assert!(requested_json.contains("\"schema_version\":2"));
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
