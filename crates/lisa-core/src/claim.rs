//! Shared identity contract for an agent's assignment claim.
//!
//! The native command publishes [`AssignmentClaim`] as filesystem evidence.
//! Scheduler admission remains responsible for comparing that evidence with
//! its authoritative current lease and retained assignment reference.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::types::TicketId;

/// One agent assertion for an exact nonce-bearing ticket assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentClaim {
    /// Ticket named by the assignment.
    pub ticket_id: TicketId,
    /// Positive E-034 attempt generation named by the assignment.
    pub attempt_id: u64,
    /// Opaque identity of the exact immutable assignment file.
    pub nonce: u128,
}

/// Stable semantic reason an assignment claim was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ClaimRejection {
    /// The native process has no usable scheduler pane identity.
    #[error("LISA_PANE_ID does not identify a scheduler pane")]
    PaneUnavailable,
    /// The pane has no readable scheduler-published lease marker.
    #[error("the pane has no published attempt lease")]
    LeaseUnavailable,
    /// The pane lease marker is not a serialized E-034 lease.
    #[error("the pane attempt lease is invalid")]
    InvalidLease,
    /// The claim names a ticket other than the pane lease.
    #[error("the claim ticket does not match the pane lease")]
    WrongTicket,
    /// The claim names an older attempt than the pane lease.
    #[error("the claim attempt is older than the pane lease")]
    StaleAttempt,
    /// The claim names a non-current attempt that is not a predecessor.
    #[error("the claim attempt does not match the pane lease")]
    AttemptMismatch,
    /// The claim nonce has no exact published assignment file.
    #[error("the claim nonce does not identify a published assignment")]
    WrongNonce,
    /// The pane lease changed while the command was validating the claim.
    #[error("the pane attempt lease changed during claim validation")]
    LeaseChanged,
}

impl ClaimRejection {
    /// Machine-stable reason name rendered by command boundaries.
    pub const fn name(self) -> &'static str {
        match self {
            Self::PaneUnavailable => "pane-unavailable",
            Self::LeaseUnavailable => "lease-unavailable",
            Self::InvalidLease => "invalid-lease",
            Self::WrongTicket => "wrong-ticket",
            Self::StaleAttempt => "stale-attempt",
            Self::AttemptMismatch => "attempt-mismatch",
            Self::WrongNonce => "wrong-nonce",
            Self::LeaseChanged => "lease-changed",
        }
    }
}

/// Durable filename shared by the assignment publisher and claim validator.
pub fn assignment_file_name(attempt_id: u64, nonce: u128) -> String {
    format!("assignment-{attempt_id}-{nonce}.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_filename_preserves_full_numeric_identity() {
        assert_eq!(
            assignment_file_name(7, 8_675_309),
            "assignment-7-8675309.md"
        );
        assert_eq!(
            assignment_file_name(u64::MAX, u128::MAX),
            format!("assignment-{}-{}.md", u64::MAX, u128::MAX)
        );
    }

    #[test]
    fn rejection_names_are_stable_and_complete() {
        let cases = [
            (ClaimRejection::PaneUnavailable, "pane-unavailable"),
            (ClaimRejection::LeaseUnavailable, "lease-unavailable"),
            (ClaimRejection::InvalidLease, "invalid-lease"),
            (ClaimRejection::WrongTicket, "wrong-ticket"),
            (ClaimRejection::StaleAttempt, "stale-attempt"),
            (ClaimRejection::AttemptMismatch, "attempt-mismatch"),
            (ClaimRejection::WrongNonce, "wrong-nonce"),
            (ClaimRejection::LeaseChanged, "lease-changed"),
        ];

        for (reason, expected) in cases {
            assert_eq!(reason.name(), expected);
            assert!(!reason.to_string().is_empty());
        }
    }

    #[test]
    fn assignment_claim_json_round_trips_u128_and_hostile_ticket_text() {
        let claim = AssignmentClaim {
            ticket_id: "T-quoted-\"slash\\-$()`".to_string(),
            attempt_id: u64::MAX,
            nonce: u128::from(u64::MAX) + 42,
        };

        let encoded = serde_json::to_string(&claim).unwrap();
        assert_eq!(
            serde_json::from_str::<AssignmentClaim>(&encoded).unwrap(),
            claim
        );
    }
}
