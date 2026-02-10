//! Lisa/Ralph - A Zellij plugin for DAG-driven concurrent task scheduling
//!
//! This plugin implements the RDSPI workflow (Research -> Design -> Structure -> Plan -> Implement)
//! as a DAG-driven concurrent scheduler. It manages Claude Code sessions for each ticket,
//! tracks phase progress, and provides a live dashboard.

mod dag;
mod scheduler;
mod ticket;
mod types;
mod ui;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use zellij_tile::prelude::*;

use crate::dag::Dag;
use crate::types::{ActivityEvent, Phase, PluginConfig, Thread, TicketId};

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
    ///
    /// Reads all `.md` ticket files from the configured ticket directory,
    /// parses their YAML frontmatter, and constructs a dependency graph.
    /// Errors during scanning or DAG construction are logged as activity
    /// events rather than propagated, so the plugin remains operational
    /// even when ticket files are malformed.
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
    ///
    /// Queries the DAG for tickets whose dependencies are all satisfied,
    /// filters out tickets that already have active threads, and spawns
    /// floating command panes running `claude --dangerously-skip-permissions`
    /// for each ready ticket (up to the configured `max_threads` limit).
    ///
    /// The spawned pane's ID is not known until a `PaneUpdate` event arrives,
    /// so newly spawned threads are tracked with `pane_id: 0` as a placeholder.
    /// The context BTreeMap passed to `open_command_pane_floating` carries the
    /// `ticket_id` so that `CommandPaneExited` events can be correlated.
    fn schedule_ready_tickets(&mut self) {
        let ready = self.dag.get_ready_tickets();

        // How many more threads can we spawn?
        let active_count = self
            .threads
            .values()
            .filter(|t| t.status == types::ThreadStatus::Running)
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
                            "Read the ticket at {} and follow the RDSPI workflow defined in CLAUDE.md. \
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
    ///
    /// Inspects changed paths to detect:
    /// - Ticket file changes (inside `ticket_dir`) -- triggers a DAG rebuild
    ///   on the next update cycle (the caller in `update()` already calls
    ///   `rebuild_dag` after this method).
    /// - Artifact creation (inside `work_dir`) -- logs `ArtifactCreated` events
    ///   and detects phase transitions by matching artifact filenames to phases.
    fn handle_filesystem_update(&mut self, paths: &[PathBuf]) {
        // Snapshot ticket phases before the DAG is rebuilt so we can detect
        // phase changes after the rebuild (which happens in the caller).
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

        // After the caller rebuilds the DAG, detect phase changes.
        // We store old_phases so the caller can compare after rebuild_dag().
        // Since the caller calls rebuild_dag() right after us, we stash the
        // old phases and check in a post-rebuild step. However, the current
        // call order in update() is: handle_filesystem_update -> rebuild_dag.
        // So we detect phase changes by comparing old_phases against the
        // *current* DAG (which will be the old DAG, before rebuild).
        // The phase change detection therefore happens on the *next* filesystem
        // update cycle. To detect it on this cycle, we do the rebuild here
        // for comparison purposes and let the caller's rebuild be a no-op
        // (idempotent operation).
        //
        // Practical approach: check if any ticket dir paths changed, and if so,
        // do an early rebuild and compare.
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
    /// Called once when the plugin is loaded.
    ///
    /// Initializes state, subscribes to relevant events, and performs initial
    /// ticket scan to build the DAG.
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

        // Request filesystem watching for ticket and work directories
        // TODO: Use watch_filesystem when we have the paths configured

        // Initial DAG build
        self.rebuild_dag();

        // Mark as initialized
        self.initialized = true;

        // Log startup
        self.log_activity(ActivityEvent::PluginStarted);
    }

    /// Called when subscribed events occur.
    ///
    /// Handles:
    /// - FileSystemUpdate: detect artifact creation or ticket phase changes
    /// - CommandPaneExited: when a Claude session ends
    /// - PaneUpdate: track which panes are alive
    /// - Key/Mouse: dashboard interaction
    /// - Timer: periodic state refresh
    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;

        match event {
            Event::FileSystemUpdate(paths_with_metadata) => {
                // Extract just the paths from the (PathBuf, Option<FileMetadata>) tuples
                let paths: Vec<std::path::PathBuf> = paths_with_metadata
                    .iter()
                    .map(|(path, _metadata)| path.clone())
                    .collect();
                self.handle_filesystem_update(&paths);
                // Rebuild DAG if tickets changed
                self.rebuild_dag();
                // Check if new tickets are ready to schedule
                self.schedule_ready_tickets();
                should_render = true;
            }

            Event::CommandPaneExited(pane_id, exit_code, _context) => {
                // Handle command pane exit
                self.handle_pane_exited(pane_id, exit_code);
                // A thread finished, maybe others can now run
                self.schedule_ready_tickets();
                should_render = true;
            }

            Event::PaneUpdate(pane_manifest) => {
                // Track pane lifecycle
                // TODO: Update internal pane tracking based on manifest
                // This helps us know which threads are still alive
                let _ = pane_manifest; // Placeholder
                should_render = true;
            }

            Event::Key(key) => {
                // Handle keyboard navigation in the dashboard
                // TODO: Implement key handling for navigation, focusing panes, etc.
                let _ = key; // Placeholder
                should_render = true;
            }

            Event::Mouse(mouse_event) => {
                // Handle mouse clicks in the dashboard
                // TODO: Implement click handling for quick-jump to thread panes
                let _ = mouse_event; // Placeholder
                should_render = true;
            }

            Event::Timer(_elapsed) => {
                // Periodic refresh - useful for updating duration displays
                should_render = true;
            }

            _ => {
                // Ignore other events
            }
        }

        should_render
    }

    /// Called to render the plugin UI.
    ///
    /// Renders the dashboard showing:
    /// - DAG visualization with dependency edges and status
    /// - Active threads: ticket, phase, duration
    /// - Parked threads: waiting for review, artifact path
    /// - Recent activity log
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

        // Convert DAG tickets to UI ticket nodes
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

        // Convert active threads
        let active_threads: Vec<ui::ActiveThread> = self
            .threads
            .values()
            .filter(|t| t.status == types::ThreadStatus::Running)
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

        // Convert parked threads
        let parked_threads: Vec<ui::ParkedThread> = self
            .threads
            .values()
            .filter(|t| t.status == types::ThreadStatus::Parked)
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

        // Convert activity log
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
fn phase_to_ui_phase(phase: types::Phase) -> ui::Phase {
    match phase {
        types::Phase::Ready => ui::Phase::Ready,
        types::Phase::Research => ui::Phase::Research,
        types::Phase::Design => ui::Phase::Design,
        types::Phase::Structure => ui::Phase::Structure,
        types::Phase::Plan => ui::Phase::Plan,
        types::Phase::Implement => ui::Phase::Implement,
        types::Phase::Review => ui::Phase::Review,
        types::Phase::Done => ui::Phase::Done,
    }
}

/// Convert internal ticket status to UI ticket status
fn ticket_status_to_ui_status(
    status: &types::TicketStatus,
    phase: types::Phase,
) -> ui::TicketStatus {
    match status {
        types::TicketStatus::Open => {
            if phase == types::Phase::Ready {
                ui::TicketStatus::Ready
            } else {
                ui::TicketStatus::InProgress
            }
        }
        types::TicketStatus::InProgress => ui::TicketStatus::InProgress,
        types::TicketStatus::Blocked => ui::TicketStatus::Blocked,
        types::TicketStatus::Review => ui::TicketStatus::WaitingReview,
        types::TicketStatus::Done => ui::TicketStatus::Done,
        types::TicketStatus::Cancelled => ui::TicketStatus::Done,
    }
}

/// Convert internal activity event to UI activity entry
fn activity_event_to_ui_entry(event: &ActivityEvent) -> Option<ui::ActivityEntry> {
    use std::time::Duration;

    // For now, use zero duration - in a real implementation we'd track timestamps
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
