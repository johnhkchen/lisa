use std::path::Path;

use crate::config;
use lisa_core::completion::CompletionSeal;
use lisa_core::dag::{CycleDetectionResult, Dag, DagError};
use lisa_core::disposition::RemedyOwner;
use lisa_core::notes::{collect_notes, QueuedNote};
use lisa_core::parking::{collect_parked_remedies, ParkedRemedy};

fn waiting_on_you_lines(remedies: &[ParkedRemedy]) -> Vec<String> {
    remedies
        .iter()
        .flat_map(|remedy| {
            let lead = match remedy.remedy_owner {
                RemedyOwner::Operator => format!("{}  {}", remedy.ticket_id, remedy.ask),
                RemedyOwner::World => format!(
                    "{}  {} — Lisa checks on its own.",
                    remedy.ticket_id, remedy.ask
                ),
                RemedyOwner::Agent => return Vec::new(),
            };
            let mut lines = Vec::new();
            if let Some(proposal) = &remedy.proposal {
                lines.push(format!(
                    "{}  First responder: {}",
                    remedy.ticket_id, proposal.summary
                ));
                lines.push(format!("       Suggested: {}", proposal.recommendation));
                lines.extend(
                    proposal
                        .prepared_steps
                        .iter()
                        .map(|step| format!("       Prepared: {}", step.display())),
                );
                lines.push(format!("       Original ask: {}", remedy.ask));
            } else {
                lines.push(lead);
            }
            lines.push(format!("       Reviewer's note: {}", remedy.reason));
            lines
        })
        .collect()
}

fn print_waiting_on_you(remedies: &[ParkedRemedy]) {
    let lines = waiting_on_you_lines(remedies);
    if lines.is_empty() {
        println!("Waiting on you");
        println!("Nothing waiting.\n");
        return;
    }

    println!("Waiting on you");
    for line in lines {
        println!("{line}");
    }
    println!();
}

fn print_status_notes(notes: &[QueuedNote]) {
    if notes.is_empty() {
        println!("Notes for you");
        println!("Nothing to read.\n");
    } else {
        crate::notes::print_notes(notes);
    }
}

fn format_config_summary(resolved: &config::ResolvedConfig, seal: CompletionSeal) -> String {
    let timeout_str = if resolved.session_timeout_secs == 0 {
        "disabled".to_string()
    } else {
        format!("{}s", resolved.session_timeout_secs)
    };
    let mut output = format!(
        "Config: max_threads={}, session_timeout={}\n",
        resolved.max_threads, timeout_str
    );
    if !resolved.phase_timeouts.is_empty() {
        let mut entries: Vec<_> = resolved.phase_timeouts.iter().collect();
        entries.sort_by_key(|(key, _)| (*key).clone());
        let parts: Vec<String> = entries
            .iter()
            .map(|(key, value)| format!("{}={}s", key, value))
            .collect();
        output.push_str(&format!("  phase_timeouts: {}\n", parts.join(" ")));
    }
    output.push_str(crate::completion_seal::visibility_line(seal));
    output.push('\n');
    output
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
    let completion_seal =
        crate::completion_seal::resolve_for_inspection(root, resolved.completion_mode);

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

    let parked_remedies = collect_parked_remedies(tickets.iter(), &root.join(&work_dir_rel));
    print_waiting_on_you(&parked_remedies);
    let notes = collect_notes(
        &root.join(".lisa/completion-journal.jsonl"),
        &root.join(".lisa/provenance.jsonl"),
    )?;
    print_status_notes(&notes);

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
    print!("{}", format_config_summary(&resolved, completion_seal));
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
    fn status_completion_fixtures_show_both_plain_language_tiers() {
        let resolved = config::ResolvedConfig::default();
        let commit = format_config_summary(&resolved, CompletionSeal::Commit);
        assert!(commit.contains("completion seal: commit-sealed — finished work lands as history"));

        let journal = format_config_summary(&resolved, CompletionSeal::Journal);
        assert!(journal.contains(
            "completion seal: journal-only — finished work is recorded but not undoable"
        ));
        assert!(!journal.to_ascii_lowercase().contains("git"));
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
                reason: "The checkout evidence is missing.".to_string(),
                check: None,
                proposal: None,
            },
            ParkedRemedy {
                ticket_id: "T-002".to_string(),
                remedy_owner: RemedyOwner::World,
                ask: "Wait for the release link.".to_string(),
                reason: "The release has not reached the mirror.".to_string(),
                check: Some("test -f release".to_string()),
                proposal: None,
            },
            ParkedRemedy {
                ticket_id: "T-003".to_string(),
                remedy_owner: RemedyOwner::Agent,
                ask: "Agent retry exhausted.".to_string(),
                reason: "The agent can retry this work.".to_string(),
                check: None,
                proposal: None,
            },
        ];

        assert_eq!(
            waiting_on_you_lines(&remedies),
            vec![
                "T-001  Run the checkout test exactly once.",
                "       Reviewer's note: The checkout evidence is missing.",
                "T-002  Wait for the release link. — Lisa checks on its own.",
                "       Reviewer's note: The release has not reached the mirror.",
            ]
        );
    }

    #[test]
    fn legacy_field_block_leads_with_the_standard_plain_ask() {
        const FIELD_REASON: &str = "The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant bypassed the old binary through managed mode instead of recording the required recovery through Lisa's error strings; John must either provide conforming reruns or explicitly amend both acceptance requirements before Review can pass.";
        let remedies = vec![ParkedRemedy {
            ticket_id: "T-046-06-03".to_string(),
            remedy_owner: RemedyOwner::Operator,
            ask: lisa_core::parking::LEGACY_BLOCK_ASK.to_string(),
            reason: FIELD_REASON.to_string(),
            check: None,
            proposal: None,
        }];

        let lines = waiting_on_you_lines(&remedies);

        assert_eq!(
            lines,
            vec![
                format!("T-046-06-03  {}", lisa_core::parking::LEGACY_BLOCK_ASK),
                format!("       Reviewer's note: {FIELD_REASON}"),
            ]
        );
        assert!(!lines[0].contains(FIELD_REASON));
    }

    #[test]
    fn field_proposal_leads_with_gap_amendment_and_prepared_edit() {
        use lisa_core::triage::{PreparedStep, TriageProposal};

        let remedies = vec![ParkedRemedy {
            ticket_id: "T-046-06-03".to_string(),
            remedy_owner: RemedyOwner::Operator,
            ask: lisa_core::parking::LEGACY_BLOCK_ASK.to_string(),
            reason: "The 225 MiB measurement conflicts with the approximately 200 MiB criterion."
                .to_string(),
            check: None,
            proposal: Some(TriageProposal {
                summary: "The written criteria conflict with the measured evidence.".to_string(),
                recommendation: "Amend the stale criteria.".to_string(),
                prepared_steps: vec![PreparedStep::FileEdit {
                    description: "Use the calibrated bound.".to_string(),
                    path: std::path::PathBuf::from("docs/active/tickets/T-046-06-03.md"),
                    old: "approximately 200 MiB".to_string(),
                    new: "the calibrated 300 MiB bound".to_string(),
                }],
            }),
        }];
        let lines = waiting_on_you_lines(&remedies);
        assert!(lines[0].contains("First responder"));
        assert!(lines[0].contains("criteria"));
        assert!(lines[0].contains("evidence"));
        assert!(lines[1].contains("Suggested: Amend"));
        assert!(lines[2].contains("Prepared:"));
        assert!(lines[3].contains("Original ask:"));
        assert!(lines[4].contains("Reviewer's note:"));
    }
}
