//! Lisa/Ralph - A Zellij plugin for DAG-driven concurrent task scheduling
//!
//! This plugin implements the RDSPI workflow (Research -> Design -> Structure -> Plan -> Implement)
//! as a DAG-driven concurrent scheduler. It manages Claude Code sessions for each ticket,
//! tracks phase progress, and provides a live dashboard.

mod scheduler;
mod ui;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use zellij_tile::prelude::*;

use lisa_core::dag::Dag;
use lisa_core::ticket;
use lisa_core::types::{ActivityEvent, Phase, PluginConfig, Thread, TicketId};

/// How often (in seconds) the plugin rescans ticket files to detect phase changes.
const POLL_INTERVAL_SECS: f64 = 5.0;

/// The prompt text sent to Claude Code for a ticket.
fn ticket_prompt(ticket_dir: &Path, ticket_id: &str) -> String {
    let ticket_path = ticket_dir.join(format!("{}.md", ticket_id));
    format!(
        "Read the ticket at {}, the project context in CLAUDE.md, and the RDSPI workflow in docs/knowledge/rdspi-workflow.md. \
         Start from the current phase indicated in the ticket frontmatter.",
        ticket_path.display()
    )
}

/// Build the full shell command to launch a *new* Claude Code session for a ticket.
/// Used when the pane has a bare shell (first use of the slot).
fn build_claude_command(ticket_dir: &Path, ticket_id: &str) -> String {
    format!(
        "claude --dangerously-skip-permissions \"{}\"\n",
        ticket_prompt(ticket_dir, ticket_id)
    )
}

/// Build the sequence to reuse an existing Claude Code session:
/// /clear to reset conversation, then send the new prompt.
fn build_reuse_command(ticket_dir: &Path, ticket_id: &str) -> String {
    format!(
        "/clear\n{}\n",
        ticket_prompt(ticket_dir, ticket_id)
    )
}

/// An agent pane slot — a pre-created terminal in the stacked layout.
struct AgentSlot {
    pane_id: u32,
    /// Which ticket is running in this slot (None = idle).
    ticket_id: Option<TicketId>,
    /// Whether this slot has had a Claude Code session started in it.
    has_session: bool,
}

/// Main plugin state
#[derive(Default)]
pub struct State {
    /// The computed dependency graph from ticket frontmatter.
    dag: Dag,

    /// Active threads indexed by ticket ID.
    threads: HashMap<TicketId, Thread>,

    /// Plugin configuration (ticket directory path, etc.)
    config: PluginConfig,

    /// Recent activity events for the dashboard display.
    activity_log: Vec<ActivityEvent>,

    /// Pre-created terminal pane slots for agent sessions.
    /// Populated on first PaneUpdate after permissions are granted.
    agent_slots: Vec<AgentSlot>,

    /// Snapshot of ticket phases from last DAG build, for change detection.
    last_phases: HashMap<TicketId, Phase>,

    /// Whether initial loading has completed.
    initialized: bool,

    /// Whether permissions have been granted.
    permissions_granted: bool,

    /// Whether agent slots have been discovered from PaneUpdate.
    slots_discovered: bool,
}

impl State {
    const MAX_ACTIVITY_LOG: usize = 100;

    fn log_activity(&mut self, event: ActivityEvent) {
        self.activity_log.push(event);
        if self.activity_log.len() > Self::MAX_ACTIVITY_LOG {
            self.activity_log.remove(0);
        }
    }

    /// Scan tickets directory and rebuild the DAG.
    /// Returns true if any ticket phases changed since last build.
    fn rebuild_dag(&mut self) -> bool {
        let tickets = match ticket::scan_tickets(&self.config.ticket_dir) {
            Ok(tickets) => tickets,
            Err(e) => {
                self.log_activity(ActivityEvent::Error {
                    message: format!("Failed to scan tickets: {}", e),
                });
                return false;
            }
        };

        let ticket_count = tickets.len();

        match Dag::from_tickets(tickets) {
            Ok(dag) => {
                // Detect phase changes
                let mut changed = false;
                for ticket in dag.tickets() {
                    if let Some(&old_phase) = self.last_phases.get(&ticket.id) {
                        if old_phase != ticket.phase {
                            self.log_activity(ActivityEvent::TicketPhaseChanged {
                                ticket_id: ticket.id.clone(),
                                old_phase,
                                new_phase: ticket.phase,
                            });
                            changed = true;
                        }
                    }
                }

                // Update phase snapshot
                self.last_phases = dag.tickets().map(|t| (t.id.clone(), t.phase)).collect();

                self.dag = dag;
                self.log_activity(ActivityEvent::DagRecomputed { ticket_count });
                changed
            }
            Err(e) => {
                self.log_activity(ActivityEvent::Error {
                    message: format!("Failed to build DAG: {:?}", e),
                });
                false
            }
        }
    }

    /// Discover agent pane slots from PaneUpdate.
    /// Agent slots are non-plugin panes that were pre-created in the layout.
    fn discover_slots(&mut self, pane_manifest: &PaneManifest) {
        if self.slots_discovered {
            return;
        }

        for panes in pane_manifest.panes.values() {
            for pane in panes {
                if !pane.is_plugin {
                    self.agent_slots.push(AgentSlot {
                        pane_id: pane.id,
                        ticket_id: None,
                        has_session: false,
                    });
                }
            }
        }

        if !self.agent_slots.is_empty() {
            self.slots_discovered = true;
            self.log_activity(ActivityEvent::Error {
                message: format!("Discovered {} agent pane slots", self.agent_slots.len()),
            });
        }
    }

    /// Find an idle agent slot.
    fn find_idle_slot(&self) -> Option<usize> {
        self.agent_slots
            .iter()
            .position(|s| s.ticket_id.is_none())
    }

    /// Mark a slot as idle when its ticket completes.
    fn release_slot_for_ticket(&mut self, ticket_id: &TicketId) {
        for slot in &mut self.agent_slots {
            if slot.ticket_id.as_ref() == Some(ticket_id) {
                slot.ticket_id = None;
                break;
            }
        }
    }

    /// Schedule ready tickets into idle agent slots.
    fn schedule_ready_tickets(&mut self) {
        if !self.permissions_granted || !self.slots_discovered {
            return;
        }

        let ready = self.dag.get_ready_tickets();

        for ticket_id in ready {
            // Skip tickets that already have a thread
            if self.threads.contains_key(&ticket_id) {
                continue;
            }

            // Find an idle slot
            let slot_idx = match self.find_idle_slot() {
                Some(idx) => idx,
                None => break, // No more slots
            };

            // Build the host-relative ticket dir (strip /host/ prefix)
            let host_ticket_dir = PathBuf::from(
                self.config
                    .ticket_dir
                    .to_string_lossy()
                    .strip_prefix("/host/")
                    .unwrap_or(&self.config.ticket_dir.to_string_lossy()),
            );

            // Write command to the slot's terminal.
            // First use: launch claude. Reuse: /clear + new prompt.
            let pane_id = self.agent_slots[slot_idx].pane_id;
            let cmd = if self.agent_slots[slot_idx].has_session {
                build_reuse_command(&host_ticket_dir, &ticket_id)
            } else {
                build_claude_command(&host_ticket_dir, &ticket_id)
            };
            write_chars_to_pane_id(&cmd, PaneId::Terminal(pane_id));

            // Mark slot as occupied and session started
            self.agent_slots[slot_idx].ticket_id = Some(ticket_id.clone());
            self.agent_slots[slot_idx].has_session = true;

            // Create thread record with the ticket's current phase
            let mut thread = Thread::new(ticket_id.clone(), pane_id);
            if let Some(ticket) = self.dag.get_ticket(&ticket_id) {
                thread.current_phase = ticket.phase;
            }
            self.threads.insert(ticket_id.clone(), thread);

            self.log_activity(ActivityEvent::ThreadSpawned {
                ticket_id,
                pane_id,
            });
        }
    }

    /// Scan active threads for new phase artifacts and advance ticket phases.
    ///
    /// For each running thread, checks if the artifact for the current phase
    /// exists in the work directory. If so, advances the ticket to the next
    /// phase by updating the YAML frontmatter and logs the appropriate events.
    /// If the new phase is Review, the thread is parked.
    fn check_artifact_advances(&mut self) {
        // Collect running threads to avoid borrow conflict
        let running: Vec<(TicketId, Phase)> = self
            .threads
            .iter()
            .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
            .map(|(tid, t)| (tid.clone(), t.current_phase))
            .collect();

        for (ticket_id, current_phase) in running {
            // Only phases with artifacts can be advanced
            let artifact_name = match current_phase.artifact_filename() {
                Some(name) => name,
                None => continue,
            };

            let artifact_path = self.config.work_dir.join(&ticket_id).join(artifact_name);
            if !artifact_path.exists() {
                continue;
            }

            // Compute next phase (always Some for phases with artifacts)
            let next_phase = match current_phase.next() {
                Some(p) => p,
                None => continue,
            };

            // Update the ticket file on disk
            let file_path = self
                .dag
                .get_ticket(&ticket_id)
                .map(|t| t.file_path.clone());
            let file_path = match file_path {
                Some(p) if !p.as_os_str().is_empty() => p,
                _ => continue,
            };

            if let Err(e) = ticket::update_ticket_phase(&file_path, next_phase) {
                self.log_activity(ActivityEvent::Error {
                    message: format!("Failed to advance {}: {}", ticket_id, e),
                });
                continue;
            }

            // Log events
            self.log_activity(ActivityEvent::PhaseCompleted {
                ticket_id: ticket_id.clone(),
                phase: current_phase,
            });
            self.log_activity(ActivityEvent::TicketPhaseChanged {
                ticket_id: ticket_id.clone(),
                old_phase: current_phase,
                new_phase: next_phase,
            });

            // Update thread phase
            if let Some(thread) = self.threads.get_mut(&ticket_id) {
                thread.current_phase = next_phase;

                // Park if advancing to Review
                if next_phase == Phase::Review {
                    thread.park();
                }
            }
        }
    }

    /// Timer-based completion detection.
    /// Rescans tickets, detects phase changes, marks completed threads,
    /// frees agent slots, and schedules new work.
    fn poll_tick(&mut self) {
        // Check for new artifacts and advance phases before rebuilding DAG
        self.check_artifact_advances();

        let changed = self.rebuild_dag();

        if changed {
            // Check for tickets that moved to Done — mark their threads complete
            let done_tickets: Vec<TicketId> = self
                .threads
                .iter()
                .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
                .filter(|(tid, _)| {
                    self.dag
                        .get_ticket(tid)
                        .map(|t| t.phase == Phase::Done)
                        .unwrap_or(false)
                })
                .map(|(tid, _)| tid.clone())
                .collect();

            for ticket_id in &done_tickets {
                if let Some(thread) = self.threads.get_mut(ticket_id) {
                    thread.complete();
                }
                self.release_slot_for_ticket(ticket_id);
                self.log_activity(ActivityEvent::ThreadExited {
                    ticket_id: ticket_id.clone(),
                    exit_code: Some(0),
                });
            }

            // Also detect tickets whose phase advanced (agent still running, update thread)
            for (tid, thread) in &mut self.threads {
                if thread.status == lisa_core::types::ThreadStatus::Running {
                    if let Some(ticket) = self.dag.get_ticket(tid) {
                        thread.current_phase = ticket.phase;
                    }
                }
            }
        }

        // Always try to schedule (slots may have freed up)
        self.schedule_ready_tickets();

        // Re-arm the timer
        set_timeout(POLL_INTERVAL_SECS);
    }
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // Parse configuration
        self.config = PluginConfig::from_config_map(&configuration);

        // Inside zellij's WASI sandbox, the host filesystem is mounted at /host.
        // Prefix relative config paths so std::fs can reach the project files.
        let host = PathBuf::from("/host");
        if !self.config.ticket_dir.is_absolute() {
            self.config.ticket_dir = host.join(&self.config.ticket_dir);
        }
        if !self.config.story_dir.is_absolute() {
            self.config.story_dir = host.join(&self.config.story_dir);
        }
        if !self.config.work_dir.is_absolute() {
            self.config.work_dir = host.join(&self.config.work_dir);
        }

        // Subscribe to the events we need
        subscribe(&[
            EventType::PaneUpdate,
            EventType::PermissionRequestResult,
            EventType::Timer,
        ]);

        // Request permissions needed to write commands to agent terminal panes
        request_permission(&[
            PermissionType::WriteToStdin,
            PermissionType::ChangeApplicationState,
            PermissionType::ReadApplicationState,
        ]);

        // Initial DAG build
        self.rebuild_dag();

        // Mark as initialized
        self.initialized = true;

        // Log startup
        self.log_activity(ActivityEvent::PluginStarted);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;

        match event {
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                self.permissions_granted = true;
                // Start the poll timer
                set_timeout(POLL_INTERVAL_SECS);
                // Try to schedule immediately if slots are already discovered
                self.schedule_ready_tickets();
                should_render = true;
            }

            Event::PermissionRequestResult(PermissionStatus::Denied) => {
                self.log_activity(ActivityEvent::Error {
                    message: "Permissions denied — cannot write to agent panes".to_string(),
                });
                should_render = true;
            }

            Event::PaneUpdate(pane_manifest) => {
                self.discover_slots(&pane_manifest);
                // Try scheduling in case slots just appeared
                if self.permissions_granted {
                    self.schedule_ready_tickets();
                }
                should_render = true;
            }

            Event::Timer(_elapsed) => {
                self.poll_tick();
                should_render = true;
            }

            _ => {}
        }

        should_render
    }

    fn render(&mut self, rows: usize, cols: usize) {
        if !self.initialized {
            println!("Lisa/Ralph initializing...");
            return;
        }

        let ui_state = self.to_ui_state();
        ui::print_dashboard(&ui_state, rows, cols);
    }
}

impl State {
    /// Convert internal plugin state to UI-compatible state for rendering
    fn to_ui_state(&self) -> ui::PluginState {
        use std::time::Duration;

        let tickets: Vec<ui::TicketNode> = self
            .dag
            .tickets()
            .map(|t| ui::TicketNode {
                id: t.id.clone(),
                title: t.title.clone(),
                phase: phase_to_ui_phase(t.phase),
                status: ticket_status_to_ui_status(&t.status, t.phase),
                depends_on: t.depends_on.iter().cloned().collect(),
                blocks: t.blocks.iter().cloned().collect(),
            })
            .collect();

        let active_threads: Vec<ui::ActiveThread> = self
            .threads
            .values()
            .filter(|t| t.status == lisa_core::types::ThreadStatus::Running)
            .map(|t| ui::ActiveThread {
                ticket_id: t.ticket_id.clone(),
                phase: phase_to_ui_phase(t.current_phase),
                started_at: Duration::from_secs(
                    t.started_at
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                ),
                pane_id: t.pane_id,
            })
            .collect();

        let parked_threads: Vec<ui::ParkedThread> = self
            .threads
            .values()
            .filter(|t| t.status == lisa_core::types::ThreadStatus::Parked)
            .map(|t| ui::ParkedThread {
                ticket_id: t.ticket_id.clone(),
                phase: phase_to_ui_phase(t.current_phase),
                artifact_path: format!(
                    "{}/{}/{}",
                    self.config.work_dir.display(),
                    t.ticket_id,
                    t.current_phase.artifact_filename().unwrap_or("artifact.md")
                ),
                parked_at: Duration::from_secs(
                    t.started_at
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                ),
                pane_id: t.pane_id,
            })
            .collect();

        let activity_log: Vec<ui::ActivityEntry> = self
            .activity_log
            .iter()
            .filter_map(|e| activity_event_to_ui_entry(e))
            .collect();

        ui::PluginState {
            tickets,
            active_threads,
            parked_threads,
            activity_log,
            current_time: Duration::from_secs(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
            selected_ticket: None,
        }
    }
}

/// Convert internal Phase to UI Phase
fn phase_to_ui_phase(phase: Phase) -> ui::Phase {
    match phase {
        Phase::Ready => ui::Phase::Ready,
        Phase::Research => ui::Phase::Research,
        Phase::Design => ui::Phase::Design,
        Phase::Structure => ui::Phase::Structure,
        Phase::Plan => ui::Phase::Plan,
        Phase::Implement => ui::Phase::Implement,
        Phase::Review => ui::Phase::Review,
        Phase::Done => ui::Phase::Done,
    }
}

/// Convert internal ticket status to UI ticket status
fn ticket_status_to_ui_status(
    status: &lisa_core::types::TicketStatus,
    phase: Phase,
) -> ui::TicketStatus {
    match status {
        lisa_core::types::TicketStatus::Open => {
            if phase == Phase::Ready {
                ui::TicketStatus::Ready
            } else {
                ui::TicketStatus::InProgress
            }
        }
        lisa_core::types::TicketStatus::InProgress => ui::TicketStatus::InProgress,
        lisa_core::types::TicketStatus::Blocked => ui::TicketStatus::Blocked,
        lisa_core::types::TicketStatus::Review => ui::TicketStatus::WaitingReview,
        lisa_core::types::TicketStatus::Done => ui::TicketStatus::Done,
        lisa_core::types::TicketStatus::Cancelled => ui::TicketStatus::Done,
    }
}

/// Convert internal activity event to UI activity entry
fn activity_event_to_ui_entry(event: &ActivityEvent) -> Option<ui::ActivityEntry> {
    use std::time::Duration;

    let timestamp = Duration::ZERO;

    let activity = match event {
        ActivityEvent::PluginStarted => return None,
        ActivityEvent::ThreadSpawned { ticket_id, .. } => ui::ActivityType::ThreadStarted {
            ticket_id: ticket_id.clone(),
            phase: ui::Phase::Ready,
        },
        ActivityEvent::ThreadExited { ticket_id, .. } => ui::ActivityType::PhaseCompleted {
            ticket_id: ticket_id.clone(),
            phase: ui::Phase::Done,
        },
        ActivityEvent::PhaseCompleted { ticket_id, phase } => ui::ActivityType::PhaseCompleted {
            ticket_id: ticket_id.clone(),
            phase: phase_to_ui_phase(*phase),
        },
        ActivityEvent::TicketPhaseChanged {
            ticket_id,
            new_phase,
            ..
        } => ui::ActivityType::PhaseCompleted {
            ticket_id: ticket_id.clone(),
            phase: phase_to_ui_phase(*new_phase),
        },
        ActivityEvent::TicketStatusChanged { .. } => return None,
        ActivityEvent::ArtifactCreated {
            ticket_id, path, ..
        } => ui::ActivityType::Commit {
            ticket_id: ticket_id.clone(),
            message: format!("Created {}", path.display()),
        },
        ActivityEvent::CommitMade {
            ticket_id,
            commit_hash,
        } => ui::ActivityType::Commit {
            ticket_id: ticket_id.clone(),
            message: format!("Commit {}", commit_hash),
        },
        ActivityEvent::DagRecomputed { .. } => return None,
        ActivityEvent::Error { message } => ui::ActivityType::Error {
            ticket_id: String::new(),
            message: message.clone(),
        },
    };

    Some(ui::ActivityEntry {
        timestamp,
        activity,
    })
}

// Register the plugin with Zellij
register_plugin!(State);

#[cfg(test)]
mod tests {
    use super::*;
    use lisa_core::types::{ActivityEvent, Phase, TicketStatus};

    #[test]
    fn test_phase_to_ui_phase() {
        assert_eq!(phase_to_ui_phase(Phase::Ready), ui::Phase::Ready);
        assert_eq!(phase_to_ui_phase(Phase::Research), ui::Phase::Research);
        assert_eq!(phase_to_ui_phase(Phase::Design), ui::Phase::Design);
        assert_eq!(phase_to_ui_phase(Phase::Structure), ui::Phase::Structure);
        assert_eq!(phase_to_ui_phase(Phase::Plan), ui::Phase::Plan);
        assert_eq!(phase_to_ui_phase(Phase::Implement), ui::Phase::Implement);
        assert_eq!(phase_to_ui_phase(Phase::Review), ui::Phase::Review);
        assert_eq!(phase_to_ui_phase(Phase::Done), ui::Phase::Done);
    }

    #[test]
    fn test_ticket_status_to_ui_status() {
        assert_eq!(
            ticket_status_to_ui_status(&TicketStatus::Open, Phase::Ready),
            ui::TicketStatus::Ready
        );
        assert_eq!(
            ticket_status_to_ui_status(&TicketStatus::Open, Phase::Research),
            ui::TicketStatus::InProgress
        );
        assert_eq!(
            ticket_status_to_ui_status(&TicketStatus::InProgress, Phase::Implement),
            ui::TicketStatus::InProgress
        );
        assert_eq!(
            ticket_status_to_ui_status(&TicketStatus::Blocked, Phase::Ready),
            ui::TicketStatus::Blocked
        );
        assert_eq!(
            ticket_status_to_ui_status(&TicketStatus::Review, Phase::Review),
            ui::TicketStatus::WaitingReview
        );
        assert_eq!(
            ticket_status_to_ui_status(&TicketStatus::Done, Phase::Done),
            ui::TicketStatus::Done
        );
        assert_eq!(
            ticket_status_to_ui_status(&TicketStatus::Cancelled, Phase::Done),
            ui::TicketStatus::Done
        );
    }

    #[test]
    fn test_activity_event_to_ui_entry() {
        assert!(activity_event_to_ui_entry(&ActivityEvent::PluginStarted).is_none());
        assert!(
            activity_event_to_ui_entry(&ActivityEvent::DagRecomputed { ticket_count: 5 }).is_none()
        );
        assert!(activity_event_to_ui_entry(&ActivityEvent::TicketStatusChanged {
            ticket_id: "T-001".to_string(),
            old_status: TicketStatus::Open,
            new_status: TicketStatus::InProgress,
        })
        .is_none());

        let entry = activity_event_to_ui_entry(&ActivityEvent::ThreadSpawned {
            ticket_id: "T-001".to_string(),
            pane_id: 42,
        });
        assert!(entry.is_some());
        match &entry.unwrap().activity {
            ui::ActivityType::ThreadStarted { ticket_id, .. } => {
                assert_eq!(ticket_id, "T-001");
            }
            other => panic!("Expected ThreadStarted, got {:?}", other),
        }

        let entry = activity_event_to_ui_entry(&ActivityEvent::PhaseCompleted {
            ticket_id: "T-002".to_string(),
            phase: Phase::Design,
        });
        assert!(entry.is_some());
        match &entry.unwrap().activity {
            ui::ActivityType::PhaseCompleted { ticket_id, phase } => {
                assert_eq!(ticket_id, "T-002");
                assert_eq!(*phase, ui::Phase::Design);
            }
            other => panic!("Expected PhaseCompleted, got {:?}", other),
        }

        let entry = activity_event_to_ui_entry(&ActivityEvent::Error {
            message: "something broke".to_string(),
        });
        assert!(entry.is_some());
        match &entry.unwrap().activity {
            ui::ActivityType::Error { message, .. } => {
                assert_eq!(message, "something broke");
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_build_claude_command() {
        let ticket_dir = Path::new("docs/active/tickets");
        let cmd = build_claude_command(ticket_dir, "T-042-01");

        assert!(cmd.contains("claude --dangerously-skip-permissions"));
        assert!(cmd.contains("docs/active/tickets/T-042-01.md"));
        assert!(cmd.contains("docs/knowledge/rdspi-workflow.md"));
        assert!(cmd.contains("CLAUDE.md"));
        assert!(cmd.ends_with('\n'));
    }

    #[test]
    fn test_check_artifact_advances_research_to_design() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // Create ticket file in tickets dir
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n\nBody\n",
        ).unwrap();

        // Create work dir with research.md artifact
        let work_dir = dir.path().join("work");
        fs::create_dir_all(work_dir.join("T-001")).unwrap();
        fs::write(work_dir.join("T-001/research.md"), "# Research done").unwrap();

        // Build state with DAG and a running thread
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Add a running thread for T-001 in research phase
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-001".to_string(), thread);

        // Run artifact advance check
        state.check_artifact_advances();

        // Verify thread phase advanced to Design
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Design);
        assert_eq!(thread.status, ThreadStatus::Running);

        // Verify activity log has PhaseCompleted and TicketPhaseChanged
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::PhaseCompleted { ticket_id, phase }
            if ticket_id == "T-001" && *phase == Phase::Research
        )));
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::TicketPhaseChanged { ticket_id, old_phase, new_phase }
            if ticket_id == "T-001" && *old_phase == Phase::Research && *new_phase == Phase::Design
        )));

        // Verify ticket file was updated
        let updated = fs::read_to_string(state.config.ticket_dir.join("T-001.md")).unwrap();
        assert!(updated.contains("phase: design"));
    }

    #[test]
    fn test_check_artifact_advances_implement_to_review_parks_thread() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // Create ticket file
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: impl-test\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
        ).unwrap();

        // Create work dir with progress.md artifact
        let work_dir = dir.path().join("work");
        fs::create_dir_all(work_dir.join("T-002")).unwrap();
        fs::write(work_dir.join("T-002/progress.md"), "# Progress done").unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        let mut thread = Thread::new("T-002", 2);
        thread.current_phase = Phase::Implement;
        state.threads.insert("T-002".to_string(), thread);

        state.check_artifact_advances();

        // Verify thread advanced to Review and is parked
        let thread = state.threads.get("T-002").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Parked);

        // Verify ticket file was updated
        let updated = fs::read_to_string(state.config.ticket_dir.join("T-002.md")).unwrap();
        assert!(updated.contains("phase: review"));
    }

    #[test]
    fn test_check_artifact_advances_no_artifact_no_change() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // Create ticket file
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-003.md"),
            "---\nid: T-003\ntitle: no-artifact\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n\nBody\n",
        ).unwrap();

        // Work dir exists but NO artifact
        let work_dir = dir.path().join("work");
        fs::create_dir_all(work_dir.join("T-003")).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        let mut thread = Thread::new("T-003", 3);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-003".to_string(), thread);

        state.check_artifact_advances();

        // Thread should remain unchanged
        let thread = state.threads.get("T-003").unwrap();
        assert_eq!(thread.current_phase, Phase::Research);
        assert_eq!(thread.status, ThreadStatus::Running);
        assert!(state.activity_log.is_empty());
    }
}

// wasm32-wasip1 + cdylib produces a reactor module (no entry point).
// Zellij expects a command-style _start export to initialize the WASM instance.
extern "C" {
    fn __wasm_call_ctors();
}

#[no_mangle]
pub extern "C" fn _start() {
    unsafe {
        __wasm_call_ctors();
    }
}
