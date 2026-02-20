use std::path::Path;

use crate::config;
use lisa_core::dag::{CycleDetectionResult, Dag, DagError};
use lisa_core::types::PluginConfig;

/// Run the status command: scan tickets, build DAG, print scheduling state.
pub fn run_status(root: &Path) -> Result<(), String> {
    // Load config to get ticket directory
    let ticket_dir_rel = match config::load_config(root) {
        Ok(validation) => validation
            .config
            .dirs
            .tickets
            .unwrap_or_else(|| PluginConfig::DEFAULT_TICKET_DIR.to_string()),
        Err(_) => PluginConfig::DEFAULT_TICKET_DIR.to_string(),
    };

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
}
