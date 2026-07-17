//! Pure domain vocabulary for ticket completion.
//!
//! This module describes completion state, inputs, and requested effects. It
//! deliberately performs no I/O and has no dependency on the scheduler,
//! Zellij, or a command runtime. Reducer and reconciliation behavior build on
//! these types in later modules of work.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::disposition::ReviewDisposition;

/// The durable guard that authorizes a ticket's completion.
///
/// `Commit` is Lisa's existing atomic Git transaction. `Journal` is the
/// lesser, explicitly labelled seal whose mechanics are implemented by the
/// journal-completion work. `Auto` is deliberately absent: it is configuration
/// intent, never a runtime seal.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum CompletionSeal {
    /// Finished work is made durable in repository history.
    #[default]
    Commit,
    /// Finished work is made durable in the completion journal.
    Journal,
}

impl CompletionSeal {
    /// Stable accepted runtime seal names.
    pub const VALID: [&'static str; 2] = ["commit", "journal"];

    /// Return the stable lowercase representation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Journal => "journal",
        }
    }

    /// Parse a runtime seal without accepting configured-only `auto`.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "commit" => Ok(Self::Commit),
            "journal" => Ok(Self::Journal),
            other => Err(format!(
                "unknown completion seal {other:?}; expected one of: {}",
                Self::VALID.join(", ")
            )),
        }
    }
}

impl fmt::Display for CompletionSeal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Configured completion stance from `.lisa.toml` `[guards].completion`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum CompletionSealMode {
    /// Select the strongest completion seal supported at loop startup.
    #[default]
    Auto,
    /// Require the existing atomic repository transaction without fallback.
    Commit,
    /// Deliberately use the journal seal even when commit support is available.
    Journal,
}

impl CompletionSealMode {
    /// Stable accepted configuration values.
    pub const VALID: [&'static str; 3] = ["auto", "commit", "journal"];

    /// Return the stable lowercase representation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Commit => "commit",
            Self::Journal => "journal",
        }
    }

    /// Parse one configured completion stance.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "commit" => Ok(Self::Commit),
            "journal" => Ok(Self::Journal),
            other => Err(format!(
                "unknown [guards].completion value {other:?}; expected one of: {}",
                Self::VALID.join(", ")
            )),
        }
    }

    /// Return the requested runtime tier when the stance is explicit.
    pub const fn explicit_seal(self) -> Option<CompletionSeal> {
        match self {
            Self::Auto => None,
            Self::Commit => Some(CompletionSeal::Commit),
            Self::Journal => Some(CompletionSeal::Journal),
        }
    }
}

impl fmt::Display for CompletionSealMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why the current environment cannot satisfy the commit seal.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CommitSealUnavailable {
    /// The project is not inside a discoverable repository.
    #[error("no repository is present")]
    RepositoryMissing,
    /// Git cannot resolve the commit identity required by the transaction.
    #[error("no commit identity is configured (git config user.email did not resolve)")]
    IdentityMissing,
    /// A prerequisite of the existing isolated transaction is unavailable.
    #[error("the commit transaction path is unavailable: {detail}")]
    TransactionUnavailable {
        /// Concise diagnostic from the failed transaction prerequisite.
        detail: String,
    },
}

/// Result of probing whether the existing commit transaction is satisfiable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitSealSupport {
    /// All startup prerequisites for the commit transaction are available.
    Available,
    /// The strongest seal cannot be used for the retained typed reason.
    Unavailable(CommitSealUnavailable),
}

/// One immutable completion-seal decision pinned for a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCompletionSeal {
    seal: CompletionSeal,
    commit_unavailable: Option<CommitSealUnavailable>,
}

impl ResolvedCompletionSeal {
    /// Return the pinned runtime tier.
    pub const fn seal(&self) -> CompletionSeal {
        self.seal
    }

    /// Return why auto selected journal, if it did.
    ///
    /// Deliberately configured journal has no unavailable reason.
    pub fn commit_unavailable(&self) -> Option<&CommitSealUnavailable> {
        self.commit_unavailable.as_ref()
    }
}

/// Explicit commit was requested in an environment that cannot satisfy it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("required commit completion seal is unavailable: {reason}")]
pub struct CompletionSealResolutionError {
    reason: CommitSealUnavailable,
}

impl CompletionSealResolutionError {
    /// Borrow the exact unsatisfied prerequisite.
    pub fn reason(&self) -> &CommitSealUnavailable {
        &self.reason
    }
}

/// Pin the completion seal selected by configured intent and probed support.
pub fn resolve_completion_seal(
    mode: CompletionSealMode,
    support: CommitSealSupport,
) -> Result<ResolvedCompletionSeal, CompletionSealResolutionError> {
    match (mode, support) {
        (CompletionSealMode::Journal, _) => Ok(ResolvedCompletionSeal {
            seal: CompletionSeal::Journal,
            commit_unavailable: None,
        }),
        (CompletionSealMode::Auto | CompletionSealMode::Commit, CommitSealSupport::Available) => {
            Ok(ResolvedCompletionSeal {
                seal: CompletionSeal::Commit,
                commit_unavailable: None,
            })
        }
        (CompletionSealMode::Auto, CommitSealSupport::Unavailable(reason)) => {
            Ok(ResolvedCompletionSeal {
                seal: CompletionSeal::Journal,
                commit_unavailable: Some(reason),
            })
        }
        (CompletionSealMode::Commit, CommitSealSupport::Unavailable(reason)) => {
            Err(CompletionSealResolutionError { reason })
        }
    }
}

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Create an identity from its opaque value.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Borrow the identity's opaque value.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }
    };
}

string_id!(
    /// Identifies the execution attempt claiming completion authority.
    AttemptId
);
string_id!(
    /// Identifies one completion aggregate instance.
    CompletionId
);
string_id!(
    /// Correlates an asynchronous command launch with its result.
    CorrelationId
);

/// Absolute wall-clock deadline for reconciling an unresolved command.
///
/// The opaque value is Unix epoch milliseconds so adapters can persist it
/// across process restarts without coupling the domain model to a clock or
/// serialization implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompletionDeadline(u64);

impl CompletionDeadline {
    /// Reconstruct a deadline from its durable Unix epoch millisecond value.
    pub fn from_unix_millis(value: u64) -> Self {
        Self(value)
    }

    /// Return the durable Unix epoch millisecond value.
    pub fn unix_millis(self) -> u64 {
        self.0
    }

    /// Return whether this deadline has elapsed at `now`.
    pub fn is_expired_at(self, now: Self) -> bool {
        now >= self
    }
}

/// Identifies one generation of a completion transaction for one ticket attempt.
///
/// The component fields remain typed so adapters cannot accidentally detach a
/// generation from the completion aggregate or attempt that owns it. Its
/// [`fmt::Display`] representation is stable ASCII suitable for durable commit
/// metadata and journal records.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompletionGenerationId {
    completion_id: CompletionId,
    attempt_id: AttemptId,
    generation: u64,
}

impl CompletionGenerationId {
    /// Bind a completion generation to its aggregate and authoritative attempt.
    pub fn new(completion_id: CompletionId, attempt_id: AttemptId, generation: u64) -> Self {
        Self {
            completion_id,
            attempt_id,
            generation,
        }
    }

    /// Borrow the completion aggregate identity, populated with the ticket ID.
    pub fn completion_id(&self) -> &CompletionId {
        &self.completion_id
    }

    /// Borrow the attempt identity authorized to complete the ticket.
    pub fn attempt_id(&self) -> &AttemptId {
        &self.attempt_id
    }

    /// Return this attempt's completion-command generation.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl fmt::Display for CompletionGenerationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("v1:")?;
        write_hex(f, self.completion_id.as_str().as_bytes())?;
        f.write_str(":")?;
        write_hex(f, self.attempt_id.as_str().as_bytes())?;
        write!(f, ":{}", self.generation)
    }
}

fn write_hex(f: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        f.write_str(
            std::str::from_utf8(&[HEX[(byte >> 4) as usize], HEX[(byte & 0x0f) as usize]])
                .expect("hexadecimal digits are UTF-8"),
        )?;
    }
    Ok(())
}

/// A completion artifact admitted as belonging to the authoritative attempt.
///
/// The adapter is responsible for checking the current lease before creating
/// this value. Reconciliation consumes the admitted fact without depending on
/// scheduler or filesystem types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentLeaseArtifactAdmission {
    /// Attempt whose artifact passed the lease boundary.
    pub attempt_id: AttemptId,
    /// Completion aggregate associated with the admitted artifact.
    pub completion_id: CompletionId,
}

/// Durable facts from which completion eligibility is re-derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableCompletionInputs {
    /// Current-attempt artifact admission, absent until authority is proven.
    pub artifact_admission: Option<CurrentLeaseArtifactAdmission>,
    /// Fail-closed Review verdict parsed at the adapter boundary.
    pub disposition: ReviewDisposition,
}

/// Whether a rejected completion can be retried without operator action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Retryability {
    /// Reconciliation may safely request another attempt.
    Retryable,
    /// A person or external state change must resolve the rejection.
    ActionRequired,
}

/// An owned adapter failure retained as the source of a launch rejection.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct LaunchFailure {
    message: String,
}

impl LaunchFailure {
    /// Create a launch failure from an operator-visible message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Borrow the underlying failure message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Why a completion transition was refused.
///
/// Each outcome is independently matchable; callers never need to infer the
/// reason from a boolean or parse its Display representation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CompletionRejection {
    /// Another completion request already owns the aggregate.
    #[error("completion {completion_id} is already pending")]
    AlreadyPending {
        /// The completion that is already pending.
        completion_id: CompletionId,
    },
    /// The requesting attempt no longer holds the current lease.
    #[error("attempt {attempt_id} holds a stale completion lease")]
    StaleLease {
        /// The stale attempt that requested completion.
        attempt_id: AttemptId,
    },
    /// The admitted Review disposition does not authorize completion.
    #[error("review disposition blocks completion: {reason}")]
    DispositionBlocked {
        /// Operator-visible disposition detail.
        reason: String,
    },
    /// One or more ticket dependencies have not completed.
    #[error("ticket dependencies block completion: {reason}")]
    DependencyBlocked {
        /// Operator-visible dependency detail.
        reason: String,
    },
    /// The adapter could not launch the requested completion command.
    #[error("completion command launch failed")]
    LaunchFailed {
        /// The adapter-neutral underlying failure.
        #[source]
        source: LaunchFailure,
    },
    /// The event cannot be applied in the aggregate's current lifecycle state.
    #[error("completion event {event} is invalid in state {state}")]
    UnexpectedEvent {
        /// Stable name of the state that refused the event.
        state: &'static str,
        /// Stable name of the event that was refused.
        event: &'static str,
    },
    /// An asynchronous result belongs to a different launched command.
    #[error("completion command correlation mismatch: expected {expected}, got {actual}")]
    CorrelationMismatch {
        /// Correlation retained by the in-flight state.
        expected: CorrelationId,
        /// Correlation carried by the refused result event.
        actual: CorrelationId,
    },
}

/// Lifecycle state of one completion aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionState {
    /// Durable inputs currently authorize a completion request.
    Eligible,
    /// A request was accepted and its launch effect was emitted.
    Requested,
    /// The external command has launched and awaits a correlated result.
    CommandInFlight {
        /// Mandatory identity for matching the asynchronous result.
        correlation: CorrelationId,
        /// Absolute bound shared by every reconciliation replay.
        deadline: CompletionDeadline,
    },
    /// The request was refused or its command failed.
    Rejected {
        /// Typed reason for the rejection.
        reason: CompletionRejection,
        /// Whether automatic retry is safe.
        retryability: Retryability,
    },
    /// The authoritative completion result was confirmed.
    Confirmed,
}

/// A typed fact presented to the completion aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionEvent {
    /// Completion was requested by an attempt for an aggregate instance.
    Request {
        /// Attempt claiming completion authority.
        attempt_id: AttemptId,
        /// Completion aggregate instance being requested.
        completion_id: CompletionId,
    },
    /// The adapter launched the command and assigned its correlation identity.
    CommandLaunched {
        /// Identity used to match the eventual result.
        correlation: CorrelationId,
        /// Absolute bound for confirming this command and every replay.
        deadline: CompletionDeadline,
    },
    /// The adapter failed before a command entered the in-flight state.
    CommandLaunchFailed {
        /// Adapter-neutral launch failure.
        source: LaunchFailure,
    },
    /// The correlated command confirmed authoritative completion.
    CommandSucceeded {
        /// Identity of the completed command.
        correlation: CorrelationId,
    },
    /// The correlated command returned a failure.
    CommandFailed {
        /// Identity of the failed command.
        correlation: CorrelationId,
        /// Adapter-neutral command failure.
        source: LaunchFailure,
        /// Whether reconciliation may retry the command.
        retryability: Retryability,
    },
}

/// An external action requested by a pure completion transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectCommand {
    /// Launch the isolated completion transaction.
    LaunchCompletion {
        /// Attempt whose lease authorizes the transaction.
        attempt_id: AttemptId,
        /// Completion aggregate instance used for idempotent attribution.
        completion_id: CompletionId,
    },
}

/// The accepted output of one completion-domain decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    /// Aggregate state after accepting the event.
    pub state: CompletionState,
    /// The only external command requested by the transition, if any.
    pub effect: Option<EffectCommand>,
}

/// One level-triggered completion reconciliation decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconciliation {
    /// The durable obligation requests one external command.
    Effect(EffectCommand),
    /// No new action is required.
    None,
    /// Replay the exact unresolved command while its durable bound remains.
    ReplayCommandInFlight {
        /// Identity of the exact unresolved command.
        correlation: CorrelationId,
        /// Original absolute bound; a replay must not extend it.
        deadline: CompletionDeadline,
    },
    /// The unresolved command exhausted its durable reconciliation window.
    CommandInFlightDeadlineExceeded {
        /// Identity of the exact unresolved command.
        correlation: CorrelationId,
        /// Original absolute bound that has elapsed.
        deadline: CompletionDeadline,
    },
}

/// Fold one typed event through the completion aggregate.
///
/// This function only transforms owned domain values. Effect commands are
/// returned as inert data for an adapter to execute; the reducer performs no
/// I/O and mutates no external state.
pub fn reduce(
    state: CompletionState,
    event: CompletionEvent,
) -> Result<Transition, CompletionRejection> {
    match state {
        CompletionState::Eligible => match event {
            CompletionEvent::Request {
                attempt_id,
                completion_id,
            } => Ok(request_transition(attempt_id, completion_id)),
            CompletionEvent::CommandLaunched {
                correlation,
                deadline,
            } => Err(unexpected(
                &CompletionState::Eligible,
                &CompletionEvent::CommandLaunched {
                    correlation,
                    deadline,
                },
            )),
            CompletionEvent::CommandLaunchFailed { source } => Err(unexpected(
                &CompletionState::Eligible,
                &CompletionEvent::CommandLaunchFailed { source },
            )),
            CompletionEvent::CommandSucceeded { correlation } => Err(unexpected(
                &CompletionState::Eligible,
                &CompletionEvent::CommandSucceeded { correlation },
            )),
            CompletionEvent::CommandFailed {
                correlation,
                source,
                retryability,
            } => Err(unexpected(
                &CompletionState::Eligible,
                &CompletionEvent::CommandFailed {
                    correlation,
                    source,
                    retryability,
                },
            )),
        },
        CompletionState::Requested => match event {
            CompletionEvent::Request { completion_id, .. } => {
                Err(CompletionRejection::AlreadyPending { completion_id })
            }
            CompletionEvent::CommandLaunched {
                correlation,
                deadline,
            } => Ok(Transition {
                state: CompletionState::CommandInFlight {
                    correlation,
                    deadline,
                },
                effect: None,
            }),
            CompletionEvent::CommandLaunchFailed { source } => {
                Ok(rejected_transition(source, Retryability::Retryable))
            }
            CompletionEvent::CommandSucceeded { correlation } => Err(unexpected(
                &CompletionState::Requested,
                &CompletionEvent::CommandSucceeded { correlation },
            )),
            CompletionEvent::CommandFailed {
                correlation,
                source,
                retryability,
            } => Err(unexpected(
                &CompletionState::Requested,
                &CompletionEvent::CommandFailed {
                    correlation,
                    source,
                    retryability,
                },
            )),
        },
        CompletionState::CommandInFlight {
            correlation,
            deadline,
        } => match event {
            CompletionEvent::Request { completion_id, .. } => {
                Err(CompletionRejection::AlreadyPending { completion_id })
            }
            CompletionEvent::CommandLaunched {
                correlation: actual,
                deadline: actual_deadline,
            } => Err(unexpected(
                &CompletionState::CommandInFlight {
                    correlation,
                    deadline,
                },
                &CompletionEvent::CommandLaunched {
                    correlation: actual,
                    deadline: actual_deadline,
                },
            )),
            CompletionEvent::CommandLaunchFailed { source } => Err(unexpected(
                &CompletionState::CommandInFlight {
                    correlation,
                    deadline,
                },
                &CompletionEvent::CommandLaunchFailed { source },
            )),
            CompletionEvent::CommandSucceeded {
                correlation: actual,
            } => {
                if correlation == actual {
                    Ok(Transition {
                        state: CompletionState::Confirmed,
                        effect: None,
                    })
                } else {
                    Err(CompletionRejection::CorrelationMismatch {
                        expected: correlation,
                        actual,
                    })
                }
            }
            CompletionEvent::CommandFailed {
                correlation: actual,
                source,
                retryability,
            } => {
                if correlation == actual {
                    Ok(rejected_transition(source, retryability))
                } else {
                    Err(CompletionRejection::CorrelationMismatch {
                        expected: correlation,
                        actual,
                    })
                }
            }
        },
        CompletionState::Rejected {
            reason,
            retryability,
        } => match event {
            CompletionEvent::Request {
                attempt_id,
                completion_id,
            } => match retryability {
                Retryability::Retryable => Ok(request_transition(attempt_id, completion_id)),
                Retryability::ActionRequired => Err(reason),
            },
            CompletionEvent::CommandLaunched {
                correlation,
                deadline,
            } => Err(unexpected(
                &CompletionState::Rejected {
                    reason,
                    retryability,
                },
                &CompletionEvent::CommandLaunched {
                    correlation,
                    deadline,
                },
            )),
            CompletionEvent::CommandLaunchFailed { source } => Err(unexpected(
                &CompletionState::Rejected {
                    reason,
                    retryability,
                },
                &CompletionEvent::CommandLaunchFailed { source },
            )),
            CompletionEvent::CommandSucceeded { correlation } => Err(unexpected(
                &CompletionState::Rejected {
                    reason,
                    retryability,
                },
                &CompletionEvent::CommandSucceeded { correlation },
            )),
            CompletionEvent::CommandFailed {
                correlation,
                source,
                retryability: event_retryability,
            } => Err(unexpected(
                &CompletionState::Rejected {
                    reason,
                    retryability,
                },
                &CompletionEvent::CommandFailed {
                    correlation,
                    source,
                    retryability: event_retryability,
                },
            )),
        },
        CompletionState::Confirmed => match event {
            CompletionEvent::Request { completion_id, .. } => {
                Err(CompletionRejection::AlreadyPending { completion_id })
            }
            CompletionEvent::CommandLaunched {
                correlation,
                deadline,
            } => Err(unexpected(
                &CompletionState::Confirmed,
                &CompletionEvent::CommandLaunched {
                    correlation,
                    deadline,
                },
            )),
            CompletionEvent::CommandLaunchFailed { source } => Err(unexpected(
                &CompletionState::Confirmed,
                &CompletionEvent::CommandLaunchFailed { source },
            )),
            CompletionEvent::CommandSucceeded { correlation } => Err(unexpected(
                &CompletionState::Confirmed,
                &CompletionEvent::CommandSucceeded { correlation },
            )),
            CompletionEvent::CommandFailed {
                correlation,
                source,
                retryability,
            } => Err(unexpected(
                &CompletionState::Confirmed,
                &CompletionEvent::CommandFailed {
                    correlation,
                    source,
                    retryability,
                },
            )),
        },
    }
}

/// Re-derive the completion obligation from durable facts and aggregate state.
///
/// Repeated calls remain level-triggered: eligible durable facts request a
/// launch whenever no pending, confirmed, or action-required transaction state
/// suppresses it. This function returns inert decision data and performs no I/O.
pub fn reconcile(
    durable_inputs: &DurableCompletionInputs,
    state: &CompletionState,
    now: CompletionDeadline,
) -> Reconciliation {
    match state {
        CompletionState::CommandInFlight {
            correlation,
            deadline,
        } => {
            return if deadline.is_expired_at(now) {
                Reconciliation::CommandInFlightDeadlineExceeded {
                    correlation: correlation.clone(),
                    deadline: *deadline,
                }
            } else {
                Reconciliation::ReplayCommandInFlight {
                    correlation: correlation.clone(),
                    deadline: *deadline,
                }
            };
        }
        CompletionState::Requested
        | CompletionState::Confirmed
        | CompletionState::Rejected {
            retryability: Retryability::ActionRequired,
            ..
        } => return Reconciliation::None,
        CompletionState::Eligible
        | CompletionState::Rejected {
            retryability: Retryability::Retryable,
            ..
        } => {}
    }

    let Some(admission) = durable_inputs.artifact_admission.as_ref() else {
        return Reconciliation::None;
    };
    if !matches!(durable_inputs.disposition, ReviewDisposition::Pass) {
        return Reconciliation::None;
    }

    Reconciliation::Effect(request_effect(
        admission.attempt_id.clone(),
        admission.completion_id.clone(),
    ))
}

fn request_transition(attempt_id: AttemptId, completion_id: CompletionId) -> Transition {
    Transition {
        state: CompletionState::Requested,
        effect: Some(request_effect(attempt_id, completion_id)),
    }
}

fn request_effect(attempt_id: AttemptId, completion_id: CompletionId) -> EffectCommand {
    EffectCommand::LaunchCompletion {
        attempt_id,
        completion_id,
    }
}

fn rejected_transition(source: LaunchFailure, retryability: Retryability) -> Transition {
    Transition {
        state: CompletionState::Rejected {
            reason: CompletionRejection::LaunchFailed { source },
            retryability,
        },
        effect: None,
    }
}

fn unexpected(state: &CompletionState, event: &CompletionEvent) -> CompletionRejection {
    CompletionRejection::UnexpectedEvent {
        state: state_name(state),
        event: event_name(event),
    }
}

fn state_name(state: &CompletionState) -> &'static str {
    match state {
        CompletionState::Eligible => "eligible",
        CompletionState::Requested => "requested",
        CompletionState::CommandInFlight { .. } => "command-in-flight",
        CompletionState::Rejected { .. } => "rejected",
        CompletionState::Confirmed => "confirmed",
    }
}

fn event_name(event: &CompletionEvent) -> &'static str {
    match event {
        CompletionEvent::Request { .. } => "request",
        CompletionEvent::CommandLaunched { .. } => "command-launched",
        CompletionEvent::CommandLaunchFailed { .. } => "command-launch-failed",
        CompletionEvent::CommandSucceeded { .. } => "command-succeeded",
        CompletionEvent::CommandFailed { .. } => "command-failed",
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    fn deadline(value: u64) -> CompletionDeadline {
        CompletionDeadline::from_unix_millis(value)
    }

    fn unavailable(reason: CommitSealUnavailable) -> CommitSealSupport {
        CommitSealSupport::Unavailable(reason)
    }

    #[test]
    fn completion_seal_strings_separate_configured_intent_from_runtime_tiers() {
        assert_eq!(CompletionSeal::default(), CompletionSeal::Commit);
        assert_eq!(CompletionSeal::parse("commit"), Ok(CompletionSeal::Commit));
        assert_eq!(
            CompletionSeal::parse("journal"),
            Ok(CompletionSeal::Journal)
        );
        assert!(CompletionSeal::parse("auto")
            .unwrap_err()
            .contains("commit, journal"));

        assert_eq!(CompletionSealMode::default(), CompletionSealMode::Auto);
        for (raw, expected) in [
            ("auto", CompletionSealMode::Auto),
            ("commit", CompletionSealMode::Commit),
            ("journal", CompletionSealMode::Journal),
        ] {
            assert_eq!(CompletionSealMode::parse(raw), Ok(expected));
            assert_eq!(expected.to_string(), raw);
        }
        let error = CompletionSealMode::parse("best-effort").unwrap_err();
        assert!(error.contains("[guards].completion"));
        assert!(error.contains("auto, commit, journal"));
    }

    #[test]
    fn auto_pins_commit_only_when_the_transaction_is_supported() {
        let resolved =
            resolve_completion_seal(CompletionSealMode::Auto, CommitSealSupport::Available)
                .unwrap();

        assert_eq!(resolved.seal(), CompletionSeal::Commit);
        assert_eq!(resolved.commit_unavailable(), None);
    }

    #[test]
    fn auto_pins_journal_and_retains_each_unavailable_environment_reason() {
        let reasons = [
            CommitSealUnavailable::RepositoryMissing,
            CommitSealUnavailable::IdentityMissing,
            CommitSealUnavailable::TransactionUnavailable {
                detail: "HEAD is unborn".to_string(),
            },
        ];

        for reason in reasons {
            let resolved =
                resolve_completion_seal(CompletionSealMode::Auto, unavailable(reason.clone()))
                    .unwrap();
            assert_eq!(resolved.seal(), CompletionSeal::Journal);
            assert_eq!(resolved.commit_unavailable(), Some(&reason));
        }
    }

    #[test]
    fn explicit_commit_succeeds_or_returns_the_exact_hard_failure() {
        let resolved =
            resolve_completion_seal(CompletionSealMode::Commit, CommitSealSupport::Available)
                .unwrap();
        assert_eq!(resolved.seal(), CompletionSeal::Commit);

        let reason = CommitSealUnavailable::IdentityMissing;
        let error =
            resolve_completion_seal(CompletionSealMode::Commit, unavailable(reason.clone()))
                .unwrap_err();
        assert_eq!(error.reason(), &reason);
        assert!(error
            .to_string()
            .contains("required commit completion seal"));
    }

    #[test]
    fn explicit_journal_is_honored_without_recording_an_automatic_fallback() {
        for support in [
            CommitSealSupport::Available,
            unavailable(CommitSealUnavailable::IdentityMissing),
        ] {
            let resolved = resolve_completion_seal(CompletionSealMode::Journal, support).unwrap();
            assert_eq!(resolved.seal(), CompletionSeal::Journal);
            assert_eq!(resolved.commit_unavailable(), None);
        }
    }

    #[test]
    fn identity_newtypes_preserve_their_opaque_values() {
        let attempt = AttemptId::new("attempt-7");
        let completion = CompletionId::from("completion-2");
        let correlation = CorrelationId::from(String::from("command-9"));

        assert_eq!(attempt.as_str(), "attempt-7");
        assert_eq!(completion.to_string(), "completion-2");
        assert_eq!(correlation.as_str(), "command-9");
    }

    #[test]
    fn completion_generation_binds_ticket_attempt_and_generation() {
        let id = CompletionGenerationId::new(
            CompletionId::new("T-042:ticket"),
            AttemptId::new("attempt:7"),
            3,
        );

        assert_eq!(id.completion_id().as_str(), "T-042:ticket");
        assert_eq!(id.attempt_id().as_str(), "attempt:7");
        assert_eq!(id.generation(), 3);
        assert_eq!(
            id.to_string(),
            "v1:542d3034323a7469636b6574:617474656d70743a37:3"
        );
    }

    #[test]
    fn completion_generation_changes_with_each_identity_component() {
        let key = CompletionGenerationId::new(CompletionId::new("T-042"), AttemptId::new("1"), 1);
        let different_ticket =
            CompletionGenerationId::new(CompletionId::new("T-043"), AttemptId::new("1"), 1);
        let different_attempt =
            CompletionGenerationId::new(CompletionId::new("T-042"), AttemptId::new("2"), 1);
        let different_generation =
            CompletionGenerationId::new(CompletionId::new("T-042"), AttemptId::new("1"), 2);

        assert_ne!(key, different_ticket);
        assert_ne!(key, different_attempt);
        assert_ne!(key, different_generation);
        assert_eq!(
            CompletionGenerationId::new(CompletionId::new("T-042"), AttemptId::new("1"), 1,),
            key
        );
    }

    #[test]
    fn command_in_flight_contains_correlation_and_deadline() {
        let state = CompletionState::CommandInFlight {
            correlation: CorrelationId::new("command-1"),
            deadline: deadline(42),
        };

        let CompletionState::CommandInFlight {
            correlation,
            deadline: actual_deadline,
        } = state
        else {
            panic!("expected command-in-flight state");
        };
        assert_eq!(correlation.as_str(), "command-1");
        assert_eq!(actual_deadline.unix_millis(), 42);
    }

    #[test]
    fn completion_deadline_expires_inclusively() {
        assert!(!deadline(42).is_expired_at(deadline(41)));
        assert!(deadline(42).is_expired_at(deadline(42)));
        assert!(deadline(42).is_expired_at(deadline(43)));
    }

    #[test]
    fn rejected_state_retains_reason_and_retryability() {
        let state = CompletionState::Rejected {
            reason: CompletionRejection::DependencyBlocked {
                reason: "T-001 is open".into(),
            },
            retryability: Retryability::ActionRequired,
        };

        assert!(matches!(
            state,
            CompletionState::Rejected {
                reason: CompletionRejection::DependencyBlocked { .. },
                retryability: Retryability::ActionRequired,
            }
        ));
    }

    #[test]
    fn transition_carries_at_most_one_effect() {
        let effect = EffectCommand::LaunchCompletion {
            attempt_id: AttemptId::new("attempt-1"),
            completion_id: CompletionId::new("completion-1"),
        };
        let requested = Transition {
            state: CompletionState::Requested,
            effect: Some(effect.clone()),
        };
        let confirmed = Transition {
            state: CompletionState::Confirmed,
            effect: None,
        };

        assert_eq!(requested.effect, Some(effect));
        assert_eq!(confirmed.effect, None);
    }

    fn eligible_durable_inputs() -> DurableCompletionInputs {
        DurableCompletionInputs {
            artifact_admission: Some(CurrentLeaseArtifactAdmission {
                attempt_id: AttemptId::new("attempt-1"),
                completion_id: CompletionId::new("completion-1"),
            }),
            disposition: ReviewDisposition::Pass,
        }
    }

    #[test]
    fn reconcile_eligible_durable_inputs_returns_the_request_effect() {
        assert_eq!(
            reconcile(
                &eligible_durable_inputs(),
                &CompletionState::Eligible,
                deadline(0),
            ),
            Reconciliation::Effect(EffectCommand::LaunchCompletion {
                attempt_id: AttemptId::new("attempt-1"),
                completion_id: CompletionId::new("completion-1"),
            })
        );
    }

    #[test]
    fn reconcile_missing_artifact_admission_is_ineligible() {
        let inputs = DurableCompletionInputs {
            artifact_admission: None,
            disposition: ReviewDisposition::Pass,
        };

        assert_eq!(
            reconcile(&inputs, &CompletionState::Eligible, deadline(0)),
            Reconciliation::None
        );
    }

    #[test]
    fn reconcile_non_passing_dispositions_are_ineligible() {
        for disposition in [
            ReviewDisposition::Block {
                reason: "fix the failing test".into(),
                remedy_owner: crate::disposition::RemedyOwner::Operator,
                ask: "fix the failing test".into(),
                steps: None,
                check: None,
                unstructured: true,
            },
            ReviewDisposition::Invalid {
                reason: "missing reason field".into(),
            },
        ] {
            let inputs = DurableCompletionInputs {
                disposition,
                ..eligible_durable_inputs()
            };

            assert_eq!(
                reconcile(&inputs, &CompletionState::Eligible, deadline(0)),
                Reconciliation::None
            );
        }
    }

    #[test]
    fn reconcile_pending_and_confirmed_transactions_return_none() {
        for state in [CompletionState::Requested, CompletionState::Confirmed] {
            assert_eq!(
                reconcile(&eligible_durable_inputs(), &state, deadline(0)),
                Reconciliation::None
            );
        }
    }

    #[test]
    fn reconcile_retries_only_retryable_rejections() {
        let reason = CompletionRejection::LaunchFailed {
            source: LaunchFailure::new("commit unavailable"),
        };

        assert_eq!(
            reconcile(
                &eligible_durable_inputs(),
                &CompletionState::Rejected {
                    reason: reason.clone(),
                    retryability: Retryability::Retryable,
                },
                deadline(0),
            ),
            Reconciliation::Effect(EffectCommand::LaunchCompletion {
                attempt_id: AttemptId::new("attempt-1"),
                completion_id: CompletionId::new("completion-1"),
            })
        );
        assert_eq!(
            reconcile(
                &eligible_durable_inputs(),
                &CompletionState::Rejected {
                    reason,
                    retryability: Retryability::ActionRequired,
                },
                deadline(0),
            ),
            Reconciliation::None
        );
    }

    #[test]
    fn reconcile_in_flight_replays_only_before_its_deadline() {
        let state = CompletionState::CommandInFlight {
            correlation: CorrelationId::new("command-17"),
            deadline: deadline(200),
        };
        let inputs = DurableCompletionInputs {
            artifact_admission: None,
            disposition: ReviewDisposition::Invalid {
                reason: "durable inputs unavailable during recovery".into(),
            },
        };

        assert_eq!(
            reconcile(&inputs, &state, deadline(199)),
            Reconciliation::ReplayCommandInFlight {
                correlation: CorrelationId::new("command-17"),
                deadline: deadline(200),
            }
        );

        for now in [deadline(200), deadline(201)] {
            assert_eq!(
                reconcile(&inputs, &state, now),
                Reconciliation::CommandInFlightDeadlineExceeded {
                    correlation: CorrelationId::new("command-17"),
                    deadline: deadline(200),
                }
            );
        }
    }

    #[test]
    fn eligible_request_emits_the_single_launch_effect() {
        let attempt_id = AttemptId::new("attempt-1");
        let completion_id = CompletionId::new("completion-1");

        assert_eq!(
            reduce(
                CompletionState::Eligible,
                CompletionEvent::Request {
                    attempt_id: attempt_id.clone(),
                    completion_id: completion_id.clone(),
                },
            ),
            Ok(Transition {
                state: CompletionState::Requested,
                effect: Some(EffectCommand::LaunchCompletion {
                    attempt_id,
                    completion_id,
                }),
            })
        );
    }

    #[test]
    fn requested_launch_records_correlation_without_an_effect() {
        let correlation = CorrelationId::new("command-1");
        let reconciliation_deadline = deadline(42);

        assert_eq!(
            reduce(
                CompletionState::Requested,
                CompletionEvent::CommandLaunched {
                    correlation: correlation.clone(),
                    deadline: reconciliation_deadline,
                },
            ),
            Ok(Transition {
                state: CompletionState::CommandInFlight {
                    correlation,
                    deadline: reconciliation_deadline,
                },
                effect: None,
            })
        );
    }

    #[test]
    fn requested_launch_failure_enters_retryable_rejection_without_an_effect() {
        let source = LaunchFailure::new("binary missing");

        assert_eq!(
            reduce(
                CompletionState::Requested,
                CompletionEvent::CommandLaunchFailed {
                    source: source.clone(),
                },
            ),
            Ok(Transition {
                state: CompletionState::Rejected {
                    reason: CompletionRejection::LaunchFailed { source },
                    retryability: Retryability::Retryable,
                },
                effect: None,
            })
        );
    }

    #[test]
    fn matching_command_success_confirms_without_an_effect() {
        let correlation = CorrelationId::new("command-1");

        assert_eq!(
            reduce(
                CompletionState::CommandInFlight {
                    correlation: correlation.clone(),
                    deadline: deadline(42),
                },
                CompletionEvent::CommandSucceeded { correlation },
            ),
            Ok(Transition {
                state: CompletionState::Confirmed,
                effect: None,
            })
        );
    }

    #[test]
    fn matching_command_failure_preserves_source_and_retryability() {
        let correlation = CorrelationId::new("command-1");
        let source = LaunchFailure::new("commit refused");

        assert_eq!(
            reduce(
                CompletionState::CommandInFlight {
                    correlation: correlation.clone(),
                    deadline: deadline(42),
                },
                CompletionEvent::CommandFailed {
                    correlation,
                    source: source.clone(),
                    retryability: Retryability::ActionRequired,
                },
            ),
            Ok(Transition {
                state: CompletionState::Rejected {
                    reason: CompletionRejection::LaunchFailed { source },
                    retryability: Retryability::ActionRequired,
                },
                effect: None,
            })
        );
    }

    #[test]
    fn retryable_rejection_accepts_a_fresh_request() {
        let attempt_id = AttemptId::new("attempt-2");
        let completion_id = CompletionId::new("completion-2");

        assert_eq!(
            reduce(
                CompletionState::Rejected {
                    reason: CompletionRejection::LaunchFailed {
                        source: LaunchFailure::new("temporary failure"),
                    },
                    retryability: Retryability::Retryable,
                },
                CompletionEvent::Request {
                    attempt_id: attempt_id.clone(),
                    completion_id: completion_id.clone(),
                },
            ),
            Ok(Transition {
                state: CompletionState::Requested,
                effect: Some(EffectCommand::LaunchCompletion {
                    attempt_id,
                    completion_id,
                }),
            })
        );
    }

    #[test]
    fn duplicate_requests_return_already_pending_without_a_transition() {
        for state in [
            CompletionState::Requested,
            CompletionState::CommandInFlight {
                correlation: CorrelationId::new("command-1"),
                deadline: deadline(42),
            },
            CompletionState::Confirmed,
        ] {
            let completion_id = CompletionId::new("completion-2");
            assert_eq!(
                reduce(
                    state,
                    CompletionEvent::Request {
                        attempt_id: AttemptId::new("attempt-2"),
                        completion_id: completion_id.clone(),
                    },
                ),
                Err(CompletionRejection::AlreadyPending { completion_id })
            );
        }
    }

    #[test]
    fn action_required_rejection_refuses_retry_with_its_named_reason() {
        let reason = CompletionRejection::StaleLease {
            attempt_id: AttemptId::new("attempt-1"),
        };

        assert_eq!(
            reduce(
                CompletionState::Rejected {
                    reason: reason.clone(),
                    retryability: Retryability::ActionRequired,
                },
                CompletionEvent::Request {
                    attempt_id: AttemptId::new("attempt-2"),
                    completion_id: CompletionId::new("completion-2"),
                },
            ),
            Err(reason)
        );
    }

    #[test]
    fn mismatched_command_results_return_typed_correlation_rejections() {
        for event in [
            CompletionEvent::CommandSucceeded {
                correlation: CorrelationId::new("actual"),
            },
            CompletionEvent::CommandFailed {
                correlation: CorrelationId::new("actual"),
                source: LaunchFailure::new("failure"),
                retryability: Retryability::Retryable,
            },
        ] {
            assert_eq!(
                reduce(
                    CompletionState::CommandInFlight {
                        correlation: CorrelationId::new("expected"),
                        deadline: deadline(42),
                    },
                    event,
                ),
                Err(CompletionRejection::CorrelationMismatch {
                    expected: CorrelationId::new("expected"),
                    actual: CorrelationId::new("actual"),
                })
            );
        }
    }

    #[test]
    fn illegal_callback_edges_return_exact_unexpected_event_rejections() {
        fn all_callbacks() -> Vec<CompletionEvent> {
            vec![
                CompletionEvent::CommandLaunched {
                    correlation: CorrelationId::new("command-1"),
                    deadline: deadline(42),
                },
                CompletionEvent::CommandLaunchFailed {
                    source: LaunchFailure::new("late failure"),
                },
                CompletionEvent::CommandSucceeded {
                    correlation: CorrelationId::new("command-1"),
                },
                CompletionEvent::CommandFailed {
                    correlation: CorrelationId::new("command-1"),
                    source: LaunchFailure::new("late failure"),
                    retryability: Retryability::Retryable,
                },
            ]
        }

        let cases = [
            (CompletionState::Eligible, "eligible", all_callbacks()),
            (
                CompletionState::Requested,
                "requested",
                vec![
                    CompletionEvent::CommandSucceeded {
                        correlation: CorrelationId::new("command-1"),
                    },
                    CompletionEvent::CommandFailed {
                        correlation: CorrelationId::new("command-1"),
                        source: LaunchFailure::new("early failure"),
                        retryability: Retryability::Retryable,
                    },
                ],
            ),
            (
                CompletionState::CommandInFlight {
                    correlation: CorrelationId::new("command-1"),
                    deadline: deadline(42),
                },
                "command-in-flight",
                vec![
                    CompletionEvent::CommandLaunched {
                        correlation: CorrelationId::new("command-1"),
                        deadline: deadline(42),
                    },
                    CompletionEvent::CommandLaunchFailed {
                        source: LaunchFailure::new("late failure"),
                    },
                ],
            ),
            (
                CompletionState::Rejected {
                    reason: CompletionRejection::DependencyBlocked {
                        reason: "dependency open".into(),
                    },
                    retryability: Retryability::ActionRequired,
                },
                "rejected",
                all_callbacks(),
            ),
            (CompletionState::Confirmed, "confirmed", all_callbacks()),
        ];

        for (state, state_name, events) in cases {
            for event in events {
                let expected_event_name = event_name(&event);
                assert_eq!(
                    reduce(state.clone(), event),
                    Err(CompletionRejection::UnexpectedEvent {
                        state: state_name,
                        event: expected_event_name,
                    })
                );
            }
        }
    }

    #[test]
    fn every_rejection_is_a_distinct_non_boolean_outcome() {
        let cases = [
            CompletionRejection::AlreadyPending {
                completion_id: CompletionId::new("completion-1"),
            },
            CompletionRejection::StaleLease {
                attempt_id: AttemptId::new("attempt-1"),
            },
            CompletionRejection::DispositionBlocked {
                reason: "review blocked".into(),
            },
            CompletionRejection::DependencyBlocked {
                reason: "dependency open".into(),
            },
            CompletionRejection::LaunchFailed {
                source: LaunchFailure::new("process unavailable"),
            },
            CompletionRejection::UnexpectedEvent {
                state: "eligible",
                event: "command-launched",
            },
            CompletionRejection::CorrelationMismatch {
                expected: CorrelationId::new("command-1"),
                actual: CorrelationId::new("command-2"),
            },
        ];

        for rejection in cases {
            assert!(!rejection.to_string().is_empty());
            match rejection {
                CompletionRejection::AlreadyPending { .. }
                | CompletionRejection::StaleLease { .. }
                | CompletionRejection::DispositionBlocked { .. }
                | CompletionRejection::DependencyBlocked { .. }
                | CompletionRejection::LaunchFailed { .. }
                | CompletionRejection::UnexpectedEvent { .. }
                | CompletionRejection::CorrelationMismatch { .. } => {}
            }
        }
    }

    #[test]
    fn launch_rejection_exposes_its_source() {
        let rejection = CompletionRejection::LaunchFailed {
            source: LaunchFailure::new("binary not found"),
        };

        assert_eq!(
            rejection.source().map(ToString::to_string).as_deref(),
            Some("binary not found")
        );
    }
}
