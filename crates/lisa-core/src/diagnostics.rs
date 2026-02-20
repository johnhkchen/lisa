//! Startup diagnostics for the Lisa plugin.
//!
//! Produces a sequence of `ActivityEvent`s that summarize what the plugin found
//! at load time: config values, scan results, DAG health, and readiness.

use std::path::Path;

use crate::dag::{CycleDetectionResult, Dag, DagError};
use crate::ticket::ScanResult;
use crate::types::{ActivityEvent, PluginConfig};

/// Produce startup diagnostic events from the results of scanning and DAG construction.
///
/// This is a pure function with no side effects — the caller is responsible for
/// feeding the returned events into the activity log.
pub fn startup_diagnostics(
    config: &PluginConfig,
    scan_result: &ScanResult,
    dag_result: &Result<Dag, DagError>,
    commit_lock_path: &Path,
) -> Vec<ActivityEvent> {
    let mut events = Vec::new();

    // 1. Config values
    events.push(ActivityEvent::Info {
        message: format!(
            "Config: ticket_dir={}, max_threads={}, commit_lock={}",
            config.ticket_dir.display(),
            config.max_threads,
            commit_lock_path.display(),
        ),
    });

    // 2. Per-file parse errors
    for (path, error) in &scan_result.errors {
        events.push(ActivityEvent::Error {
            message: format!(
                "Failed to parse {}: {}",
                path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                error,
            ),
        });
    }

    // 3. No tickets found
    if scan_result.tickets.is_empty() && scan_result.errors.is_empty() {
        events.push(ActivityEvent::Warning {
            message: format!("No tickets found in {}", config.ticket_dir.display(),),
        });
        return events;
    }

    if scan_result.tickets.is_empty() && !scan_result.errors.is_empty() {
        events.push(ActivityEvent::Warning {
            message: "No valid tickets found (all had parse errors)".to_string(),
        });
        return events;
    }

    // 4. DAG result
    match dag_result {
        Err(DagError::MissingDependency {
            ticket_id,
            missing_dep,
        }) => {
            events.push(ActivityEvent::Error {
                message: format!(
                    "DAG error: {} depends on {} which does not exist",
                    ticket_id, missing_dep,
                ),
            });
        }
        Err(DagError::CycleDetected(nodes)) => {
            events.push(ActivityEvent::Error {
                message: format!("DAG has cycle involving: {}", nodes.join(", ")),
            });
        }
        Ok(dag) => {
            // 5. Cycle check (Dag::from_tickets succeeds even with cycles)
            if let CycleDetectionResult::Cycle(cycle_nodes) = dag.detect_cycles() {
                events.push(ActivityEvent::Error {
                    message: format!("DAG has cycle involving: {}", cycle_nodes.join(", "),),
                });
            }

            // 6. Summary
            let ready_count = dag.get_ready_tickets().len();
            events.push(ActivityEvent::Info {
                message: format!(
                    "Loaded {} tickets, {} edges, {} ready, max_threads={}",
                    dag.len(),
                    dag.edge_count(),
                    ready_count,
                    config.max_threads,
                ),
            });
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Phase, Priority, Ticket, TicketStatus, TicketType};
    use std::path::PathBuf;

    fn make_ticket(id: &str, phase: Phase, depends_on: Vec<&str>) -> Ticket {
        Ticket {
            id: id.to_string(),
            story: None,
            title: format!("Test {}", id),
            ticket_type: TicketType::Task,
            status: TicketStatus::Open,
            priority: Priority::Medium,
            phase,
            depends_on: depends_on.into_iter().map(|s| s.to_string()).collect(),
            blocks: Vec::new(),
            file_path: PathBuf::new(),
            content: String::new(),
        }
    }

    fn default_config() -> PluginConfig {
        PluginConfig::new()
    }

    fn lock_path() -> PathBuf {
        PathBuf::from("/host/.lisa-commit.lock")
    }

    #[test]
    fn test_diagnostics_clean_load() {
        let config = default_config();
        let scan = ScanResult {
            tickets: vec![
                make_ticket("T-001", Phase::Done, vec![]),
                make_ticket("T-002", Phase::Ready, vec!["T-001"]),
                make_ticket("T-003", Phase::Ready, vec!["T-001"]),
            ],
            errors: vec![],
        };
        let dag = Dag::from_tickets(scan.tickets.clone()).unwrap();
        let dag_result = Ok(dag);

        let events = startup_diagnostics(&config, &scan, &dag_result, &lock_path());

        // Should have config info + summary info = 2 events
        assert_eq!(events.len(), 2);

        // First: config
        match &events[0] {
            ActivityEvent::Info { message } => {
                assert!(message.contains("ticket_dir="));
                assert!(message.contains("max_threads=2"));
                assert!(message.contains("commit_lock="));
            }
            other => panic!("Expected Info, got {:?}", other),
        }

        // Second: summary
        match &events[1] {
            ActivityEvent::Info { message } => {
                assert!(message.contains("3 tickets"));
                assert!(message.contains("2 edges"));
                assert!(message.contains("2 ready"));
                assert!(message.contains("max_threads=2"));
            }
            other => panic!("Expected Info, got {:?}", other),
        }
    }

    #[test]
    fn test_diagnostics_parse_errors() {
        let config = default_config();
        let scan = ScanResult {
            tickets: vec![make_ticket("T-001", Phase::Ready, vec![])],
            errors: vec![
                (
                    PathBuf::from("/host/docs/active/tickets/T-BAD.md"),
                    crate::ticket::TicketError::MissingFrontmatter,
                ),
                (
                    PathBuf::from("/host/docs/active/tickets/T-ALSO-BAD.md"),
                    crate::ticket::TicketError::MissingField("phase".to_string()),
                ),
            ],
        };
        let dag = Dag::from_tickets(scan.tickets.clone()).unwrap();
        let dag_result = Ok(dag);

        let events = startup_diagnostics(&config, &scan, &dag_result, &lock_path());

        // config + 2 errors + summary = 4
        assert_eq!(events.len(), 4);

        // Parse errors
        match &events[1] {
            ActivityEvent::Error { message } => {
                assert!(message.contains("T-BAD.md"));
                assert!(message.contains("frontmatter"));
            }
            other => panic!("Expected Error, got {:?}", other),
        }
        match &events[2] {
            ActivityEvent::Error { message } => {
                assert!(message.contains("T-ALSO-BAD.md"));
                assert!(message.contains("phase"));
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_diagnostics_cycles() {
        let config = default_config();
        let tickets = vec![
            make_ticket("T-001", Phase::Ready, vec!["T-002"]),
            make_ticket("T-002", Phase::Ready, vec!["T-001"]),
        ];
        let scan = ScanResult {
            tickets: tickets.clone(),
            errors: vec![],
        };
        let dag = Dag::from_tickets(tickets).unwrap();
        let dag_result = Ok(dag);

        let events = startup_diagnostics(&config, &scan, &dag_result, &lock_path());

        // Should contain a cycle error
        let has_cycle_error = events.iter().any(|e| match e {
            ActivityEvent::Error { message } => message.contains("cycle"),
            _ => false,
        });
        assert!(has_cycle_error, "Expected cycle error in: {:?}", events);
    }

    #[test]
    fn test_diagnostics_no_tickets() {
        let config = default_config();
        let scan = ScanResult {
            tickets: vec![],
            errors: vec![],
        };
        let dag = Dag::from_tickets(vec![]).unwrap();
        let dag_result = Ok(dag);

        let events = startup_diagnostics(&config, &scan, &dag_result, &lock_path());

        // Should have config info + warning
        let has_warning = events.iter().any(|e| match e {
            ActivityEvent::Warning { message } => message.contains("No tickets found"),
            _ => false,
        });
        assert!(
            has_warning,
            "Expected warning about no tickets: {:?}",
            events
        );
    }

    #[test]
    fn test_diagnostics_config_values() {
        let mut config = default_config();
        config.max_threads = 8;
        config.ticket_dir = PathBuf::from("/custom/tickets");

        let scan = ScanResult {
            tickets: vec![make_ticket("T-001", Phase::Ready, vec![])],
            errors: vec![],
        };
        let dag = Dag::from_tickets(scan.tickets.clone()).unwrap();
        let dag_result = Ok(dag);
        let lock = PathBuf::from("/custom/.lock");

        let events = startup_diagnostics(&config, &scan, &dag_result, &lock);

        match &events[0] {
            ActivityEvent::Info { message } => {
                assert!(
                    message.contains("/custom/tickets"),
                    "ticket_dir missing: {}",
                    message
                );
                assert!(
                    message.contains("max_threads=8"),
                    "max_threads missing: {}",
                    message
                );
                assert!(
                    message.contains("/custom/.lock"),
                    "commit_lock missing: {}",
                    message
                );
            }
            other => panic!("Expected Info, got {:?}", other),
        }
    }

    #[test]
    fn test_diagnostics_dag_missing_dependency() {
        let config = default_config();
        let tickets = vec![make_ticket("T-001", Phase::Ready, vec!["T-999"])];
        let scan = ScanResult {
            tickets: tickets.clone(),
            errors: vec![],
        };
        let dag_result = Dag::from_tickets(tickets);
        assert!(dag_result.is_err());

        let events = startup_diagnostics(&config, &scan, &dag_result, &lock_path());

        let has_dep_error = events.iter().any(|e| match e {
            ActivityEvent::Error { message } => {
                message.contains("T-001") && message.contains("T-999")
            }
            _ => false,
        });
        assert!(has_dep_error, "Expected missing dep error: {:?}", events);
    }
}
