//! Lisa/Ralph - A Zellij plugin for DAG-driven concurrent task scheduling
//!
//! This plugin implements the RDSPI workflow (Research -> Design -> Structure -> Plan -> Implement)
//! as a DAG-driven concurrent scheduler. It manages Claude Code sessions for each ticket,
//! tracks phase progress, and provides a live dashboard.

mod scheduler;
mod ui;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use zellij_tile::prelude::*;

use lisa_core::dag::Dag;
use lisa_core::ticket;
use lisa_core::types::{ActivityEvent, Phase, PluginConfig, Thread, TicketId};

/// Build the full shell command to launch a Claude session for a ticket.
///
/// The ticket_dir should be the host-relative path (no /host/ prefix) since
/// the terminal runs on the host, not inside WASI.
fn build_claude_command(ticket_dir: &Path, ticket_id: &str) -> String {
    let ticket_path = ticket_dir.join(format!("{}.md", ticket_id));
    format!(
        "claude --dangerously-skip-permissions \
         \"Read the ticket at {}, the project context in CLAUDE.md, and the RDSPI workflow in docs/knowledge/rdspi-workflow.md. \
         Start from the current phase indicated in the ticket frontmatter.\"\n",
        ticket_path.display()
    )
}

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

    /// Ticket IDs waiting for a terminal pane to appear.
    /// We open a terminal, then write the claude command to it once we see it in PaneUpdate.
    pending_spawns: Vec<TicketId>,

    /// Set of known terminal pane IDs (to detect newly opened ones).
    known_pane_ids: HashSet<u32>,

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

            // Skip tickets already pending spawn
            if self.pending_spawns.contains(&ticket_id) {
                continue;
            }

            // Open a terminal pane near the plugin (same tab, joins the stacked layout).
            // The claude command will be written to stdin once we detect the pane in PaneUpdate.
            open_terminal_near_plugin(PathBuf::from("/host"));

            self.pending_spawns.push(ticket_id.clone());

            self.log_activity(ActivityEvent::ThreadSpawned {
                ticket_id,
                pane_id: 0,
            });

            spawned += 1;
        }
    }

    /// Called on PaneUpdate to detect newly opened terminals and write the claude command.
    fn handle_pane_update(&mut self, pane_manifest: &PaneManifest) {
        // Collect all current terminal pane IDs across all tabs
        let mut current_terminal_ids: HashSet<u32> = HashSet::new();
        for panes in pane_manifest.panes.values() {
            for pane in panes {
                if !pane.is_plugin {
                    current_terminal_ids.insert(pane.id);
                }
            }
        }

        // Find new pane IDs that weren't in our known set
        let new_ids: Vec<u32> = current_terminal_ids
            .difference(&self.known_pane_ids)
            .copied()
            .collect();

        // For each new pane, if we have a pending spawn, wire it up
        for new_pane_id in new_ids {
            if let Some(ticket_id) = self.pending_spawns.pop() {
                // Build the host-relative ticket dir (strip /host/ prefix)
                let host_ticket_dir = PathBuf::from(
                    self.config
                        .ticket_dir
                        .to_string_lossy()
                        .strip_prefix("/host/")
                        .unwrap_or(&self.config.ticket_dir.to_string_lossy()),
                );

                // Write the claude command to the new terminal's stdin
                let cmd = build_claude_command(&host_ticket_dir, &ticket_id);
                write_chars_to_pane_id(&cmd, PaneId::Terminal(new_pane_id));

                // Create thread record with the ticket's current phase
                let mut thread = Thread::new(ticket_id.clone(), new_pane_id);
                if let Some(ticket) = self.dag.get_ticket(&ticket_id) {
                    thread.current_phase = ticket.phase;
                }
                self.threads.insert(ticket_id.clone(), thread);
                self.pane_to_ticket.insert(new_pane_id, ticket_id);
            }
        }

        // Update known set
        self.known_pane_ids = current_terminal_ids;
    }

    /// Handle a pane exiting (Claude session ended).
    ///
    /// Uses the context BTreeMap echoed back from `open_command_pane` to identify
    /// which ticket the pane belongs to. Falls back to the `pane_to_ticket` map.
    fn handle_pane_exited(
        &mut self,
        pane_id: u32,
        exit_code: Option<i32>,
        context: BTreeMap<String, String>,
    ) {
        // Prefer context-based lookup (set at spawn time), fall back to pane_to_ticket
        let ticket_id = context
            .get("ticket_id")
            .cloned()
            .or_else(|| self.pane_to_ticket.remove(&pane_id));

        if let Some(ticket_id) = ticket_id {
            if let Some(thread) = self.threads.get_mut(&ticket_id) {
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
            EventType::FileSystemUpdate,
            EventType::CommandPaneExited,
            EventType::PermissionRequestResult,
            EventType::Key,
            EventType::Mouse,
            EventType::Timer,
        ]);

        // Request permissions needed to spawn Claude sessions and write to their stdin
        request_permission(&[
            PermissionType::OpenTerminalsOrPlugins,
            PermissionType::WriteToStdin,
            PermissionType::ChangeApplicationState,
            PermissionType::ReadApplicationState,
        ]);

        // Initial DAG build (scheduling deferred until permissions are granted)
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

            Event::CommandPaneExited(pane_id, exit_code, context) => {
                self.handle_pane_exited(pane_id, exit_code, context);
                self.schedule_ready_tickets();
                should_render = true;
            }

            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                // Permissions granted — now we can spawn Claude sessions
                self.schedule_ready_tickets();
                should_render = true;
            }

            Event::PermissionRequestResult(PermissionStatus::Denied) => {
                self.log_activity(ActivityEvent::Error {
                    message: "Permissions denied — cannot spawn Claude sessions".to_string(),
                });
                should_render = true;
            }

            Event::PaneUpdate(pane_manifest) => {
                self.handle_pane_update(&pane_manifest);
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
        // Open + Ready = Ready
        assert_eq!(
            ticket_status_to_ui_status(&TicketStatus::Open, Phase::Ready),
            ui::TicketStatus::Ready
        );
        // Open + active phase = InProgress
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
        // PluginStarted returns None
        assert!(activity_event_to_ui_entry(&ActivityEvent::PluginStarted).is_none());

        // DagRecomputed returns None
        assert!(
            activity_event_to_ui_entry(&ActivityEvent::DagRecomputed { ticket_count: 5 }).is_none()
        );

        // TicketStatusChanged returns None
        assert!(activity_event_to_ui_entry(&ActivityEvent::TicketStatusChanged {
            ticket_id: "T-001".to_string(),
            old_status: TicketStatus::Open,
            new_status: TicketStatus::InProgress,
        })
        .is_none());

        // ThreadSpawned returns ThreadStarted
        let entry = activity_event_to_ui_entry(&ActivityEvent::ThreadSpawned {
            ticket_id: "T-001".to_string(),
            pane_id: 42,
        });
        assert!(entry.is_some());
        let entry = entry.unwrap();
        match &entry.activity {
            ui::ActivityType::ThreadStarted { ticket_id, .. } => {
                assert_eq!(ticket_id, "T-001");
            }
            other => panic!("Expected ThreadStarted, got {:?}", other),
        }

        // PhaseCompleted maps correctly
        let entry = activity_event_to_ui_entry(&ActivityEvent::PhaseCompleted {
            ticket_id: "T-002".to_string(),
            phase: Phase::Design,
        });
        assert!(entry.is_some());
        let entry = entry.unwrap();
        match &entry.activity {
            ui::ActivityType::PhaseCompleted { ticket_id, phase } => {
                assert_eq!(ticket_id, "T-002");
                assert_eq!(*phase, ui::Phase::Design);
            }
            other => panic!("Expected PhaseCompleted, got {:?}", other),
        }

        // Error maps correctly
        let entry = activity_event_to_ui_entry(&ActivityEvent::Error {
            message: "something broke".to_string(),
        });
        assert!(entry.is_some());
        let entry = entry.unwrap();
        match &entry.activity {
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
        assert!(
            cmd.contains("docs/active/tickets/T-042-01.md"),
            "command should contain ticket path, got: {}",
            cmd
        );
        assert!(
            cmd.contains("docs/knowledge/rdspi-workflow.md"),
            "command should reference docs/knowledge/rdspi-workflow.md, got: {}",
            cmd
        );
        assert!(
            cmd.contains("CLAUDE.md"),
            "command should reference CLAUDE.md, got: {}",
            cmd
        );
        assert!(cmd.ends_with('\n'), "command should end with newline");
    }

    #[test]
    fn test_handle_pane_exited_with_context() {
        use lisa_core::types::{Thread, ThreadStatus};

        let mut state = State::default();

        // Register a thread for T-001
        let thread = Thread::new("T-001".to_string(), 0);
        state.threads.insert("T-001".to_string(), thread);

        // Simulate CommandPaneExited with context containing ticket_id
        let context = BTreeMap::from([("ticket_id".to_string(), "T-001".to_string())]);
        state.handle_pane_exited(42, Some(0), context);

        // Thread should be marked completed
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.status, ThreadStatus::Completed);

        // Activity log should have an exit event
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::ThreadExited {
                ticket_id,
                exit_code: Some(0),
            } if ticket_id == "T-001"
        )));
    }

    #[test]
    fn test_handle_pane_exited_failure() {
        use lisa_core::types::{Thread, ThreadStatus};

        let mut state = State::default();

        let thread = Thread::new("T-002".to_string(), 0);
        state.threads.insert("T-002".to_string(), thread);

        let context = BTreeMap::from([("ticket_id".to_string(), "T-002".to_string())]);
        state.handle_pane_exited(99, Some(1), context);

        let thread = state.threads.get("T-002").unwrap();
        assert_eq!(thread.status, ThreadStatus::Failed);
    }

    #[test]
    fn test_handle_pane_exited_no_context_fallback() {
        use lisa_core::types::{Thread, ThreadStatus};

        let mut state = State::default();

        let thread = Thread::new("T-003".to_string(), 55);
        state.threads.insert("T-003".to_string(), thread);
        state.pane_to_ticket.insert(55, "T-003".to_string());

        // Empty context — should fall back to pane_to_ticket
        let context = BTreeMap::new();
        state.handle_pane_exited(55, Some(0), context);

        let thread = state.threads.get("T-003").unwrap();
        assert_eq!(thread.status, ThreadStatus::Completed);
    }
}

// wasm32-wasip1 + cdylib produces a reactor module (no entry point).
// Zellij expects a command-style _start export to initialize the WASM instance.
// We must call __wasm_call_ctors to run WASI/Rust initialization (allocator, etc.)
// before any std:: functions (like std::fs) can be used.
extern "C" {
    fn __wasm_call_ctors();
}

#[no_mangle]
pub extern "C" fn _start() {
    unsafe {
        __wasm_call_ctors();
    }
}
