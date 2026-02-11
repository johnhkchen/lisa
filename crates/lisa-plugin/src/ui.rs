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
    pub const BG_YELLOW: &str = "\x1b[43m";
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

/// Type of health alert for the attention banner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertType {
    /// Session exited with a non-zero exit code.
    Failed,
    /// Session has made no progress beyond the stuck threshold.
    Stuck,
    /// Agent went idle but expected phase artifact is missing.
    IdleWithoutArtifact,
}

/// A health alert for the attention banner.
#[derive(Debug, Clone)]
pub struct HealthAlert {
    pub ticket_id: String,
    pub alert_type: AlertType,
    pub detail: String,
    pub suggested_actions: Vec<String>,
}

/// Information about an agent pane slot for dashboard display.
#[derive(Debug, Clone)]
pub struct SlotInfo {
    pub pane_id: u32,
    pub ticket_id: Option<String>,
}

/// Activity log entry types
#[derive(Debug, Clone)]
pub enum ActivityType {
    PhaseCompleted { ticket_id: String, phase: Phase },
    Commit { ticket_id: String, message: String },
    Error { ticket_id: String, message: String },
    Warning { ticket_id: String, message: String },
    ThreadStarted { ticket_id: String, phase: Phase },
    Info { ticket_id: String, message: String },
}

/// A single activity log entry
#[derive(Debug, Clone)]
pub struct ActivityEntry {
    pub timestamp: Duration,
    pub activity: ActivityType,
}

/// State for the modal overlay (UI representation).
#[derive(Debug, Clone, Default)]
pub struct ModalState {
    /// Whether the modal is visible.
    pub open: bool,
    /// Ticket IDs shown in the list.
    pub ticket_ids: Vec<String>,
    /// Currently highlighted index.
    pub cursor: usize,
    /// Whether this is a reset modal (vs mark-done).
    pub is_reset: bool,
}

/// The complete plugin state for rendering
#[derive(Debug, Clone)]
pub struct PluginState {
    pub tickets: Vec<TicketNode>,
    pub active_threads: Vec<ActiveThread>,
    pub parked_threads: Vec<ParkedThread>,
    pub activity_log: Vec<ActivityEntry>,
    pub alerts: Vec<HealthAlert>,
    pub slots: Vec<SlotInfo>,
    pub current_time: Duration,
    pub modal: ModalState,
    /// Whether scheduling of new tickets is paused.
    pub paused: bool,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            tickets: Vec::new(),
            active_threads: Vec::new(),
            parked_threads: Vec::new(),
            activity_log: Vec::new(),
            alerts: Vec::new(),
            slots: Vec::new(),
            current_time: Duration::ZERO,
            modal: ModalState::default(),
            paused: false,
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

/// Render a horizontal separator line
fn render_separator(width: usize) -> String {
    format!("{}{}{}", DIM, "─".repeat(width.min(80)), RESET)
}

// =============================================================================
// Slot Status
// =============================================================================

/// Render the slot status section showing agent pane slot utilization.
fn render_slots(state: &PluginState, output: &mut Vec<String>) {
    if state.slots.is_empty() {
        output.push(format!("{}(no agent slots){}", DIM, RESET));
        return;
    }

    let total = state.slots.len();
    let occupied: Vec<&SlotInfo> = state.slots.iter().filter(|s| s.ticket_id.is_some()).collect();
    let idle = total - occupied.len();

    // Header
    if occupied.is_empty() {
        output.push(format!(
            "{}{}=== Slots: {} total, {} idle ==={}",
            BOLD, CYAN, total, idle, RESET
        ));
    } else {
        output.push(format!(
            "{}{}=== Slots: {} total, {} idle, {} occupied ==={}",
            BOLD, CYAN, total, idle, occupied.len(), RESET
        ));
    }
    output.push(String::new());

    // Occupied slot details
    for slot in &occupied {
        let tid = slot.ticket_id.as_deref().unwrap_or("?");
        // Look up phase from active threads
        let phase_str = state
            .active_threads
            .iter()
            .find(|t| t.ticket_id == tid)
            .map(|t| t.phase.short_name())
            .unwrap_or("---");
        output.push(format!(
            "  {}#{:<4}{} {:<12} [{}]",
            DIM, slot.pane_id, RESET, tid, phase_str
        ));
    }

    // Warning: all occupied and ready tickets waiting
    if idle == 0 {
        let active_ticket_ids: std::collections::HashSet<&str> = state
            .active_threads
            .iter()
            .map(|t| t.ticket_id.as_str())
            .collect();
        let waiting: usize = state
            .tickets
            .iter()
            .filter(|t| t.status == TicketStatus::Ready)
            .filter(|t| !active_ticket_ids.contains(t.id.as_str()))
            .count();
        if waiting > 0 {
            output.push(format!(
                "{}⚠ {} tickets waiting for slots{}",
                YELLOW, waiting, RESET
            ));
        }
    }
}

// =============================================================================
// Attention Banner
// =============================================================================

/// Render the "ATTENTION NEEDED" banner for tickets needing human action.
///
/// Shows a prominent bordered box listing:
/// 1. Tickets in review phase (with ID, title, artifact path, time waiting)
/// 2. Health alerts for stuck/failed sessions
///
/// Appends nothing when no tickets need attention.
fn render_attention_banner(state: &PluginState, width: usize, output: &mut Vec<String>) {
    // Collect tickets in review phase
    let review_tickets: Vec<&TicketNode> = state
        .tickets
        .iter()
        .filter(|t| t.phase == Phase::Review)
        .collect();

    let has_reviews = !review_tickets.is_empty();
    let has_alerts = !state.alerts.is_empty();

    if !has_reviews && !has_alerts {
        return;
    }

    // Build lookup for parked thread data (artifact path, wait time)
    let parked_by_ticket: HashMap<&str, &ParkedThread> = state
        .parked_threads
        .iter()
        .map(|pt| (pt.ticket_id.as_str(), pt))
        .collect();

    let box_w = width.min(64);
    let inner_w = box_w.saturating_sub(4); // account for "║ " and " ║"

    // Top border
    output.push(format!(
        "{}{}╔{}╗{}",
        BOLD, BRIGHT_YELLOW, "═".repeat(box_w.saturating_sub(2)), RESET
    ));

    // Header line
    let header = "⚠ ATTENTION NEEDED";
    let header_pad = inner_w.saturating_sub(header.chars().count());
    output.push(format!(
        "{}{}║ {}{}{}{}{}║{}",
        BOLD, BRIGHT_YELLOW,
        BG_YELLOW, WHITE, header, RESET,
        format!("{}{} ", " ".repeat(header_pad), format!("{}{}", BOLD, BRIGHT_YELLOW)),
        RESET
    ));

    // Review ticket rows
    for ticket in &review_tickets {
        let parked = parked_by_ticket.get(ticket.id.as_str());

        // Truncate title to 20 chars
        let title: String = if ticket.title.chars().count() > 20 {
            format!("{}..", ticket.title.chars().take(18).collect::<String>())
        } else {
            ticket.title.clone()
        };

        // Artifact: extract filename from path
        let artifact = match parked {
            Some(pt) => pt
                .artifact_path
                .rsplit('/')
                .next()
                .unwrap_or(&pt.artifact_path)
                .to_string(),
            None => "—".to_string(),
        };

        // Wait time
        let wait_time = match parked {
            Some(pt) => format_time_since(pt.parked_at, state.current_time),
            None => "—".to_string(),
        };

        let content = format!(
            "{:<10} {:<20} {:<14} {:>8}",
            ticket.id, title, artifact, wait_time
        );
        let content_visible_len = content.chars().count();
        let row_pad = inner_w.saturating_sub(content_visible_len);

        output.push(format!(
            "{}{}║{} {}{}{} {}║{}",
            BOLD, BRIGHT_YELLOW,
            RESET,
            YELLOW, content, RESET,
            format!("{}{}{}", " ".repeat(row_pad.saturating_sub(1)), BOLD, BRIGHT_YELLOW),
            RESET
        ));
    }

    // Health alert rows (stuck/failed sessions)
    for alert in state.alerts.iter().take(5) {
        let (label, color) = match alert.alert_type {
            AlertType::Failed => ("✗ FAILED", RED),
            AlertType::Stuck => ("! STUCK ", YELLOW),
            AlertType::IdleWithoutArtifact => ("⏸ IDLE  ", YELLOW),
        };

        let detail_max = inner_w.saturating_sub(24); // label + space + ticket_id + space
        let detail: String = if alert.detail.chars().count() > detail_max {
            format!("{}..", alert.detail.chars().take(detail_max.saturating_sub(2)).collect::<String>())
        } else {
            alert.detail.clone()
        };

        let content = format!("{} {:<12} {}", label, alert.ticket_id, detail);
        let content_visible_len = content.chars().count();
        let row_pad = inner_w.saturating_sub(content_visible_len);

        output.push(format!(
            "{}{}║{} {}{}{} {}║{}",
            BOLD, BRIGHT_YELLOW,
            RESET,
            color, content, RESET,
            format!("{}{}{}", " ".repeat(row_pad.saturating_sub(1)), BOLD, BRIGHT_YELLOW),
            RESET
        ));

        // Suggested actions
        if !alert.suggested_actions.is_empty() {
            let actions = format!("  {}", alert.suggested_actions.join(" | "));
            let actions_len = actions.chars().count();
            let actions_pad = inner_w.saturating_sub(actions_len);
            output.push(format!(
                "{}{}║{} {}{}{} {}║{}",
                BOLD, BRIGHT_YELLOW,
                RESET,
                DIM, actions, RESET,
                format!("{}{}{}", " ".repeat(actions_pad.saturating_sub(1)), BOLD, BRIGHT_YELLOW),
                RESET
            ));
        }
    }

    if state.alerts.len() > 5 {
        let more = format!("... and {} more alerts", state.alerts.len() - 5);
        let pad = inner_w.saturating_sub(more.len());
        output.push(format!(
            "{}{}║{} {}{}{} {}║{}",
            BOLD, BRIGHT_YELLOW,
            RESET,
            DIM, more, RESET,
            format!("{}{}{}", " ".repeat(pad.saturating_sub(1)), BOLD, BRIGHT_YELLOW),
            RESET
        ));
    }

    // Hint row
    let hint = "Press [d] to mark done";
    let hint_len = hint.chars().count();
    let hint_pad = inner_w.saturating_sub(hint_len);
    output.push(format!(
        "{}{}║{} {}{}{} {}║{}",
        BOLD, BRIGHT_YELLOW,
        RESET,
        DIM, hint, RESET,
        format!("{}{}{}", " ".repeat(hint_pad.saturating_sub(1)), BOLD, BRIGHT_YELLOW),
        RESET
    ));

    // Bottom border
    output.push(format!(
        "{}{}╚{}╝{}",
        BOLD, BRIGHT_YELLOW, "═".repeat(box_w.saturating_sub(2)), RESET
    ));

    output.push(String::new());
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

/// Render the DAG with fixed-width grid layout and properly aligned edges.
///
/// Each node occupies a fixed-width cell. Layers are sorted by ticket ID
/// for deterministic ordering. Edges are drawn as │ connectors at the
/// parent node's column center, including pass-through for multi-layer edges.
fn render_dag(state: &PluginState, output: &mut Vec<String>) {
    output.push(format!("{}{}≡≡ DAG ≡≡{}", BOLD, CYAN, RESET));
    output.push(String::new());

    if state.tickets.is_empty() {
        output.push(format!("{}(no tickets){}", DIM, RESET));
        return;
    }

    // Visible character widths
    // Cell: "{indicator} {id:<8} {status}" = 1+1+8+1+3 = 14, padded to CELL_W
    const CELL_W: usize = 15;
    const GAP: usize = 1;
    const INDENT: usize = 2;

    // Compute layers and sort each by ticket ID for stable ordering
    let mut layers = compute_dag_layers(&state.tickets);
    for layer in &mut layers {
        layer.sort_by(|a, b| state.tickets[*a].id.cmp(&state.tickets[*b].id));
    }

    // Map ticket_id → (layer_index, center_x in visible chars)
    let mut center_x: HashMap<&str, usize> = HashMap::new();
    let mut ticket_layer: HashMap<&str, usize> = HashMap::new();
    for (li, layer) in layers.iter().enumerate() {
        for (col, &idx) in layer.iter().enumerate() {
            let cx = INDENT + col * (CELL_W + GAP) + CELL_W / 2;
            center_x.insert(&state.tickets[idx].id, cx);
            ticket_layer.insert(&state.tickets[idx].id, li);
        }
    }

    // Collect all edges (parent_id, child_id)
    let edges: Vec<(&str, &str)> = state
        .tickets
        .iter()
        .flat_map(|t| t.depends_on.iter().map(move |dep| (dep.as_str(), t.id.as_str())))
        .collect();

    // Render each layer with connector lines above
    for (li, layer) in layers.iter().enumerate() {
        // Connector line between previous layer and this one
        if li > 0 {
            // Find max width needed across both layers
            let this_w = INDENT + layer.len() * (CELL_W + GAP);
            let prev_w = INDENT + layers[li - 1].len() * (CELL_W + GAP);
            let width = this_w.max(prev_w);

            let mut conn: Vec<char> = vec![' '; width];

            // Draw │ for every edge that crosses this boundary:
            // parent in layer < li, child in layer >= li
            for &(parent_id, child_id) in &edges {
                let pl = ticket_layer.get(parent_id).copied().unwrap_or(0);
                let cl = ticket_layer.get(child_id).copied().unwrap_or(0);
                if pl < li && cl >= li {
                    if let Some(&px) = center_x.get(parent_id) {
                        if px < conn.len() {
                            conn[px] = '│';
                        }
                    }
                }
            }

            let line: String = conn.iter().collect();
            let trimmed = line.trim_end();
            if !trimmed.trim().is_empty() {
                output.push(format!("{}{}{}", DIM, trimmed, RESET));
            }
        }

        // Node line
        let mut line = " ".repeat(INDENT);
        for (col, &idx) in layer.iter().enumerate() {
            if col > 0 {
                line.push_str(&" ".repeat(GAP));
            }
            let ticket = &state.tickets[idx];
            let indicator = ticket.phase.indicator();
            let phase_color = ticket.phase.color_code();

            let (status_str, status_color) = match &ticket.status {
                TicketStatus::Ready => ("RDY", CYAN),
                TicketStatus::InProgress => ("WRK", GREEN),
                TicketStatus::WaitingReview => ("REV", BRIGHT_YELLOW),
                TicketStatus::Blocked => ("BLK", RED),
                TicketStatus::Done => ("DON", BRIGHT_GREEN),
            };

            // Truncate or pad ID to 8 visible chars
            let id_str: String = ticket.id.chars().take(8).collect();

            // Visible content: indicator(1) + space(1) + id(8) + space(1) + status(3) = 14
            let content_w = 1 + 1 + 8 + 1 + 3; // 14
            let pad = CELL_W.saturating_sub(content_w);

            line.push_str(&format!(
                "{}{}{} {:<8}{}{}{}{} ",
                phase_color, indicator, RESET,
                id_str,
                " ".repeat(pad),
                status_color, status_str, RESET,
            ));
        }
        output.push(line);
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
            ActivityType::Warning { ticket_id, message } => {
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
                ("⚠", BRIGHT_YELLOW, format!("{}warn: {}", prefix, msg))
            }
            ActivityType::Info { ticket_id, message } => {
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
                ("ℹ", CYAN, format!("{}{}", prefix, msg))
            }
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
    let slot_total = state.slots.len();
    let slot_occupied = state.slots.iter().filter(|s| s.ticket_id.is_some()).count();
    let active = state.active_threads.len();
    let parked = state.parked_threads.len();
    let done = state
        .tickets
        .iter()
        .filter(|t| t.status == TicketStatus::Done)
        .count();
    let total = state.tickets.len();

    let slot_str = if slot_total > 0 {
        format!("Slots: {}/{} | ", slot_occupied, slot_total)
    } else {
        String::new()
    };

    let alert_count = state.alerts.len();
    let alert_str = if alert_count > 0 {
        format!(" | {}Alerts: {}{}", RED, alert_count, RESET)
    } else {
        String::new()
    };

    let pause_str = if state.paused {
        format!("{}PAUSED{} | ", YELLOW, RESET)
    } else {
        String::new()
    };

    let pause_hint = if state.paused { "resume" } else { "pause" };

    format!(
        "{}{}Active: {} | Parked: {} | Done: {}/{}{}  {}[p] {}  [d] mark done  [r] reset{}",
        pause_str, slot_str, active, parked, done, total, alert_str, DIM, pause_hint, RESET
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

    // Slot status
    render_slots(state, &mut output);

    // Attention banner (review gate alerts)
    render_attention_banner(state, width, &mut output);

    // DAG section
    render_dag(state, &mut output);
    output.push(render_separator(width));

    // Active threads
    render_active_threads(state, &mut output);

    // Parked threads
    render_parked_threads(state, &mut output);
    output.push(render_separator(width));

    // Calculate remaining space for activity log
    let used_lines = output.len();
    let remaining = height.saturating_sub(used_lines + 4);
    let max_log_entries = remaining.max(3).min(10);

    // Activity log
    render_activity_log(state, max_log_entries, &mut output);

    // Quick jump section at bottom
    output.push(render_separator(width));
    render_quick_jump(state, &mut output);

    output
}

/// Render the mark-done modal as an overlay.
fn render_modal(modal: &ModalState, width: usize, height: usize) -> Vec<String> {
    let mut output = Vec::new();

    let box_w = width.min(50);
    let list_h = modal.ticket_ids.len().min(height.saturating_sub(6));
    let box_h = list_h + 4; // title + separator + list + footer
    let pad_top = height.saturating_sub(box_h) / 2;

    let border_h = "─".repeat(box_w.saturating_sub(2));

    // Top padding
    for _ in 0..pad_top {
        output.push(String::new());
    }

    // Top border
    output.push(format!("┌{}┐", border_h));

    // Title
    let title = if modal.is_reset {
        " Reset Ticket to Ready "
    } else {
        " Mark Ticket Done "
    };
    let title_pad = box_w.saturating_sub(2).saturating_sub(title.len());
    let left_pad = title_pad / 2;
    let right_pad = title_pad - left_pad;
    output.push(format!(
        "│{}{}{}{}{}│",
        " ".repeat(left_pad),
        BOLD, title, RESET,
        " ".repeat(right_pad),
    ));

    // Separator
    output.push(format!("├{}┤", border_h));

    // Ticket list
    for (i, tid) in modal.ticket_ids.iter().enumerate().take(list_h) {
        let selected = i == modal.cursor;
        let prefix = if selected { "▸ " } else { "  " };
        let (color_start, color_end) = if selected {
            (format!("{}{}", BOLD, CYAN), RESET.to_string())
        } else {
            (String::new(), String::new())
        };

        let entry = format!("{}{}{}{}", prefix, color_start, tid, color_end);
        // Pad to fill the box (accounting for ANSI codes in visible width)
        let visible_len = prefix.len() + tid.len();
        let inner_pad = box_w.saturating_sub(2).saturating_sub(visible_len);
        output.push(format!("│{}{}│", entry, " ".repeat(inner_pad)));
    }

    // Footer
    output.push(format!("├{}┤", border_h));
    let footer = " Enter=confirm  Esc=cancel ";
    let footer_pad = box_w.saturating_sub(2).saturating_sub(footer.len());
    let fl = footer_pad / 2;
    let fr = footer_pad - fl;
    output.push(format!(
        "│{}{}{}{}{}│",
        " ".repeat(fl),
        DIM, footer, RESET,
        " ".repeat(fr),
    ));
    // Bottom border
    output.push(format!("└{}┘", border_h));

    output
}

/// Print the dashboard to the Zellij pane
///
/// This function is the main entry point called from the plugin's render() implementation.
/// It takes a pre-converted PluginState structure. `scroll_offset` controls how many lines
/// are skipped from the top of the rendered content.
pub fn print_dashboard(state: &PluginState, rows: usize, cols: usize, scroll_offset: usize) {
    if state.modal.open {
        let lines = render_modal(&state.modal, cols.min(60), rows);
        for line in lines.iter().take(rows) {
            println!("{}", line);
        }
        return;
    }

    let lines = render_dashboard_lines(state, cols.min(100), rows);

    // Clamp scroll so we don't scroll past content
    let max_scroll = lines.len().saturating_sub(rows);
    let offset = scroll_offset.min(max_scroll);

    for line in lines.iter().skip(offset).take(rows) {
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
                },
                TicketNode {
                    id: "T-002".to_string(),
                    title: "Second ticket".to_string(),
                    phase: Phase::Design,
                    status: TicketStatus::InProgress,
                    depends_on: vec!["T-001".to_string()],
                },
                TicketNode {
                    id: "T-003".to_string(),
                    title: "Third ticket".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Blocked,
                    depends_on: vec!["T-002".to_string()],

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
            alerts: Vec::new(),
            slots: Vec::new(),
            current_time: Duration::from_secs(120),
            modal: ModalState::default(),
            paused: false,
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

    #[test]
    fn test_pipeline_dag_to_dashboard() {
        // Diamond DAG: T-001 -> {T-002, T-003} -> T-004
        let state = PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-001".to_string(),
                    title: "root".to_string(),
                    phase: Phase::Done,
                    status: TicketStatus::Done,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-002".to_string(),
                    title: "left".to_string(),
                    phase: Phase::Design,
                    status: TicketStatus::InProgress,
                    depends_on: vec!["T-001".to_string()],
                },
                TicketNode {
                    id: "T-003".to_string(),
                    title: "right".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Ready,
                    depends_on: vec!["T-001".to_string()],
                },
                TicketNode {
                    id: "T-004".to_string(),
                    title: "leaf".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Blocked,
                    depends_on: vec!["T-002".to_string(), "T-003".to_string()],

                },
            ],
            active_threads: vec![ActiveThread {
                ticket_id: "T-002".to_string(),
                phase: Phase::Design,
                started_at: Duration::from_secs(100),
                pane_id: 5,
            }],
            parked_threads: vec![ParkedThread {
                ticket_id: "T-003".to_string(),
                phase: Phase::Research,
                artifact_path: "docs/active/work/T-003/research.md".to_string(),
                parked_at: Duration::from_secs(80),
                pane_id: 6,
            }],
            activity_log: vec![
                ActivityEntry {
                    timestamp: Duration::from_secs(50),
                    activity: ActivityType::PhaseCompleted {
                        ticket_id: "T-001".to_string(),
                        phase: Phase::Implement,
                    },
                },
                ActivityEntry {
                    timestamp: Duration::from_secs(100),
                    activity: ActivityType::ThreadStarted {
                        ticket_id: "T-002".to_string(),
                        phase: Phase::Design,
                    },
                },
                ActivityEntry {
                    timestamp: Duration::from_secs(120),
                    activity: ActivityType::Error {
                        ticket_id: "T-003".to_string(),
                        message: "test error".to_string(),
                    },
                },
            ],
            alerts: Vec::new(),
            slots: Vec::new(),
            current_time: Duration::from_secs(200),
            modal: ModalState::default(),
            paused: false,
        };

        let lines = render_dashboard_lines(&state, 80, 50);
        let full_output = lines.join("\n");

        // All ticket IDs appear
        assert!(full_output.contains("T-001"), "T-001 missing from dashboard");
        assert!(full_output.contains("T-002"), "T-002 missing from dashboard");
        assert!(full_output.contains("T-003"), "T-003 missing from dashboard");
        assert!(full_output.contains("T-004"), "T-004 missing from dashboard");

        // Dashboard header
        assert!(full_output.contains("Dashboard"), "Dashboard header missing");

        // Active thread section shows T-002 with Design phase
        assert!(full_output.contains("Design"), "Active thread phase missing");

        // Parked thread section shows T-003 with artifact
        assert!(
            full_output.contains("research.md"),
            "Parked thread artifact missing"
        );

        // Activity log has entries
        assert!(
            full_output.contains("test error"),
            "Error activity missing"
        );

        // Status line: Active: 1, Parked: 1, Done: 1/4
        assert!(full_output.contains("Active: 1"), "Active count wrong");
        assert!(full_output.contains("Parked: 1"), "Parked count wrong");
        assert!(full_output.contains("Done: 1/4"), "Done count wrong");

        // DAG layers: T-001 should be in first layer, T-002/T-003 in second, T-004 in third
        let layers = compute_dag_layers(&state.tickets);
        assert_eq!(layers.len(), 3, "Expected 3 DAG layers for diamond");
        assert!(layers[0].contains(&0), "T-001 not in first layer");
        assert!(
            layers[1].contains(&1) && layers[1].contains(&2),
            "T-002 and T-003 not in second layer"
        );
        assert!(layers[2].contains(&3), "T-004 not in third layer");
    }

    #[test]
    fn test_render_attention_banner_with_review_tickets() {
        let state = PluginState {
            tickets: vec![TicketNode {
                id: "T-005".to_string(),
                title: "review-ticket".to_string(),
                phase: Phase::Review,
                status: TicketStatus::WaitingReview,
                depends_on: vec![],

            }],
            parked_threads: vec![ParkedThread {
                ticket_id: "T-005".to_string(),
                phase: Phase::Review,
                artifact_path: "docs/active/work/T-005/design.md".to_string(),
                parked_at: Duration::from_secs(50),
                pane_id: 3,
            }],
            current_time: Duration::from_secs(200),
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_attention_banner(&state, 80, &mut output);

        let full = output.join("\n");
        assert!(full.contains("ATTENTION NEEDED"), "Banner header missing");
        assert!(full.contains("T-005"), "Ticket ID missing from banner");
        assert!(full.contains("review-ticket"), "Ticket title missing from banner");
        assert!(full.contains("design.md"), "Artifact path missing from banner");
        // Wait time: 200 - 50 = 150s = 2m 30s
        assert!(full.contains("2m 30s"), "Wait time missing from banner");
        // Box drawing characters
        assert!(full.contains("╔"), "Top border missing");
        assert!(full.contains("╚"), "Bottom border missing");
    }

    #[test]
    fn test_render_attention_banner_empty() {
        let state = PluginState {
            tickets: vec![TicketNode {
                id: "T-001".to_string(),
                title: "not-in-review".to_string(),
                phase: Phase::Implement,
                status: TicketStatus::InProgress,
                depends_on: vec![],

            }],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_attention_banner(&state, 80, &mut output);

        assert!(output.is_empty(), "Banner should not render when no review tickets");
    }

    #[test]
    fn test_render_attention_banner_no_parked_thread() {
        let state = PluginState {
            tickets: vec![TicketNode {
                id: "T-006".to_string(),
                title: "orphan-review".to_string(),
                phase: Phase::Review,
                status: TicketStatus::WaitingReview,
                depends_on: vec![],

            }],
            parked_threads: vec![], // No matching parked thread
            current_time: Duration::from_secs(100),
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_attention_banner(&state, 80, &mut output);

        let full = output.join("\n");
        assert!(full.contains("ATTENTION NEEDED"), "Banner should still render");
        assert!(full.contains("T-006"), "Ticket ID should appear");
        assert!(full.contains("—"), "Dash placeholder should appear for missing data");
    }

    #[test]
    fn test_attention_banner_in_full_dashboard() {
        let state = PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-001".to_string(),
                    title: "done-ticket".to_string(),
                    phase: Phase::Done,
                    status: TicketStatus::Done,
                    depends_on: vec![],

                },
                TicketNode {
                    id: "T-002".to_string(),
                    title: "in-review".to_string(),
                    phase: Phase::Review,
                    status: TicketStatus::WaitingReview,
                    depends_on: vec!["T-001".to_string()],

                },
            ],
            parked_threads: vec![ParkedThread {
                ticket_id: "T-002".to_string(),
                phase: Phase::Review,
                artifact_path: "docs/active/work/T-002/plan.md".to_string(),
                parked_at: Duration::from_secs(10),
                pane_id: 1,
            }],
            current_time: Duration::from_secs(100),
            ..PluginState::default()
        };

        let lines = render_dashboard_lines(&state, 80, 50);
        let full = lines.join("\n");

        // Banner should appear
        assert!(full.contains("ATTENTION NEEDED"), "Banner missing from full dashboard");

        // Banner should appear BEFORE DAG
        let banner_pos = full.find("ATTENTION NEEDED").unwrap();
        let dag_pos = full.find("DAG").unwrap();
        assert!(banner_pos < dag_pos, "Banner should appear before DAG section");
    }

    #[test]
    fn test_render_attention_banner_with_health_alerts() {
        let state = PluginState {
            alerts: vec![
                HealthAlert {
                    ticket_id: "T-010".to_string(),
                    alert_type: AlertType::Failed,
                    detail: "Exit code: 1".to_string(),
                    suggested_actions: vec!["Check logs".to_string(), "Retry".to_string()],
                },
                HealthAlert {
                    ticket_id: "T-011".to_string(),
                    alert_type: AlertType::Stuck,
                    detail: "No progress for 15+ min".to_string(),
                    suggested_actions: vec!["Check pane".to_string()],
                },
            ],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_attention_banner(&state, 80, &mut output);

        let full = output.join("\n");
        assert!(full.contains("ATTENTION NEEDED"), "Banner header missing");
        assert!(full.contains("T-010"), "Failed ticket missing");
        assert!(full.contains("FAILED"), "FAILED indicator missing");
        assert!(full.contains("T-011"), "Stuck ticket missing");
        assert!(full.contains("STUCK"), "STUCK indicator missing");
        assert!(full.contains("Check logs"), "Suggested action missing");
        assert!(full.contains("Exit code: 1"), "Detail missing");
    }

    #[test]
    fn test_render_attention_banner_alerts_only_no_reviews() {
        let state = PluginState {
            alerts: vec![HealthAlert {
                ticket_id: "T-099".to_string(),
                alert_type: AlertType::Stuck,
                detail: "No progress for 20+ min".to_string(),
                suggested_actions: vec![],
            }],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_attention_banner(&state, 80, &mut output);

        let full = output.join("\n");
        assert!(full.contains("ATTENTION NEEDED"));
        assert!(full.contains("T-099"));
        assert!(full.contains("STUCK"));
    }

    #[test]
    fn test_status_line_with_alerts() {
        let state = PluginState {
            alerts: vec![HealthAlert {
                ticket_id: "T-001".to_string(),
                alert_type: AlertType::Failed,
                detail: "test".to_string(),
                suggested_actions: vec![],
            }],
            ..PluginState::default()
        };

        let status = render_status_line(&state);
        assert!(status.contains("Alerts: 1"), "Alert count missing from status line");
    }

    #[test]
    fn test_status_line_no_alerts() {
        let state = PluginState::default();
        let status = render_status_line(&state);
        assert!(!status.contains("Alerts"), "Alerts should not appear when count is 0");
    }

    #[test]
    fn test_render_attention_banner_mixed_alerts_and_reviews() {
        let state = PluginState {
            tickets: vec![TicketNode {
                id: "T-005".to_string(),
                title: "review-ticket".to_string(),
                phase: Phase::Review,
                status: TicketStatus::WaitingReview,
                depends_on: vec![],

            }],
            alerts: vec![HealthAlert {
                ticket_id: "T-010".to_string(),
                alert_type: AlertType::Failed,
                detail: "Session crashed".to_string(),
                suggested_actions: vec!["Retry".to_string()],
            }],
            parked_threads: vec![ParkedThread {
                ticket_id: "T-005".to_string(),
                phase: Phase::Review,
                artifact_path: "docs/active/work/T-005/design.md".to_string(),
                parked_at: Duration::from_secs(50),
                pane_id: 3,
            }],
            current_time: Duration::from_secs(200),
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_attention_banner(&state, 80, &mut output);

        let full = output.join("\n");
        // Both health alert and review ticket should appear
        assert!(full.contains("T-010"), "Health alert ticket missing");
        assert!(full.contains("T-005"), "Review ticket missing");
        assert!(full.contains("FAILED"), "Failed indicator missing");
        assert!(full.contains("design.md"), "Review artifact missing");
    }

    #[test]
    fn test_render_slots_all_idle() {
        let state = PluginState {
            slots: vec![
                SlotInfo { pane_id: 1, ticket_id: None },
                SlotInfo { pane_id: 2, ticket_id: None },
            ],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_slots(&state, &mut output);
        let full = output.join("\n");

        assert!(full.contains("2 total"), "Total count missing");
        assert!(full.contains("2 idle"), "Idle count missing");
        assert!(!full.contains("occupied"), "Should not show occupied when all idle");
    }

    #[test]
    fn test_render_slots_all_occupied() {
        let state = PluginState {
            slots: vec![
                SlotInfo { pane_id: 5, ticket_id: Some("T-003-01".to_string()) },
                SlotInfo { pane_id: 6, ticket_id: Some("T-003-02".to_string()) },
            ],
            active_threads: vec![
                ActiveThread {
                    ticket_id: "T-003-01".to_string(),
                    phase: Phase::Implement,
                    started_at: Duration::from_secs(100),
                    pane_id: 5,
                },
                ActiveThread {
                    ticket_id: "T-003-02".to_string(),
                    phase: Phase::Research,
                    started_at: Duration::from_secs(100),
                    pane_id: 6,
                },
            ],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_slots(&state, &mut output);
        let full = output.join("\n");

        assert!(full.contains("2 total"), "Total count missing");
        assert!(full.contains("0 idle"), "Idle count missing");
        assert!(full.contains("2 occupied"), "Occupied count missing");
        assert!(full.contains("T-003-01"), "Occupied ticket missing");
        assert!(full.contains("T-003-02"), "Occupied ticket missing");
        assert!(full.contains("IMP"), "Phase shortname missing");
        assert!(full.contains("RES"), "Phase shortname missing");
    }

    #[test]
    fn test_render_slots_mixed() {
        let state = PluginState {
            slots: vec![
                SlotInfo { pane_id: 5, ticket_id: Some("T-001".to_string()) },
                SlotInfo { pane_id: 6, ticket_id: None },
                SlotInfo { pane_id: 7, ticket_id: None },
            ],
            active_threads: vec![ActiveThread {
                ticket_id: "T-001".to_string(),
                phase: Phase::Design,
                started_at: Duration::from_secs(50),
                pane_id: 5,
            }],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_slots(&state, &mut output);
        let full = output.join("\n");

        assert!(full.contains("3 total"), "Total count missing");
        assert!(full.contains("2 idle"), "Idle count missing");
        assert!(full.contains("1 occupied"), "Occupied count missing");
        assert!(full.contains("T-001"), "Occupied ticket missing");
        assert!(full.contains("DES"), "Phase shortname missing");
    }

    #[test]
    fn test_render_slots_no_slots() {
        let state = PluginState::default();

        let mut output = Vec::new();
        render_slots(&state, &mut output);
        let full = output.join("\n");

        assert!(full.contains("no agent slots"), "Empty message missing");
    }

    #[test]
    fn test_render_slots_warning_tickets_waiting() {
        let state = PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-001".to_string(),
                    title: "occupied".to_string(),
                    phase: Phase::Implement,
                    status: TicketStatus::InProgress,
                    depends_on: vec![],

                },
                TicketNode {
                    id: "T-002".to_string(),
                    title: "waiting".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Ready,
                    depends_on: vec![],

                },
                TicketNode {
                    id: "T-003".to_string(),
                    title: "also-waiting".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Ready,
                    depends_on: vec![],

                },
            ],
            slots: vec![
                SlotInfo { pane_id: 5, ticket_id: Some("T-001".to_string()) },
            ],
            active_threads: vec![ActiveThread {
                ticket_id: "T-001".to_string(),
                phase: Phase::Implement,
                started_at: Duration::from_secs(100),
                pane_id: 5,
            }],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_slots(&state, &mut output);
        let full = output.join("\n");

        assert!(full.contains("2 tickets waiting for slots"), "Warning missing");
    }

    #[test]
    fn test_status_line_with_slots() {
        let state = PluginState {
            slots: vec![
                SlotInfo { pane_id: 1, ticket_id: Some("T-001".to_string()) },
                SlotInfo { pane_id: 2, ticket_id: None },
            ],
            ..PluginState::default()
        };

        let status = render_status_line(&state);
        assert!(status.contains("Slots: 1/2"), "Slot count missing from status line");
        assert!(status.contains("Active: 0"), "Active count missing");
    }

    #[test]
    fn test_slots_in_full_dashboard() {
        let state = PluginState {
            tickets: vec![TicketNode {
                id: "T-001".to_string(),
                title: "test".to_string(),
                phase: Phase::Implement,
                status: TicketStatus::InProgress,
                depends_on: vec![],

            }],
            slots: vec![
                SlotInfo { pane_id: 5, ticket_id: Some("T-001".to_string()) },
                SlotInfo { pane_id: 6, ticket_id: None },
            ],
            active_threads: vec![ActiveThread {
                ticket_id: "T-001".to_string(),
                phase: Phase::Implement,
                started_at: Duration::from_secs(50),
                pane_id: 5,
            }],
            ..PluginState::default()
        };

        let lines = render_dashboard_lines(&state, 80, 50);
        let full = lines.join("\n");

        // Slots section should appear
        assert!(full.contains("Slots:"), "Slots section missing from dashboard");

        // Slots should appear before DAG
        let slots_pos = full.find("Slots:").unwrap();
        let dag_pos = full.find("DAG").unwrap();
        assert!(slots_pos < dag_pos, "Slots section should appear before DAG");
    }

    #[test]
    fn test_status_line_paused() {
        let state = PluginState {
            paused: true,
            ..sample_state()
        };
        let status = render_status_line(&state);
        assert!(status.contains("PAUSED"), "should show PAUSED indicator");
        assert!(status.contains("[p] resume"), "should show resume hint");
    }

    #[test]
    fn test_status_line_not_paused() {
        let state = sample_state();
        let status = render_status_line(&state);
        assert!(!status.contains("PAUSED"), "should not show PAUSED when unpaused");
        assert!(status.contains("[p] pause"), "should show pause hint");
    }

    #[test]
    fn test_status_line_has_reset_hint() {
        let state = sample_state();
        let status = render_status_line(&state);
        assert!(status.contains("[r] reset"), "Status line should show [r] reset hint");
    }

    #[test]
    fn test_modal_title_reset() {
        let modal = ModalState {
            open: true,
            ticket_ids: vec!["T-001".to_string()],
            cursor: 0,
            is_reset: true,
        };
        let lines = render_modal(&modal, 50, 20);
        let full = lines.join("\n");
        assert!(full.contains("Reset Ticket to Ready"), "Reset modal should have correct title");
    }

    #[test]
    fn test_modal_title_mark_done() {
        let modal = ModalState {
            open: true,
            ticket_ids: vec!["T-001".to_string()],
            cursor: 0,
            is_reset: false,
        };
        let lines = render_modal(&modal, 50, 20);
        let full = lines.join("\n");
        assert!(full.contains("Mark Ticket Done"), "Mark-done modal should have correct title");
    }

    #[test]
    fn test_compact_dashboard_fewer_lines() {
        let state = sample_state();
        let lines = render_dashboard_lines(&state, 80, 40);
        // Count blank lines
        let blank_lines = lines.iter().filter(|l| l.is_empty()).count();
        // Compacted dashboard should have fewer blank lines than before (~12).
        // Sub-sections still add a few internally, but we removed the padding between them.
        assert!(blank_lines <= 8, "Dashboard should have <= 8 blank lines, got {}", blank_lines);
    }

    #[test]
    fn test_scroll_offset_clamp() {
        let state = sample_state();
        let lines = render_dashboard_lines(&state, 80, 40);
        let total_lines = lines.len();

        // Scroll offset beyond content should be clamped
        let rows = 10;
        let max_scroll = total_lines.saturating_sub(rows);
        let clamped = 9999usize.min(max_scroll);
        assert!(clamped <= total_lines, "Clamped scroll should not exceed total lines");
        assert_eq!(clamped, max_scroll, "Clamped scroll should equal max_scroll");
    }
}
