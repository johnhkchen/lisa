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

/// Build the full shell command to launch a Claude Code session for a ticket.
/// Sends Ctrl+C (\x03) twice first to kill any lingering process in the pane,
/// then launches Claude with the ticket prompt.
/// Uses \r (carriage return) to simulate pressing Enter in the terminal.
fn build_claude_command(ticket_dir: &Path, ticket_id: &str) -> String {
    format!(
        "\x03\x03claude --dangerously-skip-permissions \"{}\"\r",
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

/// State for the "mark done" modal overlay.
#[derive(Default)]
struct MarkDoneModal {
    /// Whether the modal is currently visible.
    open: bool,
    /// Non-done ticket IDs available for selection (sorted).
    ticket_ids: Vec<TicketId>,
    /// Currently highlighted index in `ticket_ids`.
    cursor: usize,
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

    /// Whether the loop has terminated (all tickets done).
    terminated: bool,

    /// Modal for manually marking tickets as done.
    modal: MarkDoneModal,

    /// Last known health status per ticket, for transition detection.
    last_health: HashMap<TicketId, lisa_core::types::HealthStatus>,
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
    /// Resets `has_session` because Claude Code exits after one-shot prompts,
    /// so the next use must launch a fresh process.
    fn release_slot_for_ticket(&mut self, ticket_id: &TicketId) {
        for slot in &mut self.agent_slots {
            if slot.ticket_id.as_ref() == Some(ticket_id) {
                slot.ticket_id = None;
                slot.has_session = false;
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

            // Write command to the slot's terminal — always launch a fresh process.
            // Ctrl+C in build_claude_command kills any lingering process first.
            let pane_id = self.agent_slots[slot_idx].pane_id;
            let cmd = build_claude_command(&host_ticket_dir, &ticket_id);
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
            // Skip implement phase — progress.md is a living tracking document,
            // not a completion signal. The agent sets phase: done in frontmatter
            // when implement work is complete.
            if current_phase == Phase::Implement {
                continue;
            }

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
                thread.last_phase_change = std::time::SystemTime::now();

                // Park if advancing to Review
                if next_phase == Phase::Review {
                    thread.park();
                }
            }
        }
    }

    /// Evaluate health of all running threads and log state changes.
    ///
    /// Uses the configured `stuck_threshold_secs` as the warning threshold.
    /// Logs `HealthStateChanged` activity events when a thread transitions
    /// between health states (e.g., Healthy → Stuck).
    fn evaluate_health(&mut self) {
        use lisa_core::types::{HealthStatus, ThreadStatus};

        let now = std::time::SystemTime::now();
        let threshold =
            std::time::Duration::from_secs(self.config.stuck_threshold_secs);

        // Collect health transitions
        let transitions: Vec<(TicketId, HealthStatus, HealthStatus)> = self
            .threads
            .iter()
            .filter(|(_, t)| t.status == ThreadStatus::Running || t.status == ThreadStatus::Failed)
            .filter_map(|(tid, t)| {
                let current = t.health(now, threshold);
                let previous = self.last_health.get(tid).copied().unwrap_or(HealthStatus::Healthy);
                if current != previous {
                    Some((tid.clone(), previous, current))
                } else {
                    None
                }
            })
            .collect();

        for (ticket_id, old_health, new_health) in transitions {
            self.last_health.insert(ticket_id.clone(), new_health);
            self.log_activity(ActivityEvent::HealthStateChanged {
                ticket_id,
                old_health,
                new_health,
            });
        }

        // Track health for threads we haven't seen before
        for (tid, t) in &self.threads {
            if !self.last_health.contains_key(tid) {
                let health = t.health(now, threshold);
                self.last_health.insert(tid.clone(), health);
            }
        }

        // Clean up last_health for threads that no longer exist
        self.last_health
            .retain(|tid, _| self.threads.contains_key(tid));
    }

    /// Detect threads that have been stuck beyond the hard timeout.
    ///
    /// The hard timeout is 2x the configured stuck_threshold_secs.
    /// Stuck threads at this point are marked as failed, their slots released,
    /// and they are removed from the threads map for retry.
    fn detect_stale_threads(&mut self) {
        use lisa_core::types::{HealthStatus, ThreadStatus};

        let now = std::time::SystemTime::now();
        // Hard timeout: 2x the configured stuck threshold
        let hard_timeout =
            std::time::Duration::from_secs(self.config.stuck_threshold_secs * 2);

        let stale: Vec<TicketId> = self
            .threads
            .iter()
            .filter(|(_, t)| t.status == ThreadStatus::Running)
            .filter(|(_, t)| t.health(now, hard_timeout) == HealthStatus::Stuck)
            .map(|(tid, _)| tid.clone())
            .collect();

        for ticket_id in stale {
            let mins = self.config.stuck_threshold_secs * 2 / 60;
            if let Some(thread) = self.threads.get_mut(&ticket_id) {
                thread.fail();
            }
            self.release_slot_for_ticket(&ticket_id);
            self.threads.remove(&ticket_id);
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "{} stale — no phase change for {}+ minutes, marked failed for retry",
                    ticket_id, mins
                ),
            });
        }
    }

    /// Check if all tickets are done and no threads are still running.
    fn check_all_done(&self) -> bool {
        !self.dag.is_empty()
            && self.dag.tickets().all(|t| t.phase == Phase::Done)
            && !self
                .threads
                .values()
                .any(|t| t.status == lisa_core::types::ThreadStatus::Running)
    }

    /// Timer-based completion detection.
    /// Rescans tickets, detects phase changes, marks completed threads,
    /// frees agent slots, and schedules new work.
    fn poll_tick(&mut self) {
        // Check for new artifacts and advance phases before rebuilding DAG
        self.check_artifact_advances();

        // Evaluate health: log transitions (Healthy→Stuck, etc.)
        self.evaluate_health();

        // Detect and handle stale threads at hard timeout (2x threshold)
        self.detect_stale_threads();

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
                        if thread.current_phase != ticket.phase {
                            thread.current_phase = ticket.phase;
                            thread.last_phase_change = std::time::SystemTime::now();
                        }
                    }
                }
            }
        }

        // Always try to schedule (slots may have freed up)
        self.schedule_ready_tickets();

        // Check for clean termination — all tickets done, no work remaining
        if self.check_all_done() {
            self.log_activity(ActivityEvent::AllTicketsDone);
            self.terminated = true;
            // Don't re-arm the timer — loop is complete
            return;
        }

        // Re-arm the timer
        set_timeout(POLL_INTERVAL_SECS);
    }

    /// Handle keyboard input. Returns true if the UI should re-render.
    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        if self.modal.open {
            match key.bare_key {
                BareKey::Esc | BareKey::Char('q') => {
                    self.modal.open = false;
                }
                BareKey::Up | BareKey::Char('k') => {
                    if self.modal.cursor > 0 {
                        self.modal.cursor -= 1;
                    }
                }
                BareKey::Down | BareKey::Char('j') => {
                    if self.modal.cursor + 1 < self.modal.ticket_ids.len() {
                        self.modal.cursor += 1;
                    }
                }
                BareKey::Enter => {
                    if let Some(ticket_id) = self.modal.ticket_ids.get(self.modal.cursor).cloned() {
                        self.mark_ticket_done(&ticket_id);
                    }
                    self.modal.open = false;
                }
                _ => return false,
            }
            return true;
        }

        // Normal mode: 'd' opens the mark-done modal
        if key.bare_key == BareKey::Char('d') {
            self.open_mark_done_modal();
            return true;
        }

        false
    }

    /// Open the mark-done modal with a list of non-done tickets.
    fn open_mark_done_modal(&mut self) {
        // Show non-done tickets that do NOT have a running agent thread.
        // This surfaces tickets agents left behind (forgot to mark done)
        // without letting the user accidentally interrupt active work.
        let running: std::collections::HashSet<&str> = self
            .threads
            .iter()
            .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
            .map(|(tid, _)| tid.as_str())
            .collect();

        let mut ids: Vec<TicketId> = self
            .dag
            .tickets()
            .filter(|t| t.phase != Phase::Done)
            .filter(|t| !running.contains(t.id.as_str()))
            .map(|t| t.id.clone())
            .collect();
        ids.sort();

        if ids.is_empty() {
            return; // Nothing to mark done
        }

        self.modal = MarkDoneModal {
            open: true,
            ticket_ids: ids,
            cursor: 0,
        };
    }

    /// Mark a ticket as done by updating its frontmatter on disk.
    fn mark_ticket_done(&mut self, ticket_id: &str) {
        let tid = ticket_id.to_string();
        let file_path = match self.dag.get_ticket(&tid).map(|t| t.file_path.clone()) {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => {
                self.log_activity(ActivityEvent::Error {
                    message: format!("Cannot find file for {}", ticket_id),
                });
                return;
            }
        };

        let old_phase = self
            .dag
            .get_ticket(&tid)
            .map(|t| t.phase)
            .unwrap_or(Phase::Ready);

        // Update phase to done
        if let Err(e) = ticket::update_ticket_phase(&file_path, Phase::Done) {
            self.log_activity(ActivityEvent::Error {
                message: format!("Failed to mark {} done: {}", ticket_id, e),
            });
            return;
        }

        // Also update status to done
        if let Err(e) = ticket::update_ticket_status(&file_path, lisa_core::types::TicketStatus::Done) {
            self.log_activity(ActivityEvent::Error {
                message: format!("Failed to update {} status: {}", ticket_id, e),
            });
            // Phase already changed, continue anyway
        }

        self.log_activity(ActivityEvent::TicketPhaseChanged {
            ticket_id: tid.clone(),
            old_phase,
            new_phase: Phase::Done,
        });

        // Release any slot occupied by this ticket
        if let Some(thread) = self.threads.get_mut(&tid) {
            thread.complete();
        }
        self.release_slot_for_ticket(&tid);

        // Rebuild DAG immediately so dependents become ready
        self.rebuild_dag();
        self.schedule_ready_tickets();
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
            EventType::Key,
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

            Event::Key(key) => {
                should_render = self.handle_key(key);
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

        if self.terminated {
            println!("All tickets done. Lisa loop complete.");
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

        // Build health alerts from stuck/failed threads
        let now = std::time::SystemTime::now();
        let threshold = std::time::Duration::from_secs(self.config.stuck_threshold_secs);
        let alerts: Vec<ui::HealthAlert> = self
            .threads
            .values()
            .filter(|t| t.status == lisa_core::types::ThreadStatus::Running || t.status == lisa_core::types::ThreadStatus::Failed)
            .filter_map(|t| {
                let health = t.health(now, threshold);
                match health {
                    lisa_core::types::HealthStatus::Stuck => Some(ui::HealthAlert {
                        ticket_id: t.ticket_id.clone(),
                        alert_type: ui::AlertType::Stuck,
                        detail: format!("No phase change for {}+ min", threshold.as_secs() / 60),
                        suggested_actions: vec!["Check pane".to_string(), "Restart session".to_string()],
                    }),
                    lisa_core::types::HealthStatus::Failed => Some(ui::HealthAlert {
                        ticket_id: t.ticket_id.clone(),
                        alert_type: ui::AlertType::Failed,
                        detail: "Session failed".to_string(),
                        suggested_actions: vec!["Check logs".to_string(), "Retry".to_string()],
                    }),
                    _ => None,
                }
            })
            .collect();

        ui::PluginState {
            tickets,
            active_threads,
            parked_threads,
            activity_log,
            alerts,
            current_time: Duration::from_secs(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
            selected_ticket: None,
            modal: ui::ModalState {
                open: self.modal.open,
                ticket_ids: self.modal.ticket_ids.clone(),
                cursor: self.modal.cursor,
            },
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
    // Phase is the primary source of truth — agents often set phase: done
    // but forget to update status: open → done.
    if phase == Phase::Done {
        return ui::TicketStatus::Done;
    }
    if phase == Phase::Ready {
        return ui::TicketStatus::Ready;
    }

    match status {
        lisa_core::types::TicketStatus::Open
        | lisa_core::types::TicketStatus::InProgress => ui::TicketStatus::InProgress,
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
        ActivityEvent::AllTicketsDone => ui::ActivityType::PhaseCompleted {
            ticket_id: "all".to_string(),
            phase: ui::Phase::Done,
        },
        ActivityEvent::Error { message } => ui::ActivityType::Error {
            ticket_id: String::new(),
            message: message.clone(),
        },
        ActivityEvent::HealthStateChanged {
            ticket_id,
            new_health,
            ..
        } => {
            use lisa_core::types::HealthStatus;
            match new_health {
                HealthStatus::Stuck => ui::ActivityType::Warning {
                    ticket_id: ticket_id.clone(),
                    message: "Session stuck — no phase progress".to_string(),
                },
                HealthStatus::Failed => ui::ActivityType::Error {
                    ticket_id: ticket_id.clone(),
                    message: "Session failed".to_string(),
                },
                HealthStatus::Healthy => return None,
            }
        }
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
        // Phase takes priority over status
        assert_eq!(
            ticket_status_to_ui_status(&TicketStatus::Open, Phase::Done),
            ui::TicketStatus::Done,
            "phase: done overrides status: open"
        );
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
            ticket_status_to_ui_status(&TicketStatus::Blocked, Phase::Implement),
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

        assert!(cmd.starts_with("\x03\x03"), "should start with Ctrl+C to kill lingering process");
        assert!(cmd.contains("claude --dangerously-skip-permissions"));
        assert!(cmd.contains("docs/active/tickets/T-042-01.md"));
        assert!(cmd.contains("docs/knowledge/rdspi-workflow.md"));
        assert!(cmd.contains("CLAUDE.md"));
        assert!(cmd.ends_with('\r'), "should end with carriage return");
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
    fn test_check_artifact_advances_implement_skipped() {
        // progress.md is a living tracking document, not a completion signal.
        // The agent sets phase: done in frontmatter when implement work is complete.
        // So check_artifact_advances should NOT advance implement → review.
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

        // Implement phase should NOT be advanced by artifact detection
        let thread = state.threads.get("T-002").unwrap();
        assert_eq!(thread.current_phase, Phase::Implement);
        assert_eq!(thread.status, ThreadStatus::Running);
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

    #[test]
    fn test_check_all_done_true() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: done1\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n\nDone\n",
        ).unwrap();
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: done2\ntype: task\nstatus: done\npriority: high\nphase: done\ndepends_on: [T-001]\n---\n\nDone\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let state = State {
            dag,
            ..State::default()
        };

        // All tickets done, no running threads → true
        assert!(state.check_all_done());
    }

    #[test]
    fn test_check_all_done_false_not_all_done() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: done1\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n\nDone\n",
        ).unwrap();
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: wip\ntype: task\nstatus: open\npriority: high\nphase: implement\ndepends_on: [T-001]\n---\n\nWIP\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let state = State {
            dag,
            ..State::default()
        };

        // Not all tickets done → false
        assert!(!state.check_all_done());
    }

    #[test]
    fn test_check_all_done_false_running_thread() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: done\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n\nDone\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            ..State::default()
        };

        // Add a running thread — even though all tickets are done,
        // a running thread means we shouldn't terminate yet
        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);

        assert!(!state.check_all_done());
    }

    #[test]
    fn test_check_all_done_empty_dag() {
        let state = State::default();
        // Empty DAG → false (nothing to be "done" about)
        assert!(!state.check_all_done());
    }

    #[test]
    fn test_detect_stale_threads() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: stale\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n\nBody\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Create a thread that's been stuck for 31+ minutes
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change = std::time::SystemTime::now()
            - std::time::Duration::from_secs(31 * 60);
        state.threads.insert("T-001".to_string(), thread);

        // Add an agent slot so we can verify it gets released
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
        });

        state.detect_stale_threads();

        // Thread should be removed (failed + cleaned up for retry)
        assert!(state.threads.is_empty());

        // Slot should be released
        assert!(state.agent_slots[0].ticket_id.is_none());

        // Error logged
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Error { message } if message.contains("stale")
        )));
    }

    #[test]
    fn test_stale_thread_not_stale_yet() {
        use lisa_core::types::{Thread, ThreadStatus};

        let mut state = State::default();

        // Create a thread that started recently (5 minutes ago)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change = std::time::SystemTime::now()
            - std::time::Duration::from_secs(5 * 60);
        state.threads.insert("T-001".to_string(), thread);

        state.detect_stale_threads();

        // Thread should still be running — not stale yet
        assert_eq!(state.threads.len(), 1);
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.status, ThreadStatus::Running);
        assert!(state.activity_log.is_empty());
    }

    #[test]
    fn test_all_tickets_done_event_conversion() {
        let entry = activity_event_to_ui_entry(&ActivityEvent::AllTicketsDone);
        assert!(entry.is_some());
        match &entry.unwrap().activity {
            ui::ActivityType::PhaseCompleted { ticket_id, phase } => {
                assert_eq!(ticket_id, "all");
                assert_eq!(*phase, ui::Phase::Done);
            }
            other => panic!("Expected PhaseCompleted, got {:?}", other),
        }
    }

    #[test]
    fn test_rescheduling_conditions_after_completion() {
        use lisa_core::types::Thread;
        use std::fs;

        // Test that after a ticket completes, its dependents become ready
        // and the slot is freed. (We can't call schedule_ready_tickets() in
        // tests because it calls write_chars_to_pane_id which is a zellij
        // host function, so we test the preconditions instead.)

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        // T-001: done
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: done\ntype: task\nstatus: open\npriority: high\nphase: done\n---\n\nDone\n",
        ).unwrap();

        // T-002: ready, depends on T-001 (which is done)
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: next\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-001]\n---\n\nNext\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Simulate T-001 had a running thread that completed
        let mut thread = Thread::new("T-001", 1);
        thread.complete();
        state.threads.insert("T-001".to_string(), thread);

        // Simulate the slot being occupied then released
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
        });
        state.release_slot_for_ticket(&"T-001".to_string());

        // Verify: slot is now idle
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert!(state.find_idle_slot().is_some());

        // Verify: DAG shows T-002 as ready (T-001 is done)
        let ready = state.dag.get_ready_tickets();
        assert!(ready.contains(&"T-002".to_string()));

        // Verify: T-002 doesn't have a thread yet, so it would be scheduled
        assert!(!state.threads.contains_key("T-002"));
    }

    #[test]
    fn test_evaluate_health_stuck_transition() {
        use lisa_core::types::{HealthStatus, Thread};

        let mut state = State::default();

        // Create a thread that's been stuck past the threshold (default 600s)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change = std::time::SystemTime::now()
            - std::time::Duration::from_secs(700); // past 600s threshold
        state.threads.insert("T-001".to_string(), thread);

        state.evaluate_health();

        // Should have logged a HealthStateChanged event (Healthy → Stuck)
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::HealthStateChanged {
                ticket_id,
                old_health: HealthStatus::Healthy,
                new_health: HealthStatus::Stuck,
            } if ticket_id == "T-001"
        )));

        // last_health should be updated
        assert_eq!(
            state.last_health.get("T-001"),
            Some(&HealthStatus::Stuck)
        );
    }

    #[test]
    fn test_evaluate_health_no_transition_when_healthy() {
        use lisa_core::types::{HealthStatus, Thread};

        let mut state = State::default();

        // Create a fresh thread (well within threshold)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-001".to_string(), thread);

        state.evaluate_health();

        // No transitions should be logged for a fresh healthy thread
        assert!(state.activity_log.is_empty());

        // last_health should still be tracked
        assert_eq!(
            state.last_health.get("T-001"),
            Some(&HealthStatus::Healthy)
        );
    }

    #[test]
    fn test_evaluate_health_failed_transition() {
        use lisa_core::types::{HealthStatus, Thread};

        let mut state = State::default();

        // Create a failed thread
        let mut thread = Thread::new("T-001", 1);
        thread.fail();
        state.threads.insert("T-001".to_string(), thread);

        state.evaluate_health();

        // Should log Healthy → Failed transition
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::HealthStateChanged {
                ticket_id,
                old_health: HealthStatus::Healthy,
                new_health: HealthStatus::Failed,
            } if ticket_id == "T-001"
        )));
    }

    #[test]
    fn test_evaluate_health_cleanup_removed_threads() {
        use lisa_core::types::HealthStatus;

        let mut state = State::default();

        // Insert stale entry in last_health for a thread that no longer exists
        state
            .last_health
            .insert("T-GONE".to_string(), HealthStatus::Stuck);

        state.evaluate_health();

        // Should be cleaned up
        assert!(!state.last_health.contains_key("T-GONE"));
    }

    #[test]
    fn test_to_ui_state_includes_alerts_for_stuck_thread() {
        use lisa_core::types::Thread;

        let mut state = State::default();

        // Create a stuck thread
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change = std::time::SystemTime::now()
            - std::time::Duration::from_secs(700);
        state.threads.insert("T-001".to_string(), thread);
        state.initialized = true;

        let ui_state = state.to_ui_state();

        // Should have one alert for the stuck thread
        assert_eq!(ui_state.alerts.len(), 1);
        assert_eq!(ui_state.alerts[0].ticket_id, "T-001");
        assert_eq!(ui_state.alerts[0].alert_type, ui::AlertType::Stuck);
    }

    #[test]
    fn test_to_ui_state_includes_alerts_for_failed_thread() {
        use lisa_core::types::Thread;

        let mut state = State::default();

        // Create a failed thread
        let mut thread = Thread::new("T-001", 1);
        thread.fail();
        state.threads.insert("T-001".to_string(), thread);
        state.initialized = true;

        let ui_state = state.to_ui_state();

        assert_eq!(ui_state.alerts.len(), 1);
        assert_eq!(ui_state.alerts[0].ticket_id, "T-001");
        assert_eq!(ui_state.alerts[0].alert_type, ui::AlertType::Failed);
    }

    #[test]
    fn test_to_ui_state_no_alerts_for_healthy_thread() {
        use lisa_core::types::Thread;

        let mut state = State::default();

        // Create a fresh healthy thread
        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);
        state.initialized = true;

        let ui_state = state.to_ui_state();

        assert!(ui_state.alerts.is_empty());
    }

    #[test]
    fn test_health_state_changed_event_to_ui_stuck() {
        use lisa_core::types::HealthStatus;

        let entry = activity_event_to_ui_entry(&ActivityEvent::HealthStateChanged {
            ticket_id: "T-001".to_string(),
            old_health: HealthStatus::Healthy,
            new_health: HealthStatus::Stuck,
        });

        assert!(entry.is_some());
        match &entry.unwrap().activity {
            ui::ActivityType::Warning { ticket_id, message } => {
                assert_eq!(ticket_id, "T-001");
                assert!(message.contains("stuck"));
            }
            other => panic!("Expected Warning, got {:?}", other),
        }
    }

    #[test]
    fn test_health_state_changed_event_to_ui_failed() {
        use lisa_core::types::HealthStatus;

        let entry = activity_event_to_ui_entry(&ActivityEvent::HealthStateChanged {
            ticket_id: "T-001".to_string(),
            old_health: HealthStatus::Healthy,
            new_health: HealthStatus::Failed,
        });

        assert!(entry.is_some());
        match &entry.unwrap().activity {
            ui::ActivityType::Error { ticket_id, message } => {
                assert_eq!(ticket_id, "T-001");
                assert!(message.contains("failed"));
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_health_state_changed_event_to_ui_healthy_ignored() {
        use lisa_core::types::HealthStatus;

        let entry = activity_event_to_ui_entry(&ActivityEvent::HealthStateChanged {
            ticket_id: "T-001".to_string(),
            old_health: HealthStatus::Stuck,
            new_health: HealthStatus::Healthy,
        });

        // Healthy transitions are not surfaced in the UI
        assert!(entry.is_none());
    }

    #[test]
    fn test_detect_stale_uses_config_threshold() {
        use lisa_core::types::Thread;

        // Set a custom stuck_threshold_secs of 120 (2 minutes)
        // Hard timeout = 2 * 120 = 240s (4 minutes)
        let mut state = State {
            config: PluginConfig {
                stuck_threshold_secs: 120,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Create a thread stuck for 5 minutes (300s) — past hard timeout of 240s
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change = std::time::SystemTime::now()
            - std::time::Duration::from_secs(300);
        state.threads.insert("T-001".to_string(), thread);

        state.detect_stale_threads();

        // Should be removed (past hard timeout)
        assert!(state.threads.is_empty());
    }

    #[test]
    fn test_detect_stale_warning_threshold_not_hard_timeout() {
        use lisa_core::types::{Thread, ThreadStatus};

        // stuck_threshold_secs = 120, hard timeout = 240
        let mut state = State {
            config: PluginConfig {
                stuck_threshold_secs: 120,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Create a thread stuck for 180s — past warning (120s) but NOT past hard timeout (240s)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change = std::time::SystemTime::now()
            - std::time::Duration::from_secs(180);
        state.threads.insert("T-001".to_string(), thread);

        state.detect_stale_threads();

        // Should NOT be removed (only past warning threshold, not hard timeout)
        assert_eq!(state.threads.len(), 1);
        assert_eq!(
            state.threads.get("T-001").unwrap().status,
            ThreadStatus::Running
        );
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
