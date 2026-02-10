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

use zellij_tile::prelude::*;

use crate::dag::Dag;
use crate::types::{ActivityEvent, PluginConfig, Thread, TicketId};

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

    /// Scan tickets directory and rebuild the DAG
    fn rebuild_dag(&mut self) {
        // TODO: Implement ticket scanning and DAG construction
        // 1. Read all .md files from config.ticket_dir
        // 2. Parse frontmatter to extract dependencies
        // 3. Build DAG from depends_on/blocks relationships
        self.dag = Dag::default();
    }

    /// Check which tickets are ready to be scheduled
    fn schedule_ready_tickets(&mut self) {
        // TODO: Implement scheduling logic
        // 1. Query DAG for tickets with satisfied dependencies
        // 2. Filter out tickets that already have active threads
        // 3. Spawn new threads for ready tickets
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

    /// Handle filesystem changes (artifact creation, ticket phase changes)
    fn handle_filesystem_update(&mut self, paths: &[std::path::PathBuf]) {
        // TODO: Implement filesystem change handling
        // 1. Check if any paths are in the ticket directory -> rebuild DAG
        // 2. Check if any paths are artifacts -> update thread state
        // 3. Check if ticket phase changed -> log activity, maybe schedule more work
        for _path in paths {
            // Placeholder: will parse paths and update state accordingly
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
