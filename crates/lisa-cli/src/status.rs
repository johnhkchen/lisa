use std::path::Path;

use crate::config;
use lisa_core::completion::CompletionSeal;
use lisa_core::dag::{CycleDetectionResult, Dag, DagError};
use lisa_core::disposition::RemedyOwner;
use lisa_core::notes::{collect_notes, QueuedNote};
use lisa_core::parking::{collect_parked_remedies, ParkedRemedy};
use lisa_core::provenance::{
    correct_usage, latest_world_rechecks, usage_gap, ProvenanceLedgerRecord,
    WorldRecheckObservation,
};
use std::collections::HashMap;

/// How many recorded non-passes make a world remedy worth naming as stuck.
///
/// Rows land on a doubling schedule, so this is the fourth one. Below it a
/// world remedy that has not cleared is simply waiting for the world, which is
/// what a world remedy is *for*; at it, the check has kept saying no long enough
/// that the check itself is the likelier problem.
const STUCK_NON_PASS_COUNT: u64 = 8;

/// Group a token count with thousands separators so big numbers stay legible.
fn group_thousands(value: u64) -> String {
    let digits = value.to_string();
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 && (bytes.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(char::from(*byte));
    }
    out
}

/// Render the "Token usage" block from the **corrected view** — per-ticket totals
/// layered from append-only usage corrections, never the raw first-write row.
/// A completed ticket whose capture never joined stays a visible, counted gap
/// rather than a fabricated zero (T-051-03-01).
fn token_usage_lines(records: &[ProvenanceLedgerRecord]) -> Vec<String> {
    let view = correct_usage(records);
    let gap = usage_gap(records);

    let mut lines = vec!["Token usage".to_string()];

    let mut joined = 0usize;
    let mut total_in = 0u64;
    let mut total_out = 0u64;
    for (ticket_id, usage) in &view {
        if let (Some(tokens_in), Some(tokens_out)) = (usage.tokens_in, usage.tokens_out) {
            joined += 1;
            total_in = total_in.saturating_add(tokens_in);
            total_out = total_out.saturating_add(tokens_out);
            lines.push(format!(
                "  {:<12}  {} in / {} out",
                ticket_id,
                group_thousands(tokens_in),
                group_thousands(tokens_out),
            ));
        }
    }

    if joined == 0 {
        lines.push("  Nothing measured yet.".to_string());
    } else {
        lines.push(format!(
            "  Joined {} ticket{}: {} in / {} out",
            joined,
            if joined == 1 { "" } else { "s" },
            group_thousands(total_in),
            group_thousands(total_out),
        ));
    }

    if !gap.is_empty() {
        lines.push(format!(
            "  Not yet joined: {} completed ticket{} — usage capture pending or never arrived.",
            gap.len(),
            if gap.len() == 1 { "" } else { "s" },
        ));
    }

    lines
}

fn print_token_usage(ledger_path: &Path) {
    let records: Vec<ProvenanceLedgerRecord> = std::fs::read_to_string(ledger_path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    for line in token_usage_lines(&records) {
        println!("{line}");
    }
    println!();
}

/// The two lines a world remedy earns once its check has kept saying no.
///
/// Automation never acts on a non-pass — that policy is unchanged — so the only
/// thing that can change is the silence. This says what Lisa has seen and names
/// the one command that ends the wait, addressed to the person who can decide
/// the check is wrong.
fn stuck_world_lines(remedy: &ParkedRemedy, seen: &WorldRecheckObservation) -> Vec<String> {
    // "at least": rows are sampled, so the last recorded total is a floor rather
    // than a live count, and saying otherwise would overstate what Lisa knows.
    vec![
        format!(
            "       Lisa has checked at least {} times and it still isn't passing.",
            seen.non_pass_count
        ),
        format!(
            "       If you have checked this yourself, run: lisa unblock {} --override-check",
            remedy.ticket_id
        ),
    ]
}

fn waiting_on_you_lines(
    remedies: &[ParkedRemedy],
    rechecks: &HashMap<String, WorldRecheckObservation>,
) -> Vec<String> {
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
            // The count belongs to one check. A rewritten disposition is a
            // different claim about the world, and inherits nothing.
            let stuck = (remedy.remedy_owner == RemedyOwner::World)
                .then(|| rechecks.get(&remedy.ticket_id))
                .flatten()
                .filter(|seen| {
                    remedy.check.as_deref() == Some(seen.check.as_str())
                        && seen.non_pass_count >= STUCK_NON_PASS_COUNT
                });
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
            if let Some(seen) = stuck {
                lines.extend(stuck_world_lines(remedy, seen));
            }
            lines.push(format!("       Reviewer's note: {}", remedy.reason));
            lines
        })
        .collect()
}

fn print_waiting_on_you(
    remedies: &[ParkedRemedy],
    rechecks: &HashMap<String, WorldRecheckObservation>,
) {
    let lines = waiting_on_you_lines(remedies, rechecks);
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

    let ledger_path = root.join(".lisa/provenance.jsonl");
    let parked_remedies =
        collect_parked_remedies(tickets.iter(), &root.join(&work_dir_rel), &ledger_path);
    print_waiting_on_you(&parked_remedies, &latest_world_rechecks(&ledger_path));
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

    print_token_usage(&root.join(".lisa/provenance.jsonl"));

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
                steps: Vec::new(),
                check: None,
                check_timeout_secs: None,
                proposal: None,
                origin: lisa_core::disposition::DispositionOrigin::Review,
            },
            ParkedRemedy {
                ticket_id: "T-002".to_string(),
                remedy_owner: RemedyOwner::World,
                ask: "Wait for the release link.".to_string(),
                reason: "The release has not reached the mirror.".to_string(),
                steps: Vec::new(),
                check: Some("test -f release".to_string()),
                check_timeout_secs: None,
                proposal: None,
                origin: lisa_core::disposition::DispositionOrigin::Review,
            },
            ParkedRemedy {
                ticket_id: "T-003".to_string(),
                remedy_owner: RemedyOwner::Agent,
                ask: "Agent retry exhausted.".to_string(),
                reason: "The agent can retry this work.".to_string(),
                steps: Vec::new(),
                check: None,
                check_timeout_secs: None,
                proposal: None,
                origin: lisa_core::disposition::DispositionOrigin::Review,
            },
        ];

        assert_eq!(
            waiting_on_you_lines(&remedies, &HashMap::new()),
            vec![
                "T-001  Run the checkout test exactly once.",
                "       Reviewer's note: The checkout evidence is missing.",
                "T-002  Wait for the release link. — Lisa checks on its own.",
                "       Reviewer's note: The release has not reached the mirror.",
            ]
        );
    }

    /// A world remedy whose check keeps saying no stops being silent — but only
    /// once it has said so enough times to mean something.
    #[test]
    fn a_world_remedy_that_never_clears_says_so_and_names_the_way_through() {
        let remedies = vec![ParkedRemedy {
            ticket_id: "T-WORLD".to_string(),
            remedy_owner: RemedyOwner::World,
            ask: "Wait for the release link.".to_string(),
            reason: "The release has not reached the mirror.".to_string(),
            steps: Vec::new(),
            check: Some("curl -fsS https://example.test/release".to_string()),
            check_timeout_secs: None,
            proposal: None,
            origin: lisa_core::disposition::DispositionOrigin::Review,
        }];
        let observation = |count, check: &str| {
            HashMap::from([(
                "T-WORLD".to_string(),
                WorldRecheckObservation {
                    check: check.to_string(),
                    result: lisa_core::provenance::WorldRecheckOutcome::Failed,
                    non_pass_count: count,
                    occurred_at: 1_752_900_000,
                },
            )])
        };
        let quiet = vec![
            "T-WORLD  Wait for the release link. — Lisa checks on its own.".to_string(),
            "       Reviewer's note: The release has not reached the mirror.".to_string(),
        ];

        // Below the threshold, and for a check the count does not belong to,
        // nothing changes: waiting for the world is what a world remedy does.
        assert_eq!(
            waiting_on_you_lines(&remedies, &HashMap::new()),
            quiet,
            "no observation"
        );
        assert_eq!(
            waiting_on_you_lines(
                &remedies,
                &observation(
                    STUCK_NON_PASS_COUNT - 1,
                    "curl -fsS https://example.test/release"
                )
            ),
            quiet,
            "under the threshold"
        );
        assert_eq!(
            waiting_on_you_lines(
                &remedies,
                &observation(STUCK_NON_PASS_COUNT, "test -f some-older-check")
            ),
            quiet,
            "a count belonging to a different check"
        );

        assert_eq!(
            waiting_on_you_lines(
                &remedies,
                &observation(
                    STUCK_NON_PASS_COUNT,
                    "curl -fsS https://example.test/release"
                )
            ),
            vec![
                "T-WORLD  Wait for the release link. — Lisa checks on its own.",
                "       Lisa has checked at least 8 times and it still isn't passing.",
                "       If you have checked this yourself, run: lisa unblock T-WORLD --override-check",
                "       Reviewer's note: The release has not reached the mirror.",
            ]
        );
    }

    /// The count is a world remedy's fact. An operator-owned park is theirs to
    /// clear, and Lisa never rechecks one, so it never accumulates a count.
    #[test]
    fn an_operator_owned_remedy_never_shows_a_recheck_count() {
        let remedies = vec![ParkedRemedy {
            ticket_id: "T-OPERATOR".to_string(),
            remedy_owner: RemedyOwner::Operator,
            ask: "Run the checkout test.".to_string(),
            reason: "The checkout evidence is missing.".to_string(),
            steps: Vec::new(),
            check: Some("test -f evidence".to_string()),
            check_timeout_secs: None,
            proposal: None,
            origin: lisa_core::disposition::DispositionOrigin::Review,
        }];
        let rechecks = HashMap::from([(
            "T-OPERATOR".to_string(),
            WorldRecheckObservation {
                check: "test -f evidence".to_string(),
                result: lisa_core::provenance::WorldRecheckOutcome::Failed,
                non_pass_count: 64,
                occurred_at: 1_752_900_000,
            },
        )]);

        assert_eq!(
            waiting_on_you_lines(&remedies, &rechecks),
            vec![
                "T-OPERATOR  Run the checkout test.",
                "       Reviewer's note: The checkout evidence is missing.",
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
            steps: Vec::new(),
            check: None,
            check_timeout_secs: None,
            proposal: None,
            origin: lisa_core::disposition::DispositionOrigin::Review,
        }];

        let lines = waiting_on_you_lines(&remedies, &HashMap::new());

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
            steps: Vec::new(),
            check: None,
            check_timeout_secs: None,
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
            origin: lisa_core::disposition::DispositionOrigin::Review,
        }];
        let lines = waiting_on_you_lines(&remedies, &HashMap::new());
        assert!(lines[0].contains("First responder"));
        assert!(lines[0].contains("criteria"));
        assert!(lines[0].contains("evidence"));
        assert!(lines[1].contains("Suggested: Amend"));
        assert!(lines[2].contains("Prepared:"));
        assert!(lines[3].contains("Original ask:"));
        assert!(lines[4].contains("Reviewer's note:"));
    }

    fn parse_ledger(jsonl: &str) -> Vec<ProvenanceLedgerRecord> {
        jsonl
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    // A null-token Done row and a matching usage-correction, as written to the
    // ledger by the plugin.
    fn null_done(ticket: &str) -> String {
        format!(
            r#"{{"schema_version":9,"seal":"commit","ticket_id":"{ticket}","attempt_lease":{{"ticket_id":"{ticket}","attempt_id":1}},"outcome":"done","authoritative":true,"fenced":false,"requested":{{"method":"codex","provider":"openai","model":null}},"actual":{{"method":"codex","provider":"openai","model":null}},"started_at":1000,"ended_at":1100,"wall_clock_secs":100,"tokens_in":null,"tokens_out":null,"cost_usd":null,"concurrency_at_spawn":0,"pane_id":1}}"#
        )
    }

    fn correction(ticket: &str, tokens_in: u64, tokens_out: u64) -> String {
        format!(
            r#"{{"schema_version":9,"record_type":"usage-correction","ticket_id":"{ticket}","attempt_lease":{{"ticket_id":"{ticket}","attempt_id":1}},"method":"codex","session_id":"s","pane_id":1,"source_line":1,"captured_at":1150,"tokens_in":{tokens_in},"tokens_out":{tokens_out},"occurred_at":1200}}"#
        )
    }

    #[test]
    fn token_usage_reads_the_corrected_view_not_the_raw_row() {
        let ledger = format!(
            "{}\n{}\n",
            null_done("T-A"),
            correction("T-A", 1_234_567, 8_900)
        );
        let lines = token_usage_lines(&parse_ledger(&ledger));
        assert_eq!(lines[0], "Token usage");
        // The raw row is null; the corrected total comes from the correction,
        // rendered with thousands separators.
        assert!(lines.iter().any(|line| line.contains("T-A")
            && line.contains("1,234,567 in")
            && line.contains("8,900 out")));
        assert!(lines.iter().any(|line| line.contains("Joined 1 ticket:")));
        assert!(!lines.iter().any(|line| line.contains("Not yet joined")));
    }

    #[test]
    fn token_usage_counts_the_capture_never_gap() {
        // T-A joined; T-B completed but never joined a capture.
        let ledger = format!(
            "{}\n{}\n{}\n",
            null_done("T-A"),
            correction("T-A", 100, 10),
            null_done("T-B"),
        );
        let lines = token_usage_lines(&parse_ledger(&ledger));
        assert!(lines.iter().any(|line| line.contains("Joined 1 ticket:")));
        assert!(lines
            .iter()
            .any(|line| line.contains("Not yet joined: 1 completed ticket")));
        // The gap ticket is never printed with a fabricated zero.
        assert!(!lines
            .iter()
            .any(|line| line.contains("T-B") && line.contains("0 in")));
    }

    #[test]
    fn token_usage_empty_ledger_says_nothing_yet() {
        let lines = token_usage_lines(&[]);
        assert_eq!(
            lines,
            vec![
                "Token usage".to_string(),
                "  Nothing measured yet.".to_string()
            ]
        );
    }
}
