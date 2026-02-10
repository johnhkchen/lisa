//! Core data structures for the Lisa/Ralph Zellij plugin.
//!
//! This module defines the fundamental types used throughout the plugin:
//! - Ticket representation with frontmatter fields
//! - Phase workflow states
//! - Thread lifecycle management
//! - Plugin configuration
//! - Activity event logging

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::SystemTime;

/// Type alias for ticket identifiers.
pub type TicketId = String;

/// The phase of work a ticket is currently in.
///
/// Phases follow the RDSPI workflow: Research -> Design -> Structure -> Plan -> Implement.
/// Ready is the initial state, Review is for human review, and Done is terminal.
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
    /// Awaiting human review
    Review,
    /// Work completed
    Done,
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
            _ => None,
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

    /// Returns true if this phase indicates the ticket can be started.
    ///
    /// Only "ready" phase tickets can be started.
    pub fn is_startable(&self) -> bool {
        matches!(self, Phase::Ready)
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

/// A thread representing an active Claude Code session working on a ticket.
///
/// Each thread runs a single ticket through the RDSPI workflow phases.
/// Threads are managed by Ralph and can be paused at review points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thread {
    /// The ticket ID this thread is working on
    pub ticket_id: String,

    /// The Zellij pane ID where this session is running
    pub pane_id: u32,

    /// Current phase the thread is working on
    pub current_phase: Phase,

    /// When the thread was started
    #[serde(with = "system_time_serde")]
    pub started_at: SystemTime,

    /// Current status of the thread
    #[serde(default)]
    pub status: ThreadStatus,
}

impl Thread {
    /// Creates a new thread for the given ticket.
    pub fn new(ticket_id: impl Into<String>, pane_id: u32) -> Self {
        Self {
            ticket_id: ticket_id.into(),
            pane_id,
            current_phase: Phase::Ready,
            started_at: SystemTime::now(),
            status: ThreadStatus::Running,
        }
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
}

/// Configuration for the Lisa/Ralph plugin.
///
/// Parsed from the Zellij plugin configuration map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    /// Directory containing ticket files (default: "docs/active/tickets")
    pub ticket_dir: PathBuf,

    /// Directory containing story files (default: "docs/active/stories")
    pub story_dir: PathBuf,

    /// Directory for work artifacts (default: "docs/active/work")
    pub work_dir: PathBuf,

    /// Maximum number of concurrent threads (default: 4)
    pub max_threads: usize,

    /// Whether to auto-advance certain phases without review (default: false)
    pub auto_advance: bool,
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

    /// Creates a new PluginConfig with default values.
    pub fn new() -> Self {
        Self {
            ticket_dir: PathBuf::from(Self::DEFAULT_TICKET_DIR),
            story_dir: PathBuf::from(Self::DEFAULT_STORY_DIR),
            work_dir: PathBuf::from(Self::DEFAULT_WORK_DIR),
            max_threads: Self::DEFAULT_MAX_THREADS,
            auto_advance: false,
        }
    }

    /// Creates a PluginConfig from a Zellij configuration map.
    pub fn from_config_map(config: &BTreeMap<String, String>) -> Self {
        let mut result = Self::new();

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

        if let Some(auto_advance) = config.get("auto_advance") {
            result.auto_advance = auto_advance == "true" || auto_advance == "1";
        }

        result
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

    /// DAG was recomputed
    DagRecomputed { ticket_count: usize },
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
        assert_eq!(Phase::Review.artifact_filename(), None);
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
}
