//! Factual narrative for the latest Lisa loop and the current ticket board.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use lisa_core::types::{Phase, Ticket};
use serde::{Deserialize, Serialize};

const BASELINE_SCHEMA_VERSION: u32 = 1;
const PROVENANCE_PATH: &str = ".lisa/provenance.jsonl";
const COMPLETION_JOURNAL_PATH: &str = ".lisa/completion-journal.jsonl";
const EVENT_PATH: &str = ".lisa/run-events.jsonl";
const BASELINE_PATH: &str = ".lisa/run-baseline.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RunBaseline {
    schema_version: u32,
    provenance_bytes: u64,
    event_bytes: u64,
    intervention_tracking: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct OutcomeCounts {
    failed: usize,
    timed_out: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct InterventionCounts {
    questions: usize,
    permissions: usize,
}

impl InterventionCounts {
    fn total(self) -> usize {
        self.questions + self.permissions
    }
}

/// Capture byte offsets immediately before a loop starts.
///
/// The latest-run summary reads only bytes appended after these offsets, so an
/// old failed attempt or interactive gate cannot be attributed to a new loop.
pub(crate) fn record_run_baseline(root: &Path) -> Result<(), String> {
    let lisa_dir = root.join(".lisa");
    fs::create_dir_all(&lisa_dir)
        .map_err(|error| format!("Failed to create {}: {error}", lisa_dir.display()))?;

    let event_path = root.join(EVENT_PATH);
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&event_path)
        .map_err(|error| format!("Failed to open {}: {error}", event_path.display()))?;

    let baseline = RunBaseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        provenance_bytes: file_len_or_zero(&root.join(PROVENANCE_PATH))?,
        event_bytes: file_len_or_zero(&event_path)?,
        intervention_tracking: intervention_tracking_installed(root),
    };
    let body = serde_json::to_vec(&baseline)
        .map_err(|error| format!("Failed to serialize run baseline: {error}"))?;
    let destination = root.join(BASELINE_PATH);
    let temporary = lisa_dir.join(format!(".run-baseline.json.tmp.{}", std::process::id()));
    fs::write(&temporary, body)
        .map_err(|error| format!("Failed to write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &destination).map_err(|error| {
        format!(
            "Failed to publish run baseline {}: {error}",
            destination.display()
        )
    })?;
    Ok(())
}

fn intervention_tracking_installed(root: &Path) -> bool {
    let settings = match fs::read(root.join(".claude/settings.local.json")) {
        Ok(settings) => settings,
        Err(_) => return false,
    };
    let value: serde_json::Value = match serde_json::from_slice(&settings) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let mut commands = Vec::new();
    collect_commands(&value, &mut commands);
    ["question", "permission"].into_iter().all(|kind| {
        commands.iter().any(|command| {
            command.contains(EVENT_PATH)
                && command.contains("manual-intervention")
                && command.contains(&format!(r#""kind":"{kind}""#))
        })
    })
}

fn collect_commands<'a>(value: &'a serde_json::Value, commands: &mut Vec<&'a str>) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(command) = object.get("command").and_then(serde_json::Value::as_str) {
                commands.push(command);
            }
            for child in object.values() {
                collect_commands(child, commands);
            }
        }
        serde_json::Value::Array(array) => {
            for child in array {
                collect_commands(child, commands);
            }
        }
        _ => {}
    }
}

fn file_len_or_zero(path: &Path) -> Result<u64, String> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(format!("Failed to inspect {}: {error}", path.display())),
    }
}

/// The latest run's facts, assembled once and then either printed or
/// serialised. Two renderers over one collection is what keeps the JSON body
/// from disagreeing with the prose beside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RunSummary {
    pub(crate) tickets_total: usize,
    pub(crate) tickets_completed: usize,
    pub(crate) tickets_remaining: usize,
    /// `None` when the latest run's outcomes could not be read at all, which
    /// is a different fact from "no failures".
    pub(crate) failed: Option<usize>,
    pub(crate) timed_out: Option<usize>,
    /// `None` when the installed hooks cannot track approval gates.
    pub(crate) manual_interventions: Option<ManualInterventions>,
    pub(crate) evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct ManualInterventions {
    pub(crate) questions: usize,
    pub(crate) permissions: usize,
}

/// Assemble the latest run's facts. `None` when there is no board to summarise.
pub(crate) fn collect_run_summary(
    root: &Path,
    tickets: &[Ticket],
    work_dir: &Path,
) -> Option<RunSummary> {
    if tickets.is_empty() {
        return None;
    }

    let total = tickets.len();
    let completed = tickets
        .iter()
        .filter(|ticket| ticket.phase == Phase::Done)
        .count();
    let ticket_ids: HashSet<&str> = tickets.iter().map(|ticket| ticket.id.as_str()).collect();
    let baseline = load_baseline(root);
    let outcomes = baseline
        .as_ref()
        .and_then(|baseline| load_outcomes(root, baseline, &ticket_ids));
    let interventions = baseline
        .as_ref()
        .and_then(|baseline| load_interventions(root, baseline));

    Some(RunSummary {
        tickets_total: total,
        tickets_completed: completed,
        tickets_remaining: total.saturating_sub(completed),
        failed: outcomes.map(|counts| counts.failed),
        timed_out: outcomes.map(|counts| counts.timed_out),
        manual_interventions: interventions.map(|counts| ManualInterventions {
            questions: counts.questions,
            permissions: counts.permissions,
        }),
        evidence: existing_evidence_paths(root, work_dir, tickets),
    })
}

/// Print the same summary used by `lisa status` and post-loop reporting.
pub(crate) fn print_run_summary(
    root: &Path,
    tickets: &[Ticket],
    work_dir: &Path,
) -> Result<(), String> {
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    write_run_summary(root, tickets, work_dir, &mut output)
        .map_err(|error| format!("Failed to print run summary: {error}"))
}

fn write_run_summary(
    root: &Path,
    tickets: &[Ticket],
    work_dir: &Path,
    output: &mut impl Write,
) -> std::io::Result<()> {
    let Some(summary) = collect_run_summary(root, tickets, work_dir) else {
        return Ok(());
    };
    render_run_summary(&summary, output)
}

pub(crate) fn render_run_summary(
    summary: &RunSummary,
    output: &mut impl Write,
) -> std::io::Result<()> {
    let RunSummary {
        tickets_total: total,
        tickets_completed: completed,
        tickets_remaining: remaining,
        failed,
        timed_out,
        manual_interventions: interventions,
        evidence,
    } = summary;
    let (total, completed, remaining) = (*total, *completed, *remaining);
    let outcomes = failed
        .zip(*timed_out)
        .map(|(failed, timed_out)| OutcomeCounts { failed, timed_out });
    let interventions = interventions.map(|counts| InterventionCounts {
        questions: counts.questions,
        permissions: counts.permissions,
    });
    let clean_outcomes = outcomes.is_some_and(|counts| counts == OutcomeCounts::default());
    let zero_interventions =
        interventions.is_some_and(|counts| counts == InterventionCounts::default());

    writeln!(output)?;
    writeln!(output, "Run summary:")?;
    if completed == total && clean_outcomes && zero_interventions {
        writeln!(
            output,
            "Lisa completed the board without asking you to approve steps by hand."
        )?;
    }
    if remaining == 0 {
        writeln!(output, "Completed: {completed} of {total} tickets.")?;
    } else {
        let verb = if remaining == 1 { "remains" } else { "remain" };
        writeln!(
            output,
            "Completed: {completed} of {total} tickets; {remaining} {verb}."
        )?;
    }

    if let Some(counts) = outcomes {
        if counts.failed > 0 || counts.timed_out > 0 {
            writeln!(
                output,
                "Run issues: {} failed, {} timed out.",
                counts.failed, counts.timed_out
            )?;
        }
    }

    match interventions {
        Some(counts) if counts.total() == 0 => {
            writeln!(output, "Manual approvals requested: 0.")?;
        }
        Some(counts) => {
            writeln!(
                output,
                "Manual intervention gates: {} ({} questions, {} permission requests).",
                counts.total(),
                counts.questions,
                counts.permissions
            )?;
        }
        None => {
            writeln!(
                output,
                "Manual approval tracking is unavailable for the latest run."
            )?;
        }
    }

    if !evidence.is_empty() {
        writeln!(output, "Evidence: {}", evidence.join("; "))?;
    }
    Ok(())
}

fn load_baseline(root: &Path) -> Option<RunBaseline> {
    let body = fs::read(root.join(BASELINE_PATH)).ok()?;
    let baseline: RunBaseline = serde_json::from_slice(&body).ok()?;
    (baseline.schema_version == BASELINE_SCHEMA_VERSION).then_some(baseline)
}

fn load_outcomes(
    root: &Path,
    baseline: &RunBaseline,
    ticket_ids: &HashSet<&str>,
) -> Option<OutcomeCounts> {
    let segment = read_segment(&root.join(PROVENANCE_PATH), baseline.provenance_bytes)?;
    parse_json_lines(&segment, |value, counts: &mut OutcomeCounts| {
        let ticket_id = value.get("ticket_id").and_then(serde_json::Value::as_str);
        if !ticket_id.is_some_and(|ticket_id| ticket_ids.contains(ticket_id)) {
            return;
        }
        match value.get("outcome").and_then(serde_json::Value::as_str) {
            Some("failed") => counts.failed += 1,
            Some("timed-out") => counts.timed_out += 1,
            _ => {}
        }
    })
}

fn load_interventions(root: &Path, baseline: &RunBaseline) -> Option<InterventionCounts> {
    if !baseline.intervention_tracking {
        return None;
    }
    let segment = read_segment(&root.join(EVENT_PATH), baseline.event_bytes)?;
    parse_json_lines(&segment, |value, counts: &mut InterventionCounts| {
        if value.get("event").and_then(serde_json::Value::as_str) != Some("manual-intervention") {
            return;
        }
        match value.get("kind").and_then(serde_json::Value::as_str) {
            Some("question") => counts.questions += 1,
            Some("permission") => counts.permissions += 1,
            _ => {}
        }
    })
}

fn read_segment(path: &Path, offset: u64) -> Option<Vec<u8>> {
    let bytes = fs::read(path).ok()?;
    let offset = usize::try_from(offset).ok()?;
    (offset <= bytes.len()).then(|| bytes[offset..].to_vec())
}

fn parse_json_lines<T: Default>(
    bytes: &[u8],
    mut consume: impl FnMut(&serde_json::Value, &mut T),
) -> Option<T> {
    if !bytes.is_empty() && !bytes.ends_with(b"\n") {
        return None;
    }
    let mut result = T::default();
    for line in bytes.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_slice(line).ok()?;
        consume(&value, &mut result);
    }
    Some(result)
}

fn existing_evidence_paths(root: &Path, work_dir: &Path, tickets: &[Ticket]) -> Vec<String> {
    let mut evidence = Vec::new();
    for relative in [PROVENANCE_PATH, COMPLETION_JOURNAL_PATH] {
        if root.join(relative).exists() {
            evidence.push(relative.to_string());
        }
    }

    let work_path = if work_dir.is_absolute() {
        work_dir.to_path_buf()
    } else {
        root.join(work_dir)
    };
    let has_current_ticket_docs = tickets.iter().any(|ticket| {
        fs::read_dir(work_path.join(&ticket.id))
            .ok()
            .is_some_and(|entries| entries.flatten().any(|entry| entry.path().is_file()))
    });
    if has_current_ticket_docs {
        evidence.push(display_path(root, &work_path));
    }
    evidence
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .trim_end_matches('/')
        .to_string()
        + "/"
}

#[cfg(test)]
mod tests {
    use super::*;
    use lisa_core::types::TicketStatus;

    fn ticket(id: &str, phase: Phase) -> Ticket {
        let mut ticket = Ticket::new(id, format!("ticket-{id}"));
        ticket.phase = phase;
        ticket.status = if phase == Phase::Done {
            TicketStatus::Done
        } else {
            TicketStatus::Open
        };
        ticket
    }

    fn write_baseline(root: &Path, provenance_prefix: &str, event_prefix: &str) {
        fs::create_dir_all(root.join(".lisa")).unwrap();
        fs::write(root.join(PROVENANCE_PATH), provenance_prefix).unwrap();
        fs::write(root.join(EVENT_PATH), event_prefix).unwrap();
        let baseline = RunBaseline {
            schema_version: BASELINE_SCHEMA_VERSION,
            provenance_bytes: provenance_prefix.len() as u64,
            event_bytes: event_prefix.len() as u64,
            intervention_tracking: true,
        };
        fs::write(
            root.join(BASELINE_PATH),
            serde_json::to_vec(&baseline).unwrap(),
        )
        .unwrap();
    }

    fn render(root: &Path, tickets: &[Ticket]) -> String {
        let mut output = Vec::new();
        write_run_summary(root, tickets, Path::new("docs/active/work"), &mut output).unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn clean_fixture_reports_the_verified_unattended_win() {
        let dir = tempfile::tempdir().unwrap();
        write_baseline(dir.path(), "", "");
        fs::write(
            dir.path().join(PROVENANCE_PATH),
            concat!(
                "{\"ticket_id\":\"T-1\",\"outcome\":\"done\"}\n",
                "{\"ticket_id\":\"T-2\",\"outcome\":\"done\"}\n"
            ),
        )
        .unwrap();
        fs::write(dir.path().join(COMPLETION_JOURNAL_PATH), "{}\n").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work/T-1")).unwrap();
        fs::write(
            dir.path().join("docs/active/work/T-1/review.md"),
            "review evidence\n",
        )
        .unwrap();

        let output = render(
            dir.path(),
            &[ticket("T-1", Phase::Done), ticket("T-2", Phase::Done)],
        );

        assert!(output.contains("completed the board without asking you"));
        assert!(output.contains("Completed: 2 of 2 tickets."));
        assert!(output.contains("Manual approvals requested: 0."));
        assert!(output.contains(PROVENANCE_PATH));
        assert!(output.contains(COMPLETION_JOURNAL_PATH));
        assert!(output.contains("docs/active/work/"));
    }

    #[test]
    fn failed_fixture_reports_failures_and_partial_completion() {
        let dir = tempfile::tempdir().unwrap();
        write_baseline(dir.path(), "", "");
        fs::write(
            dir.path().join(PROVENANCE_PATH),
            "{\"ticket_id\":\"T-2\",\"outcome\":\"failed\"}\n",
        )
        .unwrap();

        let output = render(
            dir.path(),
            &[ticket("T-1", Phase::Done), ticket("T-2", Phase::Implement)],
        );

        assert!(output.contains("Completed: 1 of 2 tickets; 1 remains."));
        assert!(output.contains("Run issues: 1 failed, 0 timed out."));
        assert!(!output.contains("completed the board without asking you"));
    }

    #[test]
    fn partial_fixture_does_not_invent_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        write_baseline(dir.path(), "", "");

        let output = render(
            dir.path(),
            &[
                ticket("T-1", Phase::Done),
                ticket("T-2", Phase::Implement),
                ticket("T-3", Phase::Ready),
            ],
        );

        assert!(output.contains("Completed: 1 of 3 tickets; 2 remain."));
        assert!(!output.contains("Run issues:"));
        assert!(!output.contains("completed the board without asking you"));
    }

    #[test]
    fn interactive_gate_never_claims_zero_approvals() {
        let dir = tempfile::tempdir().unwrap();
        write_baseline(dir.path(), "", "");
        fs::write(
            dir.path().join(EVENT_PATH),
            "{\"event\":\"manual-intervention\",\"kind\":\"question\"}\n",
        )
        .unwrap();

        let output = render(dir.path(), &[ticket("T-1", Phase::Done)]);

        assert!(output.contains("Manual intervention gates: 1"));
        assert!(output.contains("1 questions, 0 permission requests"));
        assert!(!output.contains("Manual approvals requested: 0"));
        assert!(!output.contains("completed the board without asking you"));
    }

    #[test]
    fn absent_evidence_paths_are_not_named() {
        let dir = tempfile::tempdir().unwrap();
        let output = render(dir.path(), &[ticket("T-1", Phase::Ready)]);

        assert!(!output.contains(PROVENANCE_PATH));
        assert!(!output.contains(COMPLETION_JOURNAL_PATH));
        assert!(!output.contains("docs/active/work/"));
        assert!(output.contains("tracking is unavailable"));
    }

    #[test]
    fn empty_work_root_is_not_presented_as_per_ticket_evidence() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/active/work")).unwrap();

        let output = render(dir.path(), &[ticket("T-1", Phase::Ready)]);

        assert!(!output.contains("docs/active/work/"));
    }

    #[test]
    fn malformed_or_stale_evidence_never_produces_clean_claims() {
        let dir = tempfile::tempdir().unwrap();
        write_baseline(dir.path(), "", "");
        fs::write(dir.path().join(PROVENANCE_PATH), "not-json\n").unwrap();
        fs::write(dir.path().join(EVENT_PATH), "partial").unwrap();

        let output = render(dir.path(), &[ticket("T-1", Phase::Done)]);

        assert!(!output.contains("completed the board without asking you"));
        assert!(!output.contains("Manual approvals requested: 0"));
        assert!(output.contains("tracking is unavailable"));
    }

    #[test]
    fn baseline_offsets_exclude_prior_run_failures_and_gates() {
        let dir = tempfile::tempdir().unwrap();
        let old_provenance = "{\"ticket_id\":\"T-1\",\"outcome\":\"failed\"}\n";
        let old_event = "{\"event\":\"manual-intervention\",\"kind\":\"permission\"}\n";
        write_baseline(dir.path(), old_provenance, old_event);
        fs::write(
            dir.path().join(PROVENANCE_PATH),
            format!("{old_provenance}{{\"ticket_id\":\"T-1\",\"outcome\":\"done\"}}\n"),
        )
        .unwrap();

        let output = render(dir.path(), &[ticket("T-1", Phase::Done)]);

        assert!(output.contains("completed the board without asking you"));
        assert!(output.contains("Manual approvals requested: 0"));
        assert!(!output.contains("Run issues:"));
    }

    #[test]
    fn record_run_baseline_captures_existing_lengths() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".lisa")).unwrap();
        fs::write(dir.path().join(PROVENANCE_PATH), "before\n").unwrap();
        fs::write(dir.path().join(EVENT_PATH), "event\n").unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.local.json"),
            crate::templates::settings_local_json(),
        )
        .unwrap();

        record_run_baseline(dir.path()).unwrap();

        let baseline = load_baseline(dir.path()).unwrap();
        assert_eq!(baseline.provenance_bytes, 7);
        assert_eq!(baseline.event_bytes, 6);
        assert!(baseline.intervention_tracking);
        assert!(!dir
            .path()
            .join(".lisa")
            .join(format!(".run-baseline.json.tmp.{}", std::process::id()))
            .exists());
    }

    #[test]
    fn baseline_disables_zero_claim_when_installed_hooks_cannot_track_gates() {
        let dir = tempfile::tempdir().unwrap();

        record_run_baseline(dir.path()).unwrap();
        let output = render(dir.path(), &[ticket("T-1", Phase::Done)]);

        assert!(output.contains("tracking is unavailable"));
        assert!(!output.contains("Manual approvals requested: 0"));
        assert!(!output.contains("completed the board without asking you"));
    }
}
