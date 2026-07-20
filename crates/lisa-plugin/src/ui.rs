//! UI/Dashboard module for the Lisa Zellij plugin.
//!
//! Provides three preset views cycled with `[p]`:
//! - **Operations**: Attention banner, unified thread table, filtered activity log
//! - **DAG**: Full dependency graph visualization
//! - **Activity**: Complete activity log with all entry types
//!
//! Replaces manual status checking with a single live view.

use std::collections::HashMap;
use std::time::Duration;

use lisa_core::operator_override::{OverriddenAsk, OverrideReason};
use lisa_core::triage::TriageProposal;
use lisa_core::types::CompletionRejectionKind;

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
    pub slot_number: usize,
    /// True if this thread's pane is blocked on an `AskUserQuestion`
    /// (mirrors the plugin's `awaiting_human` set). Drives the dashboard
    /// "awaiting human" marker and is the same signal that exempts the pane
    /// from wall-clock reclamation, so the two can never disagree.
    pub awaiting: bool,
    /// Pre-formatted `(provider, model)` route cell for this pane (T-026-01),
    /// e.g. `claude`, `codex/gpt-5`, or `codex/gpt-5*` when the route was a
    /// substituted fallback. `None` for a thread spawned before routing existed;
    /// rendered as `—`.
    pub route: Option<String>,
}

/// Represents a parked thread waiting for review
#[derive(Debug, Clone)]
pub struct ParkedThread {
    pub ticket_id: String,
    pub phase: Phase,
    pub artifact_path: String,
    pub parked_at: Duration,
    pub slot_number: usize,
}

/// One durable parked remedy reduced to its human-facing dashboard line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingItem {
    pub ticket_id: String,
    pub ask: String,
    pub reason: String,
    pub checks_on_own: bool,
    pub proposal: Option<TriageProposal>,
}

/// One durable completion note reduced to dashboard display data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteItem {
    pub ticket_id: String,
    pub summary: String,
    pub criterion_quote: String,
    pub evidence_citation: String,
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
    /// Session exceeded the configured session_timeout_secs.
    TimedOut,
}

/// A health alert for the attention banner.
#[derive(Debug, Clone)]
pub struct HealthAlert {
    pub ticket_id: String,
    pub alert_type: AlertType,
    pub detail: String,
    pub suggested_actions: Vec<String>,
}

/// Scheduler-owned seat assignment state reduced to dashboard semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatAssignmentStatus {
    Starting,
    ReadyForAssignment,
    Delivering,
    DeliveredAwaitingClaim,
    AssignedPendingAck,
    Owned,
    Recovering,
    ClaimTimedOut,
    RecoveryFailed,
    StartupFailed,
    DeliveryFailed,
}

impl SeatAssignmentStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::ReadyForAssignment => "ready-for-assignment",
            Self::Delivering => "delivering",
            Self::DeliveredAwaitingClaim => "delivered-awaiting-claim",
            Self::AssignedPendingAck => "assigned-pending-ack",
            Self::Owned => "owned",
            Self::Recovering => "recovering",
            Self::ClaimTimedOut => "claim-timed-out",
            Self::RecoveryFailed => "recovery-failed",
            Self::StartupFailed => "startup-failed",
            Self::DeliveryFailed => "delivery-failed",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Self::Starting
            | Self::ReadyForAssignment
            | Self::Delivering
            | Self::DeliveredAwaitingClaim
            | Self::AssignedPendingAck => YELLOW,
            Self::Owned => GREEN,
            Self::Recovering => BRIGHT_YELLOW,
            Self::ClaimTimedOut
            | Self::RecoveryFailed
            | Self::StartupFailed
            | Self::DeliveryFailed => RED,
        }
    }
}

/// Information about an agent pane slot for dashboard display.
#[derive(Debug, Clone)]
pub struct SlotInfo {
    pub ticket_id: Option<String>,
    pub slot_number: usize,
    pub transitioning: bool,
}

/// Activity log entry types
#[derive(Debug, Clone)]
pub enum ActivityType {
    PhaseCompleted {
        ticket_id: String,
        phase: Phase,
    },
    Commit {
        ticket_id: String,
        message: String,
    },
    Error {
        ticket_id: String,
        message: String,
    },
    Warning {
        ticket_id: String,
        message: String,
    },
    ThreadStarted {
        ticket_id: String,
        phase: Phase,
    },
    Info {
        ticket_id: String,
        message: String,
    },
    CompletionRejected {
        ticket_id: String,
        kind: CompletionRejectionKind,
        correlation_id: String,
        detail: String,
    },
}

/// A single activity log entry
#[derive(Debug, Clone)]
pub struct ActivityEntry {
    pub timestamp: Duration,
    /// Consecutive identical occurrences folded into this entry; always at
    /// least 1. Greater than one renders as a trailing `(xN)` on the line.
    pub count: u32,
    pub activity: ActivityType,
}

/// Which kind of modal is being shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModalKind {
    #[default]
    MarkDone,
    ResetTicket,
    QuitConfirm,
}

/// Visible lifecycle of one completion request submitted from MarkDone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorModalOutcome {
    Pending {
        ticket_id: String,
        correlation_id: String,
    },
    Accepted {
        ticket_id: String,
        correlation_id: String,
    },
    Rejected {
        ticket_id: String,
        kind: CompletionRejectionKind,
        correlation_id: String,
        detail: String,
    },
}

/// (MarkDone only) The modal's second step: which canned reason signs this
/// ticket, and what that signature answers.
///
/// `cursor` indexes `choices` and nothing else. The ticket being signed is
/// carried here by name rather than read back out of [`ModalState::cursor`],
/// which keeps pointing at the ticket list underneath.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasonStepState {
    pub ticket_id: String,
    /// The state the signature answers, from the override catalog.
    pub ask: OverriddenAsk,
    /// The reasons that honestly fit `ask`, in catalog order.
    pub choices: Vec<OverrideReason>,
    /// Index into `choices`.
    pub cursor: usize,
}

/// State for the modal overlay (UI representation).
#[derive(Debug, Clone, Default)]
pub struct ModalState {
    /// Whether the modal is visible.
    pub open: bool,
    /// Ticket IDs shown in the list (undone tickets for QuitConfirm).
    pub ticket_ids: Vec<String>,
    /// Currently highlighted index.
    pub cursor: usize,
    /// Which modal variant is shown.
    pub kind: ModalKind,
    /// (QuitConfirm only) New ticket IDs not in the current DAG.
    pub new_ticket_ids: Vec<String>,
    /// (MarkDone only) Durable visible feedback for a submitted request.
    pub operator_outcome: Option<OperatorModalOutcome>,
    /// (MarkDone only) The reason step, when the chosen ticket needs a
    /// signature. `None` means the ticket list is showing.
    pub reason_step: Option<ReasonStepState>,
}

/// Which preset view is active on the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewPreset {
    /// Default operational monitoring view.
    #[default]
    Operations,
    /// Dedicated DAG dependency visualization.
    Dag,
    /// Full activity log with all entry types.
    Activity,
}

impl ViewPreset {
    /// Cycle to the next view preset.
    pub fn next(self) -> Self {
        match self {
            ViewPreset::Operations => ViewPreset::Dag,
            ViewPreset::Dag => ViewPreset::Activity,
            ViewPreset::Activity => ViewPreset::Operations,
        }
    }

    /// Human-readable label for the status bar.
    pub fn label(&self) -> &'static str {
        match self {
            ViewPreset::Operations => "Operations",
            ViewPreset::Dag => "DAG",
            ViewPreset::Activity => "Activity",
        }
    }
}

/// The complete plugin state for rendering
#[derive(Debug, Clone)]
pub struct PluginState {
    pub tickets: Vec<TicketNode>,
    pub active_threads: Vec<ActiveThread>,
    pub parked_threads: Vec<ParkedThread>,
    pub waiting_items: Vec<WaitingItem>,
    pub note_items: Vec<NoteItem>,
    pub activity_log: Vec<ActivityEntry>,
    pub alerts: Vec<HealthAlert>,
    pub slots: Vec<SlotInfo>,
    /// Explicit scheduler-owned assignment states keyed by dashboard slot.
    pub seat_assignment_statuses: HashMap<usize, SeatAssignmentStatus>,
    pub current_time: Duration,
    pub modal: ModalState,
    /// Whether scheduling of new tickets is paused.
    pub paused: bool,
    /// Which preset view is currently active.
    pub active_view: ViewPreset,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            tickets: Vec::new(),
            active_threads: Vec::new(),
            parked_threads: Vec::new(),
            waiting_items: Vec::new(),
            note_items: Vec::new(),
            activity_log: Vec::new(),
            alerts: Vec::new(),
            slots: Vec::new(),
            seat_assignment_statuses: HashMap::new(),
            current_time: Duration::ZERO,
            modal: ModalState::default(),
            paused: false,
            active_view: ViewPreset::default(),
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

/// Rendered age for an entry whose emit instant was never recorded.
const UNKNOWN_AGE: &str = "—";

/// Format an activity entry's age in coarse human buckets.
///
/// The activity feed answers "how long ago" in the words a person uses — "just
/// now", "5m ago", "3h ago", "2d ago" — rather than the `{h}h {m}m` composite
/// [`format_time_since`] produces for thread elapsed times.
///
/// An epoch-zero timestamp means the emit instant was never recorded. It renders
/// as a bounded [`UNKNOWN_AGE`] instead of the seconds-since-1970 figure that
/// once surfaced in the feed as `495696h 11m`.
pub(crate) fn format_age_bucket(timestamp: Duration, current_time: Duration) -> String {
    if timestamp.is_zero() {
        return UNKNOWN_AGE.to_string();
    }

    let secs = current_time.saturating_sub(timestamp).as_secs();
    match secs {
        0..=59 => "just now".to_string(),
        60..=3599 => format!("{}m ago", secs / 60),
        3600..=86_399 => format!("{}h ago", secs / 3600),
        _ => format!("{}d ago", secs / 86_400),
    }
}

/// Hang the fold's multiplier on a rendered line.
///
/// A line that absorbed echoes says so — `sweep retried (x3)` — and one that did
/// not is returned untouched, so an unfolded line renders byte-identically to
/// what it rendered before folding existed.
///
/// Callers apply this *after* their own truncation. The multiplier is the part
/// of the line an operator cannot reconstruct from anywhere else, so it must
/// never be the part the `...` eats.
pub(crate) fn with_repeat_tag(message: String, count: u32) -> String {
    if count <= 1 {
        message
    } else {
        format!("{message} (x{count})")
    }
}

/// Render a horizontal separator line
fn render_separator(width: usize) -> String {
    format!("{}{}{}", DIM, "─".repeat(width.min(80)), RESET)
}

// =============================================================================
// Waiting on you
// =============================================================================

/// Render the plain ask before the reviewer's raw note for durable parks.
fn render_waiting_on_you(state: &PluginState, output: &mut Vec<String>) {
    if state.waiting_items.is_empty() {
        return;
    }

    output.push(format!("{BOLD}Waiting on you{RESET}"));
    for item in &state.waiting_items {
        let suffix = if item.checks_on_own {
            " — Lisa checks on its own."
        } else {
            ""
        };
        if let Some(proposal) = &item.proposal {
            output.push(format!(
                "{}  First responder: {}",
                item.ticket_id, proposal.summary
            ));
            output.push(format!("       Suggested: {}", proposal.recommendation));
            output.extend(
                proposal
                    .prepared_steps
                    .iter()
                    .map(|step| format!("       Prepared: {}", step.display())),
            );
            output.push(format!("       Original ask: {}", item.ask));
        } else {
            output.push(format!("{}  {}{}", item.ticket_id, item.ask, suffix));
        }
        output.push(format!("       Reviewer's note: {}", item.reason));
    }
    output.push(String::new());
}

/// Render deferred informational notes distinctly from urgent parked work.
fn render_notes_for_you(state: &PluginState, output: &mut Vec<String>) {
    if state.note_items.is_empty() {
        return;
    }

    output.push(format!(
        "{BOLD}{CYAN}Notes for you ({}){RESET}",
        state.note_items.len()
    ));
    for item in &state.note_items {
        output.push(format!("{}  {}", item.ticket_id, item.summary));
        output.push(format!("       Criterion: “{}”", item.criterion_quote));
        output.push(format!("       Evidence: {}", item.evidence_citation));
    }
    output.push(String::new());
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

    let box_w = width.min(100);
    let inner_w = box_w.saturating_sub(4); // account for "║ " and " ║"

    // Top border
    output.push(format!(
        "{}{}╔{}╗{}",
        BOLD,
        BRIGHT_YELLOW,
        "═".repeat(box_w.saturating_sub(2)),
        RESET
    ));

    // Header line
    let header = "⚠ ATTENTION NEEDED";
    let header_pad = inner_w.saturating_sub(header.chars().count());
    output.push(format!(
        "{}{}║ {}{}{}{}{}{}{} ║{}",
        BOLD,
        BRIGHT_YELLOW,
        BG_YELLOW,
        WHITE,
        header,
        RESET,
        " ".repeat(header_pad),
        BOLD,
        BRIGHT_YELLOW,
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
            "{}{}║{} {}{}{} {}{}{}║{}",
            BOLD,
            BRIGHT_YELLOW,
            RESET,
            YELLOW,
            content,
            RESET,
            " ".repeat(row_pad.saturating_sub(1)),
            BOLD,
            BRIGHT_YELLOW,
            RESET
        ));
    }

    // Health alert rows (stuck/failed sessions)
    for alert in state.alerts.iter().take(15) {
        let (label, color) = match alert.alert_type {
            AlertType::Failed => ("✗ FAILED", RED),
            AlertType::Stuck => ("! STUCK ", YELLOW),
            AlertType::IdleWithoutArtifact => ("⏸ IDLE  ", YELLOW),
            AlertType::TimedOut => ("⏱ TIMEOUT", YELLOW),
        };

        let detail_max = inner_w.saturating_sub(24); // label + space + ticket_id + space
        let detail: String = if alert.detail.chars().count() > detail_max {
            format!(
                "{}..",
                alert
                    .detail
                    .chars()
                    .take(detail_max.saturating_sub(2))
                    .collect::<String>()
            )
        } else {
            alert.detail.clone()
        };

        let content = format!("{} {:<12} {}", label, alert.ticket_id, detail);
        let content_visible_len = content.chars().count();
        let row_pad = inner_w.saturating_sub(content_visible_len);

        output.push(format!(
            "{}{}║{} {}{}{} {}{}{}║{}",
            BOLD,
            BRIGHT_YELLOW,
            RESET,
            color,
            content,
            RESET,
            " ".repeat(row_pad.saturating_sub(1)),
            BOLD,
            BRIGHT_YELLOW,
            RESET
        ));

        // Suggested actions
        if !alert.suggested_actions.is_empty() {
            let actions = format!("  {}", alert.suggested_actions.join(" | "));
            let actions_len = actions.chars().count();
            let actions_pad = inner_w.saturating_sub(actions_len);
            output.push(format!(
                "{}{}║{} {}{}{} {}{}{}║{}",
                BOLD,
                BRIGHT_YELLOW,
                RESET,
                DIM,
                actions,
                RESET,
                " ".repeat(actions_pad.saturating_sub(1)),
                BOLD,
                BRIGHT_YELLOW,
                RESET
            ));
        }
    }

    if state.alerts.len() > 15 {
        let more = format!("... and {} more alerts", state.alerts.len() - 15);
        let pad = inner_w.saturating_sub(more.len());
        output.push(format!(
            "{}{}║{} {}{}{} {}{}{}║{}",
            BOLD,
            BRIGHT_YELLOW,
            RESET,
            DIM,
            more,
            RESET,
            " ".repeat(pad.saturating_sub(1)),
            BOLD,
            BRIGHT_YELLOW,
            RESET
        ));
    }

    // Hint row
    let hint = "Press [d] to mark done";
    let hint_len = hint.chars().count();
    let hint_pad = inner_w.saturating_sub(hint_len);
    output.push(format!(
        "{}{}║{} {}{}{} {}{}{}║{}",
        BOLD,
        BRIGHT_YELLOW,
        RESET,
        DIM,
        hint,
        RESET,
        " ".repeat(hint_pad.saturating_sub(1)),
        BOLD,
        BRIGHT_YELLOW,
        RESET
    ));

    // Bottom border
    output.push(format!(
        "{}{}╚{}╝{}",
        BOLD,
        BRIGHT_YELLOW,
        "═".repeat(box_w.saturating_sub(2)),
        RESET
    ));

    output.push(String::new());
}

// =============================================================================
// DAG Rendering
// =============================================================================

/// Compute DAG layers for visualization (topological sort into layers)
/// Render the DAG using ascii-dag for proper edge routing and layout.
///
/// Filters out completed tickets to keep the view focused on active work.
/// Uses Sugiyama layered layout via ascii-dag for crossing minimization
/// and proper fan-in/fan-out visualization.
fn render_dag(state: &PluginState, output: &mut Vec<String>) {
    output.push(format!("{}{}≡≡ DAG ≡≡{}", BOLD, CYAN, RESET));
    output.push(String::new());

    if state.tickets.is_empty() {
        output.push(format!("{}(no tickets){}", DIM, RESET));
        return;
    }

    // Filter out done tickets — they clutter the view as sessions progress
    let active: Vec<&TicketNode> = state
        .tickets
        .iter()
        .filter(|t| t.status != TicketStatus::Done)
        .collect();

    let done_count = state.tickets.len() - active.len();

    if active.is_empty() {
        output.push(format!(
            "{}All {} tickets complete!{}",
            BRIGHT_GREEN, done_count, RESET
        ));
        return;
    }

    // Build ID-to-index map for active tickets (ascii-dag uses integer IDs)
    let active_ids: std::collections::HashSet<&str> =
        active.iter().map(|t| t.id.as_str()).collect();
    let id_to_int: HashMap<&str, usize> = active
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.as_str(), i + 1)) // ascii-dag IDs are 1-based
        .collect();

    // Build ascii-dag graph
    let nodes: Vec<(usize, String)> = active
        .iter()
        .map(|t| {
            let status_str = match &t.status {
                TicketStatus::Ready => "RDY",
                TicketStatus::InProgress => "WRK",
                TicketStatus::WaitingReview => "REV",
                TicketStatus::Blocked => "BLK",
                TicketStatus::Done => "DON",
            };
            let label = format!("{} {}", t.id, status_str);
            (id_to_int[t.id.as_str()], label)
        })
        .collect();

    let mut edges: Vec<(usize, usize)> = Vec::new();
    for t in &active {
        let child_int = id_to_int[t.id.as_str()];
        for dep in &t.depends_on {
            if active_ids.contains(dep.as_str()) {
                if let Some(&parent_int) = id_to_int.get(dep.as_str()) {
                    edges.push((parent_int, child_int));
                }
            }
        }
    }

    // Build node refs for from_edges
    let node_refs: Vec<(usize, &str)> = nodes
        .iter()
        .map(|(id, label)| (*id, label.as_str()))
        .collect();

    let dag = ascii_dag::DAG::from_edges(&node_refs, &edges);
    let rendered = dag.render();

    // Build a color map for post-processing: ticket_id -> (phase_color, status_color)
    let color_map: HashMap<&str, (&str, &str)> = active
        .iter()
        .map(|t| {
            let status_color = match &t.status {
                TicketStatus::Ready => CYAN,
                TicketStatus::InProgress => GREEN,
                TicketStatus::WaitingReview => BRIGHT_YELLOW,
                TicketStatus::Blocked => RED,
                TicketStatus::Done => BRIGHT_GREEN,
            };
            (t.id.as_str(), (t.phase.color_code(), status_color))
        })
        .collect();

    // Post-process: inject ANSI colors for ticket IDs
    for line in rendered.lines() {
        let mut colored_line = line.to_string();
        for (ticket_id, (phase_color, _status_color)) in &color_map {
            if colored_line.contains(ticket_id) {
                colored_line = colored_line
                    .replace(ticket_id, &format!("{}{}{}", phase_color, ticket_id, RESET));
            }
        }
        output.push(colored_line);
    }

    // Summary + legend
    if done_count > 0 {
        output.push(String::new());
        output.push(format!(
            "{}({} done ticket{} hidden){}",
            DIM,
            done_count,
            if done_count == 1 { "" } else { "s" },
            RESET
        ));
    }
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

/// Render a unified thread table consolidating slot, active, and parked thread info.
///
/// Slot-centric: one row per slot with stable layout. Status includes explicit
/// assignment state plus Awaiting, Running, Parked, Winding Down, and Idle.
pub(crate) fn render_threads(state: &PluginState, output: &mut Vec<String>) {
    output.push(format!("{}{}=== Threads ==={}", BOLD, GREEN, RESET));
    output.push(String::new());

    if state.slots.is_empty() && state.active_threads.is_empty() {
        output.push(format!("{}(no slots){}", DIM, RESET));
        return;
    }

    // Build lookups from slot_number to thread data
    let active_by_slot: HashMap<usize, &ActiveThread> = state
        .active_threads
        .iter()
        .map(|t| (t.slot_number, t))
        .collect();
    let parked_by_slot: HashMap<usize, &ParkedThread> = state
        .parked_threads
        .iter()
        .map(|t| (t.slot_number, t))
        .collect();

    // Header. AGENT surfaces each pane's resolved (provider, model) route
    // (T-026-01); `—` when a thread predates routing.
    output.push(format!(
        "{}{:<6} {:<12} {:<10} {:<14} {:<20} {:<10}{}",
        DIM, "SLOT", "TICKET", "PHASE", "AGENT", "STATUS", "TIME", RESET
    ));
    output.push(format!("{}{}{}", DIM, "-".repeat(76), RESET));

    for slot in &state.slots {
        let slot_label = format!("[{}]", slot.slot_number);

        if let Some(active) = active_by_slot.get(&slot.slot_number) {
            // Running thread in this slot. A pane blocked on AskUserQuestion is
            // exempt from wall-clock reclamation (lib.rs), so it must be clearly
            // marked here — an exempt-but-invisible pane is the bad state to avoid.
            let elapsed = format_time_since(active.started_at, state.current_time);
            let phase_color = active.phase.color_code();
            let ticket_cell = if active.awaiting {
                format!("{} [AWAITING]", active.ticket_id)
            } else {
                active.ticket_id.clone()
            };
            let (status_color, status_text) = if active.awaiting {
                (CYAN, "Awaiting")
            } else if let Some(assignment) = state
                .seat_assignment_statuses
                .get(&slot.slot_number)
                .copied()
            {
                (assignment.color(), assignment.label())
            } else {
                (GREEN, "Running")
            };
            let agent_cell = active.route.as_deref().unwrap_or("—");
            output.push(format!(
                "{:<6} {:<12} {}{:<10}{} {:<14} {}{:<20}{} {}",
                slot_label,
                ticket_cell,
                phase_color,
                active.phase.short_name(),
                RESET,
                agent_cell,
                status_color,
                status_text,
                RESET,
                elapsed,
            ));
        } else if let Some(parked) = parked_by_slot.get(&slot.slot_number) {
            // Parked thread in this slot. ParkedThread carries no route today, so
            // the agent cell is `—` (the running row is where the route shows).
            let elapsed = format_time_since(parked.parked_at, state.current_time);
            let phase_color = parked.phase.color_code();
            output.push(format!(
                "{:<6} {:<12} {}{:<10}{} {:<14} {}{:<20}{} {}",
                slot_label,
                parked.ticket_id,
                phase_color,
                parked.phase.short_name(),
                RESET,
                "—",
                YELLOW,
                "Parked",
                RESET,
                elapsed,
            ));
        } else if slot.transitioning {
            // Slot is winding down or in cooldown
            output.push(format!(
                "{:<6} {}{:<12} {:<10} {:<14} {:<20}{} —",
                slot_label, DIM, "—", "—", "—", "Winding Down", RESET,
            ));
        } else {
            // Idle slot
            output.push(format!(
                "{:<6} {}{:<12} {:<10} {:<14} {:<20}{} —",
                slot_label, DIM, "—", "—", "—", "Idle", RESET,
            ));
        }
    }
}

// =============================================================================
// Activity Log Rendering
// =============================================================================

fn format_completion_rejection(
    ticket_id: &str,
    kind: CompletionRejectionKind,
    correlation_id: &str,
    detail: &str,
) -> String {
    format!(
        "{ticket_id}: {} — {detail} [ref {kind} · {correlation_id}]",
        kind.plain_line()
    )
}

/// Render the activity log
pub(crate) fn render_activity_log(
    state: &PluginState,
    max_entries: usize,
    output: &mut Vec<String>,
) {
    output.push(format!("{}{}=== Recent Activity ==={}", BOLD, BLUE, RESET));
    output.push(String::new());

    if state.activity_log.is_empty() {
        output.push(format!("{}(no recent activity){}", DIM, RESET));
        return;
    }

    // Show most recent entries (reversed, newest first)
    let entries: Vec<_> = state.activity_log.iter().rev().take(max_entries).collect();

    for entry in entries {
        let time_ago = format_age_bucket(entry.timestamp, state.current_time);

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
            ActivityType::CompletionRejected {
                ticket_id,
                kind,
                correlation_id,
                detail,
            } => (
                "⊘",
                BRIGHT_YELLOW,
                format_completion_rejection(ticket_id, *kind, correlation_id, detail),
            ),
        };

        let message = with_repeat_tag(message, entry.count);
        output.push(format!(
            "{}{}{} {:<12} {}{}{}",
            color, icon, RESET, time_ago, color, message, RESET
        ));
    }
}

/// Render a filtered activity log showing only high-priority entries.
///
/// Only includes Error, Warning, CompletionRejected, and PhaseCompleted events — the entries
/// that need human attention. Info, Commit, and ThreadStarted events are
/// available on the dedicated Activity view.
fn render_filtered_activity_log(state: &PluginState, max_entries: usize, output: &mut Vec<String>) {
    output.push(format!(
        "{}{}=== Activity (alerts only) ==={}",
        BOLD, BLUE, RESET
    ));
    output.push(String::new());

    let entries: Vec<_> = state
        .activity_log
        .iter()
        .rev()
        .filter(|e| {
            matches!(
                e.activity,
                ActivityType::PhaseCompleted { .. }
                    | ActivityType::Error { .. }
                    | ActivityType::Warning { .. }
                    | ActivityType::CompletionRejected { .. }
            )
        })
        .take(max_entries)
        .collect();

    if entries.is_empty() {
        output.push(format!("{}(no alerts){}", DIM, RESET));
        return;
    }

    for entry in entries {
        let time_ago = format_age_bucket(entry.timestamp, state.current_time);

        let (icon, color, message) = match &entry.activity {
            ActivityType::PhaseCompleted { ticket_id, phase } => (
                "✓",
                BRIGHT_GREEN,
                format!("{} completed {}", ticket_id, phase.full_name()),
            ),
            ActivityType::Error { ticket_id, message } => {
                let msg = if message.len() > 50 {
                    format!("{}...", &message[..47])
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
            ActivityType::Warning { ticket_id, message } => {
                let msg = if message.len() > 50 {
                    format!("{}...", &message[..47])
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
            ActivityType::CompletionRejected {
                ticket_id,
                kind,
                correlation_id,
                detail,
            } => (
                "⊘",
                BRIGHT_YELLOW,
                format_completion_rejection(ticket_id, *kind, correlation_id, detail),
            ),
            // Other types filtered out above
            _ => continue,
        };

        let message = with_repeat_tag(message, entry.count);
        output.push(format!(
            "{}{}{} {:<12} {}{}{}",
            color, icon, RESET, time_ago, color, message, RESET
        ));
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

    let view_label = state.active_view.label();

    format!(
        "{}[{}]{} {}{}Active: {} | Done: {}/{}{}  {}[p] view  [space] {}  [d] done  [r] reset{}",
        BOLD,
        view_label,
        RESET,
        pause_str,
        slot_str,
        active,
        done,
        total,
        alert_str,
        DIM,
        pause_hint,
        RESET
    )
}

// =============================================================================
// Main Dashboard Rendering
// =============================================================================

/// Render the complete dashboard to a vector of lines.
///
/// Dispatches to a view-specific renderer based on the active preset.
fn render_dashboard_lines(state: &PluginState, width: usize, height: usize) -> Vec<String> {
    let mut output = Vec::new();

    // Title bar with status (always present, all views)
    let status = render_status_line(state);
    output.push(format!(
        "{}{}  LISA Dashboard  {} {}{}",
        BOLD, BG_BLUE, RESET, DIM, status
    ));
    output.push(render_separator(width));

    match state.active_view {
        ViewPreset::Operations => render_operations_view(state, width, height, &mut output),
        ViewPreset::Dag => render_dag_view(state, &mut output),
        ViewPreset::Activity => render_activity_view(state, height, &mut output),
    }

    output
}

/// Operations view: attention banner + unified threads + filtered activity log.
fn render_operations_view(
    state: &PluginState,
    width: usize,
    height: usize,
    output: &mut Vec<String>,
) {
    // Durable parked asks are the first operational content.
    render_waiting_on_you(state, output);

    // Informational notes follow urgent asks but never gate operations.
    render_notes_for_you(state, output);

    // Attention banner (review gate + health alerts)
    render_attention_banner(state, width, output);

    // Unified thread table
    render_threads(state, output);
    output.push(render_separator(width));

    // Filtered activity log (errors, warnings, phase completions only)
    let used_lines = output.len();
    let remaining = height.saturating_sub(used_lines + 2);
    let max_log_entries = remaining.clamp(3, 15);
    render_filtered_activity_log(state, max_log_entries, output);
}

/// DAG view: full dependency graph visualization with all available space.
fn render_dag_view(state: &PluginState, output: &mut Vec<String>) {
    render_dag(state, output);
}

/// Activity view: full activity log with all entry types.
fn render_activity_view(state: &PluginState, height: usize, output: &mut Vec<String>) {
    let used_lines = output.len();
    let remaining = height.saturating_sub(used_lines + 1);
    let max_entries = remaining.max(10);
    render_activity_log(state, max_entries, output);
}

fn wrap_modal_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() <= width {
            current.push(' ');
            current.push_str(word);
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }

        let mut remainder = word;
        while remainder.chars().count() > width {
            let split = remainder
                .char_indices()
                .nth(width)
                .map(|(index, _)| index)
                .unwrap_or(remainder.len());
            lines.push(remainder[..split].to_string());
            remainder = &remainder[split..];
        }
        current.push_str(remainder);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// What the operator reads above the choices: the ask this signature answers.
///
/// The one place the reason step's copy rule lives. A block's `ask` is shown
/// verbatim — the disposition schema already requires it to be one sentence
/// addressed to a person who didn't do the work, with the jargon kept in its
/// `reason` companion. The two fail-closed shapes destructure with `{ .. }` on
/// purpose: the parse failure and the block's technical reason are not merely
/// unused here, they are unreachable, so "never a raw parse error on screen" is
/// checkable by reading this function.
fn ask_header_lines(ask: &OverriddenAsk, width: usize) -> Vec<String> {
    match ask {
        OverriddenAsk::Block { ask, .. } => wrap_modal_text(ask, width),
        OverriddenAsk::NoReviewOnFile => {
            wrap_modal_text("No review was left for this ticket.", width)
        }
        OverriddenAsk::UnreadableReview { .. } => {
            wrap_modal_text("No review Lisa can read was left for this ticket.", width)
        }
    }
}

/// Fit one line to `width` display columns, marking any cut with an ellipsis.
///
/// Measured in characters, not bytes: every catalog summary carries an em dash,
/// and byte length would over-pad the box.
fn fit_modal_line(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut fitted: String = text.chars().take(width.saturating_sub(1)).collect();
    fitted.push('…');
    fitted
}

/// Render the reason step: the ask above, the canned choices below.
///
/// One choice per row, truncated rather than wrapped — a three-item list read at
/// a glance is the point, and no sentence is lost: the chosen reason's full text
/// is what the receipt records.
fn render_reason_step_modal(step: &ReasonStepState, width: usize, height: usize) -> Vec<String> {
    let box_w = width.min(50);
    let inner_w = box_w.saturating_sub(4);

    let header = ask_header_lines(&step.ask, inner_w);
    let choices: Vec<String> = step
        .choices
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            let prefix = if index == step.cursor { "▸ " } else { "  " };
            fit_modal_line(&format!("{prefix}{}", reason.summary()), inner_w)
        })
        .collect();

    let border_h = "─".repeat(box_w.saturating_sub(2));
    let box_h = header.len() + choices.len() + 6;
    let pad_top = height.saturating_sub(box_h) / 2;
    let mut output = vec![String::new(); pad_top];

    let centered = |text: &str, decoration: &str| {
        let pad = box_w.saturating_sub(2).saturating_sub(text.chars().count());
        let left = pad / 2;
        format!(
            "│{}{}{}{}{}│",
            " ".repeat(left),
            decoration,
            text,
            RESET,
            " ".repeat(pad - left),
        )
    };

    output.push(format!("┌{}┐", border_h));
    output.push(centered(&format!(" Sign {} ", step.ticket_id), BOLD));
    output.push(format!("├{}┤", border_h));

    for line in &header {
        output.push(format!(
            "│ {}{} │",
            line,
            " ".repeat(inner_w.saturating_sub(line.chars().count()))
        ));
    }

    output.push(format!("├{}┤", border_h));

    for (index, line) in choices.iter().enumerate() {
        let pad = " ".repeat(inner_w.saturating_sub(line.chars().count()));
        if index == step.cursor {
            output.push(format!("│ {BOLD}{CYAN}{line}{RESET}{pad} │"));
        } else {
            output.push(format!("│ {line}{pad} │"));
        }
    }

    output.push(format!("├{}┤", border_h));
    output.push(centered(" Enter=sign  Esc=back ", DIM));
    output.push(format!("└{}┘", border_h));
    output
}

fn render_operator_outcome_modal(
    outcome: &OperatorModalOutcome,
    width: usize,
    height: usize,
) -> Vec<String> {
    let box_w = width.min(50);
    let inner_w = box_w.saturating_sub(4);
    let (ticket_id, correlation_id, status, color, detail, kind, pending) = match outcome {
        OperatorModalOutcome::Pending {
            ticket_id,
            correlation_id,
        } => (
            ticket_id,
            correlation_id,
            "Completion pending".to_string(),
            YELLOW,
            None,
            None,
            true,
        ),
        OperatorModalOutcome::Accepted {
            ticket_id,
            correlation_id,
        } => (
            ticket_id,
            correlation_id,
            "Completion accepted".to_string(),
            BRIGHT_GREEN,
            None,
            None,
            false,
        ),
        OperatorModalOutcome::Rejected {
            ticket_id,
            kind,
            correlation_id,
            detail,
        } => (
            ticket_id,
            correlation_id,
            "Not finished yet".to_string(),
            RED,
            Some(detail.as_str()),
            Some(*kind),
            false,
        ),
    };

    let mut body = vec![format!("Ticket: {ticket_id}"), status.clone()];
    if let Some(kind) = kind {
        body.extend(wrap_modal_text(kind.plain_line(), inner_w));
    }
    if let Some(detail) = detail {
        body.extend(wrap_modal_text(&format!("Note: {detail}"), inner_w));
        if kind == Some(CompletionRejectionKind::DispositionBlocked) {
            body.extend(wrap_modal_text(
                "You can paste this note to your coding agent.",
                inner_w,
            ));
        }
    }
    let reference = match kind {
        Some(kind) => format!("Ref: {kind} · {correlation_id}"),
        None => format!("Ref: {correlation_id}"),
    };
    body.extend(wrap_modal_text(&reference, inner_w));

    let box_h = body.len() + 5;
    let pad_top = height.saturating_sub(box_h) / 2;
    let border_h = "─".repeat(box_w.saturating_sub(2));
    let mut output = vec![String::new(); pad_top];
    output.push(format!("┌{}┐", border_h));

    let title = " Operator Completion ";
    let title_pad = box_w.saturating_sub(2).saturating_sub(title.len());
    let left_pad = title_pad / 2;
    output.push(format!(
        "│{}{}{}{}{}│",
        " ".repeat(left_pad),
        BOLD,
        title,
        RESET,
        " ".repeat(title_pad - left_pad),
    ));
    output.push(format!("├{}┤", border_h));

    for line in body {
        let visible_len = line.chars().count();
        let decorated = if line == status {
            format!("{color}{BOLD}{line}{RESET}")
        } else {
            line
        };
        output.push(format!(
            "│ {}{} │",
            decorated,
            " ".repeat(inner_w.saturating_sub(visible_len))
        ));
    }

    output.push(format!("├{}┤", border_h));
    let footer = if pending {
        " Waiting for completion result "
    } else {
        " Enter/Esc=close "
    };
    let footer_pad = box_w.saturating_sub(2).saturating_sub(footer.len());
    let footer_left = footer_pad / 2;
    output.push(format!(
        "│{}{}{}{}{}│",
        " ".repeat(footer_left),
        DIM,
        footer,
        RESET,
        " ".repeat(footer_pad - footer_left),
    ));
    output.push(format!("└{}┘", border_h));
    output
}

/// Render a modal overlay (mark-done, reset-ticket, or quit-confirm).
fn render_modal(modal: &ModalState, width: usize, height: usize) -> Vec<String> {
    if modal.kind == ModalKind::QuitConfirm {
        return render_quit_confirm_modal(modal, width, height);
    }
    if modal.kind == ModalKind::MarkDone {
        // Outcome first: a submitted request has already cleared the step, so
        // the two are never both set — the ordering states the precedence
        // rather than relying on that.
        if let Some(outcome) = modal.operator_outcome.as_ref() {
            return render_operator_outcome_modal(outcome, width, height);
        }
        if let Some(step) = modal.reason_step.as_ref() {
            return render_reason_step_modal(step, width, height);
        }
    }

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
    let title = if modal.kind == ModalKind::ResetTicket {
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
        BOLD,
        title,
        RESET,
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
        DIM,
        footer,
        RESET,
        " ".repeat(fr),
    ));
    // Bottom border
    output.push(format!("└{}┘", border_h));

    output
}

/// Render the quit confirmation modal.
///
/// Shows undone tickets (current DAG) and new tickets (not yet in DAG),
/// with Enter=keep working, q=quit.
fn render_quit_confirm_modal(modal: &ModalState, width: usize, height: usize) -> Vec<String> {
    let mut output = Vec::new();

    let box_w = width.min(50);
    let inner_w = box_w.saturating_sub(2);

    // Calculate content lines
    let has_undone = !modal.ticket_ids.is_empty();
    let has_new = !modal.new_ticket_ids.is_empty();
    // content: warning line + blank + optional sections + blank before footer
    let undone_lines = if has_undone {
        2 + modal.ticket_ids.len() // header + tickets + blank
    } else {
        0
    };
    let new_lines = if has_new {
        2 + modal.new_ticket_ids.len() // header + tickets + blank
    } else {
        0
    };
    let content_lines = 1 + undone_lines + new_lines; // warning line + sections
    let box_h = content_lines + 4; // top border + title + separator + content + footer-sep + footer + bottom border
    let pad_top = height.saturating_sub(box_h) / 2;

    let border_h = "─".repeat(inner_w);

    // Helper: pad a visible string to fill the box interior
    let pad_line = |visible: &str, visible_len: usize| -> String {
        let pad = inner_w.saturating_sub(visible_len);
        format!("│{}{}│", visible, " ".repeat(pad))
    };

    // Top padding
    for _ in 0..pad_top {
        output.push(String::new());
    }

    // Top border
    output.push(format!("┌{}┐", border_h));

    // Title
    let title = " Quit Lisa? ";
    let title_pad = inner_w.saturating_sub(title.len());
    let tl = title_pad / 2;
    let tr = title_pad - tl;
    output.push(format!(
        "│{}{}{}{}{}│",
        " ".repeat(tl),
        BOLD,
        title,
        RESET,
        " ".repeat(tr),
    ));

    // Separator
    output.push(format!("├{}┤", border_h));

    // Warning line
    let warn = "  There is pending work:";
    output.push(pad_line(
        &format!("{}{}{}", YELLOW, warn, RESET),
        warn.len(),
    ));

    // Undone tickets section
    if has_undone {
        output.push(pad_line("", 0));
        let hdr = "  In progress (current DAG):";
        output.push(pad_line(&format!("{}{}{}", DIM, hdr, RESET), hdr.len()));
        for tid in &modal.ticket_ids {
            let entry = format!("    {}", tid);
            output.push(pad_line(&entry, entry.len()));
        }
    }

    // New tickets section
    if has_new {
        output.push(pad_line("", 0));
        let hdr = "  New tickets (not yet scheduled):";
        output.push(pad_line(&format!("{}{}{}", DIM, hdr, RESET), hdr.len()));
        for tid in &modal.new_ticket_ids {
            let entry = format!("    {}", tid);
            output.push(pad_line(&entry, entry.len()));
        }
    }

    // Footer
    output.push(format!("├{}┤", border_h));
    let footer = " Enter=keep working  q=quit ";
    let footer_pad = inner_w.saturating_sub(footer.len());
    let fl = footer_pad / 2;
    let fr = footer_pad - fl;
    output.push(format!(
        "│{}{}{}{}{}│",
        " ".repeat(fl),
        DIM,
        footer,
        RESET,
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
                slot_number: 1,
                awaiting: false,
                route: None,
            }],
            parked_threads: vec![],
            waiting_items: vec![],
            note_items: vec![],
            activity_log: vec![
                ActivityEntry {
                    timestamp: Duration::from_secs(30),
                    count: 1,
                    activity: ActivityType::PhaseCompleted {
                        ticket_id: "T-001".to_string(),
                        phase: Phase::Implement,
                    },
                },
                ActivityEntry {
                    timestamp: Duration::from_secs(60),
                    count: 1,
                    activity: ActivityType::ThreadStarted {
                        ticket_id: "T-002".to_string(),
                        phase: Phase::Design,
                    },
                },
            ],
            alerts: Vec::new(),
            slots: Vec::new(),
            seat_assignment_statuses: HashMap::new(),
            current_time: Duration::from_secs(120),
            modal: ModalState::default(),
            paused: false,
            active_view: ViewPreset::default(),
        }
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3700)), "1h 1m");
    }

    /// A fixed 2026-era wall clock, so age assertions never depend on the real one.
    const TEST_NOW_SECS: u64 = 1_800_000_000;

    fn age_at(elapsed_secs: u64) -> String {
        format_age_bucket(
            Duration::from_secs(TEST_NOW_SECS - elapsed_secs),
            Duration::from_secs(TEST_NOW_SECS),
        )
    }

    #[test]
    fn format_age_bucket_covers_the_four_shapes() {
        assert_eq!(age_at(0), "just now");
        assert_eq!(age_at(5 * 60), "5m ago");
        assert_eq!(age_at(3 * 3600), "3h ago");
        assert_eq!(age_at(2 * 86_400), "2d ago");
    }

    #[test]
    fn format_age_bucket_boundaries_are_exact() {
        assert_eq!(age_at(59), "just now");
        assert_eq!(age_at(60), "1m ago");
        assert_eq!(age_at(61), "1m ago");
        assert_eq!(age_at(3599), "59m ago");
        assert_eq!(age_at(3600), "1h ago");
        assert_eq!(age_at(86_399), "23h ago");
        assert_eq!(age_at(86_400), "1d ago");
    }

    #[test]
    fn format_age_bucket_renders_epoch_zero_as_bounded_fallback() {
        // The exact shape that produced `495696h 11m` in the 2026-07 field bug:
        // a never-stamped entry measured against a genuine wall clock.
        let rendered = format_age_bucket(Duration::ZERO, Duration::from_secs(TEST_NOW_SECS));

        assert_eq!(rendered, UNKNOWN_AGE);
        assert!(
            !rendered.contains('h'),
            "epoch-zero entry must never render an hours figure, got {rendered}"
        );
        assert!(
            rendered.chars().count() <= 12,
            "fallback must stay inside the age column, got {rendered}"
        );
    }

    #[test]
    fn format_age_bucket_clamps_future_timestamps() {
        // Backwards clock skew must not surface as a wrong large number.
        let rendered = format_age_bucket(
            Duration::from_secs(TEST_NOW_SECS + 600),
            Duration::from_secs(TEST_NOW_SECS),
        );
        assert_eq!(rendered, "just now");
    }

    #[test]
    fn activity_feed_renders_only_bucket_shapes() {
        let cases = [
            (0_u64, "just now"),
            (5 * 60, "5m ago"),
            (3 * 3600, "3h ago"),
            (2 * 86_400, "2d ago"),
        ];
        // PhaseCompleted survives the alerts-only filter, so one fixture drives
        // both renderers.
        let activity_log = cases
            .iter()
            .map(|(elapsed, _)| ActivityEntry {
                timestamp: Duration::from_secs(TEST_NOW_SECS - elapsed),
                count: 1,
                activity: ActivityType::PhaseCompleted {
                    ticket_id: format!("T-AGE-{elapsed}"),
                    phase: Phase::Implement,
                },
            })
            .collect::<Vec<_>>();

        let state = PluginState {
            activity_log,
            current_time: Duration::from_secs(TEST_NOW_SECS),
            ..PluginState::default()
        };

        let mut full = Vec::new();
        render_activity_log(&state, cases.len(), &mut full);
        let full = full.join("\n");
        let mut alerts = Vec::new();
        render_filtered_activity_log(&state, cases.len(), &mut alerts);
        let alerts = alerts.join("\n");

        for (view_name, view) in [("full Activity", &full), ("alerts-only", &alerts)] {
            for (_, expected) in cases {
                // The renderers lay the age out as `{:<12}`; matching the padded
                // column pins the age position, not an incidental substring.
                let column = format!("{:<12}", expected);
                assert!(
                    view.contains(&column),
                    "{view_name} view lost the {expected:?} bucket: {view}"
                );
            }
            assert!(
                !view.contains("495696h"),
                "{view_name} view regressed to the epoch composite: {view}"
            );
        }
    }

    /// Render one entry through both activity views.
    ///
    /// `PhaseCompleted` and `Warning` both survive the alerts-only filter, so a
    /// single fixture drives both renderers — the pattern
    /// `activity_feed_renders_only_bucket_shapes` established.
    fn both_activity_views(entry: ActivityEntry) -> [(&'static str, String); 2] {
        let state = PluginState {
            activity_log: vec![entry],
            current_time: Duration::from_secs(TEST_NOW_SECS),
            ..PluginState::default()
        };

        let mut full = Vec::new();
        render_activity_log(&state, 5, &mut full);
        let mut alerts = Vec::new();
        render_filtered_activity_log(&state, 5, &mut alerts);

        [
            ("full Activity", full.join("\n")),
            ("alerts-only", alerts.join("\n")),
        ]
    }

    #[test]
    fn folded_entry_renders_the_multiplier_in_both_views() {
        let views = both_activity_views(ActivityEntry {
            timestamp: Duration::from_secs(TEST_NOW_SECS - 60),
            count: 3,
            activity: ActivityType::PhaseCompleted {
                ticket_id: "T-FOLD".to_string(),
                phase: Phase::Implement,
            },
        });

        for (view_name, view) in views {
            assert!(
                view.contains("T-FOLD completed Implement (x3)"),
                "{view_name} view dropped the multiplier: {view}"
            );
        }
    }

    #[test]
    fn single_occurrence_renders_without_a_tag() {
        let views = both_activity_views(ActivityEntry {
            timestamp: Duration::from_secs(TEST_NOW_SECS - 60),
            count: 1,
            activity: ActivityType::PhaseCompleted {
                ticket_id: "T-ONCE".to_string(),
                phase: Phase::Implement,
            },
        });

        for (view_name, view) in views {
            assert!(
                view.contains("T-ONCE completed Implement"),
                "{view_name} view lost the line: {view}"
            );
            assert!(
                !view.contains("(x"),
                "{view_name} view tagged an unfolded line: {view}"
            );
        }
    }

    /// The tag goes on after truncation. Both views cut free text (40 chars in
    /// the full feed, 50 in alerts) — the multiplier is the one part of the line
    /// nothing else can reconstruct, so the `...` must never eat it.
    #[test]
    fn the_multiplier_survives_message_truncation() {
        let views = both_activity_views(ActivityEntry {
            timestamp: Duration::from_secs(TEST_NOW_SECS - 60),
            count: 2,
            activity: ActivityType::Warning {
                ticket_id: String::new(),
                message: "a warning long enough to be cut by either view's truncation rule"
                    .to_string(),
            },
        });

        for (view_name, view) in views {
            assert!(
                view.contains("..."),
                "{view_name} fixture must actually truncate: {view}"
            );
            assert!(
                view.contains("... (x2)"),
                "{view_name} view must tag after truncating, not before: {view}"
            );
        }
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
    fn waiting_section_preserves_operator_ask_and_explains_world_waiting() {
        let state = PluginState {
            waiting_items: vec![
                WaitingItem {
                    ticket_id: "T-ASK".to_string(),
                    ask: "Run the checkout test exactly once.".to_string(),
                    reason: "The checkout evidence is missing.".to_string(),
                    checks_on_own: false,
                    proposal: None,
                },
                WaitingItem {
                    ticket_id: "T-WORLD".to_string(),
                    ask: "Wait for the release link.".to_string(),
                    reason: "The release has not reached the mirror.".to_string(),
                    checks_on_own: true,
                    proposal: None,
                },
            ],
            ..PluginState::default()
        };
        let mut output = Vec::new();

        render_waiting_on_you(&state, &mut output);

        let full = output.join("\n");
        assert!(full.contains("Waiting on you"));
        assert!(full.contains("T-ASK  Run the checkout test exactly once."));
        assert!(full.contains("Reviewer's note: The checkout evidence is missing."));
        assert!(full.contains("T-WORLD  Wait for the release link. — Lisa checks on its own."));
        assert!(full.contains("Reviewer's note: The release has not reached the mirror."));
        assert!(!full.contains("operator"));
        assert!(!full.contains("remedy_owner"));
    }

    #[test]
    fn legacy_field_block_never_puts_the_raw_reason_first() {
        const FIELD_REASON: &str = "The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant bypassed the old binary through managed mode instead of recording the required recovery through Lisa's error strings; John must either provide conforming reruns or explicitly amend both acceptance requirements before Review can pass.";
        let state = PluginState {
            waiting_items: vec![WaitingItem {
                ticket_id: "T-046-06-03".to_string(),
                ask: lisa_core::parking::LEGACY_BLOCK_ASK.to_string(),
                reason: FIELD_REASON.to_string(),
                checks_on_own: false,
                proposal: None,
            }],
            ..PluginState::default()
        };
        let mut output = Vec::new();

        render_waiting_on_you(&state, &mut output);

        assert_eq!(output.len(), 4);
        assert_eq!(
            output[1],
            format!("T-046-06-03  {}", lisa_core::parking::LEGACY_BLOCK_ASK)
        );
        assert!(!output[1].contains(FIELD_REASON));
        assert_eq!(output[2], format!("       Reviewer's note: {FIELD_REASON}"));
        assert!(output[3].is_empty());
    }

    #[test]
    fn waiting_section_is_empty_without_human_or_world_items() {
        let mut output = Vec::new();
        render_waiting_on_you(&PluginState::default(), &mut output);
        assert!(output.is_empty());
    }

    #[test]
    fn notes_section_leads_with_summary_then_citations() {
        let state = PluginState {
            note_items: vec![NoteItem {
                ticket_id: "T-046-06-03".to_string(),
                summary: "The recorded measurement and criterion text disagree.".to_string(),
                criterion_quote: "approximately 200 MiB".to_string(),
                evidence_citation: "review.md#measurement".to_string(),
            }],
            ..PluginState::default()
        };
        let mut output = Vec::new();

        render_notes_for_you(&state, &mut output);

        assert!(output[0].contains("Notes for you (1)"));
        assert_eq!(
            output[1],
            "T-046-06-03  The recorded measurement and criterion text disagree."
        );
        assert_eq!(output[2], "       Criterion: “approximately 200 MiB”");
        assert_eq!(output[3], "       Evidence: review.md#measurement");
        assert!(output[4].is_empty());
    }

    #[test]
    fn notes_section_is_empty_without_active_notes() {
        let mut output = Vec::new();
        render_notes_for_you(&PluginState::default(), &mut output);
        assert!(output.is_empty());
    }

    #[test]
    fn notes_are_distinct_from_urgent_waiting_and_precede_operations() {
        let state = PluginState {
            waiting_items: vec![WaitingItem {
                ticket_id: "T-WAIT".to_string(),
                ask: "Run the release check.".to_string(),
                reason: "Release evidence is absent.".to_string(),
                checks_on_own: false,
                proposal: None,
            }],
            note_items: vec![NoteItem {
                ticket_id: "T-NOTE".to_string(),
                summary: "The criterion text differs from the measurement.".to_string(),
                criterion_quote: "200 MiB".to_string(),
                evidence_citation: "review.md#size".to_string(),
            }],
            alerts: vec![HealthAlert {
                ticket_id: "T-FAILED".to_string(),
                alert_type: AlertType::Failed,
                detail: "Session failed".to_string(),
                suggested_actions: vec![],
            }],
            ..PluginState::default()
        };

        let full = render_dashboard_lines(&state, 80, 50).join("\n");
        let waiting = full.find("Waiting on you").unwrap();
        let notes = full.find("Notes for you").unwrap();
        let attention = full.find("ATTENTION NEEDED").unwrap();
        let threads = full.find("Threads").unwrap();
        assert!(waiting < notes);
        assert!(notes < attention);
        assert!(notes < threads);
    }

    #[test]
    fn first_responder_proposal_precedes_original_ask_and_raw_reason() {
        use lisa_core::triage::{PreparedStep, TriageProposal};

        let state = PluginState {
            waiting_items: vec![WaitingItem {
                ticket_id: "T-046-06-03".to_string(),
                ask: lisa_core::parking::LEGACY_BLOCK_ASK.to_string(),
                reason: "225 MiB conflicts with approximately 200 MiB.".to_string(),
                checks_on_own: false,
                proposal: Some(TriageProposal {
                    summary: "The written criteria conflict with the measured evidence."
                        .to_string(),
                    recommendation: "Amend the stale criteria.".to_string(),
                    prepared_steps: vec![PreparedStep::FileEdit {
                        description: "Use the calibrated bound.".to_string(),
                        path: std::path::PathBuf::from("docs/active/tickets/T-046-06-03.md"),
                        old: "approximately 200 MiB".to_string(),
                        new: "the calibrated 300 MiB bound".to_string(),
                    }],
                }),
            }],
            ..PluginState::default()
        };
        let mut output = Vec::new();
        render_waiting_on_you(&state, &mut output);
        let joined = output.join("\n");
        let summary = joined.find("First responder:").unwrap();
        let suggested = joined.find("Suggested:").unwrap();
        let prepared = joined.find("Prepared:").unwrap();
        let original = joined.find("Original ask:").unwrap();
        let raw = joined.find("Reviewer's note:").unwrap();
        assert!(summary < suggested);
        assert!(suggested < prepared);
        assert!(prepared < original);
        assert!(original < raw);
    }

    #[test]
    fn waiting_section_is_first_operations_content() {
        let state = PluginState {
            waiting_items: vec![WaitingItem {
                ticket_id: "T-ASK".to_string(),
                ask: "Run the release test.".to_string(),
                reason: "The release evidence is missing.".to_string(),
                checks_on_own: false,
                proposal: None,
            }],
            alerts: vec![HealthAlert {
                ticket_id: "T-FAILED".to_string(),
                alert_type: AlertType::Failed,
                detail: "Session failed".to_string(),
                suggested_actions: vec![],
            }],
            ..PluginState::default()
        };

        let full = render_dashboard_lines(&state, 80, 50).join("\n");
        let waiting = full.find("Waiting on you").unwrap();
        let attention = full.find("ATTENTION NEEDED").unwrap();
        let threads = full.find("Threads").unwrap();
        assert!(waiting < attention);
        assert!(waiting < threads);
    }

    #[test]
    fn test_render_threads_with_active() {
        let mut state = sample_state();
        state.slots = vec![
            SlotInfo {
                ticket_id: Some("T-002".to_string()),
                slot_number: 1,
                transitioning: false,
            },
            SlotInfo {
                ticket_id: None,
                slot_number: 2,
                transitioning: false,
            },
        ];
        let mut output = Vec::new();
        render_threads(&state, &mut output);

        let full = output.join("\n");
        assert!(full.contains("Threads"), "Header missing");
        assert!(full.contains("T-002"), "Active ticket missing");
        assert!(full.contains("Running"), "Running status missing");
        assert!(full.contains("Idle"), "Idle slot missing");
    }

    #[test]
    fn test_render_threads_marks_awaiting() {
        let state = PluginState {
            slots: vec![SlotInfo {
                ticket_id: Some("T-002".to_string()),
                slot_number: 1,
                transitioning: false,
            }],
            active_threads: vec![ActiveThread {
                ticket_id: "T-002".to_string(),
                phase: Phase::Design,
                started_at: Duration::from_secs(60),
                slot_number: 1,
                awaiting: true,
                route: None,
            }],
            ..PluginState::default()
        };
        let mut output = Vec::new();
        render_threads(&state, &mut output);

        let full = output.join("\n");
        assert!(full.contains("[AWAITING]"), "awaiting token missing");
        assert!(full.contains("Awaiting"), "awaiting status missing");
        assert!(
            !full.contains("Running"),
            "an awaiting thread must not show Running status"
        );
    }

    #[test]
    fn test_render_threads_surfaces_route() {
        // T-026-01: the AGENT column shows each pane's (provider, model) route.
        let state = PluginState {
            slots: vec![
                SlotInfo {
                    ticket_id: Some("T-001".to_string()),
                    slot_number: 1,
                    transitioning: false,
                },
                SlotInfo {
                    ticket_id: Some("T-002".to_string()),
                    slot_number: 2,
                    transitioning: false,
                },
            ],
            active_threads: vec![
                ActiveThread {
                    ticket_id: "T-001".to_string(),
                    phase: Phase::Research,
                    started_at: Duration::from_secs(30),
                    slot_number: 1,
                    awaiting: false,
                    route: Some("codex/gpt-5".to_string()),
                },
                ActiveThread {
                    ticket_id: "T-002".to_string(),
                    phase: Phase::Design,
                    started_at: Duration::from_secs(30),
                    slot_number: 2,
                    awaiting: false,
                    route: None,
                },
            ],
            ..PluginState::default()
        };
        let mut output = Vec::new();
        render_threads(&state, &mut output);
        let full = output.join("\n");
        assert!(full.contains("AGENT"), "AGENT column header missing");
        assert!(full.contains("codex/gpt-5"), "routed pane's route missing");
        // The unrouted pane shows a dash placeholder in the AGENT column.
        assert!(full.contains('—'), "unrouted pane should show a dash");
    }

    #[test]
    fn test_render_threads_empty() {
        let state = PluginState::default();
        let mut output = Vec::new();
        render_threads(&state, &mut output);

        assert!(output.iter().any(|line| line.contains("no slots")));
    }

    #[test]
    fn test_render_threads_with_parked() {
        let state = PluginState {
            slots: vec![SlotInfo {
                ticket_id: Some("T-003".to_string()),
                slot_number: 1,
                transitioning: false,
            }],
            parked_threads: vec![ParkedThread {
                ticket_id: "T-003".to_string(),
                phase: Phase::Review,
                artifact_path: "docs/active/work/T-003/design.md".to_string(),
                parked_at: Duration::from_secs(100),
                slot_number: 1,
            }],
            current_time: Duration::from_secs(200),
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_threads(&state, &mut output);

        let full = output.join("\n");
        assert!(full.contains("T-003"), "Parked ticket missing");
        assert!(full.contains("Parked"), "Parked status missing");
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
    fn completion_rejections_render_distinct_kinds_and_correlations_in_both_activity_views() {
        let cases = [
            (CompletionRejectionKind::AlreadyPending, "corr-pending"),
            (CompletionRejectionKind::StaleLease, "corr-stale"),
            (
                CompletionRejectionKind::DispositionBlocked,
                "corr-disposition",
            ),
            (
                CompletionRejectionKind::DependencyBlocked,
                "corr-dependency",
            ),
            (CompletionRejectionKind::LaunchFailed, "corr-launch"),
        ];
        let activity_log = cases
            .iter()
            .enumerate()
            .map(|(index, (kind, correlation_id))| ActivityEntry {
                timestamp: Duration::from_secs(index as u64),
                count: 1,
                activity: ActivityType::CompletionRejected {
                    ticket_id: format!("T-REJECT-{index}"),
                    kind: *kind,
                    correlation_id: (*correlation_id).to_string(),
                    detail: format!("actionable detail {index}"),
                },
            })
            .collect::<Vec<_>>();
        assert!(activity_log
            .iter()
            .all(|entry| matches!(entry.activity, ActivityType::CompletionRejected { .. })));

        let state = PluginState {
            activity_log,
            current_time: Duration::from_secs(60),
            ..PluginState::default()
        };
        let mut full = Vec::new();
        render_activity_log(&state, cases.len(), &mut full);
        let full = full.join("\n");
        let mut alerts = Vec::new();
        render_filtered_activity_log(&state, cases.len(), &mut alerts);
        let alerts = alerts.join("\n");

        for (kind, correlation_id) in cases {
            let label = kind.to_string();
            assert!(
                full.contains(&label) && full.contains(correlation_id),
                "full Activity view lost {label} or {correlation_id}: {full}"
            );
            assert!(
                alerts.contains(&label) && alerts.contains(correlation_id),
                "Operations activity view lost {label} or {correlation_id}: {alerts}"
            );
        }
    }

    #[test]
    fn test_render_dag_filters_done_tickets() {
        let state = sample_state(); // T-001 is Done, T-002 InProgress, T-003 Blocked
        let mut output = Vec::new();
        render_dag(&state, &mut output);
        let full = output.join("\n");

        // T-001 is Done — should be filtered out, mentioned in hidden count
        assert!(
            full.contains("1 done ticket"),
            "Should show hidden done ticket count"
        );
        // Active tickets should still appear
        assert!(full.contains("T-002"), "Active ticket T-002 missing");
        assert!(full.contains("T-003"), "Active ticket T-003 missing");
    }

    #[test]
    fn test_render_dag_all_done() {
        let state = PluginState {
            tickets: vec![TicketNode {
                id: "T-001".to_string(),
                title: "Done ticket".to_string(),
                phase: Phase::Done,
                status: TicketStatus::Done,
                depends_on: vec![],
            }],
            ..PluginState::default()
        };
        let mut output = Vec::new();
        render_dag(&state, &mut output);
        let full = output.join("\n");

        assert!(
            full.contains("All 1 tickets complete"),
            "Should show completion message"
        );
    }

    #[test]
    fn test_render_dag_diamond_has_edge_routing() {
        // Diamond: T-001 -> {T-002, T-003} -> T-004
        // ascii-dag should render horizontal connectors between fan-out and fan-in
        let state = PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-001".to_string(),
                    title: "root".to_string(),
                    phase: Phase::Implement,
                    status: TicketStatus::InProgress,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-002".to_string(),
                    title: "left".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Blocked,
                    depends_on: vec!["T-001".to_string()],
                },
                TicketNode {
                    id: "T-003".to_string(),
                    title: "right".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Blocked,
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
            ..PluginState::default()
        };
        let mut output = Vec::new();
        render_dag(&state, &mut output);
        let full = output.join("\n");

        // All four tickets should appear (none are done)
        assert!(full.contains("T-001"), "T-001 missing");
        assert!(full.contains("T-002"), "T-002 missing");
        assert!(full.contains("T-003"), "T-003 missing");
        assert!(full.contains("T-004"), "T-004 missing");

        // ascii-dag renders horizontal edge routing characters for fan-out/fan-in
        // Unlike old renderer which only had vertical │ pipes
        let has_horizontal = full.contains('─') || full.contains('└') || full.contains('┌');
        assert!(
            has_horizontal,
            "Diamond DAG should have horizontal edge connectors, got:\n{}",
            full
        );

        // Should have arrow indicators showing flow direction
        assert!(
            full.contains('↓'),
            "Should have directional arrows, got:\n{}",
            full
        );
    }

    #[test]
    fn test_render_dag_fan_in_convergence() {
        // Three roots converge into one leaf — tests fan-in rendering
        // A ──┐
        // B ──┤ -> D
        // C ──┘
        let state = PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-A".to_string(),
                    title: "a".to_string(),
                    phase: Phase::Implement,
                    status: TicketStatus::InProgress,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-B".to_string(),
                    title: "b".to_string(),
                    phase: Phase::Design,
                    status: TicketStatus::InProgress,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-C".to_string(),
                    title: "c".to_string(),
                    phase: Phase::Research,
                    status: TicketStatus::InProgress,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-D".to_string(),
                    title: "d".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Blocked,
                    depends_on: vec!["T-A".to_string(), "T-B".to_string(), "T-C".to_string()],
                },
            ],
            ..PluginState::default()
        };
        let mut output = Vec::new();
        render_dag(&state, &mut output);
        let full = output.join("\n");

        // All three roots and the leaf should appear
        assert!(full.contains("T-A"), "T-A missing");
        assert!(full.contains("T-B"), "T-B missing");
        assert!(full.contains("T-C"), "T-C missing");
        assert!(full.contains("T-D"), "T-D missing");

        // Fan-in of 3 parents into 1 child must have horizontal routing
        let has_horizontal = full.contains('─') || full.contains('└') || full.contains('┌');
        assert!(
            has_horizontal,
            "Fan-in of 3 parents should produce horizontal connectors, got:\n{}",
            full
        );

        // No done tickets hidden
        assert!(
            !full.contains("done ticket"),
            "No tickets are done, shouldn't show hidden count"
        );
    }

    #[test]
    fn test_render_dag_independent_chains_both_shown() {
        // Two completely independent chains — both should render
        // Chain 1: T-001 -> T-002
        // Chain 2: T-003 -> T-004
        let state = PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-001".to_string(),
                    title: "chain1-root".to_string(),
                    phase: Phase::Implement,
                    status: TicketStatus::InProgress,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-002".to_string(),
                    title: "chain1-leaf".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Blocked,
                    depends_on: vec!["T-001".to_string()],
                },
                TicketNode {
                    id: "T-003".to_string(),
                    title: "chain2-root".to_string(),
                    phase: Phase::Design,
                    status: TicketStatus::InProgress,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-004".to_string(),
                    title: "chain2-leaf".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Blocked,
                    depends_on: vec!["T-003".to_string()],
                },
            ],
            ..PluginState::default()
        };
        let mut output = Vec::new();
        render_dag(&state, &mut output);
        let full = output.join("\n");

        assert!(full.contains("T-001"), "T-001 missing");
        assert!(full.contains("T-002"), "T-002 missing");
        assert!(full.contains("T-003"), "T-003 missing");
        assert!(full.contains("T-004"), "T-004 missing");

        // Both root nodes should appear on the same line (same layer)
        let root_line = output
            .iter()
            .find(|line| line.contains("T-001") && line.contains("T-003"));
        assert!(
            root_line.is_some(),
            "Independent roots should be on the same line, got:\n{}",
            full
        );
    }

    #[test]
    fn test_render_dag_done_filtered_edges_still_work() {
        // Chain: T-001(done) -> T-002(wip) -> T-003(blocked)
        // T-001 is filtered out, but T-002 -> T-003 edge should still render
        let state = PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-001".to_string(),
                    title: "done-root".to_string(),
                    phase: Phase::Done,
                    status: TicketStatus::Done,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-002".to_string(),
                    title: "active-mid".to_string(),
                    phase: Phase::Design,
                    status: TicketStatus::InProgress,
                    depends_on: vec!["T-001".to_string()],
                },
                TicketNode {
                    id: "T-003".to_string(),
                    title: "blocked-leaf".to_string(),
                    phase: Phase::Ready,
                    status: TicketStatus::Blocked,
                    depends_on: vec!["T-002".to_string()],
                },
            ],
            ..PluginState::default()
        };
        let mut output = Vec::new();
        render_dag(&state, &mut output);
        let full = output.join("\n");

        // T-001 should be filtered
        assert!(
            full.contains("1 done ticket"),
            "Should mention filtered done ticket"
        );

        // T-002 and T-003 should appear with an edge between them
        assert!(full.contains("T-002"), "T-002 missing");
        assert!(full.contains("T-003"), "T-003 missing");

        // Should have an edge connector between T-002 and T-003
        // ascii-dag may use → for inline chains or │/↓ for vertical layout
        let has_edge = full.contains('│') || full.contains('↓') || full.contains('→');
        assert!(
            has_edge,
            "Should have edge connector between T-002 and T-003, got:\n{}",
            full
        );
    }

    #[test]
    fn test_status_line() {
        let state = sample_state();
        let status = render_status_line(&state);

        assert!(status.contains("Active: 1"));
        assert!(status.contains("Done: 1/3"));
        assert!(status.contains("[Operations]"), "View label missing");
        assert!(status.contains("[p] view"), "View hint missing");
        assert!(status.contains("[space]"), "Pause hint missing");
    }

    #[test]
    fn test_full_dashboard_operations_view() {
        let mut state = sample_state();
        state.slots = vec![
            SlotInfo {
                ticket_id: Some("T-002".to_string()),
                slot_number: 1,
                transitioning: false,
            },
            SlotInfo {
                ticket_id: None,
                slot_number: 2,
                transitioning: false,
            },
        ];
        let lines = render_dashboard_lines(&state, 80, 40);
        let full = lines.join("\n");

        assert!(!lines.is_empty());
        assert!(full.contains("Dashboard"), "Dashboard header missing");
        assert!(full.contains("Threads"), "Threads section missing");
        assert!(full.contains("Activity"), "Activity section missing");
        // DAG should NOT be in Operations view
        assert!(
            !full.contains("≡≡ DAG ≡≡"),
            "DAG should not appear in Operations view"
        );
    }

    #[test]
    fn test_full_dashboard_dag_view() {
        let mut state = sample_state();
        state.active_view = ViewPreset::Dag;
        let lines = render_dashboard_lines(&state, 80, 40);
        let full = lines.join("\n");

        assert!(full.contains("Dashboard"), "Dashboard header missing");
        assert!(full.contains("DAG"), "DAG section missing");
        // Threads should NOT be in DAG view
        assert!(
            !full.contains("=== Threads ==="),
            "Threads should not appear in DAG view"
        );
    }

    #[test]
    fn test_full_dashboard_activity_view() {
        let mut state = sample_state();
        state.active_view = ViewPreset::Activity;
        let lines = render_dashboard_lines(&state, 80, 40);
        let full = lines.join("\n");

        assert!(full.contains("Dashboard"), "Dashboard header missing");
        assert!(full.contains("Recent Activity"), "Activity section missing");
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
                slot_number: 1,
                awaiting: false,
                route: None,
            }],
            parked_threads: vec![ParkedThread {
                ticket_id: "T-003".to_string(),
                phase: Phase::Research,
                artifact_path: "docs/active/work/T-003/research.md".to_string(),
                parked_at: Duration::from_secs(80),
                slot_number: 2,
            }],
            waiting_items: Vec::new(),
            note_items: Vec::new(),
            activity_log: vec![
                ActivityEntry {
                    timestamp: Duration::from_secs(50),
                    count: 1,
                    activity: ActivityType::PhaseCompleted {
                        ticket_id: "T-001".to_string(),
                        phase: Phase::Implement,
                    },
                },
                ActivityEntry {
                    timestamp: Duration::from_secs(100),
                    count: 1,
                    activity: ActivityType::ThreadStarted {
                        ticket_id: "T-002".to_string(),
                        phase: Phase::Design,
                    },
                },
                ActivityEntry {
                    timestamp: Duration::from_secs(120),
                    count: 1,
                    activity: ActivityType::Error {
                        ticket_id: "T-003".to_string(),
                        message: "test error".to_string(),
                    },
                },
            ],
            alerts: Vec::new(),
            slots: Vec::new(),
            seat_assignment_statuses: HashMap::new(),
            current_time: Duration::from_secs(200),
            modal: ModalState::default(),
            paused: false,
            active_view: ViewPreset::default(),
        };

        // Test DAG view: done tickets filtered, active tickets shown
        let mut dag_state = state.clone();
        dag_state.active_view = ViewPreset::Dag;
        let dag_lines = render_dashboard_lines(&dag_state, 80, 50);
        let dag_output = dag_lines.join("\n");

        // T-001 is Done — filtered out, but mentioned in hidden count
        assert!(
            dag_output.contains("1 done ticket"),
            "Hidden done count missing from DAG view"
        );
        // Active tickets should appear
        assert!(dag_output.contains("T-002"), "T-002 missing from DAG view");
        assert!(dag_output.contains("T-003"), "T-003 missing from DAG view");
        assert!(dag_output.contains("T-004"), "T-004 missing from DAG view");
        assert!(dag_output.contains("Dashboard"), "Dashboard header missing");

        // Test Operations view: filtered activity should show error
        let ops_lines = render_dashboard_lines(&state, 80, 50);
        let ops_output = ops_lines.join("\n");

        assert!(ops_output.contains("test error"), "Error activity missing");
        assert!(ops_output.contains("Active: 1"), "Active count wrong");
        assert!(ops_output.contains("Done: 1/4"), "Done count wrong");
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
                slot_number: 1,
            }],
            current_time: Duration::from_secs(200),
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_attention_banner(&state, 80, &mut output);

        let full = output.join("\n");
        assert!(full.contains("ATTENTION NEEDED"), "Banner header missing");
        assert!(full.contains("T-005"), "Ticket ID missing from banner");
        assert!(
            full.contains("review-ticket"),
            "Ticket title missing from banner"
        );
        assert!(
            full.contains("design.md"),
            "Artifact path missing from banner"
        );
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

        assert!(
            output.is_empty(),
            "Banner should not render when no review tickets"
        );
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
        assert!(
            full.contains("ATTENTION NEEDED"),
            "Banner should still render"
        );
        assert!(full.contains("T-006"), "Ticket ID should appear");
        assert!(
            full.contains("—"),
            "Dash placeholder should appear for missing data"
        );
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
                slot_number: 1,
            }],
            current_time: Duration::from_secs(100),
            ..PluginState::default()
        };

        let lines = render_dashboard_lines(&state, 80, 50);
        let full = lines.join("\n");

        // Banner should appear in Operations view
        assert!(
            full.contains("ATTENTION NEEDED"),
            "Banner missing from full dashboard"
        );

        // Banner should appear before the Threads section
        let banner_pos = full.find("ATTENTION NEEDED").unwrap();
        let threads_pos = full.find("Threads").unwrap();
        assert!(
            banner_pos < threads_pos,
            "Banner should appear before Threads section"
        );
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
        assert!(
            status.contains("Alerts: 1"),
            "Alert count missing from status line"
        );
    }

    #[test]
    fn test_status_line_no_alerts() {
        let state = PluginState::default();
        let status = render_status_line(&state);
        assert!(
            !status.contains("Alerts"),
            "Alerts should not appear when count is 0"
        );
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
                slot_number: 1,
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
    fn test_render_threads_all_idle() {
        let state = PluginState {
            slots: vec![
                SlotInfo {
                    ticket_id: None,
                    slot_number: 1,
                    transitioning: false,
                },
                SlotInfo {
                    ticket_id: None,
                    slot_number: 2,
                    transitioning: false,
                },
            ],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_threads(&state, &mut output);
        let full = output.join("\n");

        assert!(full.contains("Idle"), "Idle status missing");
        assert!(full.contains("[1]"), "Slot 1 missing");
        assert!(full.contains("[2]"), "Slot 2 missing");
    }

    #[test]
    fn test_render_threads_all_running() {
        let state = PluginState {
            slots: vec![
                SlotInfo {
                    ticket_id: Some("T-003-01".to_string()),
                    slot_number: 1,
                    transitioning: false,
                },
                SlotInfo {
                    ticket_id: Some("T-003-02".to_string()),
                    slot_number: 2,
                    transitioning: false,
                },
            ],
            active_threads: vec![
                ActiveThread {
                    ticket_id: "T-003-01".to_string(),
                    phase: Phase::Implement,
                    started_at: Duration::from_secs(100),
                    slot_number: 1,
                    awaiting: false,
                    route: None,
                },
                ActiveThread {
                    ticket_id: "T-003-02".to_string(),
                    phase: Phase::Research,
                    started_at: Duration::from_secs(100),
                    slot_number: 2,
                    awaiting: false,
                    route: None,
                },
            ],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_threads(&state, &mut output);
        let full = output.join("\n");

        assert!(full.contains("T-003-01"), "First ticket missing");
        assert!(full.contains("T-003-02"), "Second ticket missing");
        assert!(full.contains("Running"), "Running status missing");
        assert!(full.contains("IMP"), "Phase shortname missing");
        assert!(full.contains("RES"), "Phase shortname missing");
    }

    #[test]
    fn test_render_threads_mixed() {
        let state = PluginState {
            slots: vec![
                SlotInfo {
                    ticket_id: Some("T-001".to_string()),
                    slot_number: 1,
                    transitioning: false,
                },
                SlotInfo {
                    ticket_id: None,
                    slot_number: 2,
                    transitioning: false,
                },
                SlotInfo {
                    ticket_id: None,
                    slot_number: 3,
                    transitioning: true,
                },
            ],
            active_threads: vec![ActiveThread {
                ticket_id: "T-001".to_string(),
                phase: Phase::Design,
                started_at: Duration::from_secs(50),
                slot_number: 1,
                awaiting: false,
                route: None,
            }],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_threads(&state, &mut output);
        let full = output.join("\n");

        assert!(full.contains("T-001"), "Active ticket missing");
        assert!(full.contains("Running"), "Running status missing");
        assert!(full.contains("Idle"), "Idle status missing");
        assert!(full.contains("Winding Down"), "Winding Down status missing");
        assert!(full.contains("DES"), "Phase shortname missing");
    }

    #[test]
    fn test_render_threads_no_slots() {
        let state = PluginState::default();

        let mut output = Vec::new();
        render_threads(&state, &mut output);
        let full = output.join("\n");

        assert!(full.contains("no slots"), "Empty message missing");
    }

    #[test]
    fn test_status_line_with_slots() {
        let state = PluginState {
            slots: vec![
                SlotInfo {
                    ticket_id: Some("T-001".to_string()),
                    slot_number: 1,
                    transitioning: false,
                },
                SlotInfo {
                    ticket_id: None,
                    slot_number: 2,
                    transitioning: false,
                },
            ],
            ..PluginState::default()
        };

        let status = render_status_line(&state);
        assert!(
            status.contains("Slots: 1/2"),
            "Slot count missing from status line"
        );
        assert!(status.contains("Active: 0"), "Active count missing");
    }

    #[test]
    fn test_threads_in_full_dashboard() {
        let state = PluginState {
            tickets: vec![TicketNode {
                id: "T-001".to_string(),
                title: "test".to_string(),
                phase: Phase::Implement,
                status: TicketStatus::InProgress,
                depends_on: vec![],
            }],
            slots: vec![
                SlotInfo {
                    ticket_id: Some("T-001".to_string()),
                    slot_number: 1,
                    transitioning: false,
                },
                SlotInfo {
                    ticket_id: None,
                    slot_number: 2,
                    transitioning: false,
                },
            ],
            active_threads: vec![ActiveThread {
                ticket_id: "T-001".to_string(),
                phase: Phase::Implement,
                started_at: Duration::from_secs(50),
                slot_number: 1,
                awaiting: false,
                route: None,
            }],
            ..PluginState::default()
        };

        // Default view is Operations
        let lines = render_dashboard_lines(&state, 80, 50);
        let full = lines.join("\n");

        // Threads section should appear in Operations view
        assert!(
            full.contains("Threads"),
            "Threads section missing from Operations view"
        );
        assert!(full.contains("T-001"), "Active ticket missing");
        assert!(full.contains("Running"), "Running status missing");
        assert!(full.contains("Idle"), "Idle slot missing");
    }

    #[test]
    fn test_status_line_paused() {
        let state = PluginState {
            paused: true,
            ..sample_state()
        };
        let status = render_status_line(&state);
        assert!(status.contains("PAUSED"), "should show PAUSED indicator");
        assert!(status.contains("[space] resume"), "should show resume hint");
    }

    #[test]
    fn test_status_line_not_paused() {
        let state = sample_state();
        let status = render_status_line(&state);
        assert!(
            !status.contains("PAUSED"),
            "should not show PAUSED when unpaused"
        );
        assert!(status.contains("[space] pause"), "should show pause hint");
    }

    #[test]
    fn test_status_line_has_reset_hint() {
        let state = sample_state();
        let status = render_status_line(&state);
        assert!(
            status.contains("[r] reset"),
            "Status line should show [r] reset hint"
        );
    }

    #[test]
    fn test_modal_title_reset() {
        let modal = ModalState {
            open: true,
            ticket_ids: vec!["T-001".to_string()],
            cursor: 0,
            kind: ModalKind::ResetTicket,
            new_ticket_ids: Vec::new(),
            operator_outcome: None,
            reason_step: None,
        };
        let lines = render_modal(&modal, 50, 20);
        let full = lines.join("\n");
        assert!(
            full.contains("Reset Ticket to Ready"),
            "Reset modal should have correct title"
        );
    }

    #[test]
    fn test_modal_title_mark_done() {
        let modal = ModalState {
            open: true,
            ticket_ids: vec!["T-001".to_string()],
            cursor: 0,
            kind: ModalKind::MarkDone,
            new_ticket_ids: Vec::new(),
            operator_outcome: None,
            reason_step: None,
        };
        let lines = render_modal(&modal, 50, 20);
        let full = lines.join("\n");
        assert!(
            full.contains("Mark Ticket Done"),
            "Mark-done modal should have correct title"
        );
    }

    #[test]
    fn operator_modal_outcomes_render_ticket_correlation_and_named_reason() {
        assert_eq!(wrap_modal_text("éé", 1), vec!["é", "é"]);
        let cases = [
            OperatorModalOutcome::Pending {
                ticket_id: "T-PENDING".to_string(),
                correlation_id: "corr-pending".to_string(),
            },
            OperatorModalOutcome::Accepted {
                ticket_id: "T-ACCEPTED".to_string(),
                correlation_id: "corr-accepted".to_string(),
            },
            OperatorModalOutcome::Rejected {
                ticket_id: "T-REJECTED".to_string(),
                kind: CompletionRejectionKind::AlreadyPending,
                correlation_id: "corr-rejected".to_string(),
                detail: "another completion request owns this ticket".to_string(),
            },
        ];

        for outcome in cases {
            let modal = ModalState {
                open: true,
                ticket_ids: vec!["unused-after-submit".to_string()],
                cursor: 0,
                kind: ModalKind::MarkDone,
                new_ticket_ids: Vec::new(),
                operator_outcome: Some(outcome.clone()),
                reason_step: None,
            };
            let rendered = render_modal(&modal, 50, 24).join("\n");

            match outcome {
                OperatorModalOutcome::Pending {
                    ticket_id,
                    correlation_id,
                } => {
                    assert!(rendered.contains("Completion pending"));
                    assert!(rendered.contains(&ticket_id));
                    assert!(rendered.contains(&correlation_id));
                    assert!(rendered.contains("Waiting for completion result"));
                }
                OperatorModalOutcome::Accepted {
                    ticket_id,
                    correlation_id,
                } => {
                    assert!(rendered.contains("Completion accepted"));
                    assert!(rendered.contains(&ticket_id));
                    assert!(rendered.contains(&correlation_id));
                    assert!(rendered.contains("Enter/Esc=close"));
                }
                OperatorModalOutcome::Rejected {
                    ticket_id,
                    kind,
                    correlation_id,
                    detail,
                } => {
                    assert!(rendered.contains("Not finished yet"));
                    assert!(
                        kind.plain_line()
                            .split_whitespace()
                            .all(|word| rendered.contains(word)),
                        "wrapped plain line lost content for {kind}: {rendered}"
                    );
                    assert!(
                        rendered.contains(&kind.to_string()),
                        "machine token missing from Ref line for {kind}: {rendered}"
                    );
                    assert!(rendered.contains(&ticket_id));
                    assert!(rendered.contains(&correlation_id));
                    assert!(
                        detail
                            .split_whitespace()
                            .all(|word| rendered.contains(word)),
                        "wrapped rejection detail lost content: {rendered}"
                    );
                    if kind == CompletionRejectionKind::DispositionBlocked {
                        assert!(rendered.contains("paste this note"));
                    }
                    assert!(rendered.contains("Enter/Esc=close"));
                }
            }
        }
    }

    // ==================================================================
    // The reason step — choices, not essays (T-053-01-02)
    // ==================================================================

    /// The field block from E-053's "Done looks like".
    const XCODE_ASK: &str = "Sign into Xcode with an Apple ID, then re-run the signed build.";
    const XCODE_REASON: &str = "codesign refused: no signing identity found";
    const PARSE_FAILURE: &str =
        "review disposition is malformed JSON: expected value at line 1 column 1";

    /// Drop SGR escapes so a rendered row can be measured as the terminal
    /// displays it.
    fn strip_ansi(line: &str) -> String {
        let mut out = String::new();
        let mut chars = line.chars();
        while let Some(character) = chars.next() {
            if character == '\u{1b}' {
                for escaped in chars.by_ref() {
                    if escaped == 'm' {
                        break;
                    }
                }
                continue;
            }
            out.push(character);
        }
        out
    }

    fn reason_step_modal(ask: OverriddenAsk, cursor: usize) -> ModalState {
        ModalState {
            open: true,
            ticket_ids: vec!["T-015-02-02".to_string()],
            cursor: 0,
            kind: ModalKind::MarkDone,
            new_ticket_ids: Vec::new(),
            operator_outcome: None,
            reason_step: Some(ReasonStepState {
                ticket_id: "T-015-02-02".to_string(),
                choices: ask.applicable_reasons().to_vec(),
                cursor,
                ask,
            }),
        }
    }

    fn blocked_ask() -> OverriddenAsk {
        OverriddenAsk::Block {
            ask: XCODE_ASK.to_string(),
            reason: XCODE_REASON.to_string(),
        }
    }

    /// Criterion 2, first half: the block's ask, above the choices, verbatim.
    #[test]
    fn reason_step_shows_the_blocks_ask_verbatim() {
        let rendered = render_modal(&reason_step_modal(blocked_ask(), 0), 50, 24).join("\n");

        assert!(rendered.contains("Sign T-015-02-02"), "{rendered}");
        // The ask wraps across rows, so it is checked word-for-word in order
        // rather than as one substring.
        let mut haystack = rendered.as_str();
        for word in XCODE_ASK.split_whitespace() {
            let found = haystack
                .find(word)
                .unwrap_or_else(|| panic!("the ask lost {word:?} on screen: {rendered}"));
            haystack = &haystack[found + word.len()..];
        }
    }

    /// Criterion 2, second half: a fail-closed ticket says so plainly — never a
    /// raw parse error.
    #[test]
    fn reason_step_never_prints_the_parse_error() {
        let ask = OverriddenAsk::UnreadableReview {
            detail: PARSE_FAILURE.to_string(),
        };
        let rendered = render_modal(&reason_step_modal(ask, 0), 50, 24).join("\n");

        for machine_word in ["malformed", "JSON", "column", "expected value"] {
            assert!(
                !rendered.contains(machine_word),
                "the parse error leaked {machine_word:?} onto the screen: {rendered}"
            );
        }
        for plain_word in ["No", "review", "Lisa", "can", "read"] {
            assert!(rendered.contains(plain_word), "{rendered}");
        }
    }

    #[test]
    fn reason_step_says_plainly_when_no_review_is_on_file() {
        let rendered =
            render_modal(&reason_step_modal(OverriddenAsk::NoReviewOnFile, 0), 50, 24).join("\n");

        for plain_word in ["No", "review", "was", "left", "ticket"] {
            assert!(rendered.contains(plain_word), "{rendered}");
        }
    }

    /// The old rejection modal's dump does not come back one line lower.
    #[test]
    fn reason_step_never_prints_the_blocks_technical_reason() {
        let rendered = render_modal(&reason_step_modal(blocked_ask(), 0), 50, 24).join("\n");

        for machine_word in ["codesign", "signing identity"] {
            assert!(
                !rendered.contains(machine_word),
                "the technical reason leaked {machine_word:?}: {rendered}"
            );
        }
    }

    #[test]
    fn reason_step_lists_every_applicable_choice_and_marks_the_cursor() {
        let ask = blocked_ask();
        let expected = ask.applicable_reasons().len();
        assert_eq!(expected, 3);

        for cursor in 0..expected {
            let lines = render_modal(&reason_step_modal(blocked_ask(), cursor), 50, 24);
            let marked: Vec<usize> = lines
                .iter()
                .enumerate()
                .filter(|(_, line)| line.contains('▸'))
                .map(|(index, _)| index)
                .collect();
            assert_eq!(marked.len(), 1, "exactly one choice is selected: {lines:?}");

            // Every choice's opening words are on screen, whether selected or not.
            let rendered = lines.join("\n");
            for reason in ask.applicable_reasons() {
                let opening: String = reason
                    .summary()
                    .split_whitespace()
                    .take(4)
                    .collect::<Vec<_>>()
                    .join(" ");
                assert!(
                    rendered.contains(&opening),
                    "choice {} missing at cursor {cursor}: {rendered}",
                    reason.id()
                );
            }
        }
    }

    /// The box stays square on copy that carries em dashes — a byte-length
    /// measurement would over-pad every choice row.
    #[test]
    fn reason_step_rows_fit_the_box() {
        for ask in [
            blocked_ask(),
            OverriddenAsk::NoReviewOnFile,
            OverriddenAsk::UnreadableReview {
                detail: PARSE_FAILURE.to_string(),
            },
        ] {
            let lines = render_modal(&reason_step_modal(ask.clone(), 0), 50, 24);
            let widths: Vec<usize> = lines
                .iter()
                .filter(|line| !line.is_empty())
                .map(|line| strip_ansi(line).chars().count())
                .collect();
            assert!(
                widths.windows(2).all(|pair| pair[0] == pair[1]),
                "ragged box for {ask:?}: {widths:?}"
            );
        }
    }

    /// Criterion 4 where it renders: the only keys the step advertises are the
    /// two that move it. No text field, no "type a reason".
    #[test]
    fn reason_step_footer_offers_only_sign_and_back() {
        let rendered = render_modal(&reason_step_modal(blocked_ask(), 0), 50, 24).join("\n");

        assert!(rendered.contains("Enter=sign"), "{rendered}");
        assert!(rendered.contains("Esc=back"), "{rendered}");
        for absent in ["type", "Type", "input", "edit", "Tab"] {
            assert!(
                !rendered.contains(absent),
                "the step advertises {absent:?}: {rendered}"
            );
        }
    }

    /// A long ask and a narrow box must not spill outside the border.
    #[test]
    fn reason_step_survives_a_narrow_box() {
        let lines = render_modal(&reason_step_modal(blocked_ask(), 2), 24, 24);
        let widths: Vec<usize> = lines
            .iter()
            .filter(|line| !line.is_empty())
            .map(|line| strip_ansi(line).chars().count())
            .collect();
        assert!(
            widths.windows(2).all(|pair| pair[0] == pair[1]),
            "ragged narrow box: {widths:?}"
        );
    }

    #[test]
    fn test_compact_dashboard_fewer_lines() {
        let state = sample_state();
        let lines = render_dashboard_lines(&state, 80, 40);
        // Count blank lines
        let blank_lines = lines.iter().filter(|l| l.is_empty()).count();
        // Compacted dashboard should have fewer blank lines than before (~12).
        // Sub-sections still add a few internally, but we removed the padding between them.
        assert!(
            blank_lines <= 8,
            "Dashboard should have <= 8 blank lines, got {}",
            blank_lines
        );
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
        assert!(
            clamped <= total_lines,
            "Clamped scroll should not exceed total lines"
        );
        assert_eq!(
            clamped, max_scroll,
            "Clamped scroll should equal max_scroll"
        );
    }
}
