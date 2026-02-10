//! Lisa/Ralph - A Zellij plugin for DAG-driven concurrent task scheduling
//!
//! This plugin implements the RDSPI workflow (Research -> Design -> Structure -> Plan -> Implement)
//! as a DAG-driven concurrent scheduler. It manages Claude Code sessions for each ticket,
//! tracks phase progress, and provides a live dashboard.

mod scheduler;
mod ui;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use zellij_tile::prelude::*;

use lisa_core::dag::Dag;
use lisa_core::ticket;
use lisa_core::types::{ActivityEvent, Phase, PluginConfig, Thread, TicketId};

/// Main plugin state
///
/// This struct holds all the runtime state for the Lisa/Ralph plugin:
/// - The computed DAG from ticket frontmatter
/// - Active threads (one per ticket being worked on)
/// - Plugin configuration
/// - Activity log for the dashboard
#[derive(Default)]
pub struct State {
    /// The computed dependency graph from ticket frontmatter.
    /// Recomputed when tickets change.
    dag: Dag,

    /// Active threads indexed by ticket ID.
    /// Each thread represents a Claude Code session working on a ticket.
    threads: HashMap<TicketId, Thread>,

    /// Plugin configuration (ticket directory path, etc.)
    config: PluginConfig,

    /// Recent activity events for the dashboard display.
    /// Kept bounded to avoid unbounded memory growth.
    activity_log: Vec<ActivityEvent>,

    /// Tracks which pane IDs correspond to which threads.
    /// Used to correlate PaneUpdate and CommandPaneExited events.
    pane_to_ticket: HashMap<u32, TicketId>,

    /// Whether initial loading has completed.
    initialized: bool,
}

impl State {
    /// Maximum number of activity log entries to retain
    const MAX_ACTIVITY_LOG: usize = 100;

    /// Add an event to the activity log, maintaining size bounds
    fn log_activity(&mut self, event: ActivityEvent) {
        self.activity_log.push(event);
        if self.activity_log.len() > Self::MAX_ACTIVITY_LOG {
            self.activity_log.remove(0);
        }
    }

    /// Scan tickets directory and rebuild the DAG.
    fn rebuild_dag(&mut self) {
        let tickets = match ticket::scan_tickets(&self.config.ticket_dir) {
            Ok(tickets) => tickets,
            Err(e) => {
                self.log_activity(ActivityEvent::Error {
                    message: format!("Failed to scan tickets: {}", e),
                });
                return;
            }
        };

        let ticket_count = tickets.len();

        match Dag::from_tickets(tickets) {
            Ok(dag) => {
                self.dag = dag;
                self.log_activity(ActivityEvent::DagRecomputed { ticket_count });
            }
            Err(e) => {
                self.log_activity(ActivityEvent::Error {
                    message: format!("Failed to build DAG: {:?}", e),
                });
            }
        }
    }

    /// Check which tickets are ready to be scheduled and spawn Claude sessions.
    fn schedule_ready_tickets(&mut self) {
        let ready = self.dag.get_ready_tickets();

        // How many more threads can we spawn?
        let active_count = self
            .threads
            .values()
            .filter(|t| t.status == lisa_core::types::ThreadStatus::Running)
            .count();
        let slots = self.config.max_threads.saturating_sub(active_count);

        if slots == 0 {
            return;
        }

        let mut spawned = 0;
        for ticket_id in ready {
            if spawned >= slots {
                break;
            }

            // Skip tickets that already have a thread (running, parked, etc.)
            if self.threads.contains_key(&ticket_id) {
                continue;
            }

            // Build the ticket file path for the prompt
            let ticket_path = self.config.ticket_dir.join(format!("{}.md", &ticket_id));

            // Context for correlating pane events back to this ticket
            let context = BTreeMap::from([("ticket_id".to_string(), ticket_id.clone())]);

            // Spawn a floating command pane running claude
            open_command_pane_floating(
                CommandToRun {
                    path: PathBuf::from("claude"),
                    args: vec![
                        "--dangerously-skip-permissions".to_string(),
                        "--print".to_string(),
                        format!(
                            "Read the ticket at {}, the project context in CLAUDE.md, and the RDSPI workflow in docs/rdspi-workflow.md. \
                             Start from the current phase indicated in the ticket frontmatter.",
                            ticket_path.display()
                        ),
                    ],
                    cwd: None,
                },
                Some(FloatingPaneCoordinates::default()),
                context,
            );

            // Create a thread record (pane_id will be updated via PaneUpdate)
            let thread = Thread::new(ticket_id.clone(), 0);
            self.threads.insert(ticket_id.clone(), thread);

            self.log_activity(ActivityEvent::ThreadSpawned {
                ticket_id,
                pane_id: 0,
            });

            spawned += 1;
        }
    }

    /// Handle a pane exiting (Claude session ended)
    fn handle_pane_exited(&mut self, pane_id: u32, exit_code: Option<i32>) {
        if let Some(ticket_id) = self.pane_to_ticket.remove(&pane_id) {
            if let Some(thread) = self.threads.get_mut(&ticket_id) {
                // Mark thread as completed or failed based on exit code
                match exit_code {
                    Some(0) | None => thread.complete(),
                    Some(_) => thread.fail(),
                }
                self.log_activity(ActivityEvent::ThreadExited {
                    ticket_id: ticket_id.clone(),
                    exit_code,
                });
            }
        }
    }

    /// Handle filesystem changes (artifact creation, ticket phase changes).
    fn handle_filesystem_update(&mut self, paths: &[PathBuf]) {
        // Snapshot ticket phases before the DAG is rebuilt
        let old_phases: HashMap<TicketId, Phase> = self
            .dag
            .tickets()
            .map(|t| (t.id.clone(), t.phase))
            .collect();

        for path in paths {
            // Check for artifact creation inside the work directory
            if let Ok(rel) = path.strip_prefix(&self.config.work_dir) {
                // work_dir layout: {ticket_id}/{artifact_filename}
                let components: Vec<_> = rel.components().collect();
                if components.len() >= 2 {
                    let ticket_id = components[0].as_os_str().to_string_lossy().to_string();
                    let filename = components[1].as_os_str().to_string_lossy().to_string();

                    // Check if the filename corresponds to a known phase artifact
                    let artifact_phase = Phase::all().iter().find(|p| {
                        p.artifact_filename()
                            .map(|f| f == filename)
                            .unwrap_or(false)
                    });

                    if let Some(&phase) = artifact_phase {
                        self.log_activity(ActivityEvent::ArtifactCreated {
                            ticket_id: ticket_id.clone(),
                            phase,
                            path: path.clone(),
                        });
                    }
                }
            }
        }

        let ticket_dir_changed = paths
            .iter()
            .any(|p| p.starts_with(&self.config.ticket_dir));

        if ticket_dir_changed {
            // Rebuild the DAG now so we can compare phases
            let tickets = match ticket::scan_tickets(&self.config.ticket_dir) {
                Ok(t) => t,
                Err(_) => return,
            };
            let ticket_count = tickets.len();
            match Dag::from_tickets(tickets) {
                Ok(new_dag) => {
                    // Compare phases between old and new
                    for ticket in new_dag.tickets() {
                        if let Some(&old_phase) = old_phases.get(&ticket.id) {
                            if old_phase != ticket.phase {
                                self.log_activity(ActivityEvent::TicketPhaseChanged {
                                    ticket_id: ticket.id.clone(),
                                    old_phase,
                                    new_phase: ticket.phase,
                                });
                            }
                        }
                    }
                    self.dag = new_dag;
                    self.log_activity(ActivityEvent::DagRecomputed { ticket_count });
                }
                Err(e) => {
                    self.log_activity(ActivityEvent::Error {
                        message: format!("Failed to build DAG: {:?}", e),
                    });
                }
            }
        }
    }
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // Parse configuration
        self.config = PluginConfig::from_config_map(&configuration);

        // Subscribe to the events we need
        subscribe(&[
            EventType::PaneUpdate,
            EventType::FileSystemUpdate,
            EventType::CommandPaneExited,
            EventType::Key,
            EventType::Mouse,
            EventType::Timer,
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
            Event::FileSystemUpdate(paths_with_metadata) => {
                let paths: Vec<std::path::PathBuf> = paths_with_metadata
                    .iter()
                    .map(|(path, _metadata)| path.clone())
                    .collect();
                self.handle_filesystem_update(&paths);
                self.rebuild_dag();
                self.schedule_ready_tickets();
                should_render = true;
            }

            Event::CommandPaneExited(pane_id, exit_code, _context) => {
                self.handle_pane_exited(pane_id, exit_code);
                self.schedule_ready_tickets();
                should_render = true;
            }

            Event::PaneUpdate(pane_manifest) => {
                let _ = pane_manifest;
                should_render = true;
            }

            Event::Key(key) => {
                let _ = key;
                should_render = true;
            }

            Event::Mouse(mouse_event) => {
                let _ = mouse_event;
                should_render = true;
            }

            Event::Timer(_elapsed) => {
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

        // Convert internal state to UI-compatible state for rendering
        let ui_state = self.to_ui_state();

        // Delegate to the UI module for rendering
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
