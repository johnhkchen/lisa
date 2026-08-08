//! Core data structures for the Lisa Zellij plugin.
//!
//! This module defines the fundamental types used throughout the plugin:
//! - Ticket representation with frontmatter fields
//! - Phase workflow states
//! - Thread lifecycle management
//! - Plugin configuration
//! - Activity event logging

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use crate::client::AgentClient;
use crate::completion::CompletionSeal;
use std::fmt;
use std::path::PathBuf;
use std::time::SystemTime;

/// Type alias for ticket identifiers.
pub type TicketId = String;

/// Identifies one execution attempt for a ticket.
///
/// Attempt IDs are positive and strictly increase per ticket when leases are
/// created through [`AttemptLease::mint`]. The complete `(ticket_id,
/// attempt_id)` pair is the authority boundary: equal numeric attempt IDs for
/// different tickets are unrelated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttemptLease {
    /// The ticket this execution attempt may act for.
    pub ticket_id: TicketId,
    /// The positive, per-ticket generation of this execution attempt.
    pub attempt_id: u64,
}

/// Why a successor [`AttemptLease`] could not be minted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptLeaseMintError {
    /// The supplied predecessor belongs to a different ticket.
    TicketMismatch {
        /// The ticket requested for the new lease.
        ticket_id: TicketId,
        /// The ticket carried by the supplied predecessor.
        previous_ticket_id: TicketId,
    },
    /// The previous attempt already used the largest representable ID.
    AttemptIdExhausted {
        /// The ticket whose attempt sequence is exhausted.
        ticket_id: TicketId,
    },
}

impl fmt::Display for AttemptLeaseMintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TicketMismatch {
                ticket_id,
                previous_ticket_id,
            } => write!(
                f,
                "cannot mint lease for {ticket_id} from predecessor for {previous_ticket_id}"
            ),
            Self::AttemptIdExhausted { ticket_id } => {
                write!(f, "attempt IDs are exhausted for {ticket_id}")
            }
        }
    }
}

impl std::error::Error for AttemptLeaseMintError {}

impl AttemptLease {
    /// Mint the first lease for a ticket or the checked successor to its prior
    /// lease.
    ///
    /// The first attempt is `1`. A predecessor from another ticket and a
    /// predecessor at `u64::MAX` both fail closed so every successful successor
    /// remains strictly greater than its predecessor.
    pub fn mint(
        ticket_id: impl Into<TicketId>,
        previous: Option<&Self>,
    ) -> Result<Self, AttemptLeaseMintError> {
        let ticket_id = ticket_id.into();
        let attempt_id = match previous {
            None => 1,
            Some(previous) if previous.ticket_id != ticket_id => {
                return Err(AttemptLeaseMintError::TicketMismatch {
                    ticket_id,
                    previous_ticket_id: previous.ticket_id.clone(),
                });
            }
            Some(previous) => previous.attempt_id.checked_add(1).ok_or_else(|| {
                AttemptLeaseMintError::AttemptIdExhausted {
                    ticket_id: ticket_id.clone(),
                }
            })?,
        };

        Ok(Self {
            ticket_id,
            attempt_id,
        })
    }

    /// Return whether this lease exactly matches the authoritative current
    /// lease.
    ///
    /// An absent current lease represents an unleased or revoked ticket and
    /// rejects every candidate.
    pub fn is_current(&self, current: Option<&Self>) -> bool {
        current == Some(self)
    }
}

/// The phase of work a ticket is currently in.
///
/// Phases follow the RDSPIR workflow: Research -> Design -> Structure -> Plan -> Implement -> Review.
/// Ready is the initial state and Done is terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// Ticket is ready to be picked up but work has not started
    #[default]
    Ready,
    /// Descriptive mapping of codebase - what exists, where, how it connects
    Research,
    /// Options explored, tradeoffs evaluated, decision with rationale
    Design,
    /// File-level changes, architecture, component boundaries, ordering
    Structure,
    /// Sequenced implementation steps, testing strategy, verification criteria
    Plan,
    /// Active implementation work
    Implement,
    /// Agent self-review: produces review.md artifact
    Review,
    /// Work completed
    Done,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Phase::Ready => write!(f, "ready"),
            Phase::Research => write!(f, "research"),
            Phase::Design => write!(f, "design"),
            Phase::Structure => write!(f, "structure"),
            Phase::Plan => write!(f, "plan"),
            Phase::Implement => write!(f, "implement"),
            Phase::Review => write!(f, "review"),
            Phase::Done => write!(f, "done"),
        }
    }
}

impl Phase {
    /// Returns the next phase in the workflow, or None if already Done.
    pub fn next(&self) -> Option<Phase> {
        match self {
            Phase::Ready => Some(Phase::Research),
            Phase::Research => Some(Phase::Design),
            Phase::Design => Some(Phase::Structure),
            Phase::Structure => Some(Phase::Plan),
            Phase::Plan => Some(Phase::Implement),
            Phase::Implement => Some(Phase::Review),
            Phase::Review => Some(Phase::Done),
            Phase::Done => None,
        }
    }

    /// Returns the artifact filename for this phase, if any.
    pub fn artifact_filename(&self) -> Option<&'static str> {
        match self {
            Phase::Research => Some("research.md"),
            Phase::Design => Some("design.md"),
            Phase::Structure => Some("structure.md"),
            Phase::Plan => Some("plan.md"),
            Phase::Implement => Some("progress.md"),
            Phase::Review => Some("review.md"),
            Phase::Ready | Phase::Done => None,
        }
    }

    /// Returns all phases in workflow order.
    pub fn all() -> &'static [Phase] {
        &[
            Phase::Ready,
            Phase::Research,
            Phase::Design,
            Phase::Structure,
            Phase::Plan,
            Phase::Implement,
            Phase::Review,
            Phase::Done,
        ]
    }

    /// Returns true if this phase indicates the ticket can be scheduled.
    ///
    /// Ready through Review are schedulable. Done is terminal.
    pub fn is_startable(&self) -> bool {
        !matches!(self, Phase::Done)
    }

    /// Returns true if this phase indicates active work is happening.
    ///
    /// Research, Design, Structure, Plan, Implement, and Review are active phases.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Phase::Research
                | Phase::Design
                | Phase::Structure
                | Phase::Plan
                | Phase::Implement
                | Phase::Review
        )
    }

    /// Returns true if this phase is complete.
    pub fn is_complete(&self) -> bool {
        matches!(self, Phase::Done)
    }

    /// Parse a phase from its lowercase string name.
    pub fn from_name(s: &str) -> Option<Phase> {
        match s {
            "ready" => Some(Phase::Ready),
            "research" => Some(Phase::Research),
            "design" => Some(Phase::Design),
            "structure" => Some(Phase::Structure),
            "plan" => Some(Phase::Plan),
            "implement" => Some(Phase::Implement),
            "review" => Some(Phase::Review),
            "done" => Some(Phase::Done),
            _ => None,
        }
    }
}

/// The type of ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TicketType {
    /// A standard work item
    #[default]
    Task,
    /// A defect to be fixed
    Bug,
    /// A feature enhancement
    Feature,
    /// Technical debt or refactoring
    Chore,
    /// Exploratory investigation
    Spike,
}

/// The status of a ticket in the workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TicketStatus {
    /// Ticket is open and available for work
    #[default]
    Open,
    /// Ticket is currently being worked on
    #[serde(rename = "in_progress")]
    InProgress,
    /// Ticket is blocked by dependencies or issues
    Blocked,
    /// Ticket is awaiting review
    Review,
    /// Ticket work is complete
    Done,
    /// Ticket has been cancelled
    Cancelled,
}

impl fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TicketStatus::Open => write!(f, "open"),
            TicketStatus::InProgress => write!(f, "in_progress"),
            TicketStatus::Blocked => write!(f, "blocked"),
            TicketStatus::Review => write!(f, "review"),
            TicketStatus::Done => write!(f, "done"),
            TicketStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Type alias for backward compatibility with code using `Status`.
pub type Status = TicketStatus;

/// Priority level for a ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// Critical priority - work on immediately
    Critical,
    /// High priority
    High,
    /// Medium priority (default)
    #[default]
    Medium,
    /// Low priority
    Low,
}

/// A ticket representing a unit of work.
///
/// Tickets are parsed from markdown files with YAML frontmatter.
/// The frontmatter contains structured metadata, and the body
/// contains the ticket description, context, and acceptance criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ticket {
    /// Unique ticket identifier (e.g., "T-024-03")
    pub id: String,

    /// Optional parent story identifier (e.g., "S-024")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story: Option<String>,

    /// Short descriptive title
    pub title: String,

    /// The type of work this ticket represents
    #[serde(rename = "type", default)]
    pub ticket_type: TicketType,

    /// Current status in the workflow
    #[serde(default)]
    pub status: TicketStatus,

    /// Priority level
    #[serde(default)]
    pub priority: Priority,

    /// Current phase in the RDSPI workflow
    #[serde(default)]
    pub phase: Phase,

    /// List of ticket IDs this ticket depends on
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,

    /// List of ticket IDs that this ticket blocks
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<String>,

    /// Optional routing hint: which agent client should run this ticket
    /// (`claude` | `codex`). Stored **raw and unvalidated** — an invalid value
    /// must fall back to the loop default at spawn (never fail the ticket), and
    /// the requested value is preserved so the substitution is visible in the
    /// dashboard and the provenance ledger. Resolution lives in
    /// [`crate::route::resolve_route`]. Absent → the loop default (S-026 /
    /// T-026-01).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// Optional routing hint: the model to run within the selected agent's
    /// provider (opaque, provider-defined vocabulary). `None` → the provider's
    /// own default model (today's behaviour). The adapter owns the model→flag
    /// mapping; the resolver passes this through untouched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Path to the ticket file on disk
    #[serde(skip)]
    pub file_path: PathBuf,

    /// The markdown content after the frontmatter
    #[serde(skip)]
    pub content: String,
}

impl Ticket {
    /// Creates a new ticket with the given id and title.
    /// Other fields are set to defaults.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            story: None,
            title: title.into(),
            ticket_type: TicketType::default(),
            status: TicketStatus::default(),
            priority: Priority::default(),
            phase: Phase::default(),
            depends_on: Vec::new(),
            blocks: Vec::new(),
            agent: None,
            model: None,
            file_path: PathBuf::new(),
            content: String::new(),
        }
    }

    /// Returns true if this ticket can be started (no unresolved dependencies).
    pub fn is_ready(&self) -> bool {
        self.status == TicketStatus::Open && self.phase == Phase::Ready
    }

    /// Returns the work directory path for this ticket's artifacts.
    /// Default location: docs/active/work/{ticket_id}/
    pub fn work_dir(&self, base_path: &std::path::Path) -> PathBuf {
        base_path.join("docs/active/work").join(&self.id)
    }
}

/// The status of a thread (Claude Code session).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThreadStatus {
    /// Thread is actively running
    #[default]
    Running,
    /// Thread is parked awaiting human review
    Parked,
    /// Thread completed successfully
    Completed,
    /// Thread encountered an error
    Failed,
}

/// Health status of a thread, computed from time-in-phase and thread status.
///
/// This is a derived property, not persisted. Call `Thread::health()` to compute it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Thread is making progress (time-in-phase below threshold)
    Healthy,
    /// Thread has been in the same phase too long without producing an artifact
    Stuck,
    /// Thread exited with an error
    Failed,
}

/// A thread representing an active Claude Code session working on a ticket.
///
/// Each thread runs a single ticket through the RDSPI workflow phases.
/// Threads are managed by Lisa and can be paused at review points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thread {
    /// The ticket ID this thread is working on
    pub ticket_id: String,

    /// The Zellij pane ID where this session is running
    pub pane_id: u32,

    /// Provider-neutral authority for this ticket execution attempt. The
    /// scheduler stamps this after dispatch admission; `None` represents a
    /// legacy or manually constructed thread with no granted attempt lease.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_lease: Option<AttemptLease>,

    /// Current phase the thread is working on
    pub current_phase: Phase,

    /// When the thread was started
    #[serde(with = "system_time_serde")]
    pub started_at: SystemTime,

    /// When the thread last changed phase (used for time-in-phase displays)
    #[serde(with = "system_time_serde")]
    pub last_phase_change: SystemTime,

    /// When activity was last observed from the session (heartbeat signal,
    /// stop/idle/cleared signal, or phase change). Used for stuck detection:
    /// a session is only considered stuck when it has gone *silent*, not
    /// merely because a phase is taking a long time.
    #[serde(with = "system_time_serde")]
    pub last_activity: SystemTime,

    /// Current status of the thread
    #[serde(default)]
    pub status: ThreadStatus,

    /// The agent client this run resolved to at spawn. Snapshotted here (rather
    /// than re-read from config at teardown) so the provenance ledger records
    /// what *actually* ran — correct today and forward-compatible with per-pane
    /// routing (S-026), where the resolved client may differ per ticket.
    #[serde(default)]
    pub client: AgentClient,

    /// Count of threads already running when this run was spawned (concurrency
    /// at spawn). Captured for the provenance ledger (T-027-01).
    #[serde(default)]
    pub concurrency_at_spawn: usize,

    /// The full `(provider, model)` route this thread was spawned with
    /// (T-026-01), including the requested value and whether an invalid hint was
    /// substituted for the loop default. Set at spawn from
    /// [`crate::route::resolve_route`]; `None` for a freshly-constructed thread
    /// and for threads persisted before routing existed. The dashboard reads it
    /// to surface each pane's route; the provenance ledger reads it for
    /// requested vs. actual. `client` above stays the authoritative "which agent
    /// ran" and always equals `route.agent` when a route is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<crate::route::ResolvedRoute>,
}

impl Thread {
    /// Creates a new thread for the given ticket.
    pub fn new(ticket_id: impl Into<String>, pane_id: u32) -> Self {
        let now = SystemTime::now();
        Self {
            ticket_id: ticket_id.into(),
            pane_id,
            attempt_lease: None,
            current_phase: Phase::Ready,
            started_at: now,
            last_phase_change: now,
            last_activity: now,
            status: ThreadStatus::Running,
            client: AgentClient::default(),
            concurrency_at_spawn: 0,
            route: None,
        }
    }

    /// Record observed activity from the session (heartbeat, signal, or
    /// phase change). Resets the inactivity clock used by `health()`.
    pub fn record_activity(&mut self, now: SystemTime) {
        self.last_activity = now;
    }

    /// Record a phase change, which also counts as activity.
    pub fn mark_phase_change(&mut self, now: SystemTime) {
        self.last_phase_change = now;
        self.last_activity = now;
    }

    /// Returns true if the thread is actively running.
    pub fn is_active(&self) -> bool {
        self.status == ThreadStatus::Running
    }

    /// Returns true if the thread is parked awaiting review.
    pub fn is_parked(&self) -> bool {
        self.status == ThreadStatus::Parked
    }

    /// Parks the thread for human review.
    pub fn park(&mut self) {
        self.status = ThreadStatus::Parked;
    }

    /// Resumes a parked thread.
    pub fn resume(&mut self) {
        if self.status == ThreadStatus::Parked {
            self.status = ThreadStatus::Running;
        }
    }

    /// Marks the thread as completed.
    pub fn complete(&mut self) {
        self.status = ThreadStatus::Completed;
    }

    /// Marks the thread as failed.
    pub fn fail(&mut self) {
        self.status = ThreadStatus::Failed;
    }

    /// Marks the thread as exited with an optional exit code.
    ///
    /// If exit_code is Some(0), marks as Completed; otherwise marks as Failed.
    pub fn mark_exited(&mut self, exit_code: Option<i32>) {
        match exit_code {
            Some(0) => self.status = ThreadStatus::Completed,
            Some(_) => self.status = ThreadStatus::Failed,
            None => self.status = ThreadStatus::Failed, // Unknown exit = failure
        }
    }

    /// Compute the health status of this thread.
    ///
    /// - `Failed` if the thread status is Failed.
    /// - `Stuck` if the thread is Running and no activity (heartbeat, signal,
    ///   or phase change) has been observed for longer than the threshold.
    ///   A session actively making tool calls is never stuck, no matter how
    ///   long it stays in one phase.
    /// - `Healthy` otherwise (including Parked and Completed threads).
    pub fn health(&self, now: SystemTime, stuck_threshold: std::time::Duration) -> HealthStatus {
        if self.status == ThreadStatus::Failed {
            return HealthStatus::Failed;
        }
        if self.status != ThreadStatus::Running {
            return HealthStatus::Healthy;
        }
        let elapsed = now.duration_since(self.last_activity).unwrap_or_default();
        if elapsed >= stuck_threshold {
            HealthStatus::Stuck
        } else {
            HealthStatus::Healthy
        }
    }

    /// Returns true if this thread needs human attention.
    ///
    /// A thread needs attention if it is stuck, failed, or parked for review.
    pub fn is_attention_needed(
        &self,
        now: SystemTime,
        stuck_threshold: std::time::Duration,
    ) -> bool {
        matches!(
            self.health(now, stuck_threshold),
            HealthStatus::Stuck | HealthStatus::Failed
        ) || self.status == ThreadStatus::Parked
    }
}

/// Configuration for the Lisa plugin.
///
/// Parsed from the Zellij plugin configuration map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Absolute root of the enclosing Git repository, discovered by the native
    /// launcher and passed through the Zellij layout. Empty only for legacy
    /// layouts and directly constructed tests.
    #[serde(default)]
    pub git_root: PathBuf,

    /// Completion tier resolved and pinned by the native loop launcher.
    /// Defaults to commit for legacy layouts so an absent or malformed runtime
    /// value can never silently weaken today's completion gate.
    #[serde(default)]
    pub completion_seal: CompletionSeal,

    /// Directory containing ticket files (default: "docs/active/tickets")
    pub ticket_dir: PathBuf,

    /// Directory containing story files (default: "docs/active/stories")
    pub story_dir: PathBuf,

    /// Directory for work artifacts (default: "docs/active/work")
    pub work_dir: PathBuf,

    /// Maximum number of concurrent threads (default: 4)
    pub max_threads: usize,

    /// Seconds of total inactivity (no heartbeats, signals, or phase changes)
    /// before a thread is flagged Stuck (default: 1200 = 20 minutes). The hard
    /// reclaim bar is 2x this value, so the default tolerates a single silent
    /// 30-minute test or integration run with room to spare.
    pub stuck_threshold_secs: u64,

    /// Seconds a running Review thread waits before receiving a finish-up prompt (default: 600).
    /// Set to 0 to disable.
    pub review_timeout_secs: u64,

    /// Advisory wall-clock budget for a single agent session (default: 3600 =
    /// 1 hour). An over-budget session is flagged but only reclaimed after
    /// prolonged total silence (2x `stuck_threshold_secs`). Set to 0 to disable.
    pub session_timeout_secs: u64,

    /// Optional per-phase timeout overrides. When set, the value for a phase
    /// overrides `session_timeout_secs` for time-in-phase checks. Missing
    /// phases fall back to `session_timeout_secs`.
    pub phase_timeouts: HashMap<Phase, u64>,

    /// Seconds a pane must be signal-silent (no heartbeat, stop, idle, or
    /// cleared signal) before the scheduler may reuse it for a new ticket,
    /// and how long a released slot stays in cooldown (default: 300).
    ///
    /// Agents often report "stopped" and then keep working for another minute
    /// or two before sending a final message, so reuse is gated on sustained
    /// quiet rather than on any single signal.
    pub wind_down_secs: u64,

    /// Maximum seconds to wait after submitting a generation-tagged recycled
    /// or recovery Codex prompt for exact provider acknowledgment (default: 30).
    /// Always positive: pending assignment ownership may never wait forever.
    pub assignment_ack_timeout_secs: u64,

    /// The agent client this loop drives by default (default: Claude). The
    /// spawn-time adapter resolver reads this as the loop-level default; per-ticket
    /// routing (S-026) will later override it per pane.
    #[serde(default)]
    pub client: AgentClient,

    /// Absolute path to the `lisa` binary, captured at `lisa loop` time via
    /// `current_exe()` and passed through the layout. Native adapters export it as
    /// `LISA_BIN` so lifecycle hooks can invoke `lisa capture-usage` without a
    /// PATH assumption. `None` falls back to bare `lisa`. Not
    /// `/host`-prefixed: the pane shell runs on the host, outside the WASI mount,
    /// so this is the real host path.
    #[serde(default)]
    pub lisa_bin: Option<String>,

    /// Optional per-provider concurrency sub-caps, keyed by agent client
    /// (T-026-02). A provider with an entry may run at most that many concurrent
    /// threads, *within* the global `max_threads` ceiling — a per-provider cap
    /// never raises the global limit, it only carves it between providers'
    /// separate auth/rate-limit pools. Absent (empty map) → only the global cap
    /// applies, i.e. byte-for-byte the pre-routing behaviour. Enforced at
    /// spawn-time slot assignment in the plugin scheduler.
    #[serde(default)]
    pub provider_caps: HashMap<AgentClient, usize>,

    /// Whether parked operator blocks receive one bounded first-responder pass.
    #[serde(default = "PluginConfig::default_triage_enabled")]
    pub triage_enabled: bool,

    /// Hard wall-clock bound for one first-responder pass.
    #[serde(default = "PluginConfig::default_triage_timeout_secs")]
    pub triage_timeout_secs: u64,
}

impl PluginConfig {
    /// Default ticket directory path.
    pub const DEFAULT_TICKET_DIR: &'static str = "docs/active/tickets";

    /// Default story directory path.
    pub const DEFAULT_STORY_DIR: &'static str = "docs/active/stories";

    /// Default work directory path.
    pub const DEFAULT_WORK_DIR: &'static str = "docs/active/work";

    /// Default maximum concurrent threads.
    pub const DEFAULT_MAX_THREADS: usize = 2;

    /// Default stuck threshold in seconds (20 minutes of silence; hard
    /// reclaim at 2x = 40 minutes, tolerating 30-minute silent test runs).
    pub const DEFAULT_STUCK_THRESHOLD_SECS: u64 = 1200;

    /// Default review timeout in seconds (10 minutes).
    pub const DEFAULT_REVIEW_TIMEOUT_SECS: u64 = 600;

    /// Default session budget in seconds (1 hour, advisory).
    pub const DEFAULT_SESSION_TIMEOUT_SECS: u64 = 3600;

    /// Default wind-down period in seconds (5 minutes).
    pub const DEFAULT_WIND_DOWN_SECS: u64 = 300;

    /// Default recycled-Codex assignment acknowledgment timeout (30 seconds).
    pub const DEFAULT_ASSIGNMENT_ACK_TIMEOUT_SECS: u64 = 30;

    pub const DEFAULT_TRIAGE_ENABLED: bool = true;

    pub const DEFAULT_TRIAGE_TIMEOUT_SECS: u64 = 120;

    fn default_triage_enabled() -> bool {
        Self::DEFAULT_TRIAGE_ENABLED
    }

    fn default_triage_timeout_secs() -> u64 {
        Self::DEFAULT_TRIAGE_TIMEOUT_SECS
    }

    /// Creates a new PluginConfig with default values.
    pub fn new() -> Self {
        Self {
            git_root: PathBuf::new(),
            completion_seal: CompletionSeal::Commit,
            ticket_dir: PathBuf::from(Self::DEFAULT_TICKET_DIR),
            story_dir: PathBuf::from(Self::DEFAULT_STORY_DIR),
            work_dir: PathBuf::from(Self::DEFAULT_WORK_DIR),
            max_threads: Self::DEFAULT_MAX_THREADS,
            stuck_threshold_secs: Self::DEFAULT_STUCK_THRESHOLD_SECS,
            review_timeout_secs: Self::DEFAULT_REVIEW_TIMEOUT_SECS,
            session_timeout_secs: Self::DEFAULT_SESSION_TIMEOUT_SECS,
            phase_timeouts: HashMap::new(),
            wind_down_secs: Self::DEFAULT_WIND_DOWN_SECS,
            assignment_ack_timeout_secs: Self::DEFAULT_ASSIGNMENT_ACK_TIMEOUT_SECS,
            client: AgentClient::default(),
            lisa_bin: None,
            provider_caps: HashMap::new(),
            triage_enabled: Self::DEFAULT_TRIAGE_ENABLED,
            triage_timeout_secs: Self::DEFAULT_TRIAGE_TIMEOUT_SECS,
        }
    }

    /// Creates a PluginConfig from a Zellij configuration map.
    pub fn from_config_map(config: &BTreeMap<String, String>) -> Self {
        let mut result = Self::new();

        if let Some(git_root) = config.get("git_root") {
            result.git_root = PathBuf::from(git_root);
        }

        if let Some(completion_seal) = config.get("completion_seal") {
            if let Ok(seal) = CompletionSeal::parse(completion_seal) {
                result.completion_seal = seal;
            }
        }

        if let Some(ticket_dir) = config.get("ticket_dir") {
            result.ticket_dir = PathBuf::from(ticket_dir);
        }

        if let Some(story_dir) = config.get("story_dir") {
            result.story_dir = PathBuf::from(story_dir);
        }

        if let Some(work_dir) = config.get("work_dir") {
            result.work_dir = PathBuf::from(work_dir);
        }

        if let Some(max_threads) = config.get("max_threads") {
            if let Ok(n) = max_threads.parse() {
                result.max_threads = n;
            }
        }

        if let Some(stuck_threshold) = config.get("stuck_threshold_secs") {
            if let Ok(n) = stuck_threshold.parse() {
                result.stuck_threshold_secs = n;
            }
        }

        if let Some(review_timeout) = config.get("review_timeout_secs") {
            if let Ok(n) = review_timeout.parse() {
                result.review_timeout_secs = n;
            }
        }

        if let Some(session_timeout) = config.get("session_timeout_secs") {
            if let Ok(n) = session_timeout.parse() {
                result.session_timeout_secs = n;
            }
        }

        if let Some(wind_down) = config.get("wind_down_secs") {
            if let Ok(n) = wind_down.parse() {
                result.wind_down_secs = n;
            }
        }

        if let Some(ack_timeout) = config.get("assignment_ack_timeout_secs") {
            if let Ok(n) = ack_timeout.parse::<u64>() {
                if n > 0 {
                    result.assignment_ack_timeout_secs = n;
                }
            }
        }

        if let Some(enabled) = config.get("triage_enabled") {
            result.triage_enabled = enabled == "true" || enabled == "1";
        }
        if let Some(timeout) = config.get("triage_timeout_secs") {
            if let Ok(timeout) = timeout.parse::<u64>() {
                if timeout > 0 {
                    result.triage_timeout_secs = timeout;
                }
            }
        }

        if let Some(client) = config.get("client") {
            // Lenient: an unrecognized value keeps the default. CLI-side
            // validation (`lisa validate`) is the gate; the plugin must never
            // panic parsing its config map.
            if let Ok(c) = AgentClient::parse(client) {
                result.client = c;
            }
        }

        if let Some(lisa_bin) = config.get("lisa_bin") {
            // Empty string (layout emitted an unset value) is treated as absent so
            // the Codex adapter falls back to the bare `lisa`.
            if !lisa_bin.is_empty() {
                result.lisa_bin = Some(lisa_bin.clone());
            }
        }

        // Parse phase_timeout_{phase} keys
        let prefix = "phase_timeout_";
        for (key, val) in config {
            if let Some(phase_name) = key.strip_prefix(prefix) {
                if let (Some(phase), Ok(secs)) = (Phase::from_name(phase_name), val.parse::<u64>())
                {
                    result.phase_timeouts.insert(phase, secs);
                }
            }
        }

        // Parse provider_cap_{client} keys (T-026-02). Lenient like `client`
        // above: an unknown provider name, a non-numeric value, or a `0` cap is
        // skipped rather than erroring — the plugin must never panic parsing its
        // config map; `lisa validate` is the gate that surfaces bad caps.
        let cap_prefix = "provider_cap_";
        for (key, val) in config {
            if let Some(name) = key.strip_prefix(cap_prefix) {
                if let (Ok(client), Ok(cap)) = (AgentClient::parse(name), val.parse::<usize>()) {
                    if cap > 0 {
                        result.provider_caps.insert(client, cap);
                    }
                }
            }
        }

        result
    }

    /// Returns the effective timeout for a given phase.
    ///
    /// If a per-phase override exists, returns it. Otherwise falls back
    /// to `session_timeout_secs`.
    pub fn timeout_for_phase(&self, phase: Phase) -> u64 {
        self.phase_timeouts
            .get(&phase)
            .copied()
            .unwrap_or(self.session_timeout_secs)
    }

    /// Returns the per-provider concurrency cap for `client`, or `None` when no
    /// cap is configured for that provider (T-026-02). `None` means only the
    /// global `max_threads` ceiling applies to that provider.
    pub fn provider_cap_for(&self, client: AgentClient) -> Option<usize> {
        self.provider_caps.get(&client).copied()
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Stable operator-visible categories for a refused completion obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionRejectionKind {
    AlreadyPending,
    StaleLease,
    DispositionBlocked,
    DependencyBlocked,
    LaunchFailed,
}

impl CompletionRejectionKind {
    /// The sentence an operator reads first, before any technical detail.
    /// The `Display` token stays the stable machine identifier; this is the
    /// human line every operator surface must lead with.
    pub fn plain_line(self) -> &'static str {
        match self {
            Self::AlreadyPending => "Lisa is already finishing this ticket — give it a moment.",
            Self::StaleLease => {
                "An older session tried to finish this ticket; the current run is in charge. Try again."
            }
            Self::DispositionBlocked => {
                "This ticket is parked with a note that needs you. Read the note below, settle it, then finish the ticket."
            }
            Self::DependencyBlocked => "This ticket is waiting for earlier tickets to finish first.",
            Self::LaunchFailed => {
                "Lisa couldn't record the finish. Try again; if it keeps failing, the activity log has details."
            }
        }
    }
}

impl fmt::Display for CompletionRejectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AlreadyPending => "already-pending",
            Self::StaleLease => "stale-lease",
            Self::DispositionBlocked => "disposition-blocked",
            Self::DependencyBlocked => "dependency-blocked",
            Self::LaunchFailed => "launch-failed",
        })
    }
}

/// Activity events for the dashboard log.
///
/// These events are logged and displayed in the plugin's activity feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityEvent {
    /// Plugin started
    PluginStarted,

    /// A new thread was spawned for a ticket
    ThreadSpawned { ticket_id: TicketId, pane_id: u32 },

    /// A thread completed a phase
    PhaseCompleted { ticket_id: TicketId, phase: Phase },

    /// A thread exited (completed or failed)
    ThreadExited {
        ticket_id: TicketId,
        exit_code: Option<i32>,
    },

    /// A ticket's status changed
    TicketStatusChanged {
        ticket_id: TicketId,
        old_status: TicketStatus,
        new_status: TicketStatus,
    },

    /// A ticket's phase changed
    TicketPhaseChanged {
        ticket_id: TicketId,
        old_phase: Phase,
        new_phase: Phase,
    },

    /// An artifact was created
    ArtifactCreated {
        ticket_id: TicketId,
        phase: Phase,
        path: PathBuf,
    },

    /// A commit was made
    CommitMade {
        ticket_id: TicketId,
        commit_hash: String,
    },

    /// An error occurred
    Error { message: String },

    /// A completion obligation was refused with typed, correlated evidence.
    CompletionRejected {
        ticket_id: TicketId,
        kind: CompletionRejectionKind,
        correlation_id: String,
        detail: String,
    },

    /// DAG was recomputed
    DagRecomputed { ticket_count: usize },

    /// All tickets have reached done phase — loop complete
    AllTicketsDone,

    /// A thread's health status changed (e.g., Healthy → Stuck)
    HealthStateChanged {
        ticket_id: TicketId,
        old_health: HealthStatus,
        new_health: HealthStatus,
    },

    /// A warning (not an error, but something the operator should notice)
    Warning { message: String },

    /// Informational log message (not an error or warning)
    Info { message: String },

    /// Poll cycle summary (filtered from UI to avoid noise)
    PollSummary {
        ready: usize,
        running: usize,
        idle_slots: usize,
    },

    /// A session was launched for a ticket (structured capture of the command)
    SessionLaunch {
        ticket_id: TicketId,
        pane_id: u32,
        command: String,
    },

    /// A finish-up prompt was sent to a parked Review session after timeout
    FinishUpPromptSent { ticket_id: TicketId, pane_id: u32 },

    /// A session was timed out (exceeded session_timeout_secs)
    SessionTimedOut {
        ticket_id: TicketId,
        elapsed_secs: u64,
        phase: Phase,
    },
}

/// Serde helper module for SystemTime serialization.
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempt_lease_ids_are_strictly_monotonic_per_ticket() {
        let a1 = AttemptLease::mint("T-A", None).unwrap();
        let b1 = AttemptLease::mint("T-B", None).unwrap();
        let a2 = AttemptLease::mint("T-A", Some(&a1)).unwrap();
        let b2 = AttemptLease::mint("T-B", Some(&b1)).unwrap();
        let a3 = AttemptLease::mint("T-A", Some(&a2)).unwrap();
        let b3 = AttemptLease::mint("T-B", Some(&b2)).unwrap();

        assert_eq!([a1.attempt_id, a2.attempt_id, a3.attempt_id], [1, 2, 3]);
        assert_eq!([b1.attempt_id, b2.attempt_id, b3.attempt_id], [1, 2, 3]);
        assert_ne!(a1, b1, "ticket identity is part of the lease");
    }

    #[test]
    fn prior_attempt_lease_never_validates_as_current() {
        let prior = AttemptLease::mint("T-A", None).unwrap();
        assert!(prior.is_current(Some(&prior)));

        let current = AttemptLease::mint("T-A", Some(&prior)).unwrap();

        assert!(!prior.is_current(Some(&current)));
        assert!(current.is_current(Some(&current)));
        assert!(!prior.is_current(None));
    }

    #[test]
    fn attempt_lease_current_check_requires_the_same_ticket() {
        let a = AttemptLease::mint("T-A", None).unwrap();
        let b = AttemptLease::mint("T-B", None).unwrap();

        assert_eq!(a.attempt_id, b.attempt_id);
        assert!(!a.is_current(Some(&b)));
        assert!(!b.is_current(Some(&a)));
    }

    #[test]
    fn attempt_lease_mint_rejects_a_different_ticket_predecessor() {
        let previous = AttemptLease::mint("T-A", None).unwrap();

        assert_eq!(
            AttemptLease::mint("T-B", Some(&previous)),
            Err(AttemptLeaseMintError::TicketMismatch {
                ticket_id: "T-B".to_string(),
                previous_ticket_id: "T-A".to_string(),
            })
        );
    }

    #[test]
    fn attempt_lease_mint_rejects_attempt_id_exhaustion() {
        let previous = AttemptLease {
            ticket_id: "T-A".to_string(),
            attempt_id: u64::MAX,
        };

        assert_eq!(
            AttemptLease::mint("T-A", Some(&previous)),
            Err(AttemptLeaseMintError::AttemptIdExhausted {
                ticket_id: "T-A".to_string(),
            })
        );
    }

    #[test]
    fn test_phase_next() {
        assert_eq!(Phase::Ready.next(), Some(Phase::Research));
        assert_eq!(Phase::Research.next(), Some(Phase::Design));
        assert_eq!(Phase::Design.next(), Some(Phase::Structure));
        assert_eq!(Phase::Structure.next(), Some(Phase::Plan));
        assert_eq!(Phase::Plan.next(), Some(Phase::Implement));
        assert_eq!(Phase::Implement.next(), Some(Phase::Review));
        assert_eq!(Phase::Review.next(), Some(Phase::Done));
        assert_eq!(Phase::Done.next(), None);
    }

    #[test]
    fn test_phase_artifact_filename() {
        assert_eq!(Phase::Research.artifact_filename(), Some("research.md"));
        assert_eq!(Phase::Design.artifact_filename(), Some("design.md"));
        assert_eq!(Phase::Structure.artifact_filename(), Some("structure.md"));
        assert_eq!(Phase::Plan.artifact_filename(), Some("plan.md"));
        assert_eq!(Phase::Implement.artifact_filename(), Some("progress.md"));
        assert_eq!(Phase::Ready.artifact_filename(), None);
        assert_eq!(Phase::Review.artifact_filename(), Some("review.md"));
        assert_eq!(Phase::Done.artifact_filename(), None);
    }

    #[test]
    fn test_ticket_new() {
        let ticket = Ticket::new("T-001", "Test ticket");
        assert_eq!(ticket.id, "T-001");
        assert_eq!(ticket.title, "Test ticket");
        assert_eq!(ticket.status, TicketStatus::Open);
        assert_eq!(ticket.phase, Phase::Ready);
        assert!(ticket.is_ready());
    }

    #[test]
    fn test_thread_lifecycle() {
        let mut thread = Thread::new("T-001", 42);
        assert!(thread.is_active());
        assert!(!thread.is_parked());

        thread.park();
        assert!(!thread.is_active());
        assert!(thread.is_parked());

        thread.resume();
        assert!(thread.is_active());
        assert!(!thread.is_parked());

        thread.complete();
        assert_eq!(thread.status, ThreadStatus::Completed);
    }

    #[test]
    fn test_thread_run_meta_defaults() {
        let thread = Thread::new("T-001", 42);
        assert_eq!(thread.attempt_lease, None);
        assert_eq!(thread.client, AgentClient::default());
        assert_eq!(thread.concurrency_at_spawn, 0);
    }

    #[test]
    fn test_thread_deserializes_without_run_meta() {
        // Older serialized threads (state dumps) predate client/concurrency; the
        // #[serde(default)] fields must fill in rather than fail to parse.
        let json = r#"{
            "ticket_id": "T-001",
            "pane_id": 3,
            "current_phase": "research",
            "started_at": 1719800000,
            "last_phase_change": 1719800000,
            "last_activity": 1719800000,
            "status": "running"
        }"#;
        let thread: Thread = serde_json::from_str(json).unwrap();
        assert_eq!(thread.attempt_lease, None);
        assert_eq!(thread.client, AgentClient::default());
        assert_eq!(thread.concurrency_at_spawn, 0);
        assert_eq!(thread.ticket_id, "T-001");
    }

    #[test]
    fn test_thread_route_serde_back_compat() {
        use crate::route::ResolvedRoute;

        // A legacy thread JSON without `route` deserialises to None.
        let legacy = r#"{"ticket_id":"T-001","pane_id":1,"current_phase":"ready","started_at":0,"last_phase_change":0,"last_activity":0}"#;
        let t: Thread = serde_json::from_str(legacy).unwrap();
        assert_eq!(t.route, None);

        // A thread carrying a route round-trips.
        let mut t2 = Thread::new("T-002", 2);
        t2.route = Some(ResolvedRoute {
            agent: AgentClient::Codex,
            model: Some("gpt-5".to_string()),
            requested_agent: Some("codex".to_string()),
            substituted: false,
            note: None,
        });
        let json = serde_json::to_string(&t2).unwrap();
        let back: Thread = serde_json::from_str(&json).unwrap();
        assert_eq!(back.route, t2.route);
    }

    #[test]
    fn test_phase_serde() {
        let phase = Phase::Research;
        let json = serde_json::to_string(&phase).unwrap();
        assert_eq!(json, "\"research\"");

        let parsed: Phase = serde_json::from_str("\"design\"").unwrap();
        assert_eq!(parsed, Phase::Design);
    }

    #[test]
    fn test_ticket_status_serde() {
        let status = TicketStatus::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"in_progress\"");

        let parsed: TicketStatus = serde_json::from_str("\"in_progress\"").unwrap();
        assert_eq!(parsed, TicketStatus::InProgress);
    }

    #[test]
    fn test_last_phase_change_initialized() {
        let before = SystemTime::now();
        let thread = Thread::new("T-001", 1);
        let after = SystemTime::now();

        // last_phase_change should be between before and after
        assert!(thread.last_phase_change >= before);
        assert!(thread.last_phase_change <= after);
        // And equal to started_at (both set from same now())
        assert_eq!(thread.started_at, thread.last_phase_change);
    }

    #[test]
    fn test_health_healthy_fresh_thread() {
        let thread = Thread::new("T-001", 1);
        let now = SystemTime::now();
        let threshold = std::time::Duration::from_secs(600);

        assert_eq!(thread.health(now, threshold), HealthStatus::Healthy);
    }

    #[test]
    fn test_health_stuck_after_threshold() {
        let mut thread = Thread::new("T-001", 1);
        // Simulate total silence for 11+ minutes
        thread.last_phase_change = SystemTime::now() - std::time::Duration::from_secs(700);
        thread.last_activity = SystemTime::now() - std::time::Duration::from_secs(700);
        let now = SystemTime::now();
        let threshold = std::time::Duration::from_secs(600);

        assert_eq!(thread.health(now, threshold), HealthStatus::Stuck);
    }

    #[test]
    fn test_health_active_long_phase_not_stuck() {
        let mut thread = Thread::new("T-001", 1);
        // Phase started long ago, but the session is still making tool calls
        thread.last_phase_change = SystemTime::now() - std::time::Duration::from_secs(3600);
        thread.record_activity(SystemTime::now());
        let now = SystemTime::now();
        let threshold = std::time::Duration::from_secs(600);

        assert_eq!(thread.health(now, threshold), HealthStatus::Healthy);
    }

    #[test]
    fn test_mark_phase_change_updates_both_clocks() {
        let mut thread = Thread::new("T-001", 1);
        let past = SystemTime::now() - std::time::Duration::from_secs(700);
        thread.last_phase_change = past;
        thread.last_activity = past;

        let now = SystemTime::now();
        thread.mark_phase_change(now);
        assert_eq!(thread.last_phase_change, now);
        assert_eq!(thread.last_activity, now);
    }

    #[test]
    fn test_health_failed_thread() {
        let mut thread = Thread::new("T-001", 1);
        thread.fail();
        let now = SystemTime::now();
        let threshold = std::time::Duration::from_secs(600);

        assert_eq!(thread.health(now, threshold), HealthStatus::Failed);
    }

    #[test]
    fn test_health_parked_not_stuck() {
        let mut thread = Thread::new("T-001", 1);
        thread.last_phase_change = SystemTime::now() - std::time::Duration::from_secs(700);
        thread.park();
        let now = SystemTime::now();
        let threshold = std::time::Duration::from_secs(600);

        // Parked threads are Healthy (not Running, so not Stuck)
        assert_eq!(thread.health(now, threshold), HealthStatus::Healthy);
    }

    #[test]
    fn test_health_completed_not_stuck() {
        let mut thread = Thread::new("T-001", 1);
        thread.last_phase_change = SystemTime::now() - std::time::Duration::from_secs(700);
        thread.complete();
        let now = SystemTime::now();
        let threshold = std::time::Duration::from_secs(600);

        assert_eq!(thread.health(now, threshold), HealthStatus::Healthy);
    }

    #[test]
    fn test_is_attention_needed_stuck() {
        let mut thread = Thread::new("T-001", 1);
        thread.last_phase_change = SystemTime::now() - std::time::Duration::from_secs(700);
        thread.last_activity = SystemTime::now() - std::time::Duration::from_secs(700);
        let now = SystemTime::now();
        let threshold = std::time::Duration::from_secs(600);

        assert!(thread.is_attention_needed(now, threshold));
    }

    #[test]
    fn test_is_attention_needed_failed() {
        let mut thread = Thread::new("T-001", 1);
        thread.fail();
        let now = SystemTime::now();
        let threshold = std::time::Duration::from_secs(600);

        assert!(thread.is_attention_needed(now, threshold));
    }

    #[test]
    fn test_is_attention_needed_parked() {
        let mut thread = Thread::new("T-001", 1);
        thread.park();
        let now = SystemTime::now();
        let threshold = std::time::Duration::from_secs(600);

        assert!(thread.is_attention_needed(now, threshold));
    }

    #[test]
    fn test_is_attention_needed_healthy() {
        let thread = Thread::new("T-001", 1);
        let now = SystemTime::now();
        let threshold = std::time::Duration::from_secs(600);

        assert!(!thread.is_attention_needed(now, threshold));
    }

    #[test]
    fn test_health_status_serde() {
        let stuck = HealthStatus::Stuck;
        let json = serde_json::to_string(&stuck).unwrap();
        assert_eq!(json, "\"stuck\"");

        let parsed: HealthStatus = serde_json::from_str("\"failed\"").unwrap();
        assert_eq!(parsed, HealthStatus::Failed);

        let healthy: HealthStatus = serde_json::from_str("\"healthy\"").unwrap();
        assert_eq!(healthy, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_state_changed_event() {
        let event = ActivityEvent::HealthStateChanged {
            ticket_id: "T-001".to_string(),
            old_health: HealthStatus::Healthy,
            new_health: HealthStatus::Stuck,
        };
        match &event {
            ActivityEvent::HealthStateChanged {
                ticket_id,
                old_health,
                new_health,
            } => {
                assert_eq!(ticket_id, "T-001");
                assert_eq!(*old_health, HealthStatus::Healthy);
                assert_eq!(*new_health, HealthStatus::Stuck);
            }
            _ => panic!("Expected HealthStateChanged"),
        }
    }

    #[test]
    fn completion_rejection_kind_has_stable_operator_labels() {
        let cases = [
            (CompletionRejectionKind::AlreadyPending, "already-pending"),
            (CompletionRejectionKind::StaleLease, "stale-lease"),
            (
                CompletionRejectionKind::DispositionBlocked,
                "disposition-blocked",
            ),
            (
                CompletionRejectionKind::DependencyBlocked,
                "dependency-blocked",
            ),
            (CompletionRejectionKind::LaunchFailed, "launch-failed"),
        ];

        for (kind, expected) in cases {
            assert_eq!(kind.to_string(), expected);
        }
    }

    #[test]
    fn test_phase_display() {
        assert_eq!(Phase::Ready.to_string(), "ready");
        assert_eq!(Phase::Research.to_string(), "research");
        assert_eq!(Phase::Design.to_string(), "design");
        assert_eq!(Phase::Structure.to_string(), "structure");
        assert_eq!(Phase::Plan.to_string(), "plan");
        assert_eq!(Phase::Implement.to_string(), "implement");
        assert_eq!(Phase::Review.to_string(), "review");
        assert_eq!(Phase::Done.to_string(), "done");
    }

    #[test]
    fn test_ticket_status_display() {
        assert_eq!(TicketStatus::Open.to_string(), "open");
        assert_eq!(TicketStatus::InProgress.to_string(), "in_progress");
        assert_eq!(TicketStatus::Blocked.to_string(), "blocked");
        assert_eq!(TicketStatus::Review.to_string(), "review");
        assert_eq!(TicketStatus::Done.to_string(), "done");
        assert_eq!(TicketStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_config_stuck_threshold_default() {
        let config = PluginConfig::new();
        assert_eq!(config.stuck_threshold_secs, 1200);
    }

    #[test]
    fn test_config_stuck_threshold_from_map() {
        let mut map = BTreeMap::new();
        map.insert("stuck_threshold_secs".to_string(), "300".to_string());
        let config = PluginConfig::from_config_map(&map);
        assert_eq!(config.stuck_threshold_secs, 300);
    }

    #[test]
    fn test_config_session_timeout_default() {
        let config = PluginConfig::new();
        assert_eq!(config.session_timeout_secs, 3600);
    }

    #[test]
    fn test_config_wind_down_default() {
        let config = PluginConfig::new();
        assert_eq!(config.wind_down_secs, 300);
    }

    #[test]
    fn test_config_wind_down_from_map() {
        let mut map = BTreeMap::new();
        map.insert("wind_down_secs".to_string(), "300".to_string());
        let config = PluginConfig::from_config_map(&map);
        assert_eq!(config.wind_down_secs, 300);
    }

    #[test]
    fn test_config_assignment_ack_timeout_default() {
        let config = PluginConfig::new();
        assert_eq!(config.assignment_ack_timeout_secs, 30);
    }

    #[test]
    fn test_config_assignment_ack_timeout_from_map() {
        let mut map = BTreeMap::new();
        map.insert("assignment_ack_timeout_secs".to_string(), "7".to_string());
        let config = PluginConfig::from_config_map(&map);
        assert_eq!(config.assignment_ack_timeout_secs, 7);
    }

    #[test]
    fn test_config_assignment_ack_timeout_stays_finite_for_bad_map_values() {
        for value in ["0", "not-a-number"] {
            let mut map = BTreeMap::new();
            map.insert("assignment_ack_timeout_secs".to_string(), value.to_string());
            let config = PluginConfig::from_config_map(&map);
            assert_eq!(
                config.assignment_ack_timeout_secs,
                PluginConfig::DEFAULT_ASSIGNMENT_ACK_TIMEOUT_SECS
            );
        }
    }

    #[test]
    fn test_config_session_timeout_from_map() {
        let mut map = BTreeMap::new();
        map.insert("session_timeout_secs".to_string(), "3600".to_string());
        let config = PluginConfig::from_config_map(&map);
        assert_eq!(config.session_timeout_secs, 3600);
    }

    #[test]
    fn test_config_phase_timeouts_default() {
        let config = PluginConfig::new();
        assert!(config.phase_timeouts.is_empty());
    }

    #[test]
    fn test_config_phase_timeouts_from_map() {
        let mut map = BTreeMap::new();
        map.insert("phase_timeout_research".to_string(), "300".to_string());
        map.insert("phase_timeout_implement".to_string(), "1800".to_string());
        let config = PluginConfig::from_config_map(&map);
        assert_eq!(config.phase_timeouts.get(&Phase::Research), Some(&300));
        assert_eq!(config.phase_timeouts.get(&Phase::Implement), Some(&1800));
        assert_eq!(config.phase_timeouts.len(), 2);
    }

    #[test]
    fn test_config_client_default() {
        let config = PluginConfig::new();
        assert_eq!(config.client, AgentClient::Claude);
    }

    #[test]
    fn test_config_client_from_map() {
        let mut map = BTreeMap::new();
        map.insert("client".to_string(), "codex".to_string());
        let config = PluginConfig::from_config_map(&map);
        assert_eq!(config.client, AgentClient::Codex);
    }

    #[test]
    fn test_config_client_bad_value_defaults_claude() {
        let mut map = BTreeMap::new();
        map.insert("client".to_string(), "bogus".to_string());
        let config = PluginConfig::from_config_map(&map);
        assert_eq!(config.client, AgentClient::Claude);
    }

    #[test]
    fn test_config_lisa_bin_round_trip() {
        assert_eq!(PluginConfig::new().lisa_bin, None);

        let mut map = BTreeMap::new();
        map.insert("lisa_bin".to_string(), "/opt/bin/lisa".to_string());
        assert_eq!(
            PluginConfig::from_config_map(&map).lisa_bin.as_deref(),
            Some("/opt/bin/lisa")
        );

        // Empty string is treated as absent (layout emitted no path).
        let mut empty = BTreeMap::new();
        empty.insert("lisa_bin".to_string(), String::new());
        assert_eq!(PluginConfig::from_config_map(&empty).lisa_bin, None);
    }

    #[test]
    fn test_config_git_root_round_trip() {
        assert!(PluginConfig::new().git_root.as_os_str().is_empty());

        let mut map = BTreeMap::new();
        map.insert("git_root".to_string(), "/repo".to_string());
        assert_eq!(
            PluginConfig::from_config_map(&map).git_root,
            PathBuf::from("/repo")
        );
    }

    #[test]
    fn test_config_completion_seal_is_pinned_or_fails_closed_to_commit() {
        assert_eq!(PluginConfig::new().completion_seal, CompletionSeal::Commit);

        for (raw, expected) in [
            ("commit", CompletionSeal::Commit),
            ("journal", CompletionSeal::Journal),
            ("auto", CompletionSeal::Commit),
            ("bogus", CompletionSeal::Commit),
        ] {
            let mut map = BTreeMap::new();
            map.insert("completion_seal".to_string(), raw.to_string());
            assert_eq!(
                PluginConfig::from_config_map(&map).completion_seal,
                expected,
                "runtime value {raw:?}"
            );
        }
    }

    #[test]
    fn test_provider_caps_default_empty() {
        let config = PluginConfig::new();
        assert!(config.provider_caps.is_empty());
        assert_eq!(config.provider_cap_for(AgentClient::Claude), None);
        assert_eq!(config.provider_cap_for(AgentClient::Codex), None);
    }

    #[test]
    fn test_provider_cap_for_present() {
        let mut config = PluginConfig::new();
        config.provider_caps.insert(AgentClient::Codex, 8);
        assert_eq!(config.provider_cap_for(AgentClient::Codex), Some(8));
        // A provider without an entry still has no cap.
        assert_eq!(config.provider_cap_for(AgentClient::Claude), None);
    }

    #[test]
    fn test_provider_caps_from_map() {
        let mut map = BTreeMap::new();
        map.insert("provider_cap_codex".to_string(), "8".to_string());
        map.insert("provider_cap_claude".to_string(), "4".to_string());
        let config = PluginConfig::from_config_map(&map);
        assert_eq!(config.provider_cap_for(AgentClient::Codex), Some(8));
        assert_eq!(config.provider_cap_for(AgentClient::Claude), Some(4));
        assert_eq!(config.provider_caps.len(), 2);
    }

    #[test]
    fn test_provider_caps_from_map_ignores_bad_entries() {
        let mut map = BTreeMap::new();
        // Unknown provider name → skipped.
        map.insert("provider_cap_gpt".to_string(), "8".to_string());
        // Non-numeric value → skipped.
        map.insert("provider_cap_claude".to_string(), "lots".to_string());
        // Zero → skipped (a 0 cap would starve the provider forever).
        map.insert("provider_cap_codex".to_string(), "0".to_string());
        let config = PluginConfig::from_config_map(&map);
        assert!(config.provider_caps.is_empty());
    }

    #[test]
    fn test_provider_caps_absent_from_map_is_empty() {
        // A legacy config map without the keys yields an empty map.
        let mut map = BTreeMap::new();
        map.insert("max_threads".to_string(), "4".to_string());
        let config = PluginConfig::from_config_map(&map);
        assert!(config.provider_caps.is_empty());
    }

    #[test]
    fn test_timeout_for_phase_with_override() {
        let mut config = PluginConfig::new();
        config.phase_timeouts.insert(Phase::Implement, 1800);
        assert_eq!(config.timeout_for_phase(Phase::Implement), 1800);
    }

    #[test]
    fn test_timeout_for_phase_fallback() {
        let config = PluginConfig::new();
        // No override → falls back to session_timeout_secs (3600)
        assert_eq!(config.timeout_for_phase(Phase::Research), 3600);
    }

    #[test]
    fn test_timeout_for_phase_disabled() {
        let mut config = PluginConfig::new();
        config.session_timeout_secs = 0;
        // No override and session timeout disabled → 0
        assert_eq!(config.timeout_for_phase(Phase::Research), 0);
        // But explicit override still works
        config.phase_timeouts.insert(Phase::Research, 300);
        assert_eq!(config.timeout_for_phase(Phase::Research), 300);
    }

    #[test]
    fn triage_config_defaults_and_runtime_map_are_bounded() {
        let defaults = PluginConfig::new();
        assert!(defaults.triage_enabled);
        assert_eq!(defaults.triage_timeout_secs, 120);

        let mut map = BTreeMap::new();
        map.insert("triage_enabled".to_string(), "false".to_string());
        map.insert("triage_timeout_secs".to_string(), "7".to_string());
        let configured = PluginConfig::from_config_map(&map);
        assert!(!configured.triage_enabled);
        assert_eq!(configured.triage_timeout_secs, 7);

        map.insert("triage_timeout_secs".to_string(), "0".to_string());
        assert_eq!(
            PluginConfig::from_config_map(&map).triage_timeout_secs,
            PluginConfig::DEFAULT_TRIAGE_TIMEOUT_SECS
        );
    }

    #[test]
    fn test_phase_from_name() {
        assert_eq!(Phase::from_name("research"), Some(Phase::Research));
        assert_eq!(Phase::from_name("implement"), Some(Phase::Implement));
        assert_eq!(Phase::from_name("done"), Some(Phase::Done));
        assert_eq!(Phase::from_name("invalid"), None);
        assert_eq!(Phase::from_name("Research"), None); // case-sensitive
    }
}
