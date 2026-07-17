use std::path::Path;

use crate::config;
use lisa_core::dag::{CycleDetectionResult, Dag, DagError};
use lisa_core::disposition::RemedyOwner;
use lisa_core::parking::{collect_parked_remedies, ParkedRemedy};

fn waiting_on_you_lines(remedies: &[ParkedRemedy]) -> Vec<String> {
    remedies
        .iter()
        .filter_map(|remedy| match remedy.remedy_owner {
            RemedyOwner::Operator => Some(format!("{}  {}", remedy.ticket_id, remedy.ask)),
            RemedyOwner::World => Some(format!(
                "{}  {} — Lisa checks on its own.",
                remedy.ticket_id, remedy.ask
            )),
            RemedyOwner::Agent => None,
        })
        .collect()
}

fn print_waiting_on_you(remedies: &[ParkedRemedy]) {
    let lines = waiting_on_you_lines(remedies);
    if lines.is_empty() {
        return;
    }

    println!("Waiting on you");
    for line in lines {
        println!("{line}");
    }
    println!();
}

/// Run the status command: scan tickets, build DAG, print scheduling state.
pub fn run_status(root: &Path) -> Result<(), String> {
    // Load config to get ticket directory and scheduling settings
    let resolved = match config::load_config(root) {
        Ok(validation) => config::resolve_config(&validation.config, None, None),
        Err(_) => config::ResolvedConfig::default(),
    };
    let ticket_dir_rel = resolved.ticket_dir.clone();
    let work_dir_rel = resolved.work_dir.clone();

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
    let dag = Dag::from_tickets(tickets.clone()).map_err(|e| match e {
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

    let parked_remedies = collect_parked_remedies(tickets.iter(), &root.join(&work_dir_rel));
    print_waiting_on_you(&parked_remedies);

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

    crate::run_summary::print_run_summary(root, &tickets, Path::new(&work_dir_rel))?;

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

    #[test]
    fn waiting_lines_preserve_operator_ask_and_explain_world_waiting() {
        let remedies = vec![
            ParkedRemedy {
                ticket_id: "T-001".to_string(),
                remedy_owner: RemedyOwner::Operator,
                ask: "Run the checkout test exactly once.".to_string(),
                check: None,
            },
            ParkedRemedy {
                ticket_id: "T-002".to_string(),
                remedy_owner: RemedyOwner::World,
                ask: "Wait for the release link.".to_string(),
                check: Some("test -f release".to_string()),
            },
            ParkedRemedy {
                ticket_id: "T-003".to_string(),
                remedy_owner: RemedyOwner::Agent,
                ask: "Agent retry exhausted.".to_string(),
                check: None,
            },
        ];

        assert_eq!(
            waiting_on_you_lines(&remedies),
            vec![
                "T-001  Run the checkout test exactly once.",
                "T-002  Wait for the release link. — Lisa checks on its own.",
            ]
        );
    }
}
