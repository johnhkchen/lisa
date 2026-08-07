//! Verify an optional parked-remedy check, then restore ordinary scheduling.

use std::path::Path;
use std::time::{Duration, SystemTime};

use lisa_core::completion_journal::MAX_ACTION_REQUIRED_GENERATIONS;
use lisa_core::parking::collect_parked_remedies;
use lisa_core::provenance::{
    append_check_override_record, append_world_recheck_record, latest_world_rechecks,
    system_time_to_epoch, CheckOverrideOutcome, CheckOverrideRecord, CheckOverrideType,
    WorldRecheckOutcome, WorldRecheckRecord, WorldRecheckType, SCHEMA_VERSION,
};
use lisa_core::ticket;
use lisa_core::types::TicketStatus;

use crate::check_run::{
    budget_for, format_budget, run_check, sanitize_observation, CheckResult, CheckRun,
};
use crate::completion_seal;
use crate::config;
use lisa_core::disposition::RemedyOwner;

const PROVENANCE_PATH: &str = ".lisa/provenance.jsonl";
/// Who a forced unblock is attributed to. `lisa unblock` is an operator command.
const OPERATOR_ACTOR: &str = "operator";

/// The lead sentence for a check that ran and reported no.
///
/// Deliberately Lisa's own words. The check's words appear only under the
/// attribution label below, because a sentence written by the project's script
/// is evidence, not Lisa's verdict.
const DECLINE_FAILED: &str = "That didn't work yet — the check ran and did not pass.";
/// The lead sentence for a check that never got to look.
///
/// It must not read as a judgement on the operator's work, and it always names
/// the way through — the override is printed under every decline.
const DECLINE_INCONCLUSIVE: &str =
    "Lisa can't tell yet — the check stopped before it could look, so this isn't a judgement on \
     your work.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnblockOutcome {
    Reopened(String),
    Declined(String),
}

/// Verify one parked ticket and restore `status: open` only when safe.
///
/// `override_check` covers exactly one gate: a check that declined. An operator
/// who has performed the ask and verified it themselves is never held by a check
/// they cannot satisfy — and the override is recorded, so a forced unblock stays
/// distinguishable afterwards from a check that passed. Every other decline
/// (unknown ticket, a ticket that is not waiting, a missing remedy, a completion
/// Lisa stopped recording) is untouched by it, because none of those is a gate
/// having done the ask would clear.
pub fn run_unblock(
    root: &Path,
    ticket_id: &str,
    override_check: bool,
) -> Result<UnblockOutcome, String> {
    let validation = config::load_config(root)?;
    let resolved = config::resolve_config(&validation.config, None, None);
    let ticket_dir = root.join(&resolved.ticket_dir);
    let work_dir = root.join(&resolved.work_dir);
    let tickets = ticket::scan_tickets(&ticket_dir)
        .map_err(|error| format!("Could not read the ticket board: {error}"))?;
    let Some(ticket) = tickets.iter().find(|ticket| ticket.id == ticket_id) else {
        return Ok(UnblockOutcome::Declined(format!(
            "I couldn't find {ticket_id}."
        )));
    };
    if ticket.status != TicketStatus::Blocked {
        return Ok(UnblockOutcome::Declined(format!(
            "{ticket_id} isn't waiting."
        )));
    }
    if let Some(decline) = recording_failure_decline(root, ticket_id) {
        return Ok(UnblockOutcome::Declined(decline));
    }

    let mut remedies = collect_parked_remedies(
        std::iter::once(ticket),
        &work_dir,
        &root.join(PROVENANCE_PATH),
    );
    let Some(remedy) = remedies.pop() else {
        return Ok(UnblockOutcome::Declined(format!(
            "I couldn't find what {ticket_id} is waiting for."
        )));
    };

    let mut overrode = false;
    if let Some(check) = remedy.check {
        let run = run_check(root, &check, budget_for(remedy.check_timeout_secs))?;
        if run.result != CheckResult::Passed {
            if !override_check {
                return Ok(UnblockOutcome::Declined(decline_report(ticket_id, &run)));
            }
            // The receipt lands before the flip. An override that left no trace
            // would be indistinguishable afterwards from a check that passed,
            // which is the whole point of recording it; the other order can
            // reach that state, this one cannot.
            record_check_override(root, &resolved, ticket_id, &run)?;
            overrode = true;
        }
    }

    ticket::update_ticket_status(&ticket.file_path, TicketStatus::Open)
        .map_err(|error| format!("Could not let {ticket_id} run again: {error}"))?;
    Ok(UnblockOutcome::Reopened(if overrode {
        format!("{ticket_id} can run again — you overrode its check.")
    } else {
        format!("{ticket_id} can run again.")
    }))
}

/// File the receipt for a check an operator decided over.
fn record_check_override(
    root: &Path,
    resolved: &config::ResolvedConfig,
    ticket_id: &str,
    run: &CheckRun,
) -> Result<(), String> {
    let mut observed = run.stderr.clone();
    observed.extend(run.stdout.iter().cloned());
    let record = CheckOverrideRecord {
        schema_version: SCHEMA_VERSION,
        seal: completion_seal::resolve_for_inspection(root, resolved.completion_mode),
        record_type: CheckOverrideType::CheckOverride,
        ticket_id: ticket_id.to_string(),
        actor: OPERATOR_ACTOR.to_string(),
        check: run.check.clone(),
        directory: run.directory.display().to_string(),
        result: override_outcome(run.result),
        exit_code: run.exit_code,
        observed,
        occurred_at: system_time_to_epoch(SystemTime::now()),
    };
    append_check_override_record(&root.join(PROVENANCE_PATH), &record)
        .map_err(|error| format!("Could not record the override: {error}"))
}

/// Map a decline onto its ledger outcome.
///
/// [`CheckOverrideOutcome::ChangedFiles`] has no source any more — checks run in
/// the project now, so Lisa no longer judges whether one wrote — but the variant
/// stays on the wire, because ledgers written before that change contain it and
/// must keep parsing.
fn override_outcome(result: CheckResult) -> CheckOverrideOutcome {
    match result {
        CheckResult::Passed => unreachable!("a passing check is never overridden"),
        CheckResult::Failed => CheckOverrideOutcome::Failed,
        CheckResult::Inconclusive => CheckOverrideOutcome::Inconclusive,
        CheckResult::TimedOut => CheckOverrideOutcome::TimedOut,
    }
}

/// Verify every observable world-owned parked remedy and reopen only passes.
///
/// This is deliberately stricter than [`run_unblock`]: automation cannot act
/// for an operator or agent, and a world-owned remedy without a check has no
/// positive evidence that external reality changed. A non-pass is still never
/// acted on — but it is no longer discarded either. It is sampled into the
/// ledger by [`record_world_non_pass`], so a remedy whose check can never clear
/// stops being a silent park and becomes something `lisa status` can name.
pub(crate) fn run_world_rechecks(root: &Path) -> Result<Vec<String>, String> {
    let validation = config::load_config(root)?;
    let resolved = config::resolve_config(&validation.config, None, None);
    let ticket_dir = root.join(&resolved.ticket_dir);
    let work_dir = root.join(&resolved.work_dir);
    let tickets = ticket::scan_tickets(&ticket_dir)
        .map_err(|error| format!("Could not read the ticket board: {error}"))?;
    let ledger = root.join(PROVENANCE_PATH);
    let remedies = collect_parked_remedies(tickets.iter(), &work_dir, &ledger);
    // Read once: every remedy's running count comes from the same walk that
    // `collect_parked_remedies` just made over the same file.
    let observations = latest_world_rechecks(&ledger);
    let mut reopened = Vec::new();

    for remedy in remedies {
        if remedy.remedy_owner != RemedyOwner::World {
            continue;
        }
        let Some(check) = remedy.check else {
            continue;
        };
        let Some(ticket) = tickets.iter().find(|ticket| ticket.id == remedy.ticket_id) else {
            continue;
        };

        let run = run_check(root, &check, budget_for(remedy.check_timeout_secs))
            .map_err(|error| format!("Could not recheck {}: {error}", remedy.ticket_id))?;
        match run.result {
            CheckResult::Passed => {
                ticket::update_ticket_status(&ticket.file_path, TicketStatus::Open).map_err(
                    |error| format!("Could not let {} run again: {error}", remedy.ticket_id),
                )?;
                reopened.push(remedy.ticket_id);
            }
            // Automation never acts on a non-pass, and it gains no override:
            // an operator can say "I checked this myself", a timer cannot. What
            // it does now is say what it saw.
            CheckResult::Failed | CheckResult::Inconclusive | CheckResult::TimedOut => {
                let previous = observations
                    .get(&remedy.ticket_id)
                    .filter(|observation| observation.check == run.check)
                    .map_or(0, |observation| observation.non_pass_count);
                record_world_non_pass(root, &resolved, &remedy.ticket_id, &run, previous + 1)?;
            }
        }
    }

    Ok(reopened)
}

/// Sample one non-pass into the ledger, on a doubling schedule.
///
/// The scheduler rechecks on its ordinary poll cadence, so a row per non-pass
/// would be hundreds an hour for one parked ticket. Writing the 1st, 2nd, 4th,
/// 8th … keeps the ledger bounded — logarithmic in poll count — while the rows
/// themselves show the repetition, and each carries the exact running total so a
/// reader never has to count rows to know how long this has been failing.
fn record_world_non_pass(
    root: &Path,
    resolved: &config::ResolvedConfig,
    ticket_id: &str,
    run: &CheckRun,
    non_pass_count: u64,
) -> Result<(), String> {
    if !non_pass_count.is_power_of_two() {
        return Ok(());
    }
    let mut observed = run.stderr.clone();
    observed.extend(run.stdout.iter().cloned());
    let record = WorldRecheckRecord {
        schema_version: SCHEMA_VERSION,
        seal: completion_seal::resolve_for_inspection(root, resolved.completion_mode),
        record_type: WorldRecheckType::WorldRecheck,
        ticket_id: ticket_id.to_string(),
        check: run.check.clone(),
        directory: run.directory.display().to_string(),
        result: world_outcome(run.result),
        exit_code: run.exit_code,
        observed,
        non_pass_count,
        occurred_at: system_time_to_epoch(SystemTime::now()),
    };
    append_world_recheck_record(&root.join(PROVENANCE_PATH), &record)
        .map_err(|error| format!("Could not record the recheck of {ticket_id}: {error}"))
}

/// Map a non-pass onto its ledger outcome.
fn world_outcome(result: CheckResult) -> WorldRecheckOutcome {
    match result {
        CheckResult::Passed => unreachable!("a passing recheck reopens instead of recording"),
        CheckResult::Failed => WorldRecheckOutcome::Failed,
        CheckResult::Inconclusive => WorldRecheckOutcome::Inconclusive,
        CheckResult::TimedOut => WorldRecheckOutcome::TimedOut,
    }
}

/// The one case unblock now steps aside for.
///
/// Unblock's meaning is unchanged: verify what changed, and let a waiting
/// ticket run again. But a completion Lisa has stopped trying to record is not
/// waiting on the review, and reopening it would hand it a session to fail in
/// again. That is a case unblock cannot fix and `already-done` can, so it says
/// so rather than reporting success and changing nothing that matters.
///
/// A ticket with no journal record, or one still under the bound, is untouched
/// by this — which is every ordinary parked ticket.
fn recording_failure_decline(root: &Path, ticket_id: &str) -> Option<String> {
    let journal = root.join(lisa_core::completion_journal::COMPLETION_JOURNAL_RELATIVE_PATH);
    // A journal Lisa cannot read is not evidence of anything; unblock keeps its
    // ordinary behavior rather than refusing on a file it failed to parse.
    let aggregates = lisa_core::completion_journal::load(&journal).ok()?;
    let aggregate = aggregates.get(ticket_id)?;
    (aggregate.action_required_generations() >= MAX_ACTION_REQUIRED_GENERATIONS).then(|| {
        format!(
            "{ticket_id} is waiting because Lisa could not record its finished work, not because \
             of the review. If that work is already saved in history, run: `lisa already-done \
             {ticket_id}`."
        )
    })
}

/// The lead sentence for a check that outlived its budget.
///
/// Formatted from the budget the run was actually held to, not from a constant:
/// a check that declared twenty-five minutes and was stopped must say twenty-five
/// minutes, or the operator goes looking for a five-second problem that is not
/// there.
fn decline_timed_out(budget: Duration) -> String {
    format!(
        "That didn't work yet — it took longer than {}.",
        format_budget(budget)
    )
}

fn decline_header(run: &CheckRun) -> String {
    match run.result {
        CheckResult::Passed => unreachable!("passing checks do not decline"),
        CheckResult::Failed => DECLINE_FAILED.to_string(),
        CheckResult::Inconclusive => DECLINE_INCONCLUSIVE.to_string(),
        CheckResult::TimedOut => decline_timed_out(run.budget),
    }
}

/// What the check exited with, or plainly why there is no code to report.
fn exit_code_line(run: &CheckRun) -> String {
    match (run.result, run.exit_code) {
        (CheckResult::TimedOut, _) => {
            format!("none — Lisa stopped it after {}", format_budget(run.budget))
        }
        (_, Some(code)) => code.to_string(),
        (_, None) => "none — the check was stopped".to_string(),
    }
}

/// The whole decline: Lisa's finding, then the check's work, then a way through.
///
/// Everything the check said is shown under its own label. Nothing it said ever
/// reaches the header line — the field failure this ticket exists for was a
/// project script's sentence relayed as Lisa's verdict.
fn decline_report(ticket_id: &str, run: &CheckRun) -> String {
    let mut report = decline_header(run);
    report.push_str("\n\n");
    // A recorded check may span lines; folding them to spaces before sanitizing
    // keeps the command readable instead of running its words together.
    report.push_str(&format!(
        "  what ran:  {}\n",
        sanitize_observation(&run.check.replace(['\n', '\r'], " "))
    ));
    report.push_str(&format!(
        "  ran in:    {}\n",
        sanitize_observation(&run.directory.display().to_string())
    ));
    report.push_str(&format!("  exit code: {}\n", exit_code_line(run)));

    let mut printed_anything = false;
    for (label, lines, dropped) in [
        ("stderr", &run.stderr, run.stderr_dropped),
        ("stdout", &run.stdout, run.stdout_dropped),
    ] {
        if lines.is_empty() {
            continue;
        }
        printed_anything = true;
        report.push_str(&format!("\n  the check wrote to {label}:\n"));
        for line in lines {
            report.push_str(&format!("    {line}\n"));
        }
        if dropped > 0 {
            report.push_str(&format!("    … ({dropped} more lines)\n"));
        }
    }
    if !printed_anything {
        report.push_str("\n  the check printed nothing.\n");
    }

    report.push_str(&format!(
        "\nIf you have done this and checked it yourself, run:\n  lisa unblock {ticket_id} --override-check"
    ));
    report
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    /// The field sentence from the 0.4.4 `tabular-recipes` run, verbatim.
    const FIELD_LINE: &str = "No build at dist/. Run: npm run build";

    fn run(root: &Path, check: &str) -> CheckRun {
        run_check(root, check, budget_for(None)).unwrap()
    }

    /// The whole decline, on a check that said nothing at all.
    #[test]
    fn a_silent_failing_check_still_reports_command_directory_and_code() {
        let root = tempfile::tempdir().unwrap();

        let silent = run(root.path(), "exit 1");
        let report = decline_report("T-1", &silent);

        assert_eq!(silent.result, CheckResult::Failed);
        assert!(report.starts_with(DECLINE_FAILED), "{report}");
        assert!(report.contains("  what ran:  exit 1\n"), "{report}");
        assert!(report.contains("  exit code: 1\n"), "{report}");
        assert!(report.contains("the check printed nothing."), "{report}");
    }

    /// Criterion 2: expiry names the budget it actually waited for.
    ///
    /// Two budgets, two sentences. The five-second one is the default and stays
    /// byte for byte what it has always been; the declared one proves the
    /// sentence is built from this run rather than from a constant.
    #[test]
    fn timeout_expiry_names_the_budget_that_was_enforced() {
        let root = tempfile::tempdir().unwrap();
        let started = Instant::now();

        let default_budget = CheckRun {
            budget: budget_for(None),
            ..run_check(root.path(), "sleep 5 & wait", Duration::from_millis(60)).unwrap()
        };
        let declared_budget = CheckRun {
            budget: budget_for(Some(1500)),
            ..run_check(root.path(), "sleep 5 & wait", Duration::from_millis(60)).unwrap()
        };

        assert!(
            started.elapsed() < Duration::from_secs(2),
            "the budget bounds the wait"
        );
        assert_eq!(default_budget.result, CheckResult::TimedOut);
        assert_eq!(
            decline_header(&default_budget),
            "That didn't work yet — it took longer than 5 seconds."
        );
        assert_eq!(
            decline_header(&declared_budget),
            "That didn't work yet — it took longer than 25 minutes."
        );
        assert!(decline_report("T-1", &default_budget)
            .contains("exit code: none — Lisa stopped it after 5 seconds"));
        assert!(decline_report("T-1", &declared_budget)
            .contains("exit code: none — Lisa stopped it after 25 minutes"));
    }

    /// Every non-passing result has its own sentence, none of them is a
    /// judgement the check did not make, and each names the way through.
    #[test]
    fn every_decline_header_is_distinct_and_names_the_way_through() {
        let root = tempfile::tempdir().unwrap();
        let runs = [
            run(root.path(), "exit 1"),
            run(root.path(), "exit 2"),
            run_check(root.path(), "sleep 5 & wait", Duration::from_millis(60)).unwrap(),
        ];
        let headers: Vec<String> = runs.iter().map(decline_header).collect();

        let mut unique = headers.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), headers.len(), "{headers:?}");

        for check in ["exit 1", "exit 2", "./definitely-not-here"] {
            let report = decline_report("T-010-03", &run(root.path(), check));
            assert!(
                report.ends_with("lisa unblock T-010-03 --override-check"),
                "{report}"
            );
        }
    }

    /// The field failure, at the unit level: the script's sentence is shown as
    /// the script's, never as Lisa's finding.
    #[test]
    fn the_field_line_is_reported_not_asserted_as_lisas_verdict() {
        let root = tempfile::tempdir().unwrap();

        let inconclusive = run(
            root.path(),
            &format!("printf '{FIELD_LINE}\\n' >&2; exit 2"),
        );
        let report = decline_report("T-010-03", &inconclusive);
        let header = report.lines().next().unwrap();

        assert_eq!(inconclusive.result, CheckResult::Inconclusive);
        assert_eq!(header, DECLINE_INCONCLUSIVE);
        assert!(!header.contains("No build at dist/"), "{header}");
        assert!(!report.contains(&format!("That didn't work yet — {FIELD_LINE}")));
        assert!(report.contains("  the check wrote to stderr:\n    No build at dist/"));
        assert!(report.contains("exit code: 2"), "{report}");
        assert!(report.contains("  what ran:  printf"), "{report}");
        assert!(
            report.contains(&format!(
                "  ran in:    {}",
                inconclusive.directory.display()
            )),
            "{report}"
        );
    }

    /// Only the sampling schedule decides which non-passes reach the ledger.
    #[test]
    fn world_non_passes_are_sampled_on_a_doubling_schedule() {
        let recorded: Vec<u64> = (1..=40u64)
            .filter(|count| count.is_power_of_two())
            .collect();
        assert_eq!(recorded, vec![1, 2, 4, 8, 16, 32]);
    }
}
