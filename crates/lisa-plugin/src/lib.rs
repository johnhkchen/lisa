//! Lisa/Ralph - A Zellij plugin for DAG-driven concurrent task scheduling
//!
//! This plugin implements the RDSPI workflow (Research -> Design -> Structure -> Plan -> Implement)
//! as a DAG-driven concurrent scheduler. It manages Claude Code sessions for each ticket,
//! tracks phase progress, and provides a live dashboard.

mod ui;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use zellij_tile::prelude::*;

use lisa_core::dag::Dag;
use lisa_core::diagnostics;
use lisa_core::ticket;
use lisa_core::types::{ActivityEvent, Phase, PluginConfig, Thread, TicketId};

/// How often (in seconds) the plugin rescans ticket files to detect phase changes.
const POLL_INTERVAL_SECS: f64 = 5.0;

/// Short delay (seconds) before flushing deferred pane commands.
/// Gives the pane time to process `/clear` before the command arrives.
const FLUSH_DELAY_SECS: f64 = 0.5;

/// The prompt text sent to Claude Code for a ticket.
fn ticket_prompt(ticket_dir: &Path, ticket_id: &str) -> String {
    let ticket_path = ticket_dir.join(format!("{}.md", ticket_id));
    format!(
        "Read the ticket at {}, the project context in CLAUDE.md, and the RDSPI workflow in docs/knowledge/rdspi-workflow.md. \
         Start from the current phase indicated in the ticket frontmatter.",
        ticket_path.display()
    )
}

/// Build the full shell command to launch Claude Code in a fresh pane.
/// Sets LISA_TICKET_ID env var so the idle signal hook knows which ticket is running.
fn build_claude_command(ticket_dir: &Path, ticket_id: &str) -> String {
    format!(
        "LISA_TICKET_ID={} claude --dangerously-skip-permissions -p \"{}\"",
        ticket_id,
        ticket_prompt(ticket_dir, ticket_id)
    )
}

/// Send text to a pane followed by Enter (carriage return as a raw byte).
///
/// `write_chars_to_pane_id` sends characters as typed text, but TUI apps like
/// Claude Code need the Enter key delivered as a raw byte (0x0D) via
/// `write_to_pane_id`, not as a `\r` character embedded in the text stream.
fn send_line_to_pane(text: &str, pane_id: PaneId) {
    write_chars_to_pane_id(text, pane_id);
    write_to_pane_id(vec![13], pane_id); // Enter key
}

/// Strip the `/host/` prefix from a WASI sandbox path to get the host-relative path.
///
/// Inside the WASI sandbox, the host filesystem is mounted at `/host/`.
/// Commands sent to agent panes run on the host, so paths must not have this prefix.
fn strip_host_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    PathBuf::from(s.strip_prefix("/host/").unwrap_or(&s).to_string())
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

    /// Commands queued for deferred writing to panes.
    /// Written on the next timer tick after a short delay, so the pane
    /// has time to process the preceding `/clear`.
    pending_pane_writes: Vec<(u32, String)>,

    /// Number of outstanding timers. Used to prevent timer chain duplication
    /// when short flush timers are set alongside the regular poll timer.
    pending_timer_count: u32,

    /// Path to the idle signal directory (`.lisa/signals/` under /host/).
    signal_dir: PathBuf,

    /// Idle-without-artifact alerts detected during the current poll cycle.
    /// Cleared and re-populated each cycle by `check_idle_signals()`.
    idle_alerts: Vec<(TicketId, String)>,
}

impl State {
    const MAX_ACTIVITY_LOG: usize = 100;

    /// Set a timer and track it so we can avoid re-arming when duplicates are pending.
    fn arm_timer(&mut self, secs: f64) {
        set_timeout(secs);
        self.pending_timer_count += 1;
    }

    /// Called when a timer fires. Decrements the counter and returns whether
    /// the poll timer should be re-armed (only when no other timers are pending).
    fn timer_fired(&mut self) -> bool {
        self.pending_timer_count = self.pending_timer_count.saturating_sub(1);
        self.pending_timer_count == 0
    }

    /// Flush any deferred pane writes (commands queued after `/clear`).
    fn flush_pending_pane_writes(&mut self) {
        for (pane_id, cmd) in self.pending_pane_writes.drain(..) {
            send_line_to_pane(&cmd, PaneId::Terminal(pane_id));
        }
    }

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
                    match self.last_phases.get(&ticket.id) {
                        Some(&old_phase) => {
                            if old_phase != ticket.phase {
                                self.log_activity(ActivityEvent::TicketPhaseChanged {
                                    ticket_id: ticket.id.clone(),
                                    old_phase,
                                    new_phase: ticket.phase,
                                });
                                changed = true;
                            }
                        }
                        None => {
                            // First-seen ticket: treat non-Ready phases as a change
                            // so downstream slot-release logic runs on first load.
                            if ticket.phase != Phase::Ready {
                                changed = true;
                            }
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
            self.log_activity(ActivityEvent::Info {
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
    /// Keeps `has_session = true` so subsequent scheduling sends `/clear` + prompt
    /// into the already-running Claude Code session instead of relaunching.
    fn release_slot_for_ticket(&mut self, ticket_id: &TicketId) {
        let mut released_pane: Option<u32> = None;
        for slot in &mut self.agent_slots {
            if slot.ticket_id.as_ref() == Some(ticket_id) {
                released_pane = Some(slot.pane_id);
                slot.ticket_id = None;
                // has_session stays true — Claude Code is still running
                break;
            }
        }
        match released_pane {
            Some(pane_id) => self.log_activity(ActivityEvent::Info {
                message: format!("Released slot #{} for {}", pane_id, ticket_id),
            }),
            None => self.log_activity(ActivityEvent::Info {
                message: format!("No slot found for {}", ticket_id),
            }),
        }
    }

    /// Schedule ready tickets into idle agent slots.
    fn schedule_ready_tickets(&mut self) {
        if !self.permissions_granted || !self.slots_discovered {
            return;
        }

        let ready = self.dag.get_ready_tickets();
        let mut unscheduled = 0usize;

        for ticket_id in ready {
            // Skip tickets that already have an active thread.
            // Defensive: if a stale Completed thread exists, remove it and proceed.
            let is_completed = self
                .threads
                .get(&ticket_id)
                .map(|t| t.status == lisa_core::types::ThreadStatus::Completed)
                .unwrap_or(false);
            if self.threads.contains_key(&ticket_id) {
                if is_completed {
                    self.threads.remove(&ticket_id);
                } else {
                    self.log_activity(ActivityEvent::Info {
                        message: format!("Skipping {}: thread already exists", ticket_id),
                    });
                    continue;
                }
            }

            // Find an idle slot
            let slot_idx = match self.find_idle_slot() {
                Some(idx) => idx,
                None => {
                    unscheduled += 1;
                    continue;
                }
            };

            // Build the host-relative ticket dir (strip /host/ prefix)
            let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);

            let pane_id = self.agent_slots[slot_idx].pane_id;

            let launch_cmd;
            if self.agent_slots[slot_idx].has_session {
                // Session reuse: exit the current Claude Code session, then
                // re-launch with the new ticket's env var so LISA_TICKET_ID
                // is correct for the idle signal hook.
                send_line_to_pane("/exit", PaneId::Terminal(pane_id));
                let cmd = build_claude_command(&host_ticket_dir, &ticket_id);
                launch_cmd = cmd.clone();
                self.pending_pane_writes.push((pane_id, cmd));
            } else {
                // Fresh pane — launch Claude Code from the shell.
                let cmd = build_claude_command(&host_ticket_dir, &ticket_id);
                launch_cmd = cmd.clone();
                send_line_to_pane(&cmd, PaneId::Terminal(pane_id));
                self.agent_slots[slot_idx].has_session = true;
            }

            self.agent_slots[slot_idx].ticket_id = Some(ticket_id.clone());

            // Create thread record with the ticket's current phase
            let mut thread = Thread::new(ticket_id.clone(), pane_id);
            if let Some(ticket) = self.dag.get_ticket(&ticket_id) {
                thread.current_phase = ticket.phase;
            }
            self.threads.insert(ticket_id.clone(), thread);

            self.log_activity(ActivityEvent::SessionLaunch {
                ticket_id: ticket_id.clone(),
                pane_id,
                command: launch_cmd,
            });
            self.log_activity(ActivityEvent::ThreadSpawned {
                ticket_id,
                pane_id,
            });
        }

        if unscheduled > 0 {
            self.log_activity(ActivityEvent::Info {
                message: format!(
                    "No idle slots available, {} ready tickets waiting",
                    unscheduled
                ),
            });
        }

        // If we queued any commands, set a short timer to flush them
        if !self.pending_pane_writes.is_empty() {
            self.arm_timer(FLUSH_DELAY_SECS);
        }
    }

    /// Safety sweep: release any agent slots still assigned to done tickets.
    ///
    /// This catches slots that the normal done-ticket detection in `poll_tick`
    /// might miss — for example, if a thread was already cleaned up but the
    /// slot assignment wasn't cleared.
    fn sweep_stale_slots(&mut self) {
        let stale: Vec<(u32, TicketId)> = self
            .agent_slots
            .iter()
            .filter_map(|slot| {
                let tid = slot.ticket_id.as_ref()?;
                let is_done = self
                    .dag
                    .get_ticket(tid)
                    .map(|t| t.phase == Phase::Done)
                    .unwrap_or(false);
                if is_done {
                    Some((slot.pane_id, tid.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (pane_id, ticket_id) in stale {
            self.release_slot_for_ticket(&ticket_id);
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Slot #{} held stale ticket {}, releasing",
                    pane_id, ticket_id
                ),
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

    /// Scan for idle signal files and advance ticket phases accordingly.
    ///
    /// When a Claude Code session goes idle, the on-idle hook writes a
    /// `.lisa/signals/{ticket_id}.idle` file. This method reads those signals
    /// and applies the phase advancement rules:
    ///
    /// - **Implement**: idle signal alone advances to Review (parks thread)
    /// - **Research/Design/Structure/Plan**: idle signal + artifact advances to next phase
    /// - **Idle without artifact**: generates an alert for the attention banner
    ///
    /// Signal files are always deleted after processing to prevent re-triggering.
    fn check_idle_signals(&mut self) {
        self.idle_alerts.clear();

        let entries = match std::fs::read_dir(&self.signal_dir) {
            Ok(entries) => entries,
            Err(_) => return, // Directory doesn't exist yet — normal on first run
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let filename = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) if name.ends_with(".idle") => name.to_string(),
                _ => continue,
            };

            let ticket_id: TicketId = filename.trim_end_matches(".idle").to_string();

            // Clean up the signal file immediately (prevents re-trigger on next poll)
            let _ = std::fs::remove_file(&path);

            // Look up thread — signal only meaningful for running threads
            let current_phase = match self.threads.get(&ticket_id) {
                Some(t) if t.status == lisa_core::types::ThreadStatus::Running => {
                    t.current_phase
                }
                _ => continue,
            };

            match current_phase {
                Phase::Implement => {
                    // Idle signal alone is the completion signal for Implement
                    let file_path = self
                        .dag
                        .get_ticket(&ticket_id)
                        .map(|t| t.file_path.clone());
                    let file_path = match file_path {
                        Some(p) if !p.as_os_str().is_empty() => p,
                        _ => continue,
                    };

                    if let Err(e) = ticket::update_ticket_phase(&file_path, Phase::Review) {
                        self.log_activity(ActivityEvent::Error {
                            message: format!(
                                "Failed to advance {} via idle signal: {}",
                                ticket_id, e
                            ),
                        });
                        continue;
                    }

                    self.log_activity(ActivityEvent::PhaseCompleted {
                        ticket_id: ticket_id.clone(),
                        phase: Phase::Implement,
                    });
                    self.log_activity(ActivityEvent::TicketPhaseChanged {
                        ticket_id: ticket_id.clone(),
                        old_phase: Phase::Implement,
                        new_phase: Phase::Review,
                    });

                    if let Some(thread) = self.threads.get_mut(&ticket_id) {
                        thread.current_phase = Phase::Review;
                        thread.last_phase_change = std::time::SystemTime::now();
                        thread.park();
                    }
                }

                Phase::Research | Phase::Design | Phase::Structure | Phase::Plan => {
                    // Need artifact + idle signal for these phases
                    let artifact_name = match current_phase.artifact_filename() {
                        Some(name) => name,
                        None => continue,
                    };
                    let artifact_path =
                        self.config.work_dir.join(&ticket_id).join(artifact_name);

                    if artifact_path.exists() {
                        let next_phase = match current_phase.next() {
                            Some(p) => p,
                            None => continue,
                        };

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
                                message: format!(
                                    "Failed to advance {} via idle signal: {}",
                                    ticket_id, e
                                ),
                            });
                            continue;
                        }

                        self.log_activity(ActivityEvent::PhaseCompleted {
                            ticket_id: ticket_id.clone(),
                            phase: current_phase,
                        });
                        self.log_activity(ActivityEvent::TicketPhaseChanged {
                            ticket_id: ticket_id.clone(),
                            old_phase: current_phase,
                            new_phase: next_phase,
                        });

                        if let Some(thread) = self.threads.get_mut(&ticket_id) {
                            thread.current_phase = next_phase;
                            thread.last_phase_change = std::time::SystemTime::now();
                            if next_phase == Phase::Review {
                                thread.park();
                            }
                        }
                    } else {
                        // Idle without artifact — alert
                        let detail = format!(
                            "Agent idle in {} phase but {} not found",
                            current_phase, artifact_name
                        );
                        self.idle_alerts
                            .push((ticket_id.clone(), detail.clone()));
                        self.log_activity(ActivityEvent::Warning {
                            message: format!("{}: {}", ticket_id, detail),
                        });
                    }
                }

                _ => {
                    // Ready, Review, Done — signal already cleaned up, nothing to do
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

    /// Periodic audit: remove any thread whose ticket is done or missing from the DAG.
    ///
    /// This is a safety net that catches threads that slipped through normal
    /// completion detection — for example, if a ticket was manually edited to
    /// done while the plugin was between poll cycles.
    fn audit_threads(&mut self) {
        let orphaned: Vec<TicketId> = self
            .threads
            .keys()
            .filter(|tid| {
                self.dag
                    .get_ticket(tid)
                    .map(|t| t.phase == Phase::Done)
                    .unwrap_or(true) // missing from DAG = orphaned
            })
            .cloned()
            .collect();

        for tid in orphaned {
            self.log_activity(ActivityEvent::Error {
                message: format!("Orphaned thread for {} — removing", tid),
            });
            self.release_slot_for_ticket(&tid);
            self.threads.remove(&tid);
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

        // Check for idle signals and advance phases / generate alerts
        self.check_idle_signals();

        // Evaluate health: log transitions (Healthy→Stuck, etc.)
        self.evaluate_health();

        // Detect and handle stale threads at hard timeout (2x threshold)
        self.detect_stale_threads();

        self.rebuild_dag();

        // Unconditionally check for tickets that moved to Done — mark their
        // threads complete and release slots. This must not be gated behind
        // change detection; if detection misses a transition, slots get stuck.
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
            self.threads.remove(ticket_id);
            self.log_activity(ActivityEvent::ThreadExited {
                ticket_id: ticket_id.clone(),
                exit_code: Some(0),
            });
        }

        // Defensive reconciliation: catch phase changes from external edits or
        // missed transitions. Normally a no-op because check_artifact_advances()
        // and check_idle_signals() already update thread.current_phase to match.
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

        // Safety sweep: release any slots still pointing at done tickets
        self.sweep_stale_slots();

        // Audit threads: remove any orphaned entries for done/missing tickets
        self.audit_threads();

        // Always try to schedule (slots may have freed up)
        self.schedule_ready_tickets();

        // Log poll cycle summary
        let ready_count = self.dag.get_ready_tickets().len();
        let running_count = self
            .threads
            .values()
            .filter(|t| t.status == lisa_core::types::ThreadStatus::Running)
            .count();
        let idle_count = self
            .agent_slots
            .iter()
            .filter(|s| s.ticket_id.is_none())
            .count();
        self.log_activity(ActivityEvent::PollSummary {
            ready: ready_count,
            running: running_count,
            idle_slots: idle_count,
        });

        // Check for clean termination — all tickets done, no work remaining
        if self.check_all_done() {
            self.log_activity(ActivityEvent::AllTicketsDone);
            self.terminated = true;
            // Don't re-arm the timer — loop is complete
            return;
        }

        // Re-arm the timer
        self.arm_timer(POLL_INTERVAL_SECS);
    }

    /// Format a single ActivityEvent as a one-line string for the state snapshot.
    fn format_activity_event(event: &ActivityEvent) -> String {
        match event {
            ActivityEvent::PluginStarted => "PluginStarted".to_string(),
            ActivityEvent::ThreadSpawned { ticket_id, pane_id } => {
                format!("ThreadSpawned: {} pane=#{}", ticket_id, pane_id)
            }
            ActivityEvent::PhaseCompleted { ticket_id, phase } => {
                format!("PhaseCompleted: {} {}", ticket_id, phase)
            }
            ActivityEvent::ThreadExited {
                ticket_id,
                exit_code,
            } => {
                format!("ThreadExited: {} exit_code={:?}", ticket_id, exit_code)
            }
            ActivityEvent::TicketStatusChanged {
                ticket_id,
                old_status,
                new_status,
            } => {
                format!("TicketStatusChanged: {} {} -> {}", ticket_id, old_status, new_status)
            }
            ActivityEvent::TicketPhaseChanged {
                ticket_id,
                old_phase,
                new_phase,
            } => {
                format!("TicketPhaseChanged: {} {} -> {}", ticket_id, old_phase, new_phase)
            }
            ActivityEvent::ArtifactCreated {
                ticket_id,
                phase,
                path,
            } => {
                format!("ArtifactCreated: {} {} {}", ticket_id, phase, path.display())
            }
            ActivityEvent::CommitMade {
                ticket_id,
                commit_hash,
            } => {
                format!("CommitMade: {} {}", ticket_id, commit_hash)
            }
            ActivityEvent::Error { message } => format!("Error: {}", message),
            ActivityEvent::DagRecomputed { ticket_count } => {
                format!("DagRecomputed: {} tickets", ticket_count)
            }
            ActivityEvent::AllTicketsDone => "AllTicketsDone".to_string(),
            ActivityEvent::HealthStateChanged {
                ticket_id,
                old_health,
                new_health,
            } => {
                format!("HealthStateChanged: {} {:?} -> {:?}", ticket_id, old_health, new_health)
            }
            ActivityEvent::Warning { message } => format!("Warning: {}", message),
            ActivityEvent::Info { message } => format!("Info: {}", message),
            ActivityEvent::PollSummary {
                ready,
                running,
                idle_slots,
            } => {
                format!("PollSummary: ready={} running={} idle_slots={}", ready, running, idle_slots)
            }
            ActivityEvent::SessionLaunch {
                ticket_id,
                pane_id,
                command,
            } => {
                format!("SessionLaunch: {} pane=#{} cmd={}", ticket_id, pane_id, command)
            }
        }
    }

    /// Format the full plugin state as a human-readable text snapshot.
    fn format_snapshot(&self) -> String {
        use std::fmt::Write;
        use std::time::SystemTime;

        let now = SystemTime::now();
        let epoch_secs = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut out = String::new();

        // Header
        writeln!(out, "=== Lisa State Snapshot ===").unwrap();
        writeln!(out, "Timestamp: {} (unix epoch)", epoch_secs).unwrap();
        writeln!(out).unwrap();

        // Config
        writeln!(out, "=== Config ===").unwrap();
        writeln!(out, "ticket_dir:          {}", self.config.ticket_dir.display()).unwrap();
        writeln!(out, "story_dir:           {}", self.config.story_dir.display()).unwrap();
        writeln!(out, "work_dir:            {}", self.config.work_dir.display()).unwrap();
        writeln!(out, "max_threads:         {}", self.config.max_threads).unwrap();
        writeln!(out, "auto_advance:        {}", self.config.auto_advance).unwrap();
        writeln!(out, "stuck_threshold_secs: {}", self.config.stuck_threshold_secs).unwrap();
        writeln!(out).unwrap();

        // Plugin status
        writeln!(out, "=== Plugin Status ===").unwrap();
        writeln!(out, "initialized:         {}", self.initialized).unwrap();
        writeln!(out, "permissions_granted: {}", self.permissions_granted).unwrap();
        writeln!(out, "slots_discovered:    {}", self.slots_discovered).unwrap();
        writeln!(out, "terminated:          {}", self.terminated).unwrap();
        writeln!(out, "pending_timer_count: {}", self.pending_timer_count).unwrap();
        writeln!(out, "pending_pane_writes: {}", self.pending_pane_writes.len()).unwrap();
        writeln!(out).unwrap();

        // Tickets
        writeln!(out, "=== Tickets ===").unwrap();
        let mut ticket_list: Vec<_> = self.dag.tickets().collect();
        ticket_list.sort_by(|a, b| a.id.cmp(&b.id));
        writeln!(out, "{:<14} {:<12} {:<12} {}", "ID", "PHASE", "STATUS", "DEPENDS_ON").unwrap();
        for t in &ticket_list {
            let deps = if t.depends_on.is_empty() {
                "—".to_string()
            } else {
                t.depends_on.join(", ")
            };
            writeln!(out, "{:<14} {:<12} {:<12} {}", t.id, t.phase, t.status, deps).unwrap();
        }
        writeln!(out).unwrap();

        // DAG Edges
        writeln!(out, "=== DAG Edges ===").unwrap();
        let mut edges: Vec<(String, String)> = Vec::new();
        for t in &ticket_list {
            let deps = self.dag.get_dependencies(&t.id);
            for dep in deps {
                edges.push((dep.clone(), t.id.clone()));
            }
        }
        edges.sort();
        if edges.is_empty() {
            writeln!(out, "(no edges)").unwrap();
        } else {
            for (from, to) in &edges {
                writeln!(out, "{} -> {}", from, to).unwrap();
            }
        }
        writeln!(out).unwrap();

        // DAG Stats
        writeln!(out, "=== DAG Stats ===").unwrap();
        let stats = self.dag.stats();
        writeln!(out, "total_tickets:       {}", stats.total_tickets).unwrap();
        writeln!(out, "done_tickets:        {}", stats.done_tickets).unwrap();
        writeln!(out, "ready_tickets:       {}", stats.ready_tickets).unwrap();
        writeln!(out, "in_progress_tickets: {}", stats.in_progress_tickets).unwrap();
        writeln!(out, "blocked_tickets:     {}", stats.blocked_tickets).unwrap();
        writeln!(out, "critical_path_length: {}", stats.critical_path_length).unwrap();
        writeln!(out).unwrap();

        // Threads
        writeln!(out, "=== Threads ===").unwrap();
        let mut thread_list: Vec<_> = self.threads.iter().collect();
        thread_list.sort_by(|a, b| a.0.cmp(b.0));
        if thread_list.is_empty() {
            writeln!(out, "(no threads)").unwrap();
        } else {
            writeln!(
                out,
                "{:<14} {:<6} {:<12} {:<10} {:<14} {}",
                "TICKET", "PANE", "PHASE", "STATUS", "STARTED_AGO", "PHASE_CHG_AGO"
            )
            .unwrap();
            let threshold = std::time::Duration::from_secs(self.config.stuck_threshold_secs);
            for (tid, thread) in &thread_list {
                let started_ago = now
                    .duration_since(thread.started_at)
                    .unwrap_or_default()
                    .as_secs();
                let phase_chg_ago = now
                    .duration_since(thread.last_phase_change)
                    .unwrap_or_default()
                    .as_secs();
                let health = thread.health(now, threshold);
                writeln!(
                    out,
                    "{:<14} #{:<4} {:<12} {:<10} {:<14} {} [health: {:?}]",
                    tid,
                    thread.pane_id,
                    thread.current_phase,
                    format!("{:?}", thread.status),
                    format!("{}s", started_ago),
                    format!("{}s", phase_chg_ago),
                    health
                )
                .unwrap();
            }
        }
        writeln!(out).unwrap();

        // Agent Slots
        writeln!(out, "=== Agent Slots ===").unwrap();
        if self.agent_slots.is_empty() {
            writeln!(out, "(no slots)").unwrap();
        } else {
            writeln!(out, "{:<8} {:<14} {}", "PANE", "TICKET", "HAS_SESSION").unwrap();
            for slot in &self.agent_slots {
                let ticket = slot.ticket_id.as_deref().unwrap_or("(idle)");
                writeln!(out, "#{:<7} {:<14} {}", slot.pane_id, ticket, slot.has_session).unwrap();
            }
        }
        writeln!(out).unwrap();

        // Health Status (last known)
        writeln!(out, "=== Last Known Health ===").unwrap();
        let mut health_list: Vec<_> = self.last_health.iter().collect();
        health_list.sort_by(|a, b| a.0.cmp(b.0));
        if health_list.is_empty() {
            writeln!(out, "(no health data)").unwrap();
        } else {
            for (tid, health) in &health_list {
                writeln!(out, "{:<14} {:?}", tid, health).unwrap();
            }
        }
        writeln!(out).unwrap();

        // Activity Log (last 50)
        writeln!(out, "=== Activity Log (last 50) ===").unwrap();
        let log_entries: Vec<_> = self.activity_log.iter().rev().take(50).collect();
        if log_entries.is_empty() {
            writeln!(out, "(no activity)").unwrap();
        } else {
            for (i, event) in log_entries.iter().enumerate() {
                writeln!(out, "{:>3}. {}", i + 1, Self::format_activity_event(event)).unwrap();
            }
        }

        out
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

        // Normal mode: 'D' (Shift+D) writes a state snapshot dump
        if key.bare_key == BareKey::Char('D') {
            let snapshot = self.format_snapshot();
            if let Err(e) = std::fs::write("/host/.lisa-state-dump.txt", &snapshot) {
                self.log_activity(ActivityEvent::Error {
                    message: format!("Failed to write state snapshot: {}", e),
                });
            } else {
                self.log_activity(ActivityEvent::Info {
                    message: "State snapshot written to .lisa-state-dump.txt".to_string(),
                });
            }
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
            self.log_activity(ActivityEvent::Info {
                message: "No tickets to mark done (all done or all have active agents)".to_string(),
            });
            return;
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

        // Release any slot occupied by this ticket and remove the thread
        if let Some(thread) = self.threads.get_mut(&tid) {
            thread.complete();
        }
        self.release_slot_for_ticket(&tid);
        self.threads.remove(&tid);

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

        // Signal directory for idle signal detection
        self.signal_dir = host.join(".lisa/signals");

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

        // Initial DAG build with startup diagnostics
        let commit_lock_path = PathBuf::from("/host/.ralph-commit.lock");
        let scan_result = match ticket::scan_tickets_with_diagnostics(&self.config.ticket_dir) {
            Ok(result) => result,
            Err(e) => {
                self.log_activity(ActivityEvent::Error {
                    message: format!("Failed to scan tickets: {}", e),
                });
                // Fall through with empty scan so diagnostics can still report config
                ticket::ScanResult {
                    tickets: Vec::new(),
                    errors: Vec::new(),
                }
            }
        };

        let dag_result = Dag::from_tickets(scan_result.tickets.clone());

        // Run startup diagnostics (pure function, no side effects)
        let diag_events = diagnostics::startup_diagnostics(
            &self.config,
            &scan_result,
            &dag_result,
            &commit_lock_path,
        );
        for event in diag_events {
            self.log_activity(event);
        }

        // Store the DAG (or keep default empty DAG on error)
        match dag_result {
            Ok(dag) => {
                self.last_phases = dag.tickets().map(|t| (t.id.clone(), t.phase)).collect();
                self.dag = dag;
            }
            Err(_) => {
                // DAG errors already logged by diagnostics
            }
        }

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
                self.arm_timer(POLL_INTERVAL_SECS);
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
                // Flush any deferred pane writes first
                self.flush_pending_pane_writes();

                // Only run the full poll cycle and re-arm if this is the last
                // pending timer. This prevents timer chain duplication when
                // short flush timers are set alongside the regular poll timer.
                if self.timer_fired() {
                    self.poll_tick();
                }
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
        let mut alerts: Vec<ui::HealthAlert> = self
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

        // Append idle-without-artifact alerts from signal detection
        for (ticket_id, detail) in &self.idle_alerts {
            alerts.push(ui::HealthAlert {
                ticket_id: ticket_id.clone(),
                alert_type: ui::AlertType::IdleWithoutArtifact,
                detail: detail.clone(),
                suggested_actions: vec![
                    "Check agent output".to_string(),
                    "Restart session".to_string(),
                ],
            });
        }

        let slots: Vec<ui::SlotInfo> = self
            .agent_slots
            .iter()
            .map(|s| ui::SlotInfo {
                pane_id: s.pane_id,
                ticket_id: s.ticket_id.clone(),
            })
            .collect();

        ui::PluginState {
            tickets,
            active_threads,
            parked_threads,
            activity_log,
            alerts,
            slots,
            current_time: Duration::from_secs(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
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
        ActivityEvent::Info { message } => ui::ActivityType::Info {
            ticket_id: String::new(),
            message: message.clone(),
        },
        ActivityEvent::PollSummary { .. } => return None,
        ActivityEvent::Warning { message } => ui::ActivityType::Warning {
            ticket_id: String::new(),
            message: message.clone(),
        },
        ActivityEvent::SessionLaunch {
            ticket_id,
            command,
            ..
        } => ui::ActivityType::Info {
            ticket_id: ticket_id.clone(),
            message: if command.len() > 120 {
                format!("Launch: {}...", &command[..120])
            } else {
                format!("Launch: {}", command)
            },
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

        assert!(cmd.starts_with("LISA_TICKET_ID=T-042-01 claude --dangerously-skip-permissions -p"));
        assert!(cmd.contains("docs/active/tickets/T-042-01.md"));
        assert!(cmd.contains("CLAUDE.md"));
        assert!(!cmd.ends_with('\r'), "Enter is now sent as a raw byte, not embedded in text");
    }

    #[test]
    fn test_build_claude_command_includes_env_var() {
        let ticket_dir = Path::new("docs/active/tickets");
        let cmd = build_claude_command(ticket_dir, "T-042-01");

        assert!(
            cmd.starts_with("LISA_TICKET_ID=T-042-01 "),
            "command should set LISA_TICKET_ID env var, got: {}",
            cmd
        );
    }

    #[test]
    fn test_build_claude_command_includes_rdspi_reference() {
        let ticket_dir = Path::new("docs/active/tickets");
        let cmd = build_claude_command(ticket_dir, "T-001");

        assert!(
            cmd.contains("docs/knowledge/rdspi-workflow.md"),
            "command should reference RDSPI workflow, got: {}",
            cmd
        );
    }

    #[test]
    fn test_strip_host_prefix_with_prefix() {
        let path = Path::new("/host/docs/active/tickets");
        assert_eq!(strip_host_prefix(path), PathBuf::from("docs/active/tickets"));
    }

    #[test]
    fn test_strip_host_prefix_without_prefix() {
        let path = Path::new("docs/active/tickets");
        assert_eq!(strip_host_prefix(path), PathBuf::from("docs/active/tickets"));
    }

    #[test]
    fn test_strip_host_prefix_just_host() {
        let path = Path::new("/host/");
        assert_eq!(strip_host_prefix(path), PathBuf::from(""));
    }

    #[test]
    fn test_strip_host_prefix_nested_host() {
        let path = Path::new("/host/host/nested");
        assert_eq!(strip_host_prefix(path), PathBuf::from("host/nested"));
    }

    #[test]
    fn test_strip_host_prefix_absolute_non_host() {
        let path = Path::new("/other/docs/active/tickets");
        assert_eq!(
            strip_host_prefix(path),
            PathBuf::from("/other/docs/active/tickets")
        );
    }

    #[test]
    fn test_session_launch_event_to_ui() {
        let event = ActivityEvent::SessionLaunch {
            ticket_id: "T-001".to_string(),
            pane_id: 42,
            command: "claude --dangerously-skip-permissions \"Read the ticket...\"".to_string(),
        };
        let entry = activity_event_to_ui_entry(&event);
        assert!(entry.is_some(), "SessionLaunch should produce a UI entry");
        match &entry.unwrap().activity {
            ui::ActivityType::Info { ticket_id, message } => {
                assert_eq!(ticket_id, "T-001");
                assert!(message.contains("Launch:"));
                assert!(message.contains("claude"));
            }
            other => panic!("Expected Info, got {:?}", other),
        }
    }

    #[test]
    fn test_session_launch_event_to_ui_truncates_long_command() {
        let long_command = "x".repeat(200);
        let event = ActivityEvent::SessionLaunch {
            ticket_id: "T-002".to_string(),
            pane_id: 7,
            command: long_command,
        };
        let entry = activity_event_to_ui_entry(&event).unwrap();
        match &entry.activity {
            ui::ActivityType::Info { message, .. } => {
                assert!(
                    message.len() < 200,
                    "Long command should be truncated, got {} chars",
                    message.len()
                );
                assert!(message.ends_with("..."));
            }
            other => panic!("Expected Info, got {:?}", other),
        }
    }

    #[test]
    fn test_ticket_prompt_content() {
        let dir = Path::new("docs/active/tickets");
        let prompt = ticket_prompt(dir, "T-024-03");

        assert!(prompt.contains("docs/active/tickets/T-024-03.md"));
        assert!(prompt.contains("CLAUDE.md"));
        assert!(prompt.contains("docs/knowledge/rdspi-workflow.md"));
        assert!(prompt.contains("current phase"));
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

        // Simulate T-001 had a running thread that completed and was cleaned up
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
        state.threads.remove("T-001");

        // Verify: slot is now idle but retains its Claude Code session
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert!(
            state.agent_slots[0].has_session,
            "has_session should stay true — Claude Code is still running"
        );
        assert!(state.find_idle_slot().is_some());

        // Verify: thread is removed from map
        assert!(!state.threads.contains_key("T-001"));

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

    #[test]
    fn test_release_slot_logs_success() {
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 7,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
        });

        state.release_slot_for_ticket(&"T-001".to_string());

        // Slot should be released
        assert!(state.agent_slots[0].ticket_id.is_none());

        // Info log should mention pane and ticket
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Info { message }
            if message.contains("Released slot #7") && message.contains("T-001")
        )));
    }

    #[test]
    fn test_release_slot_logs_not_found() {
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 7,
            ticket_id: None,
            has_session: false,
        });

        state.release_slot_for_ticket(&"T-MISSING".to_string());

        // Info log should indicate not found
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Info { message }
            if message.contains("No slot found") && message.contains("T-MISSING")
        )));
    }

    #[test]
    fn test_info_event_to_ui_entry() {
        let entry = activity_event_to_ui_entry(&ActivityEvent::Info {
            message: "test info message".to_string(),
        });
        assert!(entry.is_some());
        match &entry.unwrap().activity {
            ui::ActivityType::Info { message, .. } => {
                assert_eq!(message, "test info message");
            }
            other => panic!("Expected Info, got {:?}", other),
        }
    }

    #[test]
    fn test_poll_summary_event_filtered() {
        let entry = activity_event_to_ui_entry(&ActivityEvent::PollSummary {
            ready: 3,
            running: 2,
            idle_slots: 1,
        });
        assert!(entry.is_none(), "PollSummary should be filtered from UI");
    }

    #[test]
    fn test_done_ticket_detected_on_first_poll() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: already-done\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n\nDone\n",
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

        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
        });

        // First rebuild with empty last_phases — done ticket should be detected
        let changed = state.rebuild_dag();
        assert!(changed, "First rebuild with done ticket should detect a change");

        // Run the done-ticket detection logic (same as poll_tick)
        let done_tickets: Vec<TicketId> = state
            .threads
            .iter()
            .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
            .filter(|(tid, _)| {
                state.dag.get_ticket(tid).map(|t| t.phase == Phase::Done).unwrap_or(false)
            })
            .map(|(tid, _)| tid.clone())
            .collect();

        for ticket_id in &done_tickets {
            if let Some(t) = state.threads.get_mut(ticket_id) {
                t.complete();
            }
            state.release_slot_for_ticket(ticket_id);
            state.threads.remove(ticket_id);
        }

        // Thread should be removed from the map after completion
        assert!(!state.threads.contains_key("T-001"));
        assert!(state.agent_slots[0].ticket_id.is_none());
    }

    #[test]
    fn test_done_ticket_detected_between_polls() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: transitioned\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n\nDone\n",
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

        // Last poll saw T-001 at Research
        state.last_phases.insert("T-001".to_string(), Phase::Research);

        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
        });

        let changed = state.rebuild_dag();
        assert!(changed, "Phase change Research -> Done should be detected");

        let done_tickets: Vec<TicketId> = state
            .threads
            .iter()
            .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
            .filter(|(tid, _)| {
                state.dag.get_ticket(tid).map(|t| t.phase == Phase::Done).unwrap_or(false)
            })
            .map(|(tid, _)| tid.clone())
            .collect();

        for ticket_id in &done_tickets {
            if let Some(t) = state.threads.get_mut(ticket_id) {
                t.complete();
            }
            state.release_slot_for_ticket(ticket_id);
            state.threads.remove(ticket_id);
        }

        // Thread should be removed from the map after completion
        assert!(!state.threads.contains_key("T-001"));
        assert!(state.agent_slots[0].ticket_id.is_none());
    }

    #[test]
    fn test_sweep_stale_slots_releases_done_ticket() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: stale-slot\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n\nDone\n",
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

        // Slot assigned to done ticket, but no thread exists
        state.agent_slots.push(AgentSlot {
            pane_id: 5,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
        });
        assert!(!state.threads.contains_key("T-001"));

        state.sweep_stale_slots();

        assert!(state.agent_slots[0].ticket_id.is_none(), "Stale slot should be released");
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Error { message }
            if message.contains("stale") && message.contains("T-001") && message.contains("Slot #5")
        )));
    }

    #[test]
    fn test_completed_thread_removed_dependent_scheduled() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        // T-001: done
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: done\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n\nDone\n",
        ).unwrap();

        // T-002: ready, depends on T-001
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: next\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-001]\n---\n\nNext\n",
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

        // Running thread for T-001 (simulates agent still tracked)
        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
        });

        // Run the done-ticket detection logic (mirrors poll_tick)
        let done_tickets: Vec<TicketId> = state
            .threads
            .iter()
            .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
            .filter(|(tid, _)| {
                state.dag.get_ticket(tid).map(|t| t.phase == Phase::Done).unwrap_or(false)
            })
            .map(|(tid, _)| tid.clone())
            .collect();

        for ticket_id in &done_tickets {
            if let Some(t) = state.threads.get_mut(ticket_id) {
                t.complete();
            }
            state.release_slot_for_ticket(ticket_id);
            state.threads.remove(ticket_id);
        }

        // T-001 thread removed, slot released
        assert!(!state.threads.contains_key("T-001"));
        assert!(state.agent_slots[0].ticket_id.is_none());

        // T-002 is ready and has no thread blocking it
        let ready = state.dag.get_ready_tickets();
        assert!(ready.contains(&"T-002".to_string()));
        assert!(!state.threads.contains_key("T-002"));
    }

    #[test]
    fn test_defensive_guard_removes_completed_thread() {
        use lisa_core::types::{Thread, ThreadStatus};

        let mut state = State::default();

        // Insert a stale Completed thread
        let mut thread = Thread::new("T-001", 1);
        thread.complete();
        state.threads.insert("T-001".to_string(), thread);

        // Simulate the defensive guard logic from schedule_ready_tickets
        let ticket_id = "T-001".to_string();
        let is_completed = state
            .threads
            .get(&ticket_id)
            .map(|t| t.status == ThreadStatus::Completed)
            .unwrap_or(false);

        assert!(is_completed, "Thread should be Completed");

        if is_completed {
            state.threads.remove(&ticket_id);
        }

        // Thread should be removed, allowing rescheduling
        assert!(!state.threads.contains_key("T-001"));
    }

    #[test]
    fn test_audit_threads_removes_done_ticket_thread() {
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
            config: PluginConfig {
                ticket_dir: tickets_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Thread for a done ticket (should be cleaned up)
        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
        });

        state.audit_threads();

        // Thread removed
        assert!(!state.threads.contains_key("T-001"));
        // Slot released
        assert!(state.agent_slots[0].ticket_id.is_none());
        // Warning logged
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Error { message }
            if message.contains("Orphaned") && message.contains("T-001")
        )));
    }

    #[test]
    fn test_audit_threads_removes_missing_ticket_thread() {
        use lisa_core::types::Thread;

        // Empty DAG — no tickets at all
        let mut state = State::default();

        // Thread for a ticket that doesn't exist in the DAG
        let thread = Thread::new("T-GHOST", 1);
        state.threads.insert("T-GHOST".to_string(), thread);

        state.audit_threads();

        // Thread removed (ticket not in DAG)
        assert!(!state.threads.contains_key("T-GHOST"));
        // Warning logged
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Error { message }
            if message.contains("Orphaned") && message.contains("T-GHOST")
        )));
    }

    #[test]
    fn test_audit_threads_keeps_active_thread() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: active\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n\nActive\n",
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

        // Running thread for an active ticket — should NOT be removed
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-001".to_string(), thread);

        state.audit_threads();

        // Thread should remain
        assert!(state.threads.contains_key("T-001"));
        // No warnings
        assert!(state.activity_log.is_empty());
    }

    #[test]
    fn test_mark_done_removes_thread() {
        // Tests the thread removal logic from mark_ticket_done without calling
        // schedule_ready_tickets() (which uses zellij host functions).
        // We replicate the key mark_ticket_done operations manually.
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: to-mark\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
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

        // Running thread for the ticket
        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
        });

        // Replicate the key mark_ticket_done operations (without schedule_ready_tickets)
        let tid = "T-001".to_string();
        let file_path = state.dag.get_ticket(&tid).map(|t| t.file_path.clone()).unwrap();
        lisa_core::ticket::update_ticket_phase(&file_path, Phase::Done).unwrap();

        if let Some(thread) = state.threads.get_mut(&tid) {
            thread.complete();
        }
        state.release_slot_for_ticket(&tid);
        state.threads.remove(&tid);

        // Thread should be removed
        assert!(!state.threads.contains_key("T-001"));
        // Slot should be released
        assert!(state.agent_slots[0].ticket_id.is_none());
        // Ticket file updated
        let content = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(content.contains("phase: done"));
    }

    #[test]
    fn test_format_snapshot_contains_all_sections() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let work_dir = dir.path().join("work");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::create_dir_all(&work_dir).unwrap();

        // Create tickets: T-001 done, T-002 depends on T-001, T-003 depends on T-002
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: first\ntype: task\nstatus: open\npriority: high\nphase: done\n---\n\nDone.\n",
        ).unwrap();
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: second\ntype: task\nstatus: open\npriority: medium\nphase: research\ndepends_on: [T-001]\n---\n\nActive.\n",
        ).unwrap();
        fs::write(
            tickets_dir.join("T-003.md"),
            "---\nid: T-003\ntitle: third\ntype: task\nstatus: open\npriority: low\nphase: ready\ndepends_on: [T-002]\n---\n\nBlocked.\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir,
                ..PluginConfig::new()
            },
            initialized: true,
            permissions_granted: true,
            ..State::default()
        };

        // Add threads
        let mut thread = Thread::new("T-002", 5);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-002".to_string(), thread);

        // Add agent slots
        state.agent_slots.push(AgentSlot {
            pane_id: 5,
            ticket_id: Some("T-002".to_string()),
            has_session: true,
        });
        state.agent_slots.push(AgentSlot {
            pane_id: 6,
            ticket_id: None,
            has_session: false,
        });

        // Add health data
        state.last_health.insert("T-002".to_string(), lisa_core::types::HealthStatus::Healthy);

        // Add activity events
        state.log_activity(ActivityEvent::PluginStarted);
        state.log_activity(ActivityEvent::Info {
            message: "test info".to_string(),
        });

        let snapshot = state.format_snapshot();

        // Check all section headers
        assert!(snapshot.contains("=== Lisa State Snapshot ==="), "Missing header");
        assert!(snapshot.contains("=== Config ==="), "Missing config section");
        assert!(snapshot.contains("=== Plugin Status ==="), "Missing plugin status");
        assert!(snapshot.contains("=== Tickets ==="), "Missing tickets section");
        assert!(snapshot.contains("=== DAG Edges ==="), "Missing edges section");
        assert!(snapshot.contains("=== DAG Stats ==="), "Missing stats section");
        assert!(snapshot.contains("=== Threads ==="), "Missing threads section");
        assert!(snapshot.contains("=== Agent Slots ==="), "Missing slots section");
        assert!(snapshot.contains("=== Last Known Health ==="), "Missing health section");
        assert!(snapshot.contains("=== Activity Log (last 50) ==="), "Missing activity log");
    }

    #[test]
    fn test_format_snapshot_ticket_data() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: first\ntype: task\nstatus: open\npriority: high\nphase: done\n---\n\nDone.\n",
        ).unwrap();
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: second\ntype: task\nstatus: open\npriority: medium\nphase: research\ndepends_on: [T-001]\n---\n\nActive.\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        let snapshot = state.format_snapshot();

        // Ticket IDs and phases
        assert!(snapshot.contains("T-001"), "Missing T-001");
        assert!(snapshot.contains("T-002"), "Missing T-002");
        assert!(snapshot.contains("done"), "Missing done phase");
        assert!(snapshot.contains("research"), "Missing research phase");

        // DAG edge
        assert!(snapshot.contains("T-001 -> T-002"), "Missing edge T-001 -> T-002");

        // DAG stats
        assert!(snapshot.contains("total_tickets:       2"), "Wrong total tickets");
        assert!(snapshot.contains("done_tickets:        1"), "Wrong done tickets");
    }

    #[test]
    fn test_format_snapshot_thread_and_slot_data() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: first\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n\nActive.\n",
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

        // Thread
        let mut thread = Thread::new("T-001", 42);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-001".to_string(), thread);

        // Slots
        state.agent_slots.push(AgentSlot {
            pane_id: 42,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
        });
        state.agent_slots.push(AgentSlot {
            pane_id: 43,
            ticket_id: None,
            has_session: false,
        });

        let snapshot = state.format_snapshot();

        // Thread data
        assert!(snapshot.contains("T-001"), "Thread ticket_id missing");
        assert!(snapshot.contains("#42"), "Thread pane_id missing");
        assert!(snapshot.contains("Running"), "Thread status missing");

        // Slot data
        assert!(snapshot.contains("(idle)"), "Idle slot missing");
        assert!(snapshot.contains("true"), "has_session=true missing");
        assert!(snapshot.contains("false"), "has_session=false missing");
    }

    #[test]
    fn test_format_snapshot_activity_log_limit() {
        let mut state = State::default();

        // Add 100 activity events
        for i in 0..100 {
            state.log_activity(ActivityEvent::Info {
                message: format!("event-{}", i),
            });
        }

        let snapshot = state.format_snapshot();

        // Should contain the last 50 events (50-99), not the first 50
        assert!(snapshot.contains("event-99"), "Latest event missing");
        assert!(snapshot.contains("event-50"), "Event at boundary missing");
        assert!(!snapshot.contains("event-49"), "Old event should not appear");

        // Should be numbered 1-50
        assert!(snapshot.contains("  1. Info: event-99"), "First entry should be event-99");
    }

    #[test]
    fn test_format_activity_event_variants() {
        let cases = vec![
            (ActivityEvent::PluginStarted, "PluginStarted"),
            (
                ActivityEvent::ThreadSpawned {
                    ticket_id: "T-001".to_string(),
                    pane_id: 5,
                },
                "ThreadSpawned: T-001 pane=#5",
            ),
            (
                ActivityEvent::Error {
                    message: "bad thing".to_string(),
                },
                "Error: bad thing",
            ),
            (
                ActivityEvent::TicketPhaseChanged {
                    ticket_id: "T-002".to_string(),
                    old_phase: Phase::Research,
                    new_phase: Phase::Design,
                },
                "TicketPhaseChanged: T-002 research -> design",
            ),
        ];

        for (event, expected) in cases {
            let formatted = State::format_activity_event(&event);
            assert_eq!(formatted, expected, "Mismatch for {:?}", event);
        }
    }

    // =========================================================================
    // Idle signal tests
    // =========================================================================

    #[test]
    fn test_idle_signal_implement_advances_to_review() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // Create ticket file in implement phase
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
        ).unwrap();

        // Create signal directory with idle signal
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("T-001.idle"), "2025-01-01T00:00:00Z").unwrap();

        // Build state
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: dir.path().join("work"),
                ..PluginConfig::new()
            },
            signal_dir,
            ..State::default()
        };

        // Add running thread in implement phase
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        state.threads.insert("T-001".to_string(), thread);

        // Run idle signal check
        state.check_idle_signals();

        // Verify: thread advanced to Review and parked
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Parked);

        // Verify: signal file deleted
        assert!(!state.signal_dir.join("T-001.idle").exists());

        // Verify: ticket file updated
        let updated = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(updated.contains("phase: review"));

        // Verify: activity log
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::PhaseCompleted { ticket_id, phase }
            if ticket_id == "T-001" && *phase == Phase::Implement
        )));

        // Verify: no idle alerts
        assert!(state.idle_alerts.is_empty());
    }

    #[test]
    fn test_idle_signal_research_with_artifact_advances() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // Create ticket file in research phase
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

        // Create signal directory with idle signal
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("T-001.idle"), "2025-01-01T00:00:00Z").unwrap();

        // Build state
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                ..PluginConfig::new()
            },
            signal_dir,
            ..State::default()
        };

        // Add running thread in research phase
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-001".to_string(), thread);

        state.check_idle_signals();

        // Verify: advanced to Design, still running
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Design);
        assert_eq!(thread.status, ThreadStatus::Running);

        // Verify: signal deleted
        assert!(!state.signal_dir.join("T-001.idle").exists());

        // Verify: ticket file updated
        let updated = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(updated.contains("phase: design"));

        // Verify: no idle alerts
        assert!(state.idle_alerts.is_empty());
    }

    #[test]
    fn test_idle_signal_research_without_artifact_alerts() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // Create ticket file in research phase
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n\nBody\n",
        ).unwrap();

        // No artifact — work dir empty
        let work_dir = dir.path().join("work");
        fs::create_dir_all(&work_dir).unwrap();

        // Create signal directory with idle signal
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("T-001.idle"), "2025-01-01T00:00:00Z").unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                ..PluginConfig::new()
            },
            signal_dir,
            ..State::default()
        };

        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-001".to_string(), thread);

        state.check_idle_signals();

        // Verify: phase NOT advanced (still research)
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Research);

        // Verify: signal deleted
        assert!(!state.signal_dir.join("T-001.idle").exists());

        // Verify: ticket file NOT updated
        let updated = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(updated.contains("phase: research"));

        // Verify: idle alert generated
        assert_eq!(state.idle_alerts.len(), 1);
        assert_eq!(state.idle_alerts[0].0, "T-001");
        assert!(state.idle_alerts[0].1.contains("research.md not found"));

        // Verify: warning in activity log
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Warning { message }
            if message.contains("T-001") && message.contains("research.md")
        )));
    }

    #[test]
    fn test_idle_signal_no_thread_ignored() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // Create ticket file
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n\nBody\n",
        ).unwrap();

        // Create signal for a ticket that has NO thread
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("T-001.idle"), "2025-01-01T00:00:00Z").unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir: dir.path().join("work"),
                ..PluginConfig::new()
            },
            signal_dir,
            ..State::default()
        };
        // No threads added

        state.check_idle_signals();

        // Signal file should still be cleaned up
        assert!(!state.signal_dir.join("T-001.idle").exists());

        // No alerts
        assert!(state.idle_alerts.is_empty());
    }

    #[test]
    fn test_idle_signal_nonrunning_thread_ignored() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
        ).unwrap();

        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("T-001.idle"), "2025-01-01T00:00:00Z").unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir: dir.path().join("work"),
                ..PluginConfig::new()
            },
            signal_dir,
            ..State::default()
        };

        // Add a PARKED thread (not running)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        thread.park();
        state.threads.insert("T-001".to_string(), thread);

        state.check_idle_signals();

        // Signal cleaned up
        assert!(!state.signal_dir.join("T-001.idle").exists());

        // Thread still parked, not advanced
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(
            thread.status,
            lisa_core::types::ThreadStatus::Parked
        );
    }

    #[test]
    fn test_idle_signal_missing_dir_no_panic() {
        let dir = tempfile::tempdir().unwrap();

        let mut state = State {
            signal_dir: dir.path().join("nonexistent/signals"),
            ..State::default()
        };

        // Should not panic
        state.check_idle_signals();
        assert!(state.idle_alerts.is_empty());
    }

    #[test]
    fn test_to_ui_state_includes_idle_alerts() {
        let mut state = State::default();
        state.idle_alerts.push((
            "T-001".to_string(),
            "Agent idle in research phase but research.md not found".to_string(),
        ));

        let ui_state = state.to_ui_state();

        assert!(ui_state.alerts.iter().any(|a| {
            a.ticket_id == "T-001"
                && a.alert_type == ui::AlertType::IdleWithoutArtifact
                && a.detail.contains("research.md")
        }));
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
