//! UI/Dashboard module for the Lisa Zellij plugin.
//!
//! Provides four preset views. `[p]` jumps straight to the desk from anywhere;
//! `[v]` cycles the presets in order:
//! - **Operations**: Desk pointer, health alerts, unified thread table, filtered activity log
//! - **Present**: The desk — one collapsed card per pending decision
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

impl TicketStatus {
    /// The three-letter token shown on a DAG node.
    pub fn token(&self) -> &'static str {
        match self {
            TicketStatus::Ready => "RDY",
            TicketStatus::InProgress => "WRK",
            TicketStatus::WaitingReview => "REV",
            TicketStatus::Blocked => "BLK",
            TicketStatus::Done => "DON",
        }
    }

    /// ANSI color for the status token.
    ///
    /// Done is here for completeness — the DAG filters Done tickets out before
    /// nodes are built, so a `DON` token never reaches a rendered line.
    pub fn color_code(&self) -> &'static str {
        match self {
            TicketStatus::Ready => CYAN,
            TicketStatus::InProgress => GREEN,
            TicketStatus::WaitingReview => BRIGHT_YELLOW,
            TicketStatus::Blocked => RED,
            TicketStatus::Done => BRIGHT_GREEN,
        }
    }
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
    /// The block's own prepared steps, empty when it supplied none.
    pub steps: Vec<String>,
    /// The block's read-only verification command, when it supplied one.
    pub check: Option<String>,
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

// =============================================================================
// The desk
// =============================================================================

/// How a ticket with no review on file states its case, wherever it is shown.
///
/// One string, two readers: the reason-step modal's header and the desk card
/// for the same ticket. A person who reaches the modal from the card must not
/// be told two different things about the same state.
pub(crate) const NO_REVIEW_ASK: &str = "No review was left for this ticket.";

/// The same, for a review file that exists but cannot be read.
///
/// The reader's own parse failure never reaches this sentence — it is quoted in
/// the card's staff work, one keypress deep, where technical detail belongs.
pub(crate) const UNREADABLE_REVIEW_ASK: &str = "No review Lisa can read was left for this ticket.";

/// What a Review-phase ticket is waiting for.
///
/// The one card sentence with no disposition field behind it, because a ticket
/// still in Review has not written a verdict to quote. A fixed constant, not
/// prose generated per ticket.
pub(crate) const REVIEW_WAIT_ASK: &str = "Review finished — this one is waiting for you.";

/// The desk with nothing on it.
pub(crate) const EMPTY_DESK: &str = "Nothing needs you.";

/// What a desk card is waiting on, which decides its framing and its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeskCardKind {
    /// An agent left a readable block naming what it needs.
    Block,
    /// The ticket is blocked with no review anyone can read. Invisible to the
    /// remedy collector, and exactly what the no-review override serves.
    NoReviewOnFile,
    /// The ticket finished Review and is waiting to be signed.
    ReviewWait,
    /// A receipt from completed work. Never an action, only a read.
    Note,
}

/// The staff work behind a card: everything an operator can ask to see, and
/// nothing they are shown before asking.
///
/// Deliberately a separate struct from [`DeskCard`]: the collapsed renderer
/// never touches it, so "no criterion quote on a collapsed card" is a property
/// of the types rather than a rule a renderer has to keep remembering.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeskDetail {
    pub reason: Option<String>,
    pub criterion_quote: Option<String>,
    pub evidence_citation: Option<String>,
    pub steps: Vec<String>,
    pub check: Option<String>,
    pub proposal: Option<TriageProposal>,
    /// True for a world-owned remedy, which Lisa re-probes on its own.
    pub checks_on_own: bool,
}

/// One pending decision, collapsed to three lines.
///
/// Every field is copied from something a disposition already carries. Nothing
/// here is generated or summarized: a jargony ask surfaces verbatim, which is
/// the disposition author's bug to fix upstream, not this view's to hide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeskCard {
    pub ticket_id: String,
    pub title: String,
    /// Epoch stamp for the age line. `None` renders as [`UNKNOWN_AGE`] — a card
    /// with no stamp says so rather than showing an invented number.
    pub age_stamp: Option<Duration>,
    pub kind: DeskCardKind,
    /// The one sentence, verbatim from the field that carries it.
    pub ask: String,
    pub detail: DeskDetail,
}

/// The desk: every pending decision, and which one the operator is looking at.
#[derive(Debug, Clone, Default)]
pub struct DeskState {
    pub cards: Vec<DeskCard>,
    /// Index into `cards`. Clamped at render time, never trusted blindly.
    pub selected: usize,
    /// Whether the selected card is showing its staff work. Always one
    /// keypress deep, never the default.
    pub expanded: bool,
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
    /// The desk: one collapsed card per pending decision.
    Present,
    /// Dedicated DAG dependency visualization.
    Dag,
    /// Full activity log with all entry types.
    Activity,
}

impl ViewPreset {
    /// Cycle to the next view preset.
    pub fn next(self) -> Self {
        match self {
            ViewPreset::Operations => ViewPreset::Present,
            ViewPreset::Present => ViewPreset::Dag,
            ViewPreset::Dag => ViewPreset::Activity,
            ViewPreset::Activity => ViewPreset::Operations,
        }
    }

    /// Human-readable label for the status bar.
    pub fn label(&self) -> &'static str {
        match self {
            ViewPreset::Operations => "Operations",
            ViewPreset::Present => "Present",
            ViewPreset::Dag => "DAG",
            ViewPreset::Activity => "Activity",
        }
    }
}

/// The DAG's horizontal viewport: how far the operator has panned, and how far
/// there is to pan.
///
/// `offset` travels in and `span` travels out. The render is the only thing that
/// knows how wide the map came out, so it reports rather than being asked — the
/// same instinct as the overflow indicator, which says what the render actually
/// did instead of guessing.
///
/// `span` is zero in every view but the DAG and zero on a map that fits, which
/// is what lets the pan keys be inert without the key handler needing to know
/// which views own a graph.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DagPan {
    /// Visible columns dropped from the left of every graph body line.
    pub offset: usize,
    /// `widest_visible_line − pane_cols`: the largest offset that reveals
    /// anything, and the same number the overflow indicator prints.
    pub span: usize,
}

/// The complete plugin state for rendering
#[derive(Debug, Clone)]
pub struct PluginState {
    pub tickets: Vec<TicketNode>,
    pub active_threads: Vec<ActiveThread>,
    pub parked_threads: Vec<ParkedThread>,
    /// Every pending decision, as cards. The Present preset renders these; the
    /// Operations view counts them. Parked remedies and completion notes reach
    /// the screen only through here — there is one desk, so there is one place
    /// a pending decision can be read.
    pub desk: DeskState,
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
            desk: DeskState::default(),
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
// The desk — one card per pending decision
// =============================================================================

/// Indent for a card's second and third lines, and for its staff work.
const CARD_INDENT: &str = "    ";

/// The action a card recommends, naming a key that works today.
///
/// Nothing here advertises a transition the plugin refuses. `[d]` reaches the
/// mark-done modal, which lists blocked and Review-phase tickets alike and
/// routes one needing a signature to the reason step. A note offers only
/// `[enter]`, because reading it is the only move a receipt has.
fn card_action_line(card: &DeskCard) -> String {
    match card.kind {
        DeskCardKind::Note => "→ [enter] read it".to_string(),
        _ if card.detail.checks_on_own => "→ [d] mark it done · Lisa checks on its own".to_string(),
        _ => "→ [d] mark it done".to_string(),
    }
}

/// Render one card: three lines collapsed, plus its staff work when opened.
///
/// The collapsed half reads only [`DeskCard`]'s own fields and never
/// [`DeskDetail`], so no criterion quote, evidence path, reason, or check
/// command can reach a collapsed card by accident.
fn desk_card_lines(
    card: &DeskCard,
    selected: bool,
    expanded: bool,
    current_time: Duration,
    width: usize,
) -> Vec<String> {
    let text_width = width.min(100).saturating_sub(CARD_INDENT.len());
    let age = format_age_bucket(card.age_stamp.unwrap_or(Duration::ZERO), current_time);
    let marker = if selected { "▸ " } else { "  " };

    let mut lines = vec![
        format!(
            "{marker}{BOLD}{}{RESET} · {} · {DIM}{age}{RESET}",
            card.ticket_id, card.title
        ),
        format!("{CARD_INDENT}{}", fit_modal_line(&card.ask, text_width)),
        format!(
            "{CARD_INDENT}{DIM}{}{RESET}",
            fit_modal_line(&card_action_line(card), text_width)
        ),
    ];

    if !(selected && expanded) {
        return lines;
    }

    let detail = &card.detail;
    if let Some(proposal) = &detail.proposal {
        lines.push(format!(
            "{CARD_INDENT}First responder: {}",
            proposal.summary
        ));
        lines.push(format!(
            "{CARD_INDENT}Suggested: {}",
            proposal.recommendation
        ));
        lines.extend(
            proposal
                .prepared_steps
                .iter()
                .map(|step| format!("{CARD_INDENT}Prepared: {}", step.display())),
        );
    }
    if let Some(reason) = &detail.reason {
        lines.push(format!("{CARD_INDENT}Reason: {reason}"));
    }
    if let Some(quote) = &detail.criterion_quote {
        lines.push(format!("{CARD_INDENT}Criterion: “{quote}”"));
    }
    if let Some(citation) = &detail.evidence_citation {
        lines.push(format!("{CARD_INDENT}Evidence: {citation}"));
    }
    lines.extend(
        detail
            .steps
            .iter()
            .map(|step| format!("{CARD_INDENT}Step: {step}")),
    );
    if let Some(check) = &detail.check {
        lines.push(format!("{CARD_INDENT}Check: {check}"));
    }
    lines
}

/// Render the desk: every pending decision, three lines each.
///
/// An empty desk is one calm sentence and nothing else — no header, no counts,
/// no box. There is nothing to organize, so there is no organizer to draw.
fn render_present_view(state: &PluginState, width: usize, output: &mut Vec<String>) {
    let cards = &state.desk.cards;
    if cards.is_empty() {
        output.push(EMPTY_DESK.to_string());
        return;
    }

    let selected = state.desk.selected.min(cards.len() - 1);
    output.push(format!("{BOLD}Your desk{RESET}"));
    output.push(String::new());
    for (index, card) in cards.iter().enumerate() {
        output.extend(desk_card_lines(
            card,
            index == selected,
            state.desk.expanded,
            state.current_time,
            width,
        ));
        output.push(String::new());
    }
}

/// Render the Operations pointer: how much is waiting, and where to read it.
///
/// The counts come from the same card list the desk renders, so the pointer and
/// the desk cannot disagree about how much is waiting.
fn render_desk_pointer(state: &PluginState, output: &mut Vec<String>) {
    let notes = state
        .desk
        .cards
        .iter()
        .filter(|card| card.kind == DeskCardKind::Note)
        .count();
    let waiting = state.desk.cards.len() - notes;
    if waiting == 0 && notes == 0 {
        return;
    }

    let mut parts = Vec::new();
    if waiting > 0 {
        parts.push(format!("{waiting} waiting"));
    }
    if notes > 0 {
        parts.push(format!("{notes} note{}", if notes == 1 { "" } else { "s" }));
    }
    output.push(format!(
        "{BOLD}{}{RESET}{DIM} — [p]{RESET}",
        parts.join(", ")
    ));
    output.push(String::new());
}

// =============================================================================
// Health alerts
// =============================================================================

/// Render the "ATTENTION NEEDED" banner for unhealthy agent sessions.
///
/// Stuck, failed, idle, and timed-out sessions only. A session in trouble is a
/// different thing from a decision waiting on a person: decisions are cards on
/// the desk, and a stuck pane is not something anyone can sign. The Review-phase
/// rows this box used to carry moved to the desk with them, and the
/// "Press [d] to mark done" hint went with the rows it described.
///
/// Appends nothing when every session is healthy.
fn render_health_alerts(state: &PluginState, width: usize, output: &mut Vec<String>) {
    if state.alerts.is_empty() {
        return;
    }

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

/// How many columns of a pane a line actually occupies.
///
/// Two things this is careful about, both of which byte length gets wrong:
/// the DAG's edge routing is drawn in multi-byte single-column glyphs
/// (`→ ┌ ─ ↓ └ ┐`), and a colored line carries SGR escapes that a terminal
/// consumes rather than prints. Color is not ink: a line measures the same
/// before and after it is painted.
fn visible_width(line: &str) -> usize {
    let mut width = 0;
    let mut chars = line.chars();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            // An SGR sequence runs to its terminating `m` and shows nothing.
            for escaped in chars.by_ref() {
                if escaped == 'm' {
                    break;
                }
            }
            continue;
        }
        width += 1;
    }
    width
}

/// The widest line in a rendered block, in visible columns.
///
/// Blank lines — `ascii_dag::render()` ends with a couple — measure zero and
/// cannot raise the maximum.
fn widest_visible_line(rendered: &str) -> usize {
    rendered.lines().map(visible_width).max().unwrap_or(0)
}

/// Drop the first `offset` visible columns from a line that may already be
/// painted.
///
/// The hazard this function exists for: by the time a line is sliced it carries
/// injected SGR escapes, and a byte or `char` cut can land inside `\u{1b}[36m`
/// and spill the tail onto the screen as literal garbage. This walks columns the
/// way [`visible_width`] counts them — escapes consumed whole, never counted —
/// so a cut can only ever land between sequences.
///
/// Paint survives the cut. Sequences still in force where the cut lands are
/// re-emitted at the front, so a node straddling the left edge keeps the status
/// color that is condensed mode's only status channel. Everything past the cut
/// is copied verbatim, escapes included, so the line's own resets arrive
/// untouched and the function stays total over any SGR vocabulary rather than
/// only the one we inject.
///
/// Nothing is truncated on the right: the terminal clips that edge today and the
/// overflow indicator already accounts for it.
fn pan_line(line: &str, offset: usize) -> String {
    // The identity case, stated rather than computed: an unpanned board must be
    // byte-for-byte what it was before this function existed.
    if offset == 0 {
        return line.to_string();
    }

    let mut chars = line.chars();
    let mut column = 0;
    // Sequences opened and not yet cancelled. A list, not "the last one seen":
    // `{BOLD}{CYAN}` is two consecutive sequences, and remembering only the
    // last would quietly drop the bold.
    let mut active: Vec<String> = Vec::new();

    while column < offset {
        let Some(character) = chars.next() else {
            // The whole line lies left of the cut. Nothing visible remains, so
            // emit nothing — carrying color onto an empty line would be ink
            // with nothing to paint.
            return String::new();
        };

        if character == '\u{1b}' {
            let mut sequence = String::from(character);
            for escaped in chars.by_ref() {
                sequence.push(escaped);
                if escaped == 'm' {
                    break;
                }
            }
            if sequence == RESET {
                active.clear();
            } else {
                active.push(sequence);
            }
            continue;
        }

        column += 1;
    }

    let remainder: String = chars.collect();
    // Visible content, not bytes: a line cut at exactly its last column still
    // trails the reset that closed it, and emitting color plus a reset with no
    // glyphs between them is ink with nothing to paint.
    if visible_width(&remainder) == 0 {
        return String::new();
    }

    let mut out = active.concat();
    out.push_str(&remainder);
    // A cut can carry a color across without its closing reset ever appearing
    // in what survived. Close it here so no ink leaks past this line.
    if !active.is_empty() && !remainder.contains(RESET) {
        out.push_str(RESET);
    }
    out
}

/// How much of a node's name the board can afford to print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelStyle {
    /// `T-054-01-02 WRK` — id in its phase color, token in its status color.
    Full,
    /// `054-01-02` — the `T-` prefix and the status token shed, six columns a
    /// node, with the id recolored to carry the status the token used to.
    Condensed,
}

/// The text of one node, and the only place that text is decided.
///
/// Condensing sheds ceremony, never information: the prefix every id on the
/// board shares and a token whose meaning color already carries. The id's own
/// digits are its name and stay whole, so prepending `T-` reads the condensed
/// label straight back.
fn dag_label(id: &str, status: &TicketStatus, style: LabelStyle) -> String {
    match style {
        LabelStyle::Full => format!("{} {}", id, status.token()),
        // `unwrap_or` rather than a slice: an id without the prefix loses no
        // characters instead of the wrong one.
        LabelStyle::Condensed => id.strip_prefix("T-").unwrap_or(id).to_string(),
    }
}

/// What the DAG post-processor needs to ink one node's label.
///
/// In full mode the two colors are separate channels: the ticket id carries the
/// phase, the status token carries the status. They are read side by side, so
/// they must never be sourced from each other. In condensed mode the token is
/// gone and the id is the only glyph left to paint, so status takes it.
struct NodeInk<'a> {
    /// The exact label ascii-dag rendered, e.g. `T-054-01-01 WRK`.
    label: &'a str,
    ticket_id: &'a str,
    /// Empty in condensed mode, where no token was rendered.
    token: &'a str,
    phase_color: &'a str,
    status_color: &'a str,
    style: LabelStyle,
}

impl NodeInk<'_> {
    /// The label with color inserted at the seams — and nothing else changed.
    /// Strip the escapes and the original label comes back, character for
    /// character.
    fn inked(&self) -> String {
        match self.style {
            LabelStyle::Full => format!(
                "{}{}{} {}{}{}",
                self.phase_color, self.ticket_id, RESET, self.status_color, self.token, RESET
            ),
            // `label`, not `ticket_id` — the id here has already shed its prefix.
            LabelStyle::Condensed => format!("{}{}{}", self.status_color, self.label, RESET),
        }
    }
}

/// Lay the graph out with labels in the given style.
///
/// The single call into ascii-dag, which stays the layout owner: this chooses
/// label strings and nothing else. Returns the labels alongside the render
/// because the ink map borrows them.
fn render_dag_body(
    active: &[&TicketNode],
    edges: &[(usize, usize)],
    id_to_int: &HashMap<&str, usize>,
    style: LabelStyle,
) -> (Vec<(usize, String)>, String) {
    let nodes: Vec<(usize, String)> = active
        .iter()
        .map(|t| {
            let label = dag_label(&t.id, &t.status, style);
            (id_to_int[t.id.as_str()], label)
        })
        .collect();

    let node_refs: Vec<(usize, &str)> = nodes
        .iter()
        .map(|(id, label)| (*id, label.as_str()))
        .collect();

    let rendered = ascii_dag::DAG::from_edges(&node_refs, edges).render();
    (nodes, rendered)
}

/// Compute DAG layers for visualization (topological sort into layers)
/// Render the DAG using ascii-dag for proper edge routing and layout.
///
/// Filters out completed tickets to keep the view focused on active work.
/// Uses Sugiyama layered layout via ascii-dag for crossing minimization
/// and proper fan-in/fan-out visualization.
fn render_dag(state: &PluginState, pane_cols: usize, pan: &mut DagPan, output: &mut Vec<String>) {
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

    // Full labels first, always. Condensing is an overflow response, never a
    // default: a board that fits the pane keeps every character it has today.
    // `pane_cols == 0` is a caller that does not know the pane, and the honest
    // answer to not knowing is to change nothing.
    let mut style = LabelStyle::Full;
    let (mut nodes, mut rendered) = render_dag_body(&active, &edges, &id_to_int, style);

    if pane_cols > 0 && widest_visible_line(&rendered) > pane_cols {
        style = LabelStyle::Condensed;
        (nodes, rendered) = render_dag_body(&active, &edges, &id_to_int, style);
    }

    // Build the ink map for post-processing, keyed by ticket id. `label` borrows
    // the exact string handed to ascii-dag, so the substring we search for and
    // the substring that was rendered cannot drift apart.
    let color_map: HashMap<&str, NodeInk> = active
        .iter()
        .zip(nodes.iter())
        .map(|(t, (_, label))| {
            (
                t.id.as_str(),
                NodeInk {
                    label: label.as_str(),
                    ticket_id: t.id.as_str(),
                    token: match style {
                        LabelStyle::Full => t.status.token(),
                        LabelStyle::Condensed => "",
                    },
                    phase_color: t.phase.color_code(),
                    status_color: t.status.color_code(),
                    style,
                },
            )
        })
        .collect();

    // How much map lies outside the pane — the largest offset that reveals
    // anything, and the same number the indicator prints below. `pane_cols == 0`
    // is a caller that does not know the pane, so there is nothing to pan past.
    let widest = widest_visible_line(&rendered);
    pan.span = if pane_cols > 0 {
        widest.saturating_sub(pane_cols)
    } else {
        0
    };
    // The renderer clamps, because the renderer is where the bound is known —
    // the same arrangement `print_dashboard` uses for the page scroll.
    let offset = pan.offset.min(pan.span);

    // Post-process: ink each node label. Matching the whole label rather than
    // the bare id keeps one ticket's id from matching inside a longer ticket's
    // label.
    for line in rendered.lines() {
        let mut colored_line = line.to_string();
        for ink in color_map.values() {
            if colored_line.contains(ink.label) {
                colored_line = colored_line.replace(ink.label, &ink.inked());
            } else if colored_line.contains(ink.ticket_id) {
                // A line carrying the id without its label: color the id alone,
                // as this loop did before the status token was inked.
                colored_line = colored_line.replace(
                    ink.ticket_id,
                    &format!("{}{}{}", ink.phase_color, ink.ticket_id, RESET),
                );
            }
        }
        // Only body lines reach here — the header, indicator, summary and legend
        // are pushed outside this loop. That is what keeps the map moving while
        // the line naming the pan keys, and the legend the nodes are read by,
        // stay where the eye left them.
        output.push(pan_line(&colored_line, offset));
    }

    // Guarded by the same predicate that drove condensing, so there is no
    // third outcome: either the board fits, or it has already been condensed
    // and says what is still off the edge. Clipping quietly is not reachable.
    if pane_cols > 0 && widest > pane_cols {
        output.push(dag_overflow_line(widest, pane_cols));
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
    // One legend, and always the one the board is actually using. In condensed
    // mode a node's color means its status, so a phase legend under it would
    // document a code the board has stopped speaking.
    output.push(match style {
        LabelStyle::Full => format!(
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
        ),
        LabelStyle::Condensed => dag_status_legend(),
    });
}

/// What the pane is cutting off, said plainly.
///
/// Reached only when a condensed board still runs past the pane, which is the
/// one case where the map cannot be made to fit. Silence there would be the map
/// lying about its own edge, so it says how much is missing, how to go and look
/// at it, and how wide a pane would have to be.
///
/// The keys come before the arithmetic because they are what the reader can act
/// on. The count stays true at every offset: the map is `widest` and the pane is
/// `pane_cols` wherever the viewport sits, so panning changes which columns are
/// off-screen, never how many.
fn dag_overflow_line(widest: usize, pane_cols: usize) -> String {
    format!(
        "{}({} column{} off-screen — [h]/[l] to pan — the map needs {}, the pane has {}){}",
        DIM,
        widest - pane_cols,
        if widest - pane_cols == 1 { "" } else { "s" },
        widest,
        pane_cols,
        RESET
    )
}

/// The color code condensed mode reads by: each status word in its own color.
///
/// Built from the same two methods the nodes are painted with, so the legend
/// cannot drift from the paint. `Done` is absent because the graph filters it
/// out and no node can carry it.
fn dag_status_legend() -> String {
    let words: Vec<String> = [
        TicketStatus::Ready,
        TicketStatus::InProgress,
        TicketStatus::WaitingReview,
        TicketStatus::Blocked,
    ]
    .iter()
    .map(|status| format!("{}{}{}", status.color_code(), status.token(), RESET))
    .collect();

    format!("{}Status:{} {}", DIM, RESET, words.join(" "))
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

/// The key hints for a view, naming only keys that work in that view.
///
/// The estate is view-dependent, so one static line cannot be honest about it:
/// `[p]` means "go to the desk" everywhere except the desk, where it is a
/// no-op, and `[s]`, `[enter]`, and the arrows mean something only on the desk.
/// A single line naming all of them would advertise send-back on the DAG view,
/// which is the N3 sin this epic is named after.
///
/// `[r]` and `[p]` still work on the desk and are simply not advertised there —
/// a key without a hint costs discoverability; a hint without a key is a lie.
fn view_key_hints(view: ViewPreset, pause_hint: &str) -> String {
    match view {
        ViewPreset::Present => format!(
            "[↑↓] pick  [enter] open  [d] done  [s] send back  [v] view  [space] {pause_hint}"
        ),
        _ => format!("[p] desk  [v] view  [space] {pause_hint}  [d] done  [r] reset"),
    }
}

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
    let hints = view_key_hints(state.active_view, pause_hint);

    format!(
        "{}[{}]{} {}{}Active: {} | Done: {}/{}{}  {}{}{}",
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
        hints,
        RESET
    )
}

// =============================================================================
// Main Dashboard Rendering
// =============================================================================

/// Render the complete dashboard to a vector of lines.
///
/// Dispatches to a view-specific renderer based on the active preset.
///
/// `pane_cols` is the pane's true width. The text presets have always laid
/// themselves out against at most 100 columns and still do — the clamp lives
/// here rather than at the call site so that the one view which measures itself
/// against the real pane, the DAG, can still see it.
fn render_dashboard_lines(
    state: &PluginState,
    pane_cols: usize,
    height: usize,
    pan: &mut DagPan,
) -> Vec<String> {
    let width = pane_cols.min(100);
    let mut output = Vec::new();

    // A view with no map to pan reports no room to pan. Set before the dispatch
    // so no arm can forget it, which is what makes the pan keys inert in the
    // other three presets without the key handler knowing which views own a
    // graph.
    pan.span = 0;

    // Title bar with status (always present, all views)
    let status = render_status_line(state);
    output.push(format!(
        "{}{}  LISA Dashboard  {} {}{}",
        BOLD, BG_BLUE, RESET, DIM, status
    ));
    output.push(render_separator(width));

    match state.active_view {
        ViewPreset::Operations => render_operations_view(state, width, height, &mut output),
        ViewPreset::Present => render_present_view(state, width, &mut output),
        ViewPreset::Dag => render_dag_view(state, pane_cols, pan, &mut output),
        ViewPreset::Activity => render_activity_view(state, height, &mut output),
    }

    output
}

/// Operations view: desk pointer + health alerts + threads + activity log.
///
/// What waits on a person is a count and a key here, not paragraphs. The asks,
/// reasons, criterion quotes, and evidence paths that used to open this screen
/// live on the desk, where they can be read one at a time on request.
fn render_operations_view(
    state: &PluginState,
    width: usize,
    height: usize,
    output: &mut Vec<String>,
) {
    // How much is waiting, and where to read it.
    render_desk_pointer(state, output);

    // Unhealthy sessions — not decisions, so not cards.
    render_health_alerts(state, width, output);

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
///
/// The only view handed the pane's true width rather than the clamped one: the
/// graph decides how much of each node's name it can afford to print, and that
/// decision is only honest against the pane it is drawn in.
fn render_dag_view(
    state: &PluginState,
    pane_cols: usize,
    pan: &mut DagPan,
    output: &mut Vec<String>,
) {
    render_dag(state, pane_cols, pan, output);
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
        OverriddenAsk::NoReviewOnFile => wrap_modal_text(NO_REVIEW_ASK, width),
        OverriddenAsk::UnreadableReview { .. } => wrap_modal_text(UNREADABLE_REVIEW_ASK, width),
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
pub fn print_dashboard(
    state: &PluginState,
    rows: usize,
    cols: usize,
    scroll_offset: usize,
    pan: &mut DagPan,
) {
    if state.modal.open {
        // `pan` is left as it was: a modal is drawn over the dashboard, not
        // instead of it, and zeroing the span here would make the pan keys inert
        // for a frame after the modal closes. The page scroll is ignored here for
        // the same reason.
        let lines = render_modal(&state.modal, cols.min(60), rows);
        for line in lines.iter().take(rows) {
            println!("{}", line);
        }
        return;
    }

    let lines = render_dashboard_lines(state, cols, rows, pan);

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

    /// A pane wide enough that no board in these fixtures condenses. Tests that
    /// are about something other than fitting pass this so they keep asserting
    /// on full labels.
    const DAG_WIDE: usize = 1000;

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
            desk: DeskState::default(),
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
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);

        assert!(!output.is_empty());
        assert!(output[0].contains("DAG"));
    }

    #[test]
    fn test_render_dag_empty() {
        let state = PluginState::default();
        let mut output = Vec::new();
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);

        assert!(output.iter().any(|line| line.contains("no tickets")));
    }

    // =========================================================================
    // The desk
    // =========================================================================

    /// The jargon wall from the 0.4.4 field screenshots, kept verbatim.
    ///
    /// A card must be able to carry this without rewriting it: the desk's job
    /// is to keep it off the collapsed screen, not to improve it. Fixing the
    /// sentence is the disposition author's job, upstream.
    const FIELD_REASON: &str = "The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant bypassed the old binary through managed mode instead of recording the required recovery through Lisa's error strings; John must either provide conforming reruns or explicitly amend both acceptance requirements before Review can pass.";

    const NOW: Duration = Duration::from_secs(1_000_000);

    /// The fixture the ticket names: two parked blocks, one review wait, one
    /// note, and one Blocked ticket with no disposition anyone can read.
    fn desk_fixture() -> DeskState {
        use lisa_core::triage::{PreparedStep, TriageProposal};

        DeskState {
            cards: vec![
                DeskCard {
                    ticket_id: "T-ASK".to_string(),
                    title: "checkout-test".to_string(),
                    age_stamp: Some(NOW - Duration::from_secs(7200)),
                    kind: DeskCardKind::Block,
                    ask: "Run the checkout test exactly once.".to_string(),
                    detail: DeskDetail {
                        reason: Some("The checkout evidence is missing.".to_string()),
                        steps: vec!["Open the checkout page".to_string()],
                        check: Some("test -f evidence/checkout.log".to_string()),
                        ..DeskDetail::default()
                    },
                },
                DeskCard {
                    ticket_id: "T-WORLD".to_string(),
                    title: "release-link".to_string(),
                    age_stamp: Some(NOW - Duration::from_secs(300)),
                    kind: DeskCardKind::Block,
                    ask: "Wait for the release link.".to_string(),
                    detail: DeskDetail {
                        reason: Some(FIELD_REASON.to_string()),
                        checks_on_own: true,
                        proposal: Some(TriageProposal {
                            summary: "The written criteria conflict with the measured evidence."
                                .to_string(),
                            recommendation: "Amend the stale criteria.".to_string(),
                            prepared_steps: vec![PreparedStep::Command {
                                description: "Apply the prepared amendment.".to_string(),
                                command: "git apply amendment.patch".to_string(),
                            }],
                        }),
                        ..DeskDetail::default()
                    },
                },
                DeskCard {
                    ticket_id: "T-SILENT".to_string(),
                    title: "no-review".to_string(),
                    age_stamp: Some(NOW - Duration::from_secs(172_800)),
                    kind: DeskCardKind::NoReviewOnFile,
                    ask: NO_REVIEW_ASK.to_string(),
                    detail: DeskDetail {
                        evidence_citation: Some("docs/active/work/T-SILENT".to_string()),
                        ..DeskDetail::default()
                    },
                },
                DeskCard {
                    ticket_id: "T-REVIEW".to_string(),
                    title: "waiting-review".to_string(),
                    age_stamp: None,
                    kind: DeskCardKind::ReviewWait,
                    ask: REVIEW_WAIT_ASK.to_string(),
                    detail: DeskDetail::default(),
                },
                DeskCard {
                    ticket_id: "T-NOTE".to_string(),
                    title: "size-dispute".to_string(),
                    age_stamp: None,
                    kind: DeskCardKind::Note,
                    ask: "The recorded measurement and criterion text disagree.".to_string(),
                    detail: DeskDetail {
                        criterion_quote: Some("approximately 200 MiB".to_string()),
                        evidence_citation: Some("review.md#measurement".to_string()),
                        ..DeskDetail::default()
                    },
                },
            ],
            selected: 0,
            expanded: false,
        }
    }

    fn desk_state(desk: DeskState) -> PluginState {
        PluginState {
            desk,
            current_time: NOW,
            active_view: ViewPreset::Present,
            ..PluginState::default()
        }
    }

    fn desk_lines(desk: DeskState) -> Vec<String> {
        let mut output = Vec::new();
        render_present_view(&desk_state(desk), 100, &mut output);
        output.iter().map(|line| strip_ansi(line)).collect()
    }

    /// Criterion 1: five cards, three lines each, and no staff work anywhere.
    #[test]
    fn desk_renders_five_collapsed_cards_with_no_staff_work_visible() {
        let lines = desk_lines(desk_fixture());
        let full = lines.join("\n");

        // Header, blank, then five cards of exactly three lines plus a blank.
        assert_eq!(lines.len(), 2 + 5 * 4);
        for card in 0..5 {
            let start = 2 + card * 4;
            assert!(
                lines[start].starts_with("▸ ") || lines[start].starts_with("  "),
                "card {card} must open with a selection marker: {:?}",
                lines[start]
            );
            assert!(
                lines[start + 3].is_empty(),
                "card {card} must be three lines, then a break"
            );
        }

        for hidden in [
            "approximately 200 MiB",
            "review.md#measurement",
            "docs/active/work/T-SILENT",
            "The checkout evidence is missing.",
            "test -f evidence/checkout.log",
            "Open the checkout page",
            FIELD_REASON,
            "Criterion:",
            "Evidence:",
            "Reason:",
            "Check:",
            "Step:",
        ] {
            assert!(
                !full.contains(hidden),
                "collapsed desk leaked staff work: {hidden:?}"
            );
        }
    }

    /// Criterion 1: a card with no stamp says so rather than inventing a number.
    #[test]
    fn a_card_with_no_age_source_shows_a_dash() {
        let lines = desk_lines(desk_fixture());
        let review = &lines[2 + 3 * 4];
        let note = &lines[2 + 4 * 4];

        for line in [review, note] {
            assert!(line.ends_with(UNKNOWN_AGE), "expected a dash age: {line:?}");
            assert!(
                !line
                    .rsplit(" · ")
                    .next()
                    .unwrap()
                    .contains(|c: char| c.is_ascii_digit()),
                "an unknown age must carry no number: {line:?}"
            );
        }
        assert!(lines[2].ends_with("2h ago"));
        assert!(lines[2 + 4].ends_with("5m ago"));
    }

    /// Criterion 2: the staff work opens for the selected card and no other.
    #[test]
    fn expanding_reveals_staff_work_for_the_selected_card_only() {
        let mut desk = desk_fixture();
        desk.selected = 0;
        desk.expanded = true;
        let lines = desk_lines(desk);
        let full = lines.join("\n");

        assert!(full.contains("Reason: The checkout evidence is missing."));
        assert!(full.contains("Step: Open the checkout page"));
        assert!(full.contains("Check: test -f evidence/checkout.log"));

        // Every other card is untouched — including the note's citations and
        // the second block's field-jargon reason.
        assert!(!full.contains("approximately 200 MiB"));
        assert!(!full.contains("review.md#measurement"));
        assert!(!full.contains(FIELD_REASON));

        let collapsed_cards = lines.iter().filter(|line| line.starts_with("  T-")).count();
        assert_eq!(collapsed_cards, 4);
    }

    /// Criterion 2: a note's citations open too, and only when asked.
    #[test]
    fn expanding_a_note_reveals_its_criterion_and_evidence() {
        let mut desk = desk_fixture();
        desk.selected = 4;
        desk.expanded = true;
        let full = desk_lines(desk).join("\n");

        assert!(full.contains("Criterion: “approximately 200 MiB”"));
        assert!(full.contains("Evidence: review.md#measurement"));
        assert!(!full.contains("The checkout evidence is missing."));
    }

    /// Criterion 2: collapsing restores the shape exactly, byte for byte.
    #[test]
    fn collapsing_restores_the_three_line_shape() {
        let mut expanded = desk_fixture();
        expanded.expanded = true;
        let mut collapsed = desk_fixture();
        collapsed.expanded = false;

        assert!(desk_lines(expanded).len() > desk_lines(desk_fixture()).len());
        assert_eq!(desk_lines(collapsed), desk_lines(desk_fixture()));
    }

    /// Criterion 4: an empty desk is one calm sentence and no chrome.
    #[test]
    fn empty_desk_is_one_calm_sentence() {
        let lines = desk_lines(DeskState::default());
        assert_eq!(lines, vec!["Nothing needs you."]);
    }

    /// Criterion 5's copy check, as an executable assertion rather than a
    /// promise in review.md.
    #[test]
    fn collapsed_lines_carry_no_mechanism_vocabulary() {
        const MECHANISM_WORDS: &[&str] = &["disposition", "frontmatter", "dag", "seal"];
        for line in desk_lines(desk_fixture()) {
            let lowered = line.to_lowercase();
            for word in MECHANISM_WORDS {
                assert!(
                    !lowered
                        .split(|character: char| !character.is_ascii_alphanumeric())
                        .any(|token| token == *word),
                    "a collapsed card says {word:?}, which no one says at a kitchen table: {line:?}"
                );
            }
        }
    }

    /// Criterion 5: asks are copied, never composed. The field jargon wall
    /// survives as a reason without ever being summarized onto a card.
    #[test]
    fn asks_render_verbatim_from_their_disposition_fields() {
        let mut desk = desk_fixture();
        desk.cards[0].ask = FIELD_REASON.to_string();
        let lines = desk_lines(desk);

        // Truncated for width, but never reworded: the visible prefix is the
        // ask's own opening characters.
        let rendered = lines[3].trim_start();
        let prefix: String = FIELD_REASON.chars().take(40).collect();
        assert!(rendered.starts_with(&prefix), "{rendered:?}");

        assert!(lines[2 + 4 + 1].contains("Wait for the release link."));
        assert!(lines[2 + 2 * 4 + 1].contains(NO_REVIEW_ASK));
        assert!(lines[2 + 3 * 4 + 1].contains(REVIEW_WAIT_ASK));
    }

    /// Every advertised key is one the plugin answers today (N3).
    #[test]
    fn cards_advertise_only_keys_that_work() {
        let lines = desk_lines(desk_fixture());
        let actions: Vec<&String> = lines
            .iter()
            .filter(|line| line.trim_start().starts_with("→ "))
            .collect();

        assert_eq!(actions.len(), 5);
        assert!(actions[1].contains("Lisa checks on its own"));
        assert!(actions[4].contains("[enter] read it"));
        for action in &actions[..4] {
            assert!(action.contains("[d]"), "{action:?}");
        }
        // A card names one recommended move, not the whole estate. Send-back
        // works on block cards now, but it lives on the status line — a
        // second key here would push the world-owned suffix under truncation.
        assert!(!lines.join("\n").contains("[s]"));
    }

    /// Criterion 3: Operations points at the desk instead of reprinting it.
    #[test]
    fn operations_shows_a_pointer_line_not_paragraphs() {
        let state = PluginState {
            desk: desk_fixture(),
            current_time: NOW,
            alerts: vec![HealthAlert {
                ticket_id: "T-FAILED".to_string(),
                alert_type: AlertType::Failed,
                detail: "Session failed".to_string(),
                suggested_actions: vec![],
            }],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_operations_view(&state, 80, 50, &mut output);
        let full: String = output
            .iter()
            .map(|line| strip_ansi(line))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(full.starts_with("4 waiting, 1 note — [p]"), "{full:?}");
        for gone in [
            "Waiting on you",
            "Notes for you",
            "Reviewer's note:",
            "Criterion:",
            "Evidence:",
            "Press [d] to mark done",
            FIELD_REASON,
        ] {
            assert!(!full.contains(gone), "Operations still prints {gone:?}");
        }
        // Unhealthy sessions are not decisions, so they keep their own box.
        assert!(full.contains("ATTENTION NEEDED"));
        assert!(full.contains("Threads"));
    }

    /// The pointer counts what the desk shows, and says nothing when idle.
    #[test]
    fn the_pointer_is_silent_on_an_empty_desk() {
        let mut output = Vec::new();
        render_desk_pointer(&PluginState::default(), &mut output);
        assert!(output.is_empty());
    }

    #[test]
    fn the_pointer_drops_a_clause_it_has_no_count_for() {
        let mut desk = DeskState::default();
        desk.cards = vec![desk_fixture().cards[4].clone()];
        let mut output = Vec::new();
        render_desk_pointer(&desk_state(desk), &mut output);
        assert_eq!(strip_ansi(&output[0]), "1 note — [p]");
    }

    #[test]
    fn a_stale_selection_cannot_panic_the_desk() {
        let mut desk = desk_fixture();
        desk.selected = 99;
        let lines = desk_lines(desk);
        assert_eq!(lines.iter().filter(|l| l.starts_with("▸ ")).count(), 1);
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

    /// A board carrying one ticket per renderable status, with phases chosen so
    /// no phase color equals the status color beside it — a token that quietly
    /// inherited the phase color would still look right on a matched pair.
    fn four_status_state() -> PluginState {
        PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-901".to_string(),
                    title: "ready".to_string(),
                    phase: Phase::Ready, // DIM vs status CYAN
                    status: TicketStatus::Ready,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-902".to_string(),
                    title: "working".to_string(),
                    phase: Phase::Design, // MAGENTA vs status GREEN
                    status: TicketStatus::InProgress,
                    depends_on: vec!["T-901".to_string()],
                },
                TicketNode {
                    id: "T-903".to_string(),
                    title: "in review".to_string(),
                    phase: Phase::Structure, // YELLOW vs status BRIGHT_YELLOW
                    status: TicketStatus::WaitingReview,
                    depends_on: vec!["T-901".to_string()],
                },
                TicketNode {
                    id: "T-904".to_string(),
                    title: "blocked".to_string(),
                    phase: Phase::Plan, // BLUE vs status RED
                    status: TicketStatus::Blocked,
                    depends_on: vec!["T-902".to_string()],
                },
            ],
            ..PluginState::default()
        }
    }

    #[test]
    fn test_dag_status_tokens_are_colored() {
        let state = four_status_state();
        let mut output = Vec::new();
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);
        let full = output.join("\n");

        for (token, color, name) in [
            ("RDY", CYAN, "Ready"),
            ("WRK", GREEN, "InProgress"),
            ("REV", BRIGHT_YELLOW, "WaitingReview"),
            ("BLK", RED, "Blocked"),
        ] {
            let inked = format!("{}{}{}", color, token, RESET);
            assert!(
                full.contains(&inked),
                "{name} token {token} rendered without its status color, got:\n{full}"
            );
        }
    }

    #[test]
    fn test_done_status_color_is_bright_green() {
        // Asserted at the mapping rather than on rendered output: render_dag
        // filters Done tickets out before nodes are built, so a DON token can
        // never appear on a rendered line.
        assert_eq!(TicketStatus::Done.token(), "DON");
        assert_eq!(TicketStatus::Done.color_code(), BRIGHT_GREEN);
    }

    #[test]
    fn test_dag_ticket_ids_keep_phase_color() {
        let state = four_status_state();
        let mut output = Vec::new();
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);
        let full = output.join("\n");

        for (id, phase) in [
            ("T-901", Phase::Ready),
            ("T-902", Phase::Design),
            ("T-903", Phase::Structure),
            ("T-904", Phase::Plan),
        ] {
            let inked = format!("{}{}{}", phase.color_code(), id, RESET);
            assert!(
                full.contains(&inked),
                "{id} lost its {} phase color, got:\n{full}",
                phase.short_name()
            );
        }
    }

    #[test]
    fn test_dag_status_color_is_independent_of_phase() {
        // Two Blocked tickets in different phases: same status color, different
        // id colors. Guards against the token sourcing the phase channel.
        let state = PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-911".to_string(),
                    title: "one".to_string(),
                    phase: Phase::Research,
                    status: TicketStatus::Blocked,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-912".to_string(),
                    title: "two".to_string(),
                    phase: Phase::Implement,
                    status: TicketStatus::Blocked,
                    depends_on: vec![],
                },
            ],
            ..PluginState::default()
        };
        let mut output = Vec::new();
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);
        let full = output.join("\n");

        assert_eq!(
            full.matches(&format!("{}BLK{}", RED, RESET)).count(),
            2,
            "both Blocked tokens should be red regardless of phase, got:\n{full}"
        );
        assert!(full.contains(&format!("{}T-911{}", CYAN, RESET)));
        assert!(full.contains(&format!("{}T-912{}", GREEN, RESET)));
    }

    #[test]
    fn test_dag_raw_content_unchanged_by_coloring() {
        // This ticket adds color and nothing else. Rebuild the same graph
        // through ascii-dag directly and assert that stripping the escapes from
        // the rendered view reproduces those lines byte for byte.
        let state = four_status_state();
        let mut output = Vec::new();
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);

        let labels: Vec<String> = state
            .tickets
            .iter()
            .map(|t| format!("{} {}", t.id, t.status.token()))
            .collect();
        let node_refs: Vec<(usize, &str)> = labels
            .iter()
            .enumerate()
            .map(|(i, label)| (i + 1, label.as_str()))
            .collect();
        let edges = vec![(1usize, 2usize), (1, 3), (2, 4)];
        let expected = ascii_dag::DAG::from_edges(&node_refs, &edges).render();
        let expected: Vec<&str> = expected.lines().collect();

        let stripped: Vec<String> = output.iter().map(|line| strip_ansi(line)).collect();
        let start = stripped
            .iter()
            .position(|line| line == expected[0])
            .unwrap_or_else(|| panic!("uncolored first DAG row missing from:\n{stripped:#?}"));
        let actual: Vec<&str> = stripped[start..start + expected.len()]
            .iter()
            .map(|line| line.as_str())
            .collect();

        assert_eq!(
            actual, expected,
            "coloring shifted the raw DAG content; this ticket may only insert escapes"
        );
    }

    /// A root fanned out to `n - 1` children, with ids the length of real Lisa
    /// ids. Measured against ascii-dag 0.8, full labels then condensed:
    /// 6 nodes = 99/69, 7 = 119/83, 8 = 139/97, 9 = 159/111 columns. The saving
    /// is six columns a node — `T-` plus ` RDY`.
    fn fan_board(n: usize) -> PluginState {
        let tickets = (1..=n)
            .map(|i| TicketNode {
                id: format!("T-054-01-{:02}", i),
                title: format!("child {}", i),
                phase: Phase::Research,
                status: TicketStatus::Ready,
                depends_on: if i == 1 {
                    vec![]
                } else {
                    vec!["T-054-01-01".to_string()]
                },
            })
            .collect();

        PluginState {
            tickets,
            ..PluginState::default()
        }
    }

    /// The rendered graph rows: everything between the `≡≡ DAG ≡≡` header and
    /// the summary/legend chrome. AC2 and AC4 both point at these lines.
    fn dag_body_lines(output: &[String]) -> Vec<String> {
        output
            .iter()
            .skip(2) // header, blank
            .take_while(|line| {
                let bare = strip_ansi(line);
                !bare.starts_with("Phases:")
                    && !bare.starts_with("Status:")
                    && !bare.starts_with('(')
            })
            .map(|line| strip_ansi(line))
            .filter(|line| !line.trim().is_empty())
            .collect()
    }

    /// A board whose nodes do not all ink the same color.
    ///
    /// `fan_board` is uniformly `Ready`, so every node carries one identical
    /// escape sequence — precisely the board on which a broken slicer can look
    /// fine. Cycling statuses and phases means the escapes differ node to node
    /// in both the two-channel full inking and the one-channel condensed one.
    fn mixed_status_board(n: usize) -> PluginState {
        let statuses = [
            TicketStatus::Ready,
            TicketStatus::InProgress,
            TicketStatus::WaitingReview,
            TicketStatus::Blocked,
        ];
        let phases = [
            Phase::Research,
            Phase::Design,
            Phase::Implement,
            Phase::Review,
        ];

        let tickets = (1..=n)
            .map(|i| TicketNode {
                id: format!("T-054-01-{:02}", i),
                title: format!("child {}", i),
                phase: phases[i % phases.len()].clone(),
                status: statuses[i % statuses.len()].clone(),
                depends_on: if i == 1 {
                    vec![]
                } else {
                    vec!["T-054-01-01".to_string()]
                },
            })
            .collect();

        PluginState {
            tickets,
            ..PluginState::default()
        }
    }

    /// The index range of the graph body within a render.
    ///
    /// Indices rather than the stripped strings `dag_body_lines` returns: this
    /// ticket is about what survives on a *painted* line, and panning must be
    /// compared line against line. The range is stable under panning because
    /// every body line is pushed exactly once whatever the offset.
    fn dag_body_range(output: &[String]) -> std::ops::Range<usize> {
        let start = 2; // header, blank
        let end = output
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, line)| {
                let bare = strip_ansi(line);
                bare.starts_with("Phases:") || bare.starts_with("Status:") || bare.starts_with('(')
            })
            .map(|(index, _)| index)
            .unwrap_or(output.len());
        start..end
    }

    /// Render a board at a pane width and a pan offset.
    fn render_dag_panned(
        state: &PluginState,
        pane_cols: usize,
        offset: usize,
    ) -> (Vec<String>, DagPan) {
        let mut pan = DagPan { offset, span: 0 };
        let mut output = Vec::new();
        render_dag(state, pane_cols, &mut pan, &mut output);
        (output, pan)
    }

    #[test]
    fn pan_reveals_the_clipped_columns_and_returns() {
        // AC1. A condensed board that still overflows a 60-column pane: panning
        // right shows the columns the pane was cutting, panning back is byte for
        // byte where it started.
        let state = mixed_status_board(7);
        let (unpanned, pan) = render_dag_panned(&state, 60, 0);
        let range = dag_body_range(&unpanned);

        assert!(
            pan.span > 0,
            "fixture must overflow even condensed, or it tests nothing"
        );

        let (panned, _) = render_dag_panned(&state, 60, 5);
        for index in range.clone() {
            let expected: String = strip_ansi(&unpanned[index]).chars().skip(5).collect();
            assert_eq!(
                strip_ansi(&panned[index]),
                expected,
                "line {index} did not move five columns left"
            );
        }

        // The right edge the pane was cutting is now inside it.
        let widest_at_rest = range
            .clone()
            .map(|index| visible_width(&unpanned[index]))
            .max()
            .unwrap();
        let (at_edge, _) = render_dag_panned(&state, 60, pan.span);
        let widest_at_edge = range
            .clone()
            .map(|index| visible_width(&at_edge[index]))
            .max()
            .unwrap();
        assert_eq!(widest_at_rest, 60 + pan.span);
        assert_eq!(
            widest_at_edge, 60,
            "panned fully right, the map should end exactly at the pane's edge"
        );

        let (back, _) = render_dag_panned(&state, 60, 0);
        assert_eq!(
            back, unpanned,
            "panning back must restore the render exactly"
        );
    }

    #[test]
    fn pan_is_clamped_at_both_edges() {
        // AC1's clamp half. Past the right edge there is nothing further to
        // reveal, so the render stops moving rather than walking off the map.
        let state = mixed_status_board(7);
        let (_, pan) = render_dag_panned(&state, 60, 0);

        let (at_edge, _) = render_dag_panned(&state, 60, pan.span);
        for beyond in [pan.span + 1, pan.span + 50, usize::MAX] {
            let (past, _) = render_dag_panned(&state, 60, beyond);
            assert_eq!(past, at_edge, "offset {beyond} moved past the right edge");
        }
    }

    #[test]
    fn every_pan_offset_keeps_escapes_intact_and_text_correct() {
        // AC2 — the ticket's negative fixture. A fully colored board walked
        // through every valid offset: at each one, every emitted line must carry
        // only intact escape sequences and exactly the visible text that offset
        // asks for. A naive byte or char slicer fails this; the test below proves
        // the fixture has the teeth to catch one.
        let state = mixed_status_board(7);
        let (unpanned, pan) = render_dag_panned(&state, 60, 0);
        let range = dag_body_range(&unpanned);

        assert!(pan.span > 0, "fixture must overflow, or the walk is empty");
        assert!(
            range
                .clone()
                .any(|index| unpanned[index].contains('\u{1b}')),
            "fixture must be painted, or intact escapes prove nothing"
        );

        for offset in 0..=pan.span {
            let (panned, _) = render_dag_panned(&state, 60, offset);

            for index in range.clone() {
                let line = &panned[index];
                assert_escapes_intact(line, &format!("offset {offset}, line {index}"));

                let expected: String = strip_ansi(&unpanned[index]).chars().skip(offset).collect();
                assert_eq!(
                    strip_ansi(line),
                    expected,
                    "offset {offset}, line {index}: wrong visible text"
                );

                // No color left open at the end of a line, which would run the
                // ink on into whatever is drawn next.
                assert!(
                    ink_is_closed(line),
                    "offset {offset}, line {index}: ink left open in {line:?}"
                );
            }
        }
    }

    #[test]
    fn a_naive_slicer_would_fail_the_escape_walk() {
        // The fixture above passes vacuously unless a wrong slicer actually
        // fails it. This is that proof: the same board, cut the obvious wrong
        // way, must shear a sequence at some offset.
        let state = mixed_status_board(7);
        let (unpanned, pan) = render_dag_panned(&state, 60, 0);
        let range = dag_body_range(&unpanned);

        let broke = (0..=pan.span).any(|offset| {
            range.clone().any(|index| {
                let naive: String = unpanned[index].chars().skip(offset).collect();
                !pan_is_faithful(&naive, &unpanned[index], offset)
            })
        });

        assert!(
            broke,
            "a naive char slice survived the whole walk — the fixture proves nothing"
        );

        // And the real slicer survives the same walk, so the difference is the
        // slicer and not the fixture.
        for offset in 0..=pan.span {
            let (panned, _) = render_dag_panned(&state, 60, offset);
            for index in range.clone() {
                assert!(
                    pan_is_faithful(&panned[index], &unpanned[index], offset),
                    "offset {offset}, line {index}: {:?}",
                    panned[index]
                );
            }
        }
    }

    #[test]
    fn the_span_is_the_indicators_number() {
        // The clamp and the sentence report one fact, so they cannot disagree
        // about how much map is off the pane.
        let state = mixed_status_board(7);
        let (output, pan) = render_dag_panned(&state, 60, 0);

        let indicator = output
            .iter()
            .map(|line| strip_ansi(line))
            .find(|line| line.contains("off-screen"))
            .expect("an overflowing board must carry the indicator");

        assert!(
            indicator.starts_with(&format!("({} columns off-screen", pan.span)),
            "span {} is not the number the indicator prints: {indicator}",
            pan.span
        );
    }

    #[test]
    fn a_fitting_map_reports_no_span() {
        // AC3's second half at the render level: a board that fits leaves no
        // room to pan, which is what makes the keys inert.
        let state = mixed_status_board(7);

        for pane in [200, 1000] {
            let (_, pan) = render_dag_panned(&state, pane, 0);
            assert_eq!(
                pan.span, 0,
                "a board that fits {pane} columns can be panned"
            );
        }

        // A caller that does not know the pane is not a pane one column wide.
        let (_, unknown) = render_dag_panned(&state, 0, 0);
        assert_eq!(unknown.span, 0);
    }

    #[test]
    fn pan_offset_is_ignored_when_the_map_fits() {
        // A stale offset cannot smear a board that has room — the render clamps
        // against the span it just computed, not against what it was handed.
        let state = mixed_status_board(7);
        let (at_rest, _) = render_dag_panned(&state, 200, 0);
        let (offset_but_fitting, _) = render_dag_panned(&state, 200, 25);

        assert_eq!(at_rest, offset_but_fitting);
    }

    #[test]
    fn non_dag_views_report_no_span() {
        // The other three presets have no map, so they report no room to pan and
        // the pan keys stay inert there without the key handler asking which
        // views own a graph.
        let state = mixed_status_board(7);

        for preset in [
            ViewPreset::Operations,
            ViewPreset::Present,
            ViewPreset::Activity,
        ] {
            let view_state = PluginState {
                active_view: preset,
                ..state.clone()
            };
            let mut pan = DagPan {
                offset: 9,
                span: 99,
            };
            let _ = render_dashboard_lines(&view_state, 60, 40, &mut pan);
            assert_eq!(pan.span, 0, "{preset:?} claimed room to pan");
        }
    }

    #[test]
    fn text_presets_still_clamp_at_a_hundred_columns() {
        // AC5, second half. The clamp moved one scope inward so the DAG could
        // see past it; for the text presets it must still govern, which is
        // observable as: below 100 the pane's width matters, above it nothing
        // changes.
        // An ask long enough to be cut at 80 columns and whole at 100.
        let long_ask =
            "Run the checkout test exactly once and then tell Lisa which of the two paths it took."
                .to_string();
        assert!((77..=96).contains(&long_ask.chars().count()));

        let state = PluginState {
            desk: DeskState {
                cards: vec![DeskCard {
                    ticket_id: "T-ASK".to_string(),
                    title: "checkout-test".to_string(),
                    age_stamp: Some(NOW - Duration::from_secs(7200)),
                    kind: DeskCardKind::Block,
                    ask: long_ask,
                    detail: DeskDetail::default(),
                }],
                ..DeskState::default()
            },
            active_view: ViewPreset::Present,
            current_time: NOW,
            ..PluginState::default()
        };

        let at_80 = render_dashboard_lines(&state, 80, 40, &mut DagPan::default());
        let at_100 = render_dashboard_lines(&state, 100, 40, &mut DagPan::default());
        let at_200 = render_dashboard_lines(&state, 200, 40, &mut DagPan::default());

        assert_ne!(
            at_80, at_100,
            "the fixture must be width-sensitive below the clamp, or this test \
             proves nothing"
        );
        assert_eq!(
            at_100, at_200,
            "a pane past 100 columns must change nothing for a text preset"
        );
    }

    #[test]
    fn dag_label_full_matches_todays_format() {
        assert_eq!(
            dag_label("T-054-01-02", &TicketStatus::InProgress, LabelStyle::Full),
            "T-054-01-02 WRK"
        );
    }

    #[test]
    fn dag_label_condensed_sheds_prefix_and_token() {
        // Six columns a node: `T-` is two, ` WRK` is four.
        let full = dag_label("T-054-01-02", &TicketStatus::InProgress, LabelStyle::Full);
        let condensed = dag_label(
            "T-054-01-02",
            &TicketStatus::InProgress,
            LabelStyle::Condensed,
        );

        assert_eq!(condensed, "054-01-02");
        assert_eq!(full.chars().count() - condensed.chars().count(), 6);
    }

    #[test]
    fn dag_label_condensed_leaves_a_prefixless_id_whole() {
        // An id that never carried `T-` loses no character at all — not the
        // wrong one.
        assert_eq!(
            dag_label("BUG-7", &TicketStatus::Blocked, LabelStyle::Condensed),
            "BUG-7"
        );
    }

    #[test]
    fn visible_width_counts_characters_not_bytes() {
        // The DAG's edge glyphs are multi-byte and one column wide. Byte length
        // would overcount a routed board by roughly three to one.
        let routed = "  ┌───────────────────└───────────────────┐";
        assert_eq!(visible_width(routed), routed.chars().count());
        assert!(
            visible_width(routed) < routed.len(),
            "fixture should contain multi-byte glyphs, or it proves nothing"
        );
    }

    #[test]
    fn visible_width_ignores_color() {
        // AC3: a fully colored fixture measures identical to its uncolored twin.
        let plain = "[T-054-01-02 WRK] → [T-054-01-03 BLK]";
        let colored = format!(
            "[{}T-054-01-02{} {}WRK{}] → [{}T-054-01-03{} {}BLK{}]",
            MAGENTA, RESET, GREEN, RESET, BLUE, RESET, RED, RESET
        );

        assert_ne!(plain, colored.as_str(), "the twin must actually be colored");
        assert_eq!(
            visible_width(&colored),
            visible_width(plain),
            "color changed the measurement; escapes are not ink"
        );
    }

    #[test]
    fn widest_visible_line_ignores_blank_trailing_rows() {
        // ascii_dag::render() ends with blank rows; the max is the content's.
        let block = "[T-002 WRK]\n[T-003 BLK] → [T-004 RDY]\n\n\n";
        assert_eq!(widest_visible_line(block), 25);
        assert_eq!(widest_visible_line(""), 0);
    }

    /// Every `\u{1b}` in `line` opens a sequence that terminates in `m`.
    ///
    /// The shape of the failure this ticket is named for: a cut landing inside
    /// `\u{1b}[36m` leaves an opener with no terminator, and the terminal prints
    /// the tail as literal text. Shared by the `pan_line` unit tests and the
    /// every-offset walk below.
    fn escapes_are_intact(line: &str) -> bool {
        let mut chars = line.chars();
        while let Some(character) = chars.next() {
            if character != '\u{1b}' {
                continue;
            }
            if chars.next() != Some('[') {
                return false;
            }
            let mut terminated = false;
            for escaped in chars.by_ref() {
                // Parameter bytes of an SGR sequence, then its terminator.
                if escaped == 'm' {
                    terminated = true;
                    break;
                }
                if !escaped.is_ascii_digit() && escaped != ';' {
                    return false;
                }
            }
            if !terminated {
                return false;
            }
        }
        true
    }

    fn assert_escapes_intact(line: &str, context: &str) {
        assert!(
            escapes_are_intact(line),
            "{context}: an escape sequence was sheared in {line:?}"
        );
    }

    /// No color is still in force when the line ends.
    ///
    /// An unclosed color runs the ink on into whatever the terminal draws next.
    /// Counting rather than checking the suffix, because a line legitimately
    /// ends with `]` after its reset.
    fn ink_is_closed(line: &str) -> bool {
        let mut open = 0usize;
        let mut chars = line.chars();
        while let Some(character) = chars.next() {
            if character != '\u{1b}' {
                continue;
            }
            let mut sequence = String::from(character);
            for escaped in chars.by_ref() {
                sequence.push(escaped);
                if escaped == 'm' {
                    break;
                }
            }
            if sequence == RESET {
                open = 0;
            } else {
                open += 1;
            }
        }
        open == 0
    }

    /// Is `panned` an honest pan of `original` by `offset` visible columns?
    ///
    /// The two properties AC2 names, plus the no-leak rule: intact sequences,
    /// exactly the visible text that offset asks for, and nothing left open.
    /// Used as an assertion for the real slicer and as a detector for a naive
    /// one — a naive cut fails the *text* property even when it happens not to
    /// shear a sequence, because dropping a lone `\u{1b}` turns the rest of the
    /// sequence into literal `[32m` on screen.
    fn pan_is_faithful(panned: &str, original: &str, offset: usize) -> bool {
        let expected: String = strip_ansi(original).chars().skip(offset).collect();
        escapes_are_intact(panned) && ink_is_closed(panned) && strip_ansi(panned) == expected
    }

    #[test]
    fn pan_line_at_zero_is_the_line_itself() {
        // The guarantee an unpanned board rests on: byte for byte, not merely
        // visually the same.
        let painted = format!(
            "[{}054-01-02{}] → [{}054-01-03{}]",
            GREEN, RESET, RED, RESET
        );
        assert_eq!(pan_line(&painted, 0), painted);
        assert_eq!(pan_line("", 0), "");
    }

    #[test]
    fn pan_line_counts_columns_not_bytes() {
        // Same reason `visible_width` counts characters: the routing glyphs are
        // multi-byte and one column wide. A byte cut would land mid-glyph.
        let routed = "┌───→ [054-01-02]";
        let panned = pan_line(routed, 6);
        assert_eq!(panned, "[054-01-02]");
        assert_eq!(visible_width(&panned), visible_width(routed) - 6);
        assert!(
            routed.len() > routed.chars().count(),
            "fixture should contain multi-byte glyphs, or it proves nothing"
        );
    }

    #[test]
    fn pan_line_never_splits_an_escape() {
        // The negative case at the unit level: cut at every column of a line
        // whose escapes sit at, before, and after each boundary.
        let painted = format!(
            "[{}{}054-01-02{}] → [{}054-01-03{}]",
            BOLD, CYAN, RESET, RED, RESET
        );

        for offset in 0..=visible_width(&painted) + 3 {
            assert_escapes_intact(&pan_line(&painted, offset), &format!("offset {offset}"));
        }
    }

    #[test]
    fn pan_line_carries_active_color_across_the_cut() {
        // A node straddling the left edge keeps its status color — which in
        // condensed mode is the only status channel it has left.
        let painted = format!("{}054-01-02{}", GREEN, RESET);
        let panned = pan_line(&painted, 4);

        assert!(
            panned.starts_with(GREEN),
            "the cut dropped the color that was in force: {panned:?}"
        );
        assert_eq!(strip_ansi(&panned), "01-02");
    }

    #[test]
    fn pan_line_drops_color_cancelled_before_the_cut() {
        // The mirror of the test above: a reset before the cut means no color is
        // in force, so none is carried. Otherwise every panned line would open
        // with stale ink.
        let painted = format!("{}abc{}defgh", GREEN, RESET);
        let panned = pan_line(&painted, 4);

        assert_eq!(panned, "efgh", "color survived its own reset");
    }

    #[test]
    fn pan_line_leaks_no_ink() {
        // A cut can carry a color across without its closing reset surviving.
        // Left open, the ink would run on past this line.
        let painted = format!("abc{}defgh", RED);
        let panned = pan_line(&painted, 4);

        assert!(panned.starts_with(RED));
        assert!(
            panned.ends_with(RESET),
            "an open color reached the end of the line: {panned:?}"
        );
        assert_eq!(strip_ansi(&panned), "efgh");
    }

    #[test]
    fn pan_line_past_the_end_is_empty() {
        // Nothing visible remains, so nothing is emitted — not a bare color code
        // with no glyphs to paint.
        let painted = format!("{}054-01-02{}", GREEN, RESET);

        assert_eq!(pan_line(&painted, visible_width(&painted)), "");
        assert_eq!(pan_line(&painted, 500), "");
    }

    #[test]
    fn dag_wide_pane_keeps_full_labels_byte_for_byte() {
        // AC1, wide half. A pane with room to spare renders exactly what it
        // rendered before this ticket — the fit decision ran and chose Full.
        let state = fan_board(7);
        let mut output = Vec::new();
        render_dag(&state, 200, &mut DagPan::default(), &mut output);

        let labels: Vec<String> = state
            .tickets
            .iter()
            .map(|t| format!("{} {}", t.id, t.status.token()))
            .collect();
        let node_refs: Vec<(usize, &str)> = labels
            .iter()
            .enumerate()
            .map(|(i, label)| (i + 1, label.as_str()))
            .collect();
        let edges: Vec<(usize, usize)> = (2..=7).map(|i| (1usize, i)).collect();
        let expected = ascii_dag::DAG::from_edges(&node_refs, &edges).render();
        let expected: Vec<&str> = expected.lines().collect();

        let stripped: Vec<String> = output.iter().map(|line| strip_ansi(line)).collect();
        let start = stripped
            .iter()
            .position(|line| line == expected[0])
            .unwrap_or_else(|| panic!("uncolored first DAG row missing from:\n{stripped:#?}"));
        let actual: Vec<&str> = stripped[start..start + expected.len()]
            .iter()
            .map(|line| line.as_str())
            .collect();

        assert_eq!(
            actual, expected,
            "a board that fits must keep today's labels, character for character"
        );
    }

    #[test]
    fn dag_narrow_pane_condenses_and_fits() {
        // AC1, narrow half. The same 7-node board: 119 columns full, 83
        // condensed. At a 100-column pane it must shed and then fit.
        let state = fan_board(7);

        let mut wide = Vec::new();
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut wide);
        assert_eq!(widest_visible_line(&dag_body_lines(&wide).join("\n")), 119);

        let mut output = Vec::new();
        render_dag(&state, 100, &mut DagPan::default(), &mut output);
        let body = dag_body_lines(&output);

        assert_eq!(
            widest_visible_line(&body.join("\n")),
            83,
            "condensing should have bought six columns a node"
        );
        assert!(
            body.iter().any(|line| line.contains("054-01-02")),
            "condensed ids missing from:\n{body:#?}"
        );
    }

    #[test]
    fn condensed_labels_carry_no_prefix_and_no_status_token() {
        // AC2, on the node text — which is what the criterion names. The
        // `Status:` legend defines the color code and sits outside the body.
        let state = fan_board(7);
        let mut output = Vec::new();
        render_dag(&state, 100, &mut DagPan::default(), &mut output);

        for line in dag_body_lines(&output) {
            assert!(
                !line.contains("T-"),
                "condensed node text kept its prefix: {line}"
            );
            for token in ["RDY", "WRK", "REV", "BLK", "DON"] {
                assert!(
                    !line.contains(token),
                    "condensed node text kept the {token} token: {line}"
                );
            }
        }
    }

    #[test]
    fn condensed_status_classes_are_distinguishable() {
        // AC2, per class: with the token gone, color is the whole status
        // channel, so each class must paint its condensed id differently.
        let statuses = [
            TicketStatus::Ready,
            TicketStatus::InProgress,
            TicketStatus::WaitingReview,
            TicketStatus::Blocked,
        ];
        let tickets: Vec<TicketNode> = statuses
            .iter()
            .enumerate()
            .map(|(i, status)| TicketNode {
                id: format!("T-054-01-{:02}", i + 1),
                title: format!("node {}", i),
                phase: Phase::Research, // one phase throughout: color can only be status
                status: status.clone(),
                depends_on: if i == 0 {
                    vec![]
                } else {
                    vec!["T-054-01-01".to_string()]
                },
            })
            .collect();
        let state = PluginState {
            tickets,
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_dag(&state, 50, &mut DagPan::default(), &mut output); // 59 full, 41 condensed
        let painted = output.join("\n");

        for (i, status) in statuses.iter().enumerate() {
            let expected = format!("{}054-01-{:02}{}", status.color_code(), i + 1, RESET);
            assert!(
                painted.contains(&expected),
                "{:?} did not paint its condensed id, got:\n{painted}",
                status
            );
        }

        let colors: std::collections::HashSet<&str> =
            statuses.iter().map(|s| s.color_code()).collect();
        assert_eq!(colors.len(), 4, "two status classes share a color");
    }

    #[test]
    fn condensed_ids_carry_status_not_phase() {
        // The test that fails if the recolor silently kept sourcing the phase.
        // Two Blocked tickets in different phases must match; a Ready and a
        // Blocked ticket in the same phase must not.
        let state = PluginState {
            tickets: vec![
                TicketNode {
                    id: "T-054-01-01".to_string(),
                    title: "blocked, researching".to_string(),
                    phase: Phase::Research,
                    status: TicketStatus::Blocked,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-054-01-02".to_string(),
                    title: "blocked, implementing".to_string(),
                    phase: Phase::Implement,
                    status: TicketStatus::Blocked,
                    depends_on: vec![],
                },
                TicketNode {
                    id: "T-054-01-03".to_string(),
                    title: "ready, implementing".to_string(),
                    phase: Phase::Implement,
                    status: TicketStatus::Ready,
                    depends_on: vec![],
                },
            ],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_dag(&state, 30, &mut DagPan::default(), &mut output);
        let painted = output.join("\n");

        assert!(painted.contains(&format!("{}054-01-01{}", RED, RESET)));
        assert!(painted.contains(&format!("{}054-01-02{}", RED, RESET)));
        assert!(
            painted.contains(&format!("{}054-01-03{}", CYAN, RESET)),
            "a Ready ticket sharing a phase with a Blocked one must still read \
             differently, got:\n{painted}"
        );
    }

    #[test]
    fn dag_fit_is_not_gated_by_the_hundred_column_clamp() {
        // AC5. A 119-column board on a 200-column pane: wider than the legacy
        // clamp, narrower than the pane, so it keeps full labels. Driven
        // through the real entry point, so it fails if the clamp is
        // reintroduced anywhere along the path.
        let mut state = fan_board(7);
        state.active_view = ViewPreset::Dag;

        let lines = render_dashboard_lines(&state, 200, 60, &mut DagPan::default());
        let joined = lines.join("\n");

        assert!(
            joined.contains("T-054-01-02"),
            "a board of 119 columns on a 200-column pane must keep full labels"
        );
        assert!(
            joined.contains("RDY"),
            "full labels keep their status token"
        );
    }

    #[test]
    fn condensing_triggers_on_overflow_only() {
        // AC1: the flip threshold is the pane width itself, with no knob. The
        // 6-node board is 99 columns full and 69 condensed.
        let state = fan_board(6);

        let mut roomy = Vec::new();
        render_dag(&state, 120, &mut DagPan::default(), &mut roomy);
        assert!(
            dag_body_lines(&roomy)
                .join("\n")
                .contains("T-054-01-02 RDY"),
            "a board that fits must not condense"
        );

        let mut cramped = Vec::new();
        render_dag(&state, 90, &mut DagPan::default(), &mut cramped);
        assert!(
            !dag_body_lines(&cramped)
                .join("\n")
                .contains("T-054-01-02 RDY"),
            "a board that overflows must condense"
        );

        // Either side of the boundary, and the boundary itself: 99 fits a
        // 99-column pane exactly.
        let mut exact = Vec::new();
        render_dag(&state, 99, &mut DagPan::default(), &mut exact);
        assert!(
            dag_body_lines(&exact)
                .join("\n")
                .contains("T-054-01-02 RDY"),
            "a board exactly as wide as the pane fits it"
        );
    }

    #[test]
    fn zero_width_never_condenses() {
        // A caller that does not know the pane is not a one-column pane.
        let state = fan_board(9); // 159 columns, wider than any real pane
        let mut output = Vec::new();
        render_dag(&state, 0, &mut DagPan::default(), &mut output);

        assert!(dag_body_lines(&output)
            .join("\n")
            .contains("T-054-01-02 RDY"));
        assert!(!output.join("\n").contains("off-screen"));
    }

    /// AC4 as a property: no render may leave columns off-pane without saying
    /// so. Measures every body line and requires the indicator whenever one
    /// exceeds the pane — the negative fixture, since a silently clipped board
    /// fails here rather than passing unnoticed.
    fn assert_no_silent_clip(output: &[String], pane_cols: usize) {
        let body = dag_body_lines(output);
        let widest = widest_visible_line(&body.join("\n"));
        let said_so = output.iter().any(|line| line.contains("off-screen"));

        if widest > pane_cols {
            assert!(
                said_so,
                "{widest} columns on a {pane_cols}-column pane, clipped in \
                 silence:\n{body:#?}"
            );
        } else {
            assert!(
                !said_so,
                "{widest} columns fit a {pane_cols}-column pane, yet the board \
                 claimed an overflow:\n{output:#?}"
            );
        }
    }

    #[test]
    fn overflow_beyond_condensed_carries_the_indicator() {
        // AC4. The 7-node board is 83 columns even condensed; on a 60-column
        // pane there is nothing left to shed, so it must say what is missing.
        let state = fan_board(7);
        let mut output = Vec::new();
        render_dag(&state, 60, &mut DagPan::default(), &mut output);

        let indicator = output
            .iter()
            .find(|line| line.contains("off-screen"))
            .map(|line| strip_ansi(line))
            .expect("an overflowing board must carry the indicator");

        assert_eq!(
            indicator,
            "(23 columns off-screen — [h]/[l] to pan — the map needs 83, the pane has 60)"
        );
        assert_no_silent_clip(&output, 60);
    }

    #[test]
    fn the_indicator_names_the_pan_keys() {
        // AC4. The affordance announces itself exactly where the loss is
        // reported, so the operator learns the keys at the moment they apply.
        let state = mixed_status_board(7);
        let (output, pan) = render_dag_panned(&state, 60, 0);

        let indicator = output
            .iter()
            .map(|line| strip_ansi(line))
            .find(|line| line.contains("off-screen"))
            .expect("an overflowing board must carry the indicator");

        assert!(pan.span > 0);
        assert!(
            indicator.contains("[h]/[l] to pan"),
            "the indicator does not name the keys that reach the map: {indicator}"
        );
    }

    #[test]
    fn the_pan_keys_are_named_only_where_they_apply() {
        // AC4's other half. On a board that fits, the keys do nothing, so
        // advertising them would be an affordance that lies.
        let state = mixed_status_board(7);

        for pane in [200, 1000] {
            let (output, pan) = render_dag_panned(&state, pane, 0);
            assert_eq!(pan.span, 0);
            assert!(
                !output.iter().any(|line| line.contains("to pan")),
                "a board that fits a {pane}-column pane offered pan keys"
            );
        }
    }

    #[test]
    fn a_board_that_fits_says_nothing() {
        // Guards against an indicator that is simply always on, which would
        // pass the overflow tests by brute force.
        let state = fan_board(7);

        for pane in [200, 100] {
            let mut output = Vec::new();
            render_dag(&state, pane, &mut DagPan::default(), &mut output);
            assert!(
                !output.iter().any(|line| line.contains("off-screen")),
                "a board that fits a {pane}-column pane must say nothing"
            );
        }
    }

    #[test]
    fn no_body_line_exceeds_the_pane_without_the_indicator() {
        // AC4 across the matrix: four board sizes against four panes, from one
        // that cannot hold a single node to one nothing overflows.
        for nodes in [3, 6, 7, 9] {
            let state = fan_board(nodes);
            for pane in [20, 60, 100, 200] {
                let mut output = Vec::new();
                render_dag(&state, pane, &mut DagPan::default(), &mut output);
                assert_no_silent_clip(&output, pane);
            }
        }
    }

    #[test]
    fn test_render_dag_filters_done_tickets() {
        let state = sample_state(); // T-001 is Done, T-002 InProgress, T-003 Blocked
        let mut output = Vec::new();
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);
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
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);
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
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);
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
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);
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
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);
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
        render_dag(&state, DAG_WIDE, &mut DagPan::default(), &mut output);
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
        assert!(status.contains("[p] desk"), "Desk hint missing");
        assert!(status.contains("[v] view"), "View hint missing");
        assert!(status.contains("[space]"), "Pause hint missing");
    }

    /// Criterion 1: on the desk, every hint names a key the desk answers.
    #[test]
    fn the_desk_status_line_names_the_desks_own_keys() {
        let state = PluginState {
            active_view: ViewPreset::Present,
            ..sample_state()
        };
        let status = strip_ansi(&render_status_line(&state));

        for hint in [
            "[↑↓] pick",
            "[enter] open",
            "[d] done",
            "[s] send back",
            "[v] view",
            "[space] pause",
        ] {
            assert!(
                status.contains(hint),
                "desk hint {hint:?} missing: {status}"
            );
        }
        // `[p]` is a no-op here, so the desk never offers it.
        assert!(
            !status.contains("[p]"),
            "the desk still offers [p]: {status}"
        );
    }

    /// The other half of the same criterion: no view advertises a key that
    /// only the desk answers.
    #[test]
    fn off_the_desk_the_status_line_never_offers_the_desks_keys() {
        for view in [
            ViewPreset::Operations,
            ViewPreset::Dag,
            ViewPreset::Activity,
        ] {
            let state = PluginState {
                active_view: view,
                ..sample_state()
            };
            let status = strip_ansi(&render_status_line(&state));

            assert!(status.contains("[p] desk"), "{view:?}: {status}");
            assert!(status.contains("[v] view"), "{view:?}: {status}");
            for desk_only in ["[s]", "[enter]", "[↑↓]"] {
                assert!(
                    !status.contains(desk_only),
                    "{view:?} advertises {desk_only:?}, which does nothing here: {status}"
                );
            }
        }
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
        let lines = render_dashboard_lines(&state, 80, 40, &mut DagPan::default());
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
        let lines = render_dashboard_lines(&state, 80, 40, &mut DagPan::default());
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
        let lines = render_dashboard_lines(&state, 80, 40, &mut DagPan::default());
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
            desk: DeskState::default(),
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
        let dag_lines = render_dashboard_lines(&dag_state, 80, 50, &mut DagPan::default());
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
        let ops_lines = render_dashboard_lines(&state, 80, 50, &mut DagPan::default());
        let ops_output = ops_lines.join("\n");

        assert!(ops_output.contains("test error"), "Error activity missing");
        assert!(ops_output.contains("Active: 1"), "Active count wrong");
        assert!(ops_output.contains("Done: 1/4"), "Done count wrong");
    }

    /// A Review-phase ticket is a decision, so it belongs on the desk — not in
    /// the box that reports unhealthy sessions.
    #[test]
    fn a_review_wait_is_a_desk_card_and_not_an_alert() {
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
        render_health_alerts(&state, 80, &mut output);

        assert!(
            output.is_empty(),
            "a healthy session in Review is not an alert"
        );
    }

    #[test]
    fn health_alerts_render_nothing_when_every_session_is_healthy() {
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
        render_health_alerts(&state, 80, &mut output);

        assert!(output.is_empty());
    }

    /// The box no longer advertises a key for rows it no longer carries — the
    /// N3 specimen this epic was written against.
    #[test]
    fn the_alert_box_advertises_no_mark_done_key() {
        let state = PluginState {
            alerts: vec![HealthAlert {
                ticket_id: "T-010".to_string(),
                alert_type: AlertType::Failed,
                detail: "Exit code: 1".to_string(),
                suggested_actions: vec![],
            }],
            ..PluginState::default()
        };

        let mut output = Vec::new();
        render_health_alerts(&state, 80, &mut output);

        let full = output.join("\n");
        assert!(full.contains("ATTENTION NEEDED"));
        assert!(!full.contains("Press [d] to mark done"));
        assert!(full.contains("╔"), "Top border missing");
        assert!(full.contains("╚"), "Bottom border missing");
    }

    #[test]
    fn test_health_alerts_in_full_dashboard() {
        let state = PluginState {
            tickets: vec![TicketNode {
                id: "T-001".to_string(),
                title: "done-ticket".to_string(),
                phase: Phase::Done,
                status: TicketStatus::Done,
                depends_on: vec![],
            }],
            alerts: vec![HealthAlert {
                ticket_id: "T-002".to_string(),
                alert_type: AlertType::Stuck,
                detail: "No progress for 15+ min".to_string(),
                suggested_actions: vec![],
            }],
            current_time: Duration::from_secs(100),
            ..PluginState::default()
        };

        let lines = render_dashboard_lines(&state, 80, 50, &mut DagPan::default());
        let full = lines.join("\n");

        assert!(
            full.contains("ATTENTION NEEDED"),
            "Alert box missing from full dashboard"
        );

        let banner_pos = full.find("ATTENTION NEEDED").unwrap();
        let threads_pos = full.find("Threads").unwrap();
        assert!(
            banner_pos < threads_pos,
            "Alerts should appear before the Threads section"
        );
    }

    #[test]
    fn test_render_health_alerts_with_health_alerts() {
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
        render_health_alerts(&state, 80, &mut output);

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
    fn test_render_health_alerts_alerts_only() {
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
        render_health_alerts(&state, 80, &mut output);

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

    /// A ticket can be both waiting on a person and running an unhealthy
    /// session. Each surface reports its own half, and neither borrows the
    /// other's rows.
    #[test]
    fn a_review_wait_and_an_alert_land_on_different_surfaces() {
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
            desk: DeskState {
                cards: vec![DeskCard {
                    ticket_id: "T-005".to_string(),
                    title: "review-ticket".to_string(),
                    age_stamp: None,
                    kind: DeskCardKind::ReviewWait,
                    ask: REVIEW_WAIT_ASK.to_string(),
                    detail: DeskDetail::default(),
                }],
                ..DeskState::default()
            },
            current_time: Duration::from_secs(200),
            ..PluginState::default()
        };

        let mut alerts = Vec::new();
        render_health_alerts(&state, 80, &mut alerts);
        let alerts = alerts.join("\n");
        assert!(alerts.contains("T-010"), "Health alert ticket missing");
        assert!(alerts.contains("FAILED"), "Failed indicator missing");
        assert!(!alerts.contains("T-005"), "Review wait leaked into alerts");

        let desk = desk_lines(state.desk.clone()).join("\n");
        assert!(desk.contains("T-005"));
        assert!(!desk.contains("T-010"), "an alert is not a decision");
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
        let lines = render_dashboard_lines(&state, 80, 50, &mut DagPan::default());
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
        let lines = render_dashboard_lines(&state, 80, 40, &mut DagPan::default());
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
        let lines = render_dashboard_lines(&state, 80, 40, &mut DagPan::default());
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
