use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::config;
use lisa_core::dag::{CycleDetectionResult, Dag, DagError};
use lisa_core::provenance::{AssignmentState, ProvenanceLedgerRecord};

/// Report retained failures that ended before a provider owned `ticket_id`.
pub fn run_preownership_status(ledger_path: &Path, ticket_id: &str) -> Result<(), String> {
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    write_preownership_status(ledger_path, ticket_id, &mut output)
}

fn write_preownership_status<W: Write>(
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

/// Run the status command: scan tickets, build DAG, print scheduling state.
pub fn run_status(root: &Path) -> Result<(), String> {
    // Load config to get ticket directory and scheduling settings
    let resolved = match config::load_config(root) {
        Ok(validation) => config::resolve_config(&validation.config, None, None),
        Err(_) => config::ResolvedConfig::default(),
    };
    let ticket_dir_rel = resolved.ticket_dir.clone();

    let ticket_dir = root.join(&ticket_dir_rel);
    if !ticket_dir.exists() {
        return Err(format!(
            "Ticket directory not found: {}",
            ticket_dir.display()
        ));
    }

    // Scan tickets
    let tickets = lisa_core::ticket::scan_tickets(&ticket_dir)
        .map_err(|e| format!("Failed to scan tickets: {}", e))?;

    if tickets.is_empty() {
        println!("No tickets found in {}", ticket_dir_rel);
        return Ok(());
    }

    // Build DAG
    let dag = Dag::from_tickets(tickets).map_err(|e| match e {
        DagError::MissingDependency {
            ticket_id,
            missing_dep,
        } => format!(
            "Ticket {} depends on {} which does not exist",
            ticket_id, missing_dep
        ),
        DagError::CycleDetected(nodes) => {
            format!("Cycle detected involving: {}", nodes.join(", "))
        }
    })?;

    // Check for cycles
    match dag.detect_cycles() {
        CycleDetectionResult::NoCycle => {}
        CycleDetectionResult::Cycle(nodes) => {
            return Err(format!("Cycle detected involving: {}", nodes.join(", ")));
        }
    }

    // Print summary header
    let stats = dag.stats();
    println!(
        "DAG: {} tickets, {} edges, no cycles",
        dag.len(),
        dag.edge_count()
    );
    println!("Critical path: {} tickets", stats.critical_path_length);
    println!(
        "Status: {} done, {} in progress, {} ready, {} blocked",
        stats.done_tickets, stats.in_progress_tickets, stats.ready_tickets, stats.blocked_tickets
    );
    let timeout_str = if resolved.session_timeout_secs == 0 {
        "disabled".to_string()
    } else {
        format!("{}s", resolved.session_timeout_secs)
    };
    println!(
        "Config: max_threads={}, session_timeout={}",
        resolved.max_threads, timeout_str
    );
    if !resolved.phase_timeouts.is_empty() {
        let mut entries: Vec<_> = resolved.phase_timeouts.iter().collect();
        entries.sort_by_key(|(k, _)| (*k).clone());
        let parts: Vec<String> = entries
            .iter()
            .map(|(k, v)| format!("{}={}s", k, v))
            .collect();
        println!("  phase_timeouts: {}", parts.join(" "));
    }
    println!();

    // Print execution waves
    let waves = dag
        .execution_waves()
        .map_err(|_| "Failed to compute execution waves".to_string())?;

    for (i, wave) in waves.iter().enumerate() {
        let label = if i == 0 {
            "no dependencies".to_string()
        } else {
            format!("depends on wave {}", i - 1)
        };
        println!("Wave {} ({}):", i, label);

        for id in wave {
            if let Some(ticket) = dag.get_ticket(id) {
                let deps = dag.get_dependencies(id);
                let blocks = dag.get_blocked_by(id);

                let deps_str = if deps.is_empty() {
                    String::new()
                } else {
                    let mut deps_sorted: Vec<_> = deps.into_iter().collect();
                    deps_sorted.sort();
                    format!("  deps: {}", deps_sorted.join(", "))
                };

                let blocks_str = if blocks.is_empty() {
                    String::new()
                } else {
                    let mut blocks_sorted: Vec<_> = blocks.into_iter().collect();
                    blocks_sorted.sort();
                    format!("  blocks: {}", blocks_sorted.join(", "))
                };

                println!(
                    "  {:<12}  {:<12}  {:<12}  {}{}{}",
                    ticket.id, ticket.phase, ticket.status, ticket.title, deps_str, blocks_str
                );
            }
        }
        println!();
    }

    // Print ready-to-schedule summary
    let ready = dag.get_ready_tickets();
    if ready.is_empty() {
        println!("No tickets ready to schedule.");
    } else {
        let mut ready_sorted = ready;
        ready_sorted.sort();
        println!("Ready to schedule: {}", ready_sorted.join(", "));
    }

    Ok(())
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

    fn setup_valid_project(dir: &Path) {
        fs::write(dir.join("CLAUDE.md"), "# CLAUDE.md").unwrap();
        fs::create_dir_all(dir.join("docs/active/tickets")).unwrap();
    }

    fn write_ticket(dir: &Path, filename: &str, content: &str) {
        fs::write(dir.join("docs/active/tickets").join(filename), content).unwrap();
    }

    #[test]
    fn test_status_no_tickets() {
        let dir = tempfile::tempdir().unwrap();
        setup_valid_project(dir.path());

        let result = run_status(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_single_ticket() {
        let dir = tempfile::tempdir().unwrap();
        setup_valid_project(dir.path());

        write_ticket(
            dir.path(),
            "T-001.md",
            "---\nid: T-001\ntitle: first-ticket\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\n## Context\nTest.\n",
        );

        let result = run_status(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_dependency_chain() {
        let dir = tempfile::tempdir().unwrap();
        setup_valid_project(dir.path());

        write_ticket(
            dir.path(),
            "T-001.md",
            "---\nid: T-001\ntitle: first\ntype: task\nstatus: open\npriority: high\nphase: done\n---\n\nDone.\n",
        );
        write_ticket(
            dir.path(),
            "T-002.md",
            "---\nid: T-002\ntitle: second\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-001]\n---\n\nReady.\n",
        );
        write_ticket(
            dir.path(),
            "T-003.md",
            "---\nid: T-003\ntitle: third\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-002]\n---\n\nBlocked.\n",
        );

        let result = run_status(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_cycle_error() {
        let dir = tempfile::tempdir().unwrap();
        setup_valid_project(dir.path());

        write_ticket(
            dir.path(),
            "T-001.md",
            "---\nid: T-001\ntitle: a\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-002]\n---\n\nA.\n",
        );
        write_ticket(
            dir.path(),
            "T-002.md",
            "---\nid: T-002\ntitle: b\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-001]\n---\n\nB.\n",
        );

        let result = run_status(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_status_missing_dep_error() {
        let dir = tempfile::tempdir().unwrap();
        setup_valid_project(dir.path());

        write_ticket(
            dir.path(),
            "T-001.md",
            "---\nid: T-001\ntitle: a\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-999]\n---\n\nA.\n",
        );

        let result = run_status(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_status_missing_ticket_dir() {
        let dir = tempfile::tempdir().unwrap();
        // Don't create the ticket directory

        let result = run_status(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_status_respects_config() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("custom/tickets")).unwrap();
        fs::write(
            dir.path().join(".lisa.toml"),
            "[dirs]\ntickets = \"custom/tickets\"\n",
        )
        .unwrap();

        // Write ticket to the custom directory
        fs::write(
            dir.path().join("custom/tickets/T-001.md"),
            "---\nid: T-001\ntitle: first\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\nTest.\n",
        )
        .unwrap();

        // Should find the ticket in the custom directory
        let result = run_status(dir.path());
        assert!(result.is_ok());
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
