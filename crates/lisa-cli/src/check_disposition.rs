//! Reviewer-side validation for a newly written Review disposition.

use std::env;
use std::path::{Component, Path, PathBuf};

use lisa_core::disposition::{check_review_disposition, ReviewDisposition};
use lisa_core::parking::validate_block_ask;

use crate::check_run::{
    budget_for, format_budget, run_check, sanitize_observation, CheckResult, CheckRun,
};

const DISPOSITION_FILE: &str = "review-disposition.json";

/// Validate the current attempt's disposition without publishing or acting on it.
///
/// A block's `check` is not merely shape-checked here: it is *run*, under the
/// same contract `lisa unblock` will run it under. That is the whole point of
/// this command's timing. A check that cannot pass — one that could not look, or
/// one that outlives the budget it declared — costs one review turn when it is
/// caught here, and costs the operator an afternoon and a hand-edited file when
/// it is not.
pub fn run_check_disposition(project_root: &Path, ticket_id: &str) -> Result<String, String> {
    let disposition_path = disposition_path(project_root, ticket_id).map_err(fix)?;
    let disposition = check_review_disposition(&disposition_path).map_err(fix)?;

    let mut trailer = String::new();
    if let ReviewDisposition::Block {
        ask,
        check,
        check_timeout_secs,
        ..
    } = &disposition
    {
        validate_block_ask(ask).map_err(|error| fix(error.to_string()))?;
        if let Some(check) = check {
            let run = run_check(project_root, check, budget_for(*check_timeout_secs))?;
            match run.result {
                // A remedy that has not been performed yet is exactly why this
                // ticket is blocking, so a check that ran and said no is the
                // ordinary, healthy state at record time.
                CheckResult::Passed | CheckResult::Failed => {
                    trailer = format!("\n{}", ran_line(&run));
                }
                CheckResult::Inconclusive | CheckResult::TimedOut => {
                    return Err(fix(unrunnable_check_message(&run)))
                }
            }
        }
    }

    Ok(format!(
        "Review disposition is valid for {ticket_id}: {}{trailer}",
        disposition_path.display()
    ))
}

/// The one-line receipt that the check was actually exercised.
fn ran_line(run: &CheckRun) -> String {
    let verdict = match run.result {
        CheckResult::Passed => "passed",
        _ => "did not pass — the ordinary state of a remedy nobody has done yet",
    };
    format!(
        "The check ran in {} and {verdict}.",
        run.directory.display()
    )
}

/// What a reviewer needs to fix a check that cannot pass, while they can.
///
/// Deliberately not the operator's decline copy: the reader here is the person
/// who wrote the check, so it names the class of failure, shows the run, and
/// says what to change — rather than offering an override, which is not theirs
/// to take.
fn unrunnable_check_message(run: &CheckRun) -> String {
    let (lead, remedy) = match run.result {
        CheckResult::TimedOut => (
            format!(
                "the check outlived its {} budget, so it can never pass",
                format_budget(run.budget)
            ),
            "make the check faster, or declare a larger check_timeout_secs (up to 1800)",
        ),
        _ => (
            "the check stopped before it could look, so it can never pass".to_string(),
            "check the command exists and runs from the project root, and that it reports a verdict rather than trouble",
        ),
    };
    let mut message = format!("{lead}.\n");
    message.push_str(&format!(
        "  what ran:  {}\n",
        sanitize_observation(&run.check.replace(['\n', '\r'], " "))
    ));
    message.push_str(&format!(
        "  ran in:    {}\n",
        sanitize_observation(&run.directory.display().to_string())
    ));
    message.push_str(&format!(
        "  exit code: {}\n",
        match run.exit_code {
            Some(code) => code.to_string(),
            None => format!("none — Lisa stopped it after {}", format_budget(run.budget)),
        }
    ));
    for (label, lines) in [("stderr", &run.stderr), ("stdout", &run.stdout)] {
        for line in lines.iter().take(3) {
            message.push_str(&format!("  {label}:    {line}\n"));
        }
    }
    message.push_str(&format!("Fix: {remedy}."));
    message
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
