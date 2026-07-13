//! Pure domain vocabulary for ticket completion.
//!
//! This module describes completion state, inputs, and requested effects. It
//! deliberately performs no I/O and has no dependency on the scheduler,
//! Zellij, or a command runtime. Reducer and reconciliation behavior build on
//! these types in later modules of work.

use std::fmt;

use thiserror::Error;

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

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

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
    fn command_in_flight_always_contains_a_correlation_id() {
        let state = CompletionState::CommandInFlight {
            correlation: CorrelationId::new("command-1"),
        };

        let CompletionState::CommandInFlight { correlation } = state else {
            panic!("expected command-in-flight state");
        };
        assert_eq!(correlation.as_str(), "command-1");
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
        ];

        for rejection in cases {
            assert!(!rejection.to_string().is_empty());
            match rejection {
                CompletionRejection::AlreadyPending { .. }
                | CompletionRejection::StaleLease { .. }
                | CompletionRejection::DispositionBlocked { .. }
                | CompletionRejection::DependencyBlocked { .. }
                | CompletionRejection::LaunchFailed { .. } => {}
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
