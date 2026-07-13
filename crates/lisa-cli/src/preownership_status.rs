use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use lisa_core::provenance::{AssignmentState, ProvenanceLedgerRecord};

/// Report retained failures that ended before a provider owned `ticket_id`.
pub fn run_preownership_status(ledger_path: &Path, ticket_id: &str) -> Result<(), String> {
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    write_preownership_status(ledger_path, ticket_id, &mut output)
}

/// Read and render retained pre-ownership failures from a provenance ledger.
pub fn write_preownership_status<W: Write>(
    ledger_path: &Path,
    ticket_id: &str,
    output: &mut W,
) -> Result<(), String> {
    let file = File::open(ledger_path).map_err(|error| {
        format!(
            "Failed to open provenance ledger {}: {error}",
            ledger_path.display()
        )
    })?;
    let mut matches = Vec::new();

    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|error| {
            format!(
                "Failed to read provenance ledger {} at line {line_number}: {error}",
                ledger_path.display()
            )
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let record: ProvenanceLedgerRecord = serde_json::from_str(&line).map_err(|error| {
            format!(
                "Failed to parse provenance ledger {} at line {line_number}: {error}",
                ledger_path.display()
            )
        })?;
        if let ProvenanceLedgerRecord::AssignmentTransition(record) = record {
            if record.ticket_id == ticket_id {
                matches.push(record);
            }
        }
    }

    if matches.is_empty() {
        writeln!(output, "No pre-ownership failures found for {ticket_id}.")
            .map_err(|error| format!("Failed to write pre-ownership status: {error}"))?;
        return Ok(());
    }

    writeln!(
        output,
        "Pre-ownership failures for {ticket_id} ({}):",
        matches.len()
    )
    .map_err(|error| format!("Failed to write pre-ownership status: {error}"))?;
    for (index, record) in matches.iter().enumerate() {
        if index > 0 {
            writeln!(output)
                .map_err(|error| format!("Failed to write pre-ownership status: {error}"))?;
        }
        writeln!(
            output,
            "Attempt {} (pane {})",
            record.attempt_lease.attempt_id, record.pane_id
        )
        .and_then(|_| writeln!(output, "  state: {}", assignment_state_name(record.state)))
        .and_then(|_| writeln!(output, "  reason: {}", record.reason))
        .and_then(|_| writeln!(output, "  provider: {}", record.provider))
        .and_then(|_| writeln!(output, "  started_at: {}", record.started_at))
        .and_then(|_| writeln!(output, "  ended_at: {}", record.ended_at))
        .and_then(|_| writeln!(output, "  wall_clock_secs: {}", record.wall_clock_secs))
        .map_err(|error| format!("Failed to write pre-ownership status: {error}"))?;
    }

    Ok(())
}

fn assignment_state_name(state: AssignmentState) -> &'static str {
    match state {
        AssignmentState::DeliveryFailed => "delivery-failed",
        AssignmentState::RecoveryFailed => "recovery-failed",
        AssignmentState::StartupFailed => "startup-failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const EXECUTION_ROW: &str = r#"{"schema_version":2,"ticket_id":"T-027-01","attempt_lease":{"ticket_id":"T-027-01","attempt_id":2},"outcome":"done","authoritative":true,"fenced":false,"requested":{"method":"codex","provider":"openai","model":null},"actual":{"method":"codex","provider":"openai","model":null},"started_at":1719800000,"ended_at":1719800600,"wall_clock_secs":600,"tokens_in":12000,"tokens_out":3400,"cost_usd":null,"concurrency_at_spawn":3,"pane_id":2}"#;

    fn assignment_row(ticket_id: &str, attempt_id: u64, state: &str) -> String {
        format!(
            r#"{{"schema_version":3,"record_type":"assignment-transition","ticket_id":"{ticket_id}","attempt_lease":{{"ticket_id":"{ticket_id}","attempt_id":{attempt_id}}},"pane_id":12,"provider":"openai","state":"{state}","reason":"provider did not acknowledge the bounded chat assignment","started_at":1752000000,"ended_at":1752000030,"wall_clock_secs":30}}"#
        )
    }

    #[test]
    fn preownership_status_filters_mixed_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("provenance.jsonl");
        fs::write(
            &ledger,
            format!(
                "{EXECUTION_ROW}\n{}\n{}\n",
                assignment_row("T-OTHER", 1, "startup-failed"),
                assignment_row("T-040-02-01", 7, "delivery-failed")
            ),
        )
        .unwrap();
        let mut output = Vec::new();

        write_preownership_status(&ledger, "T-040-02-01", &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            concat!(
                "Pre-ownership failures for T-040-02-01 (1):\n",
                "Attempt 7 (pane 12)\n",
                "  state: delivery-failed\n",
                "  reason: provider did not acknowledge the bounded chat assignment\n",
                "  provider: openai\n",
                "  started_at: 1752000000\n",
                "  ended_at: 1752000030\n",
                "  wall_clock_secs: 30\n",
            )
        );
    }

    #[test]
    fn preownership_status_reports_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("provenance.jsonl");
        fs::write(&ledger, format!("{EXECUTION_ROW}\n")).unwrap();
        let mut output = Vec::new();

        write_preownership_status(&ledger, "T-040-02-01", &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "No pre-ownership failures found for T-040-02-01.\n"
        );
    }

    #[test]
    fn preownership_status_reports_malformed_line_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("provenance.jsonl");
        fs::write(&ledger, format!("{EXECUTION_ROW}\nnot-json\n")).unwrap();
        let mut output = Vec::new();

        let error = write_preownership_status(&ledger, "T-040-02-01", &mut output).unwrap_err();

        assert!(error.contains(&ledger.display().to_string()));
        assert!(error.contains("line 2"));
        assert!(output.is_empty());
    }
}
