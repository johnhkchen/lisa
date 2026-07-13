//! Native command boundary for publishing an exact assignment claim.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lisa_core::claim::{assignment_file_name, AssignmentClaim, ClaimRejection};
use lisa_core::types::AttemptLease;

pub(crate) struct ClaimRequest {
    pub(crate) project_root: PathBuf,
    pub(crate) pane_id: Option<String>,
    pub(crate) claim: AssignmentClaim,
}

pub(crate) struct ClaimReceipt {
    pub(crate) claim: AssignmentClaim,
}

#[derive(Debug)]
pub(crate) enum ClaimError {
    Rejected(ClaimRejection),
    Operational(String),
}

impl ClaimError {
    fn rejected(reason: ClaimRejection) -> Self {
        Self::Rejected(reason)
    }
}

impl fmt::Display for ClaimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(reason) => {
                write!(f, "claim rejected [{}]: {reason}", reason.name())
            }
            Self::Operational(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ClaimError {}

/// Validate one exact assignment against durable pane identity and publish it.
///
/// The scheduler consumer still performs final admission against its in-memory
/// current lease and retained assignment reference. This producer-side check
/// rejects identities that cannot match the pane's published E-034 lease or an
/// exact durable assignment.
pub(crate) fn claim_assignment(request: ClaimRequest) -> Result<ClaimReceipt, ClaimError> {
    let pane_id = request
        .pane_id
        .as_deref()
        .and_then(|pane_id| pane_id.parse::<u32>().ok())
        .ok_or_else(|| ClaimError::rejected(ClaimRejection::PaneUnavailable))?;
    let signal_dir = request.project_root.join(".lisa/signals");
    let lease_path = signal_dir.join(format!("pane-{pane_id}.lease"));
    let observed_lease = read_initial_lease(&lease_path)?;

    validate_claim_lease(&request.claim, &observed_lease)?;

    let assignment_path = request
        .project_root
        .join(".lisa/attempts")
        .join(&request.claim.ticket_id)
        .join(request.claim.attempt_id.to_string())
        .join("work")
        .join(assignment_file_name(
            request.claim.attempt_id,
            request.claim.nonce,
        ));
    if !assignment_path.is_file() {
        return Err(ClaimError::rejected(ClaimRejection::WrongNonce));
    }

    if reread_lease(&lease_path).as_ref() != Some(&observed_lease) {
        return Err(ClaimError::rejected(ClaimRejection::LeaseChanged));
    }

    publish_claim(&signal_dir, pane_id, &request.claim)?;
    Ok(ClaimReceipt {
        claim: request.claim,
    })
}

fn read_initial_lease(path: &Path) -> Result<AttemptLease, ClaimError> {
    if !path.is_file() {
        return Err(ClaimError::rejected(ClaimRejection::LeaseUnavailable));
    }
    let body = std::fs::read_to_string(path)
        .map_err(|_| ClaimError::rejected(ClaimRejection::LeaseUnavailable))?;
    serde_json::from_str(&body).map_err(|_| ClaimError::rejected(ClaimRejection::InvalidLease))
}

fn reread_lease(path: &Path) -> Option<AttemptLease> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|body| serde_json::from_str(&body).ok())
}

fn validate_claim_lease(
    claim: &AssignmentClaim,
    observed: &AttemptLease,
) -> Result<(), ClaimError> {
    if claim.ticket_id != observed.ticket_id {
        return Err(ClaimError::rejected(ClaimRejection::WrongTicket));
    }
    if claim.attempt_id < observed.attempt_id {
        return Err(ClaimError::rejected(ClaimRejection::StaleAttempt));
    }
    if claim.attempt_id != observed.attempt_id {
        return Err(ClaimError::rejected(ClaimRejection::AttemptMismatch));
    }
    Ok(())
}

fn publish_claim(
    signal_dir: &Path,
    pane_id: u32,
    claim: &AssignmentClaim,
) -> Result<PathBuf, ClaimError> {
    std::fs::create_dir_all(signal_dir).map_err(|error| {
        ClaimError::Operational(format!(
            "cannot create claim signal directory {}: {error}",
            signal_dir.display()
        ))
    })?;
    let body = serde_json::to_vec(claim).map_err(|error| {
        ClaimError::Operational(format!("cannot serialize assignment claim: {error}"))
    })?;
    let destination = signal_dir.join(format!("pane-{pane_id}.claim"));
    let temporary = signal_dir.join(format!(
        ".pane-{pane_id}.claim.tmp.{}-{}",
        std::process::id(),
        publication_nonce()
    ));

    std::fs::write(&temporary, body).map_err(|error| {
        ClaimError::Operational(format!(
            "cannot write assignment claim temporary {}: {error}",
            temporary.display()
        ))
    })?;
    if let Err(error) = std::fs::rename(&temporary, &destination) {
        let _ = std::fs::remove_file(&temporary);
        return Err(ClaimError::Operational(format!(
            "cannot publish assignment claim {}: {error}",
            destination.display()
        )));
    }

    Ok(destination)
}

fn publication_nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
