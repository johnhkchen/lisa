//! Reviewer-side validation for a newly written Review disposition.

use std::env;
use std::path::{Component, Path, PathBuf};

use lisa_core::disposition::{check_review_disposition, ReviewDisposition};
use lisa_core::parking::validate_block_ask;

const DISPOSITION_FILE: &str = "review-disposition.json";

/// Validate the current attempt's disposition without publishing or acting on it.
pub fn run_check_disposition(project_root: &Path, ticket_id: &str) -> Result<String, String> {
    let disposition_path = disposition_path(project_root, ticket_id).map_err(fix)?;
    let disposition = check_review_disposition(&disposition_path).map_err(fix)?;

    if let ReviewDisposition::Block { ask, .. } = &disposition {
        validate_block_ask(ask).map_err(|error| fix(error.to_string()))?;
    }

    Ok(format!(
        "Review disposition is valid for {ticket_id}: {}",
        disposition_path.display()
    ))
}

fn disposition_path(project_root: &Path, ticket_id: &str) -> Result<PathBuf, String> {
    validate_ticket_id(ticket_id)?;

    match (
        env::var("LISA_TICKET_ID").ok(),
        env::var("LISA_ATTEMPT_ID").ok(),
    ) {
        (Some(active_ticket), Some(attempt_id)) => {
            if active_ticket != ticket_id {
                return Err(format!(
                    "run the check for active ticket {active_ticket:?}, not {ticket_id:?}"
                ));
            }
            let attempt_id: u64 = attempt_id.parse().map_err(|_| {
                "set LISA_ATTEMPT_ID to the current positive attempt number".to_string()
            })?;
            if attempt_id == 0 {
                return Err(
                    "set LISA_ATTEMPT_ID to the current positive attempt number".to_string()
                );
            }
            Ok(project_root
                .join(".lisa/attempts")
                .join(ticket_id)
                .join(attempt_id.to_string())
                .join("work")
                .join(DISPOSITION_FILE))
        }
        (None, None) => Ok(project_root
            .join("docs/active/work")
            .join(ticket_id)
            .join(DISPOSITION_FILE)),
        _ => Err(
            "set both LISA_TICKET_ID and LISA_ATTEMPT_ID for the current attempt, or unset both"
                .to_string(),
        ),
    }
}

fn validate_ticket_id(ticket_id: &str) -> Result<(), String> {
    let mut components = Path::new(ticket_id).components();
    if ticket_id.trim().is_empty()
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        return Err("use one ticket id without path separators, such as T-049-07-02".to_string());
    }
    Ok(())
}

fn fix(message: String) -> String {
    format!("Fix {DISPOSITION_FILE}: {message}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_id_must_be_one_safe_component() {
        for valid in ["T-001", "T-049-07-02"] {
            assert_eq!(validate_ticket_id(valid), Ok(()));
        }
        for invalid in ["", ".", "../T-001", "T-001/review.md", "/T-001"] {
            assert!(
                validate_ticket_id(invalid)
                    .unwrap_err()
                    .contains("ticket id"),
                "accepted or misdiagnosed {invalid:?}"
            );
        }
    }
}
