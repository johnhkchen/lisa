//! UI/Dashboard module for the Lisa/Ralph Zellij plugin.
//!
//! This module provides the dashboard view that shows:
//! - DAG with dependency edges and ticket status
//! - Active threads: which ticket, which phase, how long running
//! - Parked threads: waiting for human review, with artifact path
//! - Recent activity log: phase completions, commits, errors
//! - Quick-jump to any thread's pane
//!
//! Replaces `just dag-status`, `just ralph-status`, and `just ralph-logs` with a single live view.

use std::collections::HashMap;
use std::time::Duration;

use crate::dag::Dag;
use crate::types::{self, ActivityEvent, PluginConfig, Thread, TicketId};

/// ANSI color codes for terminal output
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BG_BLUE: &str = "\x1b[44m";
}

use colors::*;

// =============================================================================
// UI Types - Self-contained types for the UI layer
// =============================================================================

/// Status of a ticket in the workflow (UI representation)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketStatus {
    /// Ready to be picked up
    Ready,
    /// Currently being worked on
    InProgress,
    /// Waiting for human review
    WaitingReview,
    /// Blocked by dependencies
    Blocked,
    /// Completed
    Done,
}

/// Phase in the RDSPI workflow (UI representation)
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Phase {
    Ready,
    Research,
    Design,
    Structure,
    Plan,
    Implement,
    Review,
    Done,
}

impl Phase {
    /// Get the short display name for the phase (3 chars)
    pub fn short_name(&self) -> &'static str {
        match self {
            Phase::Ready => "RDY",
            Phase::Research => "RES",
            Phase::Design => "DES",
            Phase::Structure => "STR",
            Phase::Plan => "PLN",
            Phase::Implement => "IMP",
            Phase::Review => "REV",
            Phase::Done => "DON",
        }
    }

    /// Get the full display name for the phase
    pub fn full_name(&self) -> &'static str {
        match self {
            Phase::Ready => "Ready",
            Phase::Research => "Research",
            Phase::Design => "Design",
            Phase::Structure => "Structure",
            Phase::Plan => "Plan",
            Phase::Implement => "Implement",
            Phase::Review => "Review",
            Phase::Done => "Done",
        }
    }

    /// Get ANSI color code for the phase
    pub fn color_code(&self) -> &'static str {
        match self {
            Phase::Ready => DIM,
            Phase::Research => CYAN,
            Phase::Design => MAGENTA,
            Phase::Structure => YELLOW,
            Phase::Plan => BLUE,
            Phase::Implement => GREEN,
            Phase::Review => BRIGHT_YELLOW,
            Phase::Done => BRIGHT_GREEN,
        }
    }

    /// Get the indicator symbol for the phase
    pub fn indicator(&self) -> &'static str {
        match self {
            Phase::Ready => "○",
            Phase::Research => "◐",
            Phase::Design => "◑",
            Phase::Structure => "◒",
            Phase::Plan => "◓",
            Phase::Implement => "●",
            Phase::Review => "◎",
            Phase::Done => "✓",
        }
    }
}

/// Represents a ticket node in the DAG for UI display
#[derive(Debug, Clone)]
pub struct TicketNode {
    pub id: String,
    pub title: String,
    pub phase: Phase,
    pub status: TicketStatus,
    pub depends_on: Vec<String>,
    pub blocks: Vec<String>,
}

/// Represents an active thread working on a ticket
#[derive(Debug, Clone)]
pub struct ActiveThread {
    pub ticket_id: String,
    pub phase: Phase,
    pub started_at: Duration,
    pub pane_id: u32,
}

/// Represents a parked thread waiting for review
#[derive(Debug, Clone)]
pub struct ParkedThread {
    pub ticket_id: String,
    pub phase: Phase,
    pub artifact_path: String,
    pub parked_at: Duration,
    pub pane_id: u32,
}

/// Activity log entry types
#[derive(Debug, Clone)]
pub enum ActivityType {
    PhaseCompleted { ticket_id: String, phase: Phase },
    Commit { ticket_id: String, message: String },
    Error { ticket_id: String, message: String },
    ThreadStarted { ticket_id: String, phase: Phase },
    ThreadParked { ticket_id: String, phase: Phase },
}

/// A single activity log entry
#[derive(Debug, Clone)]
pub struct ActivityEntry {
    pub timestamp: Duration,
    pub activity: ActivityType,
}

/// The complete plugin state for rendering
#[derive(Debug, Clone)]
pub struct PluginState {
    pub tickets: Vec<TicketNode>,
    pub active_threads: Vec<ActiveThread>,
    pub parked_threads: Vec<ParkedThread>,
    pub activity_log: Vec<ActivityEntry>,
    pub current_time: Duration,
    pub selected_ticket: Option<String>,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            tickets: Vec::new(),
            active_threads: Vec::new(),
            parked_threads: Vec::new(),
            activity_log: Vec::new(),
            current_time: Duration::ZERO,
            selected_ticket: None,
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Format a duration as a human-readable string (e.g., "2m 30s" or "1h 5m")
fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Format time elapsed since a Unix timestamp
fn format_time_since(timestamp: Duration, current_time: Duration) -> String {
    let elapsed = current_time.saturating_sub(timestamp);
    format_duration(elapsed)
}

/// Get status indicator with color for ticket status
fn status_indicator(status: &TicketStatus) -> String {
    match status {
        TicketStatus::Ready => format!("{}[RDY]{}", CYAN, RESET),
        TicketStatus::InProgress => format!("{}[WRK]{}", GREEN, RESET),
        TicketStatus::WaitingReview => format!("{}[REV]{}", BRIGHT_YELLOW, RESET),
        TicketStatus::Blocked => format!("{}[BLK]{}", RED, RESET),
        TicketStatus::Done => format!("{}[DON]{}", BRIGHT_GREEN, RESET),
    }
}

/// Render a horizontal separator line
fn render_separator(width: usize) -> String {
    format!("{}{}{}", DIM, "─".repeat(width.min(80)), RESET)
}

// =============================================================================
// DAG Rendering
// =============================================================================

/// Compute DAG layers for visualization (topological sort into layers)
fn compute_dag_layers(tickets: &[TicketNode]) -> Vec<Vec<usize>> {
    if tickets.is_empty() {
        return Vec::new();
    }

    // Build a map of ticket ID to index for quick lookup
    let id_to_idx: HashMap<&str, usize> = tickets
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.as_str(), i))
        .collect();

    let mut layers: Vec<Vec<usize>> = Vec::new();
    let mut placed: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut remaining: Vec<usize> = (0..tickets.len()).collect();

    while !remaining.is_empty() {
        // Find all tickets whose dependencies are satisfied
        let mut layer = Vec::new();
        for &idx in &remaining {
            let ticket = &tickets[idx];
            let deps_satisfied = ticket.depends_on.iter().all(|dep| {
                if let Some(&dep_idx) = id_to_idx.get(dep.as_str()) {
                    placed.contains(&dep_idx)
                } else {
                    // External dependency or doesn't exist, consider satisfied
                    true
                }
            });
            if deps_satisfied {
                layer.push(idx);
            }
        }

        // If no tickets can be placed, we have a cycle - just place remaining
        if layer.is_empty() && !remaining.is_empty() {
            layer = remaining.clone();
        }

        // Mark as placed and remove from remaining
        for &idx in &layer {
            placed.insert(idx);
        }
        remaining.retain(|idx| !layer.contains(idx));

        if !layer.is_empty() {
            layers.push(layer);
        }
    }

    layers
}

/// Render the DAG as ASCII art
///
/// Shows tickets as nodes with their status and phase,
/// organized in topological layers.
fn render_dag(state: &PluginState, output: &mut Vec<String>) {
    output.push(format!("{}{}=== DAG ==={}", BOLD, CYAN, RESET));
    output.push(String::new());

    if state.tickets.is_empty() {
        output.push(format!("{}(no tickets){}", DIM, RESET));
        return;
    }

    // Group tickets into layers based on dependency depth
    let layers = compute_dag_layers(&state.tickets);

    // Render each layer
    for (layer_idx, layer) in layers.iter().enumerate() {
        if layer_idx > 0 {
            // Draw connector lines between layers
            let mut connector = String::new();
            for (i, _) in layer.iter().enumerate() {
                if i > 0 {
                    connector.push_str("    ");
                }
                connector.push_str(&format!("{}  |  {}", DIM, RESET));
            }
            if !connector.trim().is_empty() {
                output.push(format!("    {}", connector));
            }
        }

        // Render tickets in this layer
        let mut ticket_line = String::from("    ");
        for (i, &ticket_idx) in layer.iter().enumerate() {
            if i > 0 {
                ticket_line.push_str("  ");
            }
            let ticket = &state.tickets[ticket_idx];
            let phase_color = ticket.phase.color_code();
            let indicator = ticket.phase.indicator();
            let status = status_indicator(&ticket.status);

            // Highlight selected ticket
            let highlight = if state.selected_ticket.as_ref() == Some(&ticket.id) {
                format!("{}> ", BOLD)
            } else {
                String::new()
            };
            let highlight_end = if state.selected_ticket.as_ref() == Some(&ticket.id) {
                format!(" <{}", RESET)
            } else {
                String::new()
            };

            ticket_line.push_str(&format!(
                "{}[{}{}{} {:<10} {}]{}",
                highlight,
                phase_color,
                indicator,
                RESET,
                ticket.id,
                status,
                highlight_end
            ));
        }
        output.push(ticket_line);
    }

    // Legend
    output.push(String::new());
    output.push(format!(
        "{}Phases: {} Rdy {} Res {} Des {} Str {} Pln {} Imp {} Rev {} Don{}",
        DIM,
        Phase::Ready.indicator(),
        Phase::Research.indicator(),
        Phase::Design.indicator(),
        Phase::Structure.indicator(),
        Phase::Plan.indicator(),
        Phase::Implement.indicator(),
        Phase::Review.indicator(),
        Phase::Done.indicator(),
        RESET
    ));
}

// =============================================================================
// Thread Status Rendering
// =============================================================================

/// Render the active threads table
fn render_active_threads(state: &PluginState, output: &mut Vec<String>) {
    output.push(format!("{}{}=== Active Threads ==={}", BOLD, GREEN, RESET));
    output.push(String::new());

    if state.active_threads.is_empty() {
        output.push(format!("{}(no active threads){}", DIM, RESET));
        return;
    }

    // Header
    output.push(format!(
        "{}{:<12} {:<10} {:<10} {:<6}{}",
        DIM, "TICKET", "PHASE", "RUNNING", "PANE", RESET
    ));
    output.push(format!("{}{}{}", DIM, "-".repeat(42), RESET));

    for thread in &state.active_threads {
        let elapsed = format_time_since(thread.started_at, state.current_time);
        let phase_color = thread.phase.color_code();

        output.push(format!(
            "{:<12} {}{:<10}{} {:<10} #{}",
            thread.ticket_id,
            phase_color,
            thread.phase.full_name(),
            RESET,
            elapsed,
            thread.pane_id
        ));
    }
}

/// Render the parked threads table (awaiting review)
fn render_parked_threads(state: &PluginState, output: &mut Vec<String>) {
    output.push(format!(
        "{}{}=== Parked (Awaiting Review) ==={}",
        BOLD, YELLOW, RESET
    ));
    output.push(String::new());

    if state.parked_threads.is_empty() {
        output.push(format!("{}(no parked threads){}", DIM, RESET));
        return;
    }

    // Header
    output.push(format!(
        "{}{:<12} {:<10} {:<10} {:<30}{}",
        DIM, "TICKET", "PHASE", "WAITING", "ARTIFACT", RESET
    ));
    output.push(format!("{}{}{}", DIM, "-".repeat(66), RESET));

    for thread in &state.parked_threads {
        let elapsed = format_time_since(thread.parked_at, state.current_time);
        let phase_color = thread.phase.color_code();

        // Truncate artifact path if too long
        let artifact_display = if thread.artifact_path.len() > 28 {
            format!("...{}", &thread.artifact_path[thread.artifact_path.len() - 25..])
        } else {
            thread.artifact_path.clone()
        };

        output.push(format!(
            "{:<12} {}{:<10}{} {:<10} {} [#{}]",
            thread.ticket_id,
            phase_color,
            thread.phase.full_name(),
            RESET,
            elapsed,
            artifact_display,
            thread.pane_id
        ));
    }
}

// =============================================================================
// Activity Log Rendering
// =============================================================================

/// Render the activity log
fn render_activity_log(state: &PluginState, max_entries: usize, output: &mut Vec<String>) {
    output.push(format!("{}{}=== Recent Activity ==={}", BOLD, BLUE, RESET));
    output.push(String::new());

    if state.activity_log.is_empty() {
        output.push(format!("{}(no recent activity){}", DIM, RESET));
        return;
    }

    // Show most recent entries (reversed, newest first)
    let entries: Vec<_> = state.activity_log.iter().rev().take(max_entries).collect();

    for entry in entries {
        let time_ago = format_time_since(entry.timestamp, state.current_time);

        let (icon, color, message) = match &entry.activity {
            ActivityType::PhaseCompleted { ticket_id, phase } => (
                "✓",
                BRIGHT_GREEN,
                format!("{} completed {}", ticket_id, phase.full_name()),
            ),
            ActivityType::Commit { ticket_id, message } => {
                let msg = if message.len() > 40 {
                    format!("{}...", &message[..37])
                } else {
                    message.clone()
                };
                ("◆", CYAN, format!("{} commit: {}", ticket_id, msg))
            }
            ActivityType::Error { ticket_id, message } => {
                let msg = if message.len() > 40 {
                    format!("{}...", &message[..37])
                } else {
                    message.clone()
                };
                let prefix = if ticket_id.is_empty() {
                    String::new()
                } else {
                    format!("{} ", ticket_id)
                };
                ("✗", RED, format!("{}error: {}", prefix, msg))
            }
            ActivityType::ThreadStarted { ticket_id, phase } => (
                "▶",
                GREEN,
                format!("{} started {}", ticket_id, phase.full_name()),
            ),
            ActivityType::ThreadParked { ticket_id, phase } => (
                "⏸",
                YELLOW,
                format!("{} parked at {}", ticket_id, phase.full_name()),
            ),
        };

        output.push(format!(
            "{}{}{} {:<12} {}{}{}",
            color, icon, RESET, time_ago, color, message, RESET
        ));
    }
}

// =============================================================================
// Quick Jump Section
// =============================================================================

/// Render the quick-jump help section
fn render_quick_jump(state: &PluginState, output: &mut Vec<String>) {
    output.push(format!("{}{}=== Quick Jump ==={}", BOLD, WHITE, RESET));
    output.push(String::new());

    // Collect all panes from active and parked threads
    let mut panes: Vec<(u32, &str, &Phase)> = Vec::new();

    for thread in &state.active_threads {
        panes.push((thread.pane_id, &thread.ticket_id, &thread.phase));
    }

    for thread in &state.parked_threads {
        panes.push((thread.pane_id, &thread.ticket_id, &thread.phase));
    }

    // Sort by pane ID for consistent ordering
    panes.sort_by_key(|(pane_id, _, _)| *pane_id);

    if panes.is_empty() {
        output.push(format!("{}(no active panes){}", DIM, RESET));
    } else {
        output.push(format!("{}Press number to jump to pane:{}", DIM, RESET));
        for (i, (pane_id, ticket_id, phase)) in panes.iter().enumerate().take(9) {
            let phase_color = phase.color_code();
            output.push(format!(
                "  {}[{}]{} {} ({}{}{}) - pane #{}",
                BOLD,
                i + 1,
                RESET,
                ticket_id,
                phase_color,
                phase.short_name(),
                RESET,
                pane_id
            ));
        }
    }
}

// =============================================================================
// Status Line
// =============================================================================

/// Render a compact status line for the title bar
fn render_status_line(state: &PluginState) -> String {
    let active = state.active_threads.len();
    let parked = state.parked_threads.len();
    let done = state
        .tickets
        .iter()
        .filter(|t| t.status == TicketStatus::Done)
        .count();
    let total = state.tickets.len();

    format!(
        "Active: {} | Parked: {} | Done: {}/{}",
        active, parked, done, total
    )
}

// =============================================================================
// Main Dashboard Rendering
// =============================================================================

/// Render the complete dashboard to a vector of lines
fn render_dashboard_lines(state: &PluginState, width: usize, height: usize) -> Vec<String> {
    let mut output = Vec::new();

    // Title bar with status
    let status = render_status_line(state);
    output.push(format!(
        "{}{}  LISA/RALPH Dashboard  {} {}{}",
        BOLD, BG_BLUE, RESET, DIM, status
    ));
    output.push(render_separator(width));
    output.push(String::new());

    // DAG section
    render_dag(state, &mut output);
    output.push(String::new());
    output.push(render_separator(width));
    output.push(String::new());

    // Active threads
    render_active_threads(state, &mut output);
    output.push(String::new());

    // Parked threads
    render_parked_threads(state, &mut output);
    output.push(String::new());
    output.push(render_separator(width));
    output.push(String::new());

    // Calculate remaining space for activity log
    let used_lines = output.len();
    let remaining = height.saturating_sub(used_lines + 8);
    let max_log_entries = remaining.max(3).min(10);

    // Activity log
    render_activity_log(state, max_log_entries, &mut output);
    output.push(String::new());

    // Quick jump section at bottom
    output.push(render_separator(width));
    render_quick_jump(state, &mut output);

    output
}

/// Bridge function that accepts decomposed plugin state from lib.rs and renders the dashboard.
///
/// This converts the internal types (dag::Dag, types::Thread, etc.) into UI-local types
/// (PluginState, TicketNode, etc.) and delegates to print_dashboard.
pub fn render_dashboard(
    rows: usize,
    cols: usize,
    dag: &Dag,
    threads: &HashMap<TicketId, Thread>,
    activity_log: &[ActivityEvent],
    config: &PluginConfig,
) {
    let state = build_plugin_state(dag, threads, activity_log, config);
    print_dashboard(&state, rows, cols);
}

/// Convert decomposed plugin state into a UI PluginState.
fn build_plugin_state(
    dag: &Dag,
    threads: &HashMap<TicketId, Thread>,
    activity_log: &[ActivityEvent],
    config: &PluginConfig,
) -> PluginState {
    // Convert DAG tickets to UI ticket nodes
    let tickets: Vec<TicketNode> = dag
        .tickets()
        .map(|t| TicketNode {
            id: t.id.clone(),
            title: t.title.clone(),
            phase: convert_phase(t.phase),
            status: convert_ticket_status(&t.status, t.phase),
            depends_on: t.depends_on.iter().cloned().collect(),
            blocks: t.blocks.iter().cloned().collect(),
        })
        .collect();

    // Convert active threads
    let active_threads: Vec<ActiveThread> = threads
        .values()
        .filter(|t| t.status == types::ThreadStatus::Running)
        .map(|t| ActiveThread {
            ticket_id: t.ticket_id.clone(),
            phase: convert_phase(t.current_phase),
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
    let parked_threads: Vec<ParkedThread> = threads
        .values()
        .filter(|t| t.status == types::ThreadStatus::Parked)
        .map(|t| ParkedThread {
            ticket_id: t.ticket_id.clone(),
            phase: convert_phase(t.current_phase),
            artifact_path: format!(
                "{}/{}/{}",
                config.work_dir.display(),
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
    let activity_entries: Vec<ActivityEntry> = activity_log
        .iter()
        .filter_map(|e| convert_activity_event(e))
        .collect();

    PluginState {
        tickets,
        active_threads,
        parked_threads,
        activity_log: activity_entries,
        current_time: Duration::from_secs(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        ),
        selected_ticket: None,
    }
}

/// Convert types::Phase to ui::Phase
fn convert_phase(phase: types::Phase) -> Phase {
    match phase {
        types::Phase::Ready => Phase::Ready,
        types::Phase::Research => Phase::Research,
        types::Phase::Design => Phase::Design,
        types::Phase::Structure => Phase::Structure,
        types::Phase::Plan => Phase::Plan,
        types::Phase::Implement => Phase::Implement,
        types::Phase::Review => Phase::Review,
        types::Phase::Done => Phase::Done,
    }
}

/// Convert types::TicketStatus to ui::TicketStatus
fn convert_ticket_status(status: &types::TicketStatus, phase: types::Phase) -> TicketStatus {
    match status {
        types::TicketStatus::Open => {
            if phase == types::Phase::Ready {
                TicketStatus::Ready
            } else {
                TicketStatus::InProgress
            }
        }
        types::TicketStatus::InProgress => TicketStatus::InProgress,
        types::TicketStatus::Blocked => TicketStatus::Blocked,
        types::TicketStatus::Review => TicketStatus::WaitingReview,
        types::TicketStatus::Done => TicketStatus::Done,
        types::TicketStatus::Cancelled => TicketStatus::Done,
    }
}

/// Convert types::ActivityEvent to ui::ActivityEntry
fn convert_activity_event(event: &ActivityEvent) -> Option<ActivityEntry> {
    let timestamp = Duration::ZERO;

    let activity = match event {
        ActivityEvent::PluginStarted => return None,
        ActivityEvent::ThreadSpawned { ticket_id, .. } => ActivityType::ThreadStarted {
            ticket_id: ticket_id.clone(),
            phase: Phase::Ready,
        },
        ActivityEvent::ThreadExited { ticket_id, .. } => ActivityType::PhaseCompleted {
            ticket_id: ticket_id.clone(),
            phase: Phase::Done,
        },
        ActivityEvent::PhaseCompleted { ticket_id, phase } => ActivityType::PhaseCompleted {
            ticket_id: ticket_id.clone(),
            phase: convert_phase(*phase),
        },
        ActivityEvent::TicketPhaseChanged {
            ticket_id,
            new_phase,
            ..
        } => ActivityType::PhaseCompleted {
            ticket_id: ticket_id.clone(),
            phase: convert_phase(*new_phase),
        },
        ActivityEvent::TicketStatusChanged { .. } => return None,
        ActivityEvent::ArtifactCreated {
            ticket_id, path, ..
        } => ActivityType::Commit {
            ticket_id: ticket_id.clone(),
            message: format!("Created {}", path.display()),
        },
        ActivityEvent::CommitMade {
            ticket_id,
            commit_hash,
        } => ActivityType::Commit {
            ticket_id: ticket_id.clone(),
            message: format!("Commit {}", commit_hash),
        },
        ActivityEvent::DagRecomputed { .. } => return None,
        ActivityEvent::Error { message } => ActivityType::Error {
            ticket_id: String::new(),
            message: message.clone(),
        },
    };

    Some(ActivityEntry {
        timestamp,
        activity,
    })
}

/// Print the dashboard to the Zellij pane
///
/// This function is the main entry point called from the plugin's render() implementation.
/// It takes a pre-converted PluginState structure.
pub fn print_dashboard(state: &PluginState, rows: usize, cols: usize) {
    let lines = render_dashboard_lines(state, cols.min(100), rows);

    for line in lines.iter().take(rows) {
        println!("{}", line);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> PluginState {
        PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-001".to_string(),
                    title: "First ticket".to_string(),
                    phase: Phase::Done,
                    status: TicketStatus::Done,
                    depends_on: vec![],
                    blocks: vec!["T-002".to_string()],
                },
                TicketNode {
                    id: "T-002".to_string(),
                    title: "Second ticket".to_string(),
                    phase: Phase::Design,
                    status: TicketStatus::InProgress,
                    depends_on: vec!["T-001".to_string()],
                    blocks: vec!["T-003".to_string()],
                },
                TicketNode {
                    id: "T-003".to_string(),
                    title: "Third ticket".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Blocked,
                    depends_on: vec!["T-002".to_string()],
                    blocks: vec![],
                },
            ],
            active_threads: vec![ActiveThread {
                ticket_id: "T-002".to_string(),
                phase: Phase::Design,
                started_at: Duration::from_secs(60),
                pane_id: 1,
            }],
            parked_threads: vec![],
            activity_log: vec![
                ActivityEntry {
                    timestamp: Duration::from_secs(30),
                    activity: ActivityType::PhaseCompleted {
                        ticket_id: "T-001".to_string(),
                        phase: Phase::Implement,
                    },
                },
                ActivityEntry {
                    timestamp: Duration::from_secs(60),
                    activity: ActivityType::ThreadStarted {
                        ticket_id: "T-002".to_string(),
                        phase: Phase::Design,
                    },
                },
            ],
            current_time: Duration::from_secs(120),
            selected_ticket: None,
        }
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3700)), "1h 1m");
    }

    #[test]
    fn test_phase_short_names() {
        assert_eq!(Phase::Research.short_name(), "RES");
        assert_eq!(Phase::Implement.short_name(), "IMP");
    }

    #[test]
    fn test_phase_full_names() {
        assert_eq!(Phase::Research.full_name(), "Research");
        assert_eq!(Phase::Implement.full_name(), "Implement");
    }

    #[test]
    fn test_phase_indicators() {
        assert_eq!(Phase::Ready.indicator(), "○");
        assert_eq!(Phase::Done.indicator(), "✓");
    }

    #[test]
    fn test_render_dag_not_empty() {
        let state = sample_state();
        let mut output = Vec::new();
        render_dag(&state, &mut output);

        assert!(!output.is_empty());
        assert!(output[0].contains("DAG"));
    }

    #[test]
    fn test_render_dag_empty() {
        let state = PluginState::default();
        let mut output = Vec::new();
        render_dag(&state, &mut output);

        assert!(output.iter().any(|line| line.contains("no tickets")));
    }

    #[test]
    fn test_render_active_threads() {
        let state = sample_state();
        let mut output = Vec::new();
        render_active_threads(&state, &mut output);

        assert!(!output.is_empty());
        assert!(output.iter().any(|l| l.contains("T-002")));
        assert!(output.iter().any(|l| l.contains("Design")));
    }

    #[test]
    fn test_render_active_threads_empty() {
        let state = PluginState::default();
        let mut output = Vec::new();
        render_active_threads(&state, &mut output);

        assert!(output.iter().any(|line| line.contains("no active threads")));
    }

    #[test]
    fn test_render_parked_threads() {
        let mut state = sample_state();
        state.parked_threads.push(ParkedThread {
            ticket_id: "T-003".to_string(),
            phase: Phase::Research,
            artifact_path: "docs/active/work/T-003/research.md".to_string(),
            parked_at: Duration::from_secs(100),
            pane_id: 2,
        });

        let mut output = Vec::new();
        render_parked_threads(&state, &mut output);

        assert!(output.iter().any(|l| l.contains("T-003")));
        assert!(output.iter().any(|l| l.contains("Research")));
    }

    #[test]
    fn test_render_activity_log() {
        let state = sample_state();
        let mut output = Vec::new();
        render_activity_log(&state, 5, &mut output);

        assert!(!output.is_empty());
        // Newest first, so T-002 should appear before T-001
    }

    #[test]
    fn test_compute_dag_layers() {
        let state = sample_state();
        let layers = compute_dag_layers(&state.tickets);

        // T-001 has no deps, should be in first layer
        assert!(layers[0].contains(&0)); // T-001 is at index 0
        // T-002 depends on T-001, should be in second layer
        assert!(layers[1].contains(&1)); // T-002 is at index 1
        // T-003 depends on T-002, should be in third layer
        assert!(layers[2].contains(&2)); // T-003 is at index 2
    }

    #[test]
    fn test_status_line() {
        let state = sample_state();
        let status = render_status_line(&state);

        assert!(status.contains("Active: 1"));
        assert!(status.contains("Parked: 0"));
        assert!(status.contains("Done: 1/3"));
    }

    #[test]
    fn test_render_quick_jump_with_threads() {
        let state = sample_state();
        let mut output = Vec::new();
        render_quick_jump(&state, &mut output);

        assert!(output.iter().any(|l| l.contains("Quick Jump")));
        assert!(output.iter().any(|l| l.contains("T-002")));
        assert!(output.iter().any(|l| l.contains("pane #1")));
    }

    #[test]
    fn test_render_quick_jump_empty() {
        let state = PluginState::default();
        let mut output = Vec::new();
        render_quick_jump(&state, &mut output);

        assert!(output.iter().any(|line| line.contains("no active panes")));
    }

    #[test]
    fn test_full_dashboard_render() {
        let state = sample_state();
        let lines = render_dashboard_lines(&state, 80, 40);

        // Should have content
        assert!(!lines.is_empty());
        // Should contain main sections
        assert!(lines.iter().any(|l| l.contains("Dashboard")));
        assert!(lines.iter().any(|l| l.contains("DAG")));
        assert!(lines.iter().any(|l| l.contains("Active Threads")));
        assert!(lines.iter().any(|l| l.contains("Parked")));
        assert!(lines.iter().any(|l| l.contains("Activity")));
        assert!(lines.iter().any(|l| l.contains("Quick Jump")));
    }
}
