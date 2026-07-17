//! Lisa - A Zellij plugin for DAG-driven concurrent task scheduling
//!
//! This plugin implements the RDSPI workflow (Research -> Design -> Structure -> Plan -> Implement)
//! as a DAG-driven concurrent scheduler. It manages Claude Code sessions for each ticket,
//! tracks phase progress, and provides a live dashboard.

mod adapter;
mod assignment;
mod codex_ack;
mod completion_journal;
mod deadline;
mod ownership;
mod pane_name;
mod publication;
mod quarantine;
mod signal;
mod ui;

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Component, Path, PathBuf};

use zellij_tile::prelude::*;

use adapter::{
    resolve_adapter_or_native, FollowUp, FollowUpContext, ReadinessMode, ResetStrategy,
    SpawnContext,
};
use assignment::{write_assignment, AssignmentRef};
use completion_journal::{
    CompletionFailureClass, CompletionJournalAggregate, CompletionJournalTransition,
    FailureConsequence,
};
use deadline::{
    AcknowledgementInput, DeadlineEvaluator, HealthInput, ReviewInput, SessionAction, SessionInput,
    StaleInput, SystemClock, TransitionAction, TransitionInput, TransitionPolicy,
};

use lisa_core::capture::CaptureRecord;
use lisa_core::claim::AssignmentClaim;
use lisa_core::client::AgentClient;
use lisa_core::completion::LaunchFailure;
use lisa_core::completion::{
    reconcile as reconcile_completion, reduce as reduce_completion, AttemptId, CompletionDeadline,
    CompletionEvent, CompletionGenerationId, CompletionId, CompletionRejection, CompletionSeal,
    CompletionSealReceipt, CompletionState, CorrelationId, CurrentLeaseArtifactAdmission,
    DurableCompletionInputs, EffectCommand, Reconciliation, Retryability,
};
use lisa_core::context::PURPOSE_PARAGRAPH;
use lisa_core::dag::Dag;
use lisa_core::diagnostics;
use lisa_core::disposition::{
    parse_review_disposition, DispositionNote, RemedyOwner, ReviewDisposition,
};
use lisa_core::provenance::{
    self, AssignmentState, AssignmentTransitionRecord, ParkingTransitionRecord,
    ParkingTransitionType, ProvenanceLedgerRecord, ProvenanceRecord, ProvenanceRecordType, Route,
    RunOutcome,
};
use lisa_core::ticket;
use lisa_core::types::{
    ActivityEvent, AttemptLease, CompletionRejectionKind, Phase, PluginConfig, Thread, TicketId,
    TicketStatus,
};
use pane_name::{format_pane_name, PaneName};
use publication::{
    publication_nonce, PublicationErrors, PublicationPath, RustPublication, ShellPublication,
    TemporaryName,
};
use signal::{IdleTarget, SignalRecord, SignalRequest};

pub(crate) use publication::shell_quote;

/// How often (in seconds) the plugin rescans ticket files to detect phase changes.
const POLL_INTERVAL_SECS: f64 = 5.0;

/// Absolute window shared by the initial completion command and every
/// same-generation reconciliation replay.
const COMPLETION_RECONCILIATION_TIMEOUT_SECS: u64 = 60;

/// Maximum failed host-command observations for one completion generation.
const MAX_COMPLETION_FAILURES: u8 = 2;

const HISTORY_IDENTITY_ASK: &str = "Lisa needs a name for recording finished work. Run: `git config user.name \"You\"` and `git config user.email you@example.com` — or rerun `lisa init` and accept the history offer.";

/// Timeout (seconds) for waiting for a `.stopped` signal after phase completion.
/// If no signal arrives AND the pane has been signal-silent for the wind-down
/// period, fall back to sending `/clear` anyway.
const STOP_SIGNAL_TIMEOUT_SECS: u64 = 60;

/// Timeout (seconds) for waiting for a `.cleared` signal after sending `/clear`.
/// If no signal arrives AND the pane has been signal-silent for the wind-down
/// period, fall back to sending the prompt anyway. The quiet requirement means
/// the prompt is never injected into a session that is still working.
const CLEAR_SIGNAL_TIMEOUT_SECS: u64 = 90;

/// Grace period after submitting `/exit` before typing a fresh provider launch
/// command into the returned shell. Enter itself is deferred by
/// `ENTER_DELAY_SECS`; using a longer grace ensures the old TUI has fully torn
/// down before the scheduler treats the pane as a shell again.
const AGENT_EXIT_GRACE_SECS: u64 = 8;

/// The prompt text sent to an agent for a ticket.
///
/// `context_file` is the per-client project-context filename the agent should
/// read (`CLAUDE.md` for Claude Code, `AGENTS.md` for Codex — see
/// [`AgentClient::context_file`]). The prompt body is otherwise identical across
/// clients, so it stays single-sourced here.
pub(crate) fn ticket_prompt(
    ticket_dir: &Path,
    ticket_id: &str,
    context_file: &str,
    artifact_dir: &Path,
) -> String {
    let ticket = lisa_core::ticket::scan_tickets(ticket_dir)
        .ok()
        .and_then(|tickets| tickets.into_iter().find(|ticket| ticket.id == ticket_id));
    let ticket_path = ticket
        .as_ref()
        .map(|ticket| ticket.file_path.clone())
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| ticket_dir.join(format!("{}.md", ticket_id)));
    let review_recovery = if ticket.as_ref().map(|ticket| ticket.phase) == Some(Phase::Review) {
        format!(
            " Recovery case: this ticket already starts in Review. Inspect any existing \
             docs/active/work/{ticket_id}/review.md and the committed ticket-owned changes, then immediately \
             write a current-attempt review.md and review-disposition.json under {}/. Do not wait for a timeout, \
             redo earlier phases, or change source unless that Review finds a real defect.",
            artifact_dir.display(),
        )
    } else {
        String::new()
    };
    format!(
        "{purpose}\n\nRead the ticket at {path}, {context}, and docs/knowledge/rdspi-workflow.md. \
         Your job: start from the current phase in the ticket frontmatter and work through ALL remaining phases \
         (Research, Design, Structure, Plan, Implement, Review) without stopping between phases. \
         For each phase, write the artifact to {artifact_dir}/ then immediately continue to the next phase. \
         This directory is private to your current attempt; Lisa publishes admitted artifacts to \
         docs/active/work/{id}/ after verifying your lease. Do not write phase artifacts directly to that shared path. \
         Do NOT update the ticket's phase or status fields in the frontmatter — \
         Lisa detects your artifacts and handles all phase transitions automatically. \
         During Implement, commit each meaningful ticket-owned source unit only with \
         lisa commit-ticket and exact repository-relative --include paths. Do not use ordinary-index git add, \
         git add -A, or git commit for ticket work, and do not leave ticket-owned files staged, modified, or untracked. \
         During Review, write review.md summarizing changes, test coverage, and open concerns, and also write \
         {artifact_dir}/review-disposition.json with exactly {pass_json} when the work is ready to complete, \
         or {block_json} with a non-empty actionable reason when it is blocked. Both Review artifacts are required. \
         After Review is complete, \
         remain on this ticket and stop. Do not start another ticket until Lisa confirms the completion commit; \
         Lisa handles Done publication and seat release.{review_recovery}",
        path = ticket_path.display(),
        purpose = PURPOSE_PARAGRAPH,
        context = context_file,
        id = ticket_id,
        artifact_dir = artifact_dir.display(),
        pass_json = r#"{"disposition":"pass","reason":null}"#,
        block_json = r#"{"disposition":"block","reason":"<non-empty actionable reason>"}"#,
        review_recovery = review_recovery,
    )
}

/// Build the full shell command to launch Claude Code in a fresh pane.
/// Sets LISA_PANE_ID env var so the idle signal hook can identify the pane,
/// and ticket/attempt identity for attempt-scoped lifecycle signals.
///
/// `lisa_bin` is the absolute `lisa` path (plugin config) exported as `LISA_BIN`
/// so the `Stop` hook's `lisa capture-usage` (T-027-02) is reachable even when
/// the pane shell lacks `lisa` on PATH — mirroring the Codex adapter's
/// `lisa_bin` threading. `None`/empty omits the var entirely, keeping the launch
/// command without that environment assignment (the hook then falls back to a
/// PATH `lisa`). Dynamic values are shell-quoted before the payload is written.
pub(crate) fn build_claude_command(
    ticket_id: &str,
    pane_id: u32,
    attempt_id: u64,
    model: Option<&str>,
    lisa_bin: Option<&str>,
) -> String {
    // The Claude adapter owns the model→flag mapping (`--model`). When no model
    // is routed the flag is omitted, preserving the provider invocation while
    // the dynamic values remain uniformly shell-quoted.
    let model_flag = match model {
        Some(m) => format!(" --model {}", shell_quote(m)),
        None => String::new(),
    };
    let lisa_bin_env = match lisa_bin.filter(|s| !s.is_empty()) {
        Some(bin) => format!("LISA_BIN={} ", shell_quote(bin)),
        None => String::new(),
    };
    format!(
        "{}LISA_PANE_ID={} LISA_TICKET_ID={} LISA_ATTEMPT_ID={} claude --dangerously-skip-permissions{}",
        lisa_bin_env,
        pane_id,
        shell_quote(ticket_id),
        attempt_id,
        model_flag,
    )
}

/// The prompt text sent to a stuck Review session after the review timeout.
pub(crate) fn finish_up_prompt(
    _ticket_dir: &Path,
    artifact_dir: &Path,
    _ticket_id: &str,
) -> String {
    let review_path = artifact_dir.join("review.md");
    let disposition_path = artifact_dir.join("review-disposition.json");
    format!(
        "You have been in the Review phase for a while. Please finish both required Review artifacts. \
         Write the narrative review at {}. \
         It should cover: what changes were made, files created/modified/deleted, test coverage, \
         any open concerns or TODOs, and critical issues to surface for human review. \
         Write {} with exactly {pass_json} when the work is ready to complete, or {block_json} with a \
         non-empty actionable reason when it is blocked. \
         Do NOT update the ticket's phase or status fields or use ordinary-index git add/git commit to publish completion. \
         Remain on this ticket and wait until Lisa confirms the completion commit before starting another ticket.",
        review_path.display(),
        disposition_path.display(),
        pass_json = r#"{"disposition":"pass","reason":null}"#,
        block_json = r#"{"disposition":"block","reason":"<non-empty actionable reason>"}"#,
    )
}

/// Delay (seconds) between sending characters and pressing Enter.
///
/// Claude Code's TUI needs a full event-loop tick to process typed characters
/// and commit them to the input field before Enter can trigger "submit".
/// Two separate `write_to_pane_id` calls can coalesce in the PTY buffer,
/// causing the TUI to read text + CR in one chunk — Enter fires before the
/// input state is committed, so it inserts a newline instead of submitting.
/// A 2-second gap is imperceptible to human operators but gives the TUI
/// plenty of time to process the characters.
const ENTER_DELAY_SECS: f64 = 2.0;

/// One same-attempt chat redelivery is allowed before operator-visible failure.
const MAX_ASSIGNMENT_DELIVERY_RETRIES: u8 = 1;

/// Fresh Review attempts allowed after an agent-owned block during one loop.
/// The counter intentionally resets with the scheduler process.
const MAX_AGENT_BLOCK_RETRIES: u8 = 2;

/// Bounded startup grace for grace-mode providers (Codex). After a fresh launch
/// Lisa waits this long for the TUI to become input-ready, then paces the first
/// prompt directly from `Starting` into `Delivering`. The elapsed grace PACES
/// the send; it is never evidence of readiness or ownership (E-037, P2).
/// SessionStart-mode providers (Claude) ignore this and gate on their positive
/// process-start signal instead.
const STARTUP_GRACE_SECS: u64 = 8;

/// One startup whose shell boundary cannot be proven may be relaunched in the
/// same physical pane. The replacement cannot recursively recover again.
const MAX_SAME_PANE_STARTUP_RELAUNCHES: u8 = 1;

/// Strip the `/host/` prefix from a WASI sandbox path to get the host-relative path.
///
/// Inside the WASI sandbox, the host filesystem is mounted at `/host/`.
/// Commands sent to agent panes run on the host, so paths must not have this prefix.
fn strip_host_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    PathBuf::from(s.strip_prefix("/host/").unwrap_or(&s).to_string())
}

/// Lexically normalize an absolute host path without requiring the enclosing
/// host filesystem to be visible from WASI.
fn normalize_absolute_path(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err(format!(
            "completion path is not absolute: {}",
            path.display()
        ));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(format!(
                        "completion path escapes its filesystem root: {}",
                        path.display()
                    ));
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    Ok(normalized)
}

/// Terminate a hard-silent attempt at the Zellij pane boundary. Native unit
/// tests observe the state transition instead of invoking a plugin host call.
fn close_fenced_pane(pane_id: u32) {
    #[cfg(not(test))]
    close_terminal_pane(pane_id);
    #[cfg(test)]
    let _ = pane_id;
}

/// An agent pane slot — a pre-created terminal in the stacked layout.
struct AgentSlot {
    pane_id: u32,
    /// Which ticket is running in this slot (None = idle).
    ticket_id: Option<TicketId>,
    /// The provider-neutral attempt assigned to this physical seat. Cleared
    /// with `ticket_id` when the seat is released.
    attempt_lease: Option<AttemptLease>,
    /// Whether this slot currently hosts a resident agent session.
    has_session: bool,
    /// Transition state machine for session reuse handshake.
    transition_state: TransitionState,
    /// When the current transition started (for timeout fallbacks).
    transition_started_at: Option<std::time::SystemTime>,
    /// Earliest time this slot can accept new work (cooldown after completion).
    cooldown_until: Option<std::time::SystemTime>,
    /// When this pane last showed signs of life: a heartbeat/stop/idle/cleared
    /// signal arrived, or the plugin sent input to it. The scheduler only
    /// reuses a pane that has been quiet for the configured wind-down period —
    /// stop/idle signals alone are not trusted because agents often report
    /// stopped and then keep working for another minute or two.
    last_activity_at: Option<std::time::SystemTime>,
    /// Which agent client owns (or is being launched into) this pane, or `None`
    /// for a clean shell. Compatible tickets reuse the resident TUI via `/clear`;
    /// an incoming ticket for the other provider first recycles it via `/exit`.
    /// This prevents a fresh CLI command from being typed into the wrong TUI.
    last_client: Option<AgentClient>,
}

/// What action the modal should perform on Enter.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum ModalMode {
    #[default]
    MarkDone,
    ResetTicket,
    /// Quit confirmation: shows pending/new work, Enter=keep working, q=quit.
    QuitConfirm,
}

/// Visible lifecycle of the completion request submitted from MarkDone.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OperatorModalOutcome {
    Pending {
        ticket_id: TicketId,
        correlation_id: String,
    },
    Accepted {
        ticket_id: TicketId,
        correlation_id: String,
    },
    Rejected {
        ticket_id: TicketId,
        kind: CompletionRejectionKind,
        correlation_id: String,
        detail: String,
    },
}

impl OperatorModalOutcome {
    fn ticket_id(&self) -> &str {
        match self {
            Self::Pending { ticket_id, .. }
            | Self::Accepted { ticket_id, .. }
            | Self::Rejected { ticket_id, .. } => ticket_id,
        }
    }

    fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }
}

/// Per-slot state machine for session transitions. Same-provider reset is gated
/// by hook-generated `.stopped`/`.cleared` signals; cross-provider recycling
/// uses a bounded `/exit` grace period before launching at the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TransitionState {
    /// No transition pending — slot is idle or running normally.
    #[default]
    Idle,
    /// Phase complete, waiting for `.stopped` signal before sending `/clear`.
    WaitingForStop,
    /// `/clear` sent, waiting for `.cleared` signal before sending the prompt.
    WaitingForClear,
    /// `/exit` sent to a released session whose provider does not match the next
    /// ticket. Once the grace period expires, launch the new provider at shell.
    WaitingForExit,
    /// The attempt hosted by this pane was hard-silent, so Lisa closed the
    /// terminal pane. This is a terminal, non-reusable state with no retry.
    Fenced,
}

/// Bounded result of fencing one ticket attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FenceOutcome {
    Fenced { pane_id: u32 },
    AlreadyFenced { pane_id: u32 },
    NoAssignedPane,
}

/// Typed result of a completed scheduler failure or reclaim transition.
///
/// These values describe mutations that have already happened. They do not
/// replace lease, seat, thread, or pane state as scheduling authority.
#[derive(Debug, Clone, PartialEq, Eq)]
enum FailureTransitionOutcome {
    AssignmentDeliveryFailed {
        pane_id: u32,
        ticket_id: Option<TicketId>,
    },
    AssignmentClaimTimedOut {
        pane_id: u32,
        ticket_id: Option<TicketId>,
    },
    AssignmentRecoveryFailed {
        pane_id: u32,
        ticket_id: Option<TicketId>,
    },
    StartupFailed {
        pane_id: u32,
        ticket_id: Option<TicketId>,
    },
    StartupRecoveryFailed {
        pane_id: u32,
        ticket_id: TicketId,
    },
    ErrorReclaimed {
        pane_id: u32,
        ticket_id: TicketId,
    },
    SessionTimedOut {
        pane_id: u32,
        ticket_id: TicketId,
        fenced: bool,
    },
    StaleThreadReclaimed {
        pane_id: u32,
        ticket_id: TicketId,
        fenced: bool,
    },
}

/// Bounded scheduler consequence of one admitted blocking Review disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReviewBlockAction {
    Retry {
        retry_count: u8,
        retry_limit: u8,
    },
    Park {
        retry_count: Option<u8>,
        retry_limit: Option<u8>,
        recheck_eligible: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionFailureAction {
    Retry,
    WaitForDeadline,
    Park,
}

fn classify_completion_failure(detail: &str) -> CompletionFailureClass {
    let detail = detail.to_ascii_lowercase();
    if detail.contains("does not have any commits yet")
        || detail.contains("author identity unknown")
        || detail.contains("please tell me who you are")
        || detail.contains("unable to auto-detect email address")
        || detail.contains("empty ident name")
    {
        CompletionFailureClass::OperatorHistoryOrIdentity
    } else if detail.contains("lock")
        && (detail.contains("stale")
            || detail.contains("dead process")
            || detail.contains("no such process"))
    {
        CompletionFailureClass::OperatorStaleLock
    } else if detail.contains("permission denied")
        || detail.contains("read-only file system")
        || detail.contains("insufficient permission")
        || detail.contains("repository is not writable")
        || detail.contains("unable to write")
        || detail.contains("could not write")
    {
        CompletionFailureClass::OperatorRepositoryUnwritable
    } else if (detail.contains("index.lock") && detail.contains("another git process"))
        || detail.contains("resource temporarily unavailable")
        || detail.contains("temporarily locked")
    {
        CompletionFailureClass::TransientContention
    } else {
        CompletionFailureClass::Unrecognized
    }
}

fn completion_failure_action(
    class: CompletionFailureClass,
    failure_count: u8,
) -> CompletionFailureAction {
    match class {
        CompletionFailureClass::OperatorHistoryOrIdentity
        | CompletionFailureClass::OperatorRepositoryUnwritable
        | CompletionFailureClass::OperatorStaleLock
            if failure_count < MAX_COMPLETION_FAILURES =>
        {
            CompletionFailureAction::Retry
        }
        CompletionFailureClass::OperatorHistoryOrIdentity
        | CompletionFailureClass::OperatorRepositoryUnwritable
        | CompletionFailureClass::OperatorStaleLock
        | CompletionFailureClass::Unrecognized
        | CompletionFailureClass::DeadlineExpired => CompletionFailureAction::Park,
        CompletionFailureClass::TransientContention if failure_count < MAX_COMPLETION_FAILURES => {
            CompletionFailureAction::Retry
        }
        CompletionFailureClass::TransientContention => CompletionFailureAction::WaitForDeadline,
    }
}

fn completion_failure_ask(class: CompletionFailureClass, ticket_id: &str) -> Option<String> {
    match class {
        CompletionFailureClass::OperatorHistoryOrIdentity => {
            Some(HISTORY_IDENTITY_ASK.to_string())
        }
        CompletionFailureClass::OperatorRepositoryUnwritable => Some(format!(
            "Lisa cannot write finished work in this repository. Make it writable, then run: `lisa unblock {ticket_id}`."
        )),
        CompletionFailureClass::OperatorStaleLock => Some(format!(
            "Lisa found an old lock blocking finished work. Remove `.lisa-commit.lock`, then run: `lisa unblock {ticket_id}`."
        )),
        CompletionFailureClass::DeadlineExpired => Some(format!(
            "Lisa could not confirm whether finished work was recorded. Check the repository, then run: `lisa unblock {ticket_id}`."
        )),
        CompletionFailureClass::Unrecognized => None,
        CompletionFailureClass::TransientContention => Some(format!(
            "Lisa is waiting for another repository operation to finish before retrying {ticket_id}."
        )),
    }
}

fn review_block_action(owner: RemedyOwner, retries_consumed: u8) -> ReviewBlockAction {
    match owner {
        RemedyOwner::Agent if retries_consumed < MAX_AGENT_BLOCK_RETRIES => {
            ReviewBlockAction::Retry {
                retry_count: retries_consumed.saturating_add(1),
                retry_limit: MAX_AGENT_BLOCK_RETRIES,
            }
        }
        RemedyOwner::Agent => ReviewBlockAction::Park {
            retry_count: Some(MAX_AGENT_BLOCK_RETRIES),
            retry_limit: Some(MAX_AGENT_BLOCK_RETRIES),
            recheck_eligible: false,
        },
        RemedyOwner::Operator => ReviewBlockAction::Park {
            retry_count: None,
            retry_limit: None,
            recheck_eligible: false,
        },
        RemedyOwner::World => ReviewBlockAction::Park {
            retry_count: None,
            retry_limit: None,
            recheck_eligible: true,
        },
    }
}

/// Test-only observation of the safety-critical timeout teardown order.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum AttemptLifecycleEvent {
    LeaseRevoked { ticket_id: TicketId },
    CleanExitRequested { ticket_id: TicketId, pane_id: u32 },
    ShellInterrupted { ticket_id: TicketId, pane_id: u32 },
    ShellRelaunched { ticket_id: TicketId, pane_id: u32 },
    PaneFenced { ticket_id: TicketId, pane_id: u32 },
    SlotReleased { ticket_id: TicketId },
}

/// Scheduler-owned truth for the ticket assigned to a physical seat.
///
/// This is deliberately independent of [`TransitionState`]: a pane can be
/// waiting for `/clear` or `/exit` while its ticket assignment is still waiting
/// for positive provider acknowledgment. Absence from `State::seat_assignments`
/// means the seat is unassigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeatAssignmentState {
    /// A fresh provider process has been launched for the reserved seat, but
    /// its exact attempt-scoped process-start signal has not been observed.
    Starting {
        /// The launched [`AttemptLease::attempt_id`].
        generation: u64,
        /// None until the fresh launcher is actually submitted. Some bounds
        /// the wait for its exact process-start signal.
        start_deadline: Option<std::time::SystemTime>,
        /// Number of same-pane startup relaunches already submitted.
        relaunches: u8,
    },
    /// The failed attempt has been revoked and a successor lease installed,
    /// but Lisa has not yet positively observed a shell command boundary.
    ResettingStartup {
        generation: u64,
        reset_deadline: std::time::SystemTime,
    },
    /// The exact fresh provider process started and is ready for Lisa to submit
    /// its bounded attempt-specific chat reference on the next scheduler poll.
    ReadyForAssignment { generation: u64 },
    /// The bounded chat reference was submitted and awaits exact provider
    /// `UserPromptSubmit` evidence.
    Delivering {
        generation: u64,
        ack_deadline: std::time::SystemTime,
        retries: u8,
    },
    /// The assignment was delivered to a live Codex TUI, but no ownership
    /// evidence arrived in the initial window. This state waits passively for
    /// a claim or bounded fallback evidence and never re-injects the prompt.
    DeliveredAwaitingClaim {
        generation: u64,
        claim_deadline: std::time::SystemTime,
    },
    /// The seat is reserved for a ticket, but Codex has not acknowledged its
    /// current attempt lease.
    AssignedPendingAck {
        /// The assigned [`AttemptLease::attempt_id`].
        generation: u64,
        /// None until the generation-tagged prompt is actually submitted.
        ack_deadline: Option<std::time::SystemTime>,
    },
    /// The provider is considered to have accepted the assigned ticket.
    Owned,
    /// The original attempt timed out. A successor lease fences its one fresh-
    /// session fallback and remains not-owned until exact acknowledgment.
    Recovering {
        /// The recovery [`AttemptLease::attempt_id`].
        generation: u64,
        /// None while the old TUI exits; Some after the fresh launch is sent.
        ack_deadline: Option<std::time::SystemTime>,
    },
    /// The single fresh fallback failed. Retain the reservation for an explicit
    /// operator reset rather than automatically retrying forever.
    RecoveryFailed,
    /// A fresh provider launch did not publish its exact process-start signal.
    /// Retain the reservation for an explicit operator reset.
    StartupFailed,
    /// A started provider did not acknowledge the bounded chat assignment after
    /// the single allowed retry. The retained reservation requires reset.
    DeliveryFailed,
    /// A live Codex TUI retained its delivered assignment but produced no
    /// admissible ownership evidence before the bounded passive deadline.
    ClaimTimedOut,
}

/// Human-facing surface that emitted an explicit operator completion request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperatorRequestSource {
    MarkDoneKey,
}

/// Diagnostic origin for a request to durably complete a ticket. Every origin
/// enters the same completion transaction and result publisher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionSource {
    Artifact,
    Reconcile,
    Idle,
    Stopped(u32),
    OperatorRequested(OperatorRequestSource),
    ObservedDone,
}

/// Scheduler and operator evidence admitted by the sole typed completion
/// adapter. Each production completion origin must enter through one of these
/// variants before an effect can reach the executor.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CompletionInput {
    Artifact {
        ticket_id: TicketId,
        source_lease: AttemptLease,
    },
    Reconcile {
        ticket_id: TicketId,
        source_lease: AttemptLease,
    },
    Stopped {
        ticket_id: TicketId,
        pane_id: u32,
        source_lease: AttemptLease,
    },
    Idle {
        ticket_id: TicketId,
        source_lease: AttemptLease,
    },
    ObservedDone {
        ticket_id: TicketId,
        source_lease: Option<AttemptLease>,
    },
    OperatorRequested {
        ticket_id: TicketId,
        source: OperatorRequestSource,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CompletionAuthority {
    Attempt(AttemptLease),
    Operator,
}

#[derive(Debug, Clone)]
struct PendingCompletion {
    completion_key: CompletionGenerationId,
    correlation: CorrelationId,
    deadline: CompletionDeadline,
    #[allow(dead_code)]
    is_reconciliation_replay: bool,
    prior_phase: Phase,
    prior_status: TicketStatus,
    source: CompletionSource,
    authority: CompletionAuthority,
    completion_note: Option<DispositionNote>,
}

/// How an idle pane can satisfy an incoming provider request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotSelection {
    /// Fresh pane or a resident session already owned by the requested client.
    Compatible(usize),
    /// Quiet, released pane with a resident session from the other client.
    Recycle(usize),
}

/// A deferred Enter keypress for text already written to a pane.
///
/// Zellij does not identify which requested timeout produced a `Timer` event,
/// so every entry carries its own deadline. Unrelated scheduler timers may
/// inspect this queue, but must not submit a line before `ready_at`.
struct PendingEnter {
    pane_id: PaneId,
    ready_at: std::time::SystemTime,
}

/// State for the modal overlay (mark-done, reset-ticket, or quit-confirm).
#[derive(Default)]
struct MarkDoneModal {
    /// Whether the modal is currently visible.
    open: bool,
    /// Ticket IDs available for selection (sorted).
    ticket_ids: Vec<TicketId>,
    /// Currently highlighted index in `ticket_ids`.
    cursor: usize,
    /// What action to take on confirm.
    mode: ModalMode,
    /// (QuitConfirm only) New ticket IDs found outside the current DAG.
    new_ticket_ids: Vec<TicketId>,
    /// (MarkDone only) Visible feedback for the submitted completion request.
    operator_outcome: Option<OperatorModalOutcome>,
}

/// Main plugin state
#[derive(Default)]
pub struct State {
    /// The computed dependency graph from ticket frontmatter.
    dag: Dag,

    /// Active threads indexed by ticket ID.
    threads: HashMap<TicketId, Thread>,

    /// Currently authorized lease for each ticket. Absence means that no
    /// attempt owns the ticket, including the interval after revocation and
    /// before a successor dispatch.
    current_leases: HashMap<TicketId, AttemptLease>,

    /// Latest lease ever minted for each ticket in this scheduler process.
    /// Entries survive revocation/release so redispatch remains monotonic.
    lease_high_water: HashMap<TicketId, AttemptLease>,

    /// Agent-owned Review block retries consumed during this loop process.
    /// Durable scheduling authority remains ticket `status: blocked`; this map
    /// only decides when another open Review attempt is allowed.
    agent_block_retries: HashMap<TicketId, u8>,

    /// Safety-order trace used only by native scheduler tests.
    #[cfg(test)]
    attempt_lifecycle: Vec<AttemptLifecycleEvent>,

    /// Plugin configuration (ticket directory path, etc.)
    config: PluginConfig,

    /// Recent activity events for the dashboard display.
    activity_log: Vec<ActivityEvent>,

    /// Pre-created terminal pane slots for agent sessions.
    /// Populated on first PaneUpdate after permissions are granted.
    agent_slots: Vec<AgentSlot>,

    /// Assignment truth keyed by physical terminal pane ID. Slot `ticket_id`
    /// remains the reservation/routing key during handoff; this map says whether
    /// that reservation is pending acknowledgment, owned, or recovering.
    /// Missing means the seat has no current assignment.
    seat_assignments: HashMap<u32, SeatAssignmentState>,

    /// Exact successfully published assignment for each ticket's current
    /// attempt. Lease authority remains in `current_leases`; this map retains
    /// the nonce-bearing path that later delivery and claim evidence must use.
    assignment_refs: HashMap<TicketId, AssignmentRef>,

    /// Provider bootstrap-readiness classification per pane, recorded at launch
    /// dispatch (T-037-01-01). Observational only in this ticket; T-037-01-02
    /// keys the Codex startup-grace transition on it. Deliberately disjoint from
    /// `seat_assignments` so the readiness-mode shape settles without touching
    /// the `SeatAssignmentState` machine.
    seat_readiness: HashMap<u32, ReadinessMode>,

    /// Last pane name applied by Lisa, keyed by physical terminal pane ID.
    /// Used to suppress redundant Zellij rename operations across scheduler polls.
    last_pane_names: HashMap<u32, String>,

    /// Snapshot of ticket phases from last DAG build, for change detection.
    last_phases: HashMap<TicketId, Phase>,

    /// Whether initial loading has completed.
    initialized: bool,

    /// Whether permissions have been granted.
    permissions_granted: bool,

    /// Whether agent slots have been discovered from PaneUpdate.
    slots_discovered: bool,

    /// Whether scheduling of new tickets is paused (toggle with space).
    paused: bool,

    /// Which preset view is active on the dashboard (cycle with 'p').
    view_preset: ui::ViewPreset,

    /// Whether the loop has terminated (all tickets done).
    terminated: bool,

    /// Modal for manually marking tickets as done.
    modal: MarkDoneModal,

    /// Last known health status per ticket, for transition detection.
    last_health: HashMap<TicketId, lisa_core::types::HealthStatus>,

    /// Number of outstanding timers. Used to prevent timer chain duplication.
    pending_timer_count: u32,

    /// Whether one aggregate native check of observable world-owned parks is
    /// already running. This is cadence coordination only; ticket status stays
    /// the durable scheduling authority.
    world_recheck_in_flight: bool,

    /// Path to the idle signal directory (`.lisa/signals/` under /host/).
    signal_dir: PathBuf,

    /// Scheduler-owned, ignored staging root for attempt-attributed workflow
    /// artifacts (`.lisa/attempts/` under /host/).
    attempt_dir: PathBuf,

    /// Path to the append-only provenance ledger (`.lisa/provenance.jsonl` under
    /// /host/). One record is appended per ticket-run at teardown (T-027-01).
    /// Empty until `load()` runs — a native test that does not set it skips the
    /// write, so unrelated teardown-triggering tests never write to disk.
    ledger_path: PathBuf,

    /// Atomic append-only completion transition journal. Empty before `load()`
    /// so legacy native fixtures remain disk-free.
    completion_journal_path: PathBuf,

    /// False after a failed production journal restore. Completion effects fail
    /// closed until the durable history can be read unambiguously.
    completion_journal_healthy: bool,

    /// Latest typed completion aggregate state reconstructed from the journal.
    completion_aggregates: HashMap<TicketId, CompletionJournalAggregate>,

    /// Directory native Codex usage capture writes its append-only
    /// `captures.jsonl` ledger into (`.lisa/codex/` under /host/).
    codex_dir: PathBuf,

    /// Directory the Claude `Stop` hook's `lisa capture-usage` writes its
    /// append-only `captures.jsonl` ledger into (`.lisa/claude/` under /host/).
    claude_dir: PathBuf,

    /// Idle-without-artifact alerts detected during the current poll cycle.
    /// Cleared and re-populated each cycle by `check_idle_signals()`.
    idle_alerts: Vec<(TicketId, String)>,

    /// Scroll offset for the dashboard view (used with j/k keys).
    scroll_offset: usize,

    /// Panes waiting for a deferred Enter keypress.
    /// Characters are sent immediately; Enter is sent after `ENTER_DELAY_SECS`
    /// so the TUI has time to commit the text to its input field.
    pending_enters: VecDeque<PendingEnter>,

    /// Ticket IDs that have already received a finish-up prompt (prevents re-sending).
    finish_up_sent: HashSet<TicketId>,

    /// Ticket IDs already warned about exceeding their session/phase timeout
    /// while still active (prevents repeated warnings while waiting for quiet).
    over_budget_warned: HashSet<TicketId>,

    /// Recent session timeouts for dashboard display.
    /// Entries: (ticket_id, elapsed_secs, phase_at_timeout).
    /// Cleared when the ticket is rescheduled.
    timeout_alerts: Vec<(TicketId, u64, Phase)>,

    /// Recent `.error`-signal reclaims for dashboard display.
    /// Entries: (ticket_id, pane_id). Cleared when the ticket is rescheduled.
    error_alerts: Vec<(TicketId, u32)>,

    /// Absolute host project root, captured from `get_plugin_ids().initial_cwd`
    /// in `load()`. Commands launched via `run_command` run on the host (where
    /// the sandbox `/host` mount is meaningless), so notification invocations
    /// build absolute paths and cwd from this. Empty until `load()` runs — the
    /// notification host call is skipped while empty (e.g. in native tests).
    project_root: PathBuf,

    /// Absolute host root of the enclosing Git repository. Unlike
    /// `project_root`, this is discovered by the native launcher because the
    /// WASI `/host` mount cannot reliably observe enclosing directories.
    git_root: PathBuf,

    /// Panes already notified for `attention` (idle-without-artifact). Prevents
    /// a ~60s-repeating idle prompt from re-pinging. An entry is cleared when the
    /// pane emits a heartbeat (genuine progress), so a resumed-then-re-stalled
    /// agent can notify again.
    notified_attention: HashSet<u32>,

    /// Panes blocked on an `AskUserQuestion` (a `pane-<id>.awaiting` signal was
    /// seen). While set, all injection into the pane is suppressed so lisa never
    /// types over the question UI. Cleared on the pane's next heartbeat (the agent
    /// resumed real work). Deliberately never touches the liveness clock — a
    /// blocked-then-abandoned pane still trips stale detection on the normal
    /// silence clock (reclaim exemption is T-020-04, not here).
    awaiting_human: HashSet<u32>,

    /// Ticket completion transactions awaiting an attributed host-command
    /// result. While present, freshly scanned Done frontmatter is masked from
    /// the in-memory DAG so no scheduler consequence can publish early.
    pending_completions: HashMap<TicketId, PendingCompletion>,

    /// Native-test effect executor: records the exact inert command accepted at
    /// the production execution boundary before the Zellij host shim.
    #[cfg(test)]
    launched_completion_effects: Vec<EffectCommand>,

    /// When the loop started, used to compute `LISA_DURATION_SECS` on `complete`.
    loop_started_at: Option<std::time::SystemTime>,
}

impl State {
    const MAX_ACTIVITY_LOG: usize = 100;

    /// Apply a terminal-pane name only when it differs from Lisa's last value.
    ///
    /// The cache is updated before the host call because Zellij's rename API has
    /// no acknowledgement. This also gives native tests an observable record of
    /// rename intent while the host shim is a no-op.
    fn rename_slot(&mut self, pane_id: u32, name: String) -> bool {
        if !self.agent_slots.iter().any(|slot| slot.pane_id == pane_id)
            || self.last_pane_names.get(&pane_id) == Some(&name)
        {
            return false;
        }

        self.last_pane_names.insert(pane_id, name.clone());
        rename_terminal_pane(pane_id, name);
        true
    }

    /// Give newly discovered, unassigned panes their initial idle names once
    /// ChangeApplicationState permission is available.
    fn name_unnamed_idle_slots(&mut self) {
        let unnamed: Vec<(u32, Option<AgentClient>)> = self
            .agent_slots
            .iter()
            .filter(|slot| {
                slot.ticket_id.is_none() && !self.last_pane_names.contains_key(&slot.pane_id)
            })
            .map(|slot| {
                let resident_agent = if slot.has_session {
                    slot.last_client
                } else {
                    None
                };
                (slot.pane_id, resident_agent)
            })
            .collect();

        for (pane_id, resident_agent) in unnamed {
            self.rename_slot(pane_id, format_pane_name(PaneName::Idle { resident_agent }));
        }
    }

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

    /// Send text to a pane and queue a deferred Enter keypress.
    ///
    /// Characters are written immediately via `write_chars_to_pane_id`.
    /// The Enter key (0x0D) is queued and sent after `ENTER_DELAY_SECS` so the
    /// TUI has time to process the characters before receiving the submit action.
    fn send_line_to_pane(&mut self, text: &str, pane_id: PaneId) {
        // Belt-and-suspenders safety net: never inject into a pane that is blocked
        // on an AskUserQuestion. The per-caller guards keep state machines coherent;
        // this in-method drop makes a missed caller fail safe (no clobber). Return
        // before queuing the deferred Enter so a dropped line leaves no stray Enter.
        if let PaneId::Terminal(id) = pane_id {
            if self.is_pane_awaiting(id) {
                self.log_activity(ActivityEvent::Info {
                    message: format!("Suppressed injection into pane {} (awaiting human)", id),
                });
                return;
            }
        }
        write_chars_to_pane_id(text, pane_id);
        self.pending_enters.push_back(PendingEnter {
            pane_id,
            ready_at: std::time::SystemTime::now()
                + std::time::Duration::from_secs_f64(ENTER_DELAY_SECS),
        });
        set_timeout(ENTER_DELAY_SECS);
        self.pending_timer_count += 1;
    }

    /// Cancel incomplete shell input without assuming that a provider process
    /// exists. Any deferred Enter for the failed launch is removed first so it
    /// cannot race the reset probe.
    fn interrupt_shell_input(&mut self, pane_id: u32) {
        self.pending_enters
            .retain(|pending| pending.pane_id != PaneId::Terminal(pane_id));
        write_to_pane_id(vec![3], PaneId::Terminal(pane_id)); // Ctrl-C
    }

    /// Atomically prepare a complete fresh provider launch outside the PTY.
    ///
    /// The pane receives only the returned `sh <path>` indirection. Publication
    /// uses a same-directory rename, so a successful return proves the shell can
    /// address a complete script. Callers must not queue pane input on `Err`.
    fn prepare_fresh_launch(
        artifact_dir: &Path,
        pane_id: u32,
        payload: &str,
    ) -> Result<String, String> {
        std::fs::create_dir_all(artifact_dir).map_err(|error| {
            format!(
                "cannot create launch directory {}: {error}",
                artifact_dir.display()
            )
        })?;

        let destination = artifact_dir.join(format!(".lisa-launch-{pane_id}.sh"));
        let script = format!("#!/bin/sh\n{payload}\n");
        let destination = RustPublication {
            path: PublicationPath {
                destination,
                temporary_name: TemporaryName::Nonce {
                    prefix: format!(".lisa-launch-{pane_id}.sh.tmp."),
                },
            },
            body: script.as_bytes(),
            errors: PublicationErrors {
                write: "cannot write launch payload",
                publish: "cannot publish launch payload",
            },
        }
        .publish()?;

        let shell_path = strip_host_prefix(&destination);
        Ok(format!("sh {}", shell_quote(&shell_path.to_string_lossy())))
    }

    /// Atomically publish and retain the complete instructions for one exact
    /// ticket attempt. A reference becomes scheduler-visible only after rename.
    fn prepare_assignment(
        &mut self,
        artifact_dir: &Path,
        lease: &AttemptLease,
        assignment: &str,
    ) -> Result<AssignmentRef, String> {
        let assignment = write_assignment(
            artifact_dir,
            lease,
            publication_nonce(),
            assignment.as_bytes(),
        )?;
        self.assignment_refs
            .insert(lease.ticket_id.clone(), assignment.clone());
        Ok(assignment)
    }

    /// Construct a bounded shell command whose successful execution positively
    /// proves that the pane returned to a shell command boundary. The payload is
    /// the exact scheduler-minted successor lease and publication is atomic.
    fn shell_readiness_probe(
        signal_dir: &Path,
        pane_id: u32,
        lease: &AttemptLease,
    ) -> Result<String, String> {
        let body = serde_json::to_string(lease)
            .map_err(|error| format!("cannot serialize shell readiness lease: {error}"))?;
        let host_signal_dir = strip_host_prefix(signal_dir);
        let destination = host_signal_dir.join(format!("pane-{pane_id}.shell-ready"));
        ShellPublication {
            path: PublicationPath {
                destination,
                temporary_name: TemporaryName::AttemptNonce {
                    prefix: format!("pane-{pane_id}.shell-ready.tmp."),
                    attempt_id: lease.attempt_id,
                },
            },
            body: &body,
        }
        .command()
    }

    /// Best-effort removal of pane-scoped predecessor lifecycle state. Exact
    /// attempt validation remains the authority boundary; cleanup prevents old
    /// input and markers from lingering around the reset transaction.
    fn clear_pane_lifecycle_signals(&self, pane_id: u32) {
        for suffix in [
            "lease",
            "started",
            "ack",
            "heartbeat",
            "idle",
            "stopped",
            "cleared",
            "error",
            "awaiting",
            "shell-ready",
            "claim",
        ] {
            let _ = std::fs::remove_file(self.signal_dir.join(format!("pane-{pane_id}.{suffix}")));
        }
    }

    /// True if `pane_id` is currently blocked on an `AskUserQuestion` (its
    /// `pane-<id>.awaiting` signal was seen and no heartbeat has cleared it yet).
    fn is_pane_awaiting(&self, pane_id: u32) -> bool {
        self.awaiting_human.contains(&pane_id)
    }

    /// Remove and return only Enter keypresses whose individual delay elapsed.
    /// Future entries retain their order so an unrelated Timer event cannot
    /// prematurely submit text that its TUI has not committed yet.
    fn take_due_pending_enters(&mut self, now: std::time::SystemTime) -> Vec<PaneId> {
        let mut due = Vec::new();
        let mut future = VecDeque::new();

        while let Some(pending) = self.pending_enters.pop_front() {
            if now.duration_since(pending.ready_at).is_ok() {
                due.push(pending.pane_id);
            } else {
                future.push_back(pending);
            }
        }

        self.pending_enters = future;
        due
    }

    /// Send Enter to panes whose deferred keypress deadlines have elapsed.
    fn flush_pending_enters(&mut self, now: std::time::SystemTime) {
        for pane_id in self.take_due_pending_enters(now) {
            write_to_pane_id(vec![13], pane_id); // Enter key
        }
    }

    /// Build the `(argv, env)` for invoking the user's `on-notify` hook.
    ///
    /// Pure and host-free so it can be unit-tested. The command is `sh -c` with a
    /// guard that runs the hook only if it is executable and **exits 0 when it is
    /// absent** (a missing/non-executable hook is a silent no-op, not a failure).
    /// `$1`/`$2` carry `event`/`detail`, matching the `on-notify <event> [detail]`
    /// contract; the rest of the contract is passed via environment variables.
    fn build_notify_command(
        project_root: &Path,
        event: &str,
        detail: &str,
        extra_env: &[(&str, String)],
    ) -> (Vec<String>, BTreeMap<String, String>) {
        let hook = project_root.join(".lisa/hooks/on-notify");

        let mut env: BTreeMap<String, String> = BTreeMap::new();
        env.insert("LISA_HOOK".to_string(), hook.to_string_lossy().into_owned());
        env.insert("LISA_EVENT".to_string(), event.to_string());
        env.insert(
            "LISA_PROJECT".to_string(),
            project_root.to_string_lossy().into_owned(),
        );
        for (k, v) in extra_env {
            env.insert((*k).to_string(), v.clone());
        }

        // `if [ -x ]` (not `test -x && ...`) so an absent hook exits 0.
        let guard = r#"if [ -x "$LISA_HOOK" ]; then "$LISA_HOOK" "$1" "$2"; fi"#;
        let argv = vec![
            "sh".to_string(),
            "-c".to_string(),
            guard.to_string(),
            "sh".to_string(),
            event.to_string(),
            detail.to_string(),
        ];

        (argv, env)
    }

    /// Fire the `on-notify` hook on the host via Zellij's `run_command`.
    ///
    /// No-op until `project_root` is captured in `load()` (so native tests, which
    /// build `State` directly, never reach the host call). The `context` carries a
    /// `lisa_notify` key so `RunCommandResult` can be attributed back to this call.
    fn fire_notify(&self, event: &str, detail: &str, extra_env: &[(&str, String)]) {
        if self.project_root.as_os_str().is_empty() {
            return;
        }
        let (argv, env) = Self::build_notify_command(&self.project_root, event, detail, extra_env);
        let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        let mut context = BTreeMap::new();
        context.insert("lisa_notify".to_string(), event.to_string());
        run_command_with_env_variables_and_cwd(&argv_refs, env, self.project_root.clone(), context);
    }

    fn completion_repository_relative_path(&self, path: &Path) -> Result<PathBuf, String> {
        if self.project_root.as_os_str().is_empty() {
            return Err("Lisa project root is not available".to_string());
        }
        if self.git_root.as_os_str().is_empty() {
            return Err("Git root is not available".to_string());
        }

        let host_path = match path.strip_prefix("/host") {
            Ok(relative) => self.project_root.join(relative),
            Err(_) if path.is_absolute() => path.to_path_buf(),
            Err(_) => self.project_root.join(path),
        };
        let host_path = normalize_absolute_path(&host_path)?;
        let git_root = normalize_absolute_path(&self.git_root)?;
        let relative = host_path.strip_prefix(&git_root).map_err(|_| {
            format!(
                "completion path outside Git root: {} is not below {}",
                host_path.display(),
                git_root.display()
            )
        })?;
        if relative.as_os_str().is_empty() {
            return Err(format!(
                "completion path outside Git root: {} selects the repository root",
                host_path.display()
            ));
        }
        Ok(relative.to_path_buf())
    }

    /// Root of the private attempt tree. Production uses `.lisa/attempts`; the
    /// fallback keeps directly-constructed native tests deterministic without
    /// requiring `load()`.
    fn attempt_root(&self) -> PathBuf {
        if self.attempt_dir.as_os_str().is_empty() {
            self.config.work_dir.join(".attempts")
        } else {
            self.attempt_dir.clone()
        }
    }

    /// Private workflow directory for one execution attempt.
    fn attempt_work_dir(&self, lease: &AttemptLease) -> PathBuf {
        self.attempt_root()
            .join(&lease.ticket_id)
            .join(lease.attempt_id.to_string())
            .join("work")
    }

    /// Highest positive attempt generation retained on disk for one ticket.
    ///
    /// Directory names are the durable generation namespace. Malformed
    /// entries cannot manufacture lease history and are ignored.
    fn durable_attempt_high_water(&self, ticket_id: &str) -> Option<AttemptLease> {
        let entries = std::fs::read_dir(self.attempt_root().join(ticket_id)).ok()?;
        let attempt_id = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .filter(|entry| entry.path().join("work").is_dir())
            .filter_map(|entry| entry.file_name().to_str()?.parse::<u64>().ok())
            .filter(|attempt_id| *attempt_id > 0)
            .max()?;
        Some(AttemptLease {
            ticket_id: ticket_id.to_string(),
            attempt_id,
        })
    }

    fn pane_attempt_lease(&self, pane_id: u32) -> Option<AttemptLease> {
        self.agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| slot.attempt_lease.clone())
    }

    /// Resolve the artifact directory for a prompt addressed to one pane. Real
    /// scheduled attempts must have an exact current lease. The canonical
    /// fallback supports pre-lease unit fixtures only when no authority exists.
    fn prompt_artifact_dir(&self, ticket_id: &str, pane_id: u32) -> Option<PathBuf> {
        match self.pane_attempt_lease(pane_id) {
            Some(lease)
                if lease.ticket_id == ticket_id
                    && lease.is_current(self.current_leases.get(ticket_id)) =>
            {
                Some(self.attempt_work_dir(&lease))
            }
            None if !self.current_leases.contains_key(ticket_id) => {
                Some(self.config.work_dir.join(ticket_id))
            }
            _ => None,
        }
    }

    /// Publish the marker immediately before delivering this attempt's prompt
    /// or launch. Deferring until after `/clear` or `/exit` prevents the
    /// predecessor process from copying a successor identity during handoff.
    fn publish_prompt_lease_marker(&self, ticket_id: &str, pane_id: u32) -> Result<(), String> {
        match self.pane_attempt_lease(pane_id) {
            Some(lease)
                if lease.ticket_id == ticket_id
                    && lease.is_current(self.current_leases.get(ticket_id)) =>
            {
                self.write_pane_lease_marker(pane_id, &lease)
            }
            None if !self.current_leases.contains_key(ticket_id) => Ok(()),
            _ => Err(format!(
                "pane {pane_id} does not carry the current lease for {ticket_id}"
            )),
        }
    }

    /// Publish the lease marker copied by native heartbeat hooks. The rename is
    /// same-directory and atomic, so consumers never observe partial JSON.
    fn write_pane_lease_marker(&self, pane_id: u32, lease: &AttemptLease) -> Result<(), String> {
        if self.signal_dir.as_os_str().is_empty() {
            #[cfg(test)]
            return Ok(());
            #[cfg(not(test))]
            return Err("signal directory is not configured".to_string());
        }
        let signal_dir = self.signal_dir.clone();
        std::fs::create_dir_all(&signal_dir).map_err(|error| {
            format!(
                "cannot create signal directory {}: {error}",
                signal_dir.display()
            )
        })?;
        let destination = signal_dir.join(format!("pane-{pane_id}.lease"));
        let body = serde_json::to_vec(lease)
            .map_err(|error| format!("cannot serialize attempt lease: {error}"))?;
        RustPublication {
            path: PublicationPath {
                destination,
                temporary_name: TemporaryName::AttemptNonce {
                    prefix: format!("pane-{pane_id}.lease.tmp."),
                    attempt_id: lease.attempt_id,
                },
            },
            body: &body,
            errors: PublicationErrors {
                write: "cannot write pane lease marker",
                publish: "cannot publish pane lease marker",
            },
        }
        .publish()?;
        Ok(())
    }

    /// Atomically publish one artifact from an explicit private attempt.
    /// Authority validation remains the caller's responsibility.
    fn publish_attempt_artifact(
        &self,
        lease: &AttemptLease,
        artifact_name: &str,
    ) -> Result<bool, String> {
        let staged = self.attempt_work_dir(lease).join(artifact_name);
        if !staged.is_file() {
            return Ok(false);
        }
        let body = std::fs::read(&staged).map_err(|error| {
            format!("cannot read staged artifact {}: {error}", staged.display())
        })?;
        let canonical_dir = self.config.work_dir.join(&lease.ticket_id);
        std::fs::create_dir_all(&canonical_dir).map_err(|error| {
            format!(
                "cannot create canonical artifact directory {}: {error}",
                canonical_dir.display()
            )
        })?;
        RustPublication {
            path: PublicationPath {
                destination: canonical_dir.join(artifact_name),
                temporary_name: TemporaryName::Exact {
                    file_name: format!(".{artifact_name}.attempt-{}.tmp", lease.attempt_id),
                },
            },
            body: &body,
            errors: PublicationErrors {
                write: "cannot write canonical artifact temporary",
                publish: "cannot publish canonical artifact",
            },
        }
        .publish()?;
        Ok(true)
    }

    /// Admit one phase artifact. Leased attempts publish only from their
    /// private staging directory after exact current-lease validation. The
    /// unleased branch exists solely for historical fixtures with no authority
    /// registered for the ticket.
    fn admit_artifact(
        &self,
        ticket_id: &str,
        candidate: Option<&AttemptLease>,
        artifact_name: &str,
    ) -> Result<bool, String> {
        let canonical = self.config.work_dir.join(ticket_id).join(artifact_name);
        let Some(lease) = candidate else {
            return Ok(!self.current_leases.contains_key(ticket_id) && canonical.exists());
        };
        if lease.ticket_id != ticket_id || !lease.is_current(self.current_leases.get(ticket_id)) {
            return Err(format!(
                "attempt {:?} does not hold the current lease for {ticket_id}",
                lease
            ));
        }
        self.publish_attempt_artifact(lease, artifact_name)
    }

    fn build_completion_command(
        &self,
        completion_key: &CompletionGenerationId,
        ticket_file: &Path,
    ) -> Result<(Vec<String>, BTreeMap<String, String>), String> {
        let ticket_id = completion_key.completion_id().as_str();
        let lisa_bin = self
            .config
            .lisa_bin
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "lisa_bin is not configured".to_string())?;
        if self.project_root.as_os_str().is_empty() {
            return Err("project root is not available".to_string());
        }
        let ticket_file = self.completion_repository_relative_path(ticket_file)?;
        let work_dir =
            self.completion_repository_relative_path(&self.config.work_dir.join(ticket_id))?;
        let argv = vec![
            lisa_bin.to_string(),
            "complete-ticket".to_string(),
            "--path".to_string(),
            self.git_root.display().to_string(),
            "--ticket-id".to_string(),
            ticket_id.to_string(),
            "--attempt-id".to_string(),
            completion_key.attempt_id().to_string(),
            "--completion-generation".to_string(),
            completion_key.generation().to_string(),
            "--message".to_string(),
            format!("Complete {ticket_id}"),
            "--ticket-file".to_string(),
            ticket_file.display().to_string(),
            "--work-dir".to_string(),
            work_dir.display().to_string(),
        ];
        let mut context = BTreeMap::new();
        context.insert("lisa_completion".to_string(), ticket_id.to_string());
        Ok((argv, context))
    }

    /// Build the native automation command that safely verifies every
    /// observable world-owned parked remedy.
    fn build_world_recheck_command(
        &self,
    ) -> Result<(Vec<String>, BTreeMap<String, String>), String> {
        let lisa_bin = self
            .config
            .lisa_bin
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "lisa_bin is not configured".to_string())?;
        if self.project_root.as_os_str().is_empty() {
            return Err("project root is not available".to_string());
        }
        let argv = vec![
            lisa_bin.to_string(),
            "recheck-world".to_string(),
            "--path".to_string(),
            self.project_root.display().to_string(),
        ];
        let mut context = BTreeMap::new();
        context.insert("lisa_world_recheck".to_string(), "world".to_string());
        Ok((argv, context))
    }

    /// True when the current durable board has a world-owned park with an
    /// observable check. The native command independently repeats this filter
    /// before it executes anything.
    fn has_observable_world_park(&self) -> bool {
        lisa_core::parking::collect_parked_remedies(self.dag.tickets(), &self.config.work_dir)
            .into_iter()
            .any(|remedy| remedy.remedy_owner == RemedyOwner::World && remedy.check.is_some())
    }

    /// Launch at most one asynchronous native recheck at the scheduler's
    /// existing cadence. Checks never execute on the WASM event thread.
    fn request_world_recheck(&mut self) -> bool {
        if self.world_recheck_in_flight || !self.has_observable_world_park() {
            return false;
        }
        let (argv, context) = match self.build_world_recheck_command() {
            Ok(command) => command,
            Err(error) => {
                self.log_activity(ActivityEvent::Warning {
                    message: format!("World recheck unavailable: {error}"),
                });
                return false;
            }
        };
        let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        self.world_recheck_in_flight = true;
        run_command_with_env_variables_and_cwd(
            &argv_refs,
            BTreeMap::new(),
            self.project_root.clone(),
            context,
        );
        true
    }

    /// Consume one attributed native recheck result. A nonempty successful
    /// result means at least one ticket status was durably reopened; the DAG,
    /// Unpark provenance, and ordinary scheduler then observe that change.
    fn handle_world_recheck_result(
        &mut self,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        _stderr: Vec<u8>,
    ) {
        self.world_recheck_in_flight = false;
        if exit_code != Some(0) {
            self.log_activity(ActivityEvent::Warning {
                message: format!("World recheck failed (exit {exit_code:?})"),
            });
            return;
        }

        let reopened: Vec<_> = String::from_utf8_lossy(&stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        if reopened.is_empty() {
            return;
        }

        self.rebuild_dag();
        self.reconcile_unpark_transitions();
        self.schedule_ready_tickets();
        self.log_activity(ActivityEvent::Info {
            message: format!("World recheck reopened {}", reopened.join(", ")),
        });
    }

    /// Admit and validate the current attempt's explicit Review outcome.
    fn admit_passing_review(
        &mut self,
        ticket_id: &str,
        source_lease: Option<&AttemptLease>,
    ) -> Result<Option<DispositionNote>, CompletionRejection> {
        const DISPOSITION_ARTIFACT: &str = "review-disposition.json";

        match self.admit_artifact(ticket_id, source_lease, DISPOSITION_ARTIFACT) {
            Ok(true) => {}
            Ok(false) => {
                return Err(CompletionRejection::DispositionBlocked {
                    reason: format!("missing required {DISPOSITION_ARTIFACT}"),
                });
            }
            Err(reason) => {
                return Err(CompletionRejection::DispositionBlocked {
                    reason: format!("could not admit {DISPOSITION_ARTIFACT}: {reason}"),
                });
            }
        }

        self.passing_review_disposition(ticket_id)
    }

    /// Evaluate the canonical E-040 Review verdict without claiming an
    /// attempt's private artifact authority.
    fn passing_review_disposition(
        &self,
        ticket_id: &str,
    ) -> Result<Option<DispositionNote>, CompletionRejection> {
        const DISPOSITION_ARTIFACT: &str = "review-disposition.json";
        let disposition_path = self
            .config
            .work_dir
            .join(ticket_id)
            .join(DISPOSITION_ARTIFACT);
        match parse_review_disposition(disposition_path) {
            ReviewDisposition::Pass => Ok(None),
            ReviewDisposition::Note(note) => Ok(Some(note)),
            ReviewDisposition::Block { reason, .. } => {
                Err(CompletionRejection::DispositionBlocked { reason })
            }
            ReviewDisposition::Invalid { reason } => Err(CompletionRejection::DispositionBlocked {
                reason: format!("invalid review disposition: {reason}"),
            }),
        }
    }

    fn operator_modal_targets(&self, ticket_id: &str) -> bool {
        self.modal.open
            && self.modal.mode == ModalMode::MarkDone
            && self
                .modal
                .operator_outcome
                .as_ref()
                .map(OperatorModalOutcome::ticket_id)
                .or_else(|| {
                    self.modal
                        .ticket_ids
                        .get(self.modal.cursor)
                        .map(String::as_str)
                })
                == Some(ticket_id)
    }

    fn show_operator_modal_rejection(
        &mut self,
        ticket_id: &str,
        kind: CompletionRejectionKind,
        correlation_id: String,
        detail: String,
    ) {
        if self.operator_modal_targets(ticket_id) {
            self.modal.operator_outcome = Some(OperatorModalOutcome::Rejected {
                ticket_id: ticket_id.to_string(),
                kind,
                correlation_id,
                detail,
            });
        }
    }

    fn show_operator_modal_accepted(&mut self, ticket_id: &str, correlation_id: String) {
        if self.operator_modal_targets(ticket_id) {
            self.modal.operator_outcome = Some(OperatorModalOutcome::Accepted {
                ticket_id: ticket_id.to_string(),
                correlation_id,
            });
        }
    }

    fn log_completion_rejection(
        &mut self,
        ticket_id: &str,
        correlation: &CompletionGenerationId,
        rejection: &CompletionRejection,
    ) {
        let (kind, detail) = match rejection {
            CompletionRejection::AlreadyPending { .. } => (
                CompletionRejectionKind::AlreadyPending,
                rejection.to_string(),
            ),
            CompletionRejection::StaleLease { .. } => {
                (CompletionRejectionKind::StaleLease, rejection.to_string())
            }
            CompletionRejection::DispositionBlocked { reason } => {
                (CompletionRejectionKind::DispositionBlocked, reason.clone())
            }
            CompletionRejection::DependencyBlocked { reason } => {
                (CompletionRejectionKind::DependencyBlocked, reason.clone())
            }
            CompletionRejection::LaunchFailed { source } => (
                CompletionRejectionKind::LaunchFailed,
                source.message().to_string(),
            ),
            CompletionRejection::UnexpectedEvent { .. }
            | CompletionRejection::CorrelationMismatch { .. } => {
                self.log_activity(ActivityEvent::Warning {
                    message: format!(
                        "{ticket_id}: a completion reply arrived out of order and was set aside; Lisa continues from the current state. [{rejection}; ref {correlation}]"
                    ),
                });
                return;
            }
        };

        let correlation_id = correlation.to_string();
        self.show_operator_modal_rejection(ticket_id, kind, correlation_id.clone(), detail.clone());
        self.log_activity(ActivityEvent::CompletionRejected {
            ticket_id: ticket_id.to_string(),
            kind,
            correlation_id,
            detail,
        });
    }

    fn completion_correlation(
        completion_id: CompletionId,
        attempt_id: AttemptId,
    ) -> CompletionGenerationId {
        CompletionGenerationId::new(completion_id, attempt_id, 1)
    }

    fn reject_stale_lease(
        &mut self,
        ticket_id: &str,
        correlation: &CompletionGenerationId,
        attempt_id: impl Into<String>,
    ) {
        self.log_completion_rejection(
            ticket_id,
            correlation,
            &CompletionRejection::StaleLease {
                attempt_id: AttemptId::new(attempt_id),
            },
        );
    }

    fn review_lease_is_current(&self, ticket_id: &str, lease: &AttemptLease) -> bool {
        lease.ticket_id == ticket_id && lease.is_current(self.current_leases.get(ticket_id))
    }

    fn admit_correlated_review(
        &mut self,
        ticket_id: &str,
        lease: &AttemptLease,
        correlation: &CompletionGenerationId,
    ) -> Option<Option<DispositionNote>> {
        if !self.review_lease_is_current(ticket_id, lease) {
            self.reject_stale_lease(ticket_id, correlation, lease.attempt_id.to_string());
            return None;
        }

        match self.admit_passing_review(ticket_id, Some(lease)) {
            Ok(note) => Some(note),
            Err(rejection) => {
                self.log_completion_rejection(ticket_id, correlation, &rejection);
                None
            }
        }
    }

    /// Restore the durable completion aggregate before initial DAG authority is
    /// derived. A malformed history is visible and fail-closed.
    fn restore_completion_journal(&mut self) {
        match completion_journal::load(&self.completion_journal_path) {
            Ok(aggregates) => {
                self.completion_aggregates = aggregates;
                self.completion_journal_healthy = true;
            }
            Err(error) => {
                self.completion_aggregates.clear();
                self.completion_journal_healthy = false;
                self.log_activity(ActivityEvent::Error {
                    message: format!("Completion journal restore failed: {error}"),
                });
            }
        }
    }

    /// Publish one transition before updating the in-memory aggregate. Empty
    /// paths retain the existing disk-free behavior of pre-load native tests.
    fn journal_completion_transition(
        &mut self,
        transition: CompletionJournalTransition,
    ) -> Result<(), String> {
        if self.completion_journal_path.as_os_str().is_empty() {
            return Ok(());
        }
        if !self.completion_journal_healthy {
            return Err("completion journal is unavailable after a failed restore".to_string());
        }
        let aggregate = completion_journal::append_with_seal(
            &self.completion_journal_path,
            self.config.completion_seal,
            transition,
        )?;
        self.completion_aggregates.insert(
            aggregate.completion_key().completion_id().to_string(),
            aggregate,
        );
        Ok(())
    }

    /// Mask Done bytes written by a completion command until its correlated
    /// result is durably confirmed. Live pending state takes precedence; after
    /// restart the journal supplies the same prior phase/status facts.
    fn mask_completion_transaction(&self, scanned: &mut lisa_core::types::Ticket) {
        if let Some(pending) = self.pending_completions.get(&scanned.id) {
            scanned.phase = pending.prior_phase;
            scanned.status = pending.prior_status;
        } else if let Some(aggregate) = self.completion_aggregates.get(&scanned.id) {
            if aggregate.masks_durable_done()
                && scanned.status != TicketStatus::Blocked
                && (scanned.phase == Phase::Done || scanned.status == TicketStatus::Done)
            {
                scanned.phase = aggregate.prior_phase();
                scanned.status = aggregate.prior_status();
            }
        }
    }

    /// Reconstruct the aggregate state from facts the adapter can currently
    /// prove. Journal state survives plugin restart; pending and DAG facts keep
    /// pre-load native fixtures and repositories without a journal compatible.
    fn reconciliation_state(&self, ticket_id: &str) -> CompletionState {
        if let Some(aggregate) = self.completion_aggregates.get(ticket_id) {
            let durable_ticket = self.dag.get_ticket(&ticket_id.to_string());
            if (matches!(aggregate.state(), CompletionState::Confirmed)
                && durable_ticket
                    .map(|ticket| {
                        ticket.phase != Phase::Done || ticket.status != TicketStatus::Done
                    })
                    .unwrap_or(false))
                || (matches!(
                    aggregate.state(),
                    CompletionState::Rejected {
                        retryability: Retryability::ActionRequired,
                        ..
                    }
                ) && durable_ticket.is_some_and(|ticket| ticket.status == TicketStatus::Open))
            {
                CompletionState::Eligible
            } else {
                aggregate.state().clone()
            }
        } else if self.pending_completions.contains_key(ticket_id) {
            CompletionState::Requested
        } else if self
            .dag
            .get_ticket(&ticket_id.to_string())
            .map(|ticket| ticket.phase == Phase::Done && ticket.status == TicketStatus::Done)
            .unwrap_or(false)
        {
            CompletionState::Confirmed
        } else {
            CompletionState::Eligible
        }
    }

    /// Re-derive the durable inputs for one exact current attempt. Missing or
    /// unreadable evidence remains fail-closed and cannot create admission.
    fn review_completion_inputs(
        &mut self,
        ticket_id: &str,
        source_lease: &AttemptLease,
    ) -> DurableCompletionInputs {
        const REVIEW_ARTIFACT: &str = "review.md";
        const DISPOSITION_ARTIFACT: &str = "review-disposition.json";

        let artifact_admission = match self.admit_artifact(
            ticket_id,
            Some(source_lease),
            REVIEW_ARTIFACT,
        ) {
            Ok(true) => Some(CurrentLeaseArtifactAdmission {
                attempt_id: AttemptId::new(source_lease.attempt_id.to_string()),
                completion_id: CompletionId::new(ticket_id),
            }),
            Ok(false) => None,
            Err(reason) => {
                self.log_activity(ActivityEvent::Error {
                    message: format!(
                        "Completion reconciliation could not admit {REVIEW_ARTIFACT} for {ticket_id}: {reason}"
                    ),
                });
                None
            }
        };

        let disposition =
            match self.admit_artifact(ticket_id, Some(source_lease), DISPOSITION_ARTIFACT) {
                Ok(true) => parse_review_disposition(
                    self.config
                        .work_dir
                        .join(ticket_id)
                        .join(DISPOSITION_ARTIFACT),
                ),
                Ok(false) => ReviewDisposition::Invalid {
                    reason: format!("missing required {DISPOSITION_ARTIFACT}"),
                },
                Err(reason) => ReviewDisposition::Invalid {
                    reason: format!("could not admit {DISPOSITION_ARTIFACT}: {reason}"),
                },
            };

        DurableCompletionInputs {
            artifact_admission,
            disposition,
        }
    }

    /// Convert scheduler/operator evidence into a typed core decision and
    /// execute only the effect returned by the pure reducer or reconciler.
    fn dispatch_completion(&mut self, input: CompletionInput) -> bool {
        self.dispatch_completion_at(input, std::time::SystemTime::now())
    }

    fn completion_time(time: std::time::SystemTime) -> CompletionDeadline {
        let millis = time
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        CompletionDeadline::from_unix_millis(millis)
    }

    fn reconciliation_deadline(now: CompletionDeadline) -> CompletionDeadline {
        CompletionDeadline::from_unix_millis(
            now.unix_millis()
                .saturating_add(COMPLETION_RECONCILIATION_TIMEOUT_SECS.saturating_mul(1_000)),
        )
    }

    fn dispatch_completion_at(
        &mut self,
        input: CompletionInput,
        now: std::time::SystemTime,
    ) -> bool {
        let now = Self::completion_time(now);
        let (ticket_id, source, authority, effect, completion_note) = match input {
            CompletionInput::Reconcile {
                ticket_id,
                source_lease,
            } => {
                let attempt_id = AttemptId::new(source_lease.attempt_id.to_string());
                let completion_id = CompletionId::new(ticket_id.clone());
                let correlation =
                    Self::completion_correlation(completion_id.clone(), attempt_id.clone());
                if !self.review_lease_is_current(&ticket_id, &source_lease) {
                    self.reject_stale_lease(
                        &ticket_id,
                        &correlation,
                        source_lease.attempt_id.to_string(),
                    );
                    return false;
                }

                let durable_inputs = self.review_completion_inputs(&ticket_id, &source_lease);
                let completion_note = match &durable_inputs.disposition {
                    ReviewDisposition::Note(note) => Some(note.clone()),
                    _ => None,
                };
                let state = self.reconciliation_state(&ticket_id);
                let effect = match reconcile_completion(&durable_inputs, &state, now) {
                    Reconciliation::Effect(effect) => Some(effect),
                    Reconciliation::None => None,
                    Reconciliation::ReplayCommandInFlight {
                        correlation,
                        deadline,
                    } => {
                        if self
                            .completion_aggregates
                            .get(&ticket_id)
                            .is_some_and(CompletionJournalAggregate::retries_exhausted)
                        {
                            return false;
                        }
                        return self.replay_in_flight_completion(
                            ticket_id,
                            CompletionSource::Reconcile,
                            CompletionAuthority::Attempt(source_lease),
                            correlation,
                            deadline,
                        );
                    }
                    Reconciliation::CommandInFlightDeadlineExceeded {
                        correlation,
                        deadline,
                    } => {
                        return self.expire_in_flight_completion(&ticket_id, correlation, deadline);
                    }
                };
                (
                    ticket_id,
                    CompletionSource::Reconcile,
                    Some(CompletionAuthority::Attempt(source_lease)),
                    effect,
                    completion_note,
                )
            }
            input => {
                let (ticket_id, source, authority, review_lease) = match input {
                    CompletionInput::Artifact {
                        ticket_id,
                        source_lease,
                    } => (
                        ticket_id,
                        CompletionSource::Artifact,
                        Some(CompletionAuthority::Attempt(source_lease.clone())),
                        Some(source_lease),
                    ),
                    CompletionInput::Stopped {
                        ticket_id,
                        pane_id,
                        source_lease,
                    } => (
                        ticket_id,
                        CompletionSource::Stopped(pane_id),
                        Some(CompletionAuthority::Attempt(source_lease.clone())),
                        Some(source_lease),
                    ),
                    CompletionInput::Idle {
                        ticket_id,
                        source_lease,
                    } => (
                        ticket_id,
                        CompletionSource::Idle,
                        Some(CompletionAuthority::Attempt(source_lease.clone())),
                        Some(source_lease),
                    ),
                    CompletionInput::ObservedDone {
                        ticket_id,
                        source_lease,
                    } => (
                        ticket_id,
                        CompletionSource::ObservedDone,
                        source_lease.map(CompletionAuthority::Attempt),
                        None,
                    ),
                    CompletionInput::OperatorRequested { ticket_id, source } => (
                        ticket_id,
                        CompletionSource::OperatorRequested(source),
                        Some(CompletionAuthority::Operator),
                        None,
                    ),
                    CompletionInput::Reconcile { .. } => unreachable!("handled above"),
                };

                // Event-driven sources preserve their existing semantics:
                // ObservedDone still enters the isolated transaction rather
                // than treating externally edited frontmatter as confirmation.
                let state = match self.reconciliation_state(&ticket_id) {
                    CompletionState::Rejected {
                        retryability: Retryability::ActionRequired,
                        ..
                    } if matches!(source, CompletionSource::OperatorRequested(_)) => {
                        CompletionState::Eligible
                    }
                    state => state,
                };
                let attempt_id = match authority.as_ref() {
                    Some(CompletionAuthority::Attempt(lease)) => lease.attempt_id.to_string(),
                    Some(CompletionAuthority::Operator) => "operator".to_string(),
                    None => "missing-authority".to_string(),
                };
                let attempt_id = AttemptId::new(attempt_id);
                let completion_id = CompletionId::new(ticket_id.clone());
                let correlation =
                    Self::completion_correlation(completion_id.clone(), attempt_id.clone());

                let mut completion_note = None;
                if matches!(source, CompletionSource::OperatorRequested(_)) {
                    match self.passing_review_disposition(&ticket_id) {
                        Ok(note) => completion_note = note,
                        Err(rejection) => {
                            self.log_completion_rejection(&ticket_id, &correlation, &rejection);
                            return false;
                        }
                    }
                }

                if let Some(review_lease) = review_lease.as_ref() {
                    completion_note = match self.admit_correlated_review(
                        &ticket_id,
                        review_lease,
                        &correlation,
                    ) {
                        Some(note) => note,
                        None => return false,
                    };
                }

                let event = CompletionEvent::Request {
                    attempt_id,
                    completion_id,
                };
                let effect = match reduce_completion(state, event) {
                    Ok(transition) => transition.effect,
                    Err(rejection) => {
                        self.log_completion_rejection(&ticket_id, &correlation, &rejection);
                        return false;
                    }
                };
                (ticket_id, source, authority, effect, completion_note)
            }
        };

        match effect {
            Some(effect) => self.execute_completion_effect(
                effect,
                ticket_id,
                source,
                authority,
                completion_note,
                now,
            ),
            None => false,
        }
    }

    /// Execute an inert core effect. This is the sole completion-command launch
    /// boundary; callers may validate evidence, but cannot launch directly.
    fn execute_completion_effect(
        &mut self,
        effect: EffectCommand,
        ticket_id: TicketId,
        source: CompletionSource,
        authority: Option<CompletionAuthority>,
        completion_note: Option<DispositionNote>,
        now: CompletionDeadline,
    ) -> bool {
        let (effect_attempt_id, effect_completion_id) = match &effect {
            EffectCommand::LaunchCompletion {
                attempt_id,
                completion_id,
            } => (attempt_id, completion_id),
        };
        let completion_key =
            Self::completion_correlation(effect_completion_id.clone(), effect_attempt_id.clone());
        let effect_matches_authority = effect_completion_id.as_str() == ticket_id
            && match authority.as_ref() {
                Some(CompletionAuthority::Attempt(lease)) => {
                    effect_attempt_id.as_str() == lease.attempt_id.to_string()
                }
                Some(CompletionAuthority::Operator) => true,
                None => false,
            };
        if !effect_matches_authority {
            self.log_activity(ActivityEvent::Warning {
                message: format!(
                    "Rejected completion effect for {ticket_id}: effect identity does not match source authority"
                ),
            });
            return false;
        }

        if self.pending_completions.contains_key(&ticket_id)
            || self
                .completion_aggregates
                .get(&ticket_id)
                .map(|aggregate| {
                    matches!(
                        aggregate.state(),
                        CompletionState::Requested | CompletionState::CommandInFlight { .. }
                    ) || (matches!(aggregate.state(), CompletionState::Confirmed)
                        && self
                            .dag
                            .get_ticket(&ticket_id)
                            .map(|ticket| {
                                ticket.phase == Phase::Done && ticket.status == TicketStatus::Done
                            })
                            .unwrap_or(false))
                })
                .unwrap_or(false)
        {
            return false;
        }
        let authority = match authority {
            Some(CompletionAuthority::Attempt(lease))
                if lease.is_current(self.current_leases.get(&ticket_id)) =>
            {
                CompletionAuthority::Attempt(lease)
            }
            Some(CompletionAuthority::Operator)
                if matches!(source, CompletionSource::OperatorRequested(_)) =>
            {
                CompletionAuthority::Operator
            }
            Some(CompletionAuthority::Attempt(lease)) => {
                self.reject_stale_lease(&ticket_id, &completion_key, lease.attempt_id.to_string());
                return false;
            }
            authority => {
                self.log_activity(ActivityEvent::Warning {
                    message: format!(
                        "Rejected completion for {ticket_id} ({source:?}): source authority {authority:?} does not hold the current lease"
                    ),
                });
                return false;
            }
        };
        if !self.dag.all_dependencies_done(&ticket_id) {
            self.log_completion_rejection(
                &ticket_id,
                &completion_key,
                &CompletionRejection::DependencyBlocked {
                    reason: "dependencies are not all done".to_string(),
                },
            );
            return false;
        }
        let (ticket_file, ticket_phase, ticket_status) = match self.dag.get_ticket(&ticket_id) {
            Some(ticket) if !ticket.file_path.as_os_str().is_empty() => {
                (ticket.file_path.clone(), ticket.phase, ticket.status)
            }
            _ => {
                self.log_activity(ActivityEvent::Error {
                    message: format!("Cannot find file for {} during completion", ticket_id),
                });
                return false;
            }
        };
        let prior_phase = self
            .threads
            .get(&ticket_id)
            .map(|thread| thread.current_phase)
            .filter(|phase| *phase != Phase::Done)
            .unwrap_or(ticket_phase);
        let prior_status = if prior_phase != Phase::Done && ticket_status == TicketStatus::Done {
            TicketStatus::Open
        } else {
            ticket_status
        };

        let command = if self.config.completion_seal == CompletionSeal::Commit {
            match self.build_completion_command(&completion_key, &ticket_file) {
                Ok(command) => Some(command),
                Err(error) => {
                    #[cfg(test)]
                    if self.completion_journal_path.as_os_str().is_empty() {
                        None
                    } else {
                        self.log_completion_rejection(
                            &ticket_id,
                            &completion_key,
                            &CompletionRejection::LaunchFailed {
                                source: LaunchFailure::new(error),
                            },
                        );
                        return false;
                    }
                    #[cfg(not(test))]
                    {
                        self.log_completion_rejection(
                            &ticket_id,
                            &completion_key,
                            &CompletionRejection::LaunchFailed {
                                source: LaunchFailure::new(error),
                            },
                        );
                        return false;
                    }
                }
            }
        } else {
            None
        };
        let correlation = CorrelationId::new(completion_key.to_string());
        let deadline = Self::reconciliation_deadline(now);

        if let Err(error) =
            self.journal_completion_transition(CompletionJournalTransition::Requested {
                key: completion_key.clone(),
                prior_phase,
                prior_status,
                note: completion_note.clone(),
            })
        {
            self.log_completion_rejection(
                &ticket_id,
                &completion_key,
                &CompletionRejection::LaunchFailed {
                    source: LaunchFailure::new(format!(
                        "could not persist completion request: {error}"
                    )),
                },
            );
            return false;
        }
        if let Err(error) =
            self.journal_completion_transition(CompletionJournalTransition::CommandInFlight {
                key: completion_key.clone(),
                correlation: correlation.clone(),
                deadline,
            })
        {
            self.log_completion_rejection(
                &ticket_id,
                &completion_key,
                &CompletionRejection::LaunchFailed {
                    source: LaunchFailure::new(format!(
                        "could not persist in-flight completion command: {error}"
                    )),
                },
            );
            return false;
        }

        self.pending_completions.insert(
            ticket_id.clone(),
            PendingCompletion {
                completion_key: completion_key.clone(),
                correlation,
                deadline,
                is_reconciliation_replay: false,
                prior_phase,
                prior_status,
                source,
                authority,
                completion_note,
            },
        );

        #[cfg(test)]
        self.launched_completion_effects.push(effect);

        if self.config.completion_seal == CompletionSeal::Journal {
            self.complete_pending_journal_seal(&ticket_id);
            return true;
        }
        let Some((argv, context)) = command else {
            return true;
        };
        self.launch_completion_host_command(&argv, context);
        self.log_activity(ActivityEvent::Info {
            message: format!("Completion commit pending for {ticket_id} ({source:?})"),
        });
        true
    }

    /// Relaunch the exact durable generation after a result was lost. The
    /// existing absolute deadline is retained and no journal transition is
    /// appended merely for replaying an idempotent host command.
    fn replay_in_flight_completion(
        &mut self,
        ticket_id: TicketId,
        source: CompletionSource,
        authority: CompletionAuthority,
        correlation: CorrelationId,
        deadline: CompletionDeadline,
    ) -> bool {
        if self.pending_completions.contains_key(&ticket_id) {
            return false;
        }
        let authority_is_current = match &authority {
            CompletionAuthority::Attempt(lease) => self.review_lease_is_current(&ticket_id, lease),
            CompletionAuthority::Operator => {
                matches!(source, CompletionSource::OperatorRequested(_))
            }
        };
        if !authority_is_current {
            return false;
        }

        let (completion_key, prior_phase, prior_status) =
            match self.completion_aggregates.get(&ticket_id) {
                Some(aggregate)
                    if !aggregate.retries_exhausted()
                        && match &authority {
                            CompletionAuthority::Attempt(lease) => {
                                aggregate.completion_key().attempt_id().as_str()
                                    == lease.attempt_id.to_string()
                            }
                            CompletionAuthority::Operator => {
                                aggregate.completion_key().attempt_id().as_str() == "operator"
                            }
                        }
                        && matches!(
                            aggregate.state(),
                            CompletionState::CommandInFlight {
                                correlation: stored_correlation,
                                deadline: stored_deadline,
                            } if stored_correlation == &correlation && stored_deadline == &deadline
                        ) =>
                {
                    (
                        aggregate.completion_key().clone(),
                        aggregate.prior_phase(),
                        aggregate.prior_status(),
                    )
                }
                _ => return false,
            };
        let ticket_file = match self.dag.get_ticket(&ticket_id) {
            Some(ticket) if !ticket.file_path.as_os_str().is_empty() => ticket.file_path.clone(),
            _ => return false,
        };
        let command = if self.config.completion_seal == CompletionSeal::Commit {
            match self.build_completion_command(&completion_key, &ticket_file) {
                Ok(command) => Some(command),
                Err(error) => {
                    self.log_activity(ActivityEvent::Warning {
                        message: format!(
                            "Completion replay could not launch for {ticket_id} correlation {correlation}: {error}"
                        ),
                    });
                    return false;
                }
            }
        } else {
            None
        };

        self.pending_completions.insert(
            ticket_id.clone(),
            PendingCompletion {
                completion_key: completion_key.clone(),
                correlation: correlation.clone(),
                deadline,
                is_reconciliation_replay: true,
                prior_phase,
                prior_status,
                source,
                authority,
                completion_note: self
                    .completion_aggregates
                    .get(&ticket_id)
                    .and_then(CompletionJournalAggregate::completion_note)
                    .cloned(),
            },
        );

        #[cfg(test)]
        self.launched_completion_effects
            .push(EffectCommand::LaunchCompletion {
                attempt_id: completion_key.attempt_id().clone(),
                completion_id: completion_key.completion_id().clone(),
            });

        if self.config.completion_seal == CompletionSeal::Journal {
            self.complete_pending_journal_seal(&ticket_id);
            return true;
        }
        let Some((argv, context)) = command else {
            return false;
        };
        self.launch_completion_host_command(&argv, context);
        self.log_activity(ActivityEvent::Info {
            message: format!(
                "Completion reconciliation replay pending for {ticket_id} correlation {correlation}"
            ),
        });
        true
    }

    /// End an unresolved generation in a durable, named state once its shared
    /// reconciliation window has elapsed.
    fn park_failed_completion(
        &mut self,
        ticket_id: &str,
        completion_key: CompletionGenerationId,
        correlation: Option<CorrelationId>,
        technical_reason: String,
        class: CompletionFailureClass,
        retry_progress: Option<(u8, u8)>,
    ) -> bool {
        let (prior_phase, ticket_file) = match (
            self.completion_aggregates.get(ticket_id),
            self.dag.get_ticket(&ticket_id.to_string()),
        ) {
            (Some(aggregate), Some(ticket))
                if aggregate.completion_key() == &completion_key
                    && !ticket.file_path.as_os_str().is_empty() =>
            {
                (aggregate.prior_phase(), ticket.file_path.clone())
            }
            _ => return false,
        };
        if let Err(error) =
            self.journal_completion_transition(CompletionJournalTransition::Rejected {
                key: completion_key.clone(),
                correlation,
                reason: technical_reason.clone(),
                retryability: Retryability::ActionRequired,
            })
        {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Completion for {ticket_id} could not be parked because its rejection was not persisted: {error}"
                ),
            });
            return false;
        }

        let ask =
            completion_failure_ask(class, ticket_id).unwrap_or_else(|| technical_reason.clone());
        let disposition = match completion_failure_ask(class, ticket_id) {
            Some(structured_ask) => serde_json::json!({
                "disposition": "block",
                "reason": technical_reason.clone(),
                "remedy_owner": "operator",
                "ask": structured_ask,
            }),
            None => serde_json::json!({
                "disposition": "block",
                "reason": technical_reason.clone(),
            }),
        };
        let disposition_bytes = match serde_json::to_vec(&disposition) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.log_activity(ActivityEvent::Error {
                    message: format!(
                        "Completion for {ticket_id} is waiting, but its recovery ask could not be serialized: {error}"
                    ),
                });
                return false;
            }
        };
        let disposition_path = self
            .config
            .work_dir
            .join(ticket_id)
            .join("review-disposition.json");
        if let Err(error) = (RustPublication {
            path: PublicationPath {
                destination: disposition_path,
                temporary_name: TemporaryName::Nonce {
                    prefix: ".review-disposition.json.tmp.".to_string(),
                },
            },
            body: &disposition_bytes,
            errors: PublicationErrors {
                write: "cannot write completion recovery disposition",
                publish: "cannot publish completion recovery disposition",
            },
        })
        .publish()
        {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Completion for {ticket_id} is waiting, but its recovery ask could not be published: {error}"
                ),
            });
            return false;
        }

        if let Err(error) = ticket::update_ticket_phase(&ticket_file, prior_phase) {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Completion for {ticket_id} is waiting, but Review state could not be restored: {error}"
                ),
            });
            return false;
        }
        if let Err(error) = ticket::update_ticket_status(&ticket_file, TicketStatus::Blocked) {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Completion for {ticket_id} is waiting, but blocked status could not be written: {error}"
                ),
            });
            return false;
        }

        let parked_at = std::time::SystemTime::now();
        self.emit_review_block_transition(
            ticket_id,
            RemedyOwner::Operator,
            ParkingTransitionType::Park,
            retry_progress,
            false,
            parked_at,
        );
        self.pending_completions.remove(ticket_id);
        self.release_slot_for_ticket(&ticket_id.to_string());
        self.threads.remove(ticket_id);
        self.finish_up_sent.remove(ticket_id);
        self.rebuild_dag();
        self.log_activity(ActivityEvent::Info {
            message: format!("{ask} [{technical_reason}]"),
        });
        true
    }

    fn expire_in_flight_completion(
        &mut self,
        ticket_id: &str,
        correlation: CorrelationId,
        deadline: CompletionDeadline,
    ) -> bool {
        let completion_key = match self.completion_aggregates.get(ticket_id) {
            Some(aggregate)
                if matches!(
                    aggregate.state(),
                    CompletionState::CommandInFlight {
                        correlation: stored_correlation,
                        deadline: stored_deadline,
                    } if stored_correlation == &correlation && stored_deadline == &deadline
                ) =>
            {
                aggregate.completion_key().clone()
            }
            _ => return false,
        };
        let reason = format!(
            "completion reconciliation deadline {} exceeded for correlation {correlation}",
            deadline.unix_millis()
        );
        self.park_failed_completion(
            ticket_id,
            completion_key,
            Some(correlation),
            reason,
            CompletionFailureClass::DeadlineExpired,
            None,
        )
    }

    /// The single host-command crossing used by both a new effect and an
    /// idempotent reconciliation replay.
    fn launch_completion_host_command(&self, argv: &[String], context: BTreeMap<String, String>) {
        let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        run_command_with_env_variables_and_cwd(
            &argv_refs,
            BTreeMap::new(),
            self.project_root.clone(),
            context,
        );
    }

    fn is_commit_id(output: &[u8]) -> bool {
        let value = String::from_utf8_lossy(output);
        let value = value.trim();
        matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    }

    fn complete_pending_journal_seal(&mut self, ticket_id: &str) -> bool {
        let Some(pending) = self.pending_completions.get(ticket_id).cloned() else {
            return false;
        };
        let ticket_file = match self.dag.get_ticket(&ticket_id.to_string()) {
            Some(ticket) if !ticket.file_path.as_os_str().is_empty() => ticket.file_path.clone(),
            _ => return false,
        };
        let work_dir = self.config.work_dir.join(ticket_id);
        match completion_journal::complete_with_journal_seal(
            &self.project_root,
            &ticket_file,
            &work_dir,
        ) {
            Ok(receipt) => self.finish_successful_completion(ticket_id, pending, receipt),
            Err(error) => {
                self.rebuild_dag();
                self.log_completion_rejection(
                    ticket_id,
                    &pending.completion_key,
                    &CompletionRejection::LaunchFailed {
                        source: LaunchFailure::new(format!(
                            "Lisa could not create the journal seal for {ticket_id}. [{error}]"
                        )),
                    },
                );
                false
            }
        }
    }

    fn finish_successful_completion(
        &mut self,
        ticket_id: &str,
        pending: PendingCompletion,
        receipt: CompletionSealReceipt,
    ) -> bool {
        if receipt.seal() != self.config.completion_seal {
            self.rebuild_dag();
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Completion evidence for {ticket_id} used {} under pinned {} seal; scheduler state remains blocked",
                    receipt.seal(),
                    self.config.completion_seal
                ),
            });
            return false;
        }
        let durable_done = ticket::scan_tickets(&self.config.ticket_dir)
            .ok()
            .and_then(|tickets| tickets.into_iter().find(|ticket| ticket.id == ticket_id))
            .map(|ticket| ticket.phase == Phase::Done && ticket.status == TicketStatus::Done)
            .unwrap_or(false);
        if !durable_done {
            self.rebuild_dag();
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Completion {} succeeded for {} but durable Done frontmatter could not be verified; scheduler state remains blocked",
                    receipt.seal(),
                    ticket_id
                ),
            });
            return false;
        }

        let success_message = match &receipt {
            CompletionSealReceipt::Commit { commit_id } => format!(
                "Completion commit verified for {} authority {:?} at {}",
                ticket_id, pending.authority, commit_id
            ),
            CompletionSealReceipt::Journal { content_hashes } => format!(
                "Journal seal verified for {} authority {:?} with {} content hashes",
                ticket_id,
                pending.authority,
                content_hashes.len()
            ),
        };
        let operator_modal_correlation =
            matches!(pending.source, CompletionSource::OperatorRequested(_))
                .then(|| pending.completion_key.to_string());
        if let Err(error) =
            self.journal_completion_transition(CompletionJournalTransition::Confirmed {
                key: pending.completion_key.clone(),
                correlation: pending.correlation.clone(),
                receipt,
                note: pending.completion_note.clone(),
            })
        {
            self.rebuild_dag();
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Completion publication succeeded for {ticket_id} but confirmation could not be persisted: {error}; scheduler state remains blocked"
                ),
            });
            return false;
        }

        if let Some(correlation_id) = operator_modal_correlation {
            self.show_operator_modal_accepted(ticket_id, correlation_id);
        }
        self.pending_completions.remove(ticket_id);
        self.rebuild_dag();

        self.log_activity(ActivityEvent::PhaseCompleted {
            ticket_id: ticket_id.to_string(),
            phase: pending.prior_phase,
        });
        self.log_activity(ActivityEvent::TicketPhaseChanged {
            ticket_id: ticket_id.to_string(),
            old_phase: pending.prior_phase,
            new_phase: Phase::Done,
        });
        self.log_activity(ActivityEvent::Info {
            message: success_message,
        });
        if let Some(thread) = self.threads.get_mut(ticket_id) {
            thread.complete();
        }
        self.emit_provenance_with_note(ticket_id, RunOutcome::Done, false, pending.completion_note);
        self.release_completed_slot_for_ticket(&ticket_id.to_string());
        self.threads.remove(ticket_id);
        self.schedule_ready_tickets();
        true
    }

    fn handle_completion_result(
        &mut self,
        ticket_id: &str,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    ) {
        let pending = match self.pending_completions.get(ticket_id).cloned() {
            Some(pending) => pending,
            None => return,
        };
        let completion_key = pending.completion_key.clone();
        let correlation = pending.correlation.clone();
        let authority_is_current = match &pending.authority {
            CompletionAuthority::Attempt(lease) => {
                lease.is_current(self.current_leases.get(ticket_id))
            }
            CompletionAuthority::Operator => {
                matches!(pending.source, CompletionSource::OperatorRequested(_))
            }
        };
        if !authority_is_current {
            let reason = format!(
                "completion result authority {:?} is no longer current",
                pending.authority
            );
            if let Err(error) =
                self.journal_completion_transition(CompletionJournalTransition::Rejected {
                    key: completion_key.clone(),
                    correlation: Some(correlation.clone()),
                    reason,
                    retryability: Retryability::Retryable,
                })
            {
                self.log_activity(ActivityEvent::Error {
                    message: format!(
                        "Completion result for {ticket_id} remains in-flight because rejection could not be persisted: {error}"
                    ),
                });
                return;
            }
            self.pending_completions.remove(ticket_id);
            self.rebuild_dag();
            match &pending.authority {
                CompletionAuthority::Attempt(_) => {
                    self.reject_stale_lease(
                        ticket_id,
                        &completion_key,
                        completion_key.attempt_id().to_string(),
                    );
                }
                CompletionAuthority::Operator => {
                    self.log_activity(ActivityEvent::Warning {
                        message: format!(
                            "Rejected completion result for {}: pending authority {:?} is no longer current",
                            ticket_id, pending.authority
                        ),
                    });
                }
            }
            return;
        }
        if exit_code != Some(0) || !Self::is_commit_id(&stdout) {
            let detail = String::from_utf8_lossy(&stderr);
            let failure = format!(
                "Completion commit failed for {} authority {:?} ({:?}, exit {:?}): {}",
                ticket_id,
                pending.authority,
                pending.source,
                exit_code,
                if detail.trim().is_empty() {
                    "no error output"
                } else {
                    detail.trim()
                }
            );
            let Some(aggregate) = self.completion_aggregates.get(ticket_id) else {
                if let Err(error) =
                    self.journal_completion_transition(CompletionJournalTransition::Rejected {
                        key: completion_key.clone(),
                        correlation: Some(correlation),
                        reason: failure.clone(),
                        retryability: Retryability::Retryable,
                    })
                {
                    self.log_activity(ActivityEvent::Error {
                        message: format!(
                            "Completion result for {ticket_id} remains in-flight because failure could not be persisted: {error}"
                        ),
                    });
                    return;
                }
                self.pending_completions.remove(ticket_id);
                self.rebuild_dag();
                self.log_completion_rejection(
                    ticket_id,
                    &completion_key,
                    &CompletionRejection::LaunchFailed {
                        source: LaunchFailure::new(format!(
                            "Lisa could not record finished work. [{failure}. Ticket remains recoverable for retry]"
                        )),
                    },
                );
                return;
            };
            let failure_count = aggregate.failure_count().saturating_add(1);
            let class = classify_completion_failure(detail.trim());
            let action = completion_failure_action(class, failure_count);
            let consequence = match action {
                CompletionFailureAction::Retry => FailureConsequence::RetryScheduled,
                CompletionFailureAction::WaitForDeadline => FailureConsequence::RetryExhausted,
                CompletionFailureAction::Park => FailureConsequence::Park,
            };
            if let Err(error) =
                self.journal_completion_transition(CompletionJournalTransition::FailureObserved {
                    key: completion_key.clone(),
                    correlation: correlation.clone(),
                    reason: failure.clone(),
                    class,
                    failure_count,
                    failure_limit: MAX_COMPLETION_FAILURES,
                    consequence,
                })
            {
                self.log_activity(ActivityEvent::Error {
                    message: format!(
                        "Completion result for {ticket_id} remains in-flight because failure could not be persisted: {error}"
                    ),
                });
                return;
            }
            let ask = completion_failure_ask(class, ticket_id).unwrap_or_else(|| failure.clone());
            let surface_detail = if ask == failure {
                ask
            } else {
                format!("{ask} [{failure}]")
            };
            self.log_completion_rejection(
                ticket_id,
                &completion_key,
                &CompletionRejection::LaunchFailed {
                    source: LaunchFailure::new(surface_detail),
                },
            );
            match action {
                CompletionFailureAction::Retry => {
                    self.pending_completions.remove(ticket_id);
                    if !self.replay_in_flight_completion(
                        ticket_id.to_string(),
                        pending.source,
                        pending.authority,
                        correlation,
                        pending.deadline,
                    ) {
                        self.log_activity(ActivityEvent::Error {
                            message: format!(
                                "Completion retry {failure_count}/{MAX_COMPLETION_FAILURES} could not launch for {ticket_id}"
                            ),
                        });
                    }
                }
                CompletionFailureAction::WaitForDeadline => {
                    self.pending_completions.remove(ticket_id);
                    self.log_activity(ActivityEvent::Info {
                        message: format!(
                            "Completion retries exhausted for {ticket_id} ({failure_count}/{MAX_COMPLETION_FAILURES}); waiting for reconciliation deadline {}",
                            pending.deadline.unix_millis()
                        ),
                    });
                }
                CompletionFailureAction::Park => {
                    self.park_failed_completion(
                        ticket_id,
                        completion_key,
                        Some(correlation),
                        failure,
                        class,
                        Some((failure_count, MAX_COMPLETION_FAILURES)),
                    );
                }
            }
            return;
        }

        let commit_id = String::from_utf8_lossy(&stdout).trim().to_string();
        let receipt = CompletionSealReceipt::commit(commit_id)
            .expect("validated completion commit output must produce a receipt");
        self.finish_successful_completion(ticket_id, pending, receipt);
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
        let mut tickets = match ticket::scan_tickets(&self.config.ticket_dir) {
            Ok(tickets) => tickets,
            Err(e) => {
                self.log_activity(ActivityEvent::Error {
                    message: format!("Failed to scan tickets: {}", e),
                });
                return false;
            }
        };

        for scanned in &mut tickets {
            self.mask_completion_transaction(scanned);
        }

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

        let mut discovered_panes = Vec::new();
        for panes in pane_manifest.panes.values() {
            for pane in panes {
                if !pane.is_plugin {
                    discovered_panes.push(pane.id);
                    self.agent_slots.push(AgentSlot {
                        pane_id: pane.id,
                        ticket_id: None,
                        attempt_lease: None,
                        has_session: false,
                        transition_state: TransitionState::Idle,
                        transition_started_at: None,
                        cooldown_until: None,
                        last_activity_at: None,
                        last_client: None,
                    });
                }
            }
        }

        if self.permissions_granted && !discovered_panes.is_empty() {
            self.name_unnamed_idle_slots();
        }

        if !self.agent_slots.is_empty() {
            self.slots_discovered = true;
            self.log_activity(ActivityEvent::Info {
                message: format!("Discovered {} agent pane slots", self.agent_slots.len()),
            });
        }
    }

    /// Return the explicit assignment state for a physical seat.
    fn seat_assignment(&self, pane_id: u32) -> Option<SeatAssignmentState> {
        self.seat_assignments.get(&pane_id).copied()
    }

    /// The provider bootstrap-readiness mode recorded for this pane at its last
    /// launch dispatch, if any (T-037-01-01). The behavioural consumer — the
    /// Codex startup-grace transition — arrives in T-037-01-02.
    #[cfg_attr(not(test), allow(dead_code))]
    fn seat_readiness_mode(&self, pane_id: u32) -> Option<ReadinessMode> {
        self.seat_readiness.get(&pane_id).copied()
    }

    /// Mark a fresh provider ready only when it reports the exact current
    /// attempt lease installed for this pane. Assignment delivery and ownership
    /// are separate later transitions.
    fn acknowledge_process_start(&mut self, pane_id: u32, candidate: &AttemptLease) -> bool {
        let Some(SeatAssignmentState::Starting { generation, .. }) = self.seat_assignment(pane_id)
        else {
            return false;
        };
        if generation != candidate.attempt_id {
            return false;
        }
        let Some((ticket_id, attempt_lease)) = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| Some((slot.ticket_id.as_ref()?, slot.attempt_lease.as_ref()?)))
        else {
            return false;
        };
        if ticket_id != &candidate.ticket_id
            || attempt_lease != candidate
            || !candidate.is_current(self.current_leases.get(&candidate.ticket_id))
        {
            return false;
        }

        self.seat_assignments.insert(
            pane_id,
            SeatAssignmentState::ReadyForAssignment { generation },
        );
        true
    }

    /// Compute the finite acceptance deadline from actual pane submission.
    fn assignment_ack_deadline(&self, now: std::time::SystemTime) -> std::time::SystemTime {
        let wait = std::time::Duration::from_secs(self.config.assignment_ack_timeout_secs)
            .saturating_add(std::time::Duration::from_secs_f64(ENTER_DELAY_SECS));
        now.checked_add(wait).unwrap_or_else(|| {
            now + std::time::Duration::from_secs(PluginConfig::DEFAULT_ASSIGNMENT_ACK_TIMEOUT_SECS)
                + std::time::Duration::from_secs_f64(ENTER_DELAY_SECS)
        })
    }

    /// The absolute deadline at which a grace-mode seat's bounded startup grace
    /// elapses and its paced first prompt is attempted. Saturating on overflow.
    fn startup_grace_deadline(&self, now: std::time::SystemTime) -> std::time::SystemTime {
        now.checked_add(std::time::Duration::from_secs(STARTUP_GRACE_SECS))
            .unwrap_or(now)
    }

    /// Submit the bounded assignment-file reference for one exact ready or
    /// retrying attempt and enter the common Delivering state.
    fn deliver_assignment_to_pane(
        &mut self,
        pane_id: u32,
        generation: u64,
        retries: u8,
        now: std::time::SystemTime,
    ) -> Result<(), String> {
        if self.is_pane_awaiting(pane_id) {
            return Err("provider is awaiting human input".to_string());
        }
        let Some((ticket_id, lease)) = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| Some((slot.ticket_id.clone()?, slot.attempt_lease.clone()?)))
        else {
            return Err("ticket reservation is missing".to_string());
        };
        if lease.ticket_id != ticket_id
            || lease.attempt_id != generation
            || !lease.is_current(self.current_leases.get(&ticket_id))
        {
            return Err("current attempt lease is missing or stale".to_string());
        }
        let assignment = self
            .assignment_refs
            .get(&ticket_id)
            .cloned()
            .ok_or_else(|| format!("assignment reference for {ticket_id} is missing"))?;
        if assignment.lease != lease {
            return Err(format!(
                "assignment reference for {ticket_id} belongs to a stale attempt"
            ));
        }
        let assignment_path = assignment.path;
        if !assignment_path.is_file() {
            return Err(format!(
                "assignment file {} is missing",
                assignment_path.display()
            ));
        }

        let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);
        let artifact_dir = strip_host_prefix(&self.attempt_work_dir(&lease));
        let chat_assignment_path = strip_host_prefix(&assignment_path);
        let (adapter, _) = resolve_adapter_or_native(
            self.dag.get_ticket(&ticket_id),
            self.config.client,
            self.config.lisa_bin.as_deref(),
        );
        let ctx = SpawnContext {
            ticket_dir: &host_ticket_dir,
            ticket_id: &ticket_id,
            pane_id,
            attempt_id: generation,
            artifact_dir: &artifact_dir,
            assignment_generation: Some(generation),
        };
        let message = adapter.assignment_reference(&ctx, &chat_assignment_path);
        self.send_line_to_pane(&message, PaneId::Terminal(pane_id));
        self.seat_assignments.insert(
            pane_id,
            SeatAssignmentState::Delivering {
                generation,
                ack_deadline: self.assignment_ack_deadline(now),
                retries,
            },
        );
        self.log_activity(ActivityEvent::Info {
            message: format!(
                "Pane {} delivering assignment for {} (attempt {}, retry {})",
                pane_id, ticket_id, generation, retries
            ),
        });
        Ok(())
    }

    /// Deliver only assignments that were already ready at the beginning of
    /// this poll. Process-start signals are consumed later, leaving readiness
    /// observable for one scheduler boundary.
    fn deliver_ready_assignments(&mut self) {
        let ready: Vec<(u32, u64)> = self
            .seat_assignments
            .iter()
            .filter_map(|(pane_id, state)| match state {
                SeatAssignmentState::ReadyForAssignment { generation } => {
                    Some((*pane_id, *generation))
                }
                _ => None,
            })
            .collect();
        let now = std::time::SystemTime::now();
        for (pane_id, generation) in ready {
            if self.seat_assignment(pane_id)
                != Some(SeatAssignmentState::ReadyForAssignment { generation })
            {
                continue;
            }
            if let Err(error) = self.deliver_assignment_to_pane(pane_id, generation, 0, now) {
                self.fail_assignment_delivery(pane_id, &error);
            }
        }
    }

    /// Return the expected lease attempt ID while an unowned Codex attempt can
    /// still be acknowledged. Original and recovery attempts have distinct
    /// leases.
    fn active_assignment_generation(&self, pane_id: u32) -> Option<u64> {
        match self.seat_assignment(pane_id) {
            Some(
                SeatAssignmentState::Delivering { generation, .. }
                | SeatAssignmentState::DeliveredAwaitingClaim { generation, .. }
                | SeatAssignmentState::AssignedPendingAck { generation, .. }
                | SeatAssignmentState::Recovering { generation, .. },
            ) => Some(generation),
            _ => None,
        }
    }

    /// Whether the addressed delivery still belongs to a live, current Codex
    /// TUI. Only this path may replace an active retry with passive claim wait;
    /// Claude and missing/stale sessions retain delivery-failure semantics.
    fn is_live_codex_delivery(&self, pane_id: u32, generation: u64) -> bool {
        self.agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .is_some_and(|slot| {
                let (Some(ticket_id), Some(lease)) =
                    (slot.ticket_id.as_ref(), slot.attempt_lease.as_ref())
                else {
                    return false;
                };
                slot.has_session
                    && slot.last_client == Some(AgentClient::Codex)
                    && lease.ticket_id == *ticket_id
                    && lease.attempt_id == generation
                    && lease.is_current(self.current_leases.get(ticket_id))
            })
    }

    /// Start the finite provider-acceptance clock after an actual fresh launch
    /// or tagged prompt delivery. Transport-only `/clear` and `/exit` steps
    /// deliberately leave it unarmed so a reservation cannot expire before the
    /// provider receives its input.
    fn start_assignment_ack_wait(&mut self, pane_id: u32, now: std::time::SystemTime) -> bool {
        // `send_line_to_pane` has typed the prompt, but its Enter is deliberately
        // deferred. Add that transport delay so even the minimum configured wait
        // begins after submission rather than expiring while text is unsubmitted.
        let deadline = self.assignment_ack_deadline(now);
        let Some(current) = self.seat_assignment(pane_id) else {
            return false;
        };
        let next = match current {
            SeatAssignmentState::Starting {
                generation,
                start_deadline: None,
                relaunches,
            } => {
                // Grace-mode (Codex) paces its first prompt after a bounded
                // startup grace; SessionStart-mode (Claude) bounds the wait for
                // its exact process-start signal with the acceptance clock.
                let start_deadline = Some(
                    if self.seat_readiness_mode(pane_id) == Some(ReadinessMode::Grace) {
                        self.startup_grace_deadline(now)
                    } else {
                        deadline
                    },
                );
                SeatAssignmentState::Starting {
                    generation,
                    start_deadline,
                    relaunches,
                }
            }
            SeatAssignmentState::AssignedPendingAck {
                generation,
                ack_deadline: None,
            } => SeatAssignmentState::AssignedPendingAck {
                generation,
                ack_deadline: Some(deadline),
            },
            SeatAssignmentState::Recovering {
                generation,
                ack_deadline: None,
            } => SeatAssignmentState::Recovering {
                generation,
                ack_deadline: Some(deadline),
            },
            _ => return false,
        };
        self.seat_assignments.insert(pane_id, next);
        true
    }

    /// Promote an acknowledgment-gated seat only when the provider payload
    /// identifies the exact current attempt lease pending in that pane.
    /// Returning true means this call performed the one pending-to-owned edge.
    fn acknowledge_codex_assignment(&mut self, pane_id: u32, payload_json: &str) -> bool {
        if self.seat_is_owned(pane_id) {
            return false;
        }
        let Some(generation) = self.active_assignment_generation(pane_id) else {
            return false;
        };
        let Some((ticket_id, attempt_lease)) = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| Some((slot.ticket_id.clone()?, slot.attempt_lease.clone()?)))
        else {
            return false;
        };
        if attempt_lease.ticket_id != ticket_id
            || attempt_lease.attempt_id != generation
            || !attempt_lease.is_current(self.current_leases.get(&ticket_id))
        {
            return false;
        }
        let pending = codex_ack::CodexAssignmentRef {
            ticket_id: &ticket_id,
            generation: attempt_lease.attempt_id,
        };
        if !codex_ack::detect_codex_ack(payload_json, pending) {
            return false;
        }

        self.seat_assignments
            .insert(pane_id, SeatAssignmentState::Owned);
        true
    }

    /// Promote a delivered assignment only when a pane-routed claim identifies
    /// the exact scheduler-retained current lease and assignment nonce.
    /// Returning true means this call performed the one pending-to-owned edge.
    fn admit_assignment_claim(&mut self, pane_id: u32, claim: &AssignmentClaim) -> bool {
        if self.seat_is_owned(pane_id) {
            return false;
        }
        let Some(generation) = self.active_assignment_generation(pane_id) else {
            return false;
        };
        let Some((ticket_id, attempt_lease)) = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| Some((slot.ticket_id.as_ref()?, slot.attempt_lease.as_ref()?)))
        else {
            return false;
        };
        if claim.ticket_id != *ticket_id
            || claim.attempt_id != generation
            || attempt_lease.ticket_id != *ticket_id
            || attempt_lease.attempt_id != claim.attempt_id
            || !attempt_lease.is_current(self.current_leases.get(ticket_id))
        {
            return false;
        }
        let Some(assignment) = self.assignment_refs.get(ticket_id) else {
            return false;
        };
        if assignment.lease != *attempt_lease || assignment.nonce != claim.nonce {
            return false;
        }

        self.seat_assignments
            .insert(pane_id, SeatAssignmentState::Owned);
        true
    }

    /// Promote a delivered assignment from recognized workflow output only
    /// after the caller has admitted that output from this exact current
    /// attempt's private work directory. Returning the pane identifies the
    /// one pending-to-owned edge for activity bookkeeping.
    fn admit_artifact_ownership(
        &mut self,
        ticket_id: &str,
        candidate: &AttemptLease,
    ) -> Option<u32> {
        if candidate.ticket_id != ticket_id
            || !candidate.is_current(self.current_leases.get(ticket_id))
        {
            return None;
        }
        let pane_id = self
            .agent_slots
            .iter()
            .find(|slot| {
                slot.ticket_id.as_deref() == Some(ticket_id)
                    && slot.attempt_lease.as_ref() == Some(candidate)
            })?
            .pane_id;
        if self.active_assignment_generation(pane_id) != Some(candidate.attempt_id) {
            return None;
        }

        self.seat_assignments
            .insert(pane_id, SeatAssignmentState::Owned);
        Some(pane_id)
    }

    /// Record the weaker, bounded ownership fallback offered by one artifact
    /// that has already crossed `admit_artifact` successfully.
    fn record_artifact_ownership(
        &mut self,
        ticket_id: &str,
        candidate: Option<&AttemptLease>,
        artifact_name: &str,
    ) {
        let Some(candidate) = candidate else {
            return;
        };
        let Some(pane_id) = self.admit_artifact_ownership(ticket_id, candidate) else {
            return;
        };

        self.bump_pane_activity(pane_id);
        self.log_activity(ActivityEvent::Info {
            message: format!(
                "Pane {} established ownership of {} attempt {} from current-attempt {}",
                pane_id, ticket_id, candidate.attempt_id, artifact_name
            ),
        });
    }

    /// Retain a started-but-unassigned provider as an explicit terminal failure
    /// after bounded chat delivery is exhausted.
    fn fail_assignment_delivery(
        &mut self,
        pane_id: u32,
        reason: &str,
    ) -> Option<FailureTransitionOutcome> {
        if !matches!(
            self.seat_assignment(pane_id),
            Some(
                // Starting is accepted so a grace-mode paced send that cannot be
                // submitted resolves in a named terminal state (E-037) rather
                // than silently remaining Starting.
                SeatAssignmentState::Starting { .. }
                    | SeatAssignmentState::ReadyForAssignment { .. }
                    | SeatAssignmentState::Delivering { .. }
            )
        ) {
            return None;
        }
        self.seat_assignments
            .insert(pane_id, SeatAssignmentState::DeliveryFailed);
        let ticket_id = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| slot.ticket_id.clone());
        let Some(ticket_id) = ticket_id else {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Assignment delivery failed on pane {}: {}; reset after repairing the reservation",
                    pane_id, reason
                ),
            });
            return Some(FailureTransitionOutcome::AssignmentDeliveryFailed {
                pane_id,
                ticket_id: None,
            });
        };
        if let Some(thread) = self.threads.get_mut(&ticket_id) {
            thread.fail();
        }
        self.emit_assignment_transition(
            pane_id,
            &ticket_id,
            AssignmentState::DeliveryFailed,
            reason,
        );
        if !self
            .error_alerts
            .iter()
            .any(|(existing, existing_pane)| existing == &ticket_id && *existing_pane == pane_id)
        {
            self.error_alerts.push((ticket_id.clone(), pane_id));
        }
        self.log_activity(ActivityEvent::Error {
            message: format!(
                "{} assignment delivery failed on pane {}: {}; reset the ticket to retry",
                ticket_id, pane_id, reason
            ),
        });
        Some(FailureTransitionOutcome::AssignmentDeliveryFailed {
            pane_id,
            ticket_id: Some(ticket_id),
        })
    }

    /// End a passive delivered-claim wait without misreporting successful
    /// transport as delivery failure. Retain the reservation and current lease
    /// for pane inspection and an explicit operator reset.
    fn fail_assignment_claim_wait(
        &mut self,
        pane_id: u32,
        reason: &str,
    ) -> Option<FailureTransitionOutcome> {
        if !matches!(
            self.seat_assignment(pane_id),
            Some(SeatAssignmentState::DeliveredAwaitingClaim { .. })
        ) {
            return None;
        }
        self.seat_assignments
            .insert(pane_id, SeatAssignmentState::ClaimTimedOut);
        let ticket_id = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| slot.ticket_id.clone());
        let Some(ticket_id) = ticket_id else {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Assignment claim timed out on pane {}: {}; reset after repairing the reservation",
                    pane_id, reason
                ),
            });
            return Some(FailureTransitionOutcome::AssignmentClaimTimedOut {
                pane_id,
                ticket_id: None,
            });
        };
        if let Some(thread) = self.threads.get_mut(&ticket_id) {
            thread.fail();
        }
        self.emit_assignment_transition(
            pane_id,
            &ticket_id,
            AssignmentState::ClaimTimedOut,
            reason,
        );
        if !self
            .error_alerts
            .iter()
            .any(|(existing, existing_pane)| existing == &ticket_id && *existing_pane == pane_id)
        {
            self.error_alerts.push((ticket_id.clone(), pane_id));
        }
        self.log_activity(ActivityEvent::Error {
            message: format!(
                "{} delivered assignment was not claimed on pane {}: {}; inspect the pane and reset the ticket to retry",
                ticket_id, pane_id, reason
            ),
        });
        Some(FailureTransitionOutcome::AssignmentClaimTimedOut {
            pane_id,
            ticket_id: Some(ticket_id),
        })
    }

    /// End the one permitted recovery attempt without releasing the reservation
    /// back into automatic scheduling. The operator can inspect the named state
    /// and use the existing ticket reset action to authorize another attempt.
    fn fail_assignment_recovery(
        &mut self,
        pane_id: u32,
        reason: &str,
    ) -> Option<FailureTransitionOutcome> {
        if !matches!(
            self.seat_assignment(pane_id),
            Some(SeatAssignmentState::Recovering { .. })
        ) {
            return None;
        }
        self.seat_assignments
            .insert(pane_id, SeatAssignmentState::RecoveryFailed);

        let ticket_id = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| slot.ticket_id.clone());
        let Some(ticket_id) = ticket_id else {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Codex assignment recovery failed on pane {}: {}; reset the ticket after repairing the seat reservation",
                    pane_id, reason
                ),
            });
            return Some(FailureTransitionOutcome::AssignmentRecoveryFailed {
                pane_id,
                ticket_id: None,
            });
        };

        if let Some(thread) = self.threads.get_mut(&ticket_id) {
            thread.fail();
        }
        self.emit_assignment_transition(
            pane_id,
            &ticket_id,
            AssignmentState::RecoveryFailed,
            reason,
        );
        if !self
            .error_alerts
            .iter()
            .any(|(existing, existing_pane)| existing == &ticket_id && *existing_pane == pane_id)
        {
            self.error_alerts.push((ticket_id.clone(), pane_id));
        }
        self.log_activity(ActivityEvent::Error {
            message: format!(
                "{} Codex assignment recovery failed on pane {}: {}; reset the ticket to retry",
                ticket_id, pane_id, reason
            ),
        });
        Some(FailureTransitionOutcome::AssignmentRecoveryFailed {
            pane_id,
            ticket_id: Some(ticket_id),
        })
    }

    /// Revoke an unproven original launch and begin the one permitted reset in
    /// the same physical pane. The successor marker is deliberately withheld
    /// until the shell probe proves a command boundary.
    fn begin_startup_recovery(
        &mut self,
        pane_id: u32,
        now: std::time::SystemTime,
    ) -> Option<FailureTransitionOutcome> {
        let Some(SeatAssignmentState::Starting {
            generation: prior_generation,
            relaunches: 0,
            ..
        }) = self.seat_assignment(pane_id)
        else {
            return None;
        };
        let Some(slot_idx) = self
            .agent_slots
            .iter()
            .position(|slot| slot.pane_id == pane_id && slot.ticket_id.is_some())
        else {
            return self.fail_startup(pane_id, "same-pane recovery reservation is missing");
        };
        let ticket_id = self.agent_slots[slot_idx]
            .ticket_id
            .clone()
            .expect("startup recovery slot has a ticket");
        let Some(predecessor) = self.agent_slots[slot_idx].attempt_lease.clone() else {
            return self.fail_startup(pane_id, "same-pane recovery attempt lease is missing");
        };
        let valid_predecessor = predecessor.ticket_id == ticket_id
            && predecessor.attempt_id == prior_generation
            && predecessor.is_current(self.current_leases.get(&ticket_id))
            && self.lease_high_water.get(&ticket_id) == Some(&predecessor);
        if !valid_predecessor {
            return self.fail_startup(pane_id, "same-pane recovery attempt lease is stale");
        }

        self.revoke_current_lease(&ticket_id);
        let successor = match AttemptLease::mint(ticket_id.clone(), Some(&predecessor)) {
            Ok(successor) => successor,
            Err(error) => {
                self.seat_assignments.insert(
                    pane_id,
                    SeatAssignmentState::ResettingStartup {
                        generation: prior_generation,
                        reset_deadline: now,
                    },
                );
                return self.fail_startup_recovery(
                    pane_id,
                    &format!("cannot mint same-pane recovery lease: {error}"),
                );
            }
        };
        self.lease_high_water
            .insert(ticket_id.clone(), successor.clone());
        self.current_leases
            .insert(ticket_id.clone(), successor.clone());
        self.agent_slots[slot_idx].attempt_lease = Some(successor.clone());
        if let Some(thread) = self.threads.get_mut(&ticket_id) {
            thread.attempt_lease = Some(successor.clone());
        }
        self.seat_assignments.insert(
            pane_id,
            SeatAssignmentState::ResettingStartup {
                generation: successor.attempt_id,
                reset_deadline: self.assignment_ack_deadline(now),
            },
        );
        self.awaiting_human.remove(&pane_id);
        self.notified_attention.remove(&pane_id);
        self.clear_pane_lifecycle_signals(pane_id);

        let probe = match Self::shell_readiness_probe(&self.signal_dir, pane_id, &successor) {
            Ok(probe) => probe,
            Err(error) => {
                return self.fail_startup_recovery(pane_id, &error);
            }
        };
        self.interrupt_shell_input(pane_id);
        self.send_line_to_pane(&probe, PaneId::Terminal(pane_id));
        self.agent_slots[slot_idx].last_activity_at = Some(now);
        #[cfg(test)]
        self.attempt_lifecycle
            .push(AttemptLifecycleEvent::ShellInterrupted {
                ticket_id: ticket_id.clone(),
                pane_id,
            });
        self.log_activity(ActivityEvent::Warning {
            message: format!(
                "{} startup was not observed on pane {}; interrupted incomplete shell input and awaiting exact readiness for attempt {}",
                ticket_id, pane_id, successor.attempt_id
            ),
        });
        None
    }

    /// Admit exact successor shell proof and submit the already-established
    /// bare-provider launch contract back into the same physical pane.
    fn acknowledge_shell_ready(
        &mut self,
        pane_id: u32,
        candidate: &AttemptLease,
        now: std::time::SystemTime,
    ) -> bool {
        let Some(SeatAssignmentState::ResettingStartup { generation, .. }) =
            self.seat_assignment(pane_id)
        else {
            return false;
        };
        if generation != candidate.attempt_id {
            return false;
        }
        let Some((ticket_id, slot_lease)) = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| Some((slot.ticket_id.clone()?, slot.attempt_lease.clone()?)))
        else {
            return false;
        };
        if candidate.ticket_id != ticket_id
            || &slot_lease != candidate
            || !candidate.is_current(self.current_leases.get(&ticket_id))
        {
            return false;
        }

        let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);
        let attempt_artifact_dir = self.attempt_work_dir(candidate);
        let artifact_dir = strip_host_prefix(&attempt_artifact_dir);
        let (adapter, route) = resolve_adapter_or_native(
            self.dag.get_ticket(&ticket_id),
            self.config.client,
            self.config.lisa_bin.as_deref(),
        );
        let ctx = SpawnContext {
            ticket_dir: &host_ticket_dir,
            ticket_id: &ticket_id,
            pane_id,
            attempt_id: candidate.attempt_id,
            artifact_dir: &artifact_dir,
            assignment_generation: None,
        };
        let assignment = adapter.assignment_text(&ctx);
        let assignment_ref =
            match self.prepare_assignment(&attempt_artifact_dir, candidate, &assignment) {
                Ok(assignment_ref) => assignment_ref,
                Err(error) => {
                    self.fail_startup_recovery(pane_id, &error);
                    return false;
                }
            };
        let assignment_path = strip_host_prefix(&assignment_ref.path);
        let payload = adapter.launch_command(&ctx, &assignment_path);
        let command = match Self::prepare_fresh_launch(&attempt_artifact_dir, pane_id, &payload) {
            Ok(command) => command,
            Err(error) => {
                self.fail_startup_recovery(pane_id, &error);
                return false;
            }
        };
        if let Err(error) = self.write_pane_lease_marker(pane_id, candidate) {
            self.fail_startup_recovery(pane_id, &error);
            return false;
        }

        self.seat_assignments.insert(
            pane_id,
            SeatAssignmentState::Starting {
                generation,
                start_deadline: Some(self.assignment_ack_deadline(now)),
                relaunches: MAX_SAME_PANE_STARTUP_RELAUNCHES,
            },
        );
        self.send_line_to_pane(&command, PaneId::Terminal(pane_id));
        if let Some(slot) = self
            .agent_slots
            .iter_mut()
            .find(|slot| slot.pane_id == pane_id)
        {
            slot.has_session = true;
            slot.last_client = Some(route.agent);
            slot.last_activity_at = Some(now);
        }
        #[cfg(test)]
        self.attempt_lifecycle
            .push(AttemptLifecycleEvent::ShellRelaunched {
                ticket_id: ticket_id.clone(),
                pane_id,
            });
        self.log_activity(ActivityEvent::SessionLaunch {
            ticket_id: ticket_id.clone(),
            pane_id,
            command,
        });
        self.log_activity(ActivityEvent::Info {
            message: format!(
                "Pane {} proved shell readiness; relaunched {} for {} as attempt {}",
                pane_id, route.agent, ticket_id, generation
            ),
        });
        true
    }

    /// Exhausted shell reset or replacement startup is terminal for the pane.
    /// Retain the failed reservation for operator inspection, but revoke its
    /// authority and permanently fence the physical seat.
    fn fail_startup_recovery(
        &mut self,
        pane_id: u32,
        reason: &str,
    ) -> Option<FailureTransitionOutcome> {
        let recoverable = match self.seat_assignment(pane_id) {
            Some(SeatAssignmentState::ResettingStartup { .. }) => true,
            Some(SeatAssignmentState::Starting { relaunches, .. }) => {
                relaunches >= MAX_SAME_PANE_STARTUP_RELAUNCHES
            }
            _ => false,
        };
        if !recoverable {
            return None;
        }
        let slot_idx = self
            .agent_slots
            .iter()
            .position(|slot| slot.pane_id == pane_id)?;
        let ticket_id = self.agent_slots[slot_idx].ticket_id.clone()?;

        self.seat_assignments
            .insert(pane_id, SeatAssignmentState::StartupFailed);
        if let Some(thread) = self.threads.get_mut(&ticket_id) {
            thread.fail();
        }
        if !self
            .error_alerts
            .iter()
            .any(|(existing, existing_pane)| existing == &ticket_id && *existing_pane == pane_id)
        {
            self.error_alerts.push((ticket_id.clone(), pane_id));
        }
        self.revoke_current_lease(&ticket_id);
        self.clear_pane_lifecycle_signals(pane_id);
        self.awaiting_human.remove(&pane_id);
        self.notified_attention.remove(&pane_id);
        self.pending_enters
            .retain(|pending| pending.pane_id != PaneId::Terminal(pane_id));
        {
            let slot = &mut self.agent_slots[slot_idx];
            slot.transition_state = TransitionState::Fenced;
            slot.transition_started_at = None;
            slot.has_session = false;
            slot.last_client = None;
            slot.cooldown_until = None;
        }
        close_fenced_pane(pane_id);
        #[cfg(test)]
        self.attempt_lifecycle
            .push(AttemptLifecycleEvent::PaneFenced {
                ticket_id: ticket_id.clone(),
                pane_id,
            });
        self.log_activity(ActivityEvent::Error {
            message: format!(
                "{} same-pane startup recovery failed on pane {}: {}; pane fenced, reset the ticket to retry",
                ticket_id, pane_id, reason
            ),
        });
        Some(FailureTransitionOutcome::StartupRecoveryFailed { pane_id, ticket_id })
    }

    /// End a fresh provider start wait without releasing the reservation back
    /// into automatic scheduling. Missing positive start evidence must remain a
    /// named operator-actionable failure, never implicit ownership or a retry
    /// loop.
    fn fail_startup(&mut self, pane_id: u32, reason: &str) -> Option<FailureTransitionOutcome> {
        if !matches!(
            self.seat_assignment(pane_id),
            Some(SeatAssignmentState::Starting { .. })
        ) {
            return None;
        }
        self.seat_assignments
            .insert(pane_id, SeatAssignmentState::StartupFailed);

        let ticket_id = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| slot.ticket_id.clone());
        let Some(ticket_id) = ticket_id else {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Provider startup failed on pane {}: {}; reset the ticket after repairing the seat reservation",
                    pane_id, reason
                ),
            });
            return Some(FailureTransitionOutcome::StartupFailed {
                pane_id,
                ticket_id: None,
            });
        };

        if let Some(thread) = self.threads.get_mut(&ticket_id) {
            thread.fail();
        }
        self.emit_assignment_transition(
            pane_id,
            &ticket_id,
            AssignmentState::StartupFailed,
            reason,
        );
        if !self
            .error_alerts
            .iter()
            .any(|(existing, existing_pane)| existing == &ticket_id && *existing_pane == pane_id)
        {
            self.error_alerts.push((ticket_id.clone(), pane_id));
        }
        self.log_activity(ActivityEvent::Error {
            message: format!(
                "{} provider startup failed on pane {}: {}; reset the ticket to retry",
                ticket_id, pane_id, reason
            ),
        });
        Some(FailureTransitionOutcome::StartupFailed {
            pane_id,
            ticket_id: Some(ticket_id),
        })
    }

    /// Fence an expired reused-session delivery and begin the one allowed fresh
    /// Codex fallback. State changes before `/exit`, making late old-generation
    /// payloads inert even if they arrive on the next poll.
    fn begin_assignment_recovery(&mut self, pane_id: u32, now: std::time::SystemTime) {
        let Some(SeatAssignmentState::AssignedPendingAck {
            generation: prior_generation,
            ..
        }) = self.seat_assignment(pane_id)
        else {
            return;
        };
        let slot_idx = self
            .agent_slots
            .iter()
            .position(|slot| slot.pane_id == pane_id && slot.ticket_id.is_some());
        let Some(slot_idx) = slot_idx else {
            self.seat_assignments.insert(
                pane_id,
                SeatAssignmentState::Recovering {
                    generation: prior_generation,
                    ack_deadline: None,
                },
            );
            self.fail_assignment_recovery(pane_id, "ticket reservation is missing");
            return;
        };
        let ticket_id = self.agent_slots[slot_idx]
            .ticket_id
            .clone()
            .expect("recovery slot has a ticket");
        let predecessor = self.agent_slots[slot_idx].attempt_lease.clone();
        let predecessor_is_current = predecessor.as_ref().is_some_and(|lease| {
            lease.ticket_id == ticket_id
                && lease.attempt_id == prior_generation
                && lease.is_current(self.current_leases.get(&ticket_id))
                && self.lease_high_water.get(&ticket_id) == Some(lease)
        });
        if !predecessor_is_current {
            self.seat_assignments.insert(
                pane_id,
                SeatAssignmentState::Recovering {
                    generation: prior_generation,
                    ack_deadline: None,
                },
            );
            self.fail_assignment_recovery(pane_id, "current attempt lease is missing or stale");
            return;
        }
        let predecessor = predecessor.expect("validated recovery predecessor");
        let successor = match AttemptLease::mint(ticket_id.clone(), Some(&predecessor)) {
            Ok(successor) => successor,
            Err(error) => {
                self.seat_assignments.insert(
                    pane_id,
                    SeatAssignmentState::Recovering {
                        generation: prior_generation,
                        ack_deadline: None,
                    },
                );
                self.fail_assignment_recovery(
                    pane_id,
                    &format!("cannot mint recovery attempt lease: {error}"),
                );
                return;
            }
        };
        self.lease_high_water
            .insert(ticket_id.clone(), successor.clone());
        self.current_leases
            .insert(ticket_id.clone(), successor.clone());
        self.agent_slots[slot_idx].attempt_lease = Some(successor.clone());
        if let Some(thread) = self.threads.get_mut(&ticket_id) {
            thread.attempt_lease = Some(successor.clone());
        }
        self.seat_assignments.insert(
            pane_id,
            SeatAssignmentState::Recovering {
                generation: successor.attempt_id,
                ack_deadline: None,
            },
        );
        // This TUI is explicitly abandoned. Its old question/attention markers
        // must not suppress the graceful exit command for the fresh fallback.
        self.awaiting_human.remove(&pane_id);
        self.notified_attention.remove(&pane_id);
        let (adapter, _) =
            resolve_adapter_or_native(None, AgentClient::Codex, self.config.lisa_bin.as_deref());
        self.send_line_to_pane(adapter.exit_command(), PaneId::Terminal(pane_id));
        let slot = &mut self.agent_slots[slot_idx];
        slot.has_session = false;
        slot.transition_state = TransitionState::WaitingForExit;
        slot.transition_started_at = Some(now);
        slot.last_client = Some(AgentClient::Codex);
        slot.last_activity_at = Some(now);
        self.log_activity(ActivityEvent::Warning {
            message: format!(
                "{} acknowledgment timed out on pane {}; recovering with one fresh Codex session",
                ticket_id, pane_id
            ),
        });
    }

    /// Evaluate absolute acknowledgment deadlines at an injected time so native
    /// tests can cover the complete recovery contract without sleeping.
    fn check_assignment_ack_timeouts_at(
        &mut self,
        now: std::time::SystemTime,
    ) -> Vec<FailureTransitionOutcome> {
        let mut outcomes = Vec::new();
        let candidates = self.seat_assignments.iter().filter_map(|(pane_id, state)| {
            let deadline = match state {
                SeatAssignmentState::Starting {
                    start_deadline: Some(deadline),
                    ..
                }
                | SeatAssignmentState::Delivering {
                    ack_deadline: deadline,
                    ..
                }
                | SeatAssignmentState::DeliveredAwaitingClaim {
                    claim_deadline: deadline,
                    ..
                }
                | SeatAssignmentState::AssignedPendingAck {
                    ack_deadline: Some(deadline),
                    ..
                }
                | SeatAssignmentState::Recovering {
                    ack_deadline: Some(deadline),
                    ..
                }
                | SeatAssignmentState::ResettingStartup {
                    reset_deadline: deadline,
                    ..
                } => *deadline,
                _ => return None,
            };
            Some(AcknowledgementInput {
                pane_id: *pane_id,
                state: *state,
                deadline,
            })
        });
        let expired = DeadlineEvaluator::new(now).acknowledgements(candidates);

        for action in expired {
            let pane_id = action.pane_id;
            let state = action.state;
            if self.seat_assignment(pane_id) != Some(state) {
                continue;
            }
            match state {
                SeatAssignmentState::Starting {
                    relaunches: 0,
                    generation,
                    ..
                } => {
                    if self.seat_readiness_mode(pane_id) == Some(ReadinessMode::Grace) {
                        // The named startup grace elapsed. Pace the first prompt
                        // now: attempt the bounded attempt-tagged assignment and
                        // enter Delivering directly. Elapsed time paced the send
                        // — it is not readiness or ownership (E-037). A missed
                        // acknowledgement is resolved by the existing bounded
                        // Delivering retry → DeliveryFailed path, and ownership
                        // stays gated on the exact UserPromptSubmit. A send that
                        // cannot be submitted resolves in a named DeliveryFailed.
                        if let Err(error) =
                            self.deliver_assignment_to_pane(pane_id, generation, 0, now)
                        {
                            if let Some(outcome) = self.fail_assignment_delivery(pane_id, &error) {
                                outcomes.push(outcome);
                            }
                        }
                    } else {
                        if let Some(outcome) = self.begin_startup_recovery(pane_id, now) {
                            outcomes.push(outcome);
                        }
                    }
                }
                SeatAssignmentState::Starting { .. } => {
                    if let Some(outcome) = self.fail_startup_recovery(
                        pane_id,
                        "replacement provider process start was not observed before the deadline",
                    ) {
                        outcomes.push(outcome);
                    }
                }
                SeatAssignmentState::ResettingStartup { .. } => {
                    if let Some(outcome) = self.fail_startup_recovery(
                        pane_id,
                        "positive shell readiness was not observed before the deadline",
                    ) {
                        outcomes.push(outcome);
                    }
                }
                SeatAssignmentState::Delivering { generation, .. }
                    if self.is_live_codex_delivery(pane_id, generation) =>
                {
                    let ticket_id = self
                        .agent_slots
                        .iter()
                        .find(|slot| slot.pane_id == pane_id)
                        .and_then(|slot| slot.ticket_id.clone())
                        .unwrap_or_else(|| "unknown ticket".to_string());
                    self.seat_assignments.insert(
                        pane_id,
                        SeatAssignmentState::DeliveredAwaitingClaim {
                            generation,
                            claim_deadline: self.assignment_ack_deadline(now),
                        },
                    );
                    self.log_activity(ActivityEvent::Warning {
                        message: format!(
                            "{} assignment is delivered on live Codex pane {}; awaiting claim without re-injecting the prompt",
                            ticket_id, pane_id
                        ),
                    });
                }
                SeatAssignmentState::Delivering {
                    generation,
                    retries,
                    ..
                } if retries < MAX_ASSIGNMENT_DELIVERY_RETRIES => {
                    if let Err(error) =
                        self.deliver_assignment_to_pane(pane_id, generation, retries + 1, now)
                    {
                        if let Some(outcome) = self.fail_assignment_delivery(pane_id, &error) {
                            outcomes.push(outcome);
                        }
                    }
                }
                SeatAssignmentState::Delivering { .. } => {
                    if let Some(outcome) = self.fail_assignment_delivery(
                        pane_id,
                        "provider did not acknowledge the bounded chat assignment",
                    ) {
                        outcomes.push(outcome);
                    }
                }
                SeatAssignmentState::DeliveredAwaitingClaim { .. } => {
                    if let Some(outcome) = self.fail_assignment_claim_wait(
                        pane_id,
                        "delivered Codex assignment was not claimed before the bounded deadline",
                    ) {
                        outcomes.push(outcome);
                    }
                }
                SeatAssignmentState::AssignedPendingAck { .. } => {
                    self.begin_assignment_recovery(pane_id, now);
                }
                SeatAssignmentState::Recovering { .. } => {
                    if let Some(outcome) = self.fail_assignment_recovery(
                        pane_id,
                        "fresh Codex session did not acknowledge before the deadline",
                    ) {
                        outcomes.push(outcome);
                    }
                }
                _ => {}
            }
        }
        outcomes
    }

    fn check_assignment_ack_timeouts(&mut self) {
        let _ = self.check_assignment_ack_timeouts_at(std::time::SystemTime::now());
    }

    /// Whether a physical seat has acknowledged ownership of its assignment.
    ///
    /// Pending and recovering seats are intentionally not owned even though
    /// their slot retains a ticket reservation.
    fn seat_is_owned(&self, pane_id: u32) -> bool {
        self.seat_assignment(pane_id) == Some(SeatAssignmentState::Owned)
    }

    /// Find an idle agent slot that has finished its cooldown period.
    ///
    /// Busy-pane guard: a slot with a live session is only eligible once the
    /// pane has been signal-silent for the wind-down period. A session that is
    /// still making tool calls (heartbeats) or emitting stop/idle signals is
    /// never reused, even if its ticket was released — clearing a pane that is
    /// mid-task wastes the partial work and forces a repeat attempt.
    /// Find an idle slot eligible to host a session for the `want` provider.
    ///
    /// Provider-affinity (T-026-02): a slot qualifies directly only if it has no
    /// resident session or last ran the same provider. Cross-provider reuse is
    /// handled separately by `find_slot_for_client`, which explicitly exits the
    /// old TUI before launching the new one.
    fn find_idle_slot(&self, want: AgentClient) -> Option<usize> {
        let now = std::time::SystemTime::now();
        let wind_down = std::time::Duration::from_secs(self.config.wind_down_secs);
        self.agent_slots.iter().position(|s| {
            s.ticket_id.is_none()
                && s.transition_state == TransitionState::Idle
                && (!s.has_session || s.last_client.is_none() || s.last_client == Some(want))
                && s.cooldown_until.is_none_or(|until| now >= until)
                && (!s.has_session
                    || s.last_activity_at
                        .is_none_or(|at| now.duration_since(at).unwrap_or_default() >= wind_down))
        })
    }

    /// Select a pane for `want`, preferring a compatible/fresh pane and falling
    /// back to graceful recycling only when affinity would otherwise starve the
    /// provider. A recyclable pane must be unassigned, idle, cooled down, quiet,
    /// and still host a live session from the opposite provider. Running panes
    /// are never candidates.
    fn find_slot_for_client(&self, want: AgentClient) -> Option<SlotSelection> {
        if let Some(idx) = self.find_idle_slot(want) {
            return Some(SlotSelection::Compatible(idx));
        }

        let now = std::time::SystemTime::now();
        let wind_down = std::time::Duration::from_secs(self.config.wind_down_secs);
        self.agent_slots
            .iter()
            .position(|s| {
                s.ticket_id.is_none()
                    && s.transition_state == TransitionState::Idle
                    && s.has_session
                    && s.last_client.is_some_and(|client| client != want)
                    && !self.is_pane_awaiting(s.pane_id)
                    && s.cooldown_until.is_none_or(|until| now >= until)
                    && s.last_activity_at
                        .is_none_or(|at| now.duration_since(at).unwrap_or_default() >= wind_down)
            })
            .map(SlotSelection::Recycle)
    }

    /// True if provider `client` is under its per-provider concurrency cap given
    /// the currently running threads (T-026-02). The global `max_threads` ceiling
    /// is enforced separately by the caller; this checks only the optional
    /// per-provider sub-cap. No cap configured → always admits. `thread.client`
    /// is the resolved agent snapshotted at spawn, so it is the authoritative
    /// per-provider counter.
    fn provider_under_cap(&self, client: AgentClient) -> bool {
        match self.config.provider_cap_for(client) {
            None => true,
            Some(cap) => {
                let running_for_provider = self
                    .threads
                    .values()
                    .filter(|t| {
                        t.status == lisa_core::types::ThreadStatus::Running && t.client == client
                    })
                    .count();
                running_for_provider < cap
            }
        }
    }

    /// End the currently authorized attempt without discarding its monotonic
    /// high-water predecessor. Repeated revocation is intentionally harmless.
    fn revoke_current_lease(&mut self, ticket_id: &TicketId) -> Option<AttemptLease> {
        let revoked = self.current_leases.remove(ticket_id);
        #[cfg(test)]
        if revoked.is_some() {
            self.attempt_lifecycle
                .push(AttemptLifecycleEvent::LeaseRevoked {
                    ticket_id: ticket_id.clone(),
                });
        }
        revoked
    }

    /// Revoke one attempt and permanently disqualify its physical pane before
    /// the ticket reservation can be released for a successor dispatch.
    fn revoke_and_fence_attempt(&mut self, ticket_id: &TicketId) -> FenceOutcome {
        self.revoke_current_lease(ticket_id);

        let Some(slot_idx) = self
            .agent_slots
            .iter()
            .position(|slot| slot.ticket_id.as_ref() == Some(ticket_id))
        else {
            self.log_activity(ActivityEvent::Warning {
                message: format!(
                    "Revoked {} after hard silence, but no assigned pane was found to fence",
                    ticket_id
                ),
            });
            return FenceOutcome::NoAssignedPane;
        };

        let pane_id = self.agent_slots[slot_idx].pane_id;
        if self.agent_slots[slot_idx].transition_state == TransitionState::Fenced {
            return FenceOutcome::AlreadyFenced { pane_id };
        }

        {
            let slot = &mut self.agent_slots[slot_idx];
            slot.transition_state = TransitionState::Fenced;
            slot.transition_started_at = None;
            slot.has_session = false;
            slot.last_client = None;
            slot.cooldown_until = None;
        }
        self.seat_assignments.remove(&pane_id);
        self.awaiting_human.remove(&pane_id);
        self.notified_attention.remove(&pane_id);
        self.pending_enters
            .retain(|pending| pending.pane_id != PaneId::Terminal(pane_id));

        close_fenced_pane(pane_id);
        #[cfg(test)]
        self.attempt_lifecycle
            .push(AttemptLifecycleEvent::PaneFenced {
                ticket_id: ticket_id.clone(),
                pane_id,
            });
        self.log_activity(ActivityEvent::Info {
            message: format!(
                "Fenced pane {} for hard-silent attempt {} (terminal state, no retry)",
                pane_id, ticket_id
            ),
        });
        FenceOutcome::Fenced { pane_id }
    }

    /// Mark a slot as idle when its ticket completes. Keeps `has_session = true`
    /// so the same provider can reuse the TUI via `/clear`, while the other
    /// provider can explicitly recycle it via `/exit` after cooldown.
    fn release_slot_for_ticket(&mut self, ticket_id: &TicketId) {
        // Release is the shared rescheduling boundary. No caller may expose a
        // ticket to the DAG while its prior attempt remains authoritative.
        self.revoke_current_lease(ticket_id);

        let mut released_pane: Option<(u32, Option<String>, bool)> = None;
        for slot in &mut self.agent_slots {
            if slot.ticket_id.as_ref() == Some(ticket_id) {
                let fenced = slot.transition_state == TransitionState::Fenced;
                slot.ticket_id = None;
                slot.attempt_lease = None;
                let idle_name = if fenced {
                    // The terminal pane no longer exists and this slot is
                    // permanently ineligible. Do not rename or cool it down.
                    slot.has_session = false;
                    slot.last_client = None;
                    slot.cooldown_until = None;
                    None
                } else {
                    // has_session stays true — the native agent TUI is still running
                    slot.cooldown_until = Some(
                        std::time::SystemTime::now()
                            + std::time::Duration::from_secs(self.config.wind_down_secs),
                    );
                    let resident_agent = if slot.has_session {
                        slot.last_client
                    } else {
                        None
                    };
                    Some(format_pane_name(PaneName::Idle { resident_agent }))
                };
                released_pane = Some((slot.pane_id, idle_name, fenced));
                break;
            }
        }
        if let Some((pane_id, _, _)) = &released_pane {
            self.seat_assignments.remove(pane_id);
        }
        match released_pane {
            Some((pane_id, idle_name, fenced)) => {
                if let Some(idle_name) = idle_name {
                    self.rename_slot(pane_id, idle_name);
                }
                #[cfg(test)]
                self.attempt_lifecycle
                    .push(AttemptLifecycleEvent::SlotReleased {
                        ticket_id: ticket_id.clone(),
                    });
                self.log_activity(ActivityEvent::Info {
                    message: if fenced {
                        format!("Released fenced slot #{} for {}", pane_id, ticket_id)
                    } else {
                        format!("Released slot #{} for {}", pane_id, ticket_id)
                    },
                });
            }
            None => self.log_activity(ActivityEvent::Info {
                message: format!("No slot found for {}", ticket_id),
            }),
        }
    }

    /// Release a durably completed ticket and gracefully retire a resident
    /// Codex TUI before this physical pane can accept another assignment.
    ///
    /// Generic release remains provider-neutral and is also used by failure
    /// paths. Successful completion alone owns this clean process boundary:
    /// revoke the predecessor authority first, then exit its TUI, and leave the
    /// unassigned pane unavailable until the existing exit grace proves a
    /// clean shell.
    fn release_completed_slot_for_ticket(&mut self, ticket_id: &TicketId) {
        let clean_exit = self
            .agent_slots
            .iter()
            .find(|slot| slot.ticket_id.as_ref() == Some(ticket_id))
            .filter(|slot| {
                slot.transition_state != TransitionState::Fenced
                    && slot.has_session
                    && slot.last_client == Some(AgentClient::Codex)
            })
            .map(|slot| {
                let (adapter, _) = resolve_adapter_or_native(
                    None,
                    AgentClient::Codex,
                    self.config.lisa_bin.as_deref(),
                );
                (slot.pane_id, adapter.exit_command())
            });

        self.release_slot_for_ticket(ticket_id);

        let Some((pane_id, exit_command)) = clean_exit else {
            return;
        };

        self.send_line_to_pane(exit_command, PaneId::Terminal(pane_id));
        if let Some(slot) = self
            .agent_slots
            .iter_mut()
            .find(|slot| slot.pane_id == pane_id)
        {
            slot.transition_state = TransitionState::WaitingForExit;
            slot.transition_started_at = Some(std::time::SystemTime::now());
            slot.has_session = false;
            slot.cooldown_until = None;
        }
        #[cfg(test)]
        self.attempt_lifecycle
            .push(AttemptLifecycleEvent::CleanExitRequested {
                ticket_id: ticket_id.clone(),
                pane_id,
            });
        self.log_activity(ActivityEvent::Info {
            message: format!(
                "Completion boundary revoked {} and requested clean Codex exit on pane {}",
                ticket_id, pane_id
            ),
        });
    }

    /// Schedule ready tickets into idle agent slots.
    fn schedule_ready_tickets(&mut self) {
        // Scheduling is an admission boundary as well as a poll consequence.
        // Permission and pane events can call this outside `poll_tick`, so an
        // orphaned durable Block must park before any scheduling early return.
        self.reconcile_orphaned_review_blocks();

        if (!self.completion_journal_path.as_os_str().is_empty()
            && !self.completion_journal_healthy)
            || !self.permissions_granted
            || !self.slots_discovered
            || self.paused
        {
            return;
        }

        let ready = self.dag.get_ready_tickets();
        let mut unscheduled = 0usize;

        for ticket_id in ready {
            if self
                .completion_aggregates
                .get(&ticket_id)
                .map(CompletionJournalAggregate::masks_durable_done)
                .unwrap_or(false)
            {
                continue;
            }

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

            // Resolve the adapter AND the route for this ticket at spawn time
            // (per-pane routing seam, T-026-01): ticket `(agent, model)`
            // frontmatter → loop default → native Claude. Resolved *before* the
            // cap gates (T-026-02) so the per-provider cap and slot affinity can
            // see the resolved agent. The returned Box owns nothing from
            // self.dag, so it is safe to hold across the &mut self work below.
            // The route is stored on the thread and drives the substitution log +
            // dashboard surfacing below.
            let (adapter, route) = resolve_adapter_or_native(
                self.dag.get_ticket(&ticket_id),
                self.config.client,
                self.config.lisa_bin.as_deref(),
            );

            // Enforce the global concurrency cap: at most max_threads running
            // threads across all providers. Extra pane slots exist for overlap
            // during transitions.
            let running_count = self
                .threads
                .values()
                .filter(|t| t.status == lisa_core::types::ThreadStatus::Running)
                .count();
            if running_count >= self.config.max_threads {
                unscheduled += 1;
                continue;
            }

            // Enforce the optional per-provider sub-cap (T-026-02): a provider
            // with a configured cap may run at most that many concurrent threads,
            // *within* the global ceiling. This keeps one provider's separate
            // auth/rate-limit pool from being saturated when mixing providers.
            // Absent cap → only the global gate applies (single-provider loops
            // unchanged). Pure decision factored into `provider_under_cap` so it
            // is unit-testable without Zellij host calls.
            if !self.provider_under_cap(route.agent) {
                unscheduled += 1;
                continue;
            }

            // Prefer a fresh/provider-compatible pane. If every released pane is
            // resident in the other client, select one for an explicit `/exit`
            // recycle instead of starving this provider forever. Busy panes are
            // excluded by `find_slot_for_client`.
            let (slot_idx, cross_provider_recycle) = match self.find_slot_for_client(route.agent) {
                Some(SlotSelection::Compatible(idx)) => (idx, false),
                Some(SlotSelection::Recycle(idx)) => (idx, true),
                None => {
                    unscheduled += 1;
                    continue;
                }
            };
            // Preserve the pre-handoff residency fact. The recycle branch below
            // clears `has_session` while the old provider exits, but this remains
            // a reassigned physical seat and needs the pending-ack contract when
            // the incoming provider is Codex.
            let reused_seat = self.agent_slots[slot_idx].has_session;
            // Some interactive providers have no reliable in-process reset
            // boundary. Treat their compatible resident session exactly like a
            // cross-provider recycle: `/exit`, allow the bounded exit grace,
            // then launch a fresh process before attempting chat delivery.
            let recycle = cross_provider_recycle
                || (reused_seat && adapter.reset_strategy() == ResetStrategy::ExitThenFresh);

            // Build the host-relative ticket dir (strip /host/ prefix)
            let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);

            let pane_id = self.agent_slots[slot_idx].pane_id;

            // Defensive: an idle slot rarely hosts an agent blocked on a question,
            // but if it does, leave the slot unassigned and retry next poll rather
            // than /clear-ing or launching over the question UI.
            if self.is_pane_awaiting(pane_id) {
                unscheduled += 1;
                continue;
            }

            // Mint only after every admission gate, but before pane lifecycle
            // side effects. Retaining the predecessor in `lease_high_water`
            // makes revocation/release followed by redispatch monotonic while
            // `current_leases` remains a truthful authority registry.
            if let Some(durable) = self.durable_attempt_high_water(&ticket_id) {
                let replace = self
                    .lease_high_water
                    .get(&ticket_id)
                    .map(|memory| durable.attempt_id > memory.attempt_id)
                    .unwrap_or(true);
                if replace {
                    self.lease_high_water.insert(ticket_id.clone(), durable);
                }
            }
            let attempt_lease = match AttemptLease::mint(
                ticket_id.clone(),
                self.lease_high_water.get(&ticket_id),
            ) {
                Ok(lease) => lease,
                Err(error) => {
                    self.log_activity(ActivityEvent::Error {
                        message: format!(
                            "Cannot dispatch {}: failed to mint attempt lease: {}",
                            ticket_id, error
                        ),
                    });
                    unscheduled += 1;
                    continue;
                }
            };
            self.lease_high_water
                .insert(ticket_id.clone(), attempt_lease.clone());
            self.current_leases
                .insert(ticket_id.clone(), attempt_lease.clone());

            if !reused_seat {
                if let Err(error) = self.write_pane_lease_marker(pane_id, &attempt_lease) {
                    self.revoke_current_lease(&ticket_id);
                    self.log_activity(ActivityEvent::Error {
                        message: format!(
                            "Cannot dispatch {}: failed to publish attempt marker: {}",
                            ticket_id, error
                        ),
                    });
                    unscheduled += 1;
                    continue;
                }
            }

            let assignment_generation = if route.agent == AgentClient::Codex && reused_seat {
                Some(attempt_lease.attempt_id)
            } else {
                None
            };

            let attempt_artifact_dir = self.attempt_work_dir(&attempt_lease);
            let artifact_dir = strip_host_prefix(&attempt_artifact_dir);

            let ctx = SpawnContext {
                ticket_dir: &host_ticket_dir,
                ticket_id: &ticket_id,
                pane_id,
                attempt_id: attempt_lease.attempt_id,
                artifact_dir: &artifact_dir,
                assignment_generation,
            };

            // Persist the complete instructions before any provider lifecycle
            // input. A fresh Codex script receives only this file's exact path;
            // Claude remains bare and waits for its established delivery path.
            let assignment_text = adapter.assignment_text(&ctx);
            let assignment_ref = match self.prepare_assignment(
                &attempt_artifact_dir,
                &attempt_lease,
                &assignment_text,
            ) {
                Ok(assignment_ref) => assignment_ref,
                Err(error) => {
                    self.revoke_current_lease(&ticket_id);
                    self.log_activity(ActivityEvent::Error {
                        message: format!(
                            "Cannot dispatch {} on pane {}: {}",
                            ticket_id, pane_id, error
                        ),
                    });
                    unscheduled += 1;
                    continue;
                }
            };
            let assignment_path = strip_host_prefix(&assignment_ref.path);

            // Replace any previous ticket/idle title before the first lifecycle
            // input for this assignment (/exit, /clear, or a fresh launch).
            let ticket_title = self
                .dag
                .get_ticket(&ticket_id)
                .map(|ticket| ticket.title.clone())
                .unwrap_or_else(|| "untitled".to_string());
            let assigned_name = format_pane_name(PaneName::Assigned {
                agent: route.agent,
                ticket_id: &ticket_id,
                title: &ticket_title,
            });
            self.rename_slot(pane_id, assigned_name);

            let launch_cmd;
            if recycle {
                // Cross-provider reuse must return to the pane's shell first.
                // Resolve the resident adapter (not the incoming one) so future
                // clients can own their graceful-exit spelling independently.
                let resident_client = self.agent_slots[slot_idx]
                    .last_client
                    .expect("recyclable slot has a resident client");
                let (resident_adapter, _) = resolve_adapter_or_native(
                    None,
                    resident_client,
                    self.config.lisa_bin.as_deref(),
                );
                let exit_command = resident_adapter.exit_command();
                let payload = adapter.launch_command(&ctx, &assignment_path);
                launch_cmd =
                    match Self::prepare_fresh_launch(&attempt_artifact_dir, pane_id, &payload) {
                        Ok(command) => command,
                        Err(error) => {
                            self.revoke_current_lease(&ticket_id);
                            self.rename_slot(
                                pane_id,
                                format_pane_name(PaneName::Idle {
                                    resident_agent: Some(resident_client),
                                }),
                            );
                            self.log_activity(ActivityEvent::Error {
                                message: format!(
                                    "Cannot dispatch {} on pane {}: {}",
                                    ticket_id, pane_id, error
                                ),
                            });
                            unscheduled += 1;
                            continue;
                        }
                    };
                self.send_line_to_pane(exit_command, PaneId::Terminal(pane_id));
                self.agent_slots[slot_idx].has_session = false;
                self.agent_slots[slot_idx].transition_state = TransitionState::WaitingForExit;
                self.agent_slots[slot_idx].transition_started_at =
                    Some(std::time::SystemTime::now());
                self.notified_attention.remove(&pane_id);
                self.awaiting_human.remove(&pane_id);
                self.log_activity(ActivityEvent::Info {
                    message: format!(
                        "Recycling pane {} from {} to {} via {}",
                        pane_id, resident_client, route.agent, exit_command
                    ),
                });
            } else if self.agent_slots[slot_idx].has_session {
                // Session reuse. For the ClearHandshake adapter (native Claude):
                // the slot is idle (ticket_id was None), so Claude Code is already
                // at its prompt. Send /clear directly and wait for the .cleared
                // signal before sending the prompt. (The old WaitingForStop
                // approach deadlocked because the previous session's .stopped
                // signal was already consumed by check_transition_signals()
                // earlier in the same poll_tick.)
                match adapter.reset_strategy() {
                    ResetStrategy::ClearHandshake => {
                        let reuse_prompt = adapter.reuse_prompt(&ctx);
                        self.send_line_to_pane("/clear", PaneId::Terminal(pane_id));
                        self.agent_slots[slot_idx].transition_state =
                            TransitionState::WaitingForClear;
                        self.agent_slots[slot_idx].transition_started_at =
                            Some(std::time::SystemTime::now());
                        launch_cmd = reuse_prompt;
                    }
                    ResetStrategy::ExitThenFresh => {
                        unreachable!("exit-then-fresh sessions enter the recycle branch")
                    }
                    // Reuse-as-fresh-exec (headless/bridge adapters). The prior
                    // process left the pane's shell at its prompt, so there is no
                    // /clear handshake: type a fresh command for the new ticket.
                    // WaitingForClear must not engage — leaving
                    // transition_state untouched (Idle) keeps the .cleared/
                    // clear-timeout machinery inert for this pane.
                    ResetStrategy::FreshExec => {
                        let payload = adapter.launch_command(&ctx, &assignment_path);
                        launch_cmd = match Self::prepare_fresh_launch(
                            &attempt_artifact_dir,
                            pane_id,
                            &payload,
                        ) {
                            Ok(command) => command,
                            Err(error) => {
                                self.revoke_current_lease(&ticket_id);
                                self.log_activity(ActivityEvent::Error {
                                    message: format!(
                                        "Cannot dispatch {} on pane {}: {}",
                                        ticket_id, pane_id, error
                                    ),
                                });
                                unscheduled += 1;
                                continue;
                            }
                        };
                        self.send_line_to_pane(&launch_cmd, PaneId::Terminal(pane_id));
                    }
                }
            } else {
                // Fresh pane — launch the agent from the shell.
                let payload = adapter.launch_command(&ctx, &assignment_path);
                launch_cmd =
                    match Self::prepare_fresh_launch(&attempt_artifact_dir, pane_id, &payload) {
                        Ok(command) => command,
                        Err(error) => {
                            self.revoke_current_lease(&ticket_id);
                            self.rename_slot(
                                pane_id,
                                format_pane_name(PaneName::Idle {
                                    resident_agent: None,
                                }),
                            );
                            self.log_activity(ActivityEvent::Error {
                                message: format!(
                                    "Cannot dispatch {} on pane {}: {}",
                                    ticket_id, pane_id, error
                                ),
                            });
                            unscheduled += 1;
                            continue;
                        }
                    };
                self.send_line_to_pane(&launch_cmd, PaneId::Terminal(pane_id));
                self.agent_slots[slot_idx].has_session = true;
            }

            self.agent_slots[slot_idx].ticket_id = Some(ticket_id.clone());
            self.agent_slots[slot_idx].attempt_lease = Some(attempt_lease.clone());
            // Stamp the provider that claimed this pane. A compatible session is
            // reused in-place; a recycled pane is reserved for this provider
            // while WaitingForExit prevents any other scheduler claim.
            self.agent_slots[slot_idx].last_client = Some(route.agent);
            let fresh_launch =
                recycle || !reused_seat || adapter.reset_strategy() == ResetStrategy::FreshExec;
            let assignment_state = if fresh_launch {
                SeatAssignmentState::Starting {
                    generation: attempt_lease.attempt_id,
                    start_deadline: None,
                    relaunches: 0,
                }
            } else if let Some(generation) = assignment_generation {
                SeatAssignmentState::AssignedPendingAck {
                    generation,
                    ack_deadline: None,
                }
            } else {
                // Same-process Claude reuse retains its established ownership;
                // fresh processes and reused Codex prompts are gated above.
                SeatAssignmentState::Owned
            };
            self.seat_assignments.insert(pane_id, assignment_state);
            // Read the provider's bootstrap-readiness mode at launch dispatch and
            // record it for the Starting seat (T-037-01-01). Classification only;
            // the grace transition that consumes it lands in T-037-01-02.
            if fresh_launch {
                self.seat_readiness
                    .insert(pane_id, adapter.readiness_mode());
            }
            if self.agent_slots[slot_idx].transition_state == TransitionState::Idle {
                self.start_assignment_ack_wait(pane_id, std::time::SystemTime::now());
            }
            // Sending input counts as pane activity — restarts the wind-down clock.
            self.agent_slots[slot_idx].last_activity_at = Some(std::time::SystemTime::now());

            // Surface a routing substitution (T-026-01): an invalid ticket route
            // fell back to the loop default. Logged here; also visible on the
            // dashboard via the stored route and recorded in provenance.
            if route.substituted {
                if let Some(note) = &route.note {
                    self.log_activity(ActivityEvent::Warning {
                        message: format!("{}: {}", ticket_id, note),
                    });
                }
            }

            // Create thread record with the ticket's current phase
            let mut thread = Thread::new(ticket_id.clone(), pane_id);
            thread.attempt_lease = Some(attempt_lease);
            // Snapshot run provenance known only at spawn: the resolved route
            // (T-026-01) and the concurrency at spawn (running_count, computed
            // above, excludes this new thread). `client` mirrors the route's
            // resolved agent — the authoritative "which agent ran" (T-027-01).
            thread.client = route.agent;
            thread.route = Some(route);
            thread.concurrency_at_spawn = running_count;
            if let Some(ticket) = self.dag.get_ticket(&ticket_id) {
                thread.current_phase = ticket.phase;

                // Ready is a scheduling sentinel — once spawned, advance to
                // Research so the artifact detection pipeline can track progress.
                if ticket.phase == Phase::Ready {
                    thread.current_phase = Phase::Research;
                    if !ticket.file_path.as_os_str().is_empty() {
                        if let Err(e) =
                            ticket::update_ticket_phase(&ticket.file_path, Phase::Research)
                        {
                            self.log_activity(ActivityEvent::Error {
                                message: format!(
                                    "Failed to advance {} from Ready: {}",
                                    ticket_id, e
                                ),
                            });
                        }
                    }
                }
            }
            self.threads.insert(ticket_id.clone(), thread);

            // Clear any stale timeout / error alert for this ticket (it's being rescheduled)
            self.timeout_alerts.retain(|(tid, _, _)| tid != &ticket_id);
            self.error_alerts.retain(|(tid, _)| tid != &ticket_id);

            self.log_activity(ActivityEvent::SessionLaunch {
                ticket_id: ticket_id.clone(),
                pane_id,
                command: launch_cmd,
            });
            self.log_activity(ActivityEvent::ThreadSpawned { ticket_id, pane_id });
        }

        if unscheduled > 0 {
            self.log_activity(ActivityEvent::Info {
                message: format!(
                    "No idle slots available, {} ready tickets waiting",
                    unscheduled
                ),
            });
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
                if self.pending_completions.contains_key(tid) {
                    return None;
                }
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
    ///
    /// Loops until no more advances can be made so that an agent which
    /// completes multiple phases in a single session catches up in one tick
    /// rather than advancing one phase per poll cycle.
    ///
    /// For the Implement phase, `review.md` (not `progress.md`) is the
    /// completion artifact. `progress.md` is a living tracking document
    /// created early in the implement phase, so it cannot serve as a
    /// completion signal. The presence of `review.md` means the agent has
    /// moved past implement into review.
    fn check_artifact_advances(&mut self) {
        loop {
            // Snapshot running threads each iteration — phases change as we advance
            let running: Vec<(TicketId, Phase, Option<AttemptLease>)> = self
                .threads
                .iter()
                .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
                .map(|(tid, t)| (tid.clone(), t.current_phase, t.attempt_lease.clone()))
                .collect();

            let mut advanced_any = false;

            for (ticket_id, current_phase, source_lease) in running {
                // progress.md is a living Implement artifact: publish current
                // bytes for durability/review, but never use it as a phase edge.
                if current_phase == Phase::Implement {
                    match self.admit_artifact(&ticket_id, source_lease.as_ref(), "progress.md") {
                        Ok(true) => self.record_artifact_ownership(
                            &ticket_id,
                            source_lease.as_ref(),
                            "progress.md",
                        ),
                        Ok(false) => {}
                        Err(error) => {
                            self.log_activity(ActivityEvent::Error {
                                message: format!(
                                    "Rejected progress publication for {}: {}",
                                    ticket_id, error
                                ),
                            });
                        }
                    }
                }
                // Determine which artifact signals completion of this phase.
                // Implement uses review.md instead of progress.md (living doc).
                let artifact_name = if current_phase == Phase::Implement {
                    "review.md"
                } else {
                    match current_phase.artifact_filename() {
                        Some(name) => name,
                        None => continue,
                    }
                };

                match self.admit_artifact(&ticket_id, source_lease.as_ref(), artifact_name) {
                    Ok(true) => self.record_artifact_ownership(
                        &ticket_id,
                        source_lease.as_ref(),
                        artifact_name,
                    ),
                    Ok(false) => continue,
                    Err(error) => {
                        self.log_activity(ActivityEvent::Error {
                            message: format!(
                                "Rejected artifact publication for {}: {}",
                                ticket_id, error
                            ),
                        });
                        continue;
                    }
                }

                // Compute next phase (always Some for phases with artifacts)
                let next_phase = match current_phase.next() {
                    Some(p) => p,
                    None => continue,
                };

                if next_phase == Phase::Done {
                    if let Some(source_lease) = source_lease {
                        self.dispatch_completion(CompletionInput::Artifact {
                            ticket_id: ticket_id.clone(),
                            source_lease,
                        });
                    } else {
                        self.log_activity(ActivityEvent::Warning {
                            message: format!(
                                "Rejected completion for {ticket_id} (Artifact): no attempt lease"
                            ),
                        });
                    }
                    continue;
                }

                // Update the ticket file on disk
                let file_path = self.dag.get_ticket(&ticket_id).map(|t| t.file_path.clone());
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
                    thread.mark_phase_change(std::time::SystemTime::now());
                }

                advanced_any = true;
            }

            if !advanced_any {
                break;
            }
        }
    }

    /// Apply bounded scheduler policy to complete, current-attempt Review
    /// blocks. The admitted canonical disposition remains the structured
    /// payload; ticket status is the only durable scheduling authority.
    fn apply_review_block_policy(&mut self) {
        let candidates: Vec<(TicketId, AttemptLease, std::time::SystemTime)> = self
            .threads
            .iter()
            .filter(|(_, thread)| {
                thread.status == lisa_core::types::ThreadStatus::Running
                    && thread.current_phase == Phase::Review
            })
            .filter_map(|(ticket_id, thread)| {
                let lease = thread.attempt_lease.clone()?;
                (lease.ticket_id == *ticket_id
                    && lease.is_current(self.current_leases.get(ticket_id)))
                .then_some((ticket_id.clone(), lease, thread.started_at))
            })
            .collect();

        let mut parked_any = false;
        for (ticket_id, source_lease, attempt_started_at) in candidates {
            let inputs = self.review_completion_inputs(&ticket_id, &source_lease);
            if inputs.artifact_admission.is_none() {
                continue;
            }
            let ReviewDisposition::Block {
                remedy_owner, ask, ..
            } = inputs.disposition
            else {
                continue;
            };

            let retries_consumed = self
                .agent_block_retries
                .get(&ticket_id)
                .copied()
                .unwrap_or(0);
            match review_block_action(remedy_owner, retries_consumed) {
                ReviewBlockAction::Retry {
                    retry_count,
                    retry_limit,
                } => {
                    self.emit_review_block_transition(
                        &ticket_id,
                        remedy_owner,
                        ParkingTransitionType::Retry,
                        Some((retry_count, retry_limit)),
                        false,
                        attempt_started_at,
                    );
                    self.agent_block_retries
                        .insert(ticket_id.clone(), retry_count);
                    self.release_slot_for_ticket(&ticket_id);
                    self.threads.remove(&ticket_id);
                    self.finish_up_sent.remove(&ticket_id);
                    self.log_activity(ActivityEvent::Info {
                        message: format!(
                            "Retrying agent-owned Review block for {ticket_id} ({retry_count}/{retry_limit}): {ask}"
                        ),
                    });
                }
                ReviewBlockAction::Park {
                    retry_count,
                    retry_limit,
                    recheck_eligible,
                } => {
                    let Some(file_path) = self
                        .dag
                        .get_ticket(&ticket_id)
                        .map(|ticket| ticket.file_path.clone())
                        .filter(|path| !path.as_os_str().is_empty())
                    else {
                        self.log_activity(ActivityEvent::Error {
                            message: format!(
                                "Cannot park {ticket_id}: ticket file path is unavailable"
                            ),
                        });
                        continue;
                    };
                    if let Err(error) =
                        ticket::update_ticket_status(&file_path, TicketStatus::Blocked)
                    {
                        self.log_activity(ActivityEvent::Error {
                            message: format!(
                                "Cannot park {ticket_id}: failed to write blocked status: {error}"
                            ),
                        });
                        continue;
                    }

                    let parked_at = std::time::SystemTime::now();
                    self.emit_review_block_transition(
                        &ticket_id,
                        remedy_owner,
                        ParkingTransitionType::Park,
                        retry_count.zip(retry_limit),
                        recheck_eligible,
                        parked_at,
                    );
                    self.release_slot_for_ticket(&ticket_id);
                    self.threads.remove(&ticket_id);
                    self.finish_up_sent.remove(&ticket_id);
                    parked_any = true;
                    self.log_activity(ActivityEvent::Info {
                        message: format!(
                            "Parked {ticket_id} ({remedy_owner:?}, recheck eligible: {recheck_eligible}): {ask}"
                        ),
                    });
                }
            }
        }

        if parked_any {
            self.rebuild_dag();
        }
    }

    /// Reconcile blocking Review verdicts whose writing attempt no longer has
    /// a live current thread.
    ///
    /// Attempt directories retain generation identity after lease revocation.
    /// An exact-generation Retry/Park/Unpark row means that generation's
    /// verdict was already consumed; otherwise a valid Block parks directly so
    /// scheduling never spends a seat re-deriving durable evidence.
    fn reconcile_orphaned_review_blocks(&mut self) {
        const DISPOSITION_ARTIFACT: &str = "review-disposition.json";

        let latest_transitions = self.latest_parking_transitions();
        let candidates: Vec<_> = self
            .dag
            .tickets()
            .filter(|ticket| {
                ticket.phase == Phase::Review
                    && !matches!(
                        ticket.status,
                        TicketStatus::Blocked | TicketStatus::Done | TicketStatus::Cancelled
                    )
            })
            .filter(|ticket| {
                !self.threads.get(&ticket.id).is_some_and(|thread| {
                    thread.status == lisa_core::types::ThreadStatus::Running
                        && thread.current_phase == Phase::Review
                        && thread.attempt_lease.as_ref().is_some_and(|lease| {
                            lease.ticket_id == ticket.id
                                && lease.is_current(self.current_leases.get(&ticket.id))
                        })
                })
            })
            .filter_map(|ticket| {
                let lease = self
                    .current_leases
                    .get(&ticket.id)
                    .cloned()
                    .or_else(|| self.durable_attempt_high_water(&ticket.id))?;
                if latest_transitions
                    .get(&ticket.id)
                    .is_some_and(|transition| {
                        transition.attempt_lease.attempt_id >= lease.attempt_id
                    })
                {
                    return None;
                }
                let private_path = self.attempt_work_dir(&lease).join(DISPOSITION_ARTIFACT);
                let canonical_path = self
                    .config
                    .work_dir
                    .join(&ticket.id)
                    .join(DISPOSITION_ARTIFACT);
                let private = std::fs::read(&private_path).ok()?;
                let canonical = std::fs::read(&canonical_path).ok()?;
                if private != canonical {
                    return None;
                }
                let started_at = private_path
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .unwrap_or_else(|_| std::time::SystemTime::now());
                let ReviewDisposition::Block {
                    remedy_owner, ask, ..
                } = parse_review_disposition(&canonical_path)
                else {
                    return None;
                };
                Some((ticket.id.clone(), lease, remedy_owner, ask, started_at))
            })
            .collect();

        let mut parked_any = false;
        for (ticket_id, lease, remedy_owner, ask, started_at) in candidates {
            let Some(ticket) = self.dag.get_ticket(&ticket_id) else {
                continue;
            };
            if ticket.phase != Phase::Review
                || matches!(
                    ticket.status,
                    TicketStatus::Blocked | TicketStatus::Done | TicketStatus::Cancelled
                )
            {
                continue;
            }
            let file_path = ticket.file_path.clone();
            if file_path.as_os_str().is_empty() {
                self.log_activity(ActivityEvent::Error {
                    message: format!(
                        "Cannot park orphaned Review block for {ticket_id}: ticket file path is unavailable"
                    ),
                });
                continue;
            }
            if let Err(error) = ticket::update_ticket_status(&file_path, TicketStatus::Blocked) {
                self.log_activity(ActivityEvent::Error {
                    message: format!(
                        "Cannot park orphaned Review block for {ticket_id}: failed to write blocked status: {error}"
                    ),
                });
                continue;
            }

            let retry_progress = (remedy_owner == RemedyOwner::Agent)
                .then(|| {
                    self.agent_block_retries
                        .get(&ticket_id)
                        .copied()
                        .unwrap_or(0)
                })
                .filter(|count| *count > 0)
                .map(|count| (count.min(MAX_AGENT_BLOCK_RETRIES), MAX_AGENT_BLOCK_RETRIES));
            self.append_review_block_transition(
                lease,
                remedy_owner,
                ParkingTransitionType::Park,
                retry_progress,
                remedy_owner == RemedyOwner::World,
                started_at,
            );
            self.release_slot_for_ticket(&ticket_id);
            self.threads.remove(&ticket_id);
            self.finish_up_sent.remove(&ticket_id);
            parked_any = true;
            self.log_activity(ActivityEvent::Info {
                message: format!(
                    "Parked orphaned Review block for {ticket_id} ({remedy_owner:?}): {ask}"
                ),
            });
        }

        if parked_any {
            self.rebuild_dag();
        }
    }

    /// Re-derive every current Review attempt's completion obligation from
    /// admitted artifacts and aggregate state. This is intentionally safe to
    /// call at every scheduler observation boundary.
    fn reconcile_review_completions(&mut self) {
        let candidates: Vec<(TicketId, AttemptLease)> = self
            .threads
            .iter()
            .filter(|(_, thread)| thread.status != lisa_core::types::ThreadStatus::Completed)
            .filter_map(|(ticket_id, thread)| {
                let source_lease = thread.attempt_lease.clone()?;
                if source_lease.ticket_id != *ticket_id
                    || !source_lease.is_current(self.current_leases.get(ticket_id))
                {
                    return None;
                }
                let dag_phase = self.dag.get_ticket(ticket_id).map(|ticket| ticket.phase);
                (thread.current_phase == Phase::Review
                    || matches!(dag_phase, Some(Phase::Review | Phase::Done)))
                .then_some((ticket_id.clone(), source_lease))
            })
            .collect();

        for (ticket_id, source_lease) in candidates {
            self.dispatch_completion(CompletionInput::Reconcile {
                ticket_id,
                source_lease,
            });
        }
    }

    /// Record observed activity for a pane: updates the slot's activity clock
    /// and, if a thread is running in that pane, the thread's inactivity clock.
    fn bump_pane_activity(&mut self, pane_id: u32) {
        let now = std::time::SystemTime::now();
        let mut ticket_id = None;
        if let Some(slot) = self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
            slot.last_activity_at = Some(now);
            ticket_id = slot.ticket_id.clone();
        }
        if let Some(tid) = ticket_id {
            if let Some(thread) = self.threads.get_mut(&tid) {
                thread.record_activity(now);
            }
        }
    }

    /// Scan for `.heartbeat` signal files written by the PostToolUse hook.
    ///
    /// Each heartbeat proves the session in that pane is actively making tool
    /// calls. Heartbeats reset both the thread's stuck/stale clocks and the
    /// pane's wind-down clock, so an active session is never flagged stuck,
    /// never reclaimed by a timeout, and never has its pane reused.
    fn check_heartbeat_signals(&mut self) {
        for record in signal::ingest(&self.signal_dir, SignalRequest::Heartbeats) {
            let SignalRecord::Heartbeat {
                pane_id,
                lease: candidate,
            } = record
            else {
                continue;
            };
            let admitted = self
                .agent_slots
                .iter()
                .find(|slot| slot.pane_id == pane_id)
                .is_some_and(|slot| {
                    slot.ticket_id.as_deref() == Some(candidate.ticket_id.as_str())
                        && slot.attempt_lease.as_ref() == Some(&candidate)
                        && candidate.is_current(self.current_leases.get(&candidate.ticket_id))
                });
            if !admitted {
                continue;
            }
            self.bump_pane_activity(pane_id);
            // A heartbeat proves genuine progress — clear any attention debounce
            // so a pane that resumes and later re-stalls can notify again.
            self.notified_attention.remove(&pane_id);
            // A real tool call means an AskUserQuestion (if any) was answered and
            // the agent resumed — stop suppressing injection into this pane.
            self.awaiting_human.remove(&pane_id);
        }
    }

    /// Consume provider-neutral process-start signals and promote only the
    /// exact current fresh attempt assigned to the addressed physical seat.
    fn check_process_start_signals(&mut self) {
        for record in signal::ingest(&self.signal_dir, SignalRequest::ProcessStarts) {
            let SignalRecord::ProcessStarted {
                pane_id,
                lease: candidate,
            } = record
            else {
                continue;
            };
            self.acknowledge_process_start(pane_id, &candidate);
        }
    }

    /// Consume attempt-scoped proof that an interrupted pane executed a command
    /// at its shell boundary. Only the exact reset successor may relaunch.
    fn check_shell_ready_signals(&mut self) {
        for record in signal::ingest(&self.signal_dir, SignalRequest::ShellReady) {
            let SignalRecord::ShellReady {
                pane_id,
                lease: candidate,
            } = record
            else {
                continue;
            };
            self.acknowledge_shell_ready(pane_id, &candidate, std::time::SystemTime::now());
        }
    }

    /// Consume agent-issued assignment claims and promote only the exact
    /// current nonce-bearing assignment retained for the addressed pane.
    fn check_claim_signals(&mut self) {
        for record in signal::ingest(&self.signal_dir, SignalRequest::Claims) {
            let SignalRecord::Claim { pane_id, claim } = record else {
                continue;
            };

            if self.admit_assignment_claim(pane_id, &claim) {
                self.bump_pane_activity(pane_id);
                self.log_activity(ActivityEvent::Info {
                    message: format!(
                        "Pane {} claimed {} attempt {} assignment",
                        pane_id, claim.ticket_id, claim.attempt_id
                    ),
                });
            }
        }
    }

    /// Consume raw provider `UserPromptSubmit` payloads and promote only the
    /// ticket/generation currently pending in the addressed physical seat.
    fn check_codex_ack_signals(&mut self) {
        for record in signal::ingest(&self.signal_dir, SignalRequest::CodexAcknowledgements) {
            let SignalRecord::CodexAcknowledgement { pane_id, payload } = record else {
                continue;
            };

            if self.acknowledge_codex_assignment(pane_id, &payload) {
                self.bump_pane_activity(pane_id);
                self.log_activity(ActivityEvent::Info {
                    message: format!("Pane {} acknowledged its assignment", pane_id),
                });
            }
        }
    }

    /// Consume `pane-<id>.awaiting` signals and flag those panes as blocked on a
    /// human-facing `AskUserQuestion`.
    ///
    /// The PreToolUse[AskUserQuestion] hook writes `pane-<id>.awaiting`
    /// unconditionally whenever an agent asks a question. While a pane is flagged,
    /// `send_line_to_pane` and every injection caller suppress writes so lisa never
    /// types over the question UI. The flag is cleared in `check_heartbeat_signals`
    /// on the pane's next heartbeat (the agent resumed real work).
    ///
    /// Must run **before** `check_idle_signals` so the flag gates this tick's
    /// consumers. Deliberately does NOT bump activity clocks — this gates writes
    /// only; a blocked-then-abandoned pane must still trip stale detection on the
    /// normal silence clock (reclaim exemption is T-020-04).
    fn check_awaiting_signals(&mut self) {
        for record in signal::ingest(&self.signal_dir, SignalRequest::Awaiting) {
            let SignalRecord::Awaiting { pane_id } = record else {
                continue;
            };
            if self.awaiting_human.insert(pane_id) {
                self.log_activity(ActivityEvent::Info {
                    message: format!(
                        "Pane {} awaiting human (AskUserQuestion) — suppressing injection",
                        pane_id
                    ),
                });
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

        for record in signal::ingest(&self.signal_dir, SignalRequest::Idle) {
            let SignalRecord::Idle { target } = record else {
                continue;
            };
            // Signal files are named pane-{pane_id}.idle — resolve ticket
            // from the agent slot that owns this pane. `idle_pane_id` is lifted
            // out of the parse branch so the IdleWithoutArtifact arm below can
            // debounce on it and export LISA_PANE_ID.
            let mut idle_pane_id: Option<u32> = None;
            let ticket_id: TicketId = match target {
                IdleTarget::Pane(pane_id) => {
                    idle_pane_id = Some(pane_id);
                    // A transition reserves the slot for its next ticket before the
                    // next prompt/CLI is actually sent. Any idle signal arriving in
                    // that window belongs to the previous session and must not
                    // advance the newly assigned ticket.
                    let slot = match self.agent_slots.iter().find(|s| s.pane_id == pane_id) {
                        Some(slot) if slot.transition_state == TransitionState::Idle => slot,
                        _ => continue,
                    };
                    let assigned_ticket = slot.ticket_id.clone();
                    // An idle signal is recent life — restart the wind-down clock.
                    self.bump_pane_activity(pane_id);
                    match assigned_ticket {
                        Some(tid) => tid,
                        None => continue,
                    }
                }
                // Legacy: {ticket_id}.idle (from older hook versions)
                IdleTarget::LegacyTicket(ticket_id) => ticket_id,
            };

            // Look up thread — signal only meaningful for running threads
            let (current_phase, source_lease) = match self.threads.get(&ticket_id) {
                Some(t) if t.status == lisa_core::types::ThreadStatus::Running => {
                    (t.current_phase, t.attempt_lease.clone())
                }
                _ => continue,
            };

            match current_phase {
                Phase::Implement => {
                    if let Err(error) =
                        self.admit_artifact(&ticket_id, source_lease.as_ref(), "progress.md")
                    {
                        self.log_activity(ActivityEvent::Error {
                            message: format!(
                                "Rejected idle progress publication for {}: {}",
                                ticket_id, error
                            ),
                        });
                    }
                    // Idle signal alone is the completion signal for Implement
                    let file_path = self.dag.get_ticket(&ticket_id).map(|t| t.file_path.clone());
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
                        thread.mark_phase_change(std::time::SystemTime::now());
                    }

                    // If review.md already exists (agent ran all phases in one
                    // session), advance straight to Done in the same tick.
                    // check_artifact_advances() already ran this cycle so it
                    // won't catch this transition.
                    if matches!(
                        self.admit_artifact(&ticket_id, source_lease.as_ref(), "review.md"),
                        Ok(true)
                    ) {
                        if let Some(source_lease) = source_lease {
                            self.dispatch_completion(CompletionInput::Idle {
                                ticket_id: ticket_id.clone(),
                                source_lease,
                            });
                        } else {
                            self.log_activity(ActivityEvent::Warning {
                                message: format!(
                                    "Rejected completion for {ticket_id} (Idle): no attempt lease"
                                ),
                            });
                        }
                    }
                }

                Phase::Research
                | Phase::Design
                | Phase::Structure
                | Phase::Plan
                | Phase::Review => {
                    // Need artifact + idle signal for these phases
                    let artifact_name = match current_phase.artifact_filename() {
                        Some(name) => name,
                        None => continue,
                    };
                    let artifact_admitted =
                        match self.admit_artifact(&ticket_id, source_lease.as_ref(), artifact_name)
                        {
                            Ok(admitted) => admitted,
                            Err(error) => {
                                self.log_activity(ActivityEvent::Error {
                                    message: format!(
                                        "Rejected idle artifact publication for {}: {}",
                                        ticket_id, error
                                    ),
                                });
                                false
                            }
                        };

                    if artifact_admitted {
                        let next_phase = match current_phase.next() {
                            Some(p) => p,
                            None => continue,
                        };

                        if next_phase == Phase::Done {
                            let source_lease = self
                                .threads
                                .get(&ticket_id)
                                .and_then(|thread| thread.attempt_lease.clone());
                            if let Some(source_lease) = source_lease {
                                self.dispatch_completion(CompletionInput::Idle {
                                    ticket_id: ticket_id.clone(),
                                    source_lease,
                                });
                            } else {
                                self.log_activity(ActivityEvent::Warning {
                                    message: format!(
                                        "Rejected completion for {ticket_id} (Idle): no attempt lease"
                                    ),
                                });
                            }
                            continue;
                        }

                        let file_path =
                            self.dag.get_ticket(&ticket_id).map(|t| t.file_path.clone());
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
                            thread.mark_phase_change(std::time::SystemTime::now());
                        }
                    } else {
                        // Idle without artifact — alert
                        let detail = format!(
                            "Agent idle in {} phase but {} not found",
                            current_phase, artifact_name
                        );
                        self.idle_alerts.push((ticket_id.clone(), detail.clone()));
                        self.log_activity(ActivityEvent::Warning {
                            message: format!("{}: {}", ticket_id, detail),
                        });

                        // Fire the `attention` notification once per stall. The
                        // debounce set suppresses re-firing while the pane stays
                        // stalled (idle prompts repeat ~60s); a heartbeat clears
                        // the entry so a resumed-then-re-stalled agent re-notifies.
                        if let Some(pane_id) = idle_pane_id {
                            if self.notified_attention.insert(pane_id) {
                                let env: Vec<(&str, String)> = vec![
                                    ("LISA_PANE_ID", pane_id.to_string()),
                                    ("LISA_TICKET_ID", ticket_id.clone()),
                                    ("LISA_REASON", "idle-without-artifact".to_string()),
                                ];
                                let notify_detail = format!(
                                    "{} idle in {} without {}",
                                    ticket_id, current_phase, artifact_name
                                );
                                self.fire_notify("attention", &notify_detail, &env);
                            }
                        }
                    }
                }

                _ => {
                    // Ready, Done — signal already cleaned up, nothing to do
                }
            }
        }
    }

    /// Scan for `.stopped` and `.cleared` signal files and advance the
    /// per-slot transition state machine accordingly.
    ///
    /// - `.stopped` → if slot is `WaitingForStop`, send `/clear` and move to `WaitingForClear`
    /// - `.cleared` → if slot is `WaitingForClear`, send the prompt and move to `Idle`
    ///
    /// Signal files are deleted immediately after reading (same as `.idle` signals).
    fn check_transition_signals(&mut self) {
        for record in signal::ingest(&self.signal_dir, SignalRequest::Transitions) {
            match record {
                SignalRecord::Stopped { pane_id } => {
                    // A stop signal is recent life — restart the wind-down
                    // clock. Agents often keep working past their stop signal.
                    self.bump_pane_activity(pane_id);
                    self.handle_stopped_signal(pane_id);
                }
                SignalRecord::Cleared { pane_id } => {
                    self.bump_pane_activity(pane_id);
                    self.handle_cleared_signal(pane_id);
                }
                _ => {}
            }
        }
    }

    /// Scan for `pane-<id>.error` signal files and fail the owning thread promptly.
    ///
    /// Emitted by adapters (native Codex on non-zero TUI exit, the JSON fallback
    /// on `turn.failed`, and future bridges) — never by Claude Code hooks, so this
    /// consumer is inert for Claude panes. On `.error` for a running thread it performs the same reclaim
    /// `check_session_timeouts` does on silence, but immediately: fail the thread,
    /// release its slot, remove it (so the ticket re-enters `get_ready_tickets` for
    /// retry), and surface a `Failed` alert. For an idle/unknown pane the file is
    /// consumed harmlessly (logged, no state change).
    ///
    /// Runs before `check_transition_timeouts` so an errored pane is failed, not
    /// force-advanced by the transition-timeout fallback. Presence is the signal;
    /// any body is ignored.
    fn check_error_signals(&mut self) -> Vec<FailureTransitionOutcome> {
        let mut outcomes = Vec::new();
        for record in signal::ingest(&self.signal_dir, SignalRequest::Errors) {
            let SignalRecord::Error { pane_id } = record else {
                continue;
            };

            if matches!(
                self.seat_assignment(pane_id),
                Some(SeatAssignmentState::Recovering { .. })
            ) {
                if let Some(outcome) =
                    self.fail_assignment_recovery(pane_id, "fresh Codex process reported an error")
                {
                    outcomes.push(outcome);
                }
                continue;
            }

            // Resolve the running thread that owns this pane. `threads` (not
            // `agent_slots`) is the authority on what is running; a slot binding can
            // be stale mid-transition or already released.
            let ticket_id = self
                .threads
                .iter()
                .find(|(_, t)| {
                    t.pane_id == pane_id && t.status == lisa_core::types::ThreadStatus::Running
                })
                .map(|(tid, _)| tid.clone());

            match ticket_id {
                Some(tid) => {
                    if let Some(thread) = self.threads.get_mut(&tid) {
                        thread.fail();
                    }
                    self.emit_provenance(&tid, RunOutcome::Failed, false);
                    self.release_slot_for_ticket(&tid);
                    self.threads.remove(&tid);
                    self.error_alerts.push((tid.clone(), pane_id));
                    self.log_activity(ActivityEvent::Error {
                        message: format!(
                            "{} reported an error on pane {} — marked failed for retry",
                            tid, pane_id
                        ),
                    });
                    outcomes.push(FailureTransitionOutcome::ErrorReclaimed {
                        pane_id,
                        ticket_id: tid,
                    });
                }
                None => {
                    self.log_activity(ActivityEvent::Info {
                        message: format!(
                            "Error signal for pane {} with no running thread — ignored",
                            pane_id
                        ),
                    });
                }
            }
        }
        outcomes
    }

    /// Handle a `.stopped` signal for the given pane.
    ///
    /// Two cases:
    /// 1. Slot is `WaitingForStop` (mid-transition): send `/clear` and advance to `WaitingForClear`.
    /// 2. Slot is `Idle` and ticket is in Review phase: auto-complete the ticket as Done.
    fn handle_stopped_signal(&mut self, pane_id: u32) {
        let slot_info = self
            .agent_slots
            .iter()
            .find(|s| s.pane_id == pane_id)
            .map(|s| (s.transition_state, s.ticket_id.clone()));

        let (transition_state, ticket_id) = match slot_info {
            Some((state, tid)) => (state, tid),
            None => return,
        };

        // Case 1: Mid-transition — send /clear
        if transition_state == TransitionState::WaitingForStop {
            // Never /clear a pane blocked on a question — would discard the agent's
            // session mid-question. Stay in WaitingForStop; retry once unblocked.
            if self.is_pane_awaiting(pane_id) {
                return;
            }
            self.send_line_to_pane("/clear", PaneId::Terminal(pane_id));
            if let Some(slot) = self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
                slot.transition_state = TransitionState::WaitingForClear;
                slot.transition_started_at = Some(std::time::SystemTime::now());
            }
            self.log_activity(ActivityEvent::Info {
                message: format!("Pane {} stopped, sent /clear", pane_id),
            });
            return;
        }

        // Case 2: Idle slot with Review-phase ticket — auto-complete
        if transition_state == TransitionState::Idle {
            if let Some(ref tid) = ticket_id {
                let is_review = self
                    .dag
                    .get_ticket(tid)
                    .map(|t| t.phase == Phase::Review)
                    .unwrap_or(false);

                let thread_completable = self
                    .threads
                    .get(tid)
                    .map(|t| t.status != lisa_core::types::ThreadStatus::Completed)
                    .unwrap_or(false);

                if is_review && thread_completable {
                    self.auto_complete_review(tid.clone(), pane_id);
                }
            }
        }
    }

    /// Route a stopped Review session through commit-gated completion.
    fn auto_complete_review(&mut self, ticket_id: TicketId, pane_id: u32) {
        let source_lease = self
            .agent_slots
            .iter()
            .find(|slot| {
                slot.pane_id == pane_id && slot.ticket_id.as_deref() == Some(ticket_id.as_str())
            })
            .and_then(|slot| slot.attempt_lease.clone());
        let Some(source_lease) = source_lease else {
            self.log_activity(ActivityEvent::Warning {
                message: format!(
                    "Rejected completion for {ticket_id} (Stopped({pane_id})): no attempt lease"
                ),
            });
            return;
        };
        self.dispatch_completion(CompletionInput::Stopped {
            ticket_id,
            pane_id,
            source_lease,
        });
    }

    /// Append one attempt-scoped transition that failed before provider
    /// ownership. The terminal assignment-state guard at each caller is the
    /// exact-once boundary; this method only validates and persists evidence.
    fn emit_assignment_transition(
        &mut self,
        pane_id: u32,
        ticket_id: &str,
        state: AssignmentState,
        reason: &str,
    ) -> bool {
        if self.ledger_path.as_os_str().is_empty() {
            return false;
        }
        let evidence = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id && slot.ticket_id.as_deref() == Some(ticket_id))
            .and_then(|slot| slot.attempt_lease.clone())
            .and_then(|attempt_lease| {
                let thread = self.threads.get(ticket_id)?;
                (attempt_lease.ticket_id == ticket_id
                    && thread.pane_id == pane_id
                    && thread.attempt_lease.as_ref() == Some(&attempt_lease))
                .then_some((attempt_lease, thread.client, thread.started_at))
            });
        let Some((attempt_lease, client, started_at)) = evidence else {
            self.log_activity(ActivityEvent::Warning {
                message: format!(
                    "assignment-transition provenance rejected for {} on pane {}: matching attempt evidence is missing or inconsistent",
                    ticket_id, pane_id
                ),
            });
            return false;
        };

        let started = provenance::system_time_to_epoch(started_at);
        let ended = provenance::system_time_to_epoch(std::time::SystemTime::now());
        let record = AssignmentTransitionRecord {
            schema_version: provenance::SCHEMA_VERSION,
            seal: self.config.completion_seal,
            record_type: ProvenanceRecordType::AssignmentTransition,
            ticket_id: ticket_id.to_string(),
            attempt_lease,
            pane_id,
            provider: Route::from_client(client).provider,
            state,
            reason: reason.to_string(),
            started_at: started,
            ended_at: ended,
            wall_clock_secs: ended.saturating_sub(started),
        };
        if let Err(error) =
            provenance::append_assignment_transition_record(&self.ledger_path, &record)
        {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "assignment-transition provenance write failed for {}: {}",
                    ticket_id, error
                ),
            });
            return false;
        }
        true
    }

    /// Append one current-attempt block retry or park transition.
    ///
    /// `retry_progress` is `(count, limit)` — the pair is only meaningful
    /// together, so it travels as one value.
    fn emit_review_block_transition(
        &mut self,
        ticket_id: &str,
        remedy_owner: RemedyOwner,
        record_type: ParkingTransitionType,
        retry_progress: Option<(u8, u8)>,
        recheck_eligible: bool,
        started_at: std::time::SystemTime,
    ) -> bool {
        if self.ledger_path.as_os_str().is_empty() {
            return false;
        }
        let attempt_lease = self
            .threads
            .get(ticket_id)
            .and_then(|thread| thread.attempt_lease.clone())
            .filter(|lease| {
                lease.ticket_id == ticket_id && lease.is_current(self.current_leases.get(ticket_id))
            });
        let Some(attempt_lease) = attempt_lease else {
            self.log_activity(ActivityEvent::Warning {
                message: format!(
                    "block-transition provenance rejected for {ticket_id}: current attempt evidence is missing or inconsistent"
                ),
            });
            return false;
        };

        self.append_review_block_transition(
            attempt_lease,
            remedy_owner,
            record_type,
            retry_progress,
            recheck_eligible,
            started_at,
        )
    }

    /// Append one block transition attributed to an explicit durable attempt.
    ///
    /// The lease is attribution, not restored execution authority. Live
    /// callers validate it in `emit_review_block_transition`; level-triggered
    /// orphan reconciliation validates it against the durable attempt tree.
    fn append_review_block_transition(
        &mut self,
        attempt_lease: AttemptLease,
        remedy_owner: RemedyOwner,
        record_type: ParkingTransitionType,
        retry_progress: Option<(u8, u8)>,
        recheck_eligible: bool,
        started_at: std::time::SystemTime,
    ) -> bool {
        if self.ledger_path.as_os_str().is_empty() {
            return false;
        }

        let ticket_id = attempt_lease.ticket_id.clone();
        let started = provenance::system_time_to_epoch(started_at);
        let ended = provenance::system_time_to_epoch(std::time::SystemTime::now());
        let (retry_count, retry_limit) = match retry_progress {
            Some((count, limit)) => (Some(count), Some(limit)),
            None => (None, None),
        };
        let record = ParkingTransitionRecord {
            schema_version: provenance::SCHEMA_VERSION,
            seal: self.config.completion_seal,
            record_type,
            ticket_id: ticket_id.clone(),
            attempt_lease,
            remedy_owner,
            retry_count,
            retry_limit,
            recheck_eligible,
            started_at: started,
            ended_at: ended,
            wall_clock_secs: ended.saturating_sub(started),
        };
        if let Err(error) = provenance::append_parking_transition_record(&self.ledger_path, &record)
        {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "block-transition provenance write failed for {ticket_id}: {error}"
                ),
            });
            return false;
        }
        true
    }

    /// Latest blocked-work transition per ticket from the mixed ledger.
    fn latest_parking_transitions(&self) -> HashMap<TicketId, ParkingTransitionRecord> {
        if self.ledger_path.as_os_str().is_empty() {
            return HashMap::new();
        }
        let Ok(ledger) = std::fs::read_to_string(&self.ledger_path) else {
            return HashMap::new();
        };
        let mut latest = HashMap::new();
        for record in ledger
            .lines()
            .filter_map(|line| serde_json::from_str::<ProvenanceLedgerRecord>(line).ok())
        {
            if let ProvenanceLedgerRecord::ParkingTransition(record) = record {
                latest.insert(record.ticket_id.clone(), record);
            }
        }
        latest
    }

    /// Record status-driven unparking without making provenance scheduling
    /// authority. The latest Park row supplies the interval start and attempt.
    fn reconcile_unpark_transitions(&mut self) {
        let reopened: Vec<ParkingTransitionRecord> = self
            .latest_parking_transitions()
            .into_values()
            .filter(|record| record.record_type == ParkingTransitionType::Park)
            .filter(|record| {
                self.dag
                    .get_ticket(&record.ticket_id)
                    .is_some_and(|ticket| {
                        ticket.status == TicketStatus::Open && ticket.phase != Phase::Done
                    })
            })
            .collect();

        for park in reopened {
            let ended = provenance::system_time_to_epoch(std::time::SystemTime::now());
            let unpark = ParkingTransitionRecord {
                schema_version: provenance::SCHEMA_VERSION,
                seal: park.seal,
                record_type: ParkingTransitionType::Unpark,
                ticket_id: park.ticket_id.clone(),
                attempt_lease: park.attempt_lease,
                remedy_owner: park.remedy_owner,
                retry_count: park.retry_count,
                retry_limit: park.retry_limit,
                recheck_eligible: park.recheck_eligible,
                started_at: park.started_at,
                ended_at: ended,
                wall_clock_secs: ended.saturating_sub(park.started_at),
            };
            // The durable open status starts a fresh scheduling episode even
            // when best-effort provenance cannot be appended.
            self.agent_block_retries.remove(&park.ticket_id);
            match provenance::append_parking_transition_record(&self.ledger_path, &unpark) {
                Ok(()) => {
                    self.log_activity(ActivityEvent::Info {
                        message: format!(
                            "Unparked {} after {}s; status open restored ordinary DAG eligibility",
                            park.ticket_id, unpark.wall_clock_secs
                        ),
                    });
                }
                Err(error) => self.log_activity(ActivityEvent::Error {
                    message: format!(
                        "unpark-transition provenance write failed for {}: {}",
                        park.ticket_id, error
                    ),
                }),
            }
        }
    }

    /// Append one provenance record for a finishing ticket-run (T-027-01).
    ///
    /// Called at each teardown site immediately **before** the thread is removed,
    /// so the thread's spawn-time facts (client, concurrency, `started_at`,
    /// `pane_id`) are still readable. Write-after by construction — the ticket
    /// frontmatter was already updated by the caller; this only appends to the
    /// ledger and never touches thread/slot state. A write error logs and is
    /// swallowed (never fatal to the loop). A no-op when `ledger_path` is unset
    /// (native tests that don't exercise the ledger).
    fn emit_provenance(&mut self, ticket_id: &str, outcome: RunOutcome, fenced: bool) -> bool {
        self.emit_provenance_with_note(ticket_id, outcome, fenced, None)
    }

    fn emit_provenance_with_note(
        &mut self,
        ticket_id: &str,
        outcome: RunOutcome,
        fenced: bool,
        completion_note: Option<DispositionNote>,
    ) -> bool {
        if self.ledger_path.as_os_str().is_empty() {
            return false;
        }
        let Some(thread) = self.threads.get(ticket_id) else {
            return false;
        };
        let Some(attempt_lease) = thread.attempt_lease.clone() else {
            self.log_activity(ActivityEvent::Warning {
                message: format!(
                    "provenance rejected for {}: active thread has no attempt lease",
                    ticket_id
                ),
            });
            return false;
        };
        let authoritative = outcome == RunOutcome::Done;
        if authoritative && !attempt_lease.is_current(self.current_leases.get(ticket_id)) {
            self.log_activity(ActivityEvent::Warning {
                message: format!(
                    "provenance rejected for {}: Done attempt {:?} is no longer current",
                    ticket_id, attempt_lease
                ),
            });
            return false;
        }
        let client = thread.client;
        let started = provenance::system_time_to_epoch(thread.started_at);
        let ended = provenance::system_time_to_epoch(std::time::SystemTime::now());
        let route = Route::from_client(client);
        let record = ProvenanceRecord {
            schema_version: provenance::SCHEMA_VERSION,
            seal: self.config.completion_seal,
            completion_note,
            ticket_id: ticket_id.to_string(),
            attempt_lease,
            outcome,
            authoritative,
            fenced,
            // requested == actual until per-pane routing (T-026-01) can differ them.
            requested: route.clone(),
            actual: route,
            started_at: started,
            ended_at: ended,
            wall_clock_secs: ended.saturating_sub(started),
            tokens_in: None,
            tokens_out: None,
            cost_usd: None,
            concurrency_at_spawn: thread.concurrency_at_spawn,
            pane_id: thread.pane_id,
        };
        let (tokens_in, tokens_out, cost_usd) = self.read_usage(client, &record);
        let record = ProvenanceRecord {
            tokens_in,
            tokens_out,
            cost_usd,
            ..record
        };
        if let Err(e) = provenance::append_record(&self.ledger_path, &record) {
            self.log_activity(ActivityEvent::Error {
                message: format!("provenance write failed for {}: {}", ticket_id, e),
            });
            return false;
        }
        true
    }

    /// Sum capture rows uniquely owned by the current ticket's pane-time window.
    ///
    /// Prior execution records provide durable ownership for recycled panes;
    /// `current` closes the still-in-memory interval that has not been appended
    /// yet. Assignment-transition rows never establish provider ownership. A
    /// missing ledger, malformed row, or capture without a unique owner cannot
    /// fabricate usage. Capture rows contain no dollar-cost observation, so
    /// `cost_usd` remains `None`.
    fn read_usage(
        &mut self,
        client: AgentClient,
        current: &ProvenanceRecord,
    ) -> (Option<u64>, Option<u64>, Option<f64>) {
        let dir = match client {
            AgentClient::Codex => &self.codex_dir,
            AgentClient::Claude => &self.claude_dir,
        };
        let raw = match std::fs::read_to_string(dir.join("captures.jsonl")) {
            Ok(s) => s,
            Err(_) => return (None, None, None),
        };

        let prior_records: Vec<ProvenanceRecord> = std::fs::read_to_string(&self.ledger_path)
            .ok()
            .map(|ledger| {
                ledger
                    .lines()
                    .filter_map(|line| serde_json::from_str::<ProvenanceLedgerRecord>(line).ok())
                    .filter_map(|record| match record {
                        ProvenanceLedgerRecord::Execution(record) => Some(record),
                        ProvenanceLedgerRecord::AssignmentTransition(_)
                        | ProvenanceLedgerRecord::ParkingTransition(_) => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut totals = None;
        for (source_index, line) in raw.lines().enumerate() {
            let Ok(capture) = serde_json::from_str::<CaptureRecord>(line) else {
                continue;
            };
            if capture.pane_id != current.pane_id || capture.captured_at > current.ended_at {
                continue;
            }

            match ownership::owner_at(
                prior_records.iter().chain(std::iter::once(current)),
                capture.pane_id,
                capture.captured_at,
            ) {
                Some(owner) if owner == current.ticket_id.as_str() => {}
                Some(_) => continue,
                None => {
                    let Some(source_line) = u64::try_from(source_index)
                        .ok()
                        .and_then(|index| index.checked_add(1))
                    else {
                        self.log_activity(ActivityEvent::Error {
                            message: format!(
                                "usage capture quarantine failed: {} capture ledger line is not representable as u64",
                                client
                            ),
                        });
                        break;
                    };
                    self.quarantine_capture(client, source_line, &capture);
                    continue;
                }
            }

            let (input_tokens, output_tokens) = totals.unwrap_or((0_u64, 0_u64));
            let Some(input_tokens) = input_tokens.checked_add(capture.input_tokens) else {
                return (None, None, None);
            };
            let Some(output_tokens) = output_tokens.checked_add(capture.output_tokens) else {
                return (None, None, None);
            };
            totals = Some((input_tokens, output_tokens));
        }

        match totals {
            Some((input_tokens, output_tokens)) => (Some(input_tokens), Some(output_tokens), None),
            None => (None, None, None),
        }
    }

    /// Persist one valid capture that has no unique pane-time owner and make
    /// that quarantine visible without assigning its tokens to any ticket.
    fn quarantine_capture(
        &mut self,
        client: AgentClient,
        source_line: u64,
        capture: &CaptureRecord,
    ) {
        let provider_dir = match client {
            AgentClient::Codex => self.codex_dir.clone(),
            AgentClient::Claude => self.claude_dir.clone(),
        };
        let path = quarantine::session_path(&provider_dir, &capture.session_id);

        match quarantine::append(&provider_dir, source_line, capture) {
            Ok(quarantine::AppendOutcome::Appended(path)) => {
                self.log_activity(ActivityEvent::Warning {
                    message: format!(
                        "usage capture quarantined: client={} session={:?} pane=#{} captured_at={} path={}",
                        client,
                        capture.session_id,
                        capture.pane_id,
                        capture.captured_at,
                        path.display(),
                    ),
                });
            }
            Ok(quarantine::AppendOutcome::AlreadyPresent(_)) => {}
            Err(error) => {
                self.log_activity(ActivityEvent::Error {
                    message: format!(
                        "usage capture quarantine failed: client={} session={:?} pane=#{} captured_at={} source_line={} path={}: {}",
                        client,
                        capture.session_id,
                        capture.pane_id,
                        capture.captured_at,
                        source_line,
                        path.display(),
                        error,
                    ),
                });
            }
        }
    }

    /// Handle a `.cleared` signal for the given pane.
    /// If the slot is waiting for clear, send the new ticket prompt and return to `Idle`.
    fn handle_cleared_signal(&mut self, pane_id: u32) {
        // Check state and collect data before mutating, to avoid borrow conflicts.
        let action = self
            .agent_slots
            .iter()
            .find(|s| s.pane_id == pane_id)
            .and_then(|slot| {
                if slot.transition_state == TransitionState::WaitingForClear {
                    slot.ticket_id.clone()
                } else {
                    None
                }
            });

        if let Some(ticket_id) = action {
            // Don't type the next-ticket prompt over a question. Leave the slot in
            // WaitingForClear; the prompt goes out on a later tick once unblocked.
            if self.is_pane_awaiting(pane_id) {
                return;
            }
            let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);
            // Adapter owns the reuse prompt (native Claude → ticket_prompt).
            // Reuse path only needs the adapter; the route is surfaced at spawn.
            let (adapter, _route) = resolve_adapter_or_native(
                self.dag.get_ticket(&ticket_id),
                self.config.client,
                self.config.lisa_bin.as_deref(),
            );
            let Some(artifact_dir) = self.prompt_artifact_dir(&ticket_id, pane_id) else {
                return;
            };
            if let Err(error) = self.publish_prompt_lease_marker(&ticket_id, pane_id) {
                self.log_activity(ActivityEvent::Error {
                    message: format!(
                        "Cannot deliver prompt for {} on pane {}: {}",
                        ticket_id, pane_id, error
                    ),
                });
                return;
            }
            let artifact_dir = strip_host_prefix(&artifact_dir);
            let ctx = SpawnContext {
                ticket_dir: &host_ticket_dir,
                ticket_id: &ticket_id,
                pane_id,
                attempt_id: self
                    .pane_attempt_lease(pane_id)
                    .map_or(0, |lease| lease.attempt_id),
                artifact_dir: &artifact_dir,
                assignment_generation: self.active_assignment_generation(pane_id),
            };
            let prompt = adapter.reuse_prompt(&ctx);
            self.send_line_to_pane(&prompt, PaneId::Terminal(pane_id));
            self.start_assignment_ack_wait(pane_id, std::time::SystemTime::now());

            self.log_activity(ActivityEvent::Info {
                message: format!("Pane {} cleared, sent prompt for {}", pane_id, ticket_id),
            });

            if let Some(slot) = self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
                slot.transition_state = TransitionState::Idle;
                slot.transition_started_at = None;
            }
        }
    }

    /// Check for transition deadlines and advance stalled transitions.
    ///
    /// Prevents indefinite stalls if hooks fail to produce signal files.
    ///
    /// Busy-pane guard: a fallback only fires once the pane has also been
    /// signal-silent for the wind-down period. If the expected signal never
    /// arrives because the session is still working (heartbeats flowing), the
    /// transition waits rather than injecting input into a busy session.
    fn check_transition_timeouts(&mut self) {
        let evaluator = DeadlineEvaluator::new(SystemClock);
        let now = evaluator.now();
        let actions = evaluator.transitions(
            self.agent_slots.iter().map(|slot| TransitionInput {
                pane_id: slot.pane_id,
                ticket_id: slot.ticket_id.clone(),
                state: slot.transition_state,
                started: slot.transition_started_at,
                last_activity: slot.last_activity_at,
                awaiting_human: self.awaiting_human.contains(&slot.pane_id),
            }),
            TransitionPolicy {
                wind_down: std::time::Duration::from_secs(self.config.wind_down_secs),
                exit_grace_secs: AGENT_EXIT_GRACE_SECS,
                stop_timeout_secs: STOP_SIGNAL_TIMEOUT_SECS,
                clear_timeout_secs: CLEAR_SIGNAL_TIMEOUT_SECS,
            },
        );

        let mut exit_ready: Vec<(u32, Option<TicketId>)> = Vec::new();
        let mut stop_timeouts: Vec<u32> = Vec::new();
        let mut clear_timeouts: Vec<(u32, Option<TicketId>)> = Vec::new();
        for action in actions {
            match action {
                TransitionAction::ExitReady { pane_id, ticket_id } => {
                    exit_ready.push((pane_id, ticket_id));
                }
                TransitionAction::StopTimedOut { pane_id } => stop_timeouts.push(pane_id),
                TransitionAction::ClearTimedOut { pane_id, ticket_id } => {
                    clear_timeouts.push((pane_id, ticket_id));
                }
            }
        }

        for (pane_id, ticket_id) in exit_ready {
            let Some(ticket_id) = ticket_id else {
                // The pending ticket disappeared while the old client was
                // exiting. Leave a clean shell available to either provider.
                if let Some(slot) = self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
                    slot.transition_state = TransitionState::Idle;
                    slot.transition_started_at = None;
                    slot.has_session = false;
                    slot.last_client = None;
                }
                self.seat_assignments.remove(&pane_id);
                self.rename_slot(
                    pane_id,
                    format_pane_name(PaneName::Idle {
                        resident_agent: None,
                    }),
                );
                continue;
            };

            // `/exit` is documented to return immediately; the grace period is
            // deliberately longer than the deferred Enter delay. Any stale
            // question/attention marker belonged to the exited client and must
            // not suppress the fresh shell command.
            self.awaiting_human.remove(&pane_id);
            self.notified_attention.remove(&pane_id);

            let recovering = matches!(
                self.seat_assignment(pane_id),
                Some(SeatAssignmentState::Recovering { .. })
            );
            let recovery_generation = recovering
                .then(|| self.active_assignment_generation(pane_id))
                .flatten();

            let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);
            let (adapter, route) = resolve_adapter_or_native(
                self.dag.get_ticket(&ticket_id),
                self.config.client,
                self.config.lisa_bin.as_deref(),
            );
            if recovering && route.agent != AgentClient::Codex {
                if let Some(slot) = self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
                    slot.transition_state = TransitionState::Idle;
                    slot.transition_started_at = None;
                }
                self.fail_assignment_recovery(pane_id, "ticket route no longer resolves to Codex");
                continue;
            }
            let Some(artifact_dir) = self.prompt_artifact_dir(&ticket_id, pane_id) else {
                continue;
            };
            if let Err(error) = self.publish_prompt_lease_marker(&ticket_id, pane_id) {
                self.log_activity(ActivityEvent::Error {
                    message: format!(
                        "Cannot launch {} on pane {} after exit: {}",
                        ticket_id, pane_id, error
                    ),
                });
                continue;
            }
            let launch_artifact_dir = artifact_dir;
            let Some(launch_lease) = self.pane_attempt_lease(pane_id) else {
                if recovering {
                    self.fail_assignment_recovery(pane_id, "current attempt lease is missing");
                } else {
                    self.log_activity(ActivityEvent::Error {
                        message: format!(
                            "Cannot prepare assignment for {} on pane {} after exit: current attempt lease is missing",
                            ticket_id, pane_id
                        ),
                    });
                }
                continue;
            };
            let artifact_dir = strip_host_prefix(&launch_artifact_dir);
            let ctx = SpawnContext {
                ticket_dir: &host_ticket_dir,
                ticket_id: &ticket_id,
                pane_id,
                attempt_id: launch_lease.attempt_id,
                artifact_dir: &artifact_dir,
                assignment_generation: self.active_assignment_generation(pane_id),
            };
            let assignment_text = adapter.assignment_text(&ctx);
            let assignment_ref = match self.prepare_assignment(
                &launch_artifact_dir,
                &launch_lease,
                &assignment_text,
            ) {
                Ok(assignment_ref) => assignment_ref,
                Err(error) => {
                    if recovering {
                        self.fail_assignment_recovery(pane_id, &error);
                    } else {
                        self.log_activity(ActivityEvent::Error {
                            message: format!(
                                "Cannot prepare assignment for {} on pane {} after exit: {}",
                                ticket_id, pane_id, error
                            ),
                        });
                    }
                    continue;
                }
            };
            let assignment_path = strip_host_prefix(&assignment_ref.path);
            let payload = adapter.launch_command(&ctx, &assignment_path);
            let command = match Self::prepare_fresh_launch(&launch_artifact_dir, pane_id, &payload)
            {
                Ok(command) => command,
                Err(error) => {
                    self.log_activity(ActivityEvent::Error {
                        message: format!(
                            "Cannot launch {} on pane {} after exit: {}",
                            ticket_id, pane_id, error
                        ),
                    });
                    continue;
                }
            };
            self.send_line_to_pane(&command, PaneId::Terminal(pane_id));

            if let Some(slot) = self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
                slot.transition_state = TransitionState::Idle;
                slot.transition_started_at = None;
                slot.has_session = true;
                slot.last_client = Some(route.agent);
                slot.last_activity_at = Some(now);
            }
            if let Some(generation) = recovery_generation {
                // The one fresh fallback carries the successor's exact assignment
                // path through the same launcher. Process readiness and ownership
                // evidence remain independently gated.
                self.seat_assignments.insert(
                    pane_id,
                    SeatAssignmentState::Starting {
                        generation,
                        start_deadline: None,
                        relaunches: 0,
                    },
                );
                // Same launch-dispatch readiness read as the primary path, so a
                // recovery-relaunched Starting seat is also classified
                // (T-037-01-01).
                self.seat_readiness
                    .insert(pane_id, adapter.readiness_mode());
            }
            self.start_assignment_ack_wait(pane_id, now);
            if recovering {
                self.log_activity(ActivityEvent::SessionLaunch {
                    ticket_id: ticket_id.clone(),
                    pane_id,
                    command: command.clone(),
                });
            }
            self.log_activity(ActivityEvent::Info {
                message: format!(
                    "Pane {} exited previous client, launched {} for {}",
                    pane_id, route.agent, ticket_id
                ),
            });
        }

        for pane_id in stop_timeouts {
            self.log_activity(ActivityEvent::Warning {
                message: format!(
                    "Stop signal timeout for pane {}, sending /clear anyway",
                    pane_id
                ),
            });
            self.send_line_to_pane("/clear", PaneId::Terminal(pane_id));
            if let Some(slot) = self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
                slot.transition_state = TransitionState::WaitingForClear;
                slot.transition_started_at = Some(now);
            }
        }

        for (pane_id, ticket_id) in clear_timeouts {
            self.log_activity(ActivityEvent::Warning {
                message: format!(
                    "Clear signal timeout for pane {}, sending prompt anyway",
                    pane_id
                ),
            });
            if let Some(tid) = &ticket_id {
                let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);
                // Adapter owns the reuse prompt (native Claude → ticket_prompt).
                let (adapter, _route) = resolve_adapter_or_native(
                    self.dag.get_ticket(tid),
                    self.config.client,
                    self.config.lisa_bin.as_deref(),
                );
                let Some(artifact_dir) = self.prompt_artifact_dir(tid, pane_id) else {
                    continue;
                };
                if let Err(error) = self.publish_prompt_lease_marker(tid, pane_id) {
                    self.log_activity(ActivityEvent::Error {
                        message: format!(
                            "Cannot deliver timeout prompt for {} on pane {}: {}",
                            tid, pane_id, error
                        ),
                    });
                    continue;
                }
                let artifact_dir = strip_host_prefix(&artifact_dir);
                let ctx = SpawnContext {
                    ticket_dir: &host_ticket_dir,
                    ticket_id: tid,
                    pane_id,
                    attempt_id: self
                        .pane_attempt_lease(pane_id)
                        .map_or(0, |lease| lease.attempt_id),
                    artifact_dir: &artifact_dir,
                    assignment_generation: self.active_assignment_generation(pane_id),
                };
                let prompt = adapter.reuse_prompt(&ctx);
                self.send_line_to_pane(&prompt, PaneId::Terminal(pane_id));
                self.start_assignment_ack_wait(pane_id, now);
            }
            if let Some(slot) = self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
                slot.transition_state = TransitionState::Idle;
                slot.transition_started_at = None;
            }
        }
    }

    /// A pending transaction or a complete, valid Review makes the generic
    /// finish-your-review follow-up false. A narrative `review.md` alone is not
    /// enough: completion also requires an explicit pass/block disposition, so
    /// missing or malformed disposition evidence must keep the prompt armed.
    fn review_completion_suppresses_finish_up(&mut self, ticket_id: &str) -> bool {
        if self.pending_completions.contains_key(ticket_id) {
            return true;
        }
        let Some(source_lease) = self
            .threads
            .get(ticket_id)
            .and_then(|thread| thread.attempt_lease.clone())
        else {
            return false;
        };
        if !self.review_lease_is_current(ticket_id, &source_lease) {
            return false;
        }
        let inputs = self.review_completion_inputs(ticket_id, &source_lease);
        inputs.artifact_admission.is_some()
            && matches!(
                inputs.disposition,
                ReviewDisposition::Pass
                    | ReviewDisposition::Note(_)
                    | ReviewDisposition::Block { .. }
            )
    }

    /// Explain an attempt-local Review protocol blocker without mutating or
    /// publishing artifacts. This feeds the attention banner, where a precise
    /// missing/invalid disposition is more useful than generic phase inactivity.
    fn review_protocol_blocker(&self, ticket_id: &str) -> Option<(String, Vec<String>)> {
        let thread = self.threads.get(ticket_id)?;
        if thread.current_phase != Phase::Review {
            return None;
        }
        let lease = thread.attempt_lease.as_ref()?;
        if !self.review_lease_is_current(ticket_id, lease) {
            return None;
        }
        let attempt_dir = self.attempt_work_dir(lease);
        if !attempt_dir.join("review.md").is_file() {
            return None;
        }

        let disposition_path = attempt_dir.join("review-disposition.json");
        if !disposition_path.is_file() {
            return Some((
                "Missing review-disposition.json".to_string(),
                vec![
                    "Write pass/block disposition".to_string(),
                    "Check pane".to_string(),
                ],
            ));
        }

        match parse_review_disposition(&disposition_path) {
            ReviewDisposition::Pass | ReviewDisposition::Note(_) => None,
            ReviewDisposition::Block { reason, .. } => Some((
                format!("Review blocked: {reason}"),
                vec![
                    "Resolve review blocker".to_string(),
                    "Check pane".to_string(),
                ],
            )),
            ReviewDisposition::Invalid { reason } => Some((
                format!("Invalid review disposition: {reason}"),
                vec!["Fix disposition JSON".to_string(), "Check pane".to_string()],
            )),
        }
    }

    /// Check for running Review threads that have exceeded the review timeout.
    ///
    /// When a thread has been running in Review phase longer than `review_timeout_secs`
    /// without producing `review.md`, sends a finish-up prompt to prod the agent.
    ///
    /// Set `review_timeout_secs = 0` to disable this feature.
    fn check_review_timeouts(&mut self) {
        let evaluator = DeadlineEvaluator::new(SystemClock);
        let now = evaluator.now();
        let timeout = std::time::Duration::from_secs(self.config.review_timeout_secs);
        let wind_down = std::time::Duration::from_secs(self.config.wind_down_secs);
        let actions = evaluator.reviews(
            self.threads.iter().map(|(ticket_id, thread)| ReviewInput {
                ticket_id: ticket_id.clone(),
                pane_id: thread.pane_id,
                status: thread.status,
                phase: thread.current_phase,
                already_prompted: self.finish_up_sent.contains(ticket_id),
                awaiting_human: self.awaiting_human.contains(&thread.pane_id),
                last_phase_change: thread.last_phase_change,
                last_activity: thread.last_activity,
            }),
            timeout,
            wind_down,
        );

        for action in actions {
            let ticket_id = action.ticket_id;
            let pane_id = action.pane_id;
            if self.review_completion_suppresses_finish_up(&ticket_id) {
                continue;
            }
            let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);
            let host_work_dir = match self
                .threads
                .get(&ticket_id)
                .and_then(|thread| thread.attempt_lease.clone())
            {
                Some(lease) if lease.is_current(self.current_leases.get(&ticket_id)) => {
                    strip_host_prefix(&self.attempt_work_dir(&lease))
                }
                None if !self.current_leases.contains_key(&ticket_id) => {
                    strip_host_prefix(&self.config.work_dir.join(&ticket_id))
                }
                _ => continue,
            };
            // Adapter owns the follow-up mechanism. Native Claude and Codex type
            // the finish-up prompt into their live TUIs; headless/future bridges
            // may instead return a full spawn command.
            // Reuse path only needs the adapter; the route is surfaced at spawn.
            let (adapter, _route) = resolve_adapter_or_native(
                self.dag.get_ticket(&ticket_id),
                self.config.client,
                self.config.lisa_bin.as_deref(),
            );
            let follow_up = adapter.follow_up(&FollowUpContext {
                ticket_dir: &host_ticket_dir,
                work_dir: &host_work_dir,
                ticket_id: &ticket_id,
                pane_id,
            });
            match follow_up {
                // Both variants reach the pane the same way — send_line_to_pane is
                // the only pane I/O the WASM plugin has. The distinction is the
                // string: a live-TUI prompt vs a shell command for a headless or
                // bridged adapter.
                FollowUp::TypeIntoPane(prompt) => {
                    self.send_line_to_pane(&prompt, PaneId::Terminal(pane_id));
                }
                FollowUp::SpawnCommand(cmd) => {
                    self.send_line_to_pane(&cmd, PaneId::Terminal(pane_id));
                }
            }
            self.bump_pane_activity(pane_id);

            if let Some(thread) = self.threads.get_mut(&ticket_id) {
                thread.mark_phase_change(now);
            }

            self.finish_up_sent.insert(ticket_id.clone());
            self.log_activity(ActivityEvent::FinishUpPromptSent { ticket_id, pane_id });
        }
    }

    /// Evaluate health of all running threads and log state changes.
    ///
    /// Uses the configured `stuck_threshold_secs` as the warning threshold.
    /// Logs `HealthStateChanged` activity events when a thread transitions
    /// between health states (e.g., Healthy → Stuck).
    fn evaluate_health(&mut self) {
        use lisa_core::types::ThreadStatus;

        let threshold = std::time::Duration::from_secs(self.config.stuck_threshold_secs);
        let observations = DeadlineEvaluator::new(SystemClock).health(
            self.threads
                .iter()
                .filter(|(ticket_id, thread)| {
                    thread.status == ThreadStatus::Running
                        || thread.status == ThreadStatus::Failed
                        || !self.last_health.contains_key(*ticket_id)
                })
                .map(|(ticket_id, thread)| HealthInput {
                    ticket_id: ticket_id.clone(),
                    status: thread.status,
                    last_activity: thread.last_activity,
                    previous: self.last_health.get(ticket_id).copied(),
                }),
            threshold,
        );

        for observation in observations {
            let previous = observation
                .previous
                .unwrap_or(lisa_core::types::HealthStatus::Healthy);
            self.last_health
                .insert(observation.ticket_id.clone(), observation.current);
            if observation.current != previous {
                self.log_activity(ActivityEvent::HealthStateChanged {
                    ticket_id: observation.ticket_id,
                    old_health: previous,
                    new_health: observation.current,
                });
            }
        }

        // Clean up last_health for threads that no longer exist
        self.last_health
            .retain(|tid, _| self.threads.contains_key(tid));
    }

    /// Check for sessions that have exceeded the configured session timeout.
    ///
    /// When `session_timeout_secs > 0` and a running thread's total wall-clock
    /// time (since `started_at`) exceeds the limit, the thread is marked failed.
    /// Once it is also hard-silent, its lease is revoked and its terminal pane
    /// closed/fenced before the slot is released and the thread removed.
    ///
    /// Busy-pane guard: a session that is over budget but not provably dead
    /// is never reclaimed — interrupting a partially-done ticket wastes the
    /// work and forces a repeat attempt. A warning is logged once, and
    /// reclamation requires the same prolonged silence as stale detection
    /// (2x stuck_threshold_secs), so slow tests or long integration calls
    /// (multi-minute silent stretches) never get a progressing session
    /// reclaimed. Budgets warn; only silence kills.
    fn check_session_timeouts(&mut self) -> Vec<FailureTransitionOutcome> {
        let global_timeout = self.config.session_timeout_secs;
        let has_phase_timeouts = !self.config.phase_timeouts.is_empty();
        let hard_silence = std::time::Duration::from_secs(self.config.stuck_threshold_secs * 2);

        // If both global and per-phase timeouts are disabled, skip entirely
        if global_timeout == 0 && !has_phase_timeouts {
            return Vec::new();
        }

        let actions = DeadlineEvaluator::new(SystemClock).sessions(
            self.threads.iter().map(|(ticket_id, thread)| SessionInput {
                ticket_id: ticket_id.clone(),
                pane_id: thread.pane_id,
                status: thread.status,
                phase: thread.current_phase,
                pending_completion: self.pending_completions.contains_key(ticket_id),
                awaiting_human: self.awaiting_human.contains(&thread.pane_id),
                started_at: thread.started_at,
                last_phase_change: thread.last_phase_change,
                last_activity: thread.last_activity,
                phase_timeout: std::time::Duration::from_secs(
                    self.config.timeout_for_phase(thread.current_phase),
                ),
            }),
            std::time::Duration::from_secs(global_timeout),
            hard_silence,
        );
        let mut timed_out = Vec::new();
        let mut over_budget_active = Vec::new();
        for action in actions {
            match action {
                SessionAction::Warn(deadline) => over_budget_active.push(deadline),
                SessionAction::Reclaim(deadline) => timed_out.push(deadline),
            }
        }

        for deadline in over_budget_active {
            if self.over_budget_warned.insert(deadline.ticket_id.clone()) {
                self.log_activity(ActivityEvent::Warning {
                    message: format!(
                        "{} exceeded its timeout ({}s in {}) but is still active — \
                         waiting for it to wind down instead of interrupting",
                        deadline.ticket_id, deadline.elapsed_secs, deadline.phase
                    ),
                });
            }
        }

        let mut outcomes = Vec::new();
        for deadline in timed_out {
            let ticket_id = deadline.ticket_id;
            let pane_id = deadline.pane_id;
            if let Some(thread) = self.threads.get_mut(&ticket_id) {
                thread.fail();
            }
            let fenced = matches!(
                self.revoke_and_fence_attempt(&ticket_id),
                FenceOutcome::Fenced { .. } | FenceOutcome::AlreadyFenced { .. }
            );
            self.emit_provenance(&ticket_id, RunOutcome::TimedOut, fenced);
            self.release_slot_for_ticket(&ticket_id);
            self.threads.remove(&ticket_id);
            self.timeout_alerts
                .push((ticket_id.clone(), deadline.elapsed_secs, deadline.phase));
            self.log_activity(ActivityEvent::SessionTimedOut {
                ticket_id: ticket_id.clone(),
                elapsed_secs: deadline.elapsed_secs,
                phase: deadline.phase,
            });
            outcomes.push(FailureTransitionOutcome::SessionTimedOut {
                pane_id,
                ticket_id,
                fenced,
            });
        }
        outcomes
    }

    /// Detect threads that have been silent beyond the hard timeout.
    ///
    /// The hard timeout is 2x the configured stuck_threshold_secs of total
    /// inactivity — no heartbeats, signals, or phase changes. A session that
    /// is actively making tool calls never trips this, no matter how long its
    /// phase runs. Silent threads are marked as failed, their slots released,
    /// and they are removed from the threads map for retry.
    fn detect_stale_threads(&mut self) -> Vec<FailureTransitionOutcome> {
        // Hard timeout: 2x the configured stuck threshold
        let hard_timeout = std::time::Duration::from_secs(self.config.stuck_threshold_secs * 2);

        let stale = DeadlineEvaluator::new(SystemClock).stale(
            self.threads.iter().map(|(ticket_id, thread)| StaleInput {
                ticket_id: ticket_id.clone(),
                pane_id: thread.pane_id,
                status: thread.status,
                pending_completion: self.pending_completions.contains_key(ticket_id),
                awaiting_human: self.awaiting_human.contains(&thread.pane_id),
                last_activity: thread.last_activity,
            }),
            hard_timeout,
        );

        let mut outcomes = Vec::new();
        for action in stale {
            let ticket_id = action.ticket_id;
            let pane_id = action.pane_id;
            let mins = self.config.stuck_threshold_secs * 2 / 60;
            if let Some(thread) = self.threads.get_mut(&ticket_id) {
                thread.fail();
            }
            let fenced = matches!(
                self.revoke_and_fence_attempt(&ticket_id),
                FenceOutcome::Fenced { .. } | FenceOutcome::AlreadyFenced { .. }
            );
            self.emit_provenance(&ticket_id, RunOutcome::Failed, fenced);
            self.release_slot_for_ticket(&ticket_id);
            self.threads.remove(&ticket_id);
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "{} stale — no activity for {}+ minutes, marked failed for retry",
                    ticket_id, mins
                ),
            });
            outcomes.push(FailureTransitionOutcome::StaleThreadReclaimed {
                pane_id,
                ticket_id,
                fenced,
            });
        }
        outcomes
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
                if self.pending_completions.contains_key(*tid) {
                    return false;
                }
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
        (self.completion_journal_path.as_os_str().is_empty() || self.completion_journal_healthy)
            && !self.dag.is_empty()
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
        // Consume heartbeat signals first so activity clocks are current
        // before any health or timeout decisions this tick.
        self.check_heartbeat_signals();

        // Flag panes blocked on AskUserQuestion before any consumer can inject
        // into them this tick (must precede check_idle_signals and the timeout
        // fallbacks). Heartbeats above already cleared resumed panes.
        self.check_awaiting_signals();

        // Submit assignments that were already ready before this poll. New
        // process-start evidence is consumed afterwards so ReadyForAssignment
        // remains observable for one complete scheduler boundary.
        self.deliver_ready_assignments();

        // Exact provider start proves readiness only, never ticket ownership.
        self.check_process_start_signals();

        // An interrupted startup may relaunch only after its successor-scoped
        // shell probe positively executes in the same pane.
        self.check_shell_ready_signals();

        // The exact assignment claim is authoritative and gets the first
        // opportunity to own when multiple evidence tiers arrive together.
        self.check_claim_signals();

        // Matching provider prompt evidence remains a supplemental fast path
        // while the claim is pending.
        self.check_codex_ack_signals();

        // Admitted current-attempt workflow output is the final bounded
        // ownership fallback before timeout policy, as well as a phase edge.
        self.check_artifact_advances();

        // A complete blocking Review is a scheduler decision, not a completion
        // failure. Externally owned remedies park immediately; agent-owned
        // remedies consume their bounded per-loop retry before parking.
        self.apply_review_block_policy();

        // A writer can disappear between publishing its durable disposition
        // and the live-thread policy above. Re-derive that unmet park
        // obligation from the latest attempt generation on every poll.
        self.reconcile_orphaned_review_blocks();

        // Check for idle signals and advance phases / generate alerts
        self.check_idle_signals();

        // Level-triggered completion is re-derived after every source of phase
        // advancement and before Review timeout policy can inject a follow-up.
        self.reconcile_review_completions();

        // Process .stopped and .cleared signals for session transitions
        self.check_transition_signals();

        // Fail panes that reported an error before the transition-timeout fallback
        // can force-advance them (adapter-emitted; inert for Claude panes).
        self.check_error_signals();

        // Fallback: force-advance stalled transitions
        self.check_transition_timeouts();

        // Bound provider acceptance after actual tagged prompt delivery. Ack
        // signals above win at the deadline; transition delivery above arms the
        // clock before it is evaluated.
        self.check_assignment_ack_timeouts();

        // Send finish-up prompts to parked Review threads past timeout
        self.check_review_timeouts();

        // Evaluate health: log transitions (Healthy→Stuck, etc.)
        self.evaluate_health();

        // Check for sessions that exceeded the configured session timeout
        self.check_session_timeouts();

        // Detect and handle stale threads at hard timeout (2x threshold)
        self.detect_stale_threads();

        self.rebuild_dag();

        // An operator reopening a parked ticket restores ordinary DAG
        // eligibility directly. Provenance observes that durable state change
        // and never gates scheduling.
        self.reconcile_unpark_transitions();

        // Post-timeout/reload reconciliation routes externally observed Done
        // through the typed adapter. The pending mask prevents this path from
        // publishing while a command result is outstanding.
        let done_tickets: Vec<(TicketId, Option<AttemptLease>)> = self
            .threads
            .iter()
            .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
            .filter(|(tid, _)| {
                self.dag
                    .get_ticket(tid)
                    .map(|t| t.phase == Phase::Done)
                    .unwrap_or(false)
            })
            .map(|(tid, thread)| (tid.clone(), thread.attempt_lease.clone()))
            .collect();

        for (ticket_id, source_lease) in done_tickets {
            self.dispatch_completion(CompletionInput::ObservedDone {
                ticket_id,
                source_lease,
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
                        thread.mark_phase_change(std::time::SystemTime::now());
                    }
                }
            }
        }

        // Safety sweep: release any slots still pointing at done tickets
        self.sweep_stale_slots();

        // Audit threads: remove any orphaned entries for done/missing tickets
        self.audit_threads();

        // Clean up finish_up_sent for threads that no longer exist
        self.finish_up_sent
            .retain(|tid| self.threads.contains_key(tid));
        self.over_budget_warned
            .retain(|tid| self.threads.contains_key(tid));

        // Always try to schedule (slots may have freed up)
        self.schedule_ready_tickets();

        // Observable world-owned parks are verified by the native CLI without
        // blocking this WASM poll. The in-flight guard prevents overlap when a
        // check approaches the same duration as the poll interval.
        self.request_world_recheck();

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

            // Notify the operator that the loop finished. Fires once per
            // completion (timer not re-armed); re-fires if keep_working() resets
            // `terminated` and the DAG later drains again.
            let tickets_done = self
                .dag
                .tickets()
                .filter(|t| t.phase == Phase::Done)
                .count();
            let mut env: Vec<(&str, String)> =
                vec![("LISA_TICKETS_DONE", tickets_done.to_string())];
            if let Some(start) = self.loop_started_at {
                if let Ok(d) = std::time::SystemTime::now().duration_since(start) {
                    env.push(("LISA_DURATION_SECS", d.as_secs().to_string()));
                }
            }
            let detail = format!("{} tickets done", tickets_done);
            self.fire_notify("complete", &detail, &env);

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
                format!(
                    "TicketStatusChanged: {} {} -> {}",
                    ticket_id, old_status, new_status
                )
            }
            ActivityEvent::TicketPhaseChanged {
                ticket_id,
                old_phase,
                new_phase,
            } => {
                format!(
                    "TicketPhaseChanged: {} {} -> {}",
                    ticket_id, old_phase, new_phase
                )
            }
            ActivityEvent::ArtifactCreated {
                ticket_id,
                phase,
                path,
            } => {
                format!(
                    "ArtifactCreated: {} {} {}",
                    ticket_id,
                    phase,
                    path.display()
                )
            }
            ActivityEvent::CommitMade {
                ticket_id,
                commit_hash,
            } => {
                format!("CommitMade: {} {}", ticket_id, commit_hash)
            }
            ActivityEvent::Error { message } => format!("Error: {}", message),
            ActivityEvent::CompletionRejected {
                ticket_id,
                kind,
                correlation_id,
                detail,
            } => format!(
                "CompletionRejected: {} {} correlation={} detail={}",
                ticket_id, kind, correlation_id, detail
            ),
            ActivityEvent::DagRecomputed { ticket_count } => {
                format!("DagRecomputed: {} tickets", ticket_count)
            }
            ActivityEvent::AllTicketsDone => "AllTicketsDone".to_string(),
            ActivityEvent::HealthStateChanged {
                ticket_id,
                old_health,
                new_health,
            } => {
                format!(
                    "HealthStateChanged: {} {:?} -> {:?}",
                    ticket_id, old_health, new_health
                )
            }
            ActivityEvent::Warning { message } => format!("Warning: {}", message),
            ActivityEvent::Info { message } => format!("Info: {}", message),
            ActivityEvent::PollSummary {
                ready,
                running,
                idle_slots,
            } => {
                format!(
                    "PollSummary: ready={} running={} idle_slots={}",
                    ready, running, idle_slots
                )
            }
            ActivityEvent::SessionLaunch {
                ticket_id,
                pane_id,
                command,
            } => {
                format!(
                    "SessionLaunch: {} pane=#{} cmd={}",
                    ticket_id, pane_id, command
                )
            }
            ActivityEvent::FinishUpPromptSent { ticket_id, pane_id } => {
                format!("FinishUpPromptSent: {} pane=#{}", ticket_id, pane_id)
            }
            ActivityEvent::SessionTimedOut {
                ticket_id,
                elapsed_secs,
                phase,
            } => {
                format!(
                    "SessionTimedOut: {} after {}s ({})",
                    ticket_id, elapsed_secs, phase
                )
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
        writeln!(
            out,
            "ticket_dir:          {}",
            self.config.ticket_dir.display()
        )
        .unwrap();
        writeln!(
            out,
            "story_dir:           {}",
            self.config.story_dir.display()
        )
        .unwrap();
        writeln!(
            out,
            "work_dir:            {}",
            self.config.work_dir.display()
        )
        .unwrap();
        writeln!(out, "max_threads:         {}", self.config.max_threads).unwrap();
        writeln!(out, "auto_advance:        {}", self.config.auto_advance).unwrap();
        writeln!(
            out,
            "stuck_threshold_secs: {}",
            self.config.stuck_threshold_secs
        )
        .unwrap();
        writeln!(
            out,
            "review_timeout_secs: {}",
            self.config.review_timeout_secs
        )
        .unwrap();
        writeln!(out).unwrap();

        // Plugin status
        writeln!(out, "=== Plugin Status ===").unwrap();
        writeln!(out, "initialized:         {}", self.initialized).unwrap();
        writeln!(out, "permissions_granted: {}", self.permissions_granted).unwrap();
        writeln!(out, "slots_discovered:    {}", self.slots_discovered).unwrap();
        writeln!(out, "terminated:          {}", self.terminated).unwrap();
        writeln!(out, "pending_timer_count: {}", self.pending_timer_count).unwrap();
        writeln!(out).unwrap();

        // Agent slot transition states
        writeln!(out, "=== Slot Transitions ===").unwrap();
        for slot in &self.agent_slots {
            let ticket = slot.ticket_id.as_deref().unwrap_or("(idle)");
            writeln!(
                out,
                "pane-{}: {:?} ticket={} has_session={}",
                slot.pane_id, slot.transition_state, ticket, slot.has_session
            )
            .unwrap();
        }
        writeln!(out).unwrap();

        // Tickets
        writeln!(out, "=== Tickets ===").unwrap();
        let mut ticket_list: Vec<_> = self.dag.tickets().collect();
        ticket_list.sort_by(|a, b| a.id.cmp(&b.id));
        writeln!(
            out,
            "{:<14} {:<12} {:<12} DEPENDS_ON",
            "ID", "PHASE", "STATUS"
        )
        .unwrap();
        for t in &ticket_list {
            let deps = if t.depends_on.is_empty() {
                "—".to_string()
            } else {
                t.depends_on.join(", ")
            };
            writeln!(
                out,
                "{:<14} {:<12} {:<12} {}",
                t.id, t.phase, t.status, deps
            )
            .unwrap();
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
                "{:<14} {:<6} {:<12} {:<10} {:<14} PHASE_CHG_AGO",
                "TICKET", "PANE", "PHASE", "STATUS", "STARTED_AGO"
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
                let status_str = format!("{:?}", thread.status);
                let started_str = format!("{}s", started_ago);
                let phase_str = format!("{}s", phase_chg_ago);
                writeln!(
                    out,
                    "{:<14} #{:<4} {:<12} {:<10} {:<14} {} [health: {:?}]",
                    tid,
                    thread.pane_id,
                    thread.current_phase,
                    status_str,
                    started_str,
                    phase_str,
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
            writeln!(out, "{:<8} {:<14} HAS_SESSION", "PANE", "TICKET").unwrap();
            for slot in &self.agent_slots {
                let ticket = slot.ticket_id.as_deref().unwrap_or("(idle)");
                writeln!(
                    out,
                    "#{:<7} {:<14} {}",
                    slot.pane_id, ticket, slot.has_session
                )
                .unwrap();
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
            // Quit-confirm modal has its own key handling
            if self.modal.mode == ModalMode::QuitConfirm {
                match key.bare_key {
                    BareKey::Char('q') => {
                        // Actually quit
                        self.modal.open = false;
                        quit_zellij();
                    }
                    BareKey::Enter => {
                        // Keep working: rescan, acquire new tickets, resume
                        self.keep_working();
                    }
                    BareKey::Esc => {
                        // Dismiss without quitting (back to dashboard)
                        self.modal.open = false;
                    }
                    _ => return false,
                }
                return true;
            }

            if self.modal.mode == ModalMode::MarkDone {
                if let Some(outcome) = self.modal.operator_outcome.as_ref() {
                    if outcome.is_pending() {
                        return false;
                    }
                    return match key.bare_key {
                        BareKey::Enter | BareKey::Esc | BareKey::Char('q') => {
                            self.modal.open = false;
                            true
                        }
                        _ => false,
                    };
                }
            }

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
                    let ticket_id = self.modal.ticket_ids.get(self.modal.cursor).cloned();
                    match self.modal.mode {
                        ModalMode::MarkDone => {
                            if let Some(ticket_id) = ticket_id {
                                self.mark_ticket_done(&ticket_id);
                            }
                        }
                        ModalMode::ResetTicket => {
                            if let Some(ticket_id) = ticket_id {
                                self.reset_ticket(&ticket_id);
                            }
                            self.modal.open = false;
                        }
                        ModalMode::QuitConfirm => {} // handled above
                    }
                }
                _ => return false,
            }
            return true;
        }

        // Normal mode: 'p' cycles preset views
        if key.bare_key == BareKey::Char('p') {
            self.view_preset = self.view_preset.next();
            self.scroll_offset = 0;
            return true;
        }

        // Normal mode: space toggles pause (stop scheduling new tickets)
        if key.bare_key == BareKey::Char(' ') {
            self.paused = !self.paused;
            self.log_activity(ActivityEvent::Info {
                message: if self.paused {
                    "Scheduling paused".to_string()
                } else {
                    "Scheduling resumed".to_string()
                },
            });
            return true;
        }

        // Normal mode: 'd' opens the mark-done modal
        if key.bare_key == BareKey::Char('d') {
            self.open_mark_done_modal();
            return true;
        }

        // Normal mode: 'r' opens the reset-ticket modal
        if key.bare_key == BareKey::Char('r') {
            self.open_reset_modal();
            return true;
        }

        // Normal mode: j/k scroll the dashboard
        if key.bare_key == BareKey::Char('j') || key.bare_key == BareKey::Down {
            self.scroll_offset += 1;
            return true;
        }
        if key.bare_key == BareKey::Char('k') || key.bare_key == BareKey::Up {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
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

        // Normal mode: 'q' tries to quit — shows confirmation if work remains
        if key.bare_key == BareKey::Char('q') {
            self.try_quit();
            return true;
        }

        false
    }

    /// Open the mark-done modal with a list of non-done tickets.
    fn open_mark_done_modal(&mut self) {
        // Show non-done tickets that do NOT have a running agent thread,
        // UNLESS the ticket is in Review phase (review tickets may have been
        // resumed by the review-timeout finish-up prompt but should still be
        // manually completable).
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
            .filter(|t| {
                // Always show tickets without a running agent
                if !running.contains(t.id.as_str()) {
                    return true;
                }
                // Review-phase tickets are manually completable
                if t.phase == Phase::Review {
                    return true;
                }
                // Implement-phase tickets where review.md exists — the agent
                // finished all phases but the transition didn't fire
                if t.phase == Phase::Implement {
                    let review_path = self.config.work_dir.join(&t.id).join("review.md");
                    return review_path.exists();
                }
                false
            })
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
            mode: ModalMode::MarkDone,
            new_ticket_ids: Vec::new(),
            operator_outcome: None,
        };
    }

    /// Request manual completion through the same isolated transaction.
    fn mark_ticket_done(&mut self, ticket_id: &str) {
        let dispatched = self.dispatch_completion(CompletionInput::OperatorRequested {
            ticket_id: ticket_id.to_string(),
            source: OperatorRequestSource::MarkDoneKey,
        });
        if dispatched {
            let correlation_id = self
                .pending_completions
                .get(ticket_id)
                .map(|pending| pending.completion_key.to_string());
            if let Some(correlation_id) = correlation_id {
                if self.operator_modal_targets(ticket_id) {
                    self.modal.operator_outcome = Some(OperatorModalOutcome::Pending {
                        ticket_id: ticket_id.to_string(),
                        correlation_id,
                    });
                }
            }
        }
    }

    /// Open the reset modal with tickets that are in non-ready, non-done phases.
    fn open_reset_modal(&mut self) {
        let mut ids: Vec<TicketId> = self
            .dag
            .tickets()
            .filter(|t| t.phase != Phase::Ready && t.phase != Phase::Done)
            .map(|t| t.id.clone())
            .collect();
        ids.sort();

        if ids.is_empty() {
            self.log_activity(ActivityEvent::Info {
                message: "No tickets to reset (all are ready or done)".to_string(),
            });
            return;
        }

        self.modal = MarkDoneModal {
            open: true,
            ticket_ids: ids,
            cursor: 0,
            mode: ModalMode::ResetTicket,
            new_ticket_ids: Vec::new(),
            operator_outcome: None,
        };
    }

    /// Reset a ticket back to ready phase for retry.
    fn reset_ticket(&mut self, ticket_id: &str) {
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

        // Update phase to ready
        if let Err(e) = ticket::update_ticket_phase(&file_path, Phase::Ready) {
            self.log_activity(ActivityEvent::Error {
                message: format!("Failed to reset {} phase: {}", ticket_id, e),
            });
            return;
        }

        // Update status to open
        if let Err(e) =
            ticket::update_ticket_status(&file_path, lisa_core::types::TicketStatus::Open)
        {
            self.log_activity(ActivityEvent::Error {
                message: format!("Failed to reset {} status: {}", ticket_id, e),
            });
        }

        self.log_activity(ActivityEvent::TicketPhaseChanged {
            ticket_id: tid.clone(),
            old_phase,
            new_phase: Phase::Ready,
        });

        // Kill thread and release slot if present
        if let Some(thread) = self.threads.get_mut(&tid) {
            thread.fail();
        }
        self.release_slot_for_ticket(&tid);
        self.threads.remove(&tid);

        // Rebuild DAG but don't schedule — user is likely paused
        self.rebuild_dag();
    }

    /// Try to quit: rescan tickets and show confirmation if there's undone or new work.
    /// If nothing remains, quit immediately.
    fn try_quit(&mut self) {
        // Rescan tickets from disk to detect any new ones
        let fresh_tickets = match ticket::scan_tickets(&self.config.ticket_dir) {
            Ok(t) => t,
            Err(_) => {
                // Can't scan — just quit
                quit_zellij();
                return;
            }
        };

        // Current DAG ticket IDs
        let current_ids: HashSet<&str> = self.dag.tickets().map(|t| t.id.as_str()).collect();

        // Undone tickets in the current DAG
        let mut undone: Vec<TicketId> = self
            .dag
            .tickets()
            .filter(|t| t.phase != Phase::Done)
            .map(|t| t.id.clone())
            .collect();
        undone.sort();

        // New tickets not in the current DAG (any phase)
        let mut new_tickets: Vec<TicketId> = fresh_tickets
            .iter()
            .filter(|t| !current_ids.contains(t.id.as_str()))
            .map(|t| t.id.clone())
            .collect();
        new_tickets.sort();

        if undone.is_empty() && new_tickets.is_empty() {
            // Nothing pending — quit immediately
            quit_zellij();
            return;
        }

        // Show confirmation modal
        self.modal = MarkDoneModal {
            open: true,
            ticket_ids: undone,
            cursor: 0,
            mode: ModalMode::QuitConfirm,
            new_ticket_ids: new_tickets,
            operator_outcome: None,
        };
    }

    /// Resume work after quit confirmation: rebuild DAG (acquires new tickets),
    /// clear terminated state, and re-arm the scheduler.
    fn keep_working(&mut self) {
        self.modal.open = false;
        self.terminated = false;
        self.rebuild_dag();
        self.schedule_ready_tickets();
        // Re-arm the poll timer if it was stopped
        if self.pending_timer_count == 0 {
            self.arm_timer(POLL_INTERVAL_SECS);
        }
        self.log_activity(ActivityEvent::Info {
            message: "Resuming — rescanned tickets and rebuilt DAG".to_string(),
        });
    }
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // Parse configuration
        self.config = PluginConfig::from_config_map(&configuration);
        self.git_root = self.config.git_root.clone();

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
        self.attempt_dir = host.join(".lisa/attempts");

        // Provenance ledger + per-provider usage-artifact directories.
        self.ledger_path = host.join(".lisa/provenance.jsonl");
        self.completion_journal_path = host.join(".lisa/completion-journal.jsonl");
        self.restore_completion_journal();
        self.codex_dir = host.join(".lisa/codex");
        self.claude_dir = host.join(".lisa/claude");

        // Absolute host project root (run_command runs on the host, where the
        // /host sandbox mount does not exist) and loop-start timestamp for
        // LISA_DURATION_SECS on completion.
        self.project_root = get_plugin_ids().initial_cwd;
        self.loop_started_at = Some(std::time::SystemTime::now());

        // Subscribe to the events we need
        subscribe(&[
            EventType::PaneUpdate,
            EventType::PermissionRequestResult,
            EventType::Timer,
            EventType::Key,
            EventType::RunCommandResult,
        ]);

        // Request permissions needed to write commands to agent terminal panes
        // and to invoke the on-notify hook on the host (RunCommands).
        request_permission(&[
            PermissionType::WriteToStdin,
            PermissionType::ChangeApplicationState,
            PermissionType::ReadApplicationState,
            PermissionType::RunCommands,
        ]);

        // Initial DAG build with startup diagnostics
        let commit_lock_path = PathBuf::from("/host/.lisa-commit.lock");
        let mut scan_result = match ticket::scan_tickets_with_diagnostics(&self.config.ticket_dir) {
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

        for scanned in &mut scan_result.tickets {
            self.mask_completion_transaction(scanned);
        }

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

        // Reconstruct an unpark interval if status was reopened while the loop
        // was stopped. Scheduling itself depends only on the scanned status.
        self.reconcile_unpark_transitions();

        // Park an unconsumed current-generation Block before permission or pane
        // events can schedule from the freshly scanned DAG.
        self.reconcile_orphaned_review_blocks();

        // A fresh State normally has no reconstructed attempt authority here,
        // making this a safe no-op. Any authority-preserving load path uses the
        // same level-triggered reconciliation boundary as polling.
        self.reconcile_review_completions();

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
                self.name_unnamed_idle_slots();
                // Start the poll timer
                self.arm_timer(POLL_INTERVAL_SECS);
                // Try to schedule immediately if slots are already discovered
                self.schedule_ready_tickets();
                // Run the first world check at loop start; later checks share
                // the existing poll cadence.
                self.request_world_recheck();
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
                // Each line has its own absolute deadline because Timer events
                // carry no caller identity. An unrelated poll timer may inspect
                // the queue, but cannot flush a freshly queued Codex prompt.
                self.flush_pending_enters(std::time::SystemTime::now());

                if self.timer_fired() {
                    self.poll_tick();
                }
                should_render = true;
            }

            Event::Key(key) => {
                should_render = self.handle_key(key);
            }

            Event::RunCommandResult(exit_code, stdout, stderr, context) => {
                if let Some(ticket_id) = context.get("lisa_completion") {
                    self.handle_completion_result(ticket_id, exit_code, stdout, stderr);
                    should_render = true;
                    return should_render;
                }
                if context.contains_key("lisa_world_recheck") {
                    self.handle_world_recheck_result(exit_code, stdout, stderr);
                    should_render = true;
                    return should_render;
                }
                // Only our on-notify invocations carry the `lisa_notify` context
                // key. Keep hook failures visible without spamming on success.
                if let Some(notify_event) = context.get("lisa_notify") {
                    match exit_code {
                        Some(0) => self.log_activity(ActivityEvent::Info {
                            message: format!("on-notify {} ok", notify_event),
                        }),
                        other => self.log_activity(ActivityEvent::Warning {
                            message: format!(
                                "on-notify {} failed (exit {:?})",
                                notify_event, other
                            ),
                        }),
                    }
                    should_render = true;
                }
            }

            _ => {}
        }

        should_render
    }

    fn render(&mut self, rows: usize, cols: usize) {
        if !self.initialized {
            println!("Lisa initializing...");
            return;
        }

        if self.terminated && !self.modal.open {
            println!("All tickets done. Lisa loop complete. Press [q] to quit.");
            return;
        }

        let ui_state = self.to_ui_state();
        ui::print_dashboard(&ui_state, rows, cols, self.scroll_offset);
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
                depends_on: t.depends_on.to_vec(),
            })
            .collect();

        let active_threads: Vec<ui::ActiveThread> = self
            .threads
            .values()
            .filter(|t| t.status == lisa_core::types::ThreadStatus::Running)
            .map(|t| {
                let slot_number = self
                    .agent_slots
                    .iter()
                    .position(|s| s.pane_id == t.pane_id)
                    .map(|i| i + 1)
                    .unwrap_or(0);
                ui::ActiveThread {
                    ticket_id: t.ticket_id.clone(),
                    phase: phase_to_ui_phase(t.current_phase),
                    started_at: Duration::from_secs(
                        t.started_at
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    ),
                    slot_number,
                    awaiting: self.is_pane_awaiting(t.pane_id),
                    // Surface the pane's resolved (provider, model) route
                    // (T-026-01). `None` for a thread spawned before routing.
                    route: t.route.as_ref().map(|r| r.display_cell()),
                }
            })
            .collect();

        let parked_threads: Vec<ui::ParkedThread> = self
            .threads
            .values()
            .filter(|t| t.status == lisa_core::types::ThreadStatus::Parked)
            .map(|t| {
                let slot_number = self
                    .agent_slots
                    .iter()
                    .position(|s| s.pane_id == t.pane_id)
                    .map(|i| i + 1)
                    .unwrap_or(0);
                ui::ParkedThread {
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
                    slot_number,
                }
            })
            .collect();

        let waiting_items: Vec<ui::WaitingItem> =
            lisa_core::parking::collect_parked_remedies(self.dag.tickets(), &self.config.work_dir)
                .into_iter()
                .filter_map(|remedy| match remedy.remedy_owner {
                    RemedyOwner::Operator => Some(ui::WaitingItem {
                        ticket_id: remedy.ticket_id,
                        ask: remedy.ask,
                        reason: remedy.reason,
                        checks_on_own: false,
                    }),
                    RemedyOwner::World => Some(ui::WaitingItem {
                        ticket_id: remedy.ticket_id,
                        ask: remedy.ask,
                        reason: remedy.reason,
                        checks_on_own: true,
                    }),
                    RemedyOwner::Agent => None,
                })
                .collect();

        let activity_log: Vec<ui::ActivityEntry> = self
            .activity_log
            .iter()
            .filter_map(activity_event_to_ui_entry)
            .collect();

        // Build health alerts from stuck/failed threads
        let now = std::time::SystemTime::now();
        let threshold = std::time::Duration::from_secs(self.config.stuck_threshold_secs);
        let mut alerts: Vec<ui::HealthAlert> = self
            .threads
            .values()
            .filter(|t| {
                t.status == lisa_core::types::ThreadStatus::Running
                    || t.status == lisa_core::types::ThreadStatus::Failed
            })
            .filter_map(|t| {
                let health = t.health(now, threshold);
                match health {
                    lisa_core::types::HealthStatus::Stuck => {
                        let (detail, suggested_actions) = self
                            .review_protocol_blocker(&t.ticket_id)
                            .unwrap_or_else(|| {
                                (
                                    format!(
                                        "No phase change for {}+ min",
                                        threshold.as_secs() / 60
                                    ),
                                    vec!["Check pane".to_string(), "Restart session".to_string()],
                                )
                            });
                        Some(ui::HealthAlert {
                            ticket_id: t.ticket_id.clone(),
                            alert_type: ui::AlertType::Stuck,
                            detail,
                            suggested_actions,
                        })
                    }
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

        // Append session timeout alerts
        for (ticket_id, elapsed_secs, phase) in &self.timeout_alerts {
            alerts.push(ui::HealthAlert {
                ticket_id: ticket_id.clone(),
                alert_type: ui::AlertType::TimedOut,
                detail: format!(
                    "Ran for {}m, timed out in {} phase",
                    elapsed_secs / 60,
                    phase
                ),
                suggested_actions: vec![
                    "Check pane output".to_string(),
                    "Increase session_timeout_secs".to_string(),
                ],
            });
        }

        // Append error-signal reclaims (adapter-emitted failures)
        for (ticket_id, pane_id) in &self.error_alerts {
            alerts.push(ui::HealthAlert {
                ticket_id: ticket_id.clone(),
                alert_type: ui::AlertType::Failed,
                detail: format!("Session reported an error (pane {})", pane_id),
                suggested_actions: vec!["Check pane output".to_string(), "Retry".to_string()],
            });
        }

        let slots: Vec<ui::SlotInfo> = self
            .agent_slots
            .iter()
            .enumerate()
            .map(|(i, s)| ui::SlotInfo {
                ticket_id: s.ticket_id.clone(),
                slot_number: i + 1,
                transitioning: s.transition_state != TransitionState::Idle
                    || s.cooldown_until
                        .is_some_and(|until| std::time::SystemTime::now() < until),
            })
            .collect();

        let seat_assignment_statuses = self
            .agent_slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                self.seat_assignment(slot.pane_id).map(|assignment| {
                    let status = match assignment {
                        SeatAssignmentState::Starting { .. }
                        | SeatAssignmentState::ResettingStartup { .. } => {
                            ui::SeatAssignmentStatus::Starting
                        }
                        SeatAssignmentState::ReadyForAssignment { .. } => {
                            ui::SeatAssignmentStatus::ReadyForAssignment
                        }
                        SeatAssignmentState::Delivering { .. } => {
                            ui::SeatAssignmentStatus::Delivering
                        }
                        SeatAssignmentState::DeliveredAwaitingClaim { .. } => {
                            ui::SeatAssignmentStatus::DeliveredAwaitingClaim
                        }
                        SeatAssignmentState::AssignedPendingAck { .. } => {
                            ui::SeatAssignmentStatus::AssignedPendingAck
                        }
                        SeatAssignmentState::Owned => ui::SeatAssignmentStatus::Owned,
                        SeatAssignmentState::Recovering { .. } => {
                            ui::SeatAssignmentStatus::Recovering
                        }
                        SeatAssignmentState::ClaimTimedOut => {
                            ui::SeatAssignmentStatus::ClaimTimedOut
                        }
                        SeatAssignmentState::RecoveryFailed => {
                            ui::SeatAssignmentStatus::RecoveryFailed
                        }
                        SeatAssignmentState::StartupFailed => {
                            ui::SeatAssignmentStatus::StartupFailed
                        }
                        SeatAssignmentState::DeliveryFailed => {
                            ui::SeatAssignmentStatus::DeliveryFailed
                        }
                    };
                    (i + 1, status)
                })
            })
            .collect();

        ui::PluginState {
            tickets,
            active_threads,
            parked_threads,
            waiting_items,
            activity_log,
            alerts,
            slots,
            seat_assignment_statuses,
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
                kind: match self.modal.mode {
                    ModalMode::MarkDone => ui::ModalKind::MarkDone,
                    ModalMode::ResetTicket => ui::ModalKind::ResetTicket,
                    ModalMode::QuitConfirm => ui::ModalKind::QuitConfirm,
                },
                new_ticket_ids: self.modal.new_ticket_ids.clone(),
                operator_outcome: self.modal.operator_outcome.as_ref().map(
                    |outcome| match outcome {
                        OperatorModalOutcome::Pending {
                            ticket_id,
                            correlation_id,
                        } => ui::OperatorModalOutcome::Pending {
                            ticket_id: ticket_id.clone(),
                            correlation_id: correlation_id.clone(),
                        },
                        OperatorModalOutcome::Accepted {
                            ticket_id,
                            correlation_id,
                        } => ui::OperatorModalOutcome::Accepted {
                            ticket_id: ticket_id.clone(),
                            correlation_id: correlation_id.clone(),
                        },
                        OperatorModalOutcome::Rejected {
                            ticket_id,
                            kind,
                            correlation_id,
                            detail,
                        } => ui::OperatorModalOutcome::Rejected {
                            ticket_id: ticket_id.clone(),
                            kind: *kind,
                            correlation_id: correlation_id.clone(),
                            detail: detail.clone(),
                        },
                    },
                ),
            },
            paused: self.paused,
            active_view: self.view_preset,
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
        lisa_core::types::TicketStatus::Open | lisa_core::types::TicketStatus::InProgress => {
            ui::TicketStatus::InProgress
        }
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
        ActivityEvent::CompletionRejected {
            ticket_id,
            kind,
            correlation_id,
            detail,
        } => ui::ActivityType::CompletionRejected {
            ticket_id: ticket_id.clone(),
            kind: *kind,
            correlation_id: correlation_id.clone(),
            detail: detail.clone(),
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
            ticket_id, command, ..
        } => ui::ActivityType::Info {
            ticket_id: ticket_id.clone(),
            message: if command.len() > 120 {
                format!("Launch: {}...", &command[..120])
            } else {
                format!("Launch: {}", command)
            },
        },
        ActivityEvent::FinishUpPromptSent { ticket_id, pane_id } => ui::ActivityType::Info {
            ticket_id: ticket_id.clone(),
            message: format!("Finish-up prompt sent (pane #{})", pane_id),
        },
        ActivityEvent::SessionTimedOut {
            ticket_id,
            elapsed_secs,
            phase,
        } => ui::ActivityType::Warning {
            ticket_id: ticket_id.clone(),
            message: format!(
                "Session timed out after {}m (in {} phase)",
                elapsed_secs / 60,
                phase,
            ),
        },
    };

    Some(ui::ActivityEntry {
        timestamp,
        activity,
    })
}

// Register the plugin with Zellij
#[cfg(target_arch = "wasm32")]
register_plugin!(State);

// Provide a no-op stub for the Zellij host function on native targets so the
// test binary can link.  The real implementation is injected by the Zellij WASM
// runtime at load time.
#[cfg(not(target_arch = "wasm32"))]
#[no_mangle]
pub extern "C" fn host_run_plugin_command() {}

#[cfg(test)]
mod tests {
    use super::*;
    use lisa_core::completion::CompletionSeal;
    use lisa_core::types::{ActivityEvent, Phase, TicketStatus};

    #[allow(dead_code)]
    mod preownership_status_surface {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../lisa-cli/src/preownership_status.rs"
        ));
    }

    mod hostile_order_regression;
    mod operator_recovery_matrix;
    mod signal_consumer_characterization;
    mod signal_ingestion_regression;

    #[test]
    fn completion_has_one_typed_request_gateway() {
        let source = include_str!("lib.rs");
        let production = source
            .split_once("#[cfg(test)]\nmod tests {")
            .expect("test module marker remains available")
            .0;

        assert!(!production.contains("fn request_completion("));
        assert!(!production.contains("fn request_review_completion("));

        let dispatch_start = production
            .find("fn dispatch_completion(")
            .expect("typed completion dispatcher exists");
        let executor_start = production
            .find("fn execute_completion_effect(")
            .expect("completion effect executor exists");
        assert!(dispatch_start < executor_start);

        let executor_call = "self.execute_completion_effect(";
        assert_eq!(production.matches(executor_call).count(), 1);
        assert_eq!(
            production[dispatch_start..executor_start]
                .matches(executor_call)
                .count(),
            1,
            "the sole production effect-executor call must be inside typed dispatch"
        );

        let executor_end = production[executor_start..]
            .find("fn is_commit_id(")
            .map(|offset| executor_start + offset)
            .expect("executor remains bounded by result validation");
        assert_eq!(
            production[executor_start..executor_end]
                .matches("run_command_with_env_variables_and_cwd(")
                .count(),
            1,
            "completion effects must have one host command launch boundary"
        );
    }

    #[test]
    fn named_completion_rejections_become_distinct_correlated_activity_events() {
        use lisa_core::completion::LaunchFailure;

        let mut state = State::default();
        let correlation =
            CompletionGenerationId::new(CompletionId::new("T-REJECT"), AttemptId::new("7"), 1);
        let cases = [
            (
                CompletionRejection::AlreadyPending {
                    completion_id: CompletionId::new("T-REJECT"),
                },
                CompletionRejectionKind::AlreadyPending,
            ),
            (
                CompletionRejection::StaleLease {
                    attempt_id: AttemptId::new("6"),
                },
                CompletionRejectionKind::StaleLease,
            ),
            (
                CompletionRejection::DispositionBlocked {
                    reason: "review requires action".to_string(),
                },
                CompletionRejectionKind::DispositionBlocked,
            ),
            (
                CompletionRejection::DependencyBlocked {
                    reason: "T-BLOCK is not done".to_string(),
                },
                CompletionRejectionKind::DependencyBlocked,
            ),
            (
                CompletionRejection::LaunchFailed {
                    source: LaunchFailure::new("host command unavailable"),
                },
                CompletionRejectionKind::LaunchFailed,
            ),
        ];

        for (rejection, _) in &cases {
            state.log_completion_rejection("T-REJECT", &correlation, rejection);
        }

        assert_eq!(state.activity_log.len(), cases.len());
        for (event, (_, expected_kind)) in state.activity_log.iter().zip(cases) {
            match event {
                ActivityEvent::CompletionRejected {
                    ticket_id,
                    kind,
                    correlation_id,
                    detail,
                } => {
                    assert_eq!(ticket_id, "T-REJECT");
                    assert_eq!(*kind, expected_kind);
                    assert_eq!(correlation_id, &correlation.to_string());
                    assert!(!detail.is_empty());
                }
                other => panic!("expected structured completion rejection, got {other:?}"),
            }
        }
    }

    /// Mirror production dispatch by installing one newly minted lease as the
    /// scheduler's high-water/current authority and stamping matching records.
    fn install_current_attempt(state: &mut State, ticket_id: &str) -> AttemptLease {
        let lease =
            AttemptLease::mint(ticket_id.to_string(), state.lease_high_water.get(ticket_id))
                .unwrap();
        state
            .lease_high_water
            .insert(ticket_id.to_string(), lease.clone());
        state
            .current_leases
            .insert(ticket_id.to_string(), lease.clone());
        if let Some(thread) = state.threads.get_mut(ticket_id) {
            thread.attempt_lease = Some(lease.clone());
        }
        if let Some(slot) = state
            .agent_slots
            .iter_mut()
            .find(|slot| slot.ticket_id.as_deref() == Some(ticket_id))
        {
            slot.attempt_lease = Some(lease.clone());
        }
        lease
    }

    fn write_canonical_review_disposition(state: &State, ticket_id: &str, disposition: &str) {
        let canonical = state.config.work_dir.join(ticket_id);
        std::fs::create_dir_all(&canonical).unwrap();
        std::fs::write(canonical.join("review-disposition.json"), disposition).unwrap();
    }

    fn write_review_disposition(state: &State, lease: &AttemptLease, disposition: &str) {
        let staged = state.attempt_work_dir(lease);
        std::fs::create_dir_all(&staged).unwrap();
        std::fs::write(staged.join("review-disposition.json"), disposition).unwrap();
    }

    fn write_passing_review_disposition(state: &State, lease: &AttemptLease) {
        write_review_disposition(state, lease, r#"{"disposition":"pass","reason":null}"#);
    }

    fn t046_completion_note() -> DispositionNote {
        DispositionNote::new(
            "approximately 200 MiB",
            "docs/active/work/T-046-06-03/cbt-0716-210943-closing-codex/run-record.md",
            "The 225 MiB measurement supports completion while the written gate is stale.",
        )
        .unwrap()
    }

    fn write_t046_note_disposition(state: &State, lease: &AttemptLease) {
        write_review_disposition(
            state,
            lease,
            r#"{"disposition":"note","reason":null,"criterion_quote":"approximately 200 MiB","evidence_citation":"docs/active/work/T-046-06-03/cbt-0716-210943-closing-codex/run-record.md","summary":"The 225 MiB measurement supports completion while the written gate is stale."}"#,
        );
    }

    fn write_block_policy_ticket(tickets_dir: &Path, ticket_id: &str, phase: Phase) {
        let phase = phase.to_string();
        std::fs::write(
            tickets_dir.join(format!("{ticket_id}.md")),
            format!(
                "---\nid: {ticket_id}\ntitle: block policy {ticket_id}\ntype: task\nstatus: open\npriority: high\nphase: {phase}\n---\n\nFixture\n"
            ),
        )
        .unwrap();
    }

    fn world_recheck_state(root: &Path, ticket_id: &str) -> State {
        let tickets_dir = root.join("tickets");
        let work_dir = root.join("work");
        let ledger_path = root.join("provenance.jsonl");
        std::fs::create_dir_all(&tickets_dir).unwrap();
        write_block_policy_ticket(&tickets_dir, ticket_id, Phase::Review);
        ticket::update_ticket_status(
            tickets_dir.join(format!("{ticket_id}.md")),
            TicketStatus::Blocked,
        )
        .unwrap();
        let tickets = ticket::scan_tickets(&tickets_dir).unwrap();
        let lease = AttemptLease::mint(ticket_id.to_string(), None).unwrap();
        let park = ParkingTransitionRecord {
            schema_version: provenance::SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            record_type: ParkingTransitionType::Park,
            ticket_id: ticket_id.to_string(),
            attempt_lease: lease.clone(),
            remedy_owner: RemedyOwner::World,
            retry_count: None,
            retry_limit: None,
            recheck_eligible: true,
            started_at: 10,
            ended_at: 20,
            wall_clock_secs: 10,
        };
        provenance::append_parking_transition_record(&ledger_path, &park).unwrap();

        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                max_threads: 1,
                wind_down_secs: 0,
                lisa_bin: Some("/opt/lisa bin".to_string()),
                ..PluginConfig::new()
            },
            project_root: root.to_path_buf(),
            git_root: root.to_path_buf(),
            attempt_dir: root.join("attempts"),
            signal_dir: root.join("signals"),
            ledger_path,
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };
        state.lease_high_water.insert(ticket_id.to_string(), lease);
        state.agent_slots.push(fresh_slot(30, None));
        write_canonical_review_disposition(
            &state,
            ticket_id,
            r#"{"disposition":"block","reason":"release missing","remedy_owner":"world","ask":"Wait for the release.","check":"test -f release"}"#,
        );
        state
    }

    #[test]
    fn world_recheck_command_is_exact_and_requires_host_boundaries() {
        let mut state = State {
            config: PluginConfig {
                lisa_bin: Some("/opt/lisa bin".to_string()),
                ..PluginConfig::new()
            },
            project_root: PathBuf::from("/project with spaces"),
            ..State::default()
        };

        let (argv, context) = state.build_world_recheck_command().unwrap();
        assert_eq!(
            argv,
            vec![
                "/opt/lisa bin",
                "recheck-world",
                "--path",
                "/project with spaces"
            ]
        );
        assert_eq!(
            context.get("lisa_world_recheck").map(String::as_str),
            Some("world")
        );

        state.config.lisa_bin = None;
        assert_eq!(
            state.build_world_recheck_command().unwrap_err(),
            "lisa_bin is not configured"
        );
        state.config.lisa_bin = Some("lisa".to_string());
        state.project_root = PathBuf::new();
        assert_eq!(
            state.build_world_recheck_command().unwrap_err(),
            "project root is not available"
        );
    }

    #[test]
    fn world_recheck_eligibility_requires_world_owner_and_check() {
        let dir = tempfile::tempdir().unwrap();
        let state = world_recheck_state(dir.path(), "T-WORLD");
        assert!(state.has_observable_world_park());

        write_canonical_review_disposition(
            &state,
            "T-WORLD",
            r#"{"disposition":"block","reason":"approval missing","remedy_owner":"operator","ask":"Approve the release.","check":"test -f release"}"#,
        );
        assert!(!state.has_observable_world_park());

        write_canonical_review_disposition(
            &state,
            "T-WORLD",
            r#"{"disposition":"block","reason":"release missing","remedy_owner":"world","ask":"Wait for the release."}"#,
        );
        assert!(!state.has_observable_world_park());
    }

    #[test]
    fn world_recheck_runs_at_start_and_existing_poll_cadence_without_overlap() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = world_recheck_state(dir.path(), "T-WORLD");
        state.permissions_granted = false;

        state.update(Event::PermissionRequestResult(PermissionStatus::Granted));
        assert!(state.world_recheck_in_flight);
        assert!(
            !state.request_world_recheck(),
            "an in-flight check suppresses overlap"
        );

        state.handle_world_recheck_result(Some(0), Vec::new(), Vec::new());
        assert!(!state.world_recheck_in_flight);
        state.poll_tick();
        assert!(state.world_recheck_in_flight);
    }

    #[test]
    fn passing_world_recheck_records_unpark_and_seats_on_the_result_pass() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = world_recheck_state(dir.path(), "T-WORLD");
        ticket::update_ticket_status(
            state.config.ticket_dir.join("T-WORLD.md"),
            TicketStatus::Open,
        )
        .unwrap();
        state.world_recheck_in_flight = true;

        state.handle_world_recheck_result(Some(0), b"T-WORLD\n".to_vec(), Vec::new());

        assert!(!state.world_recheck_in_flight);
        assert_eq!(
            state.dag.get_ticket(&"T-WORLD".to_string()).unwrap().status,
            TicketStatus::Open
        );
        assert!(state.threads.contains_key("T-WORLD"));
        assert_eq!(state.current_leases["T-WORLD"].attempt_id, 2);
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-WORLD"));

        let records = read_mixed_ledger(&state.ledger_path);
        assert_eq!(records.len(), 2);
        let ProvenanceLedgerRecord::ParkingTransition(unpark) = &records[1] else {
            panic!("expected unpark provenance")
        };
        assert_eq!(unpark.record_type, ParkingTransitionType::Unpark);
        assert_eq!(unpark.remedy_owner, RemedyOwner::World);
        assert!(unpark.recheck_eligible);
        assert_eq!(unpark.attempt_lease.attempt_id, 1);

        state.reconcile_unpark_transitions();
        assert_eq!(read_mixed_ledger(&state.ledger_path).len(), 2);
    }

    #[test]
    fn unsuccessful_world_rechecks_clear_in_flight_without_durable_churn() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = world_recheck_state(dir.path(), "T-WORLD");
        let ticket_path = state.config.ticket_dir.join("T-WORLD.md");
        let ticket_before = std::fs::read(&ticket_path).unwrap();
        let ledger_before = std::fs::read(&state.ledger_path).unwrap();

        state.world_recheck_in_flight = true;
        state.handle_world_recheck_result(Some(0), Vec::new(), Vec::new());
        assert!(!state.world_recheck_in_flight);
        assert_eq!(std::fs::read(&ticket_path).unwrap(), ticket_before);
        assert_eq!(std::fs::read(&state.ledger_path).unwrap(), ledger_before);
        assert!(!state.threads.contains_key("T-WORLD"));

        state.world_recheck_in_flight = true;
        state.handle_world_recheck_result(Some(1), Vec::new(), b"failed".to_vec());
        assert!(!state.world_recheck_in_flight);
        assert_eq!(std::fs::read(&ticket_path).unwrap(), ticket_before);
        assert_eq!(std::fs::read(&state.ledger_path).unwrap(), ledger_before);
        assert!(!state.threads.contains_key("T-WORLD"));
    }

    #[test]
    fn dashboard_projection_reads_the_canonical_operator_ask_for_a_durable_park() {
        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let work_dir = dir.path().join("work");
        std::fs::create_dir_all(&tickets_dir).unwrap();
        std::fs::write(
            tickets_dir.join("T-ASK.md"),
            "---\nid: T-ASK\ntitle: parked ask\ntype: task\nstatus: blocked\npriority: high\nphase: review\n---\n\nFixture\n",
        )
        .unwrap();
        let state = State {
            dag: Dag::from_tickets(ticket::scan_tickets(&tickets_dir).unwrap()).unwrap(),
            config: PluginConfig {
                work_dir,
                ..PluginConfig::default()
            },
            ..State::default()
        };
        write_canonical_review_disposition(
            &state,
            "T-ASK",
            r#"{"disposition":"block","reason":"engineering reason","remedy_owner":"operator","ask":"Run the checkout test."}"#,
        );

        assert_eq!(
            state.to_ui_state().waiting_items,
            vec![ui::WaitingItem {
                ticket_id: "T-ASK".to_string(),
                ask: "Run the checkout test.".to_string(),
                reason: "engineering reason".to_string(),
                checks_on_own: false,
            }]
        );
    }

    fn attach_review_block_attempt(
        state: &mut State,
        ticket_id: &str,
        pane_id: u32,
        disposition: &str,
    ) -> AttemptLease {
        let slot = state
            .agent_slots
            .iter_mut()
            .find(|slot| slot.pane_id == pane_id)
            .expect("fixture pane exists");
        slot.ticket_id = Some(ticket_id.to_string());
        slot.transition_state = TransitionState::Idle;
        slot.cooldown_until = None;

        let mut thread = Thread::new(ticket_id, pane_id);
        thread.current_phase = Phase::Review;
        state.threads.insert(ticket_id.to_string(), thread);
        let lease = install_current_attempt(state, ticket_id);
        state
            .seat_assignments
            .insert(pane_id, SeatAssignmentState::Owned);

        let attempt_dir = state.attempt_work_dir(&lease);
        std::fs::create_dir_all(&attempt_dir).unwrap();
        std::fs::write(attempt_dir.join("review.md"), "# Review\n\nBlocked.\n").unwrap();
        std::fs::write(attempt_dir.join("review-disposition.json"), disposition).unwrap();
        lease
    }

    fn write_current_review_block(state: &mut State, ticket_id: &str, disposition: &str) {
        let lease = state.current_leases[ticket_id].clone();
        let pane_id = state.threads[ticket_id].pane_id;
        let slot = state
            .agent_slots
            .iter_mut()
            .find(|slot| slot.pane_id == pane_id)
            .expect("scheduled fixture pane exists");
        slot.transition_state = TransitionState::Idle;
        state
            .seat_assignments
            .insert(pane_id, SeatAssignmentState::Owned);
        let attempt_dir = state.attempt_work_dir(&lease);
        std::fs::create_dir_all(&attempt_dir).unwrap();
        std::fs::write(attempt_dir.join("review.md"), "# Review\n\nBlocked.\n").unwrap();
        std::fs::write(attempt_dir.join("review-disposition.json"), disposition).unwrap();
    }

    fn orphan_review_state(root: &Path, ticket_id: &str, pane_id: u32) -> State {
        let tickets_dir = root.join("tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();
        write_block_policy_ticket(&tickets_dir, ticket_id, Phase::Review);
        let tickets = ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir: root.join("work"),
                max_threads: 1,
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            attempt_dir: root.join("attempts"),
            signal_dir: root.join("signals"),
            ledger_path: root.join("provenance.jsonl"),
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };
        state.agent_slots.push(fresh_slot(pane_id, None));
        state
    }

    fn write_orphan_review_disposition(state: &State, lease: &AttemptLease, disposition: &str) {
        let work = state.attempt_work_dir(lease);
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("review-disposition.json"), disposition).unwrap();
        write_canonical_review_disposition(state, &lease.ticket_id, disposition);
    }

    #[test]
    fn orphaned_legacy_block_parks_at_load_boundary_without_spawning() {
        const TICKET_ID: &str = "T-ORPHAN-LOAD";
        const FIELD_REASON: &str = "The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant bypassed the old binary through managed mode instead of recording the required recovery through Lisa's error strings; John must either provide conforming reruns or explicitly amend both acceptance requirements before Review can pass.";

        let dir = tempfile::tempdir().unwrap();
        let mut state = orphan_review_state(dir.path(), TICKET_ID, 41);
        let lease = AttemptLease::mint(TICKET_ID, None).unwrap();
        let disposition = serde_json::json!({
            "disposition": "block",
            "reason": FIELD_REASON,
        })
        .to_string();
        write_orphan_review_disposition(&state, &lease, &disposition);

        // This is the production load ordering before permission/pane events
        // are allowed to schedule from the freshly scanned DAG.
        state.reconcile_unpark_transitions();
        state.reconcile_orphaned_review_blocks();
        state.schedule_ready_tickets();

        assert_eq!(
            state.dag.get_ticket(&TICKET_ID.to_string()).unwrap().status,
            TicketStatus::Blocked
        );
        assert!(
            std::fs::read_to_string(state.config.ticket_dir.join(format!("{TICKET_ID}.md")))
                .unwrap()
                .contains("status: blocked")
        );
        assert!(!state.threads.contains_key(TICKET_ID));
        assert!(!state.current_leases.contains_key(TICKET_ID));
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert_eq!(
            std::fs::read_to_string(
                state
                    .config
                    .work_dir
                    .join(TICKET_ID)
                    .join("review-disposition.json")
            )
            .unwrap(),
            disposition
        );
        assert_eq!(
            state.to_ui_state().waiting_items,
            vec![ui::WaitingItem {
                ticket_id: TICKET_ID.to_string(),
                ask: lisa_core::parking::LEGACY_BLOCK_ASK.to_string(),
                reason: FIELD_REASON.to_string(),
                checks_on_own: false,
            }]
        );

        let records = read_mixed_ledger(&state.ledger_path);
        assert_eq!(records.len(), 1);
        let ProvenanceLedgerRecord::ParkingTransition(park) = &records[0] else {
            panic!("expected orphan park provenance")
        };
        assert_eq!(park.record_type, ParkingTransitionType::Park);
        assert_eq!(park.attempt_lease, lease);
        assert_eq!(park.remedy_owner, RemedyOwner::Operator);

        state.reconcile_orphaned_review_blocks();
        state.schedule_ready_tickets();
        assert_eq!(read_mixed_ledger(&state.ledger_path).len(), 1);
        assert!(!state.threads.contains_key(TICKET_ID));
    }

    #[test]
    fn orphaned_block_appearing_after_thread_loss_parks_and_releases_seat() {
        const TICKET_ID: &str = "T-ORPHAN-MID";
        let dir = tempfile::tempdir().unwrap();
        let mut state = orphan_review_state(dir.path(), TICKET_ID, 42);
        let disposition = r#"{"disposition":"block","reason":"manual verification remains","remedy_owner":"operator","ask":"Run the checkout test."}"#;
        let lease = attach_review_block_attempt(&mut state, TICKET_ID, 42, disposition);
        write_canonical_review_disposition(&state, TICKET_ID, disposition);

        // The session disappears after writing its verdict but before the live
        // block-policy pass. Retain the slot/current lease to prove the durable
        // reconciliation does not require the thread and still releases both.
        state.threads.remove(TICKET_ID);
        state.reconcile_orphaned_review_blocks();

        assert_eq!(
            state.dag.get_ticket(&TICKET_ID.to_string()).unwrap().status,
            TicketStatus::Blocked
        );
        assert!(!state.threads.contains_key(TICKET_ID));
        assert!(!state.current_leases.contains_key(TICKET_ID));
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert_eq!(state.seat_assignment(42), None);
        let records = read_mixed_ledger(&state.ledger_path);
        assert_eq!(records.len(), 1);
        let ProvenanceLedgerRecord::ParkingTransition(park) = &records[0] else {
            panic!("expected mid-run orphan park provenance")
        };
        assert_eq!(park.attempt_lease, lease);
        assert_eq!(park.record_type, ParkingTransitionType::Park);

        state.schedule_ready_tickets();
        assert!(!state.threads.contains_key(TICKET_ID));
    }

    #[test]
    fn scheduling_parks_durable_block_then_unpark_seats_fresh_generation() {
        const TICKET_ID: &str = "T-ORPHAN-SCHEDULE";
        let dir = tempfile::tempdir().unwrap();
        let mut state = orphan_review_state(dir.path(), TICKET_ID, 43);
        let blocked = AttemptLease::mint(TICKET_ID, None).unwrap();
        let disposition = r#"{"disposition":"block","reason":"operator decision remains","remedy_owner":"operator","ask":"Choose whether to amend the requirement."}"#;
        write_orphan_review_disposition(&state, &blocked, disposition);

        // The scheduling entry point itself is the final admission guard.
        state.schedule_ready_tickets();
        assert_eq!(
            state.dag.get_ticket(&TICKET_ID.to_string()).unwrap().status,
            TicketStatus::Blocked
        );
        assert!(!state.threads.contains_key(TICKET_ID));
        assert!(!state.current_leases.contains_key(TICKET_ID));

        ticket::update_ticket_status(
            state.config.ticket_dir.join(format!("{TICKET_ID}.md")),
            TicketStatus::Open,
        )
        .unwrap();
        state.rebuild_dag();
        state.reconcile_unpark_transitions();
        state.schedule_ready_tickets();

        let fresh = state.current_leases[TICKET_ID].clone();
        assert_eq!(fresh.attempt_id, blocked.attempt_id + 1);
        assert_eq!(
            state.threads[TICKET_ID].attempt_lease.as_ref(),
            Some(&fresh)
        );
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some(TICKET_ID));
        assert_eq!(
            state.dag.get_ticket(&TICKET_ID.to_string()).unwrap().status,
            TicketStatus::Open
        );

        // Generation 1 remains on disk but cannot park the live generation 2.
        state.reconcile_orphaned_review_blocks();
        assert_eq!(state.current_leases.get(TICKET_ID), Some(&fresh));
        assert!(state.threads.contains_key(TICKET_ID));
        assert_eq!(
            std::fs::read_to_string(
                state
                    .attempt_work_dir(&blocked)
                    .join("review-disposition.json")
            )
            .unwrap(),
            disposition
        );

        let records = read_mixed_ledger(&state.ledger_path);
        assert_eq!(records.len(), 2);
        let transitions: Vec<_> = records
            .iter()
            .map(|record| match record {
                ProvenanceLedgerRecord::ParkingTransition(record) => record.record_type,
                other => panic!("expected parking transition, got {other:?}"),
            })
            .collect();
        assert_eq!(
            transitions,
            vec![ParkingTransitionType::Park, ParkingTransitionType::Unpark]
        );
    }

    #[test]
    fn stale_prior_generation_disposition_does_not_park_fresh_attempt() {
        const TICKET_ID: &str = "T-ORPHAN-STALE";
        let dir = tempfile::tempdir().unwrap();
        let mut state = orphan_review_state(dir.path(), TICKET_ID, 44);
        let stale = AttemptLease::mint(TICKET_ID, None).unwrap();
        write_orphan_review_disposition(
            &state,
            &stale,
            r#"{"disposition":"block","reason":"stale predecessor verdict","remedy_owner":"operator","ask":"Do not replay this old verdict."}"#,
        );

        // A later durable attempt directory is the current generation even
        // after its thread/lease disappears. Its missing disposition must not
        // fall back to generation 1's retained Block.
        let fresh = AttemptLease::mint(TICKET_ID, Some(&stale)).unwrap();
        std::fs::create_dir_all(state.attempt_work_dir(&fresh)).unwrap();
        let canonical = state
            .config
            .work_dir
            .join(TICKET_ID)
            .join("review-disposition.json");
        assert!(canonical.exists());
        state.reconcile_orphaned_review_blocks();
        assert_eq!(
            state.dag.get_ticket(&TICKET_ID.to_string()).unwrap().status,
            TicketStatus::Open
        );
        assert!(read_mixed_ledger(&state.ledger_path).is_empty());

        state.schedule_ready_tickets();

        let scheduled = state.current_leases[TICKET_ID].clone();
        assert_eq!(scheduled.attempt_id, fresh.attempt_id + 1);
        assert_eq!(
            state.dag.get_ticket(&TICKET_ID.to_string()).unwrap().status,
            TicketStatus::Open
        );
        assert_eq!(
            state.threads[TICKET_ID].attempt_lease.as_ref(),
            Some(&scheduled)
        );
        assert!(read_mixed_ledger(&state.ledger_path).is_empty());
        assert!(canonical.exists());
    }

    #[test]
    fn review_block_policy_has_exact_owner_and_retry_bound() {
        assert_eq!(
            review_block_action(RemedyOwner::Agent, 0),
            ReviewBlockAction::Retry {
                retry_count: 1,
                retry_limit: 2,
            }
        );
        assert_eq!(
            review_block_action(RemedyOwner::Agent, 1),
            ReviewBlockAction::Retry {
                retry_count: 2,
                retry_limit: 2,
            }
        );
        for consumed in [2, 3, u8::MAX] {
            assert_eq!(
                review_block_action(RemedyOwner::Agent, consumed),
                ReviewBlockAction::Park {
                    retry_count: Some(2),
                    retry_limit: Some(2),
                    recheck_eligible: false,
                }
            );
        }
        assert_eq!(
            review_block_action(RemedyOwner::Operator, 0),
            ReviewBlockAction::Park {
                retry_count: None,
                retry_limit: None,
                recheck_eligible: false,
            }
        );
        assert_eq!(
            review_block_action(RemedyOwner::World, 0),
            ReviewBlockAction::Park {
                retry_count: None,
                retry_limit: None,
                recheck_eligible: true,
            }
        );
    }

    #[test]
    fn park_instead_of_churn_replay_frees_two_seats_for_ready_work() {
        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();
        write_block_policy_ticket(&tickets_dir, "T-OPERATOR", Phase::Review);
        write_block_policy_ticket(&tickets_dir, "T-WORLD", Phase::Review);
        write_block_policy_ticket(&tickets_dir, "T-READY-A", Phase::Ready);
        write_block_policy_ticket(&tickets_dir, "T-READY-B", Phase::Ready);

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let ledger_path = dir.path().join("provenance.jsonl");
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: dir.path().join("work"),
                max_threads: 2,
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            attempt_dir: dir.path().join("attempts"),
            signal_dir: dir.path().join("signals"),
            ledger_path: ledger_path.clone(),
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };
        state.agent_slots.push(fresh_slot(10, None));
        state.agent_slots.push(fresh_slot(11, None));

        attach_review_block_attempt(
            &mut state,
            "T-OPERATOR",
            10,
            r#"{"disposition":"block","reason":"human tests required","remedy_owner":"operator","ask":"Run the release tests."}"#,
        );
        attach_review_block_attempt(
            &mut state,
            "T-WORLD",
            11,
            r#"{"disposition":"block","reason":"release is not published","remedy_owner":"world","ask":"Publish the release.","check":"test -f release"}"#,
        );
        assert_eq!(
            state
                .threads
                .values()
                .filter(|thread| thread.status == lisa_core::types::ThreadStatus::Running)
                .count(),
            2
        );

        state.apply_review_block_policy();

        for blocked in ["T-OPERATOR", "T-WORLD"] {
            assert!(!state.threads.contains_key(blocked));
            assert_eq!(
                state.dag.get_ticket(&blocked.to_string()).unwrap().status,
                TicketStatus::Blocked
            );
            assert!(!state.current_leases.contains_key(blocked));
            assert!(state
                .agent_slots
                .iter()
                .all(|slot| slot.ticket_id.as_deref() != Some(blocked)));
        }

        state.schedule_ready_tickets();
        let mut scheduled: Vec<_> = state.threads.keys().cloned().collect();
        scheduled.sort();
        assert_eq!(scheduled, vec!["T-READY-A", "T-READY-B"]);
        assert_eq!(
            state
                .agent_slots
                .iter()
                .filter(|slot| slot.ticket_id.is_some())
                .count(),
            2
        );

        state.schedule_ready_tickets();
        assert!(!state.threads.contains_key("T-OPERATOR"));
        assert!(!state.threads.contains_key("T-WORLD"));

        let records = read_mixed_ledger(&ledger_path);
        let parks: Vec<_> = records
            .iter()
            .filter_map(|record| match record {
                ProvenanceLedgerRecord::ParkingTransition(record)
                    if record.record_type == ParkingTransitionType::Park =>
                {
                    Some(record)
                }
                _ => None,
            })
            .collect();
        assert_eq!(parks.len(), 2);
        assert!(parks.iter().any(|record| {
            record.ticket_id == "T-OPERATOR"
                && record.remedy_owner == RemedyOwner::Operator
                && !record.recheck_eligible
        }));
        assert!(parks.iter().any(|record| {
            record.ticket_id == "T-WORLD"
                && record.remedy_owner == RemedyOwner::World
                && record.recheck_eligible
        }));
    }

    #[test]
    fn agent_owned_block_retries_exact_bound_then_parks_and_status_open_unparks() {
        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();
        write_block_policy_ticket(&tickets_dir, "T-AGENT", Phase::Review);
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let ledger_path = dir.path().join("provenance.jsonl");
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: dir.path().join("work"),
                max_threads: 1,
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            attempt_dir: dir.path().join("attempts"),
            signal_dir: dir.path().join("signals"),
            ledger_path: ledger_path.clone(),
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };
        state.agent_slots.push(fresh_slot(20, None));
        let disposition = r#"{"disposition":"block","reason":"agent fix remains","remedy_owner":"agent","ask":"Fix the remaining defect."}"#;
        let first = attach_review_block_attempt(&mut state, "T-AGENT", 20, disposition);
        assert_eq!(first.attempt_id, 1);

        for expected_attempt in [2, 3] {
            state.apply_review_block_policy();
            assert!(!state.threads.contains_key("T-AGENT"));
            assert_eq!(
                state.dag.get_ticket(&"T-AGENT".to_string()).unwrap().status,
                TicketStatus::Open
            );
            state.schedule_ready_tickets();
            let lease = state.current_leases["T-AGENT"].clone();
            assert_eq!(lease.attempt_id, expected_attempt);
            write_current_review_block(&mut state, "T-AGENT", disposition);
        }

        state.apply_review_block_policy();
        assert!(!state.threads.contains_key("T-AGENT"));
        assert_eq!(
            state.dag.get_ticket(&"T-AGENT".to_string()).unwrap().status,
            TicketStatus::Blocked
        );
        state.schedule_ready_tickets();
        assert!(!state.threads.contains_key("T-AGENT"));

        let records = read_mixed_ledger(&ledger_path);
        let transitions: Vec<_> = records
            .iter()
            .filter_map(|record| match record {
                ProvenanceLedgerRecord::ParkingTransition(record) => Some(record),
                _ => None,
            })
            .collect();
        assert_eq!(transitions.len(), 3);
        assert_eq!(transitions[0].record_type, ParkingTransitionType::Retry);
        assert_eq!(
            (transitions[0].retry_count, transitions[0].retry_limit),
            (Some(1), Some(2))
        );
        assert_eq!(transitions[1].record_type, ParkingTransitionType::Retry);
        assert_eq!(
            (transitions[1].retry_count, transitions[1].retry_limit),
            (Some(2), Some(2))
        );
        assert_eq!(transitions[2].record_type, ParkingTransitionType::Park);
        assert_eq!(
            (transitions[2].retry_count, transitions[2].retry_limit),
            (Some(2), Some(2))
        );
        assert_eq!(
            transitions
                .iter()
                .map(|record| record.attempt_lease.attempt_id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );

        ticket::update_ticket_status(
            tickets_dir.join("T-AGENT.md"),
            lisa_core::types::TicketStatus::Open,
        )
        .unwrap();
        state.rebuild_dag();
        state.reconcile_unpark_transitions();
        state.schedule_ready_tickets();

        assert!(state.threads.contains_key("T-AGENT"));
        assert_eq!(state.current_leases["T-AGENT"].attempt_id, 4);
        assert!(!state.agent_block_retries.contains_key("T-AGENT"));
        let records = read_mixed_ledger(&ledger_path);
        assert_eq!(records.len(), 4);
        let ProvenanceLedgerRecord::ParkingTransition(unpark) = &records[3] else {
            panic!("expected unpark provenance")
        };
        assert_eq!(unpark.record_type, ParkingTransitionType::Unpark);
        assert_eq!(unpark.ticket_id, "T-AGENT");
        assert_eq!(unpark.attempt_lease.attempt_id, 3);

        state.reconcile_unpark_transitions();
        assert_eq!(
            read_mixed_ledger(&ledger_path).len(),
            4,
            "latest Unpark makes reconciliation idempotent"
        );
    }

    /// Construct an expired Review attempt around real scanned ticket and work
    /// paths. A non-empty journal path also selects production launch-error
    /// handling in the native completion executor.
    fn review_timeout_state(
        ticket_id: &str,
        tickets_dir: PathBuf,
        work_dir: PathBuf,
        project_root: PathBuf,
        git_root: PathBuf,
        completion_journal_path: PathBuf,
    ) -> (State, AttemptLease) {
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir,
                lisa_bin: Some("lisa".to_string()),
                review_timeout_secs: 1,
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            project_root,
            git_root,
            completion_journal_path,
            completion_journal_healthy: true,
            ..State::default()
        };
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        let mut thread = lisa_core::types::Thread::new(ticket_id, 42);
        thread.current_phase = Phase::Review;
        thread.last_phase_change = old;
        thread.last_activity = old;
        state.threads.insert(ticket_id.to_string(), thread);
        let lease = install_current_attempt(&mut state, ticket_id);
        std::fs::create_dir_all(state.attempt_work_dir(&lease)).unwrap();
        (state, lease)
    }

    fn write_private_review(state: &State, lease: &AttemptLease) {
        std::fs::write(
            state.attempt_work_dir(lease).join("review.md"),
            "# Review\n\nReady to complete.\n",
        )
        .unwrap();
        write_passing_review_disposition(state, lease);
    }

    fn completion_failure_fixture(
        ticket_id: &str,
    ) -> (State, AttemptLease, tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let tickets_dir = root.join("tickets");
        let work_dir = root.join("work");
        let journal = root.join("completion-journal.jsonl");
        let ledger = root.join("provenance.jsonl");
        std::fs::create_dir_all(&tickets_dir).unwrap();
        std::fs::write(
            tickets_dir.join(format!("{ticket_id}.md")),
            format!(
                "---\nid: {ticket_id}\ntitle: bounded completion failure\ntype: bug\nstatus: open\npriority: critical\nphase: review\n---\n"
            ),
        )
        .unwrap();
        let (mut state, lease) = review_timeout_state(
            ticket_id,
            tickets_dir,
            work_dir,
            root.to_path_buf(),
            root.to_path_buf(),
            journal.clone(),
        );
        state.ledger_path = ledger.clone();
        state.attempt_dir = root.join("attempts");
        let mut slot = fresh_slot(42, None);
        slot.ticket_id = Some(ticket_id.to_string());
        slot.attempt_lease = Some(lease.clone());
        state.agent_slots.push(slot);
        state
            .seat_assignments
            .insert(42, SeatAssignmentState::Owned);
        std::fs::create_dir_all(state.attempt_work_dir(&lease)).unwrap();
        write_private_review(&state, &lease);
        (state, lease, dir, journal, ledger)
    }

    fn initialize_unborn_identityless_repository(root: &Path) {
        let run = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
        };
        run(&["init", "--quiet"]);
        run(&["config", "user.name", ""]);
        run(&["config", "user.email", ""]);

        for args in [
            &["rev-parse", "--verify", "HEAD"][..],
            &["var", "GIT_AUTHOR_IDENT"][..],
        ] {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .unwrap();
            assert!(
                !output.status.success(),
                "field replay precondition unexpectedly succeeded: git {:?}",
                args
            );
        }
    }

    fn real_unborn_completion_error(
        state: &State,
        lease: &AttemptLease,
        ticket_id: &str,
    ) -> String {
        let ticket_file = state
            .config
            .ticket_dir
            .join(format!("{ticket_id}.md"))
            .strip_prefix(&state.project_root)
            .unwrap()
            .to_path_buf();
        let work_dir = state
            .config
            .work_dir
            .join(ticket_id)
            .strip_prefix(&state.project_root)
            .unwrap()
            .to_path_buf();
        let error = lisa_cli::commit_transaction::complete_ticket(
            lisa_cli::commit_transaction::CompleteTicketRequest {
                repo_root: state.project_root.clone(),
                ticket_id: ticket_id.to_string(),
                message: format!("Complete {ticket_id}"),
                ticket_file,
                work_dir,
                completion_key: CompletionGenerationId::new(
                    CompletionId::new(ticket_id),
                    AttemptId::new(lease.attempt_id.to_string()),
                    1,
                ),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            !state.project_root.join(".lisa-commit.lock").exists(),
            "failed completion left .lisa-commit.lock behind: {error}"
        );
        format!("Error: {error}")
    }

    fn assert_no_finish_up(state: &State, ticket_id: &str) {
        assert!(!state.finish_up_sent.contains(ticket_id));
        assert!(!state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::FinishUpPromptSent { ticket_id: actual, .. }
                if actual == ticket_id
        )));
    }

    fn correlated_launch_failure<'a>(
        state: &'a State,
        ticket_id: &str,
        correlation: &CompletionGenerationId,
    ) -> &'a ActivityEvent {
        state
            .activity_log
            .iter()
            .find(|event| {
                matches!(
                    event,
                    ActivityEvent::CompletionRejected {
                        ticket_id: actual,
                        kind: CompletionRejectionKind::LaunchFailed,
                        correlation_id,
                        ..
                    } if actual == ticket_id && correlation_id == &correlation.to_string()
                )
            })
            .unwrap_or_else(|| {
                panic!(
                    "missing correlated launch failure for {ticket_id} ({correlation}): {:?}",
                    state.activity_log
                )
            })
    }

    fn assert_rejection_renders_unchanged(event: &ActivityEvent) {
        let ActivityEvent::CompletionRejected {
            ticket_id,
            kind,
            correlation_id,
            detail,
        } = event
        else {
            panic!("expected completion rejection, got {event:?}");
        };
        let entry = activity_event_to_ui_entry(event).unwrap();
        match entry.activity {
            ui::ActivityType::CompletionRejected {
                ticket_id: rendered_ticket,
                kind: rendered_kind,
                correlation_id: rendered_correlation,
                detail: rendered_detail,
            } => {
                assert_eq!(rendered_ticket, *ticket_id);
                assert_eq!(rendered_kind, *kind);
                assert_eq!(rendered_correlation, *correlation_id);
                assert_eq!(rendered_detail, *detail);
            }
            other => panic!("expected rendered completion rejection, got {other:?}"),
        }
    }

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
        assert!(
            activity_event_to_ui_entry(&ActivityEvent::TicketStatusChanged {
                ticket_id: "T-001".to_string(),
                old_status: TicketStatus::Open,
                new_status: TicketStatus::InProgress,
            })
            .is_none()
        );

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

        let entry = activity_event_to_ui_entry(&ActivityEvent::CompletionRejected {
            ticket_id: "T-003".to_string(),
            kind: CompletionRejectionKind::StaleLease,
            correlation_id: "corr-3".to_string(),
            detail: "attempt 2 is stale".to_string(),
        })
        .unwrap();
        match entry.activity {
            ui::ActivityType::CompletionRejected {
                ticket_id,
                kind,
                correlation_id,
                detail,
            } => {
                assert_eq!(ticket_id, "T-003");
                assert_eq!(kind, CompletionRejectionKind::StaleLease);
                assert_eq!(correlation_id, "corr-3");
                assert_eq!(detail, "attempt 2 is stale");
            }
            other => panic!("Expected CompletionRejected, got {other:?}"),
        }
    }

    #[test]
    fn test_build_claude_command() {
        let cmd = build_claude_command("T-042-01", 7, 1, None, None);

        assert!(cmd.starts_with(
            "LISA_PANE_ID=7 LISA_TICKET_ID='T-042-01' LISA_ATTEMPT_ID=1 claude --dangerously-skip-permissions"
        ));
        assert!(!cmd.contains("Read the ticket"));
        assert!(!cmd.contains("CLAUDE.md"));
        // No routed model → no --model flag.
        assert!(!cmd.contains("--model"));
        assert!(
            !cmd.ends_with('\r'),
            "Enter is now sent as a raw byte, not embedded in text"
        );
    }

    #[test]
    fn test_build_claude_command_with_model() {
        let cmd = build_claude_command("T-042-01", 7, 1, Some("opus"), None);
        assert!(
            cmd.ends_with("--dangerously-skip-permissions --model 'opus'"),
            "got: {cmd}"
        );
    }

    #[test]
    fn test_build_claude_command_includes_env_vars() {
        let cmd = build_claude_command("T-042-01", 42, 9, None, None);

        assert!(
            cmd.starts_with("LISA_PANE_ID=42 LISA_TICKET_ID='T-042-01' LISA_ATTEMPT_ID=9 "),
            "command should set pane, ticket, and attempt env vars, got: {}",
            cmd
        );
    }

    #[test]
    fn test_build_claude_command_excludes_assignment_reference() {
        let cmd = build_claude_command("T-001", 1, 1, None, None);
        assert!(!cmd.contains("docs/knowledge/rdspi-workflow.md"));
        assert!(!cmd.contains("assignment.md"));
    }

    #[test]
    fn test_shell_quote_round_trips_long_control_and_quote_heavy_values() {
        let values = [
            "",
            "plain",
            "space and\nnewline\ttab\rreturn",
            "single ' double \" dollar $(touch nope) ${HOME} `id` * ; \\",
            "escape:\u{1b} unicode: λ雪",
            &"long-'-$()-\n".repeat(4_096),
        ];

        for value in values {
            let quoted = shell_quote(value);
            assert!(quoted.starts_with('\'') && quoted.ends_with('\''));
            let decoded = quoted[1..quoted.len() - 1].replace("'\"'\"'", "'");
            assert_eq!(decoded, value, "value: {value:?}");
        }
    }

    #[test]
    fn shell_readiness_probe_publishes_exact_attempt_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let signal_dir = temp.path().join("signal path with ' quote");
        std::fs::create_dir_all(&signal_dir).unwrap();
        let lease = AttemptLease::mint("T-' ; touch SHOULD-NOT-EXIST".to_string(), None).unwrap();
        let probe = State::shell_readiness_probe(&signal_dir, 17, &lease).unwrap();

        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&probe)
            .current_dir(temp.path())
            .status()
            .unwrap();

        assert!(status.success());
        let body = std::fs::read_to_string(signal_dir.join("pane-17.shell-ready")).unwrap();
        assert_eq!(serde_json::from_str::<AttemptLease>(&body).unwrap(), lease);
        assert!(!temp.path().join("SHOULD-NOT-EXIST").exists());
        assert!(std::fs::read_dir(&signal_dir).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp.")));
    }

    #[test]
    fn publication_sites_preserve_serialization_and_collision_contracts() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let hostile_root = temp.path().join("publication path ' ; $(touch INJECTED)");

        let launch_dir = hostile_root.join("launch");
        fs::create_dir_all(&launch_dir).unwrap();
        let launch_destination = launch_dir.join(".lisa-launch-7.sh");
        fs::write(&launch_destination, "old launch").unwrap();
        let launch_payload = "printf '%s' \"quote:' dollar:$() backtick:`x`\"";
        let launch_command = State::prepare_fresh_launch(&launch_dir, 7, launch_payload).unwrap();
        assert_eq!(
            fs::read_to_string(&launch_destination).unwrap(),
            format!("#!/bin/sh\n{launch_payload}\n")
        );
        assert_eq!(
            launch_command,
            format!("sh {}", shell_quote(&launch_destination.to_string_lossy()))
        );
        assert!(!launch_command.contains(launch_payload));

        let assignment_dir = hostile_root.join("assignment");
        fs::create_dir_all(&assignment_dir).unwrap();
        let lease = AttemptLease::mint("T-PUB-'-$()", None).unwrap();
        let assignment_destination = assignment_dir.join("assignment-1-17.md");
        fs::write(&assignment_destination, "old assignment").unwrap();
        let assignment = "raw assignment\nquote:' dollar:$() backtick:`x`\n";
        assert_eq!(
            write_assignment(&assignment_dir, &lease, 17, assignment.as_bytes())
                .unwrap()
                .path,
            assignment_destination,
        );
        assert_eq!(
            fs::read_to_string(&assignment_destination).unwrap(),
            assignment
        );

        let signal_dir = hostile_root.join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        let lease_destination = signal_dir.join("pane-19.lease");
        fs::write(&lease_destination, "old lease").unwrap();
        let lease_state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        lease_state.write_pane_lease_marker(19, &lease).unwrap();
        let expected_lease = serde_json::to_string(&lease).unwrap();
        assert_eq!(
            fs::read_to_string(&lease_destination).unwrap(),
            expected_lease
        );
        assert_eq!(
            serde_json::from_str::<AttemptLease>(&fs::read_to_string(&lease_destination).unwrap())
                .unwrap(),
            lease
        );

        let mut artifact_state = State {
            config: PluginConfig {
                work_dir: hostile_root.join("canonical work"),
                ..PluginConfig::new()
            },
            attempt_dir: hostile_root.join("attempt staging"),
            ..State::default()
        };
        artifact_state
            .current_leases
            .insert(lease.ticket_id.clone(), lease.clone());
        let staged_dir = artifact_state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged_dir).unwrap();
        let staged = staged_dir.join("research.md");
        fs::write(&staged, "current attempt bytes\n' $() `x`").unwrap();
        let canonical_dir = artifact_state.config.work_dir.join(&lease.ticket_id);
        fs::create_dir_all(&canonical_dir).unwrap();
        let canonical = canonical_dir.join("research.md");
        fs::write(&canonical, "old canonical bytes").unwrap();
        let artifact_temporary = canonical_dir.join(".research.md.attempt-1.tmp");
        fs::write(&artifact_temporary, "old temporary collision").unwrap();

        assert!(artifact_state
            .admit_artifact(&lease.ticket_id, Some(&lease), "research.md")
            .unwrap());
        assert_eq!(
            fs::read_to_string(&canonical).unwrap(),
            "current attempt bytes\n' $() `x`"
        );
        assert_eq!(
            fs::read_to_string(&staged).unwrap(),
            "current attempt bytes\n' $() `x`",
            "admission copies rather than consumes the attributed source"
        );
        assert!(!artifact_temporary.exists());

        let shell_destination = signal_dir.join("pane-23.shell-ready");
        fs::write(&shell_destination, "old readiness").unwrap();
        let probe = State::shell_readiness_probe(&signal_dir, 23, &lease).unwrap();
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(probe)
            .current_dir(temp.path())
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(
            fs::read_to_string(&shell_destination).unwrap(),
            expected_lease
        );
        assert!(!temp.path().join("INJECTED").exists());

        for directory in [&launch_dir, &assignment_dir, &signal_dir, &canonical_dir] {
            assert!(fs::read_dir(directory).unwrap().all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp")));
        }
    }

    #[test]
    fn publication_sites_preserve_temp_names_cleanup_and_operator_errors() {
        use std::fs;

        fn assert_nonce_temp_in_error(error: &str, marker: &str) {
            let suffix = error
                .split_once(marker)
                .unwrap_or_else(|| panic!("missing temp marker {marker:?} in {error:?}"))
                .1
                .split(':')
                .next()
                .unwrap();
            assert!(
                !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()),
                "expected numeric nonce after {marker:?}, got {suffix:?} in {error:?}"
            );
        }

        fn assert_only_destination_directory(directory: &Path, destination: &Path) {
            let entries: Vec<_> = fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            assert_eq!(entries, vec![destination.to_path_buf()]);
            assert!(destination.is_dir());
        }

        fn deepest_addressable_directory(root: &Path) -> PathBuf {
            fs::create_dir_all(root).unwrap();
            let mut current = root.to_path_buf();
            for depth in 0..2_000 {
                let candidate = current.join(format!("d{depth:06}"));
                match fs::create_dir(&candidate) {
                    Ok(()) => current = candidate,
                    Err(_) => return current,
                }
            }
            panic!("filesystem accepted an unexpectedly deep test path");
        }

        let temp = tempfile::tempdir().unwrap();
        let hostile_root = temp.path().join("hostile path ' ; $() `x`");
        fs::create_dir_all(&hostile_root).unwrap();

        let long_dir = deepest_addressable_directory(&hostile_root.join("long-path"));
        let error = State::prepare_fresh_launch(&long_dir, 7, "payload").unwrap_err();
        assert!(error.starts_with("cannot write launch payload "));
        assert!(error.contains(&long_dir.to_string_lossy().into_owned()));
        assert_nonce_temp_in_error(&error, "/.lisa-launch-7.sh.tmp.");

        let lease = AttemptLease::mint("T-PUB", None).unwrap();
        let error = write_assignment(&long_dir, &lease, 17, b"assignment").unwrap_err();
        assert!(error.starts_with("cannot write assignment payload "));
        assert!(error.contains(&long_dir.to_string_lossy().into_owned()));
        assert_nonce_temp_in_error(&error, "/.assignment-1-17.md.tmp.");

        let state = State {
            signal_dir: long_dir.clone(),
            ..State::default()
        };
        let error = state.write_pane_lease_marker(19, &lease).unwrap_err();
        assert!(error.starts_with("cannot write pane lease marker "));
        assert!(error.contains(&long_dir.to_string_lossy().into_owned()));
        assert_nonce_temp_in_error(&error, "/pane-19.lease.tmp.1-");

        let launch_dir = hostile_root.join("failed-launch");
        fs::create_dir_all(&launch_dir).unwrap();
        let launch_destination = launch_dir.join(".lisa-launch-7.sh");
        fs::create_dir(&launch_destination).unwrap();
        let error = State::prepare_fresh_launch(&launch_dir, 7, "payload").unwrap_err();
        assert!(error.starts_with("cannot publish launch payload "));
        assert!(error.contains(&launch_destination.to_string_lossy().into_owned()));
        assert_only_destination_directory(&launch_dir, &launch_destination);

        let assignment_dir = hostile_root.join("failed-assignment");
        fs::create_dir_all(&assignment_dir).unwrap();
        let assignment_destination = assignment_dir.join("assignment-1-19.md");
        fs::create_dir(&assignment_destination).unwrap();
        let error = write_assignment(&assignment_dir, &lease, 19, b"assignment").unwrap_err();
        assert!(error.starts_with("cannot publish assignment payload "));
        assert!(error.contains(&assignment_destination.to_string_lossy().into_owned()));
        assert_only_destination_directory(&assignment_dir, &assignment_destination);

        let signal_dir = hostile_root.join("failed-signal");
        fs::create_dir_all(&signal_dir).unwrap();
        let lease_destination = signal_dir.join("pane-19.lease");
        fs::create_dir(&lease_destination).unwrap();
        let state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        let error = state.write_pane_lease_marker(19, &lease).unwrap_err();
        assert!(error.starts_with("cannot publish pane lease marker "));
        assert!(error.contains(&lease_destination.to_string_lossy().into_owned()));
        assert_only_destination_directory(&signal_dir, &lease_destination);

        let mut artifact_state = State {
            config: PluginConfig {
                work_dir: hostile_root.join("failed-canonical"),
                ..PluginConfig::new()
            },
            attempt_dir: hostile_root.join("failed-attempt"),
            ..State::default()
        };
        artifact_state
            .current_leases
            .insert(lease.ticket_id.clone(), lease.clone());
        let staged_dir = artifact_state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged_dir).unwrap();
        fs::write(staged_dir.join("research.md"), "new bytes").unwrap();
        let canonical_dir = artifact_state.config.work_dir.join(&lease.ticket_id);
        fs::create_dir_all(&canonical_dir).unwrap();
        let canonical = canonical_dir.join("research.md");
        fs::write(&canonical, "old canonical bytes").unwrap();
        let artifact_temporary = canonical_dir.join(".research.md.attempt-1.tmp");
        fs::create_dir(&artifact_temporary).unwrap();
        let error = artifact_state
            .admit_artifact(&lease.ticket_id, Some(&lease), "research.md")
            .unwrap_err();
        assert!(error.starts_with("cannot write canonical artifact temporary "));
        assert!(error.contains(&artifact_temporary.to_string_lossy().into_owned()));
        assert_eq!(
            fs::read_to_string(&canonical).unwrap(),
            "old canonical bytes"
        );
        assert!(artifact_temporary.is_dir());

        let shell_dir = hostile_root.join("failed-shell-readiness");
        fs::create_dir_all(&shell_dir).unwrap();
        let shell_destination = shell_dir.join("pane-23.shell-ready");
        fs::create_dir(&shell_destination).unwrap();
        let probe = State::shell_readiness_probe(&shell_dir, 23, &lease).unwrap();
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(probe)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "mv treats an existing destination directory as its target directory"
        );
        assert!(shell_destination.is_dir());
        assert_eq!(fs::read_dir(&shell_dir).unwrap().count(), 1);
        let moved: Vec<_> = fs::read_dir(&shell_destination)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(moved.len(), 1);
        let residual_name = moved[0].file_name().unwrap().to_string_lossy();
        let nonce = residual_name
            .strip_prefix("pane-23.shell-ready.tmp.1-")
            .unwrap();
        assert!(!nonce.is_empty() && nonce.bytes().all(|byte| byte.is_ascii_digit()));
        assert_eq!(
            fs::read_to_string(&moved[0]).unwrap(),
            serde_json::to_string(&lease).unwrap()
        );
    }

    #[test]
    fn test_prepare_fresh_launch_is_bounded_and_preserves_complete_payload() {
        let temp = tempfile::tempdir().unwrap();
        let artifact_dir = temp.path().join("attempt path with ' quote");
        let small = "printf '%s' small";
        let small_launcher = State::prepare_fresh_launch(&artifact_dir, 7, small).unwrap();
        assert!(!small_launcher.contains(small));

        let hostile = "quote:' double:\" dollar:$() backtick:`x` slash:\\\n\t\r\u{1b}";
        let large = format!("printf '%s' {}", shell_quote(&hostile.repeat(32_768)));
        let large_launcher = State::prepare_fresh_launch(&artifact_dir, 7, &large).unwrap();

        assert_eq!(small_launcher, large_launcher);
        assert!(!large_launcher.contains(&large));
        assert!(large.len() > 500_000);
        assert_eq!(
            std::fs::read_to_string(artifact_dir.join(".lisa-launch-7.sh")).unwrap(),
            format!("#!/bin/sh\n{large}\n")
        );
        assert!(std::fs::read_dir(&artifact_dir).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp.")));
    }

    #[test]
    fn test_prepare_fresh_launch_failure_cannot_queue_enter() {
        let temp = tempfile::tempdir().unwrap();
        let blocking_file = temp.path().join("not-a-directory");
        std::fs::write(&blocking_file, "occupied").unwrap();
        let mut state = State::default();

        let result = State::prepare_fresh_launch(&blocking_file.join("work"), 3, "payload");

        assert!(result.is_err());
        assert!(state.pending_enters.is_empty());
        assert_eq!(state.pending_timer_count, 0);
        // The only API that queues Enter is deliberately not called on Err.
        state.flush_pending_enters(std::time::SystemTime::now());
        assert!(state.pending_enters.is_empty());
    }

    #[test]
    fn test_prepare_assignment_atomically_preserves_complete_hostile_payload() {
        let temp = tempfile::tempdir().unwrap();
        let artifact_dir = temp.path().join("attempt path with ' quote");
        let lease = AttemptLease::mint("T-ASSIGNMENT", None).unwrap();
        let payload = format!(
            "Read everything exactly.\n{}",
            "quote:' double:\" dollar:$() backtick:`x` slash:\\\n\t\r\u{1b}".repeat(8_192)
        );

        let assignment = write_assignment(&artifact_dir, &lease, 23, payload.as_bytes()).unwrap();

        assert_eq!(assignment.path, artifact_dir.join("assignment-1-23.md"));
        assert_eq!(std::fs::read_to_string(assignment.path).unwrap(), payload);
        assert!(std::fs::read_dir(&artifact_dir).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp.")));
    }

    #[test]
    fn test_strip_host_prefix_with_prefix() {
        let path = Path::new("/host/docs/active/tickets");
        assert_eq!(
            strip_host_prefix(path),
            PathBuf::from("docs/active/tickets")
        );
    }

    #[test]
    fn test_strip_host_prefix_without_prefix() {
        let path = Path::new("docs/active/tickets");
        assert_eq!(
            strip_host_prefix(path),
            PathBuf::from("docs/active/tickets")
        );
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
        let prompt = ticket_prompt(
            dir,
            "T-024-03",
            AgentClient::Claude.context_file(),
            Path::new(".lisa/attempts/T-024-03/1/work"),
        );

        assert!(prompt.contains("docs/active/tickets/T-024-03.md"));
        assert!(prompt.contains("CLAUDE.md"));
        assert!(prompt.contains("docs/knowledge/rdspi-workflow.md"));
        assert!(prompt.contains(".lisa/attempts/T-024-03/1/work"));
        assert!(prompt.contains("Do not write phase artifacts directly"));
        assert!(prompt.contains("current phase"));
        assert!(prompt.contains("lisa commit-ticket"));
        assert!(prompt.contains("exact repository-relative --include paths"));
        assert!(prompt.contains("Do not use ordinary-index git add"));
        assert!(prompt.contains("git add -A"));
        assert!(prompt.contains("do not leave ticket-owned files staged, modified, or untracked"));
        assert!(prompt.contains("review-disposition.json"));
        assert!(prompt.contains(r#"{"disposition":"pass","reason":null}"#));
        assert!(
            prompt.contains(r#"{"disposition":"block","reason":"<non-empty actionable reason>"}"#)
        );
        assert!(prompt.contains("Both Review artifacts are required"));
        assert!(prompt.contains("Do not start another ticket until Lisa confirms"));
    }

    #[test]
    fn test_ticket_prompt_opens_with_canonical_purpose_before_mechanics() {
        let prompt = ticket_prompt(
            Path::new("docs/active/tickets"),
            "T-024-03",
            AgentClient::Claude.context_file(),
            Path::new(".lisa/attempts/T-024-03/1/work"),
        );
        assert!(prompt.starts_with(PURPOSE_PARAGRAPH));

        let lower = prompt.to_lowercase();
        let purpose_position = lower.find(&PURPOSE_PARAGRAPH.to_lowercase()).unwrap();
        for mechanism in ["dag", "phase", "scheduling", "zellij"] {
            if let Some(position) = lower.find(mechanism) {
                assert!(
                    purpose_position < position,
                    "purpose must precede {mechanism}"
                );
            }
        }
    }

    #[test]
    fn test_ticket_prompt_uses_given_context_file() {
        let dir = Path::new("docs/active/tickets");
        // Codex's context file replaces CLAUDE.md in the shared prompt body.
        let prompt = ticket_prompt(
            dir,
            "T-024-03",
            "AGENTS.md",
            Path::new(".lisa/attempts/T-024-03/1/work"),
        );
        assert!(prompt.contains("AGENTS.md"));
        assert!(!prompt.contains("CLAUDE.md"));
        assert!(prompt.contains("docs/knowledge/rdspi-workflow.md"));
    }

    #[test]
    fn test_ticket_prompt_uses_discovered_descriptive_ticket_path() {
        let dir = tempfile::tempdir().unwrap();
        let ticket_dir = dir.path().join("tickets");
        std::fs::create_dir_all(&ticket_dir).unwrap();
        std::fs::write(
            ticket_dir.join("T-024-03-descriptive-title.md"),
            "---\nid: T-024-03\ntitle: descriptive-title\ntype: task\nstatus: open\npriority: medium\nphase: research\n---\n",
        )
        .unwrap();

        let prompt = ticket_prompt(
            &ticket_dir,
            "T-024-03",
            "AGENTS.md",
            Path::new(".lisa/attempts/T-024-03/1/work"),
        );

        assert!(prompt.contains("T-024-03-descriptive-title.md"));
        assert!(!prompt.contains("tickets/T-024-03.md"));
        assert!(!prompt.contains("Recovery case:"));
    }

    #[test]
    fn review_startup_prompt_recovers_missing_disposition_without_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let ticket_dir = dir.path().join("tickets");
        std::fs::create_dir_all(&ticket_dir).unwrap();
        std::fs::write(
            ticket_dir.join("T-RECOVER.md"),
            "---\nid: T-RECOVER\ntitle: recover-review\ntype: bug\nstatus: open\npriority: critical\nphase: review\n---\n",
        )
        .unwrap();
        let artifact_dir = Path::new(".lisa/attempts/T-RECOVER/2/work");

        let prompt = ticket_prompt(&ticket_dir, "T-RECOVER", "AGENTS.md", artifact_dir);

        assert!(prompt.contains("Recovery case: this ticket already starts in Review"));
        assert!(prompt.contains("docs/active/work/T-RECOVER/review.md"));
        assert!(prompt.contains("immediately write a current-attempt review.md"));
        assert!(prompt.contains("review-disposition.json"));
        assert!(prompt.contains("Do not wait for a timeout"));
        assert!(prompt.contains(".lisa/attempts/T-RECOVER/2/work/"));
    }

    #[test]
    fn test_finish_up_prompt_preserves_atomic_completion_contract() {
        let prompt = finish_up_prompt(
            Path::new("docs/active/tickets"),
            Path::new(".lisa/attempts/T-024-03/1/work"),
            "T-024-03",
        );

        assert!(prompt.contains(".lisa/attempts/T-024-03/1/work/review.md"));
        assert!(prompt.contains(".lisa/attempts/T-024-03/1/work/review-disposition.json"));
        assert!(prompt.contains(r#"{"disposition":"pass","reason":null}"#));
        assert!(
            prompt.contains(r#"{"disposition":"block","reason":"<non-empty actionable reason>"}"#)
        );
        assert!(prompt.contains("Do NOT update the ticket's phase or status"));
        assert!(prompt.contains("ordinary-index git add/git commit"));
        assert!(prompt.contains("wait until Lisa confirms the completion commit"));
        assert!(prompt.contains("before starting another ticket"));
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
    fn test_check_artifact_advances_implement_ignores_progress_md() {
        // progress.md is a living tracking document, not a completion signal.
        // Only review.md advances implement → review.
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: impl-test\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
        ).unwrap();

        let work_dir = dir.path().join("work");

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
        let lease = install_current_attempt(&mut state, "T-002");
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();
        fs::write(staged.join("progress.md"), "# Progress").unwrap();

        state.check_artifact_advances();

        let thread = state.threads.get("T-002").unwrap();
        assert_eq!(thread.current_phase, Phase::Implement);
        assert_eq!(thread.status, ThreadStatus::Running);
        assert_eq!(
            fs::read_to_string(state.config.work_dir.join("T-002/progress.md")).unwrap(),
            "# Progress"
        );
    }

    #[test]
    fn test_check_artifact_advances_implement_to_review_via_review_md() {
        // review.md is the completion artifact for implement phase.
        // When it exists, implement should advance to review.
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: impl-test\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
        ).unwrap();

        let work_dir = dir.path().join("work");

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        let mut thread = Thread::new("T-002", 2);
        thread.current_phase = Phase::Implement;
        state.threads.insert("T-002".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-002");
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();
        fs::write(staged.join("review.md"), "# Review\nAll good.").unwrap();
        write_passing_review_disposition(&state, &lease);

        state.check_artifact_advances();

        // review.md advances Implement→Review, then starts commit-gated
        // completion without publishing Done.
        let thread = state.threads.get("T-002").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Running);
        assert!(state.pending_completions.contains_key("T-002"));

        // Ticket remains Review until the native transaction prepares Done.
        let updated = fs::read_to_string(tickets_dir.join("T-002.md")).unwrap();
        assert!(updated.contains("phase: review"));
    }

    #[test]
    fn poll_then_reload_reconciles_review_once_without_finish_up() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        let ticket_file = tickets_dir.join("T-LEVEL.md");
        fs::write(
            &ticket_file,
            "---\nid: T-LEVEL\ntitle: level-triggered\ntype: task\nstatus: open\npriority: critical\nphase: implement\n---\n",
        )
        .unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: dir.path().join("work"),
                review_timeout_secs: 10,
                wind_down_secs: 10,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        let mut thread = Thread::new("T-LEVEL", 7);
        thread.current_phase = Phase::Implement;
        state.threads.insert("T-LEVEL".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-LEVEL");
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();

        // The Review exists before the Implement→Review observation. Leave the
        // disposition absent for that edge so only the later level-triggered
        // poll can recover the obligation.
        fs::write(staged.join("review.md"), "# Review\nReady.\n").unwrap();
        state.check_artifact_advances();
        assert_eq!(state.threads["T-LEVEL"].current_phase, Phase::Review);
        assert!(!state.pending_completions.contains_key("T-LEVEL"));
        assert!(state.launched_completion_effects.is_empty());

        write_passing_review_disposition(&state, &lease);
        state.reconcile_review_completions();

        assert_eq!(state.launched_completion_effects.len(), 1);
        assert_eq!(
            state.launched_completion_effects[0],
            EffectCommand::LaunchCompletion {
                attempt_id: AttemptId::new(lease.attempt_id.to_string()),
                completion_id: CompletionId::new("T-LEVEL"),
            }
        );
        let pending = state.pending_completions.get("T-LEVEL").unwrap();
        assert_eq!(pending.source, CompletionSource::Reconcile);

        // Pending completion and the admitted current-attempt Review both make
        // the generic finish-up prompt false, even past both deadline bars.
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        let thread = state.threads.get_mut("T-LEVEL").unwrap();
        thread.last_phase_change = old;
        thread.last_activity = old;
        state.check_review_timeouts();
        assert!(!state.finish_up_sent.contains("T-LEVEL"));
        assert!(!state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::FinishUpPromptSent { ticket_id, .. } if ticket_id == "T-LEVEL"
        )));

        // A reload observation sees Requested and cannot emit a second effect.
        state.reconcile_review_completions();
        assert_eq!(state.launched_completion_effects.len(), 1);

        // Durable Done reconstructs Confirmed once no command is pending.
        state.pending_completions.remove("T-LEVEL");
        fs::write(
            &ticket_file,
            "---\nid: T-LEVEL\ntitle: level-triggered\ntype: task\nstatus: done\npriority: critical\nphase: done\n---\n",
        )
        .unwrap();
        state.dag =
            Dag::from_tickets(lisa_core::ticket::scan_tickets(&tickets_dir).unwrap()).unwrap();
        state.reconcile_review_completions();
        assert_eq!(state.launched_completion_effects.len(), 1);

        // A blocked E-040 disposition is likewise never eligible.
        fs::write(
            &ticket_file,
            "---\nid: T-LEVEL\ntitle: level-triggered\ntype: task\nstatus: review\npriority: critical\nphase: review\n---\n",
        )
        .unwrap();
        state.dag =
            Dag::from_tickets(lisa_core::ticket::scan_tickets(&tickets_dir).unwrap()).unwrap();
        write_review_disposition(
            &state,
            &lease,
            r#"{"disposition":"block","reason":"resolve the failing audit"}"#,
        );
        state.reconcile_review_completions();
        assert_eq!(state.launched_completion_effects.len(), 1);
        assert!(!state.pending_completions.contains_key("T-LEVEL"));
        state.check_review_timeouts();
        assert!(!state.finish_up_sent.contains("T-LEVEL"));
    }

    #[test]
    fn test_check_artifact_advances_full_catchup() {
        // When all artifacts exist, a single call should advance
        // from research all the way through to done.
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-005.md"),
            "---\nid: T-005\ntitle: full-run\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n\nBody\n",
        ).unwrap();

        let work_dir = dir.path().join("work");

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        let mut thread = Thread::new("T-005", 5);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-005".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-005");
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();
        fs::write(staged.join("research.md"), "# Research").unwrap();
        fs::write(staged.join("design.md"), "# Design").unwrap();
        fs::write(staged.join("structure.md"), "# Structure").unwrap();
        fs::write(staged.join("plan.md"), "# Plan").unwrap();
        fs::write(staged.join("review.md"), "# Review").unwrap();
        write_passing_review_disposition(&state, &lease);

        state.check_artifact_advances();

        // Should catch up to Review and then wait for the commit result.
        let thread = state.threads.get("T-005").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Running);
        assert!(state.pending_completions.contains_key("T-005"));

        let updated = fs::read_to_string(tickets_dir.join("T-005.md")).unwrap();
        assert!(updated.contains("phase: review"));
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
    fn test_check_artifact_advances_review_to_done() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // Create ticket file in review phase
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: high\nphase: review\n---\n\nBody\n",
        ).unwrap();

        let work_dir = dir.path().join("work");

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Running thread in Review phase
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        state.threads.insert("T-001".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-001");
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();
        fs::write(staged.join("review.md"), "# Review summary").unwrap();
        write_passing_review_disposition(&state, &lease);

        state.check_artifact_advances();

        // Thread and disk remain Review while the commit is pending.
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Running);
        assert!(state.pending_completions.contains_key("T-001"));
        assert_eq!(state.launched_completion_effects.len(), 1);
        assert_eq!(
            state.launched_completion_effects[0],
            EffectCommand::LaunchCompletion {
                attempt_id: AttemptId::new(lease.attempt_id.to_string()),
                completion_id: CompletionId::new("T-001"),
            }
        );

        // The stopped source traverses the same reducer seam. Because the
        // artifact request already made the aggregate pending, it cannot
        // execute a second launch effect.
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: Some(lease.clone()),
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        state.auto_complete_review("T-001".to_string(), 1);
        assert_eq!(state.launched_completion_effects.len(), 1);

        let updated = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(updated.contains("phase: review"));

        // Done transition is not logged before commit success.
        assert!(!state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::TicketPhaseChanged { ticket_id, old_phase, new_phase }
            if ticket_id == "T-001" && *old_phase == Phase::Review && *new_phase == Phase::Done
        )));
    }

    #[test]
    fn journal_seal_completes_repo_less_ticket_with_hashes_and_unblocks_dependent() {
        use lisa_core::provenance::ProvenanceLedgerRecord;
        use std::fs;

        const PREDECESSOR: &str = "T-JOURNAL";
        const DEPENDENT: &str = "T-AFTER-JOURNAL";
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let tickets_dir = root.join("tickets");
        let work_dir = root.join("work");
        let attempt_dir = root.join("attempts");
        let journal = root.join("completion-journal.jsonl");
        let ledger = root.join("provenance.jsonl");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join(format!("{PREDECESSOR}.md")),
            format!(
                "---\nid: {PREDECESSOR}\ntitle: journal predecessor\ntype: task\nstatus: open\npriority: high\nphase: review\n---\n"
            ),
        )
        .unwrap();
        fs::write(
            tickets_dir.join(format!("{DEPENDENT}.md")),
            format!(
                "---\nid: {DEPENDENT}\ntitle: journal dependent\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [{PREDECESSOR}]\n---\n"
            ),
        )
        .unwrap();

        let mut state = State {
            dag: Dag::from_tickets(ticket::scan_tickets(&tickets_dir).unwrap()).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: work_dir.clone(),
                completion_seal: CompletionSeal::Journal,
                lisa_bin: None,
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            project_root: root.to_path_buf(),
            git_root: PathBuf::new(),
            attempt_dir,
            completion_journal_path: journal.clone(),
            completion_journal_healthy: true,
            ledger_path: ledger.clone(),
            ..State::default()
        };
        let mut thread = Thread::new(PREDECESSOR, 42);
        thread.current_phase = Phase::Review;
        state.threads.insert(PREDECESSOR.to_string(), thread);
        let lease = install_current_attempt(&mut state, PREDECESSOR);
        fs::create_dir_all(state.attempt_work_dir(&lease)).unwrap();
        fs::write(
            state.attempt_work_dir(&lease).join("review.md"),
            "# Review\nReady with a criteria note.\n",
        )
        .unwrap();
        write_t046_note_disposition(&state, &lease);
        fs::create_dir_all(work_dir.join(PREDECESSOR).join("nested")).unwrap();
        fs::write(
            work_dir.join(PREDECESSOR).join("nested/evidence.txt"),
            "retained evidence\n",
        )
        .unwrap();

        assert!(!root.join(".git").exists());
        assert!(state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: PREDECESSOR.to_string(),
            source_lease: lease,
        }));

        assert!(!state.pending_completions.contains_key(PREDECESSOR));
        assert!(!state.threads.contains_key(PREDECESSOR));
        assert_eq!(
            state
                .dag
                .get_ticket(&PREDECESSOR.to_string())
                .unwrap()
                .phase,
            Phase::Done
        );
        assert_eq!(
            state
                .dag
                .get_ticket(&PREDECESSOR.to_string())
                .unwrap()
                .status,
            TicketStatus::Done
        );
        assert!(state.dag.all_dependencies_done(&DEPENDENT.to_string()));

        let receipt = state.completion_aggregates[PREDECESSOR]
            .confirmed_receipt()
            .unwrap();
        assert_eq!(
            state.completion_aggregates[PREDECESSOR].completion_note(),
            Some(&t046_completion_note())
        );
        assert_eq!(receipt.seal(), CompletionSeal::Journal);
        assert_eq!(receipt.content_hashes().len(), 4);
        for binding in receipt.content_hashes() {
            let bytes = fs::read(root.join(binding.path())).unwrap();
            assert_eq!(binding.sha256(), completion_journal::sha256(&bytes));
        }
        let journal_body = fs::read_to_string(&journal).unwrap();
        assert_eq!(journal_body.matches("\"seal\":\"journal\"").count(), 3);
        assert!(journal_body.contains("\"content_hashes\""));
        assert!(journal_body.contains("\"criterion_quote\":\"approximately 200 MiB\""));
        assert!(!journal_body.contains("\"commit_id\""));

        let records = read_mixed_ledger(&ledger);
        assert_eq!(records.len(), 1);
        let ProvenanceLedgerRecord::Execution(record) = &records[0] else {
            panic!("journal completion must retain execution provenance")
        };
        assert_eq!(record.ticket_id, PREDECESSOR);
        assert_eq!(record.seal, CompletionSeal::Journal);
        assert_eq!(record.outcome, RunOutcome::Done);
        assert_eq!(
            record.completion_note.as_ref(),
            Some(&t046_completion_note())
        );
        assert!(records
            .iter()
            .all(|record| !matches!(record, ProvenanceLedgerRecord::ParkingTransition(_))));
    }

    #[test]
    fn completion_command_uses_git_root_and_nested_repository_paths() {
        let state = State {
            config: PluginConfig {
                work_dir: PathBuf::from("/host/docs/active/work"),
                lisa_bin: Some("/usr/local/bin/lisa".to_string()),
                ..PluginConfig::new()
            },
            project_root: PathBuf::from("/repo/games/midsummer"),
            git_root: PathBuf::from("/repo"),
            ..State::default()
        };

        let completion_key =
            CompletionGenerationId::new(CompletionId::new("T-001"), AttemptId::new("7"), 1);
        let (argv, context) = state
            .build_completion_command(
                &completion_key,
                Path::new("/host/docs/active/tickets/T-001.md"),
            )
            .unwrap();

        assert_eq!(
            argv,
            vec![
                "/usr/local/bin/lisa",
                "complete-ticket",
                "--path",
                "/repo",
                "--ticket-id",
                "T-001",
                "--attempt-id",
                "7",
                "--completion-generation",
                "1",
                "--message",
                "Complete T-001",
                "--ticket-file",
                "games/midsummer/docs/active/tickets/T-001.md",
                "--work-dir",
                "games/midsummer/docs/active/work/T-001",
            ]
        );
        assert_eq!(context.get("lisa_completion"), Some(&"T-001".to_string()));
    }

    #[test]
    fn nested_monorepo_completion_command_drives_real_transaction() {
        use lisa_cli::commit_transaction::{complete_ticket, CompleteTicketRequest};
        use std::ffi::OsStr;
        use std::process::{Command, Output};

        struct NestedRepo {
            temp: tempfile::TempDir,
        }

        impl NestedRepo {
            fn new() -> Self {
                let repo = Self {
                    temp: tempfile::tempdir().unwrap(),
                };
                repo.git(["init", "--quiet"]);
                repo.git(["config", "user.name", "Lisa Test"]);
                repo.git(["config", "user.email", "lisa@example.test"]);
                repo
            }

            fn root(&self) -> &Path {
                self.temp.path()
            }

            fn write(&self, path: &str, contents: &str) {
                let path = self.root().join(path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, contents).unwrap();
            }

            fn git<I, S>(&self, args: I) -> Output
            where
                I: IntoIterator<Item = S>,
                S: AsRef<OsStr>,
            {
                let output = Command::new("git")
                    .arg("-C")
                    .arg(self.root())
                    .args(args)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "git failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                output
            }

            fn git_string<I, S>(&self, args: I) -> String
            where
                I: IntoIterator<Item = S>,
                S: AsRef<OsStr>,
            {
                String::from_utf8(self.git(args).stdout)
                    .unwrap()
                    .trim()
                    .to_string()
            }
        }

        fn option(argv: &[String], name: &str) -> Result<String, String> {
            argv.windows(2)
                .find(|pair| pair[0] == name)
                .map(|pair| pair[1].clone())
                .ok_or_else(|| format!("completion argv is missing {name}"))
        }

        fn assert_nested_contract(
            argv: &[String],
            git_root: &Path,
            ticket_id: &str,
        ) -> Result<(), String> {
            let expected_ticket = format!("games/midsummer/docs/active/tickets/{ticket_id}.md");
            let expected_work = format!("games/midsummer/docs/active/work/{ticket_id}");
            let path = option(argv, "--path")?;
            if Path::new(&path) != git_root {
                return Err(format!(
                    "--path must select Git root {}, got {path}",
                    git_root.display()
                ));
            }
            let ticket = option(argv, "--ticket-file")?;
            if ticket != expected_ticket {
                return Err(format!(
                    "--ticket-file must select {expected_ticket}, got {ticket}"
                ));
            }
            let work = option(argv, "--work-dir")?;
            if work != expected_work {
                return Err(format!(
                    "--work-dir must select {expected_work}, got {work}"
                ));
            }
            Ok(())
        }

        const TICKET_ID: &str = "T-009-02-01";
        let repo = NestedRepo::new();
        let nested_ticket = format!("games/midsummer/docs/active/tickets/{TICKET_ID}.md");
        let nested_work = format!("games/midsummer/docs/active/work/{TICKET_ID}");
        repo.write(
            &nested_ticket,
            &format!("---\nid: {TICKET_ID}\nstatus: open\nphase: review\n---\nArcade regression\n"),
        );
        repo.write("docs/root-sentinel.md", "root docs remain untouched\n");
        repo.git(["add", "-A"]);
        repo.git(["commit", "--quiet", "-m", "base"]);
        repo.write(&format!("{nested_work}/review.md"), "# Passing review\n");
        let old_head = repo.git_string(["rev-parse", "HEAD"]);

        let legacy_argv = vec![
            "lisa",
            "complete-ticket",
            "--path",
            "games/midsummer",
            "--ticket-id",
            TICKET_ID,
            "--message",
            "Complete T-009-02-01",
            "--ticket-file",
            "docs/active/tickets/T-009-02-01.md",
            "--work-dir",
            "docs/active/work/T-009-02-01",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let legacy_error = assert_nested_contract(&legacy_argv, repo.root(), TICKET_ID)
            .expect_err("the field-recorded pre-fix argv must fail the fixture contract");
        assert!(legacy_error.contains("--path"), "{legacy_error}");

        let state = State {
            config: PluginConfig {
                work_dir: PathBuf::from("/host/docs/active/work"),
                lisa_bin: Some("lisa".to_string()),
                ..PluginConfig::new()
            },
            project_root: repo.root().join("games/midsummer"),
            git_root: repo.root().to_path_buf(),
            ..State::default()
        };
        let completion_key =
            CompletionGenerationId::new(CompletionId::new(TICKET_ID), AttemptId::new("1"), 1);
        let (argv, context) = state
            .build_completion_command(
                &completion_key,
                Path::new("/host/docs/active/tickets/T-009-02-01.md"),
            )
            .unwrap();
        assert_nested_contract(&argv, repo.root(), TICKET_ID).unwrap();
        assert_eq!(context.get("lisa_completion"), Some(&TICKET_ID.to_string()));

        let result = complete_ticket(CompleteTicketRequest {
            repo_root: PathBuf::from(option(&argv, "--path").unwrap()),
            ticket_id: option(&argv, "--ticket-id").unwrap(),
            message: option(&argv, "--message").unwrap(),
            ticket_file: PathBuf::from(option(&argv, "--ticket-file").unwrap()),
            work_dir: PathBuf::from(option(&argv, "--work-dir").unwrap()),
            completion_key: CompletionGenerationId::new(
                CompletionId::new(option(&argv, "--ticket-id").unwrap()),
                AttemptId::new(option(&argv, "--attempt-id").unwrap()),
                option(&argv, "--completion-generation")
                    .unwrap()
                    .parse()
                    .unwrap(),
            ),
        })
        .unwrap();

        assert_ne!(result.commit_id, old_head);
        assert_eq!(repo.git_string(["rev-parse", "HEAD"]), result.commit_id);
        assert_eq!(repo.git_string(["rev-parse", "HEAD^"]), old_head);
        assert_eq!(
            result.committed_paths,
            vec![
                PathBuf::from(&nested_ticket),
                PathBuf::from(format!("{nested_work}/review.md")),
            ]
        );
        let committed_ticket = repo.git_string(["show", &format!("HEAD:{nested_ticket}")]);
        assert!(committed_ticket.contains("status: done"));
        assert!(committed_ticket.contains("phase: done"));
        assert_eq!(
            repo.git_string(["show", &format!("HEAD:{nested_work}/review.md")]),
            "# Passing review"
        );
        assert_eq!(
            repo.git_string(["show", "HEAD:docs/root-sentinel.md"]),
            "root docs remain untouched"
        );
        let tree = repo.git_string(["ls-tree", "-r", "--name-only", "HEAD"]);
        assert!(!tree
            .lines()
            .any(|path| path == format!("docs/active/tickets/{TICKET_ID}.md")));
        assert!(!tree
            .lines()
            .any(|path| path.starts_with(&format!("docs/active/work/{TICKET_ID}/"))));
    }

    #[test]
    fn completion_command_rejects_path_outside_git_root() {
        let state = State {
            config: PluginConfig {
                work_dir: PathBuf::from("/host/docs/active/work"),
                lisa_bin: Some("/usr/local/bin/lisa".to_string()),
                ..PluginConfig::new()
            },
            project_root: PathBuf::from("/repo/games/midsummer"),
            git_root: PathBuf::from("/repo"),
            ..State::default()
        };

        let completion_key =
            CompletionGenerationId::new(CompletionId::new("T-001"), AttemptId::new("1"), 1);
        let error = state
            .build_completion_command(&completion_key, Path::new("/outside/T-001.md"))
            .unwrap_err();
        assert!(error.contains("completion path outside Git root"));
        assert!(error.contains("/outside/T-001.md"));
    }

    #[test]
    fn review_disposition_gates_artifact_completion_and_dependents() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let cases = [
            (
                "block",
                r#"{"disposition":"block","reason":"resolve the failing release test"}"#,
                false,
                "resolve the failing release test",
            ),
            ("pass", r#"{"disposition":"pass","reason":null}"#, true, ""),
            (
                "note",
                r#"{"disposition":"note","reason":null,"criterion_quote":"approximately 200 MiB","evidence_citation":"docs/active/work/T-046-06-03/cbt-0716-210943-closing-codex/run-record.md","summary":"The 225 MiB measurement supports completion while the written gate is stale."}"#,
                true,
                "",
            ),
            (
                "invalid",
                r#"{"disposition":"pass","reason":"contradictory"}"#,
                false,
                "invalid review disposition",
            ),
        ];

        for (case, disposition, should_request, visible_reason) in cases {
            let dir = tempfile::tempdir().unwrap();
            let tickets_dir = dir.path().join("tickets");
            let work_dir = dir.path().join("work");
            fs::create_dir_all(&tickets_dir).unwrap();
            fs::write(
                tickets_dir.join("T-REVIEW.md"),
                "---\nid: T-REVIEW\ntitle: review gate\ntype: task\nstatus: review\npriority: high\nphase: review\n---\n",
            )
            .unwrap();
            fs::write(
                tickets_dir.join("T-DEPENDENT.md"),
                "---\nid: T-DEPENDENT\ntitle: downstream\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-REVIEW]\n---\n",
            )
            .unwrap();

            let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
            let mut state = State {
                dag: Dag::from_tickets(tickets).unwrap(),
                config: PluginConfig {
                    ticket_dir: tickets_dir.clone(),
                    work_dir: work_dir.clone(),
                    ..PluginConfig::new()
                },
                ..State::default()
            };
            state.agent_slots.push(AgentSlot {
                pane_id: 7,
                ticket_id: Some("T-REVIEW".to_string()),
                attempt_lease: None,
                has_session: true,
                transition_state: TransitionState::Idle,
                transition_started_at: None,
                cooldown_until: None,
                last_activity_at: None,
                last_client: None,
            });
            let mut thread = Thread::new("T-REVIEW", 7);
            thread.current_phase = Phase::Review;
            state.threads.insert("T-REVIEW".to_string(), thread);
            let lease = install_current_attempt(&mut state, "T-REVIEW");
            let staged = state.attempt_work_dir(&lease);
            fs::create_dir_all(&staged).unwrap();
            fs::write(staged.join("review.md"), "# Review\n").unwrap();
            write_review_disposition(&state, &lease, disposition);

            state.check_artifact_advances();

            assert_eq!(
                state.pending_completions.contains_key("T-REVIEW"),
                should_request,
                "{case}: only an authorizing disposition may request completion"
            );
            let thread = state.threads.get("T-REVIEW").unwrap();
            assert_eq!(thread.current_phase, Phase::Review, "{case}");
            assert_eq!(thread.status, ThreadStatus::Running, "{case}");
            assert_eq!(
                state.agent_slots[0].ticket_id.as_deref(),
                Some("T-REVIEW"),
                "{case}: assignment must remain until commit success"
            );
            assert_eq!(
                state.current_leases.get("T-REVIEW"),
                Some(&lease),
                "{case}: current attempt must remain authoritative"
            );
            assert!(
                fs::read_to_string(tickets_dir.join("T-REVIEW.md"))
                    .unwrap()
                    .contains("phase: review"),
                "{case}: Done must not publish before a successful transaction"
            );
            assert!(
                !state.dag.all_dependencies_done(&"T-DEPENDENT".to_string()),
                "{case}: the dependent must remain blocked before committed Done"
            );
            assert_eq!(
                fs::read_to_string(work_dir.join("T-REVIEW").join("review-disposition.json"))
                    .unwrap(),
                disposition,
                "{case}: admitted evidence must match the current attempt"
            );
            if !visible_reason.is_empty() {
                assert!(
                    state.activity_log.iter().any(|event| match event {
                        ActivityEvent::CompletionRejected {
                            kind: CompletionRejectionKind::DispositionBlocked,
                            detail,
                            ..
                        } => detail.contains(visible_reason),
                        _ => false,
                    }),
                    "{case}: refusal reason must be operator-visible"
                );
            }
        }
    }

    /// Regression for the T-039-06-02 field boundary: an already-written
    /// review with an explicit blocking disposition is not completion intent.
    #[test]
    fn test_t039_06_02_blocking_review_never_prepares_done() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let work_dir = dir.path().join("work");
        let ledger_path = dir.path().join("provenance.jsonl");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-REVIEW.md"),
            "---\nid: T-REVIEW\ntitle: hostile review\ntype: task\nstatus: review\npriority: high\nphase: review\n---\n",
        )
        .unwrap();
        fs::write(
            tickets_dir.join("T-DEPENDENT.md"),
            "---\nid: T-DEPENDENT\ntitle: blocked downstream\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-REVIEW]\n---\n",
        )
        .unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                ..PluginConfig::new()
            },
            ledger_path: ledger_path.clone(),
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 39,
            ticket_id: Some("T-REVIEW".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        let mut thread = Thread::new("T-REVIEW", 39);
        thread.current_phase = Phase::Review;
        state.threads.insert("T-REVIEW".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-REVIEW");
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();
        fs::write(staged.join("review.md"), "# Review\nBlocking finding.\n").unwrap();
        write_review_disposition(
            &state,
            &lease,
            r#"{"disposition":"block","reason":"resolve the hostile review finding"}"#,
        );

        state.check_artifact_advances();

        assert!(
            !state.pending_completions.contains_key("T-REVIEW"),
            "a block must not prepare Done; the pre-T-040-01-03 unconditional path did"
        );
        let thread = state.threads.get("T-REVIEW").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Running);
        assert_eq!(
            state.agent_slots[0].ticket_id.as_deref(),
            Some("T-REVIEW"),
            "the blocking ticket must stay assigned"
        );
        assert_eq!(state.agent_slots[0].attempt_lease.as_ref(), Some(&lease));
        assert_eq!(state.current_leases.get("T-REVIEW"), Some(&lease));

        let ticket = fs::read_to_string(tickets_dir.join("T-REVIEW.md")).unwrap();
        assert!(ticket.contains("status: review"), "ticket: {ticket}");
        assert!(ticket.contains("phase: review"), "ticket: {ticket}");
        assert!(
            !ledger_path.exists(),
            "a blocking Review must produce no authoritative Done provenance"
        );
        assert!(
            !state.dag.all_dependencies_done(&"T-DEPENDENT".to_string()),
            "the dependent must remain blocked"
        );
        assert!(
            !state.threads.contains_key("T-DEPENDENT"),
            "the blocked dependent must not be scheduled"
        );
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::CompletionRejected {
                kind: CompletionRejectionKind::DispositionBlocked,
                detail,
                ..
            } if detail.contains("resolve the hostile review finding")
        )));
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
                stuck_threshold_secs: 600, // hard-silence bar = 2x = 1200s
                ..PluginConfig::new()
            },
            ..State::default()
        };

        let ticket_id = "T-001".to_string();
        let lease = AttemptLease::mint(ticket_id.clone(), None).unwrap();
        state
            .lease_high_water
            .insert(ticket_id.clone(), lease.clone());
        state
            .current_leases
            .insert(ticket_id.clone(), lease.clone());

        // Create a thread that's been silent for 31+ minutes (past the bar)
        let mut thread = Thread::new("T-001", 1);
        thread.attempt_lease = Some(lease.clone());
        thread.current_phase = Phase::Research;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(31 * 60);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-001".to_string(), thread);

        // Add an agent slot so we can verify it gets released
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: Some(lease.clone()),
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        let outcomes = state.detect_stale_threads();

        assert_eq!(
            outcomes,
            vec![FailureTransitionOutcome::StaleThreadReclaimed {
                pane_id: 1,
                ticket_id: ticket_id.clone(),
                fenced: true,
            }]
        );

        // Thread should be removed (failed + cleaned up for retry)
        assert!(state.threads.is_empty());

        // Slot should be released
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Fenced
        );
        assert_eq!(state.current_leases.get(&ticket_id), None);
        assert_eq!(state.lease_high_water.get(&ticket_id), Some(&lease));

        // Error logged
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Error { message } if message.contains("stale")
        )));
        assert!(
            state.detect_stale_threads().is_empty(),
            "a reclaimed stale thread cannot be reclaimed again"
        );
    }

    #[test]
    fn test_stale_thread_not_stale_yet() {
        use lisa_core::types::{Thread, ThreadStatus};

        let mut state = State::default();

        // Create a thread that started recently (5 minutes ago)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(5 * 60);
        thread.last_activity = thread.last_phase_change;
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
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        state.release_slot_for_ticket(&"T-001".to_string());
        state.threads.remove("T-001");

        // Verify: slot is now idle but retains its Claude Code session
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert!(
            state.agent_slots[0].has_session,
            "has_session should stay true — Claude Code is still running"
        );
        // Slot has a 60s cooldown — not immediately available for scheduling
        assert!(
            state.agent_slots[0].cooldown_until.is_some(),
            "Released slot should have a cooldown set"
        );
        assert!(
            state.find_idle_slot(AgentClient::Claude).is_none(),
            "Slot should not be idle during cooldown"
        );

        // Verify: thread is removed from map
        assert!(!state.threads.contains_key("T-001"));

        // Verify: DAG shows T-002 as ready (T-001 is done)
        let ready = state.dag.get_ready_tickets();
        assert!(ready.contains(&"T-002".to_string()));

        // Verify: T-002 doesn't have a thread yet, so it would be scheduled
        assert!(!state.threads.contains_key("T-002"));
    }

    #[test]
    fn test_slot_cooldown_expires() {
        // After the cooldown period, a released slot becomes available again.
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: None,
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            // Cooldown already expired (set to 1 second ago)
            cooldown_until: Some(std::time::SystemTime::now() - std::time::Duration::from_secs(1)),
            last_activity_at: None,
            last_client: None,
        });
        assert!(
            state.find_idle_slot(AgentClient::Claude).is_some(),
            "Slot should be available after cooldown expires"
        );
    }

    #[test]
    fn test_slot_cooldown_blocks_scheduling() {
        // During the cooldown period, a released slot is not available.
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: None,
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            // Cooldown expires 30 seconds from now
            cooldown_until: Some(std::time::SystemTime::now() + std::time::Duration::from_secs(30)),
            last_activity_at: None,
            last_client: None,
        });
        assert!(
            state.find_idle_slot(AgentClient::Claude).is_none(),
            "Slot should not be available during cooldown"
        );
    }

    #[test]
    fn test_evaluate_health_stuck_transition() {
        use lisa_core::types::{HealthStatus, Thread};

        let mut state = State::default();
        state.config.stuck_threshold_secs = 600;

        // Create a thread that's been silent past the 600s threshold
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(700);
        thread.last_activity = thread.last_phase_change;
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
        assert_eq!(state.last_health.get("T-001"), Some(&HealthStatus::Stuck));
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
        assert_eq!(state.last_health.get("T-001"), Some(&HealthStatus::Healthy));
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
        state.config.stuck_threshold_secs = 600;

        // Create a thread silent past the stuck threshold
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(700);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-001".to_string(), thread);
        state.initialized = true;

        let ui_state = state.to_ui_state();

        // Should have one alert for the stuck thread
        assert_eq!(ui_state.alerts.len(), 1);
        assert_eq!(ui_state.alerts[0].ticket_id, "T-001");
        assert_eq!(ui_state.alerts[0].alert_type, ui::AlertType::Stuck);
    }

    #[test]
    fn review_protocol_blocker_replaces_generic_stuck_detail() {
        use lisa_core::types::Thread;

        let dir = tempfile::tempdir().unwrap();
        let mut state = State {
            attempt_dir: dir.path().join("attempts"),
            ..State::default()
        };
        state.config.stuck_threshold_secs = 600;

        let mut thread = Thread::new("T-REVIEW", 1);
        thread.current_phase = Phase::Review;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(700);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-REVIEW".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-REVIEW");
        std::fs::create_dir_all(state.attempt_work_dir(&lease)).unwrap();
        std::fs::write(
            state.attempt_work_dir(&lease).join("review.md"),
            "# Review\n",
        )
        .unwrap();

        let ui_state = state.to_ui_state();

        assert_eq!(ui_state.alerts.len(), 1);
        assert_eq!(ui_state.alerts[0].detail, "Missing review-disposition.json");
        assert_eq!(
            ui_state.alerts[0].suggested_actions,
            vec!["Write pass/block disposition", "Check pane"]
        );
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
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(300);
        thread.last_activity = thread.last_phase_change;
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
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(180);
        thread.last_activity = thread.last_phase_change;
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
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
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
            attempt_lease: None,
            has_session: false,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
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
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        // First rebuild with empty last_phases — done ticket should be detected
        let changed = state.rebuild_dag();
        assert!(
            changed,
            "First rebuild with done ticket should detect a change"
        );

        // Run the done-ticket detection logic (same as poll_tick)
        let done_tickets: Vec<TicketId> = state
            .threads
            .iter()
            .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
            .filter(|(tid, _)| {
                state
                    .dag
                    .get_ticket(tid)
                    .map(|t| t.phase == Phase::Done)
                    .unwrap_or(false)
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
        state
            .last_phases
            .insert("T-001".to_string(), Phase::Research);

        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        let changed = state.rebuild_dag();
        assert!(changed, "Phase change Research -> Done should be detected");

        let done_tickets: Vec<TicketId> = state
            .threads
            .iter()
            .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
            .filter(|(tid, _)| {
                state
                    .dag
                    .get_ticket(tid)
                    .map(|t| t.phase == Phase::Done)
                    .unwrap_or(false)
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
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        assert!(!state.threads.contains_key("T-001"));

        state.sweep_stale_slots();

        assert!(
            state.agent_slots[0].ticket_id.is_none(),
            "Stale slot should be released"
        );
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
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        // Run the done-ticket detection logic (mirrors poll_tick)
        let done_tickets: Vec<TicketId> = state
            .threads
            .iter()
            .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
            .filter(|(tid, _)| {
                state
                    .dag
                    .get_ticket(tid)
                    .map(|t| t.phase == Phase::Done)
                    .unwrap_or(false)
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
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
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
    fn test_mark_done_keeps_thread_and_slot_until_commit_result() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let work_dir = dir.path().join("work");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: to-mark\ntype: task\nstatus: review\npriority: high\nphase: review\n---\n\nBody\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
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
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        let lease = install_current_attempt(&mut state, "T-001");
        write_canonical_review_disposition(
            &state,
            "T-001",
            r#"{"disposition":"pass","reason":null}"#,
        );

        assert!(state.handle_key(KeyWithModifier {
            bare_key: BareKey::Char('d'),
            key_modifiers: Default::default(),
        }));
        assert!(state.modal.open);
        assert!(state.handle_key(KeyWithModifier {
            bare_key: BareKey::Enter,
            key_modifiers: Default::default(),
        }));

        let pending = state.pending_completions.get("T-001").unwrap();
        assert_eq!(pending.authority, CompletionAuthority::Operator);
        assert_eq!(
            pending.source,
            CompletionSource::OperatorRequested(OperatorRequestSource::MarkDoneKey)
        );
        assert_ne!(
            pending.completion_key.attempt_id().as_str(),
            lease.attempt_id.to_string()
        );
        assert_eq!(
            state.launched_completion_effects,
            vec![EffectCommand::LaunchCompletion {
                attempt_id: AttemptId::new("operator"),
                completion_id: CompletionId::new("T-001"),
            }]
        );
        let correlation_id = pending.completion_key.to_string();
        assert!(state.modal.open, "submission must not close the modal");
        assert_eq!(
            state.modal.operator_outcome,
            Some(OperatorModalOutcome::Pending {
                ticket_id: "T-001".to_string(),
                correlation_id: correlation_id.clone(),
            })
        );
        assert!(
            !state.handle_key(KeyWithModifier {
                bare_key: BareKey::Esc,
                key_modifiers: Default::default(),
            }),
            "an unresolved request cannot be silently dismissed"
        );

        state.poll_tick();

        assert!(state.modal.open, "the pending modal must survive a poll");
        assert_eq!(
            state.modal.operator_outcome,
            Some(OperatorModalOutcome::Pending {
                ticket_id: "T-001".to_string(),
                correlation_id,
            })
        );
        assert_eq!(
            state.launched_completion_effects.len(),
            1,
            "polling must not duplicate the operator request"
        );
        assert!(state.threads.contains_key("T-001"));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-001"));
        let content = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(content.contains("phase: review"));
    }

    #[test]
    fn test_mark_done_without_active_attempt_uses_operator_authority() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let work_dir = dir.path().join("work");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: orphaned\ntype: task\nstatus: open\npriority: high\nphase: review\n---\n\nBody\n",
        )
        .unwrap();
        let dag =
            Dag::from_tickets(lisa_core::ticket::scan_tickets(&tickets_dir).unwrap()).unwrap();
        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        write_canonical_review_disposition(
            &state,
            "T-001",
            r#"{"disposition":"pass","reason":null}"#,
        );

        state.mark_ticket_done("T-001");

        let pending = state.pending_completions.get("T-001").unwrap();
        assert_eq!(pending.authority, CompletionAuthority::Operator);
        assert_eq!(
            pending.source,
            CompletionSource::OperatorRequested(OperatorRequestSource::MarkDoneKey)
        );
        assert_eq!(
            state.launched_completion_effects,
            vec![EffectCommand::LaunchCompletion {
                attempt_id: AttemptId::new("operator"),
                completion_id: CompletionId::new("T-001"),
            }]
        );
        assert!(fs::read_to_string(tickets_dir.join("T-001.md"))
            .unwrap()
            .contains("phase: review"));
    }

    #[test]
    fn test_mark_done_already_pending_keeps_named_correlated_rejection_visible() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let work_dir = dir.path().join("work");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: duplicate\ntype: task\nstatus: review\npriority: high\nphase: review\n---\n",
        )
        .unwrap();
        let dag =
            Dag::from_tickets(lisa_core::ticket::scan_tickets(&tickets_dir).unwrap()).unwrap();
        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        write_canonical_review_disposition(
            &state,
            "T-001",
            r#"{"disposition":"pass","reason":null}"#,
        );

        state.mark_ticket_done("T-001");
        let correlation_id = state.pending_completions["T-001"]
            .completion_key
            .to_string();
        assert_eq!(state.launched_completion_effects.len(), 1);

        state.open_mark_done_modal();
        assert!(state.handle_key(KeyWithModifier {
            bare_key: BareKey::Enter,
            key_modifiers: Default::default(),
        }));

        assert!(state.modal.open);
        assert!(matches!(
            state.modal.operator_outcome.as_ref(),
            Some(OperatorModalOutcome::Rejected {
                ticket_id,
                kind: CompletionRejectionKind::AlreadyPending,
                correlation_id: rejected_correlation,
                detail,
            }) if ticket_id == "T-001"
                && rejected_correlation == &correlation_id
                && detail.contains("already pending")
        ));
        assert_eq!(
            state.launched_completion_effects.len(),
            1,
            "already-pending feedback must not launch a duplicate effect"
        );
        assert!(state.handle_key(KeyWithModifier {
            bare_key: BareKey::Enter,
            key_modifiers: Default::default(),
        }));
        assert!(
            !state.modal.open,
            "terminal feedback closes only on acknowledgement"
        );
    }

    #[test]
    fn test_operator_requested_refuses_blocked_disposition_and_unmet_dependencies() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let work_dir = dir.path().join("work");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-BLOCKED.md"),
            "---\nid: T-BLOCKED\ntitle: blocked review\ntype: task\nstatus: review\npriority: high\nphase: review\n---\n",
        )
        .unwrap();
        fs::write(
            tickets_dir.join("T-DEPENDENCY.md"),
            "---\nid: T-DEPENDENCY\ntitle: unfinished dependency\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n",
        )
        .unwrap();
        fs::write(
            tickets_dir.join("T-DEPENDENT.md"),
            "---\nid: T-DEPENDENT\ntitle: dependent review\ntype: task\nstatus: review\npriority: high\nphase: review\ndepends_on: [T-DEPENDENCY]\n---\n",
        )
        .unwrap();

        let dag =
            Dag::from_tickets(lisa_core::ticket::scan_tickets(&tickets_dir).unwrap()).unwrap();
        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        let mut thread = Thread::new("T-BLOCKED", 1);
        thread.current_phase = Phase::Review;
        state.threads.insert("T-BLOCKED".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-BLOCKED");
        write_canonical_review_disposition(
            &state,
            "T-BLOCKED",
            r#"{"disposition":"block","reason":"resolve the operator-blocking review"}"#,
        );
        write_canonical_review_disposition(
            &state,
            "T-DEPENDENT",
            r#"{"disposition":"pass","reason":null}"#,
        );

        state.mark_ticket_done("T-BLOCKED");

        assert!(state.pending_completions.is_empty());
        assert!(state.launched_completion_effects.is_empty());
        assert_eq!(
            state.threads["T-BLOCKED"].attempt_lease.as_ref(),
            Some(&lease),
            "the operator request must not consume or replace attempt authority"
        );
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::CompletionRejected {
                ticket_id,
                kind: CompletionRejectionKind::DispositionBlocked,
                correlation_id,
                detail,
            } if ticket_id == "T-BLOCKED"
                && correlation_id.contains("6f70657261746f72")
                && detail.contains("resolve the operator-blocking review")
        )));

        state.mark_ticket_done("T-DEPENDENT");

        assert!(state.pending_completions.is_empty());
        assert!(state.launched_completion_effects.is_empty());
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::CompletionRejected {
                ticket_id,
                kind: CompletionRejectionKind::DependencyBlocked,
                correlation_id,
                detail,
            } if ticket_id == "T-DEPENDENT"
                && correlation_id.contains("6f70657261746f72")
                && detail.contains("dependencies are not all done")
        )));
        assert!(fs::read_to_string(tickets_dir.join("T-BLOCKED.md"))
            .unwrap()
            .contains("phase: review"));
        assert!(fs::read_to_string(tickets_dir.join("T-DEPENDENT.md"))
            .unwrap()
            .contains("phase: review"));
    }

    #[test]
    fn test_review_ticket_appears_in_mark_done_modal() {
        // A Review-phase ticket should appear in the mark-done modal even if
        // it has a Running thread (e.g. from review-timeout finish-up prompt).
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: in-review\ntype: task\nstatus: open\npriority: high\nphase: review\n---\n\nBody\n",
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

        // Simulate a running thread (as if review_timeout resumed it)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        // thread starts as Running by default
        state.threads.insert("T-001".to_string(), thread);

        state.open_mark_done_modal();

        // Review ticket should appear despite having a Running thread
        assert!(state.modal.open, "Modal should open");
        assert!(
            state.modal.ticket_ids.contains(&"T-001".to_string()),
            "Review-phase ticket should be in mark-done list even with Running thread"
        );
    }

    #[test]
    fn test_running_non_review_ticket_excluded_from_mark_done() {
        // A non-Review ticket with a Running thread should NOT appear.
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: implementing\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
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

        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);

        state.open_mark_done_modal();

        // Implement-phase ticket with Running thread should be excluded
        assert!(
            !state.modal.open,
            "Modal should not open — no eligible tickets"
        );
    }

    #[test]
    fn test_implement_ticket_with_review_artifact_in_mark_done() {
        // An Implement-phase ticket with review.md should appear in the
        // mark-done modal — the agent finished all phases but transitions
        // didn't fire.
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: stuck-implement\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
        ).unwrap();

        // review.md exists — agent completed all work
        let work_dir = dir.path().join("work");
        fs::create_dir_all(work_dir.join("T-001")).unwrap();
        fs::write(work_dir.join("T-001/review.md"), "# Review\nDone.").unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);

        state.open_mark_done_modal();

        assert!(state.modal.open, "Modal should open");
        assert!(
            state.modal.ticket_ids.contains(&"T-001".to_string()),
            "Implement ticket with review.md should be in mark-done list"
        );
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
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        state.agent_slots.push(AgentSlot {
            pane_id: 6,
            ticket_id: None,
            attempt_lease: None,
            has_session: false,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        // Add health data
        state
            .last_health
            .insert("T-002".to_string(), lisa_core::types::HealthStatus::Healthy);

        // Add activity events
        state.log_activity(ActivityEvent::PluginStarted);
        state.log_activity(ActivityEvent::Info {
            message: "test info".to_string(),
        });

        let snapshot = state.format_snapshot();

        // Check all section headers
        assert!(
            snapshot.contains("=== Lisa State Snapshot ==="),
            "Missing header"
        );
        assert!(
            snapshot.contains("=== Config ==="),
            "Missing config section"
        );
        assert!(
            snapshot.contains("=== Plugin Status ==="),
            "Missing plugin status"
        );
        assert!(
            snapshot.contains("=== Tickets ==="),
            "Missing tickets section"
        );
        assert!(
            snapshot.contains("=== DAG Edges ==="),
            "Missing edges section"
        );
        assert!(
            snapshot.contains("=== DAG Stats ==="),
            "Missing stats section"
        );
        assert!(
            snapshot.contains("=== Threads ==="),
            "Missing threads section"
        );
        assert!(
            snapshot.contains("=== Agent Slots ==="),
            "Missing slots section"
        );
        assert!(
            snapshot.contains("=== Last Known Health ==="),
            "Missing health section"
        );
        assert!(
            snapshot.contains("=== Activity Log (last 50) ==="),
            "Missing activity log"
        );
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
        assert!(
            snapshot.contains("T-001 -> T-002"),
            "Missing edge T-001 -> T-002"
        );

        // DAG stats
        assert!(
            snapshot.contains("total_tickets:       2"),
            "Wrong total tickets"
        );
        assert!(
            snapshot.contains("done_tickets:        1"),
            "Wrong done tickets"
        );
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
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        state.agent_slots.push(AgentSlot {
            pane_id: 43,
            ticket_id: None,
            attempt_lease: None,
            has_session: false,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
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
        assert!(
            !snapshot.contains("event-49"),
            "Old event should not appear"
        );

        // Should be numbered 1-50
        assert!(
            snapshot.contains("  1. Info: event-99"),
            "First entry should be event-99"
        );
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
            (
                ActivityEvent::CompletionRejected {
                    ticket_id: "T-003".to_string(),
                    kind: CompletionRejectionKind::LaunchFailed,
                    correlation_id: "corr-launch".to_string(),
                    detail: "host unavailable".to_string(),
                },
                "CompletionRejected: T-003 launch-failed correlation=corr-launch detail=host unavailable",
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

        // Create signal directory with idle signal (pane-based)
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.idle"), "2025-01-01T00:00:00Z").unwrap();

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

        // Agent slot maps pane 1 → T-001
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        // Add running thread in implement phase
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        state.threads.insert("T-001".to_string(), thread);
        install_current_attempt(&mut state, "T-001");

        // Run idle signal check
        state.check_idle_signals();

        // Verify: thread advanced to Review, stays running
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Running);

        // Verify: signal file deleted
        assert!(!state.signal_dir.join("pane-1.idle").exists());

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
    fn test_idle_signal_implement_with_review_artifact_advances_to_done() {
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

        let work_dir = dir.path().join("work");

        // Create signal directory with idle signal
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.idle"), "2025-01-01T00:00:00Z").unwrap();

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

        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        state.threads.insert("T-001".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-001");
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();
        fs::write(staged.join("review.md"), "# Review\nAll good.").unwrap();
        write_passing_review_disposition(&state, &lease);

        // Run idle signal check
        state.check_idle_signals();

        // Verify: Review is published locally, while Done awaits the commit.
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Running);
        assert!(state.pending_completions.contains_key("T-001"));
        let pending = state.pending_completions.get("T-001").unwrap();
        assert_eq!(pending.source, CompletionSource::Idle);
        assert_eq!(
            state.launched_completion_effects,
            vec![EffectCommand::LaunchCompletion {
                attempt_id: AttemptId::new(lease.attempt_id.to_string()),
                completion_id: CompletionId::new("T-001"),
            }]
        );

        // Verify: ticket file has not published Done.
        let updated = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(updated.contains("phase: review"));

        // Verify: only Implement completion is published before commit success.
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::PhaseCompleted { phase, .. }
            if *phase == Phase::Implement
        )));
        assert!(!state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::PhaseCompleted { phase, .. }
            if *phase == Phase::Review
        )));
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

        // Create signal directory with idle signal (pane-based)
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.idle"), "2025-01-01T00:00:00Z").unwrap();

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

        // Agent slot maps pane 1 → T-001
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

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
        assert!(!state.signal_dir.join("pane-1.idle").exists());

        // Verify: ticket file updated
        let updated = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(updated.contains("phase: design"));

        // Verify: no idle alerts
        assert!(state.idle_alerts.is_empty());
    }

    #[test]
    fn test_codex_ack_signal_promotes_matching_pending_seat() {
        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        std::fs::create_dir_all(&signal_dir).unwrap();

        let mut slot = fresh_slot(7, Some(AgentClient::Codex));
        slot.ticket_id = Some("T-ACK".to_string());
        let lease = AttemptLease {
            ticket_id: "T-ACK".to_string(),
            attempt_id: 9,
        };
        slot.attempt_lease = Some(lease.clone());
        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state
            .lease_high_water
            .insert("T-ACK".to_string(), lease.clone());
        state
            .current_leases
            .insert("T-ACK".to_string(), lease.clone());
        state.agent_slots.push(slot);
        state.seat_assignments.insert(
            7,
            SeatAssignmentState::AssignedPendingAck {
                generation: 9,
                ack_deadline: None,
            },
        );

        let payload = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "assigned work",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-ACK",
                    generation: 9,
                },
            ),
        });
        std::fs::write(signal_dir.join("pane-7.ack"), payload.to_string()).unwrap();

        state.check_codex_ack_signals();

        assert!(!signal_dir.join("pane-7.ack").exists());
        assert_eq!(state.seat_assignment(7), Some(SeatAssignmentState::Owned));
        assert!(state.seat_is_owned(7));
        assert_eq!(
            state
                .activity_log
                .iter()
                .filter(|event| matches!(event, ActivityEvent::Info { message } if message.contains("acknowledged its assignment")))
                .count(),
            1
        );

        std::fs::write(signal_dir.join("pane-7.ack"), payload.to_string()).unwrap();
        state.check_codex_ack_signals();
        assert!(!signal_dir.join("pane-7.ack").exists());
        assert_eq!(
            state
                .activity_log
                .iter()
                .filter(|event| matches!(event, ActivityEvent::Info { message } if message.contains("acknowledged its assignment")))
                .count(),
            1,
            "duplicate ack is consumed without a second promotion"
        );
    }

    #[test]
    fn test_build_notify_command_complete() {
        let root = Path::new("/proj");
        let extra = vec![
            ("LISA_TICKETS_DONE", "3".to_string()),
            ("LISA_DURATION_SECS", "120".to_string()),
        ];
        let (argv, env) = State::build_notify_command(root, "complete", "3 tickets done", &extra);

        // argv: sh -c <guard> sh <event> <detail>
        assert_eq!(argv[0], "sh");
        assert_eq!(argv[1], "-c");
        assert!(argv[2].contains("if [ -x \"$LISA_HOOK\" ]"));
        assert_eq!(argv[3], "sh");
        assert_eq!(argv[4], "complete");
        assert_eq!(argv[5], "3 tickets done");

        assert_eq!(env.get("LISA_EVENT").unwrap(), "complete");
        assert_eq!(env.get("LISA_PROJECT").unwrap(), "/proj");
        assert_eq!(env.get("LISA_HOOK").unwrap(), "/proj/.lisa/hooks/on-notify");
        assert_eq!(env.get("LISA_TICKETS_DONE").unwrap(), "3");
        assert_eq!(env.get("LISA_DURATION_SECS").unwrap(), "120");
    }

    #[test]
    fn test_build_notify_command_attention() {
        let root = Path::new("/proj");
        let extra = vec![
            ("LISA_PANE_ID", "7".to_string()),
            ("LISA_TICKET_ID", "T-042".to_string()),
            ("LISA_REASON", "idle-without-artifact".to_string()),
        ];
        let detail = "T-042 idle in research without research.md";
        let (argv, env) = State::build_notify_command(root, "attention", detail, &extra);

        assert_eq!(argv[4], "attention");
        assert_eq!(argv[5], detail);

        assert_eq!(env.get("LISA_EVENT").unwrap(), "attention");
        assert_eq!(env.get("LISA_PANE_ID").unwrap(), "7");
        assert_eq!(env.get("LISA_TICKET_ID").unwrap(), "T-042");
        assert_eq!(env.get("LISA_REASON").unwrap(), "idle-without-artifact");
        assert_eq!(env.get("LISA_HOOK").unwrap(), "/proj/.lisa/hooks/on-notify");
    }

    #[test]
    fn test_attention_debounce_add_skip_and_clear() {
        let mut state = State::default();

        // First stall for pane 5 → newly inserted (would fire).
        assert!(state.notified_attention.insert(5));
        // Repeated stall while still stalled → already present (suppressed).
        assert!(!state.notified_attention.insert(5));

        // A heartbeat clears the entry → a later re-stall can notify again.
        state.notified_attention.remove(&5);
        assert!(state.notified_attention.insert(5));
    }

    // --- T-020-03: awaiting-human suppression -------------------------------

    #[test]
    fn test_check_awaiting_signals_inserts_and_deletes() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-7.awaiting"), "2026-06-20T00:00:00Z").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };

        state.check_awaiting_signals();

        // Pane flagged and the signal file consumed (so it doesn't re-trigger).
        assert!(state.is_pane_awaiting(7));
        assert!(!signal_dir.join("pane-7.awaiting").exists());
    }

    #[test]
    fn test_heartbeat_clears_awaiting() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 7,
            ticket_id: Some("T-007".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        state
            .threads
            .insert("T-007".to_string(), Thread::new("T-007", 7));
        let lease = install_current_attempt(&mut state, "T-007");
        fs::write(
            signal_dir.join("pane-7.heartbeat"),
            serde_json::to_string(&lease).unwrap(),
        )
        .unwrap();
        state.awaiting_human.insert(7);

        state.check_heartbeat_signals();

        // A real tool call (heartbeat) means the question was answered.
        assert!(!state.is_pane_awaiting(7));
        assert!(!signal_dir.join("pane-7.heartbeat").exists());
    }

    #[test]
    fn test_is_pane_awaiting_accessor() {
        let mut state = State::default();
        assert!(!state.is_pane_awaiting(3));
        state.awaiting_human.insert(3);
        assert!(state.is_pane_awaiting(3));
        state.awaiting_human.remove(&3);
        assert!(!state.is_pane_awaiting(3));
    }

    #[test]
    fn test_stopped_signal_skips_when_awaiting() {
        // A WaitingForStop pane blocked on a question must not be /clear-ed.
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::WaitingForStop,
            transition_started_at: Some(std::time::SystemTime::now()),
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        state.awaiting_human.insert(1);

        // Would call send_line_to_pane("/clear", ..) (a zellij host call that
        // panics natively) if the guard were missing — so reaching the assert
        // proves the guard short-circuited.
        state.handle_stopped_signal(1);

        // No state-machine advance: still WaitingForStop, not WaitingForClear.
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForStop
        );
    }

    #[test]
    fn test_cleared_signal_skips_when_awaiting() {
        // A WaitingForClear pane blocked on a question must not receive the prompt.
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::WaitingForClear,
            transition_started_at: Some(std::time::SystemTime::now()),
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        state.awaiting_human.insert(1);

        state.handle_cleared_signal(1);

        // Still WaitingForClear — the prompt was not sent, slot did not flip to Idle.
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForClear
        );
    }

    #[test]
    fn test_transition_timeouts_skip_when_awaiting() {
        // A timed-out WaitingForStop pane that is quiet would normally be force
        // /clear-ed; while awaiting it must be skipped, leaving state unchanged.
        let mut state = State::default();
        let long_ago = std::time::SystemTime::now()
            - std::time::Duration::from_secs(state.config.wind_down_secs + 100_000);
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::WaitingForStop,
            transition_started_at: Some(long_ago),
            cooldown_until: None,
            last_activity_at: Some(long_ago),
            last_client: None,
        });
        state.awaiting_human.insert(1);

        state.check_transition_timeouts();

        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForStop
        );
    }

    #[test]
    fn test_review_timeout_skips_when_awaiting() {
        use lisa_core::types::Thread;

        // A Review thread past timeout + quiet would get a finish-up prompt; while
        // awaiting it must be skipped without being marked finish_up_sent.
        let mut state = State::default();
        let now = std::time::SystemTime::now();
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        thread.last_phase_change =
            now - std::time::Duration::from_secs(state.config.review_timeout_secs + 100);
        thread.last_activity =
            now - std::time::Duration::from_secs(state.config.wind_down_secs + 100);
        state.threads.insert("T-001".to_string(), thread);
        state.awaiting_human.insert(1);

        state.check_review_timeouts();

        // Skipped: no finish-up prompt counted, so it re-evaluates once unblocked.
        assert!(!state.finish_up_sent.contains("T-001"));
    }

    #[test]
    fn test_session_timeout_skips_kill_when_awaiting() {
        use lisa_core::types::Thread;

        // Over budget AND silent past hard-silence — would normally be reclaimed.
        let mut state = State::default();
        let now = std::time::SystemTime::now();
        let hard_silence = state.config.stuck_threshold_secs * 2;
        let mut thread = Thread::new("T-001", 1);
        thread.started_at =
            now - std::time::Duration::from_secs(state.config.session_timeout_secs + 1000);
        thread.last_activity = now - std::time::Duration::from_secs(hard_silence + 100);
        thread.last_phase_change = thread.last_activity;
        state.threads.insert("T-001".to_string(), thread);
        state.awaiting_human.insert(1);

        let outcomes = state.check_session_timeouts();

        assert!(outcomes.is_empty());

        // Exempt: still present, not reclaimed.
        assert!(state.threads.contains_key("T-001"));
    }

    #[test]
    fn test_session_timeout_kills_after_flag_clears() {
        use lisa_core::types::Thread;

        // Identical fixture, but the pane is NOT awaiting — normal reclaim applies.
        let mut state = State::default();
        let now = std::time::SystemTime::now();
        let hard_silence = state.config.stuck_threshold_secs * 2;
        let mut thread = Thread::new("T-001", 1);
        thread.started_at =
            now - std::time::Duration::from_secs(state.config.session_timeout_secs + 1000);
        thread.last_activity = now - std::time::Duration::from_secs(hard_silence + 100);
        thread.last_phase_change = thread.last_activity;
        state.threads.insert("T-001".to_string(), thread);
        // awaiting_human empty → the only difference from the test above.

        state.check_session_timeouts();

        // Reclaimed: removed once the exemption no longer applies.
        assert!(!state.threads.contains_key("T-001"));
    }

    #[test]
    fn test_detect_stale_skips_when_awaiting() {
        use lisa_core::types::Thread;

        // Silent past the hard timeout — would normally be marked stale.
        let mut state = State::default();
        let now = std::time::SystemTime::now();
        let hard_timeout = state.config.stuck_threshold_secs * 2;
        let mut thread = Thread::new("T-001", 1);
        thread.last_activity = now - std::time::Duration::from_secs(hard_timeout + 100);
        state.threads.insert("T-001".to_string(), thread);
        state.awaiting_human.insert(1);

        state.detect_stale_threads();

        // Exempt: still present.
        assert!(state.threads.contains_key("T-001"));
    }

    #[test]
    fn test_detect_stale_kills_after_flag_clears() {
        use lisa_core::types::Thread;

        // Identical fixture, no awaiting flag — stale reclamation applies.
        let mut state = State::default();
        let now = std::time::SystemTime::now();
        let hard_timeout = state.config.stuck_threshold_secs * 2;
        let mut thread = Thread::new("T-001", 1);
        thread.last_activity = now - std::time::Duration::from_secs(hard_timeout + 100);
        state.threads.insert("T-001".to_string(), thread);

        state.detect_stale_threads();

        // Reclaimed.
        assert!(!state.threads.contains_key("T-001"));
    }

    #[test]
    fn test_to_ui_state_marks_awaiting_thread() {
        use lisa_core::types::Thread;

        // Two running threads on two panes; only pane 1 is awaiting.
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        state.agent_slots.push(AgentSlot {
            pane_id: 2,
            ticket_id: Some("T-002".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        state
            .threads
            .insert("T-001".to_string(), Thread::new("T-001", 1));
        state
            .threads
            .insert("T-002".to_string(), Thread::new("T-002", 2));
        state.awaiting_human.insert(1);

        let ui_state = state.to_ui_state();

        // The UI marker is a pure projection of the awaiting_human set, so the
        // exemption and the marker can never disagree.
        let awaiting_ids: Vec<&str> = ui_state
            .active_threads
            .iter()
            .filter(|t| t.awaiting)
            .map(|t| t.ticket_id.as_str())
            .collect();
        assert_eq!(awaiting_ids, vec!["T-001"]);
    }

    #[test]
    fn test_fire_notify_noop_when_project_root_empty() {
        // Default State has an empty project_root; fire_notify must early-return
        // (never reaching the host run_command stub) so native tests are safe.
        let state = State::default();
        assert!(state.project_root.as_os_str().is_empty());
        state.fire_notify("complete", "noop", &[]); // must not panic
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

        // Create signal directory with idle signal (pane-based)
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.idle"), "2025-01-01T00:00:00Z").unwrap();

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

        // Agent slot maps pane 1 → T-001
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-001".to_string(), thread);

        state.check_idle_signals();

        // Verify: phase NOT advanced (still research)
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Research);

        // Verify: signal deleted
        assert!(!state.signal_dir.join("pane-1.idle").exists());

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
    fn test_idle_signal_review_with_artifact_advances_to_done() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // Create ticket file in review phase
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: high\nphase: review\n---\n\nBody\n",
        ).unwrap();

        // Create signal directory with idle signal
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.idle"), "2025-01-01T00:00:00Z").unwrap();

        let work_dir = dir.path().join("work");

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

        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        // Running thread in Review phase
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        state.threads.insert("T-001".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-001");
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();
        fs::write(staged.join("review.md"), "# Review summary").unwrap();
        write_passing_review_disposition(&state, &lease);

        state.check_idle_signals();

        // Thread remains Review while completion commit is pending.
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Running);
        assert!(state.pending_completions.contains_key("T-001"));

        // Signal file cleaned up
        assert!(!state.signal_dir.join("pane-1.idle").exists());

        // Ticket file remains non-Done until native preparation.
        let updated = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(updated.contains("phase: review"));

        // Review completion is not published early.
        assert!(!state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::PhaseCompleted { ticket_id, phase }
            if ticket_id == "T-001" && *phase == Phase::Review
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

        // Create signal for a pane whose ticket has NO thread
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.idle"), "2025-01-01T00:00:00Z").unwrap();

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

        // Agent slot maps pane 1 → T-001, but no thread exists
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.check_idle_signals();

        // Signal file should still be cleaned up
        assert!(!state.signal_dir.join("pane-1.idle").exists());

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
        fs::write(signal_dir.join("pane-1.idle"), "2025-01-01T00:00:00Z").unwrap();

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

        // Agent slot maps pane 1 → T-001
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        // Add a PARKED thread (not running)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        thread.park();
        state.threads.insert("T-001".to_string(), thread);

        state.check_idle_signals();

        // Signal cleaned up
        assert!(!state.signal_dir.join("pane-1.idle").exists());

        // Thread still parked, not advanced
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.status, lisa_core::types::ThreadStatus::Parked);
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

    // =========================================================================
    // Pause feature tests
    // =========================================================================

    #[test]
    fn test_pause_toggle_and_activity_log() {
        // Can't call handle_key directly (links zellij host fns), so
        // test the toggle logic and activity logging through state manipulation.
        let mut state = State::default();
        assert!(!state.paused);

        // Simulate what handle_key(space) does
        state.paused = !state.paused;
        state.log_activity(ActivityEvent::Info {
            message: "Scheduling paused".to_string(),
        });
        assert!(state.paused);
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Info { message } if message.contains("paused")
        )));

        // Toggle back
        state.paused = !state.paused;
        state.log_activity(ActivityEvent::Info {
            message: "Scheduling resumed".to_string(),
        });
        assert!(!state.paused);
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Info { message } if message.contains("resumed")
        )));
    }

    #[test]
    fn test_pause_propagates_to_ui_state() {
        let mut state = State::default();
        assert!(!state.to_ui_state().paused);

        state.paused = true;
        assert!(state.to_ui_state().paused);
    }

    #[test]
    fn test_pause_blocks_scheduling_precondition() {
        // We can't call schedule_ready_tickets directly (zellij host fns),
        // but we verify the guard condition includes paused state.
        // The guard at the top of schedule_ready_tickets is:
        //   if !self.permissions_granted || !self.slots_discovered || self.paused { return; }
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\nBody\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                ..PluginConfig::new()
            },
            permissions_granted: true,
            slots_discovered: true,
            paused: true,
            ..State::default()
        };

        // Ready tickets exist
        assert!(!state.dag.get_ready_tickets().is_empty());
        // But scheduling is paused
        assert!(state.paused);
    }

    #[test]
    fn test_concurrency_cap_respects_max_threads() {
        // Verify the concurrency guard logic: when running_count >= max_threads,
        // new tickets should not be scheduled even if idle slots exist.
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        // Create 3 ready tickets
        for i in 1..=3 {
            fs::write(
                tickets_dir.join(format!("T-00{}.md", i)),
                format!(
                    "---\nid: T-00{}\ntitle: ticket-{}\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\nBody\n",
                    i, i
                ),
            ).unwrap();
        }

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                max_threads: 2,
                ..PluginConfig::new()
            },
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };

        // Create 4 agent slots (2x max_threads) to simulate the new layout
        for i in 0..4 {
            state.agent_slots.push(AgentSlot {
                pane_id: 10 + i,
                ticket_id: None,
                attempt_lease: None,
                has_session: false,
                transition_state: TransitionState::Idle,
                transition_started_at: None,
                cooldown_until: None,
                last_activity_at: None,
                last_client: None,
            });
        }

        // Insert 2 running threads (at max_threads capacity)
        state
            .threads
            .insert("T-001".to_string(), Thread::new("T-001", 10));
        state
            .threads
            .insert("T-002".to_string(), Thread::new("T-002", 11));

        // Verify: 3 ready tickets, 2 running threads, 4 idle slots
        assert_eq!(state.dag.get_ready_tickets().len(), 3);
        let running = state
            .threads
            .values()
            .filter(|t| t.status == lisa_core::types::ThreadStatus::Running)
            .count();
        assert_eq!(running, 2);
        assert_eq!(state.config.max_threads, 2);

        // The concurrency guard: running_count >= max_threads should be true
        assert!(running >= state.config.max_threads);
        // Even though idle slots exist
        assert!(state.agent_slots.iter().any(|s| s.ticket_id.is_none()));
    }

    // ---- T-026-02: provider-aware concurrency ----

    fn running_thread(id: &str, pane: u32, client: AgentClient) -> lisa_core::types::Thread {
        let mut t = lisa_core::types::Thread::new(id, pane);
        t.client = client;
        t
    }

    fn fresh_slot(pane_id: u32, last_client: Option<AgentClient>) -> AgentSlot {
        AgentSlot {
            pane_id,
            ticket_id: None,
            attempt_lease: None,
            has_session: last_client.is_some(),
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client,
        }
    }

    fn pane_name_schedule_state(
        requested_agent: &str,
        default_agent: AgentClient,
        resident_agent: Option<AgentClient>,
    ) -> (State, tempfile::TempDir) {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-NAME.md"),
            format!(
                "---\nid: T-NAME\ntitle: pane lifecycle\ntype: task\nstatus: open\npriority: high\nphase: ready\nagent: {requested_agent}\n---\n"
            ),
        )
        .unwrap();
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir: dir.path().join("work"),
                client: default_agent,
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            signal_dir: dir.path().join("signals"),
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };
        state.agent_slots.push(fresh_slot(10, resident_agent));
        (state, dir)
    }

    fn consecutive_reuse_state(
        provider: AgentClient,
        ticket_prefix: &str,
        pane_ids: &[u32],
    ) -> (State, tempfile::TempDir) {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        let provider_name = match provider {
            AgentClient::Claude => "claude",
            AgentClient::Codex => "codex",
        };
        for sequence in 1..=10 {
            let ticket_id = format!("{ticket_prefix}-{sequence:02}");
            fs::write(
                tickets_dir.join(format!("{ticket_id}.md")),
                format!(
                    "---\nid: {ticket_id}\ntitle: consecutive reuse {sequence:02}\ntype: task\nstatus: open\npriority: high\nphase: ready\nagent: {provider_name}\n---\n"
                ),
            )
            .unwrap();
        }

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir: dir.path().join("work"),
                client: provider,
                max_threads: pane_ids.len(),
                wind_down_secs: 0,
                assignment_ack_timeout_secs: 1,
                ..PluginConfig::new()
            },
            signal_dir: dir.path().join("signals"),
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };
        state.agent_slots.extend(
            pane_ids
                .iter()
                .map(|pane_id| fresh_slot(*pane_id, Some(provider))),
        );
        (state, dir)
    }

    fn refresh_fixture_dag(state: &mut State) {
        let tickets = lisa_core::ticket::scan_tickets(&state.config.ticket_dir).unwrap();
        state.dag = Dag::from_tickets(tickets).unwrap();
    }

    fn active_ticket_panes(state: &State) -> Vec<(TicketId, u32)> {
        let mut active: Vec<(TicketId, u32)> = state
            .agent_slots
            .iter()
            .filter_map(|slot| {
                slot.ticket_id
                    .as_ref()
                    .map(|ticket_id| (ticket_id.clone(), slot.pane_id))
            })
            .collect();
        active.sort();
        active
    }

    #[test]
    fn codex_stub_panes_receive_only_fresh_per_ticket_launcher_lines() {
        let (mut state, _dir) = consecutive_reuse_state(AgentClient::Codex, "T-LAUNCH", &[10, 11]);
        state.config.lisa_bin = Some("/fixture path/lisa'bin".to_string());
        for slot in &mut state.agent_slots {
            slot.has_session = false;
            slot.last_client = None;
        }

        // Native tests link no-op Zellij host calls. Scheduling still crosses
        // the production send_line_to_pane boundary and queues each deferred
        // Enter, making these two empty slots deterministic stub panes.
        state.schedule_ready_tickets();

        let active = active_ticket_panes(&state);
        assert_eq!(active.len(), 2, "both empty stub panes receive a ticket");
        assert_eq!(
            state.pending_enters.len(),
            2,
            "one actual pane submission is queued per fresh TUI"
        );

        let mut assignment_paths = std::collections::HashSet::new();
        let mut launch_paths = std::collections::HashSet::new();
        for (ticket_id, pane_id) in active {
            let lease = state.current_leases[&ticket_id].clone();
            let assignment_ref = &state.assignment_refs[&ticket_id];
            assert_eq!(assignment_ref.lease, lease);
            assert!(assignment_paths.insert(assignment_ref.path.clone()));

            let assignment_body = std::fs::read_to_string(&assignment_ref.path).unwrap();
            assert!(assignment_body.contains("Read the ticket"));
            assert!(assignment_body.contains("AGENTS.md"));

            let launch_path = state
                .attempt_work_dir(&lease)
                .join(format!(".lisa-launch-{pane_id}.sh"));
            assert!(launch_paths.insert(launch_path.clone()));
            let launch_script = std::fs::read_to_string(&launch_path).unwrap();
            let pane_assignment_path = strip_host_prefix(&assignment_ref.path);

            assert!(
                launch_script.contains("'/fixture path/lisa'\"'\"'bin' launch-codex"),
                "script invokes the resolved Lisa launcher: {launch_script}"
            );
            assert!(launch_script.contains(&format!(
                " -- {} ||",
                shell_quote(&pane_assignment_path.to_string_lossy())
            )));
            assert!(!launch_script.contains("Read the ticket"));
            assert!(!launch_script.contains("AGENTS.md"));
            assert!(!launch_script.contains(" codex --dangerously"));

            let pane_line = state
                .activity_log
                .iter()
                .find_map(|event| match event {
                    ActivityEvent::SessionLaunch {
                        ticket_id: launched_ticket,
                        pane_id: launched_pane,
                        command,
                    } if launched_ticket == &ticket_id && launched_pane == &pane_id => {
                        Some(command)
                    }
                    _ => None,
                })
                .expect("fresh dispatch records its exact pane line");
            assert_eq!(
                pane_line,
                &format!(
                    "sh {}",
                    shell_quote(&strip_host_prefix(&launch_path).to_string_lossy())
                )
            );
            assert!(!pane_line.contains("Read the ticket"));
            assert!(!pane_line.contains("AGENTS.md"));
            assert!(!pane_line.contains("launch-codex"));

            let slot = state
                .agent_slots
                .iter()
                .find(|slot| slot.pane_id == pane_id)
                .unwrap();
            assert!(slot.has_session, "launcher starts a fresh resident TUI");
            assert_eq!(slot.last_client, Some(AgentClient::Codex));
            assert_eq!(slot.transition_state, TransitionState::Idle);
            assert!(matches!(
                state.seat_assignment(pane_id),
                Some(SeatAssignmentState::Starting { generation, .. })
                    if generation == lease.attempt_id
            ));
        }

        assert_eq!(assignment_paths.len(), 2);
        assert_eq!(launch_paths.len(), 2);
    }

    #[test]
    fn codex_completion_exits_revokes_and_launches_next_fresh_tui() {
        use std::fs;

        const PREDECESSOR: &str = "T-BOUNDARY-01";
        const SUCCESSOR: &str = "T-BOUNDARY-02";
        const PANE_ID: u32 = 10;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let completion_journal = dir.path().join("completion-journal.jsonl");
        let provenance_ledger = dir.path().join("provenance.jsonl");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join(format!("{PREDECESSOR}.md")),
            format!(
                "---\nid: {PREDECESSOR}\ntitle: completed boundary\ntype: task\nstatus: open\npriority: high\nphase: ready\nagent: codex\n---\n"
            ),
        )
        .unwrap();
        fs::write(
            tickets_dir.join(format!("{SUCCESSOR}.md")),
            format!(
                "---\nid: {SUCCESSOR}\ntitle: fresh successor\ntype: task\nstatus: open\npriority: high\nphase: ready\nagent: codex\ndepends_on: [{PREDECESSOR}]\n---\n"
            ),
        )
        .unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: dir.path().join("work"),
                client: AgentClient::Codex,
                lisa_bin: Some("/fixture/lisa".to_string()),
                max_threads: 1,
                wind_down_secs: 0,
                assignment_ack_timeout_secs: 1,
                ..PluginConfig::new()
            },
            project_root: dir.path().to_path_buf(),
            git_root: dir.path().to_path_buf(),
            signal_dir: dir.path().join("signals"),
            attempt_dir: dir.path().join("attempts"),
            ledger_path: provenance_ledger.clone(),
            completion_journal_path: completion_journal.clone(),
            completion_journal_healthy: true,
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };
        state.agent_slots.push(fresh_slot(PANE_ID, None));

        state.schedule_ready_tickets();
        let predecessor_lease = state.current_leases[PREDECESSOR].clone();
        let predecessor_assignment = state.assignment_refs[PREDECESSOR].clone();
        let startup_deadline = match state.seat_assignment(PANE_ID) {
            Some(SeatAssignmentState::Starting {
                generation,
                start_deadline: Some(deadline),
                relaunches: 0,
            }) if generation == predecessor_lease.attempt_id => deadline,
            other => panic!("expected fresh predecessor startup, got {other:?}"),
        };
        state.check_assignment_ack_timeouts_at(startup_deadline);
        assert!(matches!(
            state.seat_assignment(PANE_ID),
            Some(SeatAssignmentState::Delivering { generation, .. })
                if generation == predecessor_lease.attempt_id
        ));

        let predecessor_claim = AssignmentClaim {
            ticket_id: PREDECESSOR.to_string(),
            attempt_id: predecessor_lease.attempt_id,
            nonce: predecessor_assignment.nonce,
        };
        assert!(state.admit_assignment_claim(PANE_ID, &predecessor_claim));
        assert_eq!(
            state.seat_assignment(PANE_ID),
            Some(SeatAssignmentState::Owned)
        );
        println!(
            "T0450401|boundary|step=claimed|ticket={PREDECESSOR}|pane={PANE_ID}|attempt={}|nonce={}",
            predecessor_claim.attempt_id, predecessor_claim.nonce
        );

        ticket::update_ticket_phase(tickets_dir.join(format!("{PREDECESSOR}.md")), Phase::Review)
            .unwrap();
        refresh_fixture_dag(&mut state);
        state.threads.get_mut(PREDECESSOR).unwrap().current_phase = Phase::Review;
        let predecessor_work = state.attempt_work_dir(&predecessor_lease);
        fs::create_dir_all(&predecessor_work).unwrap();
        fs::write(
            predecessor_work.join("review.md"),
            "# Review\n\nThe claimed Codex attempt is ready to complete.\n",
        )
        .unwrap();
        write_t046_note_disposition(&state, &predecessor_lease);

        state.check_artifact_advances();
        let pending = state.pending_completions[PREDECESSOR].clone();
        assert_eq!(pending.source, CompletionSource::Artifact);
        assert_eq!(
            pending.authority,
            CompletionAuthority::Attempt(predecessor_lease.clone())
        );
        assert_eq!(
            state.launched_completion_effects,
            vec![EffectCommand::LaunchCompletion {
                attempt_id: AttemptId::new(predecessor_lease.attempt_id.to_string()),
                completion_id: CompletionId::new(PREDECESSOR),
            }]
        );
        let in_flight_journal = fs::read_to_string(&completion_journal).unwrap();
        assert_eq!(in_flight_journal.lines().count(), 2);
        assert_eq!(
            in_flight_journal.matches("\"state\":\"requested\"").count(),
            1
        );
        assert_eq!(
            in_flight_journal
                .matches("\"state\":\"command-in-flight\"")
                .count(),
            1
        );

        state.check_artifact_advances();
        assert!(!state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: PREDECESSOR.to_string(),
            source_lease: predecessor_lease.clone(),
        }));
        assert_eq!(state.launched_completion_effects.len(), 1);
        assert_eq!(
            fs::read_to_string(&completion_journal).unwrap(),
            in_flight_journal,
            "repeated Review evidence must not inject another completion command"
        );

        ticket::update_ticket_done(tickets_dir.join(format!("{PREDECESSOR}.md"))).unwrap();
        let commit_id = vec![b'c'; 40];
        state.handle_completion_result(PREDECESSOR, Some(0), commit_id.clone(), Vec::new());

        assert!(!state.pending_completions.contains_key(PREDECESSOR));
        assert!(!state.threads.contains_key(PREDECESSOR));
        let aggregate = &state.completion_aggregates[PREDECESSOR];
        assert_eq!(aggregate.completion_key(), &pending.completion_key);
        assert_eq!(aggregate.state(), &CompletionState::Confirmed);
        assert_eq!(aggregate.completion_note(), Some(&t046_completion_note()));
        assert_eq!(
            aggregate.confirmed_commit_id(),
            Some(String::from_utf8_lossy(&commit_id).as_ref())
        );

        let confirmed_journal = fs::read_to_string(&completion_journal).unwrap();
        let confirmed_provenance = fs::read_to_string(&provenance_ledger).unwrap();
        state.handle_completion_result(PREDECESSOR, Some(0), commit_id, Vec::new());
        assert_eq!(
            fs::read_to_string(&completion_journal).unwrap(),
            confirmed_journal,
            "duplicate result delivery must not append a second confirmation"
        );
        assert_eq!(
            fs::read_to_string(&provenance_ledger).unwrap(),
            confirmed_provenance,
            "duplicate result delivery must not append a second completion record"
        );
        assert_eq!(state.launched_completion_effects.len(), 1);
        assert_eq!(confirmed_journal.lines().count(), 3);
        assert_eq!(
            confirmed_journal.matches("\"state\":\"requested\"").count(),
            1
        );
        assert_eq!(
            confirmed_journal
                .matches("\"state\":\"command-in-flight\"")
                .count(),
            1
        );
        assert_eq!(
            confirmed_journal.matches("\"state\":\"confirmed\"").count(),
            1
        );
        let completion_records = read_ledger(&provenance_ledger);
        assert_eq!(completion_records.len(), 1);
        assert_eq!(completion_records[0].ticket_id, PREDECESSOR);
        assert_eq!(
            completion_records[0].attempt_lease,
            predecessor_lease.clone()
        );
        assert_eq!(completion_records[0].outcome, RunOutcome::Done);
        assert_eq!(
            completion_records[0].completion_note.as_ref(),
            Some(&t046_completion_note())
        );
        assert!(completion_records[0].authoritative);
        assert!(!completion_records[0].fenced);
        println!(
            "T0450402|completion|ticket={PREDECESSOR}|effects=1|confirmed=1|authoritative=1|duplicate_result=ignored"
        );

        assert_eq!(
            state.attempt_lifecycle,
            vec![
                AttemptLifecycleEvent::LeaseRevoked {
                    ticket_id: PREDECESSOR.to_string(),
                },
                AttemptLifecycleEvent::SlotReleased {
                    ticket_id: PREDECESSOR.to_string(),
                },
                AttemptLifecycleEvent::CleanExitRequested {
                    ticket_id: PREDECESSOR.to_string(),
                    pane_id: PANE_ID,
                },
            ]
        );
        assert!(!state.current_leases.contains_key(PREDECESSOR));
        assert_eq!(
            state.lease_high_water.get(PREDECESSOR),
            Some(&predecessor_lease)
        );
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert!(state.agent_slots[0].attempt_lease.is_none());
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForExit
        );
        assert!(!state.agent_slots[0].has_session);
        assert_eq!(state.agent_slots[0].last_client, Some(AgentClient::Codex));
        assert_eq!(state.seat_assignment(PANE_ID), None);
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Info { message }
                if message.contains("Completion boundary revoked")
                    && message.contains(PREDECESSOR)
                    && message.contains(&PANE_ID.to_string())
        )));

        assert!(
            !state.admit_assignment_claim(PANE_ID, &predecessor_claim),
            "the exact predecessor nonce must lose authority at completion"
        );
        assert_eq!(state.seat_assignment(PANE_ID), None);
        println!(
            "T0450401|boundary|step=exit-requested|ticket={PREDECESSOR}|pane={PANE_ID}|lease=revoked|late_claim=rejected"
        );

        let launches_before_exit = state
            .activity_log
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    ActivityEvent::SessionLaunch { ticket_id, .. } if ticket_id == SUCCESSOR
                )
            })
            .count();
        state.schedule_ready_tickets();
        assert!(!state.current_leases.contains_key(SUCCESSOR));
        assert!(!state.assignment_refs.contains_key(SUCCESSOR));
        assert!(!state.threads.contains_key(SUCCESSOR));
        assert_eq!(
            state
                .activity_log
                .iter()
                .filter(|event| matches!(
                    event,
                    ActivityEvent::SessionLaunch { ticket_id, .. } if ticket_id == SUCCESSOR
                ))
                .count(),
            launches_before_exit,
            "the successor cannot launch while the predecessor TUI exits"
        );

        state.agent_slots[0].transition_started_at = Some(
            std::time::SystemTime::now()
                - std::time::Duration::from_secs(AGENT_EXIT_GRACE_SECS + 1),
        );
        state.check_transition_timeouts();
        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
        assert!(!state.agent_slots[0].has_session);
        assert_eq!(state.agent_slots[0].last_client, None);
        assert_eq!(
            state.last_pane_names.get(&PANE_ID).map(String::as_str),
            Some("lisa · idle")
        );
        println!(
            "T0450401|boundary|step=shell-ready|pane={PANE_ID}|resident=none|next_reserved=false"
        );

        state.schedule_ready_tickets();
        let successor_lease = state.current_leases[SUCCESSOR].clone();
        let successor_assignment = state.assignment_refs[SUCCESSOR].clone();
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some(SUCCESSOR));
        assert_eq!(
            state.agent_slots[0].attempt_lease.as_ref(),
            Some(&successor_lease)
        );
        assert!(state.agent_slots[0].has_session);
        assert_eq!(state.agent_slots[0].last_client, Some(AgentClient::Codex));
        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
        assert!(matches!(
            state.seat_assignment(PANE_ID),
            Some(SeatAssignmentState::Starting { generation, .. })
                if generation == successor_lease.attempt_id
        ));
        assert_ne!(successor_assignment.path, predecessor_assignment.path);
        assert_ne!(successor_assignment.nonce, predecessor_assignment.nonce);
        let launch_path = state
            .attempt_work_dir(&successor_lease)
            .join(format!(".lisa-launch-{PANE_ID}.sh"));
        let launch_script = fs::read_to_string(launch_path).unwrap();
        assert!(launch_script.contains("'/fixture/lisa' launch-codex"));
        assert!(launch_script.contains(&shell_quote(
            &strip_host_prefix(&successor_assignment.path).to_string_lossy()
        )));
        assert!(
            !state.admit_assignment_claim(PANE_ID, &predecessor_claim),
            "a new TUI cannot make the predecessor nonce authoritative again"
        );
        assert!(!state.seat_is_owned(PANE_ID));
        println!(
            "T0450401|boundary|step=fresh-launch|ticket={SUCCESSOR}|pane={PANE_ID}|attempt={}|nonce={}|state=starting|predecessor_claim=rejected",
            successor_lease.attempt_id, successor_assignment.nonce
        );
    }

    fn acknowledge_assignment(
        state: &mut State,
        pane_id: u32,
        ticket_id: &str,
        generation: u64,
    ) -> bool {
        let payload = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "consecutive reuse proof",
                codex_ack::CodexAssignmentRef {
                    ticket_id,
                    generation,
                },
            ),
        });
        state.acknowledge_codex_assignment(pane_id, &payload.to_string())
    }

    /// Advance a provider policy that exits a resident TUI before launching a
    /// fresh process, then elapse Codex's bounded startup grace into the first
    /// tagged chat delivery. This mirrors the production timer sequence without
    /// sleeping or inventing a pre-prompt Codex SessionStart signal.
    fn exit_then_deliver_fresh_codex(
        state: &mut State,
        pane_id: u32,
        lease: &AttemptLease,
    ) -> std::time::SystemTime {
        assert_eq!(
            state
                .agent_slots
                .iter()
                .find(|slot| slot.pane_id == pane_id)
                .map(|slot| slot.transition_state),
            Some(TransitionState::WaitingForExit)
        );
        assert!(matches!(
            state.seat_assignment(pane_id),
            Some(SeatAssignmentState::Starting {
                generation,
                start_deadline: None,
                ..
            }) if generation == lease.attempt_id
        ));

        let slot = state
            .agent_slots
            .iter_mut()
            .find(|slot| slot.pane_id == pane_id)
            .unwrap();
        slot.transition_started_at = Some(
            std::time::SystemTime::now()
                - std::time::Duration::from_secs(AGENT_EXIT_GRACE_SECS + 1),
        );
        state.check_transition_timeouts();

        let grace_deadline = match state.seat_assignment(pane_id) {
            Some(SeatAssignmentState::Starting {
                generation,
                start_deadline: Some(deadline),
                ..
            }) if generation == lease.attempt_id => deadline,
            other => panic!("expected fresh Codex startup grace, got {other:?}"),
        };
        state.check_assignment_ack_timeouts_at(grace_deadline);
        match state.seat_assignment(pane_id) {
            Some(SeatAssignmentState::Delivering {
                generation,
                ack_deadline,
                retries: 0,
            }) if generation == lease.attempt_id => ack_deadline,
            other => panic!("expected fresh Codex chat delivery, got {other:?}"),
        }
    }

    #[test]
    fn test_pane_title_rename_gate_deduplicates() {
        let mut state = State::default();
        state.agent_slots.push(fresh_slot(10, None));

        assert!(state.rename_slot(10, "lisa · idle".to_string()));
        assert!(!state.rename_slot(10, "lisa · idle".to_string()));
        assert!(state.rename_slot(10, "codex · idle".to_string()));
        assert_eq!(
            state.last_pane_names.get(&10).map(String::as_str),
            Some("codex · idle")
        );
        assert!(!state.rename_slot(99, "lisa · idle".to_string()));
        assert!(!state.last_pane_names.contains_key(&99));
    }

    #[test]
    fn test_pane_title_fresh_launch_uses_actual_fallback_route() {
        let (mut state, _dir) =
            pane_name_schedule_state("not-a-provider", AgentClient::Codex, None);

        state.schedule_ready_tickets();

        assert_eq!(
            state.last_pane_names.get(&10).map(String::as_str),
            Some("codex · T-NAME · pane lifecycle")
        );
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-NAME"));
        assert!(state.agent_slots[0].has_session);
        assert_eq!(state.threads["T-NAME"].client, AgentClient::Codex);
        assert!(
            matches!(
                state.seat_assignment(10),
                Some(SeatAssignmentState::Starting {
                    generation: 1,
                    start_deadline: Some(_),
                    ..
                })
            ),
            "a fresh Codex launch awaits its exact process-start signal"
        );
        assert!(!state.seat_is_owned(10));
    }

    #[test]
    fn scheduler_records_provider_readiness_mode_at_dispatch() {
        // T-037-01-01: the scheduler reads the adapter's readiness mode at launch
        // dispatch and records it per pane, with no seat-state behavior change.
        let (mut codex, _codex_dir) = pane_name_schedule_state("codex", AgentClient::Codex, None);
        codex.schedule_ready_tickets();
        assert_eq!(
            codex.seat_readiness_mode(10),
            Some(ReadinessMode::Grace),
            "a fresh Codex launch is classified as grace-paced readiness"
        );
        assert!(
            matches!(
                codex.seat_assignment(10),
                Some(SeatAssignmentState::Starting { generation: 1, .. })
            ),
            "recording the mode does not change the Starting seat state"
        );

        let (mut claude, _claude_dir) =
            pane_name_schedule_state("claude", AgentClient::Claude, None);
        claude.schedule_ready_tickets();
        assert_eq!(
            claude.seat_readiness_mode(10),
            Some(ReadinessMode::SessionStart),
            "a fresh Claude launch is classified as SessionStart-proven readiness"
        );
    }

    #[test]
    fn codex_startup_grace_paces_first_prompt_into_delivering() {
        // T-037-01-02: a grace-mode (Codex) seat, with no process-start signal
        // ever emitted, remains Starting until its bounded named startup grace
        // elapses, then attempts the tagged chat assignment and enters Delivering
        // directly — never ReadyForAssignment, StartupFailed, ResettingStartup,
        // or Owned merely because time passed. Ownership is published only by the
        // exact current-attempt UserPromptSubmit.
        let (mut codex, _codex_dir) = pane_name_schedule_state("codex", AgentClient::Codex, None);
        codex.schedule_ready_tickets();
        assert_eq!(codex.seat_readiness_mode(10), Some(ReadinessMode::Grace));
        let lease = codex.current_leases["T-NAME"].clone();
        let grace_deadline = match codex.seat_assignment(10) {
            Some(SeatAssignmentState::Starting {
                generation,
                start_deadline: Some(deadline),
                relaunches: 0,
            }) => {
                assert_eq!(generation, lease.attempt_id);
                deadline
            }
            other => panic!("expected a fresh Codex Starting with a named grace, got {other:?}"),
        };
        assert!(!codex.seat_is_owned(10));

        // A stray process-start signal must not exist for grace mode; before the
        // grace elapses the seat is still Starting, nothing has been delivered.
        assert!(matches!(
            codex.seat_assignment(10),
            Some(SeatAssignmentState::Starting { .. })
        ));

        // The named grace elapses: pace the first prompt directly into Delivering.
        codex.check_assignment_ack_timeouts_at(grace_deadline);
        match codex.seat_assignment(10) {
            Some(SeatAssignmentState::Delivering {
                generation,
                retries: 0,
                ..
            }) => assert_eq!(generation, lease.attempt_id),
            other => panic!("grace elapse must enter Delivering directly, got {other:?}"),
        }
        assert!(
            !codex.seat_is_owned(10),
            "elapsed grace never publishes Owned"
        );
        assert_eq!(
            codex.to_ui_state().seat_assignment_statuses.get(&1),
            Some(&ui::SeatAssignmentStatus::Delivering),
            "grace never surfaces ReadyForAssignment or StartupFailed"
        );

        // Ownership is gated solely on the exact current-attempt acknowledgement.
        assert!(
            !acknowledge_assignment(&mut codex, 10, "T-NAME", lease.attempt_id + 1),
            "a stale-generation payload cannot own the paced assignment"
        );
        assert!(!codex.seat_is_owned(10));
        assert!(acknowledge_assignment(
            &mut codex,
            10,
            "T-NAME",
            lease.attempt_id,
        ));
        assert_eq!(codex.seat_assignment(10), Some(SeatAssignmentState::Owned));
    }

    #[test]
    fn session_start_seat_never_paces_on_grace_and_still_requires_the_signal() {
        // T-037-01-02: the SessionStart-mode (Claude) path is unchanged. Its
        // Starting deadline does NOT auto-deliver; it enters same-pane startup
        // recovery, and only a matching process-start signal reaches
        // ReadyForAssignment.
        let (mut claude, _claude_dir) =
            pane_name_schedule_state("claude", AgentClient::Claude, None);
        claude.config.assignment_ack_timeout_secs = 1;
        claude.schedule_ready_tickets();
        assert_eq!(
            claude.seat_readiness_mode(10),
            Some(ReadinessMode::SessionStart)
        );
        let lease = claude.current_leases["T-NAME"].clone();
        assert!(matches!(
            claude.seat_assignment(10),
            Some(SeatAssignmentState::Starting {
                start_deadline: Some(_),
                relaunches: 0,
                ..
            })
        ));

        // A matching process-start signal is the only route to ReadyForAssignment.
        assert!(claude.acknowledge_process_start(10, &lease));
        assert_eq!(
            claude.seat_assignment(10),
            Some(SeatAssignmentState::ReadyForAssignment {
                generation: lease.attempt_id,
            })
        );

        // On a fresh Claude launch, the elapsed deadline enters startup recovery,
        // never a paced Delivering.
        let (mut claude2, _claude_dir2) =
            pane_name_schedule_state("claude", AgentClient::Claude, None);
        claude2.config.assignment_ack_timeout_secs = 1;
        claude2.schedule_ready_tickets();
        let deadline2 = match claude2.seat_assignment(10) {
            Some(SeatAssignmentState::Starting {
                start_deadline: Some(deadline),
                ..
            }) => deadline,
            other => panic!("expected a fresh Claude Starting, got {other:?}"),
        };
        claude2.check_assignment_ack_timeouts_at(deadline2);
        assert!(
            matches!(
                claude2.seat_assignment(10),
                Some(SeatAssignmentState::ResettingStartup { .. })
            ),
            "a SessionStart seat recovers on deadline; it never paces into Delivering"
        );
    }

    /// Count the operator-visible "delivering assignment" info logs for a ticket
    /// — one per actual chat send, so it distinguishes the initial paced send
    /// from its bounded retry.
    fn delivery_log_count(state: &State, ticket_id: &str) -> usize {
        let needle = format!("delivering assignment for {ticket_id}");
        state
            .activity_log
            .iter()
            .filter(|event| matches!(event, ActivityEvent::Info { message } if message.contains(&needle)))
            .count()
    }

    #[test]
    fn codex_delayed_send_reaches_owned_only_on_current_attempt_ack() {
        // T-037-01-03 (delayed-send regression): a grace-mode (Codex) seat holds
        // its first prompt through the bounded startup grace — a poll strictly
        // before the deadline delivers nothing and never fabricates a
        // ReadyForAssignment — then paces the send directly from Starting into
        // Delivering when the grace elapses. Ownership is published only by the
        // exact current-attempt UserPromptSubmit; elapsed time, a stale
        // generation, and a foreign ticket all fail to own.
        let (mut codex, _codex_dir) = pane_name_schedule_state("codex", AgentClient::Codex, None);
        codex.schedule_ready_tickets();
        assert_eq!(codex.seat_readiness_mode(10), Some(ReadinessMode::Grace));
        let lease = codex.current_leases["T-NAME"].clone();
        let grace_deadline = match codex.seat_assignment(10) {
            Some(SeatAssignmentState::Starting {
                generation,
                start_deadline: Some(deadline),
                relaunches: 0,
            }) => {
                assert_eq!(generation, lease.attempt_id);
                deadline
            }
            other => panic!("expected a fresh Codex Starting with a named grace, got {other:?}"),
        };

        // Delayed send: a poll strictly before the grace deadline paces nothing.
        // The seat stays Starting, surfaces Starting (never a synthetic
        // ReadyForAssignment), and no assignment has been delivered.
        codex.check_assignment_ack_timeouts_at(grace_deadline - std::time::Duration::from_secs(1));
        assert!(
            matches!(
                codex.seat_assignment(10),
                Some(SeatAssignmentState::Starting {
                    start_deadline: Some(_),
                    relaunches: 0,
                    ..
                })
            ),
            "the paced send is delayed until the grace deadline"
        );
        assert_eq!(
            codex.to_ui_state().seat_assignment_statuses.get(&1),
            Some(&ui::SeatAssignmentStatus::Starting),
            "before the grace elapses the seat never shows ReadyForAssignment"
        );
        assert_eq!(
            delivery_log_count(&codex, "T-NAME"),
            0,
            "nothing is delivered before the grace deadline"
        );
        assert!(!codex.seat_is_owned(10));

        // The grace elapses: pace the first prompt directly into Delivering,
        // with no ReadyForAssignment node in between.
        codex.check_assignment_ack_timeouts_at(grace_deadline);
        match codex.seat_assignment(10) {
            Some(SeatAssignmentState::Delivering {
                generation,
                retries: 0,
                ..
            }) => assert_eq!(generation, lease.attempt_id),
            other => panic!("grace elapse must enter Delivering directly, got {other:?}"),
        }
        assert_eq!(
            codex.to_ui_state().seat_assignment_statuses.get(&1),
            Some(&ui::SeatAssignmentStatus::Delivering),
            "the grace pace surfaces Delivering, never ReadyForAssignment"
        );
        assert_eq!(
            delivery_log_count(&codex, "T-NAME"),
            1,
            "the grace elapse issues exactly the first paced send"
        );
        assert!(
            !codex.seat_is_owned(10),
            "elapsed grace never publishes Owned"
        );

        // Owned is gated solely on the exact current-attempt acknowledgement.
        assert!(
            !acknowledge_assignment(&mut codex, 10, "T-NAME", lease.attempt_id + 1),
            "a stale-generation payload cannot own the paced assignment"
        );
        assert!(
            !acknowledge_assignment(&mut codex, 10, "T-OTHER", lease.attempt_id),
            "a foreign-ticket payload cannot own the paced assignment"
        );
        assert!(!codex.seat_is_owned(10));
        assert!(acknowledge_assignment(
            &mut codex,
            10,
            "T-NAME",
            lease.attempt_id,
        ));
        assert_eq!(codex.seat_assignment(10), Some(SeatAssignmentState::Owned));
    }

    #[test]
    fn codex_prompt_miss_waits_for_claim_then_times_out_never_owned() {
        // T-037-01-03 prompt-miss regression updated by T-045-03-03: when the
        // grace-paced send is never acknowledged, a live Codex seat waits
        // passively for ownership evidence, then reaches ClaimTimedOut.
        // Stale-attempt signals are rejected throughout, and even an exact-
        // generation ack arriving after the failure cannot resurrect ownership.
        let (mut codex, _codex_dir) = pane_name_schedule_state("codex", AgentClient::Codex, None);
        codex.config.assignment_ack_timeout_secs = 1;
        codex.schedule_ready_tickets();
        assert_eq!(codex.seat_readiness_mode(10), Some(ReadinessMode::Grace));
        let lease = codex.current_leases["T-NAME"].clone();
        let grace_deadline = match codex.seat_assignment(10) {
            Some(SeatAssignmentState::Starting {
                start_deadline: Some(deadline),
                relaunches: 0,
                ..
            }) => deadline,
            other => panic!("expected a fresh Codex Starting with a named grace, got {other:?}"),
        };

        // Grace elapses into the first paced send; no matching ack is ever sent.
        codex.check_assignment_ack_timeouts_at(grace_deadline);
        let first_deadline = match codex.seat_assignment(10) {
            Some(SeatAssignmentState::Delivering {
                generation,
                ack_deadline,
                retries: 0,
            }) => {
                assert_eq!(generation, lease.attempt_id);
                ack_deadline
            }
            other => panic!("grace elapse must enter Delivering directly, got {other:?}"),
        };
        assert!(!codex.seat_is_owned(10));

        // The old acceptance clock elapses once with no evidence. The live TUI
        // enters passive claim wait without another delivery.
        codex.check_assignment_ack_timeouts_at(first_deadline);
        let claim_deadline = match codex.seat_assignment(10) {
            Some(SeatAssignmentState::DeliveredAwaitingClaim {
                generation,
                claim_deadline,
            }) => {
                assert_eq!(generation, lease.attempt_id);
                claim_deadline
            }
            other => panic!("expected passive delivered claim wait, got {other:?}"),
        };
        assert_eq!(
            delivery_log_count(&codex, "T-NAME"),
            1,
            "passive claim wait must not re-inject the paced send"
        );
        assert!(!codex.seat_is_owned(10));

        // A stale-attempt signal mid-miss is rejected and never owns.
        assert!(
            !acknowledge_assignment(&mut codex, 10, "T-NAME", lease.attempt_id + 1),
            "a stale-generation payload cannot own a missing-ack seat"
        );
        assert!(!codex.seat_is_owned(10));

        // The miss resolves in the named, operator-visible ClaimTimedOut state,
        // retaining the reservation and current lease for an explicit reset.
        codex.check_assignment_ack_timeouts_at(claim_deadline);
        assert_eq!(
            codex.seat_assignment(10),
            Some(SeatAssignmentState::ClaimTimedOut)
        );
        assert_eq!(
            codex.to_ui_state().seat_assignment_statuses.get(&1),
            Some(&ui::SeatAssignmentStatus::ClaimTimedOut),
            "the resolved miss is a named, operator-visible failure"
        );
        assert_eq!(
            codex.threads["T-NAME"].status,
            lisa_core::types::ThreadStatus::Failed
        );
        assert_eq!(codex.agent_slots[0].attempt_lease.as_ref(), Some(&lease));
        assert_eq!(codex.current_leases.get("T-NAME"), Some(&lease));
        assert!(!codex.seat_is_owned(10));

        // Terminal: an exact-generation ack arriving after the failure cannot own.
        assert!(
            !acknowledge_assignment(&mut codex, 10, "T-NAME", lease.attempt_id),
            "ClaimTimedOut is terminal; a late exact ack cannot publish Owned"
        );
        assert_eq!(
            codex.seat_assignment(10),
            Some(SeatAssignmentState::ClaimTimedOut)
        );
        assert!(!codex.seat_is_owned(10));
    }

    #[test]
    fn dispatch_mints_and_stamps_strictly_new_attempt_lease() {
        let (mut state, _dir) = pane_name_schedule_state("claude", AgentClient::Claude, None);
        let ticket_id = "T-NAME".to_string();

        state.schedule_ready_tickets();

        let first = state.current_leases[&ticket_id].clone();
        assert_eq!(first.ticket_id, ticket_id);
        assert_eq!(first.attempt_id, 1);
        assert_eq!(state.lease_high_water.get(&ticket_id), Some(&first));
        assert_eq!(
            state.threads[&ticket_id].attempt_lease.as_ref(),
            Some(&first),
            "the logical thread carries the ticket's current lease"
        );
        assert_eq!(
            state.agent_slots[0].attempt_lease.as_ref(),
            Some(&first),
            "the assigned physical seat carries the same lease"
        );
        let marker: AttemptLease = serde_json::from_str(
            &std::fs::read_to_string(state.signal_dir.join("pane-10.lease")).unwrap(),
        )
        .unwrap();
        assert_eq!(marker, first, "heartbeat marker carries the exact lease");

        state.release_slot_for_ticket(&ticket_id);
        state.threads.remove(&ticket_id);
        // Make eligibility deterministic independently of clock granularity.
        state.agent_slots[0].cooldown_until =
            Some(std::time::SystemTime::now() - std::time::Duration::from_secs(1));
        assert_eq!(state.agent_slots[0].ticket_id, None);
        assert_eq!(state.agent_slots[0].attempt_lease, None);
        assert_eq!(
            state.current_leases.get(&ticket_id),
            None,
            "release revokes the prior attempt before redispatch"
        );
        assert!(!first.is_current(state.current_leases.get(&ticket_id)));
        assert_eq!(
            state.lease_high_water.get(&ticket_id),
            Some(&first),
            "release retains only the predecessor needed by redispatch"
        );

        state.schedule_ready_tickets();

        let second = state.current_leases[&ticket_id].clone();
        assert_eq!(second.attempt_id, 2);
        assert!(second.attempt_id > first.attempt_id);
        assert_eq!(state.lease_high_water.get(&ticket_id), Some(&second));
        assert!(!first.is_current(Some(&second)));
        assert!(second.is_current(Some(&second)));
        assert_eq!(
            state.threads[&ticket_id].attempt_lease.as_ref(),
            Some(&second),
            "the redispatched thread carries the successor lease"
        );
        assert_eq!(
            state.agent_slots[0].attempt_lease.as_ref(),
            Some(&second),
            "the reassigned seat carries the successor lease"
        );
        let marker: AttemptLease = serde_json::from_str(
            &std::fs::read_to_string(state.signal_dir.join("pane-10.lease")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            marker, first,
            "the predecessor marker remains while the resident session clears"
        );
        state.handle_cleared_signal(10);
        let marker: AttemptLease = serde_json::from_str(
            &std::fs::read_to_string(state.signal_dir.join("pane-10.lease")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            marker, second,
            "the successor marker is published at prompt delivery"
        );
    }

    fn dashboard_thread_row(state: &State, ticket_id: &str) -> String {
        let ui_state = state.to_ui_state();
        let mut lines = Vec::new();
        ui::render_threads(&ui_state, &mut lines);
        let row = lines
            .iter()
            .find(|line| line.contains(ticket_id))
            .expect("ticket row should be present");

        let mut visible = String::new();
        let mut chars = row.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' && chars.next_if_eq(&'[').is_some() {
                for code in chars.by_ref() {
                    if code == 'm' {
                        break;
                    }
                }
            } else {
                visible.push(ch);
            }
        }
        let visible = visible.trim_end();
        let (row, _elapsed) = visible
            .rsplit_once(' ')
            .expect("active dashboard row should end with elapsed time");
        format!("{row} <elapsed>")
    }

    #[test]
    fn test_fresh_dispatch_requires_start_then_chat_ack_for_both_providers() {
        for provider in [AgentClient::Claude, AgentClient::Codex] {
            let requested = match provider {
                AgentClient::Claude => "claude",
                AgentClient::Codex => "codex",
            };
            let (mut state, dir) = pane_name_schedule_state(requested, AgentClient::Claude, None);
            std::fs::create_dir_all(&state.signal_dir).unwrap();

            state.schedule_ready_tickets();

            let lease = state.current_leases["T-NAME"].clone();
            assert!(matches!(
                state.seat_assignment(10),
                Some(SeatAssignmentState::Starting {
                    generation,
                    start_deadline: Some(_),
                    ..
                }) if generation == lease.attempt_id
            ));
            assert!(!state.seat_is_owned(10));
            assert!(dashboard_thread_row(&state, "T-NAME").contains("starting"));

            let attempt_dir = state.attempt_work_dir(&lease);
            let assignment_ref = state.assignment_refs.get("T-NAME").unwrap();
            assert_eq!(assignment_ref.lease, lease);
            assert_eq!(assignment_ref.path.parent(), Some(attempt_dir.as_path()));
            let assignment = std::fs::read_to_string(&assignment_ref.path).unwrap();
            assert!(assignment.contains("Read the ticket"));
            let launch = std::fs::read_to_string(attempt_dir.join(".lisa-launch-10.sh")).unwrap();
            assert!(!launch.contains("Read the ticket"));
            assert!(!launch.contains("LISA_ASSIGNMENT"));

            let started = dir.path().join("signals/pane-10.started");
            std::fs::write(&started, "not an attempt lease").unwrap();
            state.check_process_start_signals();
            assert!(!started.exists(), "malformed start signals are one-shot");
            assert!(!state.seat_is_owned(10));

            let stale = AttemptLease {
                ticket_id: lease.ticket_id.clone(),
                attempt_id: lease.attempt_id + 1,
            };
            std::fs::write(&started, serde_json::to_string(&stale).unwrap()).unwrap();
            state.check_process_start_signals();
            assert!(!state.seat_is_owned(10), "a stale generation fails closed");

            std::fs::write(&started, serde_json::to_string(&lease).unwrap()).unwrap();
            state.check_process_start_signals();
            assert_eq!(
                state.seat_assignment(10),
                Some(SeatAssignmentState::ReadyForAssignment {
                    generation: lease.attempt_id,
                })
            );
            assert!(!state.seat_is_owned(10));
            assert!(dashboard_thread_row(&state, "T-NAME").contains("ready-for-assignment"));

            state.deliver_ready_assignments();
            assert!(matches!(
                state.seat_assignment(10),
                Some(SeatAssignmentState::Delivering {
                    generation,
                    retries: 0,
                    ..
                }) if generation == lease.attempt_id
            ));
            assert!(!state.seat_is_owned(10));
            assert!(dashboard_thread_row(&state, "T-NAME").contains("delivering"));

            let stale_ack = serde_json::json!({
                "hook_event_name": "UserPromptSubmit",
                "prompt": codex_ack::tag_codex_assignment(
                    "stale",
                    codex_ack::CodexAssignmentRef {
                        ticket_id: "T-NAME",
                        generation: lease.attempt_id + 1,
                    },
                ),
            });
            assert!(!state.acknowledge_codex_assignment(10, &stale_ack.to_string()));
            assert!(!state.seat_is_owned(10));

            let exact_ack = serde_json::json!({
                "hook_event_name": "UserPromptSubmit",
                "prompt": codex_ack::tag_codex_assignment(
                    "read assignment",
                    codex_ack::CodexAssignmentRef {
                        ticket_id: "T-NAME",
                        generation: lease.attempt_id,
                    },
                ),
            });
            assert!(state.acknowledge_codex_assignment(10, &exact_ack.to_string()));
            assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
            assert!(state.seat_is_owned(10));
            assert!(dashboard_thread_row(&state, "T-NAME").contains("owned"));

            std::fs::write(&started, serde_json::to_string(&lease).unwrap()).unwrap();
            state.check_process_start_signals();
            assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
        }
    }

    #[test]
    fn test_missing_shell_readiness_fences_without_relaunch() {
        let (mut state, _dir) = pane_name_schedule_state("claude", AgentClient::Claude, None);
        state.config.assignment_ack_timeout_secs = 1;

        state.schedule_ready_tickets();

        let predecessor = state.current_leases["T-NAME"].clone();
        let deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::Starting {
                generation,
                start_deadline: Some(deadline),
                ..
            }) => {
                assert_eq!(generation, predecessor.attempt_id);
                deadline
            }
            other => panic!("expected an armed fresh start wait, got {other:?}"),
        };
        assert!(!state.seat_is_owned(10));
        assert!(dashboard_thread_row(&state, "T-NAME").contains("starting"));

        let launch_count = state
            .activity_log
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    ActivityEvent::SessionLaunch { ticket_id, .. } if ticket_id == "T-NAME"
                )
            })
            .count();
        assert_eq!(launch_count, 1);

        assert!(state.check_assignment_ack_timeouts_at(deadline).is_empty());

        let successor = state.current_leases["T-NAME"].clone();
        let reset_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::ResettingStartup {
                generation,
                reset_deadline,
            }) => {
                assert_eq!(generation, successor.attempt_id);
                reset_deadline
            }
            other => panic!("expected shell reset wait, got {other:?}"),
        };
        assert_eq!(successor.attempt_id, predecessor.attempt_id + 1);
        assert!(!predecessor.is_current(state.current_leases.get("T-NAME")));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-NAME"));
        assert_eq!(
            state.agent_slots[0].attempt_lease.as_ref(),
            Some(&successor)
        );
        assert_eq!(
            state.threads["T-NAME"].attempt_lease.as_ref(),
            Some(&successor)
        );
        assert_eq!(
            state.agent_slots.len(),
            1,
            "recovery must not consume a spare"
        );
        assert!(!state.seat_is_owned(10));
        assert_eq!(state.error_alerts, Vec::<(String, u32)>::new());

        assert_eq!(
            state.check_assignment_ack_timeouts_at(reset_deadline),
            vec![FailureTransitionOutcome::StartupRecoveryFailed {
                pane_id: 10,
                ticket_id: "T-NAME".to_string(),
            }]
        );

        for extra_secs in [1, 30, 300] {
            assert!(state
                .check_assignment_ack_timeouts_at(
                    reset_deadline + std::time::Duration::from_secs(extra_secs),
                )
                .is_empty());
        }

        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::StartupFailed)
        );
        assert_eq!(state.error_alerts, vec![("T-NAME".to_string(), 10)]);
        assert_eq!(state.current_leases.get("T-NAME"), None);
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Fenced
        );
        assert_eq!(
            state.threads["T-NAME"].status,
            lisa_core::types::ThreadStatus::Failed
        );
        assert_eq!(
            state.to_ui_state().seat_assignment_statuses.get(&1),
            Some(&ui::SeatAssignmentStatus::StartupFailed)
        );
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Error { message }
                if message.contains("same-pane startup recovery failed")
                    && message.contains("positive shell readiness")
                    && message.contains("pane fenced")
        )));
        assert_eq!(
            state
                .activity_log
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        ActivityEvent::SessionLaunch { ticket_id, .. } if ticket_id == "T-NAME"
                    )
                })
                .count(),
            launch_count,
            "missing shell proof cannot relaunch the provider"
        );
    }

    #[test]
    fn invalid_startup_recovery_authority_fails_once_in_named_state() {
        let (mut state, _dir) = pane_name_schedule_state("claude", AgentClient::Claude, None);
        state.config.assignment_ack_timeout_secs = 1;
        state.schedule_ready_tickets();

        let original = state.current_leases["T-NAME"].clone();
        let deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::Starting {
                generation,
                start_deadline: Some(deadline),
                relaunches: 0,
            }) => {
                assert_eq!(generation, original.attempt_id);
                deadline
            }
            other => panic!("expected initial startup wait, got {other:?}"),
        };

        state.agent_slots[0].attempt_lease = None;
        assert_eq!(
            state.check_assignment_ack_timeouts_at(deadline),
            vec![FailureTransitionOutcome::StartupFailed {
                pane_id: 10,
                ticket_id: Some("T-NAME".to_string()),
            }]
        );

        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::StartupFailed)
        );
        assert_eq!(
            state.threads["T-NAME"].status,
            lisa_core::types::ThreadStatus::Failed
        );
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-NAME"));
        assert_eq!(state.current_leases.get("T-NAME"), Some(&original));
        assert_eq!(state.lease_high_water.get("T-NAME"), Some(&original));
        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
        assert_eq!(state.error_alerts, vec![("T-NAME".to_string(), 10)]);

        assert!(state
            .check_assignment_ack_timeouts_at(deadline + std::time::Duration::from_secs(300),)
            .is_empty());
        assert_eq!(state.current_leases.get("T-NAME"), Some(&original));
        assert_eq!(state.error_alerts, vec![("T-NAME".to_string(), 10)]);
    }

    #[test]
    fn same_pane_replacement_requires_start_and_chat_ack_for_claude() {
        // SessionStart-mode (Claude) same-pane startup replacement contract: a
        // fresh Starting whose process-start signal is not observed before the
        // deadline resets in the same pane, requires a fresh shell proof to
        // relaunch, then requires the exact process-start signal to reach
        // ReadyForAssignment and the exact chat ack to reach Owned. Grace-mode
        // (Codex) diverges here — its Starting deadline paces the first prompt
        // directly into Delivering instead of recovering; that path is covered by
        // `codex_startup_grace_paces_first_prompt_into_delivering`.
        let (mut state, _dir) = pane_name_schedule_state("claude", AgentClient::Claude, None);
        state.config.assignment_ack_timeout_secs = 1;
        state.schedule_ready_tickets();
        let predecessor = state.current_leases["T-NAME"].clone();
        let first_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::Starting {
                start_deadline: Some(deadline),
                relaunches: 0,
                ..
            }) => deadline,
            other => panic!("expected initial Starting, got {other:?}"),
        };

        assert!(state
            .check_assignment_ack_timeouts_at(first_deadline)
            .is_empty());
        let successor = state.current_leases["T-NAME"].clone();
        assert_eq!(successor.attempt_id, predecessor.attempt_id + 1);
        let replacement_activity = state.agent_slots[0].last_activity_at;
        std::fs::write(
            state.signal_dir.join("pane-10.heartbeat"),
            serde_json::to_string(&predecessor).unwrap(),
        )
        .unwrap();
        state.check_heartbeat_signals();
        assert_eq!(state.agent_slots[0].last_activity_at, replacement_activity);
        assert!(state
            .admit_artifact("T-NAME", Some(&predecessor), "research.md")
            .is_err());
        assert!(!state.acknowledge_shell_ready(10, &predecessor, first_deadline));
        assert!(matches!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::ResettingStartup { generation, .. })
                if generation == successor.attempt_id
        ));

        std::fs::write(
            state.signal_dir.join("pane-10.shell-ready"),
            serde_json::to_string(&successor).unwrap(),
        )
        .unwrap();
        state.check_shell_ready_signals();
        let replacement_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::Starting {
                generation,
                start_deadline: Some(deadline),
                relaunches: 1,
            }) => {
                assert_eq!(generation, successor.attempt_id);
                deadline
            }
            other => panic!("expected replacement Starting, got {other:?}"),
        };
        assert!(replacement_deadline > first_deadline);
        assert_eq!(state.agent_slots[0].pane_id, 10);
        assert_eq!(state.agent_slots.len(), 1);
        let assignment = state.assignment_refs.get(&successor.ticket_id).unwrap();
        assert_eq!(assignment.lease, successor);
        assert!(assignment.path.is_file());
        let launch = std::fs::read_to_string(
            state
                .attempt_work_dir(&successor)
                .join(".lisa-launch-10.sh"),
        )
        .unwrap();
        assert!(!launch.contains("Read the ticket"));
        assert!(!launch.contains("LISA_ASSIGNMENT"));
        let marker: AttemptLease = serde_json::from_str(
            &std::fs::read_to_string(state.signal_dir.join("pane-10.lease")).unwrap(),
        )
        .unwrap();
        assert_eq!(marker, successor);

        assert!(!state.acknowledge_process_start(10, &predecessor));
        assert!(state.acknowledge_process_start(10, &successor));
        assert!(matches!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::ReadyForAssignment { generation })
                if generation == successor.attempt_id
        ));
        state.deliver_ready_assignments();
        assert!(matches!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::Delivering { generation, .. })
                if generation == successor.attempt_id
        ));
        assert!(!acknowledge_assignment(
            &mut state,
            10,
            "T-NAME",
            predecessor.attempt_id,
        ));
        assert!(acknowledge_assignment(
            &mut state,
            10,
            "T-NAME",
            successor.attempt_id,
        ));
        assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
    }

    #[test]
    fn missing_replacement_start_fences_without_second_relaunch() {
        let (mut state, _dir) = pane_name_schedule_state("claude", AgentClient::Claude, None);
        state.config.assignment_ack_timeout_secs = 1;
        state.schedule_ready_tickets();
        let first_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::Starting {
                start_deadline: Some(deadline),
                ..
            }) => deadline,
            other => panic!("expected initial Starting, got {other:?}"),
        };
        state.check_assignment_ack_timeouts_at(first_deadline);
        let successor = state.current_leases["T-NAME"].clone();
        assert!(state.acknowledge_shell_ready(10, &successor, first_deadline));
        let replacement_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::Starting {
                start_deadline: Some(deadline),
                relaunches: 1,
                ..
            }) => deadline,
            other => panic!("expected replacement Starting, got {other:?}"),
        };
        let launches_before_failure = state
            .activity_log
            .iter()
            .filter(|event| matches!(event, ActivityEvent::SessionLaunch { .. }))
            .count();
        assert_eq!(launches_before_failure, 2);

        assert_eq!(
            state.check_assignment_ack_timeouts_at(replacement_deadline),
            vec![FailureTransitionOutcome::StartupRecoveryFailed {
                pane_id: 10,
                ticket_id: "T-NAME".to_string(),
            }]
        );
        assert!(state
            .check_assignment_ack_timeouts_at(
                replacement_deadline + std::time::Duration::from_secs(300),
            )
            .is_empty());

        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::StartupFailed)
        );
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Fenced
        );
        assert_eq!(state.current_leases.get("T-NAME"), None);
        assert_eq!(
            state
                .activity_log
                .iter()
                .filter(|event| matches!(event, ActivityEvent::SessionLaunch { .. }))
                .count(),
            launches_before_failure,
            "replacement timeout must not submit a second recovery relaunch"
        );
    }

    #[test]
    fn test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership() {
        let (mut state, dir) = pane_name_schedule_state("claude", AgentClient::Claude, None);
        state.config.assignment_ack_timeout_secs = 1;
        std::fs::create_dir_all(&state.signal_dir).unwrap();
        state.schedule_ready_tickets();
        let lease = state.current_leases["T-NAME"].clone();
        std::fs::write(
            dir.path().join("signals/pane-10.started"),
            serde_json::to_string(&lease).unwrap(),
        )
        .unwrap();
        state.check_process_start_signals();
        state.deliver_ready_assignments();

        let first_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::Delivering {
                generation,
                ack_deadline,
                retries: 0,
            }) => {
                assert_eq!(generation, lease.attempt_id);
                ack_deadline
            }
            other => panic!("expected initial chat delivery, got {other:?}"),
        };
        let launch_count = state
            .activity_log
            .iter()
            .filter(|event| matches!(event, ActivityEvent::SessionLaunch { ticket_id, .. } if ticket_id == "T-NAME"))
            .count();

        assert!(state
            .check_assignment_ack_timeouts_at(first_deadline)
            .is_empty());
        let retry_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::Delivering {
                generation,
                ack_deadline,
                retries: 1,
            }) => {
                assert_eq!(generation, lease.attempt_id);
                ack_deadline
            }
            other => panic!("expected one bounded chat retry, got {other:?}"),
        };
        assert!(!state.seat_is_owned(10));
        assert_eq!(
            state
                .activity_log
                .iter()
                .filter(|event| matches!(event, ActivityEvent::Info { message } if message.contains("delivering assignment for T-NAME")))
                .count(),
            2,
            "one initial delivery plus exactly one retry"
        );

        assert_eq!(
            state.check_assignment_ack_timeouts_at(retry_deadline),
            vec![FailureTransitionOutcome::AssignmentDeliveryFailed {
                pane_id: 10,
                ticket_id: Some("T-NAME".to_string()),
            }]
        );
        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::DeliveryFailed)
        );
        assert!(!state.seat_is_owned(10));
        assert_eq!(
            state.threads["T-NAME"].status,
            lisa_core::types::ThreadStatus::Failed
        );
        assert_eq!(state.agent_slots[0].attempt_lease.as_ref(), Some(&lease));
        assert_eq!(state.current_leases.get("T-NAME"), Some(&lease));
        assert_eq!(
            state.to_ui_state().seat_assignment_statuses.get(&1),
            Some(&ui::SeatAssignmentStatus::DeliveryFailed)
        );

        let late = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "late",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-NAME",
                    generation: lease.attempt_id,
                },
            ),
        });
        assert!(!state.acknowledge_codex_assignment(10, &late.to_string()));

        assert!(state
            .check_assignment_ack_timeouts_at(retry_deadline + std::time::Duration::from_secs(300),)
            .is_empty());
        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::DeliveryFailed)
        );
        assert_eq!(
            state
                .activity_log
                .iter()
                .filter(|event| matches!(event, ActivityEvent::SessionLaunch { ticket_id, .. } if ticket_id == "T-NAME"))
                .count(),
            launch_count,
            "chat recovery never relaunches the started provider"
        );
    }

    #[test]
    fn retained_failure_helpers_return_path_specific_outcomes() {
        use lisa_core::types::Thread;

        fn reserved_state(seat: SeatAssignmentState) -> State {
            let mut state = State::default();
            state.agent_slots.push(AgentSlot {
                pane_id: 10,
                ticket_id: Some("T-NAME".to_string()),
                attempt_lease: None,
                has_session: true,
                transition_state: TransitionState::Idle,
                transition_started_at: None,
                cooldown_until: None,
                last_activity_at: None,
                last_client: Some(AgentClient::Claude),
            });
            state
                .threads
                .insert("T-NAME".to_string(), Thread::new("T-NAME", 10));
            state.seat_assignments.insert(10, seat);
            state
        }

        let deadline = std::time::SystemTime::now();
        let mut delivery = reserved_state(SeatAssignmentState::Delivering {
            generation: 1,
            ack_deadline: deadline,
            retries: MAX_ASSIGNMENT_DELIVERY_RETRIES,
        });
        assert_eq!(
            delivery.fail_assignment_delivery(10, "test"),
            Some(FailureTransitionOutcome::AssignmentDeliveryFailed {
                pane_id: 10,
                ticket_id: Some("T-NAME".to_string()),
            })
        );

        let mut recovery = reserved_state(SeatAssignmentState::Recovering {
            generation: 2,
            ack_deadline: Some(deadline),
        });
        assert_eq!(
            recovery.fail_assignment_recovery(10, "test"),
            Some(FailureTransitionOutcome::AssignmentRecoveryFailed {
                pane_id: 10,
                ticket_id: Some("T-NAME".to_string()),
            })
        );

        let mut startup = reserved_state(SeatAssignmentState::Starting {
            generation: 1,
            start_deadline: Some(deadline),
            relaunches: 0,
        });
        assert_eq!(
            startup.fail_startup(10, "test"),
            Some(FailureTransitionOutcome::StartupFailed {
                pane_id: 10,
                ticket_id: Some("T-NAME".to_string()),
            })
        );

        let mut startup_recovery = reserved_state(SeatAssignmentState::ResettingStartup {
            generation: 2,
            reset_deadline: deadline,
        });
        assert_eq!(
            startup_recovery.fail_startup_recovery(10, "test"),
            Some(FailureTransitionOutcome::StartupRecoveryFailed {
                pane_id: 10,
                ticket_id: "T-NAME".to_string(),
            })
        );
    }

    #[test]
    fn assignment_recovery_failure_retains_authority_for_operator_reset() {
        let (mut state, dir) =
            pane_name_schedule_state("codex", AgentClient::Codex, Some(AgentClient::Codex));
        state.schedule_ready_tickets();

        let predecessor = state.current_leases["T-NAME"].clone();
        state.seat_assignments.insert(
            10,
            SeatAssignmentState::AssignedPendingAck {
                generation: predecessor.attempt_id,
                ack_deadline: Some(std::time::SystemTime::now()),
            },
        );

        let recovery_started = std::time::SystemTime::now();
        state.begin_assignment_recovery(10, recovery_started);
        let successor = state.current_leases["T-NAME"].clone();
        assert_eq!(successor.attempt_id, predecessor.attempt_id + 1);
        assert_eq!(state.lease_high_water.get("T-NAME"), Some(&successor));
        assert_eq!(
            state.agent_slots[0].attempt_lease.as_ref(),
            Some(&successor)
        );
        assert_eq!(
            state.threads["T-NAME"].attempt_lease.as_ref(),
            Some(&successor)
        );
        assert!(matches!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::Recovering {
                generation,
                ack_deadline: None,
            }) if generation == successor.attempt_id
        ));

        let recovery_deadline = recovery_started + std::time::Duration::from_secs(1);
        state.seat_assignments.insert(
            10,
            SeatAssignmentState::Recovering {
                generation: successor.attempt_id,
                ack_deadline: Some(recovery_deadline),
            },
        );
        state.ledger_path = dir.path().join("provenance.jsonl");
        assert_eq!(
            state.check_assignment_ack_timeouts_at(recovery_deadline),
            vec![FailureTransitionOutcome::AssignmentRecoveryFailed {
                pane_id: 10,
                ticket_id: Some("T-NAME".to_string()),
            }]
        );

        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::RecoveryFailed)
        );
        assert_eq!(
            state.threads["T-NAME"].status,
            lisa_core::types::ThreadStatus::Failed
        );
        assert_eq!(state.current_leases.get("T-NAME"), Some(&successor));
        assert_eq!(state.lease_high_water.get("T-NAME"), Some(&successor));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-NAME"));
        assert_eq!(
            state.agent_slots[0].attempt_lease.as_ref(),
            Some(&successor)
        );
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForExit
        );
        assert!(!state.agent_slots[0].has_session);
        assert!(state.threads.contains_key("T-NAME"));
        let records = read_mixed_ledger(&state.ledger_path);
        assert_eq!(records.len(), 1, "one retained recovery failure row");
        assert!(matches!(
            &records[0],
            lisa_core::provenance::ProvenanceLedgerRecord::AssignmentTransition(record)
                if record.ticket_id == "T-NAME"
                    && record.attempt_lease == successor
                    && record.pane_id == 10
                    && record.provider == "openai"
                    && record.state == lisa_core::provenance::AssignmentState::RecoveryFailed
                    && record.reason == "fresh Codex session did not acknowledge before the deadline"
        ));
        assert_eq!(state.error_alerts, vec![("T-NAME".to_string(), 10)]);

        assert!(state
            .check_assignment_ack_timeouts_at(
                recovery_deadline + std::time::Duration::from_secs(300),
            )
            .is_empty());
        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::RecoveryFailed),
            "terminal recovery failure cannot start another automatic attempt"
        );
        assert_eq!(state.current_leases.get("T-NAME"), Some(&successor));
        assert_eq!(state.error_alerts, vec![("T-NAME".to_string(), 10)]);
        assert_eq!(
            read_mixed_ledger(&state.ledger_path).len(),
            1,
            "terminal recovery state cannot append duplicate evidence"
        );
    }

    #[test]
    fn test_dashboard_snapshot_shows_fresh_codex_handoff_states() {
        let (mut acknowledged, _dir) =
            pane_name_schedule_state("codex", AgentClient::Claude, Some(AgentClient::Codex));
        acknowledged.schedule_ready_tickets();
        let starting_row = dashboard_thread_row(&acknowledged, "T-NAME");
        let lease = acknowledged.current_leases["T-NAME"].clone();
        exit_then_deliver_fresh_codex(&mut acknowledged, 10, &lease);
        let delivering_row = dashboard_thread_row(&acknowledged, "T-NAME");

        let matching = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "new work",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-NAME",
                    generation: 1,
                },
            ),
        });
        assert!(acknowledged.acknowledge_codex_assignment(10, &matching.to_string()));
        let owned_row = dashboard_thread_row(&acknowledged, "T-NAME");

        let snapshot =
            format!("starting\n{starting_row}\ndelivering\n{delivering_row}\nowned\n{owned_row}");
        assert_eq!(
            snapshot,
            "starting\n\
[1]    T-NAME       RES        codex          starting             <elapsed>\n\
delivering\n\
[1]    T-NAME       RES        codex          delivering           <elapsed>\n\
owned\n\
[1]    T-NAME       RES        codex          owned                <elapsed>"
        );
    }

    #[test]
    fn delivered_assignment_becomes_owned_on_exact_claim_without_hook() {
        let (mut state, _dir) =
            pane_name_schedule_state("codex", AgentClient::Claude, Some(AgentClient::Codex));
        std::fs::create_dir_all(&state.signal_dir).unwrap();
        state.schedule_ready_tickets();

        let lease = state.current_leases["T-NAME"].clone();
        exit_then_deliver_fresh_codex(&mut state, 10, &lease);
        let nonce = state.assignment_refs["T-NAME"].nonce;

        assert!(matches!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::Delivering {
                generation,
                retries: 0,
                ..
            }) if generation == lease.attempt_id
        ));
        assert!(!state.seat_is_owned(10));
        assert!(dashboard_thread_row(&state, "T-NAME").contains("delivering"));
        assert!(!state.signal_dir.join("pane-10.ack").exists());

        let claim_path = state.signal_dir.join("pane-10.claim");
        let wrong_nonce = AssignmentClaim {
            ticket_id: lease.ticket_id.clone(),
            attempt_id: lease.attempt_id,
            nonce: nonce + 1,
        };
        std::fs::write(&claim_path, serde_json::to_string(&wrong_nonce).unwrap()).unwrap();
        state.check_claim_signals();
        assert!(!claim_path.exists());
        assert!(!state.seat_is_owned(10));
        assert!(dashboard_thread_row(&state, "T-NAME").contains("delivering"));

        let delivery_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::Delivering { ack_deadline, .. }) => ack_deadline,
            other => panic!("expected delivered assignment, got {other:?}"),
        };
        state.check_assignment_ack_timeouts_at(delivery_deadline);
        assert!(matches!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::DeliveredAwaitingClaim { generation, .. })
                if generation == lease.attempt_id
        ));
        assert!(dashboard_thread_row(&state, "T-NAME").contains("delivered-awaiting-claim"));

        let exact = AssignmentClaim {
            nonce,
            ..wrong_nonce
        };
        std::fs::write(&claim_path, serde_json::to_string(&exact).unwrap()).unwrap();
        state.check_claim_signals();

        assert!(!claim_path.exists());
        assert!(!state.signal_dir.join("pane-10.ack").exists());
        assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
        assert!(state.seat_is_owned(10));
        assert!(dashboard_thread_row(&state, "T-NAME").contains("owned"));
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Info { message }
                if message.contains("claimed T-NAME attempt 1 assignment")
        )));
    }

    #[test]
    fn matching_hook_accelerates_pending_claim_ownership() {
        let (mut state, _dir) =
            pane_name_schedule_state("codex", AgentClient::Claude, Some(AgentClient::Codex));
        std::fs::create_dir_all(&state.signal_dir).unwrap();
        state.schedule_ready_tickets();

        let lease = state.current_leases["T-NAME"].clone();
        exit_then_deliver_fresh_codex(&mut state, 10, &lease);
        assert!(matches!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::Delivering {
                generation,
                retries: 0,
                ..
            }) if generation == lease.attempt_id
        ));
        assert!(!state.seat_is_owned(10));
        assert!(!state.signal_dir.join("pane-10.claim").exists());

        let matching = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "supplemental ownership evidence",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-NAME",
                    generation: lease.attempt_id,
                },
            ),
        });
        let hook_path = state.signal_dir.join("pane-10.ack");
        std::fs::write(&hook_path, matching.to_string()).unwrap();
        state.check_codex_ack_signals();

        assert!(!hook_path.exists());
        assert!(!state.signal_dir.join("pane-10.claim").exists());
        assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Info { message }
                if message.contains("Pane 10 acknowledged its assignment")
        )));
    }

    #[test]
    fn current_artifact_is_bounded_fallback_and_stale_evidence_is_ignored() {
        let (mut state, _dir) =
            pane_name_schedule_state("codex", AgentClient::Claude, Some(AgentClient::Codex));
        std::fs::create_dir_all(&state.signal_dir).unwrap();
        state.schedule_ready_tickets();

        let predecessor = state.current_leases["T-NAME"].clone();
        exit_then_deliver_fresh_codex(&mut state, 10, &predecessor);
        state.release_slot_for_ticket(&"T-NAME".to_string());
        state.threads.remove("T-NAME");
        state.agent_slots[0].cooldown_until =
            Some(std::time::SystemTime::now() - std::time::Duration::from_secs(1));
        state.agent_slots[0].last_activity_at =
            Some(std::time::SystemTime::now() - std::time::Duration::from_secs(1));
        state.schedule_ready_tickets();

        let replacement = state.current_leases["T-NAME"].clone();
        assert_eq!(replacement.attempt_id, predecessor.attempt_id + 1);
        exit_then_deliver_fresh_codex(&mut state, 10, &replacement);
        state.threads.get_mut("T-NAME").unwrap().current_phase = Phase::Research;
        let old_activity = std::time::SystemTime::UNIX_EPOCH;
        state.threads.get_mut("T-NAME").unwrap().last_activity = old_activity;
        state.agent_slots[0].last_activity_at = Some(old_activity);

        let stale_hook = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "predecessor ownership evidence",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-NAME",
                    generation: predecessor.attempt_id,
                },
            ),
        });
        let hook_path = state.signal_dir.join("pane-10.ack");
        std::fs::write(&hook_path, stale_hook.to_string()).unwrap();
        let predecessor_stage = state.attempt_work_dir(&predecessor);
        std::fs::create_dir_all(&predecessor_stage).unwrap();
        std::fs::write(
            predecessor_stage.join("research.md"),
            "predecessor output must remain private\n",
        )
        .unwrap();

        state.check_codex_ack_signals();
        state.check_artifact_advances();

        assert!(!hook_path.exists(), "stale hook evidence remains one-shot");
        assert!(matches!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::Delivering { generation, .. })
                if generation == replacement.attempt_id
        ));
        assert_eq!(state.threads["T-NAME"].last_activity, old_activity);
        assert_eq!(state.agent_slots[0].last_activity_at, Some(old_activity));
        assert_eq!(state.threads["T-NAME"].current_phase, Phase::Research);
        assert!(!state.config.work_dir.join("T-NAME/research.md").exists());
        assert!(
            state
                .admit_artifact("T-NAME", Some(&predecessor), "research.md")
                .is_err(),
            "a predecessor artifact cannot cross the current lease boundary"
        );
        assert_eq!(
            std::fs::read_to_string(predecessor_stage.join("research.md")).unwrap(),
            "predecessor output must remain private\n"
        );

        let replacement_stage = state.attempt_work_dir(&replacement);
        std::fs::create_dir_all(&replacement_stage).unwrap();
        std::fs::write(
            replacement_stage.join("research.md"),
            "replacement output is current\n",
        )
        .unwrap();
        state.check_artifact_advances();

        assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
        assert_eq!(state.threads["T-NAME"].current_phase, Phase::Design);
        assert!(state.threads["T-NAME"].last_activity > old_activity);
        assert!(state.agent_slots[0].last_activity_at.unwrap() > old_activity);
        assert_eq!(
            std::fs::read_to_string(state.config.work_dir.join("T-NAME/research.md")).unwrap(),
            "replacement output is current\n"
        );
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Info { message }
                if message.contains("Pane 10 established ownership of T-NAME attempt 2")
                    && message.contains("current-attempt research.md")
        )));
    }

    #[test]
    fn test_recycled_codex_ownership_requires_matching_ack_exactly_once() {
        let (mut state, _dir) =
            pane_name_schedule_state("codex", AgentClient::Claude, Some(AgentClient::Codex));
        state
            .last_pane_names
            .insert(10, "codex · T-OLD · old work".to_string());

        state.schedule_ready_tickets();

        let first = state.current_leases["T-NAME"].clone();

        assert_eq!(
            state.last_pane_names.get(&10).map(String::as_str),
            Some("codex · T-NAME · pane lifecycle")
        );
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForExit
        );
        assert!(matches!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::Starting {
                generation,
                start_deadline: None,
                ..
            }) if generation == first.attempt_id
        ));
        assert_eq!(first.attempt_id, 1);
        assert_eq!(state.agent_slots[0].attempt_lease.as_ref(), Some(&first));
        assert!(
            !state.seat_is_owned(10),
            "ticket reservation must not imply acknowledged Codex ownership"
        );

        let stale_ticket = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "old work",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-OLD",
                    generation: first.attempt_id,
                },
            ),
        });
        assert!(!state.acknowledge_codex_assignment(10, &stale_ticket.to_string()));
        assert!(!state.seat_is_owned(10));

        // Abandon the first unacknowledged delivery and redispatch the same
        // ticket onto the resident Codex seat. The replacement gets a strictly
        // newer lease, and its marker generation is sourced from that lease.
        exit_then_deliver_fresh_codex(&mut state, 10, &first);
        state.release_slot_for_ticket(&"T-NAME".to_string());
        state.threads.remove("T-NAME");
        state.agent_slots[0].cooldown_until =
            Some(std::time::SystemTime::now() - std::time::Duration::from_secs(1));
        state.agent_slots[0].last_activity_at =
            Some(std::time::SystemTime::now() - std::time::Duration::from_secs(1));
        state.schedule_ready_tickets();

        let second = state.current_leases["T-NAME"].clone();
        assert_eq!(second.attempt_id, first.attempt_id + 1);
        assert!(!first.is_current(Some(&second)));
        assert!(second.is_current(state.current_leases.get("T-NAME")));
        assert_eq!(state.agent_slots[0].attempt_lease.as_ref(), Some(&second));
        exit_then_deliver_fresh_codex(&mut state, 10, &second);
        assert!(matches!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::Delivering {
                generation,
                retries: 0,
                ..
            }) if generation == second.attempt_id
        ));

        let prior_lease_ack = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "prior attempt work",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-NAME",
                    generation: first.attempt_id,
                },
            ),
        });
        assert!(!state.acknowledge_codex_assignment(10, &prior_lease_ack.to_string()));
        assert!(
            matches!(
                state.seat_assignment(10),
                Some(SeatAssignmentState::Delivering {
                    generation,
                    retries: 0,
                    ..
                }) if generation == second.attempt_id
            ),
            "an acknowledgement carrying the prior lease cannot promote the replacement"
        );
        assert!(!state.seat_is_owned(10));

        let matching = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "new work",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-NAME",
                    generation: second.attempt_id,
                },
            ),
        });
        let removed = state.current_leases.remove("T-NAME").unwrap();
        assert!(!state.acknowledge_codex_assignment(10, &matching.to_string()));
        assert!(!state.seat_is_owned(10), "revoked authority fails closed");
        state.current_leases.insert("T-NAME".to_string(), removed);
        assert!(state.acknowledge_codex_assignment(10, &matching.to_string()));
        assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
        assert!(state.seat_is_owned(10));
        assert!(
            !state.acknowledge_codex_assignment(10, &matching.to_string()),
            "duplicate acknowledgment cannot perform a second transition"
        );
        assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
    }

    #[test]
    fn test_dropped_post_prompt_ack_reproduces_open_loop_stall_and_recovers_boundedly() {
        let (mut state, dir) =
            pane_name_schedule_state("codex", AgentClient::Claude, Some(AgentClient::Codex));
        state.config.assignment_ack_timeout_secs = 1;
        state.signal_dir = dir.path().join("signals");
        std::fs::create_dir_all(&state.signal_dir).unwrap();

        state.schedule_ready_tickets();
        let original_lease = state.current_leases["T-NAME"].clone();
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForExit,
            "Codex must exit before any next-ticket prompt is attempted"
        );
        let first_deadline = exit_then_deliver_fresh_codex(&mut state, 10, &original_lease);

        // Reproduce the field failure at its real transport seam: Codex accepted
        // the tagged prompt, but the pane-scoped event vanished before Lisa's
        // scanner could consume it.
        let acceptance = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "assigned work",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-NAME",
                    generation: 1,
                },
            ),
        });
        let ack_path = state.signal_dir.join("pane-10.ack");
        std::fs::write(&ack_path, acceptance.to_string()).unwrap();
        assert!(
            ack_path.exists(),
            "matching acceptance event was materialized"
        );
        std::fs::remove_file(&ack_path).unwrap();
        state.check_codex_ack_signals();

        assert!(
            matches!(
                state.seat_assignment(10),
                Some(SeatAssignmentState::Delivering {
                    generation: 1,
                    ack_deadline,
                    retries: 0,
                }) if ack_deadline == first_deadline
            ),
            "a dropped event cannot promote the acknowledgment-gated seat"
        );
        assert!(!state.seat_is_owned(10));
        assert!(!state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Info { message }
                if message.contains("acknowledged its assignment")
        )));

        state.check_assignment_ack_timeouts_at(first_deadline);
        let claim_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::DeliveredAwaitingClaim { claim_deadline, .. }) => {
                claim_deadline
            }
            other => panic!("expected passive delivered claim wait, got {other:?}"),
        };
        state.check_assignment_ack_timeouts_at(claim_deadline);
        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::ClaimTimedOut)
        );
        assert!(!state.seat_is_owned(10));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-NAME"));
        assert_eq!(
            state.threads["T-NAME"].status,
            lisa_core::types::ThreadStatus::Failed
        );
        assert!(state.error_alerts.contains(&("T-NAME".to_string(), 10)));
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Error { message }
                if message.contains("delivered assignment was not claimed") && message.contains("reset the ticket")
        )));

        state.check_assignment_ack_timeouts_at(claim_deadline + std::time::Duration::from_secs(60));
        assert_eq!(
            state.current_leases.get("T-NAME"),
            Some(&original_lease),
            "passive claim wait does not mint duplicate attempts or processes"
        );
    }

    #[test]
    fn live_codex_slow_claim_waits_without_reinjection_then_times_out_actionably() {
        let (mut state, dir) =
            pane_name_schedule_state("codex", AgentClient::Claude, Some(AgentClient::Codex));
        state.config.assignment_ack_timeout_secs = 1;
        state.ledger_path = dir.path().join("provenance.jsonl");

        state.schedule_ready_tickets();
        let lease = state.current_leases["T-NAME"].clone();
        let first_deadline = exit_then_deliver_fresh_codex(&mut state, 10, &lease);
        assert!(!state.seat_is_owned(10));
        assert!(state.agent_slots[0].has_session);
        assert_eq!(state.agent_slots[0].last_client, Some(AgentClient::Codex));
        assert!(!state.signal_dir.join("pane-10.claim").exists());
        assert!(!state.signal_dir.join("pane-10.ack").exists());

        let delivery_logs = delivery_log_count(&state, "T-NAME");
        let pending_enters = state.pending_enters.len();
        let launches = state
            .activity_log
            .iter()
            .filter(|event| matches!(event, ActivityEvent::SessionLaunch { ticket_id, .. } if ticket_id == "T-NAME"))
            .count();

        assert!(state
            .check_assignment_ack_timeouts_at(first_deadline)
            .is_empty());
        let claim_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::DeliveredAwaitingClaim {
                generation,
                claim_deadline,
            }) if generation == lease.attempt_id => claim_deadline,
            other => panic!("expected delivered-awaiting-claim, got {other:?}"),
        };
        assert!(claim_deadline > first_deadline);
        assert_eq!(delivery_log_count(&state, "T-NAME"), delivery_logs);
        assert_eq!(state.pending_enters.len(), pending_enters);
        assert_eq!(
            state
                .activity_log
                .iter()
                .filter(|event| matches!(event, ActivityEvent::SessionLaunch { ticket_id, .. } if ticket_id == "T-NAME"))
                .count(),
            launches
        );
        assert_eq!(state.current_leases.get("T-NAME"), Some(&lease));
        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
        assert!(!state.seat_is_owned(10));
        assert_eq!(
            state.to_ui_state().seat_assignment_statuses.get(&1),
            Some(&ui::SeatAssignmentStatus::DeliveredAwaitingClaim)
        );

        assert_eq!(
            state.check_assignment_ack_timeouts_at(claim_deadline),
            vec![FailureTransitionOutcome::AssignmentClaimTimedOut {
                pane_id: 10,
                ticket_id: Some("T-NAME".to_string()),
            }]
        );
        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::ClaimTimedOut)
        );
        assert!(!state.seat_is_owned(10));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-NAME"));
        assert_eq!(state.agent_slots[0].attempt_lease.as_ref(), Some(&lease));
        assert_eq!(
            state.threads["T-NAME"].status,
            lisa_core::types::ThreadStatus::Failed
        );
        assert!(state.error_alerts.contains(&("T-NAME".to_string(), 10)));
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Error { message }
                if message.contains("delivered assignment was not claimed")
                    && message.contains("inspect the pane")
                    && message.contains("reset the ticket")
        )));
        assert!(!state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Error { message } if message.contains("assignment delivery failed")
        )));
        assert_eq!(delivery_log_count(&state, "T-NAME"), delivery_logs);
        assert_eq!(state.pending_enters.len(), pending_enters);

        let records = read_mixed_ledger(&state.ledger_path);
        assert!(matches!(
            records.as_slice(),
            [lisa_core::provenance::ProvenanceLedgerRecord::AssignmentTransition(record)]
                if record.ticket_id == "T-NAME"
                    && record.attempt_lease == lease
                    && record.state == AssignmentState::ClaimTimedOut
                    && record.reason == "delivered Codex assignment was not claimed before the bounded deadline"
        ));

        assert!(state
            .check_assignment_ack_timeouts_at(claim_deadline + std::time::Duration::from_secs(60),)
            .is_empty());
        assert_eq!(state.current_leases.get("T-NAME"), Some(&lease));
        assert_eq!(read_mixed_ledger(&state.ledger_path).len(), 1);
    }

    #[test]
    fn passive_claim_wait_promotes_only_the_current_fresh_generation() {
        let (mut state, _dir) =
            pane_name_schedule_state("codex", AgentClient::Claude, Some(AgentClient::Codex));
        state.config.assignment_ack_timeout_secs = 1;
        state.schedule_ready_tickets();
        let lease = state.current_leases["T-NAME"].clone();
        let first_deadline = exit_then_deliver_fresh_codex(&mut state, 10, &lease);
        state.check_assignment_ack_timeouts_at(first_deadline);
        let claim_deadline = match state.seat_assignment(10) {
            Some(SeatAssignmentState::DeliveredAwaitingClaim {
                generation: 1,
                claim_deadline,
            }) => claim_deadline,
            other => panic!("expected current-generation claim wait, got {other:?}"),
        };

        let stale = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "wrong generation",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-NAME",
                    generation: 2,
                },
            ),
        });
        assert!(!state.acknowledge_codex_assignment(10, &stale.to_string()));

        let matching = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "fresh retry",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-NAME",
                    generation: 1,
                },
            ),
        });
        assert!(state.acknowledge_codex_assignment(10, &matching.to_string()));
        assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
        assert!(state.seat_is_owned(10));

        state.check_assignment_ack_timeouts_at(claim_deadline + std::time::Duration::from_secs(1));
        assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
        assert!(state.error_alerts.is_empty());
    }

    #[test]
    fn test_reused_claude_assignment_remains_owned() {
        let (mut state, _dir) =
            pane_name_schedule_state("claude", AgentClient::Codex, Some(AgentClient::Claude));

        state.schedule_ready_tickets();

        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForClear,
            "Claude keeps its existing clear handshake"
        );
        assert_eq!(state.seat_assignment(10), Some(SeatAssignmentState::Owned));
        assert!(state.seat_is_owned(10));
    }

    #[test]
    fn test_consecutive_reused_panes_resolve_codex_ack_or_fallback_and_preserve_claude() {
        let (mut codex, _codex_dir) =
            consecutive_reuse_state(AgentClient::Codex, "T-CODEX", &[10, 11]);
        let mut codex_tickets = std::collections::HashSet::new();
        let mut codex_panes = std::collections::HashSet::new();
        let mut ack_then_owned = 0usize;
        let mut timeout_then_claim = 0usize;

        for _round in 0..5 {
            codex.schedule_ready_tickets();
            let active = active_ticket_panes(&codex);
            assert_eq!(active.len(), 2, "each round must reuse both Codex panes");

            for (ticket_id, pane_id) in &active {
                let sequence: usize = ticket_id.rsplit('-').next().unwrap().parse().unwrap();
                assert_eq!(
                    codex
                        .agent_slots
                        .iter()
                        .find(|slot| slot.pane_id == *pane_id)
                        .map(|slot| slot.transition_state),
                    Some(TransitionState::WaitingForExit),
                    "native Codex exits the resident TUI before fresh delivery"
                );
                assert!(matches!(
                    codex.seat_assignment(*pane_id),
                    Some(SeatAssignmentState::Starting {
                        start_deadline: None,
                        ..
                    })
                ));
                let lease = codex.current_leases[ticket_id].clone();
                let original_generation = lease.attempt_id;
                let original_deadline = exit_then_deliver_fresh_codex(&mut codex, *pane_id, &lease);
                assert!(!codex.seat_is_owned(*pane_id));

                let (outcome, fallback_launches) = if sequence == 6 {
                    // Deterministically lose the first submitted acceptance. The
                    // exact finite deadline enters passive claim wait in the
                    // same fresh process, never another send, process, or attempt.
                    codex.check_assignment_ack_timeouts_at(original_deadline);
                    let claim_deadline = match codex.seat_assignment(*pane_id) {
                        Some(SeatAssignmentState::DeliveredAwaitingClaim {
                            generation,
                            claim_deadline,
                        }) if generation == original_generation => claim_deadline,
                        other => {
                            panic!("{ticket_id} must enter passive claim wait, got {other:?}")
                        }
                    };
                    assert!(!codex.seat_is_owned(*pane_id));
                    assert!(acknowledge_assignment(
                        &mut codex,
                        *pane_id,
                        ticket_id,
                        original_generation,
                    ));
                    codex.check_assignment_ack_timeouts_at(claim_deadline);
                    timeout_then_claim += 1;
                    ("timeout-then-claim", 0)
                } else {
                    assert!(acknowledge_assignment(
                        &mut codex,
                        *pane_id,
                        ticket_id,
                        original_generation,
                    ));
                    ack_then_owned += 1;
                    ("ack-then-owned", 0)
                };

                assert_eq!(
                    codex.seat_assignment(*pane_id),
                    Some(SeatAssignmentState::Owned)
                );
                assert!(codex.seat_is_owned(*pane_id));
                assert!(codex_tickets.insert(ticket_id.clone()));
                codex_panes.insert(*pane_id);
                println!(
                    "T0330302|assignment|provider=codex|sequence={sequence:02}|ticket={ticket_id}|pane={pane_id}|generation={original_generation}|outcome={outcome}|fallback_launches={fallback_launches}|final=owned|silent_stall=false"
                );
            }

            for (ticket_id, pane_id) in active {
                codex.threads.get_mut(&ticket_id).unwrap().complete();
                let ticket_path = codex.dag.get_ticket(&ticket_id).unwrap().file_path.clone();
                ticket::update_ticket_phase(&ticket_path, Phase::Done).unwrap();
                codex.release_slot_for_ticket(&ticket_id);
                let slot = codex
                    .agent_slots
                    .iter()
                    .find(|slot| slot.pane_id == pane_id)
                    .unwrap();
                assert_eq!(slot.ticket_id, None);
                assert!(slot.has_session);
                assert_eq!(slot.last_client, Some(AgentClient::Codex));
                assert_eq!(codex.seat_assignment(pane_id), None);
            }
            refresh_fixture_dag(&mut codex);
        }

        assert_eq!(codex_tickets.len(), 10);
        assert_eq!(codex_panes, std::collections::HashSet::from([10, 11]));
        assert_eq!(ack_then_owned, 9);
        assert_eq!(timeout_then_claim, 1);
        assert!(codex.error_alerts.is_empty());

        let (mut claude, _claude_dir) =
            consecutive_reuse_state(AgentClient::Claude, "T-CLAUDE", &[20, 21]);
        let mut claude_tickets = std::collections::HashSet::new();
        let mut claude_panes = std::collections::HashSet::new();

        for _round in 0..5 {
            claude.schedule_ready_tickets();
            let active = active_ticket_panes(&claude);
            assert_eq!(active.len(), 2, "each round must reuse both Claude panes");

            for (ticket_id, pane_id) in &active {
                let sequence: usize = ticket_id.rsplit('-').next().unwrap().parse().unwrap();
                assert_eq!(
                    claude
                        .agent_slots
                        .iter()
                        .find(|slot| slot.pane_id == *pane_id)
                        .map(|slot| slot.transition_state),
                    Some(TransitionState::WaitingForClear),
                    "Claude must retain its clear handshake"
                );
                assert_eq!(
                    claude.seat_assignment(*pane_id),
                    Some(SeatAssignmentState::Owned)
                );
                assert!(claude.seat_is_owned(*pane_id));
                assert_eq!(claude.active_assignment_generation(*pane_id), None);

                claude.handle_cleared_signal(*pane_id);
                assert_eq!(
                    claude
                        .agent_slots
                        .iter()
                        .find(|slot| slot.pane_id == *pane_id)
                        .map(|slot| slot.transition_state),
                    Some(TransitionState::Idle)
                );
                assert_eq!(
                    claude.seat_assignment(*pane_id),
                    Some(SeatAssignmentState::Owned)
                );
                assert!(claude.seat_is_owned(*pane_id));
                assert!(claude_tickets.insert(ticket_id.clone()));
                claude_panes.insert(*pane_id);
                println!(
                    "T0330302|assignment|provider=claude|sequence={sequence:02}|ticket={ticket_id}|pane={pane_id}|generation=none|outcome=clear-then-owned-unchanged|fallback_launches=0|final=owned|silent_stall=false"
                );
            }

            for (ticket_id, pane_id) in active {
                claude.threads.get_mut(&ticket_id).unwrap().complete();
                let ticket_path = claude.dag.get_ticket(&ticket_id).unwrap().file_path.clone();
                ticket::update_ticket_phase(&ticket_path, Phase::Done).unwrap();
                claude.release_slot_for_ticket(&ticket_id);
                let slot = claude
                    .agent_slots
                    .iter()
                    .find(|slot| slot.pane_id == pane_id)
                    .unwrap();
                assert_eq!(slot.ticket_id, None);
                assert!(slot.has_session);
                assert_eq!(slot.last_client, Some(AgentClient::Claude));
                assert_eq!(claude.seat_assignment(pane_id), None);
            }
            refresh_fixture_dag(&mut claude);
        }

        assert_eq!(claude_tickets.len(), 10);
        assert_eq!(claude_panes, std::collections::HashSet::from([20, 21]));
        assert!(claude.error_alerts.is_empty());
        println!(
            "T0330302|summary|codex=10|ack_then_owned={ack_then_owned}|timeout_then_claim={timeout_then_claim}|claude=10|silent_stalls=0"
        );
    }

    #[test]
    fn test_pane_title_cross_provider_switch_uses_incoming_provider() {
        let (mut state, _dir) =
            pane_name_schedule_state("codex", AgentClient::Claude, Some(AgentClient::Claude));
        state
            .last_pane_names
            .insert(10, "claude · idle".to_string());

        state.schedule_ready_tickets();

        assert_eq!(
            state.last_pane_names.get(&10).map(String::as_str),
            Some("codex · T-NAME · pane lifecycle")
        );
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForExit
        );
        assert_eq!(state.agent_slots[0].last_client, Some(AgentClient::Codex));
        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::Starting {
                generation: 1,
                start_deadline: None,
                relaunches: 0,
            }),
            "cross-provider recycling launches a fresh process after exit"
        );
        assert!(!state.seat_is_owned(10));
    }

    #[test]
    fn test_pane_title_release_reflects_resident_or_empty_slot() {
        let mut state = State::default();
        let mut codex = fresh_slot(10, Some(AgentClient::Codex));
        codex.ticket_id = Some("T-CODEX".to_string());
        let mut shell = fresh_slot(11, None);
        shell.ticket_id = Some("T-SHELL".to_string());
        state.agent_slots.extend([codex, shell]);
        state.seat_assignments.insert(
            10,
            SeatAssignmentState::AssignedPendingAck {
                generation: 1,
                ack_deadline: None,
            },
        );
        state
            .seat_assignments
            .insert(11, SeatAssignmentState::Owned);

        state.release_slot_for_ticket(&"T-CODEX".to_string());
        state.release_slot_for_ticket(&"T-SHELL".to_string());

        assert_eq!(state.seat_assignment(10), None);
        assert_eq!(state.seat_assignment(11), None);
        assert!(!state.seat_is_owned(10));
        assert!(!state.seat_is_owned(11));

        assert_eq!(
            state.last_pane_names.get(&10).map(String::as_str),
            Some("codex · idle")
        );
        assert_eq!(
            state.last_pane_names.get(&11).map(String::as_str),
            Some("lisa · idle")
        );
    }

    #[test]
    fn test_provider_under_cap_no_cap_always_admits() {
        // No provider_caps configured → per-provider gate never blocks, even with
        // many running threads of that provider (only the global cap applies).
        let mut state = State::default();
        for i in 0..5u32 {
            state.threads.insert(
                format!("C-{i}"),
                running_thread(&format!("C-{i}"), 10 + i, AgentClient::Codex),
            );
        }
        assert!(state.provider_under_cap(AgentClient::Codex));
        assert!(state.provider_under_cap(AgentClient::Claude));
    }

    #[test]
    fn test_provider_under_cap_blocks_one_provider_not_other() {
        let mut state = State {
            config: PluginConfig {
                provider_caps: [(AgentClient::Codex, 2)].into_iter().collect(),
                ..PluginConfig::new()
            },
            ..State::default()
        };
        // Two running Codex threads == the Codex cap.
        for i in 0..2u32 {
            state.threads.insert(
                format!("C-{i}"),
                running_thread(&format!("C-{i}"), 10 + i, AgentClient::Codex),
            );
        }
        assert!(
            !state.provider_under_cap(AgentClient::Codex),
            "codex is at its cap"
        );
        assert!(
            state.provider_under_cap(AgentClient::Claude),
            "claude has no cap and is unaffected"
        );
    }

    #[test]
    fn test_provider_under_cap_counts_only_matching_provider() {
        let mut state = State {
            config: PluginConfig {
                provider_caps: [(AgentClient::Codex, 2)].into_iter().collect(),
                ..PluginConfig::new()
            },
            ..State::default()
        };
        // Three running Claude threads must NOT count against the Codex cap.
        for i in 0..3u32 {
            state.threads.insert(
                format!("A-{i}"),
                running_thread(&format!("A-{i}"), 10 + i, AgentClient::Claude),
            );
        }
        assert!(
            state.provider_under_cap(AgentClient::Codex),
            "codex has zero running threads despite the claude load"
        );
    }

    #[test]
    fn test_find_idle_slot_provider_affinity() {
        let mut state = State {
            config: PluginConfig {
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        // Slot 0 last ran Claude; slot 1 is fresh (never hosted a session).
        state
            .agent_slots
            .push(fresh_slot(10, Some(AgentClient::Claude)));
        state.agent_slots.push(fresh_slot(11, None));

        // Codex skips the Claude-affine slot and takes the fresh one.
        assert_eq!(state.find_idle_slot(AgentClient::Codex), Some(1));
        // Claude prefers the matching slot 0 (first eligible).
        assert_eq!(state.find_idle_slot(AgentClient::Claude), Some(0));
    }

    #[test]
    fn test_find_idle_slot_rejects_mismatched_resident_provider() {
        let mut state = State {
            config: PluginConfig {
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        // Only a Claude-affine slot is available.
        state
            .agent_slots
            .push(fresh_slot(10, Some(AgentClient::Claude)));
        // The direct-reuse helper rejects the mismatch. The higher-level
        // find_slot_for_client helper turns this into an explicit recycle.
        assert_eq!(state.find_idle_slot(AgentClient::Codex), None);
    }

    #[test]
    fn test_find_slot_for_client_recycles_when_all_idle_panes_have_other_provider() {
        let mut state = State {
            config: PluginConfig {
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        for pane_id in 10..14 {
            state
                .agent_slots
                .push(fresh_slot(pane_id, Some(AgentClient::Claude)));
        }

        assert_eq!(
            state.find_slot_for_client(AgentClient::Codex),
            Some(SlotSelection::Recycle(0))
        );
    }

    #[test]
    fn test_find_slot_for_client_prefers_compatible_pane_over_recycling() {
        let mut state = State {
            config: PluginConfig {
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        state
            .agent_slots
            .push(fresh_slot(10, Some(AgentClient::Claude)));
        state
            .agent_slots
            .push(fresh_slot(11, Some(AgentClient::Codex)));

        assert_eq!(
            state.find_slot_for_client(AgentClient::Codex),
            Some(SlotSelection::Compatible(1))
        );
    }

    #[test]
    fn test_find_slot_for_client_never_recycles_running_pane() {
        let mut state = State {
            config: PluginConfig {
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        for pane_id in 10..14 {
            let mut slot = fresh_slot(pane_id, Some(AgentClient::Claude));
            slot.ticket_id = Some(format!("T-{pane_id}"));
            state.agent_slots.push(slot);
        }

        assert_eq!(state.find_slot_for_client(AgentClient::Codex), None);
    }

    #[test]
    fn test_mixed_provider_stress_16() {
        // The acceptance-criterion-2 stress artifact: 16 mixed agents with
        // per-provider caps 8/8 under a global cap of 16, 32 slots. Drives the
        // real spawn-gate decision functions (global count, provider_under_cap,
        // find_idle_slot affinity) in the exact order schedule_ready_tickets uses,
        // committing each admission as the scheduler would. Asserts every
        // invariant the ticket names: global cap, per-provider caps, unique slot
        // per thread, no cross-provider slot reuse, surplus stays unscheduled.
        use lisa_core::types::ThreadStatus;

        let mut state = State {
            config: PluginConfig {
                max_threads: 16,
                wind_down_secs: 0,
                provider_caps: [(AgentClient::Claude, 8), (AgentClient::Codex, 8)]
                    .into_iter()
                    .collect(),
                ..PluginConfig::new()
            },
            ..State::default()
        };
        for i in 0..32u32 {
            state.agent_slots.push(fresh_slot(100 + i, None));
        }

        // Offer 16 Claude tickets then 16 Codex tickets — far more than can run.
        // Claude fills its cap (8) while the global still has room, proving the
        // per-provider cap binds independently of the global; Codex then fills to
        // 16 total.
        let offered: Vec<(String, AgentClient)> = (0..16)
            .map(|i| (format!("A-{i}"), AgentClient::Claude))
            .chain((0..16).map(|i| (format!("C-{i}"), AgentClient::Codex)))
            .collect();

        let mut admitted = 0usize;
        let mut unscheduled = 0usize;
        for (tid, want) in &offered {
            let running_total = state
                .threads
                .values()
                .filter(|t| t.status == ThreadStatus::Running)
                .count();
            if running_total >= state.config.max_threads {
                unscheduled += 1;
                continue;
            }
            if !state.provider_under_cap(*want) {
                unscheduled += 1;
                continue;
            }
            let slot_idx = match state.find_idle_slot(*want) {
                Some(s) => s,
                None => {
                    unscheduled += 1;
                    continue;
                }
            };
            let pane_id = state.agent_slots[slot_idx].pane_id;
            state.agent_slots[slot_idx].ticket_id = Some(tid.clone());
            state.agent_slots[slot_idx].last_client = Some(*want);
            state
                .threads
                .insert(tid.clone(), running_thread(tid, pane_id, *want));
            admitted += 1;
        }

        let running = |c: AgentClient| {
            state
                .threads
                .values()
                .filter(|t| t.status == ThreadStatus::Running && t.client == c)
                .count()
        };
        let total = running(AgentClient::Claude) + running(AgentClient::Codex);

        assert_eq!(total, 16, "exactly the global cap of concurrent agents");
        assert_eq!(running(AgentClient::Claude), 8, "claude per-provider cap");
        assert_eq!(running(AgentClient::Codex), 8, "codex per-provider cap");
        assert_eq!(admitted, 16);
        assert_eq!(
            unscheduled, 16,
            "the surplus 16 tickets stay unscheduled, not dropped"
        );

        // No slot serves a provider other than the one stamped on it.
        for slot in &state.agent_slots {
            if let (Some(_), Some(last)) = (&slot.ticket_id, slot.last_client) {
                let owner = state.threads.values().find(|t| t.pane_id == slot.pane_id);
                assert_eq!(
                    owner.map(|t| t.client),
                    Some(last),
                    "slot {} provider matches its running thread",
                    slot.pane_id
                );
            }
        }
        // No two running threads share a pane (no slot leak / double-assignment).
        let mut panes: Vec<u32> = state.threads.values().map(|t| t.pane_id).collect();
        panes.sort_unstable();
        let before = panes.len();
        panes.dedup();
        assert_eq!(panes.len(), before, "each running thread has a unique pane");
    }

    #[test]
    fn test_signal_scan_cost_at_32_panes() {
        // Signal-dir cost probe (T-026-02 findings / ticket note): populate the
        // dir with ~32 panes' worth of mixed signal files and confirm one
        // heartbeat scan consumes exactly the heartbeat files, leaving the rest.
        // Documents the O(files) per-scan behaviour — poll_tick runs five such
        // scans per tick, at the 5s POLL_INTERVAL_SECS cadence.
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let sigdir = dir.path().join("signals");
        fs::create_dir_all(&sigdir).unwrap();
        for pane in 0..32u32 {
            fs::write(sigdir.join(format!("pane-{pane}.heartbeat")), "").unwrap();
            fs::write(sigdir.join(format!("pane-{pane}.idle")), "").unwrap();
        }
        let mut state = State {
            signal_dir: sigdir.clone(),
            ..State::default()
        };
        state.check_heartbeat_signals();

        let remaining: Vec<String> = fs::read_dir(&sigdir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            remaining.len(),
            32,
            "only the 32 idle files remain after the heartbeat scan"
        );
        assert!(
            remaining.iter().all(|n| n.ends_with(".idle")),
            "heartbeat scan leaves non-heartbeat signals untouched"
        );
    }

    #[test]
    fn test_reset_ticket_sets_ready_phase() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: in-progress\npriority: high\nphase: implement\n---\n\nBody\n",
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

        // Add a running thread
        let thread = Thread::new("T-001", 1);
        state.threads.insert("T-001".to_string(), thread);

        state.reset_ticket("T-001");

        // Phase should now be ready
        let content = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(
            content.contains("phase: ready"),
            "Phase should be reset to ready, got: {}",
            content
        );
        assert!(
            content.contains("status: open"),
            "Status should be reset to open, got: {}",
            content
        );

        // Thread should be removed
        assert!(
            !state.threads.contains_key("T-001"),
            "Thread should be removed after reset"
        );
    }

    #[test]
    fn test_reset_modal_shows_working_tickets() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        // One ready, one implementing, one done
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: ready ticket\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n",
        ).unwrap();
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: impl ticket\ntype: task\nstatus: in-progress\npriority: high\nphase: implement\n---\n",
        ).unwrap();
        fs::write(
            tickets_dir.join("T-003.md"),
            "---\nid: T-003\ntitle: done ticket\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n",
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

        state.open_reset_modal();

        assert!(state.modal.open, "Modal should be open");
        assert_eq!(
            state.modal.mode,
            ModalMode::ResetTicket,
            "Mode should be ResetTicket"
        );
        // Only T-002 (implement) should appear — not T-001 (ready) or T-003 (done)
        assert_eq!(state.modal.ticket_ids, vec!["T-002".to_string()]);
    }

    #[test]
    fn test_reset_modal_excludes_ready_and_done() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();

        // All ready and done — nothing to reset
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: a\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n",
        ).unwrap();
        fs::write(
            tickets_dir.join("T-002.md"),
            "---\nid: T-002\ntitle: b\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n",
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

        state.open_reset_modal();

        assert!(
            !state.modal.open,
            "Modal should NOT open when nothing to reset"
        );
    }

    // =========================================================================
    // Transition state machine tests (T-010-02)
    // =========================================================================

    #[test]
    fn test_unrelated_timer_does_not_flush_pending_enter_early() {
        let base = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000);
        let mut state = State::default();
        state.pending_enters.push_back(PendingEnter {
            pane_id: PaneId::Terminal(7),
            ready_at: base + std::time::Duration::from_secs(2),
        });

        let early = state.take_due_pending_enters(base + std::time::Duration::from_secs(1));

        assert!(early.is_empty(), "an unrelated timer must not submit early");
        assert_eq!(state.pending_enters.len(), 1);

        let due = state.take_due_pending_enters(base + std::time::Duration::from_secs(2));
        assert_eq!(due, vec![PaneId::Terminal(7)]);
        assert!(state.pending_enters.is_empty());
    }

    #[test]
    fn test_pending_enters_keep_independent_deadlines_and_order() {
        let base = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(2_000);
        let mut state = State::default();
        state.pending_enters.extend([
            PendingEnter {
                pane_id: PaneId::Terminal(1),
                ready_at: base + std::time::Duration::from_secs(1),
            },
            PendingEnter {
                pane_id: PaneId::Terminal(2),
                ready_at: base + std::time::Duration::from_secs(3),
            },
            PendingEnter {
                pane_id: PaneId::Terminal(3),
                ready_at: base + std::time::Duration::from_secs(2),
            },
        ]);

        let due = state.take_due_pending_enters(base + std::time::Duration::from_secs(2));

        assert_eq!(due, vec![PaneId::Terminal(1), PaneId::Terminal(3)]);
        assert_eq!(state.pending_enters.len(), 1);
        assert_eq!(state.pending_enters[0].pane_id, PaneId::Terminal(2));

        let remaining = state.take_due_pending_enters(base + std::time::Duration::from_secs(3));
        assert_eq!(remaining, vec![PaneId::Terminal(2)]);
        assert!(state.pending_enters.is_empty());
    }

    #[test]
    fn test_transition_state_default_is_idle() {
        let slot = AgentSlot {
            pane_id: 1,
            ticket_id: None,
            attempt_lease: None,
            has_session: false,
            transition_state: TransitionState::default(),
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        };
        assert_eq!(slot.transition_state, TransitionState::Idle);
    }

    #[test]
    fn test_check_transition_signals_stopped_advances_state() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.stopped"), "2025-01-01T00:00:00Z").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::WaitingForStop,
            transition_started_at: Some(std::time::SystemTime::now()),
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.check_transition_signals();

        // Signal file should be deleted
        assert!(!signal_dir.join("pane-1.stopped").exists());

        // State should advance to WaitingForClear
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForClear
        );
        assert!(state.agent_slots[0].transition_started_at.is_some());

        // Should have logged an info event
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Info { message } if message.contains("stopped") && message.contains("/clear")
        )));
    }

    #[test]
    fn test_check_transition_signals_stopped_ignored_when_idle() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.stopped"), "2025-01-01T00:00:00Z").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.check_transition_signals();

        // Signal file should be deleted (always cleaned up)
        assert!(!signal_dir.join("pane-1.stopped").exists());

        // State should remain Idle
        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
    }

    #[test]
    fn test_check_transition_signals_cleared_advances_state() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.cleared"), "2025-01-01T00:00:00Z").unwrap();

        let mut state = State {
            config: PluginConfig {
                ticket_dir: dir.path().join("tickets"),
                ..PluginConfig::new()
            },
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::WaitingForClear,
            transition_started_at: Some(std::time::SystemTime::now()),
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.check_transition_signals();

        // Signal file should be deleted
        assert!(!signal_dir.join("pane-1.cleared").exists());

        // State should return to Idle
        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
        assert!(state.agent_slots[0].transition_started_at.is_none());

        // Should have logged an info event about sending prompt
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Info { message } if message.contains("cleared") && message.contains("T-001")
        )));
    }

    #[test]
    fn test_check_transition_signals_cleared_ignored_when_idle() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.cleared"), "2025-01-01T00:00:00Z").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.check_transition_signals();

        // Signal cleaned up but state unchanged
        assert!(!signal_dir.join("pane-1.cleared").exists());
        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
    }

    #[test]
    fn test_check_transition_signals_unknown_pane_ignored() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-999.stopped"), "2025-01-01T00:00:00Z").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        // No slots — pane 999 doesn't exist
        state.check_transition_signals();

        // Signal cleaned up, no crash
        assert!(!signal_dir.join("pane-999.stopped").exists());
    }

    #[test]
    fn test_check_transition_timeouts_stop_timeout() {
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::WaitingForStop,
            // Set to 61 seconds ago
            transition_started_at: Some(
                std::time::SystemTime::now() - std::time::Duration::from_secs(61),
            ),
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.check_transition_timeouts();

        // Should have forced to WaitingForClear
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForClear
        );
        assert!(state.agent_slots[0].transition_started_at.is_some());

        // Should have logged a warning
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Warning { message } if message.contains("Stop signal timeout")
        )));
    }

    #[test]
    fn test_check_transition_timeouts_clear_timeout() {
        let mut state = State {
            config: PluginConfig {
                ticket_dir: std::path::PathBuf::from("/tmp/tickets"),
                ..PluginConfig::new()
            },
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::WaitingForClear,
            // Past the 90s clear-signal timeout
            transition_started_at: Some(
                std::time::SystemTime::now() - std::time::Duration::from_secs(91),
            ),
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        state.seat_assignments.insert(
            1,
            SeatAssignmentState::AssignedPendingAck {
                generation: 1,
                ack_deadline: None,
            },
        );

        state.check_transition_timeouts();

        // Should have forced to Idle
        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
        assert!(state.agent_slots[0].transition_started_at.is_none());
        assert!(matches!(
            state.seat_assignment(1),
            Some(SeatAssignmentState::AssignedPendingAck {
                generation: 1,
                ack_deadline: Some(_),
            })
        ));
        assert!(!state.seat_is_owned(1));

        // Should have logged a warning
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Warning { message } if message.contains("Clear signal timeout")
        )));
    }

    #[test]
    fn test_check_transition_timeouts_within_threshold_no_change() {
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::WaitingForStop,
            // Set to 5 seconds ago — well within the 60s threshold
            transition_started_at: Some(
                std::time::SystemTime::now() - std::time::Duration::from_secs(5),
            ),
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.check_transition_timeouts();

        // No change — still WaitingForStop
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForStop
        );
        assert!(state.activity_log.is_empty());
    }

    #[test]
    fn test_recycle_exit_grace_launches_fresh_incoming_client() {
        let dir = tempfile::tempdir().unwrap();
        let lease = AttemptLease::mint("T-RECYCLE", None).unwrap();
        let mut state = State {
            config: PluginConfig {
                client: AgentClient::Codex,
                ticket_dir: std::path::PathBuf::from("/tmp/tickets"),
                work_dir: dir.path().join("work"),
                ..PluginConfig::new()
            },
            ..State::default()
        };
        state
            .current_leases
            .insert(lease.ticket_id.clone(), lease.clone());
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-RECYCLE".to_string()),
            attempt_lease: Some(lease),
            has_session: false,
            transition_state: TransitionState::WaitingForExit,
            transition_started_at: Some(
                std::time::SystemTime::now()
                    - std::time::Duration::from_secs(AGENT_EXIT_GRACE_SECS + 1),
            ),
            cooldown_until: None,
            last_activity_at: None,
            // Scheduling stamps the incoming provider while `/exit` is pending.
            last_client: Some(AgentClient::Codex),
        });
        state.seat_assignments.insert(
            1,
            SeatAssignmentState::AssignedPendingAck {
                generation: 1,
                ack_deadline: None,
            },
        );

        state.check_transition_timeouts();

        let slot = &state.agent_slots[0];
        assert_eq!(slot.transition_state, TransitionState::Idle);
        assert!(slot.transition_started_at.is_none());
        assert!(slot.has_session);
        assert_eq!(slot.last_client, Some(AgentClient::Codex));
        assert!(matches!(
            state.seat_assignment(1),
            Some(SeatAssignmentState::AssignedPendingAck {
                generation: 1,
                ack_deadline: Some(_),
            })
        ));
        assert!(!state.seat_is_owned(1));
        assert_eq!(state.pending_enters.len(), 1, "fresh launch queued Enter");
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Info { message }
                if message.contains("launched codex") && message.contains("T-RECYCLE")
        )));
    }

    #[test]
    fn test_recycle_waits_for_exit_grace_before_launch() {
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-RECYCLE".to_string()),
            attempt_lease: None,
            has_session: false,
            transition_state: TransitionState::WaitingForExit,
            transition_started_at: Some(std::time::SystemTime::now()),
            cooldown_until: None,
            last_activity_at: None,
            last_client: Some(AgentClient::Codex),
        });

        state.check_transition_timeouts();

        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForExit
        );
        assert!(state.pending_enters.is_empty());
    }

    #[test]
    fn test_pane_title_missing_recycle_ticket_restores_empty_shell_idle() {
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: None,
            attempt_lease: None,
            has_session: false,
            transition_state: TransitionState::WaitingForExit,
            transition_started_at: Some(
                std::time::SystemTime::now()
                    - std::time::Duration::from_secs(AGENT_EXIT_GRACE_SECS + 1),
            ),
            cooldown_until: None,
            last_activity_at: None,
            last_client: Some(AgentClient::Codex),
        });
        state
            .last_pane_names
            .insert(1, "codex · T-GONE · removed".to_string());

        state.check_transition_timeouts();

        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
        assert!(!state.agent_slots[0].has_session);
        assert_eq!(state.agent_slots[0].last_client, None);
        assert_eq!(
            state.last_pane_names.get(&1).map(String::as_str),
            Some("lisa · idle")
        );
    }

    #[test]
    fn test_recycle_discards_idle_signal_from_exiting_client() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.idle"), "stale").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-RECYCLE".to_string()),
            attempt_lease: None,
            has_session: false,
            transition_state: TransitionState::WaitingForExit,
            transition_started_at: Some(std::time::SystemTime::now()),
            cooldown_until: None,
            last_activity_at: None,
            last_client: Some(AgentClient::Codex),
        });
        state.threads.insert(
            "T-RECYCLE".to_string(),
            running_thread("T-RECYCLE", 1, AgentClient::Codex),
        );

        state.check_idle_signals();

        assert!(!signal_dir.join("pane-1.idle").exists());
        assert!(state.agent_slots[0].last_activity_at.is_none());
        assert!(state.idle_alerts.is_empty());
    }

    #[test]
    fn test_check_transition_signals_idle_files_not_consumed() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        // .idle files should be left for check_idle_signals(), not consumed here
        fs::write(signal_dir.join("pane-1.idle"), "2025-01-01T00:00:00Z").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };

        state.check_transition_signals();

        // .idle file should still exist — not consumed by check_transition_signals
        assert!(signal_dir.join("pane-1.idle").exists());
    }

    // =========================================================================
    // Review auto-complete tests (T-010-03)
    //
    // Note: We test auto_complete_review() directly instead of
    // handle_stopped_signal() because the latter calls self.send_line_to_pane()
    // (a zellij host function) in the WaitingForStop branch, which
    // can't link on native test targets.
    // =========================================================================

    #[test]
    fn test_auto_complete_review_updates_ticket_and_cleans_up() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: review\npriority: high\nphase: review\n---\n\nBody\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let mut state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: dir.path().join("work"),
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Agent slot with ticket assigned
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        // Running thread in Review phase
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        state.threads.insert("T-001".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-001");
        write_passing_review_disposition(&state, &lease);

        // Directly call auto_complete_review
        state.auto_complete_review("T-001".to_string(), 1);

        // Nothing publishes until the native transaction result succeeds.
        assert!(state.threads.contains_key("T-001"));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-001"));
        assert!(state.pending_completions.contains_key("T-001"));
        assert!(!state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::TicketPhaseChanged { new_phase, .. } if *new_phase == Phase::Done
        )));

        let content = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(content.contains("phase: review"), "{content}");
        assert!(content.contains("status: review"), "{content}");
    }

    #[test]
    fn test_auto_complete_review_block_retains_assignment_with_visible_reason() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: review\npriority: high\nphase: review\n---\n",
        )
        .unwrap();
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: dir.path().join("work"),
                ..PluginConfig::new()
            },
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        state.threads.insert("T-001".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-001");
        write_review_disposition(
            &state,
            &lease,
            r#"{"disposition":"block","reason":"rerun the release suite"}"#,
        );

        state.auto_complete_review("T-001".to_string(), 1);

        assert!(!state.pending_completions.contains_key("T-001"));
        assert!(state.threads.contains_key("T-001"));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-001"));
        assert!(fs::read_to_string(tickets_dir.join("T-001.md"))
            .unwrap()
            .contains("phase: review"));
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::CompletionRejected {
                kind: CompletionRejectionKind::DispositionBlocked,
                detail,
                ..
            } if detail.contains("rerun the release suite")
        )));
    }

    #[test]
    fn test_auto_complete_review_condition_non_review_skipped() {
        // Verify that the condition logic in handle_stopped_signal correctly
        // identifies non-Review tickets as ineligible for auto-complete.
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: test\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
        ).unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();

        let state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Check: ticket is NOT in Review phase
        let is_review = state
            .dag
            .get_ticket(&"T-001".to_string())
            .map(|t| t.phase == Phase::Review)
            .unwrap_or(false);
        assert!(
            !is_review,
            "Implement-phase ticket should not be detected as Review"
        );
    }

    #[test]
    fn test_auto_complete_review_condition_completed_thread_skipped() {
        // Verify that already-Completed threads are not re-processed.
        use lisa_core::types::{Thread, ThreadStatus};

        let mut state = State::default();

        // Thread already completed
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        thread.complete();
        state.threads.insert("T-001".to_string(), thread);

        // The condition in handle_stopped_signal:
        let skip = state
            .threads
            .get("T-001")
            .map(|t| t.status == ThreadStatus::Completed)
            .unwrap_or(true);
        assert!(skip, "Completed thread should be skipped");
    }

    #[test]
    fn test_auto_complete_review_condition_missing_thread_skipped() {
        // Verify that missing threads are skipped.
        let state = State::default();

        let skip = state
            .threads
            .get("T-NONEXISTENT")
            .map(|t| t.status == lisa_core::types::ThreadStatus::Completed)
            .unwrap_or(true);
        assert!(skip, "Missing thread should be skipped (unwrap_or(true))");
    }

    #[test]
    fn test_auto_complete_review_condition_parked_thread_eligible() {
        // Verify that Parked threads ARE eligible for auto-complete.
        use lisa_core::types::Thread;

        let mut state = State::default();

        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        thread.park();
        state.threads.insert("T-001".to_string(), thread);

        let skip = state
            .threads
            .get("T-001")
            .map(|t| t.status == lisa_core::types::ThreadStatus::Completed)
            .unwrap_or(true);
        assert!(!skip, "Parked thread should NOT be skipped");
    }

    #[test]
    fn test_auto_complete_review_condition_running_thread_eligible() {
        // Verify that Running threads in Review phase ARE eligible.
        use lisa_core::types::Thread;

        let mut state = State::default();

        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        // status is Running by default
        state.threads.insert("T-001".to_string(), thread);

        let skip = state
            .threads
            .get("T-001")
            .map(|t| t.status == lisa_core::types::ThreadStatus::Completed)
            .unwrap_or(true);
        assert!(!skip, "Running thread should NOT be skipped");
    }

    // ---- Finish-up prompt tests ----

    #[test]
    fn test_check_review_timeouts_sends_prompt_after_timeout() {
        use lisa_core::types::Thread;

        let mut state = State {
            config: PluginConfig {
                review_timeout_secs: 10,
                wind_down_secs: 180,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Running Review thread, silent past both the review timeout and the
        // wind-down period — eligible for a finish-up prompt
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(200);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-001".to_string(), thread);

        state.check_review_timeouts();

        // Thread should still be Running
        let t = state.threads.get("T-001").unwrap();
        assert_eq!(t.status, lisa_core::types::ThreadStatus::Running);

        // Should be in finish_up_sent
        assert!(state.finish_up_sent.contains("T-001"));

        // Activity log should contain FinishUpPromptSent
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::FinishUpPromptSent { ticket_id, .. } if ticket_id == "T-001"
        )));
    }

    #[test]
    fn review_timeout_prompts_only_when_current_attempt_review_is_missing() {
        use std::fs;

        const TICKET_ID: &str = "T-TIMEOUT-MISSING";
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path().join("games/midsummer");
        let tickets_dir = project_root.join("tickets");
        let work_dir = project_root.join("work");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join(format!("{TICKET_ID}.md")),
            format!(
                "---\nid: {TICKET_ID}\ntitle: missing-review\ntype: bug\nstatus: review\npriority: high\nphase: review\n---\n"
            ),
        )
        .unwrap();

        let (mut state, lease) = review_timeout_state(
            TICKET_ID,
            tickets_dir,
            work_dir,
            project_root,
            dir.path().to_path_buf(),
            PathBuf::new(),
        );
        assert!(!state.attempt_work_dir(&lease).join("review.md").exists());

        state.check_review_timeouts();

        assert!(state.finish_up_sent.contains(TICKET_ID));
        assert_eq!(
            state
                .activity_log
                .iter()
                .filter(|event| matches!(
                    event,
                    ActivityEvent::FinishUpPromptSent { ticket_id, pane_id }
                        if ticket_id == TICKET_ID && *pane_id == 42
                ))
                .count(),
            1
        );
    }

    #[test]
    fn review_timeout_prompts_when_disposition_is_missing_after_review() {
        use std::fs;

        const TICKET_ID: &str = "T-TIMEOUT-DISPOSITION";
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path().join("project");
        let tickets_dir = project_root.join("tickets");
        let work_dir = project_root.join("work");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join(format!("{TICKET_ID}.md")),
            format!(
                "---\nid: {TICKET_ID}\ntitle: missing-disposition\ntype: bug\nstatus: open\npriority: critical\nphase: review\n---\n"
            ),
        )
        .unwrap();

        let (mut state, lease) = review_timeout_state(
            TICKET_ID,
            tickets_dir,
            work_dir,
            project_root,
            dir.path().to_path_buf(),
            PathBuf::new(),
        );
        fs::write(
            state.attempt_work_dir(&lease).join("review.md"),
            "# Review\n\nReady, but an old workflow omitted the disposition.\n",
        )
        .unwrap();

        state.reconcile_review_completions();
        assert!(!state.pending_completions.contains_key(TICKET_ID));
        state.check_review_timeouts();

        assert!(state.finish_up_sent.contains(TICKET_ID));
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::FinishUpPromptSent { ticket_id, pane_id }
                if ticket_id == TICKET_ID && *pane_id == 42
        )));
    }

    #[test]
    fn review_timeout_suppresses_admitted_pending_and_confirmed_completion() {
        use std::fs;

        const TICKET_ID: &str = "T-TIMEOUT-PENDING";
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path().join("games/midsummer");
        let tickets_dir = project_root.join("tickets");
        let work_dir = project_root.join("work");
        let ticket_file = tickets_dir.join(format!("{TICKET_ID}.md"));
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            &ticket_file,
            format!(
                "---\nid: {TICKET_ID}\ntitle: pending-review\ntype: bug\nstatus: review\npriority: high\nphase: review\n---\n"
            ),
        )
        .unwrap();

        let journal = dir.path().join(".lisa/completion-journal.jsonl");
        let (mut state, lease) = review_timeout_state(
            TICKET_ID,
            tickets_dir,
            work_dir,
            project_root,
            dir.path().to_path_buf(),
            journal,
        );
        write_private_review(&state, &lease);
        let completion_key = CompletionGenerationId::new(
            CompletionId::new(TICKET_ID),
            AttemptId::new(lease.attempt_id.to_string()),
            1,
        );

        assert!(state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: TICKET_ID.to_string(),
            source_lease: lease,
        }));
        assert!(state.pending_completions.contains_key(TICKET_ID));
        assert!(matches!(
            state.completion_aggregates[TICKET_ID].state(),
            CompletionState::CommandInFlight { correlation, .. }
                if correlation.as_str() == completion_key.to_string()
        ));

        state.reconcile_review_completions();
        assert_eq!(state.launched_completion_effects.len(), 1);
        assert_eq!(
            fs::read_to_string(&state.completion_journal_path)
                .unwrap()
                .lines()
                .count(),
            2,
            "a live pending command suppresses duplicate reconciliation replay"
        );

        state.check_review_timeouts();
        assert_no_finish_up(&state, TICKET_ID);
        assert!(state.pending_completions.contains_key(TICKET_ID));

        lisa_core::ticket::update_ticket_done(&ticket_file).unwrap();
        state.handle_completion_result(TICKET_ID, Some(0), vec![b'a'; 40], Vec::new());
        assert!(matches!(
            state.completion_aggregates[TICKET_ID].state(),
            CompletionState::Confirmed
        ));
        assert!(!state.threads.contains_key(TICKET_ID));

        state.check_review_timeouts();
        assert_no_finish_up(&state, TICKET_ID);
    }

    #[test]
    fn review_timeout_preserves_nested_path_launch_rejection() {
        use std::fs;

        const TICKET_ID: &str = "T-TIMEOUT-PATH";
        let dir = tempfile::tempdir().unwrap();
        let git_root = dir.path().join("repo");
        let project_root = git_root.join("games/midsummer");
        let tickets_dir = dir.path().join("outside/tickets");
        let work_dir = project_root.join("work");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::create_dir_all(&project_root).unwrap();
        fs::write(
            tickets_dir.join(format!("{TICKET_ID}.md")),
            format!(
                "---\nid: {TICKET_ID}\ntitle: outside-git-root\ntype: bug\nstatus: review\npriority: high\nphase: review\n---\n"
            ),
        )
        .unwrap();

        let journal = git_root.join(".lisa/completion-journal.jsonl");
        let (mut state, lease) = review_timeout_state(
            TICKET_ID,
            tickets_dir,
            work_dir,
            project_root,
            git_root,
            journal,
        );
        write_private_review(&state, &lease);
        let completion_key = CompletionGenerationId::new(
            CompletionId::new(TICKET_ID),
            AttemptId::new(lease.attempt_id.to_string()),
            1,
        );

        assert!(!state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: TICKET_ID.to_string(),
            source_lease: lease,
        }));
        assert!(!state.pending_completions.contains_key(TICKET_ID));
        assert!(!state.completion_aggregates.contains_key(TICKET_ID));
        let rejection = correlated_launch_failure(&state, TICKET_ID, &completion_key);
        assert!(matches!(
            rejection,
            ActivityEvent::CompletionRejected { detail, .. }
                if detail.contains("completion path outside Git root")
                    && detail.contains("outside/tickets")
        ));
        assert_rejection_renders_unchanged(rejection);

        state.check_review_timeouts();
        assert_no_finish_up(&state, TICKET_ID);
    }

    #[test]
    fn review_timeout_preserves_bounded_command_retry() {
        use std::fs;

        const TICKET_ID: &str = "T-TIMEOUT-RETRY";
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path().join("games/midsummer");
        let tickets_dir = project_root.join("tickets");
        let work_dir = project_root.join("work");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join(format!("{TICKET_ID}.md")),
            format!(
                "---\nid: {TICKET_ID}\ntitle: retry-review\ntype: bug\nstatus: review\npriority: high\nphase: review\n---\n"
            ),
        )
        .unwrap();

        let journal = dir.path().join(".lisa/completion-journal.jsonl");
        let (mut state, lease) = review_timeout_state(
            TICKET_ID,
            tickets_dir,
            work_dir,
            project_root,
            dir.path().to_path_buf(),
            journal,
        );
        write_private_review(&state, &lease);
        let completion_key = CompletionGenerationId::new(
            CompletionId::new(TICKET_ID),
            AttemptId::new(lease.attempt_id.to_string()),
            1,
        );

        assert!(state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: TICKET_ID.to_string(),
            source_lease: lease.clone(),
        }));
        state.handle_completion_result(
            TICKET_ID,
            Some(1),
            Vec::new(),
            b"Author identity unknown".to_vec(),
        );

        assert!(state.pending_completions.contains_key(TICKET_ID));
        assert!(state.threads.contains_key(TICKET_ID));
        assert_eq!(state.current_leases.get(TICKET_ID), Some(&lease));
        assert!(matches!(
            state.completion_aggregates[TICKET_ID].state(),
            CompletionState::CommandInFlight { .. }
        ));
        assert_eq!(state.completion_aggregates[TICKET_ID].failure_count(), 1);
        let rejection = correlated_launch_failure(&state, TICKET_ID, &completion_key);
        assert!(matches!(
            rejection,
            ActivityEvent::CompletionRejected { detail, .. }
                if detail.starts_with(HISTORY_IDENTITY_ASK)
                    && detail.contains("Author identity unknown")
        ));
        assert_rejection_renders_unchanged(rejection);

        state.check_review_timeouts();
        assert_no_finish_up(&state, TICKET_ID);
        assert!(matches!(
            state.reconciliation_state(TICKET_ID),
            CompletionState::CommandInFlight { .. }
        ));
    }

    #[test]
    fn completion_failure_classifier_is_conservative_and_asks_are_plain() {
        let cases = [
            (
                "fatal: your current branch 'main' does not have any commits yet",
                CompletionFailureClass::OperatorHistoryOrIdentity,
            ),
            (
                "Author identity unknown; Please tell me who you are.",
                CompletionFailureClass::OperatorHistoryOrIdentity,
            ),
            (
                "fatal: unable to write new index file: Permission denied",
                CompletionFailureClass::OperatorRepositoryUnwritable,
            ),
            (
                "stale .lisa-commit.lock belongs to a dead process",
                CompletionFailureClass::OperatorStaleLock,
            ),
            (
                "Unable to create .git/index.lock: another git process seems to be running",
                CompletionFailureClass::TransientContention,
            ),
            (
                "fatal: unexpected ref transaction",
                CompletionFailureClass::Unrecognized,
            ),
        ];
        for (detail, expected) in cases {
            assert_eq!(classify_completion_failure(detail), expected, "{detail}");
        }
        assert_eq!(
            completion_failure_ask(CompletionFailureClass::OperatorHistoryOrIdentity, "T-ASK")
                .as_deref(),
            Some(HISTORY_IDENTITY_ASK)
        );
        for class in [
            CompletionFailureClass::OperatorHistoryOrIdentity,
            CompletionFailureClass::OperatorRepositoryUnwritable,
            CompletionFailureClass::OperatorStaleLock,
        ] {
            assert_eq!(
                completion_failure_action(class, 1),
                CompletionFailureAction::Retry
            );
            assert_eq!(
                completion_failure_action(class, 2),
                CompletionFailureAction::Park
            );
            let ask = completion_failure_ask(class, "T-ASK").unwrap();
            assert!(ask.starts_with("Lisa"));
            assert!(!ask.contains("AttemptLease"));
        }
        assert_eq!(
            completion_failure_action(CompletionFailureClass::TransientContention, 2),
            CompletionFailureAction::WaitForDeadline
        );
        assert_eq!(
            completion_failure_action(CompletionFailureClass::Unrecognized, 1),
            CompletionFailureAction::Park
        );
    }

    #[test]
    fn field_journal_replay_bounds_unborn_identityless_completion_and_cleans_lock() {
        const TICKET_ID: &str = "T-001";
        const PRESERVED_JOURNAL: &str = include_str!(
            "../../../docs/active/work/T-046-06-03/cbt-0716-211915-variant-xdg/demo-completion-journal.jsonl"
        );

        let field_rejections = PRESERVED_JOURNAL
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .filter(|row| {
                row["state"] == "rejected"
                    && row["completion_id"] == TICKET_ID
                    && row["reason"].as_str().is_some_and(|reason| {
                        reason.contains("discover prior completion commit")
                            && reason.contains("does not have any commits yet")
                    })
            })
            .count();
        assert_eq!(
            field_rejections, 80,
            "the preserved 2026-07-16 journal remains the replay source"
        );

        let (mut state, lease, dir, journal, ledger) = completion_failure_fixture(TICKET_ID);
        initialize_unborn_identityless_repository(dir.path());
        assert!(state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: TICKET_ID.to_string(),
            source_lease: lease.clone(),
        }));
        assert_eq!(state.launched_completion_effects.len(), 1);

        let first = real_unborn_completion_error(&state, &lease, TICKET_ID);
        assert!(first.contains("resolve HEAD for ticket commit"), "{first}");
        assert!(first.contains("does not have any commits yet"), "{first}");
        assert!(
            !first.contains("discover prior completion commit"),
            "{first}"
        );
        state.handle_completion_result(TICKET_ID, Some(1), Vec::new(), first.into_bytes());
        assert_eq!(state.launched_completion_effects.len(), 2);
        assert_eq!(state.completion_aggregates[TICKET_ID].failure_count(), 1);
        assert!(state.pending_completions.contains_key(TICKET_ID));

        let second = real_unborn_completion_error(&state, &lease, TICKET_ID);
        state.handle_completion_result(TICKET_ID, Some(1), Vec::new(), second.into_bytes());

        assert_eq!(
            state.launched_completion_effects.len(),
            2,
            "one initial command plus one bounded retry"
        );
        assert!(!state.pending_completions.contains_key(TICKET_ID));
        assert!(!state.threads.contains_key(TICKET_ID));
        assert!(state.agent_slots[0].ticket_id.is_none());
        let ticket = ticket::scan_tickets(&state.config.ticket_dir)
            .unwrap()
            .into_iter()
            .find(|ticket| ticket.id == TICKET_ID)
            .unwrap();
        assert_eq!(ticket.phase, Phase::Review);
        assert_eq!(ticket.status, TicketStatus::Blocked);
        assert!(matches!(
            parse_review_disposition(
                state
                    .config
                    .work_dir
                    .join(TICKET_ID)
                    .join("review-disposition.json")
            ),
            ReviewDisposition::Block {
                remedy_owner: RemedyOwner::Operator,
                ask,
                unstructured: false,
                ..
            } if ask == HISTORY_IDENTITY_ASK
        ));

        let journal_body = std::fs::read_to_string(&journal).unwrap();
        assert_eq!(
            journal_body
                .matches("\"state\":\"failure-observed\"")
                .count(),
            2
        );
        assert!(journal_body.contains("\"failure_count\":1,\"failure_limit\":2"));
        assert!(journal_body.contains("\"consequence\":\"retry-scheduled\""));
        assert!(journal_body.contains("\"failure_count\":2,\"failure_limit\":2"));
        assert!(journal_body.contains("\"consequence\":\"park\""));
        let records = read_mixed_ledger(&ledger);
        assert_eq!(records.len(), 1);
        let ProvenanceLedgerRecord::ParkingTransition(park) = &records[0] else {
            panic!("expected exactly one Park provenance row")
        };
        assert_eq!(park.record_type, ParkingTransitionType::Park);
        assert_eq!(park.retry_count, Some(2));
        assert_eq!(park.retry_limit, Some(2));

        state.reconcile_review_completions();
        assert_eq!(
            state.launched_completion_effects.len(),
            2,
            "blocked replay must not launch a third completion command"
        );
        assert!(!dir.path().join(".lisa-commit.lock").exists());
    }

    #[test]
    fn history_and_identity_failures_retry_to_bound_then_park_and_unpark() {
        for (ticket_id, failure) in [
            (
                "T-UNBORN",
                "fatal: your current branch 'main' does not have any commits yet",
            ),
            ("T-IDENTITY", "Author identity unknown"),
        ] {
            let (mut state, lease, _dir, journal, ledger) = completion_failure_fixture(ticket_id);
            assert!(state.dispatch_completion(CompletionInput::Reconcile {
                ticket_id: ticket_id.to_string(),
                source_lease: lease,
            }));

            state.handle_completion_result(
                ticket_id,
                Some(1),
                Vec::new(),
                failure.as_bytes().to_vec(),
            );
            assert!(state.pending_completions.contains_key(ticket_id));
            assert_eq!(state.completion_aggregates[ticket_id].failure_count(), 1);
            assert!(state.threads.contains_key(ticket_id));

            state.handle_completion_result(
                ticket_id,
                Some(1),
                Vec::new(),
                failure.as_bytes().to_vec(),
            );
            assert!(!state.pending_completions.contains_key(ticket_id));
            assert!(!state.threads.contains_key(ticket_id));
            assert!(state.agent_slots[0].ticket_id.is_none());
            let ticket = ticket::scan_tickets(&state.config.ticket_dir)
                .unwrap()
                .into_iter()
                .find(|ticket| ticket.id == ticket_id)
                .unwrap();
            assert_eq!(ticket.phase, Phase::Review);
            assert_eq!(ticket.status, TicketStatus::Blocked);
            assert!(matches!(
                parse_review_disposition(
                    state
                        .config
                        .work_dir
                        .join(ticket_id)
                        .join("review-disposition.json")
                ),
                ReviewDisposition::Block {
                    reason,
                    remedy_owner: RemedyOwner::Operator,
                    ask,
                    unstructured: false,
                    ..
                } if reason.contains(failure) && ask == HISTORY_IDENTITY_ASK
            ));
            let body = std::fs::read_to_string(&journal).unwrap();
            assert_eq!(body.matches("\"state\":\"failure-observed\"").count(), 2);
            assert!(body.contains("\"failure_count\":1,\"failure_limit\":2"));
            assert!(body.contains("\"failure_count\":2,\"failure_limit\":2"));
            assert_eq!(body.matches(failure).count(), 3);

            let records = read_mixed_ledger(&ledger);
            let ProvenanceLedgerRecord::ParkingTransition(park) = &records[0] else {
                panic!("expected Park provenance")
            };
            assert_eq!(park.record_type, ParkingTransitionType::Park);
            assert_eq!(park.retry_count, Some(2));
            assert_eq!(park.retry_limit, Some(2));

            ticket::update_ticket_status(&ticket.file_path, TicketStatus::Open).unwrap();
            state.rebuild_dag();
            assert_eq!(
                state.reconciliation_state(ticket_id),
                CompletionState::Eligible
            );
            state.reconcile_unpark_transitions();
            let records = read_mixed_ledger(&ledger);
            let ProvenanceLedgerRecord::ParkingTransition(unpark) = &records[1] else {
                panic!("expected Unpark provenance")
            };
            assert_eq!(unpark.record_type, ParkingTransitionType::Unpark);
            assert_eq!(unpark.retry_count, Some(2));
            assert_eq!(unpark.retry_limit, Some(2));
        }
    }

    #[test]
    fn auto_pinned_commit_with_mid_run_repository_loss_parks_without_journal_seal() {
        const TICKET_ID: &str = "T-PINNED-COMMIT";
        let (mut state, lease, dir, journal, ledger) = completion_failure_fixture(TICKET_ID);
        let resolution = lisa_core::completion::resolve_completion_seal(
            lisa_core::completion::CompletionSealMode::Auto,
            lisa_core::completion::CommitSealSupport::Available,
        )
        .unwrap();
        assert_eq!(resolution.seal(), CompletionSeal::Commit);
        state.config.completion_seal = resolution.seal();

        let git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        git(&["init", "--quiet"]);
        git(&["config", "user.name", "Pinned Commit Fixture"]);
        git(&["config", "user.email", "pinned-commit@example.invalid"]);
        git(&["commit", "--quiet", "--allow-empty", "-m", "fixture root"]);

        assert!(state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: TICKET_ID.to_string(),
            source_lease: lease,
        }));
        assert_eq!(state.config.completion_seal, CompletionSeal::Commit);
        assert_eq!(state.launched_completion_effects.len(), 1);
        let completion_key = state.pending_completions[TICKET_ID].completion_key.clone();

        let git_dir = dir.path().join(".git");
        assert!(git_dir.is_dir());
        std::fs::remove_dir_all(&git_dir).unwrap();

        let ticket_file = state.config.ticket_dir.join(format!("{TICKET_ID}.md"));
        let error = lisa_cli::commit_transaction::complete_ticket(
            lisa_cli::commit_transaction::CompleteTicketRequest {
                repo_root: dir.path().to_path_buf(),
                ticket_id: TICKET_ID.to_string(),
                message: format!("Complete {TICKET_ID}"),
                ticket_file: ticket_file.strip_prefix(dir.path()).unwrap().to_path_buf(),
                work_dir: state
                    .config
                    .work_dir
                    .join(TICKET_ID)
                    .strip_prefix(dir.path())
                    .unwrap()
                    .to_path_buf(),
                completion_key,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("discover repository root"), "{error}");
        state.handle_completion_result(
            TICKET_ID,
            Some(1),
            Vec::new(),
            format!("Error: {error}").into_bytes(),
        );

        assert_eq!(state.config.completion_seal, CompletionSeal::Commit);
        assert!(!state.pending_completions.contains_key(TICKET_ID));
        assert!(!state.threads.contains_key(TICKET_ID));
        assert!(state.agent_slots[0].ticket_id.is_none());
        let ticket = ticket::scan_tickets(&state.config.ticket_dir)
            .unwrap()
            .into_iter()
            .find(|ticket| ticket.id == TICKET_ID)
            .unwrap();
        assert_eq!(ticket.phase, Phase::Review);
        assert_eq!(ticket.status, TicketStatus::Blocked);
        assert!(matches!(
            parse_review_disposition(
                state
                    .config
                    .work_dir
                    .join(TICKET_ID)
                    .join("review-disposition.json")
            ),
            ReviewDisposition::Block {
                reason,
                ask,
                remedy_owner: RemedyOwner::Operator,
                unstructured: true,
                ..
            } if !ask.trim().is_empty()
                && ask == reason
                && reason.contains("discover repository root")
        ));

        let journal_body = std::fs::read_to_string(journal).unwrap();
        assert!(journal_body.contains("\"state\":\"failure-observed\""));
        assert!(journal_body.contains("\"state\":\"rejected\""));
        assert!(journal_body.contains("\"retryability\":\"action-required\""));
        assert!(journal_body.contains("discover repository root"));
        assert!(journal_body
            .lines()
            .all(|line| line.contains("\"seal\":\"commit\"")));
        assert!(!journal_body.contains("\"seal\":\"journal\""));
        assert!(!journal_body.contains("\"state\":\"confirmed\""));
        assert!(!journal_body.contains("\"content_hashes\""));
        assert!(!journal_body.contains("\"commit_id\""));

        let records = read_mixed_ledger(&ledger);
        let ProvenanceLedgerRecord::ParkingTransition(park) = &records[0] else {
            panic!("expected Park provenance")
        };
        assert_eq!(park.record_type, ParkingTransitionType::Park);
        assert_eq!(park.seal, CompletionSeal::Commit);
    }

    #[test]
    fn transient_completion_failure_exhausts_launches_without_immediate_park() {
        const TICKET_ID: &str = "T-CONTENTION";
        let (mut state, lease, _dir, journal, ledger) = completion_failure_fixture(TICKET_ID);
        assert!(state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: TICKET_ID.to_string(),
            source_lease: lease.clone(),
        }));
        let failure = b"Unable to create .git/index.lock: another git process seems to be running";

        state.handle_completion_result(TICKET_ID, Some(1), Vec::new(), failure.to_vec());
        assert!(state.pending_completions.contains_key(TICKET_ID));
        assert_eq!(state.launched_completion_effects.len(), 2);
        state.handle_completion_result(TICKET_ID, Some(1), Vec::new(), failure.to_vec());

        assert!(!state.pending_completions.contains_key(TICKET_ID));
        assert!(state.threads.contains_key(TICKET_ID));
        assert!(matches!(
            state.completion_aggregates[TICKET_ID].state(),
            CompletionState::CommandInFlight { .. }
        ));
        assert!(state.completion_aggregates[TICKET_ID].retries_exhausted());
        assert_eq!(state.launched_completion_effects.len(), 2);
        assert!(!ledger.exists());
        assert!(!state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: TICKET_ID.to_string(),
            source_lease: lease,
        }));
        assert_eq!(state.launched_completion_effects.len(), 2);
        let body = std::fs::read_to_string(journal).unwrap();
        assert!(body.contains("\"consequence\":\"retry-exhausted\""));
        assert!(!body.contains("\"state\":\"rejected\""));
    }

    #[test]
    fn unrecognized_completion_failure_parks_with_raw_unstructured_ask() {
        const TICKET_ID: &str = "T-UNKNOWN-GIT";
        let (mut state, lease, _dir, journal, ledger) = completion_failure_fixture(TICKET_ID);
        assert!(state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: TICKET_ID.to_string(),
            source_lease: lease,
        }));
        state.handle_completion_result(
            TICKET_ID,
            Some(1),
            Vec::new(),
            b"fatal: a surprising ref failure".to_vec(),
        );

        assert!(!state.threads.contains_key(TICKET_ID));
        let disposition = parse_review_disposition(
            state
                .config
                .work_dir
                .join(TICKET_ID)
                .join("review-disposition.json"),
        );
        assert!(matches!(
            disposition,
            ReviewDisposition::Block {
                reason,
                ask,
                remedy_owner: RemedyOwner::Operator,
                unstructured: true,
                ..
            } if ask == reason && reason.contains("fatal: a surprising ref failure")
        ));
        let body = std::fs::read_to_string(journal).unwrap();
        assert_eq!(body.matches("\"state\":\"failure-observed\"").count(), 1);
        assert!(body.contains("\"class\":\"unrecognized\""));
        let records = read_mixed_ledger(&ledger);
        let ProvenanceLedgerRecord::ParkingTransition(park) = &records[0] else {
            panic!("expected Park provenance")
        };
        assert_eq!(park.record_type, ParkingTransitionType::Park);
        assert_eq!(park.retry_count, Some(1));
        assert_eq!(park.retry_limit, Some(2));
    }

    #[test]
    fn test_check_review_timeouts_idempotent() {
        use lisa_core::types::Thread;

        let mut state = State {
            config: PluginConfig {
                review_timeout_secs: 10,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-001".to_string(), thread);

        state.check_review_timeouts();
        let log_count = state.activity_log.len();

        state.check_review_timeouts();
        // No new events — already in finish_up_sent
        assert_eq!(state.activity_log.len(), log_count);
    }

    #[test]
    fn test_check_review_timeouts_not_yet_timed_out() {
        use lisa_core::types::Thread;

        let mut state = State {
            config: PluginConfig {
                review_timeout_secs: 300,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Running Review thread that just entered Review (within timeout)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        // last_phase_change is now (default from Thread::new)
        state.threads.insert("T-001".to_string(), thread);

        state.check_review_timeouts();

        // Thread should still be Running, no prompt sent
        let t = state.threads.get("T-001").unwrap();
        assert_eq!(t.status, lisa_core::types::ThreadStatus::Running);
        assert!(state.finish_up_sent.is_empty());
    }

    #[test]
    fn test_check_review_timeouts_disabled_when_zero() {
        use lisa_core::types::Thread;

        let mut state = State {
            config: PluginConfig {
                review_timeout_secs: 0,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(600);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-001".to_string(), thread);

        state.check_review_timeouts();

        // Thread should still be Running (feature disabled)
        let t = state.threads.get("T-001").unwrap();
        assert_eq!(t.status, lisa_core::types::ThreadStatus::Running);
        assert!(state.finish_up_sent.is_empty());
    }

    #[test]
    fn test_check_review_timeouts_only_running_review() {
        use lisa_core::types::Thread;

        let mut state = State {
            config: PluginConfig {
                review_timeout_secs: 10,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Running Implement thread (wrong phase — should not be affected)
        let mut t1 = Thread::new("T-001", 1);
        t1.current_phase = Phase::Implement;
        t1.last_phase_change = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        t1.last_activity = t1.last_phase_change;
        state.threads.insert("T-001".to_string(), t1);

        // Parked Review thread (not Running — should not be affected)
        let mut t2 = Thread::new("T-002", 2);
        t2.current_phase = Phase::Review;
        t2.park();
        t2.last_phase_change = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        t2.last_activity = t2.last_phase_change;
        state.threads.insert("T-002".to_string(), t2);

        // Completed Review thread (should not be affected)
        let mut t3 = Thread::new("T-003", 3);
        t3.current_phase = Phase::Review;
        t3.complete();
        t3.last_phase_change = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        t3.last_activity = t3.last_phase_change;
        state.threads.insert("T-003".to_string(), t3);

        state.check_review_timeouts();

        // None should be prompted — wrong phase, wrong status
        assert!(state.finish_up_sent.is_empty());
        assert!(state.activity_log.is_empty());
    }

    #[test]
    fn test_check_session_timeouts_expired() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: timeout-test\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
        ).unwrap();
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();

        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: dir.path().join("work"),
                session_timeout_secs: 1800, // 30 minutes
                stuck_threshold_secs: 600,  // hard-silence bar = 2x = 1200s
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };

        let ticket_id = "T-001".to_string();
        let first = AttemptLease::mint(ticket_id.clone(), None).unwrap();
        state
            .lease_high_water
            .insert(ticket_id.clone(), first.clone());
        state
            .current_leases
            .insert(ticket_id.clone(), first.clone());

        // Create a thread that started 31+ minutes ago (past 1800s timeout)
        // and has been silent the whole time (past the hard-silence bar),
        // so it is reclaimable
        let mut thread = Thread::new("T-001", 1);
        thread.attempt_lease = Some(first.clone());
        thread.current_phase = Phase::Implement;
        thread.started_at = std::time::SystemTime::now() - std::time::Duration::from_secs(31 * 60);
        thread.last_activity = thread.started_at;
        state.threads.insert("T-001".to_string(), thread);

        // Add an agent slot
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: Some(first.clone()),
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: Some(AgentClient::Claude),
        });
        // The fenced pane is never reusable; a successor can use this separate
        // eligible pane instead.
        state.agent_slots.push(fresh_slot(2, None));

        let outcomes = state.check_session_timeouts();

        assert_eq!(
            outcomes,
            vec![FailureTransitionOutcome::SessionTimedOut {
                pane_id: 1,
                ticket_id: ticket_id.clone(),
                fenced: true,
            }]
        );

        assert_eq!(
            state.attempt_lifecycle,
            vec![
                AttemptLifecycleEvent::LeaseRevoked {
                    ticket_id: ticket_id.clone(),
                },
                AttemptLifecycleEvent::PaneFenced {
                    ticket_id: ticket_id.clone(),
                    pane_id: 1,
                },
                AttemptLifecycleEvent::SlotReleased {
                    ticket_id: ticket_id.clone(),
                },
            ],
            "hard-silence teardown must revoke, fence, then release"
        );
        assert_eq!(state.current_leases.get(&ticket_id), None);
        assert!(!first.is_current(state.current_leases.get(&ticket_id)));
        assert_eq!(state.lease_high_water.get(&ticket_id), Some(&first));

        // Thread should be removed
        assert!(state.threads.is_empty());

        // The old slot is released but permanently disqualified.
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert_eq!(state.agent_slots[0].attempt_lease, None);
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Fenced
        );
        assert!(!state.agent_slots[0].has_session);

        // Activity log should have SessionTimedOut event
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::SessionTimedOut { ticket_id, phase, .. }
            if ticket_id == "T-001" && *phase == Phase::Implement
        )));

        // timeout_alerts should be populated
        assert_eq!(state.timeout_alerts.len(), 1);
        assert_eq!(state.timeout_alerts[0].0, "T-001");
        assert_eq!(state.timeout_alerts[0].2, Phase::Implement);
        assert!(
            state.check_session_timeouts().is_empty(),
            "a reclaimed timed-out thread cannot be reclaimed again"
        );

        state.schedule_ready_tickets();

        let second = state.current_leases[&ticket_id].clone();
        assert!(second.attempt_id > first.attempt_id);
        assert_eq!(second.attempt_id, 2);
        assert!(!first.is_current(Some(&second)));
        assert!(second.is_current(state.current_leases.get(&ticket_id)));
        assert_eq!(state.lease_high_water.get(&ticket_id), Some(&second));
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Fenced
        );
        assert_eq!(state.agent_slots[0].ticket_id, None);
        assert_eq!(state.agent_slots[1].ticket_id.as_ref(), Some(&ticket_id));
        assert_eq!(state.agent_slots[1].attempt_lease.as_ref(), Some(&second));
        assert_eq!(
            state.threads[&ticket_id].attempt_lease.as_ref(),
            Some(&second)
        );
    }

    #[test]
    fn test_check_session_timeouts_active_session_deferred() {
        use lisa_core::types::Thread;

        let mut state = State {
            config: PluginConfig {
                session_timeout_secs: 1800,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Over budget (started 31 minutes ago) but still active right now
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        thread.started_at = std::time::SystemTime::now() - std::time::Duration::from_secs(31 * 60);
        thread.record_activity(std::time::SystemTime::now());
        state.threads.insert("T-001".to_string(), thread);

        state.check_session_timeouts();

        // Thread must NOT be reclaimed while active — clean completion wins
        assert!(state.threads.contains_key("T-001"));
        assert!(state.timeout_alerts.is_empty());

        // A single over-budget warning is logged
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Warning { message } if message.contains("still active")
        )));

        // Repeated checks do not spam the warning
        let log_count = state.activity_log.len();
        state.check_session_timeouts();
        assert_eq!(state.activity_log.len(), log_count);
    }

    #[test]
    fn test_check_session_timeouts_slow_test_gap_not_reclaimed() {
        use lisa_core::types::Thread;

        let mut state = State {
            config: PluginConfig {
                session_timeout_secs: 1800,
                stuck_threshold_secs: 600, // hard-silence bar = 2x = 1200s
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // The long-ticket scenario: 75 minutes in, far over the 30-minute
        // budget, and mid-way through a slow test run — silent for 5 minutes
        // (past wind_down, but nowhere near the 20-minute hard-silence bar).
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        thread.started_at = std::time::SystemTime::now() - std::time::Duration::from_secs(75 * 60);
        thread.last_activity = std::time::SystemTime::now() - std::time::Duration::from_secs(300);
        state.threads.insert("T-001".to_string(), thread);

        state.check_session_timeouts();

        // Must NOT be reclaimed — the session is progressing, just slowly
        assert!(state.threads.contains_key("T-001"));
        assert!(state.timeout_alerts.is_empty());
        assert!(state.over_budget_warned.contains("T-001"));

        // But a session silent past the hard bar (20 min) IS reclaimed
        state.threads.get_mut("T-001").unwrap().last_activity =
            std::time::SystemTime::now() - std::time::Duration::from_secs(21 * 60);
        state.check_session_timeouts();
        assert!(state.threads.is_empty());
        assert_eq!(state.timeout_alerts.len(), 1);
    }

    #[test]
    fn test_detect_stale_threads_active_session_not_stale() {
        use lisa_core::types::Thread;

        let mut state = State::default();

        // Phase started 31 minutes ago, but heartbeats prove the session is
        // actively working — long phases are not staleness.
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(31 * 60);
        thread.record_activity(std::time::SystemTime::now());
        state.threads.insert("T-001".to_string(), thread);

        state.detect_stale_threads();

        assert!(state.threads.contains_key("T-001"));
    }

    #[test]
    fn test_check_review_timeouts_skips_active_thread() {
        use lisa_core::types::Thread;

        let mut state = State {
            config: PluginConfig {
                review_timeout_secs: 10,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Past the review timeout, but the session is actively working
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Review;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(200);
        thread.record_activity(std::time::SystemTime::now());
        state.threads.insert("T-001".to_string(), thread);

        state.check_review_timeouts();

        // No finish-up prompt while the agent is busy
        assert!(state.finish_up_sent.is_empty());
    }

    #[test]
    fn test_check_heartbeat_signals_updates_activity() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        let stale = std::time::SystemTime::now() - std::time::Duration::from_secs(700);
        let mut thread = Thread::new("T-001", 1);
        thread.last_activity = stale;
        state.threads.insert("T-001".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-001");
        fs::write(
            signal_dir.join("pane-1.heartbeat"),
            serde_json::to_string(&lease).unwrap(),
        )
        .unwrap();

        state.check_heartbeat_signals();

        // Signal file consumed
        assert!(!signal_dir.join("pane-1.heartbeat").exists());

        // Thread and slot activity clocks refreshed
        assert!(state.threads.get("T-001").unwrap().last_activity > stale);
        assert!(state.agent_slots[0].last_activity_at.is_some());
    }

    #[test]
    fn stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let work_dir = dir.path().join("work");
        let signal_dir = dir.path().join("signals");
        let attempt_dir = dir.path().join("attempts");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(
            tickets_dir.join("T-LEASE.md"),
            "---\nid: T-LEASE\ntitle: lease boundary\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n",
        )
        .unwrap();
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: work_dir.clone(),
                ..PluginConfig::new()
            },
            signal_dir: signal_dir.clone(),
            attempt_dir,
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 2,
            ticket_id: Some("T-LEASE".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: Some(AgentClient::Codex),
        });
        let stale_clock = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10);
        let mut thread = Thread::new("T-LEASE", 2);
        thread.current_phase = Phase::Research;
        thread.last_activity = stale_clock;
        state.threads.insert("T-LEASE".to_string(), thread);

        let predecessor = install_current_attempt(&mut state, "T-LEASE");
        let current = install_current_attempt(&mut state, "T-LEASE");
        assert!(!predecessor.is_current(state.current_leases.get("T-LEASE")));
        assert!(current.is_current(state.current_leases.get("T-LEASE")));

        let stale_stage = state.attempt_work_dir(&predecessor);
        fs::create_dir_all(&stale_stage).unwrap();
        fs::write(stale_stage.join("research.md"), "stale predecessor bytes").unwrap();
        state.awaiting_human.insert(2);
        state.notified_attention.insert(2);
        fs::write(
            signal_dir.join("pane-2.heartbeat"),
            serde_json::to_string(&predecessor).unwrap(),
        )
        .unwrap();

        state.check_heartbeat_signals();
        state.check_artifact_advances();

        assert_eq!(
            state.threads.get("T-LEASE").unwrap().last_activity,
            stale_clock,
            "a predecessor heartbeat must not refresh the replacement"
        );
        assert!(state.agent_slots[0].last_activity_at.is_none());
        assert!(state.awaiting_human.contains(&2));
        assert!(state.notified_attention.contains(&2));
        assert_eq!(
            state.threads.get("T-LEASE").unwrap().current_phase,
            Phase::Research
        );
        assert!(!work_dir.join("T-LEASE/research.md").exists());

        let current_stage = state.attempt_work_dir(&current);
        fs::create_dir_all(&current_stage).unwrap();
        fs::write(current_stage.join("research.md"), "current lease bytes").unwrap();
        fs::write(
            signal_dir.join("pane-2.heartbeat"),
            serde_json::to_string(&current).unwrap(),
        )
        .unwrap();

        state.check_heartbeat_signals();
        state.check_artifact_advances();

        assert!(state.threads.get("T-LEASE").unwrap().last_activity > stale_clock);
        assert!(state.agent_slots[0].last_activity_at.is_some());
        assert!(!state.awaiting_human.contains(&2));
        assert!(!state.notified_attention.contains(&2));
        assert_eq!(
            state.threads.get("T-LEASE").unwrap().current_phase,
            Phase::Design
        );
        assert_eq!(
            fs::read_to_string(work_dir.join("T-LEASE/research.md")).unwrap(),
            "current lease bytes"
        );
        assert_eq!(
            fs::read_to_string(stale_stage.join("research.md")).unwrap(),
            "stale predecessor bytes"
        );
    }

    #[test]
    fn test_find_idle_slot_busy_pane_guard() {
        let mut state = State::default();

        // Released slot whose session showed activity moments ago — not reusable
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: None,
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: Some(std::time::SystemTime::now()),
            last_client: None,
        });

        assert_eq!(state.find_idle_slot(AgentClient::Claude), None);

        // Once the pane has been quiet past the wind-down period, it's eligible
        state.agent_slots[0].last_activity_at = Some(
            std::time::SystemTime::now()
                - std::time::Duration::from_secs(state.config.wind_down_secs + 1),
        );
        assert_eq!(state.find_idle_slot(AgentClient::Claude), Some(0));
    }

    #[test]
    fn test_find_idle_slot_fresh_pane_not_gated() {
        let mut state = State::default();

        // A pane with no session yet is immediately usable regardless of the guard
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: None,
            attempt_lease: None,
            has_session: false,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        assert_eq!(state.find_idle_slot(AgentClient::Claude), Some(0));
    }

    #[test]
    fn test_check_transition_timeouts_deferred_while_pane_active() {
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::WaitingForClear,
            // Far past the 90s clear-signal timeout...
            transition_started_at: Some(
                std::time::SystemTime::now() - std::time::Duration::from_secs(600),
            ),
            cooldown_until: None,
            // ...but the pane is still active, so the fallback must wait
            last_activity_at: Some(std::time::SystemTime::now()),
            last_client: None,
        });

        state.check_transition_timeouts();

        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForClear
        );
        assert!(state.activity_log.is_empty());
    }

    // =========================================================================
    // Deadline policy characterization (T-039-04-01)
    //
    // These tests deliberately exercise the existing policy entry points. They
    // pin each policy's clock, exemptions, and action before the traversal and
    // clock plumbing are centralized by the next ticket.
    // =========================================================================

    #[test]
    fn characterizes_acknowledgement_deadline_clock_and_recovery_action() {
        use lisa_core::types::Thread;

        let deadline = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10_000);
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-ACK".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: Some(AgentClient::Codex),
        });
        state
            .threads
            .insert("T-ACK".to_string(), Thread::new("T-ACK", 1));
        let predecessor = install_current_attempt(&mut state, "T-ACK");
        state.seat_assignments.insert(
            1,
            SeatAssignmentState::AssignedPendingAck {
                generation: predecessor.attempt_id,
                ack_deadline: Some(deadline),
            },
        );
        // Awaiting-human is intentionally not an acknowledgement exemption: the
        // timed-out TUI is abandoned and this marker is cleared during recovery.
        state.awaiting_human.insert(1);

        assert!(state
            .check_assignment_ack_timeouts_at(deadline - std::time::Duration::from_nanos(1))
            .is_empty());
        assert_eq!(
            state.seat_assignment(1),
            Some(SeatAssignmentState::AssignedPendingAck {
                generation: predecessor.attempt_id,
                ack_deadline: Some(deadline),
            })
        );
        assert!(predecessor.is_current(state.current_leases.get("T-ACK")));

        assert!(state.check_assignment_ack_timeouts_at(deadline).is_empty());
        let successor = state.current_leases["T-ACK"].clone();
        assert_eq!(successor.attempt_id, predecessor.attempt_id + 1);
        assert!(!predecessor.is_current(state.current_leases.get("T-ACK")));
        assert_eq!(
            state.seat_assignment(1),
            Some(SeatAssignmentState::Recovering {
                generation: successor.attempt_id,
                ack_deadline: None,
            })
        );
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForExit
        );
        assert!(!state.awaiting_human.contains(&1));
    }

    #[test]
    fn characterizes_transition_deadline_and_active_session_exemption() {
        let now = std::time::SystemTime::now();
        let mut state = State::default();
        state.agent_slots.extend([
            AgentSlot {
                pane_id: 1,
                ticket_id: None,
                attempt_lease: None,
                has_session: true,
                transition_state: TransitionState::WaitingForExit,
                transition_started_at: Some(
                    now - std::time::Duration::from_secs(AGENT_EXIT_GRACE_SECS + 10),
                ),
                cooldown_until: None,
                last_activity_at: Some(now),
                last_client: Some(AgentClient::Codex),
            },
            AgentSlot {
                pane_id: 2,
                ticket_id: None,
                attempt_lease: None,
                has_session: true,
                transition_state: TransitionState::WaitingForExit,
                transition_started_at: Some(
                    now - std::time::Duration::from_secs(AGENT_EXIT_GRACE_SECS - 1),
                ),
                cooldown_until: None,
                last_activity_at: None,
                last_client: Some(AgentClient::Codex),
            },
            AgentSlot {
                pane_id: 3,
                ticket_id: Some("T-ACTIVE".to_string()),
                attempt_lease: None,
                has_session: true,
                transition_state: TransitionState::WaitingForClear,
                transition_started_at: Some(
                    now - std::time::Duration::from_secs(CLEAR_SIGNAL_TIMEOUT_SECS + 10),
                ),
                cooldown_until: None,
                last_activity_at: Some(now),
                last_client: Some(AgentClient::Codex),
            },
            AgentSlot {
                pane_id: 4,
                ticket_id: Some("T-HUMAN".to_string()),
                attempt_lease: None,
                has_session: true,
                transition_state: TransitionState::WaitingForClear,
                transition_started_at: Some(
                    now - std::time::Duration::from_secs(CLEAR_SIGNAL_TIMEOUT_SECS + 10),
                ),
                cooldown_until: None,
                last_activity_at: Some(
                    now - std::time::Duration::from_secs(state.config.wind_down_secs + 10),
                ),
                last_client: Some(AgentClient::Codex),
            },
        ]);
        state.awaiting_human.insert(4);

        state.check_transition_timeouts();

        // Exit uses only transition_started_at: recent pane activity is not an
        // exemption once the grace deadline has elapsed.
        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
        assert!(!state.agent_slots[0].has_session);
        assert_eq!(state.agent_slots[0].last_client, None);
        assert_eq!(
            state.agent_slots[1].transition_state,
            TransitionState::WaitingForExit
        );
        // Stop/clear use last_activity_at as an independent busy-pane guard.
        assert_eq!(
            state.agent_slots[2].transition_state,
            TransitionState::WaitingForClear
        );
        // A quiet pane is independently exempt while its question is awaiting
        // a human answer.
        assert_eq!(
            state.agent_slots[3].transition_state,
            TransitionState::WaitingForClear
        );
        assert!(state.awaiting_human.contains(&4));
    }

    #[test]
    fn characterizes_review_deadline_exemptions_and_finish_up_action() {
        use lisa_core::types::{Thread, ThreadStatus};

        let now = std::time::SystemTime::now();
        let mut state = State {
            config: PluginConfig {
                review_timeout_secs: 10,
                wind_down_secs: 20,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        for (ticket_id, pane_id, last_activity) in [
            ("T-REVIEW-ACTIVE", 1, now),
            (
                "T-REVIEW-HUMAN",
                2,
                now - std::time::Duration::from_secs(30),
            ),
            ("T-REVIEW-FIRE", 3, now - std::time::Duration::from_secs(30)),
        ] {
            let mut thread = Thread::new(ticket_id, pane_id);
            thread.current_phase = Phase::Review;
            // Review expiry is measured from the phase-change clock.
            thread.last_phase_change = now - std::time::Duration::from_secs(30);
            thread.last_activity = last_activity;
            state.threads.insert(ticket_id.to_string(), thread);
        }
        state.awaiting_human.insert(2);

        state.check_review_timeouts();

        assert!(!state.finish_up_sent.contains("T-REVIEW-ACTIVE"));
        assert!(!state.finish_up_sent.contains("T-REVIEW-HUMAN"));
        assert!(state.finish_up_sent.contains("T-REVIEW-FIRE"));
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::FinishUpPromptSent { ticket_id, pane_id }
                if ticket_id == "T-REVIEW-FIRE" && *pane_id == 3
        )));
        assert_eq!(
            state.threads["T-REVIEW-ACTIVE"].status,
            ThreadStatus::Running
        );
        assert_eq!(
            state.threads["T-REVIEW-HUMAN"].status,
            ThreadStatus::Running
        );
        assert!(state.awaiting_human.contains(&2));
    }

    #[test]
    fn characterizes_health_deadline_as_observational_for_awaiting_human() {
        use lisa_core::types::{HealthStatus, Thread};

        let now = std::time::SystemTime::now();
        let mut state = State {
            config: PluginConfig {
                stuck_threshold_secs: 10,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        let mut thread = Thread::new("T-HEALTH", 1);
        // Health is measured from last_activity, not the phase clock.
        thread.last_activity = now - std::time::Duration::from_secs(20);
        state.threads.insert("T-HEALTH".to_string(), thread);
        state.awaiting_human.insert(1);

        state.evaluate_health();

        assert_eq!(
            state.last_health.get("T-HEALTH"),
            Some(&HealthStatus::Stuck)
        );
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::HealthStateChanged {
                ticket_id,
                old_health: HealthStatus::Healthy,
                new_health: HealthStatus::Stuck,
            } if ticket_id == "T-HEALTH"
        )));
        assert!(state.threads.contains_key("T-HEALTH"));
        assert!(state.awaiting_human.contains(&1));
    }

    #[test]
    fn characterizes_session_deadline_exemptions_and_timeout_action() {
        use lisa_core::types::Thread;

        let now = std::time::SystemTime::now();
        let mut state = State {
            config: PluginConfig {
                session_timeout_secs: 100,
                stuck_threshold_secs: 10,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        for (ticket_id, pane_id, last_activity) in [
            ("T-SESSION-ACTIVE", 1, now),
            (
                "T-SESSION-HUMAN",
                2,
                now - std::time::Duration::from_secs(30),
            ),
            (
                "T-SESSION-FIRE",
                3,
                now - std::time::Duration::from_secs(30),
            ),
        ] {
            let mut thread = Thread::new(ticket_id, pane_id);
            // Global budget uses started_at; destructive action additionally
            // requires hard silence measured from last_activity.
            thread.started_at = now - std::time::Duration::from_secs(200);
            thread.last_activity = last_activity;
            state.threads.insert(ticket_id.to_string(), thread);
        }
        state.awaiting_human.insert(2);
        state.agent_slots.push(AgentSlot {
            pane_id: 3,
            ticket_id: Some("T-SESSION-FIRE".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: Some(AgentClient::Codex),
        });
        install_current_attempt(&mut state, "T-SESSION-FIRE");

        let outcomes = state.check_session_timeouts();

        assert_eq!(outcomes.len(), 1);
        assert!(matches!(
            &outcomes[0],
            FailureTransitionOutcome::SessionTimedOut {
                pane_id: 3,
                ticket_id,
                fenced: true,
            } if ticket_id == "T-SESSION-FIRE"
        ));
        assert!(state.threads.contains_key("T-SESSION-ACTIVE"));
        assert!(state.threads.contains_key("T-SESSION-HUMAN"));
        assert!(!state.threads.contains_key("T-SESSION-FIRE"));
        assert!(state.over_budget_warned.contains("T-SESSION-ACTIVE"));
        assert!(state.over_budget_warned.contains("T-SESSION-HUMAN"));
        assert!(state.awaiting_human.contains(&2));
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Fenced
        );
        assert!(!state.current_leases.contains_key("T-SESSION-FIRE"));
    }

    #[test]
    fn characterizes_stale_deadline_exemptions_and_reclaim_action() {
        use lisa_core::types::Thread;

        let now = std::time::SystemTime::now();
        let mut state = State {
            config: PluginConfig {
                stuck_threshold_secs: 10,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        let mut active = Thread::new("T-STALE-ACTIVE", 1);
        active.last_phase_change = now - std::time::Duration::from_secs(1_000);
        active.last_activity = now;
        state.threads.insert("T-STALE-ACTIVE".to_string(), active);

        let mut awaiting = Thread::new("T-STALE-HUMAN", 2);
        awaiting.last_activity = now - std::time::Duration::from_secs(30);
        state.threads.insert("T-STALE-HUMAN".to_string(), awaiting);
        state.awaiting_human.insert(2);

        let mut reclaimable = Thread::new("T-STALE-FIRE", 3);
        reclaimable.last_activity = now - std::time::Duration::from_secs(30);
        state
            .threads
            .insert("T-STALE-FIRE".to_string(), reclaimable);
        state.agent_slots.push(AgentSlot {
            pane_id: 3,
            ticket_id: Some("T-STALE-FIRE".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: Some(AgentClient::Codex),
        });
        install_current_attempt(&mut state, "T-STALE-FIRE");

        let outcomes = state.detect_stale_threads();

        assert_eq!(
            outcomes,
            vec![FailureTransitionOutcome::StaleThreadReclaimed {
                pane_id: 3,
                ticket_id: "T-STALE-FIRE".to_string(),
                fenced: true,
            }]
        );
        assert!(state.threads.contains_key("T-STALE-ACTIVE"));
        assert!(state.threads.contains_key("T-STALE-HUMAN"));
        assert!(!state.threads.contains_key("T-STALE-FIRE"));
        assert!(state.awaiting_human.contains(&2));
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Fenced
        );
        assert!(!state.current_leases.contains_key("T-STALE-FIRE"));
    }

    #[test]
    fn test_check_session_timeouts_not_expired() {
        use lisa_core::types::{Thread, ThreadStatus};

        let mut state = State {
            config: PluginConfig {
                session_timeout_secs: 1800,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Create a thread that started 5 minutes ago (well within 1800s timeout)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.started_at = std::time::SystemTime::now() - std::time::Duration::from_secs(5 * 60);
        state.threads.insert("T-001".to_string(), thread);

        state.check_session_timeouts();

        // Thread should still be running
        assert_eq!(state.threads.len(), 1);
        assert_eq!(
            state.threads.get("T-001").unwrap().status,
            ThreadStatus::Running
        );
        assert!(state.timeout_alerts.is_empty());
        assert!(state.activity_log.is_empty());
    }

    #[test]
    fn test_check_session_timeouts_disabled() {
        use lisa_core::types::Thread;

        let mut state = State {
            config: PluginConfig {
                session_timeout_secs: 0, // disabled
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Create a thread that started 2 hours ago
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.started_at =
            std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 60 * 60);
        state.threads.insert("T-001".to_string(), thread);

        state.check_session_timeouts();

        // Thread should still exist — timeout is disabled
        assert_eq!(state.threads.len(), 1);
        assert!(state.timeout_alerts.is_empty());
        assert!(state.activity_log.is_empty());
    }

    #[test]
    fn test_session_timed_out_event_to_ui() {
        let event = ActivityEvent::SessionTimedOut {
            ticket_id: "T-024-01".to_string(),
            elapsed_secs: 1920, // 32 minutes
            phase: Phase::Implement,
        };
        let entry = activity_event_to_ui_entry(&event).unwrap();
        match &entry.activity {
            ui::ActivityType::Warning { ticket_id, message } => {
                assert_eq!(ticket_id, "T-024-01");
                assert!(message.contains("32m"));
                assert!(message.contains("implement"));
            }
            other => panic!("Expected Warning, got {:?}", other),
        }
    }

    #[test]
    fn test_per_phase_timeout_triggers() {
        use lisa_core::types::Thread;
        use std::collections::HashMap;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: phase-timeout\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n\nBody\n",
        ).unwrap();

        let mut phase_timeouts = HashMap::new();
        phase_timeouts.insert(Phase::Research, 300); // 5 minutes

        let mut state = State {
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                session_timeout_secs: 1800, // 30 min global
                stuck_threshold_secs: 150,  // hard-silence bar = 2x = 300s
                phase_timeouts,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Thread started 10 min ago, phase change was 6 min ago (exceeds the
        // 300s phase timeout) and silent since (exceeds the 300s hard-silence
        // bar), so it is reclaimable
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.started_at = std::time::SystemTime::now() - std::time::Duration::from_secs(10 * 60);
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(6 * 60);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-001".to_string(), thread);

        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.check_session_timeouts();

        // Should be timed out by per-phase timeout (not global — only 10 min < 30 min)
        assert!(state.threads.is_empty());
        assert_eq!(state.timeout_alerts.len(), 1);
        assert_eq!(state.timeout_alerts[0].2, Phase::Research);
    }

    #[test]
    fn test_per_phase_timeout_not_triggered_within_limit() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::collections::HashMap;

        let mut phase_timeouts = HashMap::new();
        phase_timeouts.insert(Phase::Research, 300);

        let mut state = State {
            config: PluginConfig {
                session_timeout_secs: 1800,
                phase_timeouts,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Thread in research phase for 4 minutes (within 300s limit)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(4 * 60);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-001".to_string(), thread);

        state.check_session_timeouts();

        assert_eq!(state.threads.len(), 1);
        assert_eq!(
            state.threads.get("T-001").unwrap().status,
            ThreadStatus::Running
        );
        assert!(state.timeout_alerts.is_empty());
    }

    #[test]
    fn test_per_phase_timeout_fallback_to_global() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::collections::HashMap;

        // Only set per-phase timeout for research, not implement
        let mut phase_timeouts = HashMap::new();
        phase_timeouts.insert(Phase::Research, 300);

        let mut state = State {
            config: PluginConfig {
                session_timeout_secs: 1800,
                phase_timeouts,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Thread in implement phase for 10 minutes — no per-phase override,
        // falls back to global session_timeout_secs (1800s) which hasn't elapsed
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(10 * 60);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-001".to_string(), thread);

        state.check_session_timeouts();

        // Should still be running (fallback timeout is 1800s, only 600s elapsed)
        assert_eq!(state.threads.len(), 1);
        assert_eq!(
            state.threads.get("T-001").unwrap().status,
            ThreadStatus::Running
        );
    }

    #[test]
    fn test_global_timeout_still_enforced_with_phase_timeouts() {
        use lisa_core::types::Thread;
        use std::collections::HashMap;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::write(
            tickets_dir.join("T-001.md"),
            "---\nid: T-001\ntitle: global-timeout\ntype: task\nstatus: open\npriority: high\nphase: implement\n---\n\nBody\n",
        ).unwrap();

        let mut phase_timeouts = HashMap::new();
        phase_timeouts.insert(Phase::Implement, 3600); // 1 hour per-phase (generous)

        let mut state = State {
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                session_timeout_secs: 1800, // 30 min global cap
                stuck_threshold_secs: 600,  // hard-silence bar = 2x = 1200s
                phase_timeouts,
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Thread started 35 minutes ago, but phase change was 20 min ago
        // Global timeout (1800s) exceeded, even though per-phase (3600s) is not;
        // 20 min of silence also clears the 1200s hard-silence bar
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        thread.started_at = std::time::SystemTime::now() - std::time::Duration::from_secs(35 * 60);
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(20 * 60);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-001".to_string(), thread);

        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.check_session_timeouts();

        // Should be timed out by global timeout
        assert!(state.threads.is_empty());
        assert_eq!(state.timeout_alerts.len(), 1);
    }

    #[test]
    fn test_to_ui_state_includes_timeout_alerts() {
        let mut state = State {
            initialized: true,
            ..State::default()
        };
        state.timeout_alerts.push((
            "T-001".to_string(),
            1920, // 32 minutes
            Phase::Implement,
        ));

        let ui_state = state.to_ui_state();

        assert_eq!(ui_state.alerts.len(), 1);
        assert_eq!(ui_state.alerts[0].ticket_id, "T-001");
        assert_eq!(ui_state.alerts[0].alert_type, ui::AlertType::TimedOut);
        assert!(ui_state.alerts[0].detail.contains("32m"));
    }

    #[test]
    fn test_check_error_signals_fails_running_thread() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-1.error"), "turn.failed: boom").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
        let thread = Thread::new("T-001", 1);
        assert_eq!(thread.status, ThreadStatus::Running);
        state.threads.insert("T-001".to_string(), thread);

        let outcomes = state.check_error_signals();

        assert_eq!(
            outcomes,
            vec![FailureTransitionOutcome::ErrorReclaimed {
                pane_id: 1,
                ticket_id: "T-001".to_string(),
            }]
        );

        // Signal consumed
        assert!(!signal_dir.join("pane-1.error").exists());
        // Thread removed (re-schedulable for retry)
        assert!(state.threads.is_empty());
        // Slot released but session kept alive
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert!(state.agent_slots[0].has_session);
        // Alert surfaced
        assert_eq!(state.error_alerts.len(), 1);
        assert_eq!(state.error_alerts[0], ("T-001".to_string(), 1));
        // Error logged
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Error { message } if message.contains("T-001") && message.contains("error")
        )));
        assert!(
            state.check_error_signals().is_empty(),
            "a consumed error signal cannot reclaim the thread again"
        );
    }

    #[test]
    fn test_check_error_signals_idle_pane_noop() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        // Error for pane 9, but the only running thread is on pane 1.
        fs::write(signal_dir.join("pane-9.error"), "").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state
            .threads
            .insert("T-001".to_string(), Thread::new("T-001", 1));

        state.check_error_signals();

        // Signal consumed even though it matched no running thread
        assert!(!signal_dir.join("pane-9.error").exists());
        // No state change
        assert!(state.threads.contains_key("T-001"));
        assert!(state.error_alerts.is_empty());
        // Harmless info logged
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Info { message } if message.contains("pane 9") && message.contains("no running thread")
        )));
    }

    #[test]
    fn test_to_ui_state_includes_error_alerts() {
        let mut state = State {
            initialized: true,
            ..State::default()
        };
        state.error_alerts.push(("T-001".to_string(), 3));

        let ui_state = state.to_ui_state();

        assert_eq!(ui_state.alerts.len(), 1);
        assert_eq!(ui_state.alerts[0].ticket_id, "T-001");
        assert_eq!(ui_state.alerts[0].alert_type, ui::AlertType::Failed);
        assert!(ui_state.alerts[0].detail.contains("pane 3"));
    }

    // --- T-024-01: Codex loop parity ----------------------------------------
    //
    // Composition tests: drive the real scheduler consumers under
    // `client = Codex` with Codex-shaped signal files / artifacts, proving the
    // parity mechanisms (already unit-tested in isolation by T-022-02 / T-023-01
    // / T-023-02) behave correctly *together* as a Codex loop lifecycle. The
    // scheduler consumes signal *files*, never JSON, so the whole scheduler side
    // is reachable natively; the live `codex exec` spawn/stream is the manual
    // remainder covered by `validate-codex-loop.sh`.

    /// Build a `State` configured for a Codex loop, with a real 2-ticket DAG on
    /// disk (`T-CDX-01`; `T-CDX-02` depends on it) and tempdir work/signal dirs.
    /// Returns (state, tempdir) — keep the tempdir alive for the test's duration.
    fn codex_state_with_dag() -> (State, tempfile::TempDir) {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let work_dir = dir.path().join("work");
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::create_dir_all(&work_dir).unwrap();
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(
            tickets_dir.join("T-CDX-01.md"),
            "---\nid: T-CDX-01\ntitle: codex-a\ntype: task\nstatus: open\npriority: high\nphase: research\n---\n\nBody\n",
        ).unwrap();
        fs::write(
            tickets_dir.join("T-CDX-02.md"),
            "---\nid: T-CDX-02\ntitle: codex-b\ntype: task\nstatus: open\npriority: high\nphase: research\ndepends_on: [T-CDX-01]\n---\n\nBody\n",
        ).unwrap();
        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let dag = Dag::from_tickets(tickets).unwrap();
        let state = State {
            dag,
            config: PluginConfig {
                ticket_dir: tickets_dir,
                work_dir,
                client: AgentClient::Codex,
                ..PluginConfig::new()
            },
            signal_dir,
            ..State::default()
        };
        (state, dir)
    }

    fn codex_slot(state: &mut State, pane_id: u32, ticket: &str) {
        state.agent_slots.push(AgentSlot {
            pane_id,
            ticket_id: Some(ticket.to_string()),
            attempt_lease: None,
            has_session: true,
            // A running/ready native TUI sits Idle. When the slot is reassigned,
            // scheduling moves it through WaitingForClear before the next prompt.
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });
    }

    /// AC: phases advance on artifacts through all RDSPI phases — purely on
    /// artifact presence, with *no* `.idle`/`.stopped` signal involved. This is
    /// the parity load-bearer for Codex (which emits no `.idle`): advancement
    /// rides `check_artifact_advances`.
    #[test]
    fn test_codex_dag_advances_all_phases_via_artifacts() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, _dir) = codex_state_with_dag();
        let mut thread = Thread::new("T-CDX-01", 1);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-CDX-01".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-CDX-01");
        let ticket_work = state.attempt_work_dir(&lease);
        fs::create_dir_all(&ticket_work).unwrap();

        // Each artifact advances exactly one phase boundary. Implement→Review
        // and Review→Done both ride review.md, so writing it cascades to Done in
        // a single fixpoint pass — the full RDSPI walk.
        let steps: &[(&str, Phase)] = &[
            ("research.md", Phase::Design),
            ("design.md", Phase::Structure),
            ("structure.md", Phase::Plan),
            ("plan.md", Phase::Implement),
        ];
        for (artifact, expected) in steps {
            fs::write(ticket_work.join(artifact), "x").unwrap();
            state.check_artifact_advances();
            assert_eq!(
                state.threads.get("T-CDX-01").unwrap().current_phase,
                *expected,
                "writing {artifact} should advance to {expected:?}"
            );
        }

        // review.md reaches Review and starts commit-gated completion.
        fs::write(ticket_work.join("review.md"), "x").unwrap();
        write_passing_review_disposition(&state, &lease);
        state.check_artifact_advances();
        assert_eq!(
            state.threads.get("T-CDX-01").unwrap().current_phase,
            Phase::Review,
            "review.md should reach Review before the completion commit"
        );
        assert!(state.pending_completions.contains_key("T-CDX-01"));
        let on_disk = fs::read_to_string(state.config.ticket_dir.join("T-CDX-01.md")).unwrap();
        assert!(on_disk.contains("phase: review"), "ticket file: {on_disk}");

        // No signal files were ever written — advancement was artifact-only.
        assert!(state.signal_dir.read_dir().unwrap().next().is_none());
    }

    /// AC: `.stopped` at run end triggers Review auto-completion, dependencies
    /// respected. Codex's `.stopped` lands on an Idle live-TUI slot, so
    /// `handle_stopped_signal` Case 2 fires; the dep guard blocks a dependent
    /// ticket whose dependency is not yet Done.
    #[test]
    fn test_codex_stopped_auto_completes_review_respecting_deps() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, _dir) = codex_state_with_dag();
        // Put T-CDX-01 (dep-free) into Review on disk and in the DAG.
        let t1 = state.config.ticket_dir.join("T-CDX-01.md");
        fs::write(
            &t1,
            "---\nid: T-CDX-01\ntitle: codex-a\ntype: task\nstatus: review\npriority: high\nphase: review\n---\n\nBody\n",
        ).unwrap();
        // Dependent T-CDX-02 also into Review while its dep is NOT done.
        let t2 = state.config.ticket_dir.join("T-CDX-02.md");
        fs::write(
            &t2,
            "---\nid: T-CDX-02\ntitle: codex-b\ntype: task\nstatus: review\npriority: high\nphase: review\ndepends_on: [T-CDX-01]\n---\n\nBody\n",
        ).unwrap();
        let tickets = lisa_core::ticket::scan_tickets(&state.config.ticket_dir).unwrap();
        state.dag = Dag::from_tickets(tickets).unwrap();

        codex_slot(&mut state, 1, "T-CDX-01");
        codex_slot(&mut state, 2, "T-CDX-02");
        let mut th1 = Thread::new("T-CDX-01", 1);
        th1.current_phase = Phase::Review;
        state.threads.insert("T-CDX-01".to_string(), th1);
        let mut th2 = Thread::new("T-CDX-02", 2);
        th2.current_phase = Phase::Review;
        state.threads.insert("T-CDX-02".to_string(), th2);
        let lease1 = install_current_attempt(&mut state, "T-CDX-01");
        let lease2 = install_current_attempt(&mut state, "T-CDX-02");
        write_passing_review_disposition(&state, &lease1);
        write_passing_review_disposition(&state, &lease2);

        // Negative first: T-CDX-02's dep (T-CDX-01) is not Done → guard blocks.
        state.auto_complete_review("T-CDX-02".to_string(), 2);
        assert!(
            state.threads.contains_key("T-CDX-02"),
            "dependent ticket must NOT auto-complete while its dep is open"
        );
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::CompletionRejected {
                kind: CompletionRejectionKind::DependencyBlocked,
                detail,
                ..
            } if detail.contains("dependencies are not all done")
        )));
        assert!(fs::read_to_string(&t2).unwrap().contains("phase: review"));

        // Positive: the dep-free ticket enters the shared pending transaction.
        state.handle_stopped_signal(1);
        assert!(state.threads.contains_key("T-CDX-01"));
        assert!(state.pending_completions.contains_key("T-CDX-01"));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-CDX-01"));
        let done = fs::read_to_string(&t1).unwrap();
        assert!(
            done.contains("phase: review") && done.contains("status: review"),
            "{done}"
        );
    }

    /// AC: a long tool-free stretch does not false-trip stuck detection while
    /// heartbeats flow — and a genuinely hung run IS reclaimed. Codex `item.*`
    /// heartbeats reset the same activity clock Claude's PostToolUse heartbeats do.
    #[test]
    fn test_codex_heartbeat_honest_then_genuine_hang_reclaimed() {
        use lisa_core::types::Thread;

        // hard-silence bar = 2 × 600 = 1200s.
        let mk = || {
            let mut state = State {
                config: PluginConfig {
                    stuck_threshold_secs: 600,
                    ..PluginConfig::new()
                },
                ..State::default()
            };
            codex_slot(&mut state, 1, "T-CDX-01");
            state
        };

        // Honest: recent activity (a heartbeat 300s ago) — well under 1200s.
        let mut honest = mk();
        let mut alive = Thread::new("T-CDX-01", 1);
        alive.current_phase = Phase::Implement;
        alive.last_activity = std::time::SystemTime::now() - std::time::Duration::from_secs(300);
        honest.threads.insert("T-CDX-01".to_string(), alive);
        honest.detect_stale_threads();
        assert!(
            honest.threads.contains_key("T-CDX-01"),
            "a heartbeating session must never be reclaimed as stuck"
        );
        assert!(
            honest.agent_slots[0].ticket_id.is_some(),
            "slot stays bound"
        );

        // Genuine hang: silent 2000s > 1200s bar → reclaimed for retry.
        let mut hung = mk();
        let mut dead = Thread::new("T-CDX-01", 1);
        dead.current_phase = Phase::Implement;
        dead.last_activity = std::time::SystemTime::now() - std::time::Duration::from_secs(2000);
        dead.last_phase_change = dead.last_activity;
        hung.threads.insert("T-CDX-01".to_string(), dead);
        hung.detect_stale_threads();
        assert!(
            hung.threads.is_empty(),
            "a genuinely hung run must be reclaimed"
        );
        assert!(
            hung.agent_slots[0].ticket_id.is_none(),
            "slot released on reclaim"
        );
    }

    /// AC: a forced failure (`turn.failed`/non-zero exit → `.error`) fails the
    /// thread promptly and releases the slot — no waiting for 2× stuck-threshold
    /// of silence. Framed under Codex config; the consumer is adapter-agnostic.
    #[test]
    fn test_codex_error_signal_fails_thread_promptly() {
        use lisa_core::types::{Thread, ThreadStatus};
        use std::fs;

        let (mut state, _dir) = codex_state_with_dag();
        fs::write(state.signal_dir.join("pane-1.error"), "turn.failed: boom").unwrap();
        codex_slot(&mut state, 1, "T-CDX-01");
        let thread = Thread::new("T-CDX-01", 1);
        assert_eq!(thread.status, ThreadStatus::Running);
        state.threads.insert("T-CDX-01".to_string(), thread);

        state.check_error_signals();

        assert!(
            !state.signal_dir.join("pane-1.error").exists(),
            "signal consumed"
        );
        assert!(
            state.threads.is_empty(),
            "thread failed + removed for retry"
        );
        assert!(state.agent_slots[0].ticket_id.is_none(), "slot released");
        assert!(state.agent_slots[0].has_session, "session kept alive");
        assert_eq!(state.error_alerts, vec![("T-CDX-01".to_string(), 1)]);
    }

    /// AC: the review-timeout finish-up path types into the native Codex TUI.
    #[test]
    fn test_codex_review_timeout_finish_up_types_into_tui() {
        use lisa_core::types::Thread;

        // (a) path fires for a Codex Review thread past timeout + wind-down.
        let mut state = State {
            config: PluginConfig {
                client: AgentClient::Codex,
                lisa_bin: Some("/abs/lisa".to_string()),
                review_timeout_secs: 10,
                wind_down_secs: 180,
                ..PluginConfig::new()
            },
            ..State::default()
        };
        let mut thread = Thread::new("T-CDX-01", 1);
        thread.current_phase = Phase::Review;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(200);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-CDX-01".to_string(), thread);

        state.check_review_timeouts();
        assert!(
            state.finish_up_sent.contains("T-CDX-01"),
            "finish-up path taken"
        );
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::FinishUpPromptSent { ticket_id, .. } if ticket_id == "T-CDX-01"
        )));

        // (b) the delivered value is the bare finish-up prompt for the composer.
        let ticket_dir = Path::new("docs/active/tickets");
        let work_dir = Path::new("docs/active/work");
        let (adapter, _route) =
            resolve_adapter_or_native(None, AgentClient::Codex, Some("/abs/lisa"));
        let follow_up = adapter.follow_up(&FollowUpContext {
            ticket_dir,
            work_dir,
            ticket_id: "T-CDX-01",
            pane_id: 1,
        });
        assert_eq!(
            follow_up,
            FollowUp::TypeIntoPane(finish_up_prompt(ticket_dir, work_dir, "T-CDX-01"))
        );
    }

    /// AC: the dashboard shows sane states throughout — no phantom "awaiting".
    /// Codex never writes `.awaiting`, so `check_awaiting_signals` leaves the set
    /// empty and `to_ui_state` projects `awaiting=false` for every Codex pane.
    #[test]
    fn test_codex_pane_never_phantom_awaiting() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, _dir) = codex_state_with_dag();
        state.initialized = true;
        // The entire Codex signal vocabulary sans `.error` — no `.awaiting`.
        fs::write(state.signal_dir.join("pane-1.heartbeat"), "0").unwrap();
        fs::write(state.signal_dir.join("pane-1.stopped"), "0").unwrap();
        codex_slot(&mut state, 1, "T-CDX-01");
        let mut thread = Thread::new("T-CDX-01", 1);
        thread.current_phase = Phase::Implement;
        state.threads.insert("T-CDX-01".to_string(), thread);

        state.check_awaiting_signals();
        state.check_heartbeat_signals();

        assert!(
            state.awaiting_human.is_empty(),
            "no pane may be flagged awaiting"
        );
        assert!(!state.is_pane_awaiting(1));
        let ui = state.to_ui_state();
        let row = ui
            .active_threads
            .iter()
            .find(|t| t.ticket_id == "T-CDX-01")
            .expect("Codex thread should render as active");
        assert!(
            !row.awaiting,
            "dashboard must not invent an awaiting state for Codex"
        );
    }

    /// AC (mixed loop): signals are attributed per pane. Two running threads on
    /// panes 1 and 2; a `.error` for pane 2 fails only that pane's thread, pane 1
    /// untouched. (True single-loop client mixing is loop-wide-`client`-gated and
    /// deferred to S-026; per-`pane-<id>` attribution is the guarantee that holds.)
    #[test]
    fn test_mixed_panes_error_attributed_per_pane() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, _dir) = codex_state_with_dag();
        fs::write(state.signal_dir.join("pane-2.error"), "boom").unwrap();
        codex_slot(&mut state, 1, "T-CDX-01");
        codex_slot(&mut state, 2, "T-CDX-02");
        state
            .threads
            .insert("T-CDX-01".to_string(), Thread::new("T-CDX-01", 1));
        state
            .threads
            .insert("T-CDX-02".to_string(), Thread::new("T-CDX-02", 2));

        state.check_error_signals();

        assert!(
            state.threads.contains_key("T-CDX-01"),
            "pane-1 thread untouched"
        );
        assert!(
            !state.threads.contains_key("T-CDX-02"),
            "pane-2 thread failed"
        );
        assert!(
            state.agent_slots[0].ticket_id.is_some(),
            "pane-1 slot still bound"
        );
        assert!(
            state.agent_slots[1].ticket_id.is_none(),
            "pane-2 slot released"
        );
        assert_eq!(state.error_alerts, vec![("T-CDX-02".to_string(), 2)]);
    }

    // --- Provenance ledger (T-027-01) ---------------------------------------

    /// Point `state` at a ledger + codex dir inside `dir`, and return the ledger
    /// path so a test can read it back.
    fn with_ledger(state: &mut State, dir: &tempfile::TempDir) -> std::path::PathBuf {
        let ledger = dir.path().join("provenance.jsonl");
        state.ledger_path = ledger.clone();
        state.codex_dir = dir.path().join("codex");
        state.claude_dir = dir.path().join("claude");
        ledger
    }

    fn read_ledger(path: &std::path::Path) -> Vec<lisa_core::provenance::ProvenanceRecord> {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).expect("ledger line parses"))
            .collect()
    }

    fn read_mixed_ledger(
        path: &std::path::Path,
    ) -> Vec<lisa_core::provenance::ProvenanceLedgerRecord> {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).expect("mixed ledger line parses"))
            .collect()
    }

    fn preownership_failure_state(
        seat: SeatAssignmentState,
        client: AgentClient,
    ) -> (State, tempfile::TempDir, AttemptLease, std::path::PathBuf) {
        use lisa_core::types::Thread;

        let dir = tempfile::tempdir().unwrap();
        let mut state = State::default();
        state.agent_slots.push(AgentSlot {
            pane_id: 10,
            ticket_id: Some("T-NAME".to_string()),
            attempt_lease: None,
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: Some(client),
        });
        let mut thread = Thread::new("T-NAME", 10);
        thread.client = client;
        thread.started_at = std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(3))
            .unwrap();
        state.threads.insert("T-NAME".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-NAME");
        state.seat_assignments.insert(10, seat);
        let ledger = with_ledger(&mut state, &dir);
        (state, dir, lease, ledger)
    }

    #[test]
    fn rc6_preownership_delivery_miss_is_durable_and_cli_retrievable() {
        use lisa_core::provenance::{
            AssignmentState, ProvenanceLedgerRecord, ProvenanceRecordType,
        };

        const REASON: &str = "provider did not acknowledge the bounded chat assignment";
        let deadline = std::time::SystemTime::now();
        let (mut state, _dir, lease, ledger) = preownership_failure_state(
            SeatAssignmentState::Delivering {
                generation: 6,
                ack_deadline: deadline,
                retries: MAX_ASSIGNMENT_DELIVERY_RETRIES,
            },
            AgentClient::Codex,
        );

        let outcomes = state.check_assignment_ack_timeouts_at(
            deadline
                .checked_add(std::time::Duration::from_secs(1))
                .unwrap(),
        );

        assert_eq!(
            outcomes,
            vec![FailureTransitionOutcome::AssignmentDeliveryFailed {
                pane_id: 10,
                ticket_id: Some("T-NAME".to_string()),
            }]
        );
        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::DeliveryFailed),
            "a missed assignment must remain explicitly failed, never owned"
        );

        let raw = std::fs::read_to_string(&ledger)
            .expect("the pre-ownership miss must create a durable ledger");
        assert_eq!(
            raw.lines().count(),
            1,
            "one terminal miss must append exactly one physical row"
        );
        let value: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
        assert!(value.get("authoritative").is_none());
        assert!(value.get("outcome").is_none());

        let record: ProvenanceLedgerRecord = serde_json::from_str(raw.trim()).unwrap();
        let ProvenanceLedgerRecord::AssignmentTransition(record) = record else {
            panic!("a pre-ownership miss must append assignment-transition evidence");
        };
        assert_eq!(record.schema_version, lisa_core::provenance::SCHEMA_VERSION);
        assert_eq!(
            record.record_type,
            ProvenanceRecordType::AssignmentTransition
        );
        assert_eq!(record.ticket_id, "T-NAME");
        assert_eq!(record.attempt_lease, lease);
        assert_eq!(record.pane_id, 10);
        assert_eq!(record.provider, "openai");
        assert_eq!(record.state, AssignmentState::DeliveryFailed);
        assert_eq!(record.reason, REASON);

        let mut report = Vec::new();
        preownership_status_surface::write_preownership_status(&ledger, "T-NAME", &mut report)
            .expect("lisa status must retrieve the scheduler-written row");
        let report = String::from_utf8(report).unwrap();
        assert!(report.starts_with("Pre-ownership failures for T-NAME (1):\n"));
        assert!(report.contains(&format!("Attempt {} (pane 10)\n", lease.attempt_id)));
        assert!(report.contains("  state: delivery-failed\n"));
        assert!(report.contains(&format!("  reason: {REASON}\n")));
        assert!(report.contains("  provider: openai\n"));
        assert!(report.contains("  started_at: "));
        assert!(report.contains("  ended_at: "));
        assert!(report.contains("  wall_clock_secs: "));
    }

    #[test]
    fn preownership_terminal_transitions_append_once_and_coexist_with_later_done() {
        use lisa_core::provenance::{
            AssignmentState, ProvenanceLedgerRecord, ProvenanceRecordType,
        };

        let deadline = std::time::SystemTime::now();

        let (mut delivery, _delivery_dir, delivery_lease, delivery_ledger) =
            preownership_failure_state(
                SeatAssignmentState::Delivering {
                    generation: 1,
                    ack_deadline: deadline,
                    retries: MAX_ASSIGNMENT_DELIVERY_RETRIES,
                },
                AgentClient::Claude,
            );
        assert!(delivery
            .fail_assignment_delivery(10, "delivery evidence")
            .is_some());
        assert_eq!(
            delivery.fail_assignment_delivery(10, "duplicate delivery"),
            None
        );
        assert!(
            delivery_ledger.exists(),
            "delivery ledger missing: {:?}",
            delivery.activity_log
        );

        let (mut recovery, _recovery_dir, recovery_lease, recovery_ledger) =
            preownership_failure_state(
                SeatAssignmentState::Recovering {
                    generation: 1,
                    ack_deadline: Some(deadline),
                },
                AgentClient::Codex,
            );
        assert!(recovery
            .fail_assignment_recovery(10, "recovery evidence")
            .is_some());
        assert_eq!(
            recovery.fail_assignment_recovery(10, "duplicate recovery"),
            None
        );
        assert!(
            recovery_ledger.exists(),
            "recovery ledger missing: {:?}",
            recovery.activity_log
        );

        let (mut startup, _startup_dir, startup_lease, startup_ledger) = preownership_failure_state(
            SeatAssignmentState::Starting {
                generation: 1,
                start_deadline: Some(deadline),
                relaunches: 0,
            },
            AgentClient::Codex,
        );
        assert!(startup.fail_startup(10, "startup evidence").is_some());
        assert_eq!(startup.fail_startup(10, "duplicate startup"), None);
        assert!(
            startup_ledger.exists(),
            "startup ledger missing: {:?}",
            startup.activity_log
        );

        let cases = [
            (
                &delivery_ledger,
                delivery_lease.clone(),
                "anthropic",
                AssignmentState::DeliveryFailed,
                "delivery evidence",
            ),
            (
                &recovery_ledger,
                recovery_lease,
                "openai",
                AssignmentState::RecoveryFailed,
                "recovery evidence",
            ),
            (
                &startup_ledger,
                startup_lease,
                "openai",
                AssignmentState::StartupFailed,
                "startup evidence",
            ),
        ];
        for (ledger, lease, provider, expected_state, reason) in cases {
            let raw = std::fs::read_to_string(ledger).unwrap();
            assert_eq!(raw.lines().count(), 1, "terminal edge appends exactly once");
            let value: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
            assert!(value.get("authoritative").is_none());
            assert!(value.get("outcome").is_none());

            let records = read_mixed_ledger(ledger);
            let ProvenanceLedgerRecord::AssignmentTransition(record) = &records[0] else {
                panic!("pre-ownership failure must use the assignment row shape");
            };
            assert_eq!(record.schema_version, lisa_core::provenance::SCHEMA_VERSION);
            assert_eq!(
                record.record_type,
                ProvenanceRecordType::AssignmentTransition
            );
            assert_eq!(record.ticket_id, "T-NAME");
            assert_eq!(record.attempt_lease, lease);
            assert_eq!(record.pane_id, 10);
            assert_eq!(record.provider, provider);
            assert_eq!(record.state, expected_state);
            assert_eq!(record.reason, reason);
            assert!(record.ended_at >= record.started_at);
            assert_eq!(
                record.wall_clock_secs,
                record.ended_at.saturating_sub(record.started_at)
            );
        }

        let later_lease = install_current_attempt(&mut delivery, "T-NAME");
        assert!(delivery.emit_provenance("T-NAME", RunOutcome::Done, false));
        let records = read_mixed_ledger(&delivery_ledger);
        assert_eq!(records.len(), 2, "later terminal evidence appends");
        assert!(matches!(
            &records[0],
            ProvenanceLedgerRecord::AssignmentTransition(record)
                if record.attempt_lease == delivery_lease
                    && record.state == AssignmentState::DeliveryFailed
        ));
        assert!(matches!(
            &records[1],
            ProvenanceLedgerRecord::Execution(record)
                if record.attempt_lease == later_lease
                    && record.outcome == RunOutcome::Done
                    && record.authoritative
        ));
    }

    /// AC: a record is emitted on terminal failure (`.error` reclaim), driven
    /// end-to-end through the real teardown site — proves the call-site wiring.
    #[test]
    fn provenance_emitted_on_error_signal() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);
        fs::write(state.signal_dir.join("pane-2.error"), "boom").unwrap();
        codex_slot(&mut state, 2, "T-CDX-02");
        // A Codex-loop thread carries client=Codex (set at spawn, lib.rs:687); the
        // manual construction here must match so the recorded route is codex.
        let mut thread = Thread::new("T-CDX-02", 2);
        thread.client = AgentClient::Codex;
        state.threads.insert("T-CDX-02".to_string(), thread);
        let lease = install_current_attempt(&mut state, "T-CDX-02");

        state.check_error_signals();

        let records = read_ledger(&ledger);
        assert_eq!(records.len(), 1, "one record on failure");
        assert_eq!(records[0].ticket_id, "T-CDX-02");
        assert_eq!(records[0].attempt_lease, lease);
        assert_eq!(records[0].outcome, RunOutcome::Failed);
        assert!(!records[0].authoritative);
        assert!(!records[0].fenced);
        assert_eq!(records[0].actual.method, "codex");
        assert_eq!(records[0].actual.provider, "openai");
        assert_eq!(
            records[0].schema_version,
            lisa_core::provenance::SCHEMA_VERSION
        );
    }

    /// AC: retries/resets append additional records; nothing rewrites history.
    #[test]
    fn provenance_retry_appends_not_rewrites() {
        use lisa_core::types::Thread;

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);

        // First run: completes.
        state
            .threads
            .insert("T-CDX-01".to_string(), Thread::new("T-CDX-01", 1));
        let first = install_current_attempt(&mut state, "T-CDX-01");
        state.emit_provenance("T-CDX-01", RunOutcome::Done, false);
        state.threads.remove("T-CDX-01");

        // Retry of the same ticket: fails.
        state
            .threads
            .insert("T-CDX-01".to_string(), Thread::new("T-CDX-01", 1));
        let second = install_current_attempt(&mut state, "T-CDX-01");
        state.emit_provenance("T-CDX-01", RunOutcome::Failed, false);

        let records = read_ledger(&ledger);
        assert_eq!(records.len(), 2, "retry appends a second record");
        assert_eq!(records[0].attempt_lease, first);
        assert_eq!(records[1].attempt_lease, second);
        assert_eq!(records[0].outcome, RunOutcome::Done, "first record intact");
        assert_eq!(records[1].outcome, RunOutcome::Failed);
    }

    #[test]
    fn provenance_append_failure_is_logged_without_mutating_target() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("provenance path ' ; $()");
        fs::create_dir(&ledger).unwrap();
        let sentinel = ledger.join("existing-ledger-bytes");
        fs::write(&sentinel, "prior provenance remains intact\n").unwrap();

        let mut state = State {
            ledger_path: ledger.clone(),
            ..State::default()
        };
        let ticket_id = "T-PROV-FAIL";
        let mut thread = Thread::new(ticket_id, 47);
        thread.client = AgentClient::Codex;
        state.threads.insert(ticket_id.to_string(), thread);
        let lease = install_current_attempt(&mut state, ticket_id);

        assert!(!state.emit_provenance(ticket_id, RunOutcome::Done, false));

        assert!(ledger.is_dir());
        assert_eq!(
            fs::read_to_string(&sentinel).unwrap(),
            "prior provenance remains intact\n"
        );
        assert_eq!(fs::read_dir(&ledger).unwrap().count(), 1);
        assert_eq!(state.current_leases.get(ticket_id), Some(&lease));
        assert_eq!(
            state.threads.get(ticket_id).unwrap().attempt_lease.as_ref(),
            Some(&lease)
        );
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::Error { message }
                if message.starts_with("provenance write failed for T-PROV-FAIL: ")
        )));
    }

    /// AC: Codex captures flow into the owning provenance record.
    #[test]
    fn provenance_codex_usage_flows_into_record() {
        use lisa_core::capture::{append_capture_record, CaptureRecord};
        use lisa_core::types::Thread;

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);

        let mut thread = Thread::new("T-CDX-01", 1);
        thread.client = AgentClient::Codex;
        append_capture_record(
            &state.codex_dir.join("captures.jsonl"),
            &CaptureRecord {
                pane_id: thread.pane_id,
                session_id: "codex-session".to_string(),
                captured_at: provenance::system_time_to_epoch(std::time::SystemTime::now()),
                input_tokens: 120,
                output_tokens: 34,
            },
        )
        .unwrap();
        state.threads.insert("T-CDX-01".to_string(), thread);
        install_current_attempt(&mut state, "T-CDX-01");
        state.emit_provenance("T-CDX-01", RunOutcome::Done, false);

        let records = read_ledger(&ledger);
        assert_eq!(records[0].tokens_in, Some(120));
        assert_eq!(records[0].tokens_out, Some(34));
        assert_eq!(
            records[0].cost_usd, None,
            "no cost field → null, never fabricated"
        );
    }

    /// T-043-03-01 AC: a physical pane recycled from A to B attributes each
    /// capture to its pane-time owner, sums per ticket, and appends B without
    /// rewriting A's terminal provenance row.
    #[test]
    fn provenance_recycled_pane_attributes_capture_sums_to_each_ticket() {
        use lisa_core::capture::{append_capture_record, CaptureRecord};
        use lisa_core::types::Thread;

        const PANE_ID: u32 = 7;
        const TICKET_A: &str = "T-CDX-01";
        const TICKET_B: &str = "T-CDX-02";

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);
        let captures = state.codex_dir.join("captures.jsonl");
        let now = provenance::system_time_to_epoch(std::time::SystemTime::now());
        let a_started = now.saturating_sub(1_000);
        let a_ended = now.saturating_sub(800);
        let b_started = now.saturating_sub(600);

        for capture in [
            CaptureRecord {
                pane_id: PANE_ID,
                session_id: "session-a".to_string(),
                captured_at: a_started + 50,
                input_tokens: 10,
                output_tokens: 3,
            },
            CaptureRecord {
                pane_id: PANE_ID,
                session_id: "session-a".to_string(),
                captured_at: a_started + 100,
                input_tokens: 20,
                output_tokens: 7,
            },
            CaptureRecord {
                pane_id: PANE_ID,
                session_id: "session-b".to_string(),
                captured_at: b_started + 50,
                input_tokens: 100,
                output_tokens: 40,
            },
            CaptureRecord {
                pane_id: PANE_ID,
                session_id: "session-b".to_string(),
                captured_at: b_started + 100,
                input_tokens: 200,
                output_tokens: 60,
            },
        ] {
            append_capture_record(&captures, &capture).unwrap();
        }

        let route = Route::from_client(AgentClient::Codex);
        let a_without_usage = ProvenanceRecord {
            schema_version: provenance::SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            completion_note: None,
            ticket_id: TICKET_A.to_string(),
            attempt_lease: AttemptLease {
                ticket_id: TICKET_A.to_string(),
                attempt_id: 1,
            },
            outcome: RunOutcome::Done,
            authoritative: true,
            fenced: false,
            requested: route.clone(),
            actual: route,
            started_at: a_started,
            ended_at: a_ended,
            wall_clock_secs: a_ended.saturating_sub(a_started),
            tokens_in: None,
            tokens_out: None,
            cost_usd: None,
            concurrency_at_spawn: 0,
            pane_id: PANE_ID,
        };
        let (tokens_in, tokens_out, cost_usd) =
            state.read_usage(AgentClient::Codex, &a_without_usage);
        assert!(
            !state.codex_dir.join("quarantine").exists(),
            "captures after A's closed interval must remain pending for B"
        );
        let a_record = ProvenanceRecord {
            tokens_in,
            tokens_out,
            cost_usd,
            ..a_without_usage
        };
        provenance::append_record(&ledger, &a_record).unwrap();

        let after_a = read_ledger(&ledger);
        assert_eq!(after_a.len(), 1);
        assert_eq!(after_a[0].ticket_id, TICKET_A);
        assert_eq!(after_a[0].tokens_in, Some(30));
        assert_eq!(after_a[0].tokens_out, Some(10));

        let mut b_thread = Thread::new(TICKET_B, PANE_ID);
        b_thread.client = AgentClient::Codex;
        b_thread.started_at = std::time::UNIX_EPOCH
            .checked_add(std::time::Duration::from_secs(b_started))
            .unwrap();
        state.threads.insert(TICKET_B.to_string(), b_thread);
        install_current_attempt(&mut state, TICKET_B);
        assert!(state.emit_provenance(TICKET_B, RunOutcome::Done, false));

        let records = read_ledger(&ledger);
        assert_eq!(records.len(), 2, "B must append rather than overwrite A");
        assert_eq!(records[0].ticket_id, TICKET_A);
        assert_eq!(records[0].tokens_in, Some(30));
        assert_eq!(records[0].tokens_out, Some(10));
        assert_eq!(records[1].ticket_id, TICKET_B);
        assert_eq!(records[1].tokens_in, Some(300));
        assert_eq!(records[1].tokens_out, Some(100));
    }

    /// T-043-03-03 AC: replay the field incident as one deterministic chain.
    /// Tickets 2 through 7 are the six recycled-pane Stops that the old writer
    /// keyed to ticket 1's inherited environment and overwrote in place.
    #[test]
    fn provenance_field_repro_keeps_six_recycles_distinct_and_surfaces_failures() {
        use lisa_cli::capture_usage::run_capture_usage_for_test;
        use lisa_core::capture::CaptureRecord;
        use serde::Deserialize;

        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct NoCaptureMarker {
            pane_id: u32,
            session_id: String,
            captured_at: u64,
            reason: String,
        }

        const PANE_ID: u32 = 43;
        const UNOWNED_SESSION: &str = "session-unattributable";
        const NO_CAPTURE_SESSION: &str = "session-no-capture";
        const TICKETS: [&str; 7] = [
            "T-FIELD-01",
            "T-FIELD-02",
            "T-FIELD-03",
            "T-FIELD-04",
            "T-FIELD-05",
            "T-FIELD-06",
            "T-FIELD-07",
        ];

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);
        state.claude_dir = dir.path().join(".lisa/claude");

        let write_stop = |session_id: &str,
                          transcript: &std::path::Path,
                          captured_at: u64,
                          diagnostics: &mut Vec<u8>| {
            let payload = serde_json::json!({
                "session_id": session_id,
                "transcript_path": transcript,
            });
            run_capture_usage_for_test(
                dir.path(),
                payload.to_string().as_bytes(),
                false,
                PANE_ID,
                captured_at,
                diagnostics,
            )
            .unwrap();
        };

        let unowned_transcript = dir.path().join("unowned.jsonl");
        std::fs::write(
            &unowned_transcript,
            r#"{"type":"assistant","message":{"usage":{"input_tokens":9999,"output_tokens":888}}}
"#,
        )
        .unwrap();
        let unowned_capture = CaptureRecord {
            pane_id: PANE_ID,
            session_id: UNOWNED_SESSION.to_string(),
            captured_at: 50,
            input_tokens: 9_999,
            output_tokens: 888,
        };
        let mut successful_diagnostics = Vec::new();
        write_stop(
            UNOWNED_SESSION,
            &unowned_transcript,
            unowned_capture.captured_at,
            &mut successful_diagnostics,
        );

        let mut expected_captures = vec![unowned_capture.clone()];
        let mut expected_usage = Vec::new();
        for (index, ticket_id) in TICKETS.iter().enumerate() {
            let sequence = u64::try_from(index).unwrap() + 1;
            let input_tokens = sequence * 100 + 7;
            let output_tokens = sequence * 10 + 3;
            let captured_at = 110 + u64::try_from(index).unwrap() * 100;
            let session_id = format!("session-field-{sequence}");
            let transcript = dir.path().join(format!("field-{sequence}.jsonl"));
            let transcript_line = serde_json::json!({
                "type": "assistant",
                "message": {
                    "usage": {
                        "input_tokens": input_tokens,
                        "output_tokens": output_tokens,
                    }
                }
            });
            std::fs::write(&transcript, format!("{transcript_line}\n")).unwrap();
            write_stop(
                &session_id,
                &transcript,
                captured_at,
                &mut successful_diagnostics,
            );
            expected_captures.push(CaptureRecord {
                pane_id: PANE_ID,
                session_id,
                captured_at,
                input_tokens,
                output_tokens,
            });
            expected_usage.push(((*ticket_id).to_string(), input_tokens, output_tokens));
        }
        assert!(
            successful_diagnostics.is_empty(),
            "successful captures should not emit no-capture diagnostics"
        );

        let empty_transcript = dir.path().join("empty.jsonl");
        std::fs::write(&empty_transcript, "").unwrap();
        let mut no_capture_diagnostics = Vec::new();
        write_stop(
            NO_CAPTURE_SESSION,
            &empty_transcript,
            75,
            &mut no_capture_diagnostics,
        );

        let captures_path = state.claude_dir.join("captures.jsonl");
        let captures: Vec<CaptureRecord> = std::fs::read_to_string(&captures_path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            captures, expected_captures,
            "the unowned observation and all seven ticket Stops must survive"
        );
        assert_eq!(captures.len(), 8);
        assert!(
            captures
                .iter()
                .all(|capture| capture.session_id != NO_CAPTURE_SESSION),
            "a failed observation must not become measured zero usage"
        );
        assert!(!state.claude_dir.join("T-FIELD-01.usage.json").exists());
        assert!(!state.claude_dir.join("last.usage.json").exists());

        let no_capture_rows: Vec<NoCaptureMarker> =
            std::fs::read_to_string(state.claude_dir.join("no-captures.jsonl"))
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        assert_eq!(
            no_capture_rows,
            vec![NoCaptureMarker {
                pane_id: PANE_ID,
                session_id: NO_CAPTURE_SESSION.to_string(),
                captured_at: 75,
                reason: "empty-transcript".to_string(),
            }]
        );
        let no_capture_diagnostics = String::from_utf8(no_capture_diagnostics).unwrap();
        assert!(no_capture_diagnostics.contains("lisa capture-usage: no capture"));
        assert!(no_capture_diagnostics.contains(NO_CAPTURE_SESSION));
        assert!(no_capture_diagnostics.contains("empty-transcript"));

        let route = Route::from_client(AgentClient::Claude);
        for (index, (ticket_id, input_tokens, output_tokens)) in expected_usage.iter().enumerate() {
            let started_at = 100 + u64::try_from(index).unwrap() * 100;
            let ended_at = started_at + 49;
            let current = ProvenanceRecord {
                schema_version: provenance::SCHEMA_VERSION,
                seal: CompletionSeal::Commit,
                completion_note: None,
                ticket_id: ticket_id.clone(),
                attempt_lease: AttemptLease {
                    ticket_id: ticket_id.clone(),
                    attempt_id: 1,
                },
                outcome: RunOutcome::Done,
                authoritative: true,
                fenced: false,
                requested: route.clone(),
                actual: route.clone(),
                started_at,
                ended_at,
                wall_clock_secs: ended_at - started_at,
                tokens_in: None,
                tokens_out: None,
                cost_usd: None,
                concurrency_at_spawn: 0,
                pane_id: PANE_ID,
            };
            let usage = state.read_usage(AgentClient::Claude, &current);
            assert_eq!(
                usage,
                (Some(*input_tokens), Some(*output_tokens), None),
                "{ticket_id} must receive only its pane-time capture"
            );
            provenance::append_record(
                &ledger,
                &ProvenanceRecord {
                    tokens_in: usage.0,
                    tokens_out: usage.1,
                    cost_usd: usage.2,
                    ..current
                },
            )
            .unwrap();
        }

        let records = read_ledger(&ledger);
        assert_eq!(records.len(), TICKETS.len());
        let actual_usage: Vec<_> = records
            .iter()
            .map(|record| {
                (
                    record.ticket_id.clone(),
                    record.tokens_in.unwrap(),
                    record.tokens_out.unwrap(),
                )
            })
            .collect();
        assert_eq!(
            actual_usage, expected_usage,
            "all six later pane recycles must append without rewriting ticket 1"
        );
        assert!(records.iter().all(|record| record.cost_usd.is_none()));
        assert!(records.iter().all(|record| {
            record.tokens_in != Some(unowned_capture.input_tokens)
                && record.tokens_out != Some(unowned_capture.output_tokens)
        }));

        let quarantine_path = quarantine::session_path(&state.claude_dir, UNOWNED_SESSION);
        let quarantined: Vec<quarantine::QuarantinedCaptureRecord> =
            std::fs::read_to_string(&quarantine_path)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        assert_eq!(
            quarantined,
            vec![quarantine::QuarantinedCaptureRecord {
                source_line: 1,
                capture: unowned_capture,
            }]
        );
        assert!(!state.claude_dir.join("quarantine.jsonl").exists());

        let quarantine_warnings: Vec<_> = state
            .activity_log
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    ActivityEvent::Warning { message }
                        if message.contains("usage capture quarantined")
                            && message.contains(UNOWNED_SESSION)
                )
            })
            .collect();
        assert_eq!(
            quarantine_warnings.len(),
            1,
            "ledger rescans must not duplicate quarantine visibility"
        );
        let entry = activity_event_to_ui_entry(quarantine_warnings[0])
            .expect("quarantine warning should reach the dashboard activity feed");
        assert!(matches!(
            entry.activity,
            ui::ActivityType::Warning { message, .. }
                if message.contains(UNOWNED_SESSION)
        ));
        assert_eq!(
            std::fs::read_to_string(quarantine_path)
                .unwrap()
                .lines()
                .count(),
            1
        );
    }

    /// T-043-03-02 AC: a valid capture with no pane-time owner is held in its
    /// session's quarantine and surfaced as a visible warning, never usage.
    #[test]
    fn provenance_unattributable_capture_is_quarantined_by_session_and_visible() {
        use lisa_core::capture::{append_capture_record, CaptureRecord};

        const PANE_ID: u32 = 7;
        const SESSION_ID: &str = "session-unowned";

        let (mut state, dir) = codex_state_with_dag();
        with_ledger(&mut state, &dir);
        let capture = CaptureRecord {
            pane_id: PANE_ID,
            session_id: SESSION_ID.to_string(),
            captured_at: 150,
            input_tokens: 999,
            output_tokens: 111,
        };
        append_capture_record(&state.codex_dir.join("captures.jsonl"), &capture).unwrap();

        let route = Route::from_client(AgentClient::Codex);
        let current = ProvenanceRecord {
            schema_version: provenance::SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            completion_note: None,
            ticket_id: "T-CDX-01".to_string(),
            attempt_lease: AttemptLease {
                ticket_id: "T-CDX-01".to_string(),
                attempt_id: 1,
            },
            outcome: RunOutcome::Done,
            authoritative: true,
            fenced: false,
            requested: route.clone(),
            actual: route,
            started_at: 200,
            ended_at: 300,
            wall_clock_secs: 100,
            tokens_in: None,
            tokens_out: None,
            cost_usd: None,
            concurrency_at_spawn: 0,
            pane_id: PANE_ID,
        };

        assert_eq!(
            state.read_usage(AgentClient::Codex, &current),
            (None, None, None),
            "unowned tokens must not blend into the current ticket"
        );

        let quarantine_path = quarantine::session_path(&state.codex_dir, SESSION_ID);
        let rows: Vec<quarantine::QuarantinedCaptureRecord> =
            std::fs::read_to_string(&quarantine_path)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        assert_eq!(
            rows,
            vec![quarantine::QuarantinedCaptureRecord {
                source_line: 1,
                capture: capture.clone(),
            }]
        );
        assert!(
            !state.codex_dir.join("quarantine.jsonl").exists(),
            "quarantine must not use a provider-wide shared bucket"
        );
        assert!(!state.codex_dir.join("last").exists());
        assert!(!state.codex_dir.join("last.usage.json").exists());

        let warning = state
            .activity_log
            .iter()
            .find(|event| {
                matches!(
                    event,
                    ActivityEvent::Warning { message }
                        if message.contains("usage capture quarantined")
                            && message.contains(SESSION_ID)
                )
            })
            .expect("new quarantine should raise an activity warning");
        let entry = activity_event_to_ui_entry(warning)
            .expect("quarantine warning should be visible in the dashboard activity feed");
        assert!(matches!(
            entry.activity,
            ui::ActivityType::Warning { message, .. }
                if message.contains("usage capture quarantined")
                    && message.contains(SESSION_ID)
        ));

        assert_eq!(
            state.read_usage(AgentClient::Codex, &current),
            (None, None, None)
        );
        assert_eq!(
            std::fs::read_to_string(&quarantine_path)
                .unwrap()
                .lines()
                .count(),
            1,
            "a rescan must not duplicate the quarantined row"
        );
        assert_eq!(
            state
                .activity_log
                .iter()
                .filter(|event| matches!(
                    event,
                    ActivityEvent::Warning { message }
                        if message.contains("usage capture quarantined")
                            && message.contains(SESSION_ID)
                ))
                .count(),
            1,
            "a rescan must not repeat the operator warning"
        );
    }

    /// AC: Claude records carry null cost/tokens until T-027-02 (no artifact).
    #[test]
    fn provenance_claude_record_has_null_tokens() {
        use lisa_core::types::Thread;

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);
        let mut thread = Thread::new("T-CDX-01", 1);
        thread.client = AgentClient::Claude;
        thread.concurrency_at_spawn = 3;
        state.threads.insert("T-CDX-01".to_string(), thread);

        install_current_attempt(&mut state, "T-CDX-01");
        state.emit_provenance("T-CDX-01", RunOutcome::Done, false);

        let records = read_ledger(&ledger);
        assert_eq!(records[0].tokens_in, None);
        assert_eq!(records[0].tokens_out, None);
        assert_eq!(records[0].cost_usd, None);
        assert_eq!(records[0].actual.method, "claude");
        assert_eq!(records[0].actual.provider, "anthropic");
        assert_eq!(
            records[0].concurrency_at_spawn, 3,
            "spawn concurrency recorded"
        );
    }

    /// T-027-02 AC: a Claude run's capture flows from `.lisa/claude` into the
    /// record; `cost_usd` stays null (derived downstream, never fabricated).
    #[test]
    fn provenance_claude_usage_flows_into_record() {
        use lisa_core::capture::{append_capture_record, CaptureRecord};
        use lisa_core::types::Thread;

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);

        let mut thread = Thread::new("T-CDX-01", 1);
        thread.client = AgentClient::Claude;
        append_capture_record(
            &state.claude_dir.join("captures.jsonl"),
            &CaptureRecord {
                pane_id: thread.pane_id,
                session_id: "claude-session".to_string(),
                captured_at: provenance::system_time_to_epoch(std::time::SystemTime::now()),
                input_tokens: 167,
                output_tokens: 37,
            },
        )
        .unwrap();
        state.threads.insert("T-CDX-01".to_string(), thread);
        install_current_attempt(&mut state, "T-CDX-01");
        state.emit_provenance("T-CDX-01", RunOutcome::Done, false);

        let records = read_ledger(&ledger);
        assert_eq!(records[0].tokens_in, Some(167));
        assert_eq!(records[0].tokens_out, Some(37));
        assert_eq!(records[0].cost_usd, None, "Claude records no dollar cost");
        assert_eq!(records[0].actual.method, "claude");
    }

    /// AC: the emission never touches agent-owned ticket frontmatter.
    #[test]
    fn provenance_does_not_touch_ticket_frontmatter() {
        use lisa_core::types::Thread;

        let (mut state, dir) = codex_state_with_dag();
        with_ledger(&mut state, &dir);
        let ticket_file = state.config.ticket_dir.join("T-CDX-01.md");
        let before = std::fs::read(&ticket_file).unwrap();

        state
            .threads
            .insert("T-CDX-01".to_string(), Thread::new("T-CDX-01", 1));
        install_current_attempt(&mut state, "T-CDX-01");
        state.emit_provenance("T-CDX-01", RunOutcome::Done, false);

        let after = std::fs::read(&ticket_file).unwrap();
        assert_eq!(before, after, "ticket frontmatter must be byte-identical");
    }

    /// A write with an unset ledger path (native tests / pre-load) is a no-op,
    /// never a panic — so unrelated teardown-triggering tests don't hit disk.
    #[test]
    fn provenance_noop_when_ledger_unset() {
        use lisa_core::types::Thread;

        let (mut state, _dir) = codex_state_with_dag();
        // ledger_path deliberately left empty (State::default()).
        assert!(state.ledger_path.as_os_str().is_empty());
        state
            .threads
            .insert("T-CDX-01".to_string(), Thread::new("T-CDX-01", 1));
        // Must not panic or write anywhere.
        state.emit_provenance("T-CDX-01", RunOutcome::Done, false);
    }

    #[test]
    fn typed_completion_rejects_stale_attempt_and_accepts_current_lease() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, _dir) = codex_state_with_dag();
        let ticket_path = state.config.ticket_dir.join("T-CDX-01.md");
        fs::write(
            &ticket_path,
            "---\nid: T-CDX-01\ntitle: codex-a\ntype: task\nstatus: review\npriority: high\nphase: review\nagent: codex\n---\n\nBody\n",
        )
        .unwrap();
        state.dag =
            Dag::from_tickets(lisa_core::ticket::scan_tickets(&state.config.ticket_dir).unwrap())
                .unwrap();
        let mut thread = Thread::new("T-CDX-01", 1);
        thread.current_phase = Phase::Review;
        state.threads.insert("T-CDX-01".to_string(), thread);
        codex_slot(&mut state, 1, "T-CDX-01");

        let stale = install_current_attempt(&mut state, "T-CDX-01");
        let current = install_current_attempt(&mut state, "T-CDX-01");
        assert!(!stale.is_current(state.current_leases.get("T-CDX-01")));
        assert!(current.is_current(state.current_leases.get("T-CDX-01")));

        assert!(!state.dispatch_completion(CompletionInput::ObservedDone {
            ticket_id: "T-CDX-01".to_string(),
            source_lease: Some(stale.clone()),
        }));
        assert!(!state.pending_completions.contains_key("T-CDX-01"));
        assert!(state.threads.contains_key("T-CDX-01"));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-CDX-01"));
        let stale_correlation = CompletionGenerationId::new(
            CompletionId::new("T-CDX-01"),
            AttemptId::new(stale.attempt_id.to_string()),
            1,
        )
        .to_string();
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::CompletionRejected {
                kind: CompletionRejectionKind::StaleLease,
                correlation_id,
                detail,
                ..
            } if correlation_id == &stale_correlation
                && detail.contains(&stale.attempt_id.to_string())
        )));

        assert!(state.dispatch_completion(CompletionInput::ObservedDone {
            ticket_id: "T-CDX-01".to_string(),
            source_lease: Some(current.clone()),
        }));
        let pending = state.pending_completions.get("T-CDX-01").unwrap();
        assert_eq!(
            pending.authority,
            CompletionAuthority::Attempt(current.clone())
        );
        assert_eq!(pending.source, CompletionSource::ObservedDone);
        assert_eq!(
            state.launched_completion_effects,
            vec![EffectCommand::LaunchCompletion {
                attempt_id: AttemptId::new(current.attempt_id.to_string()),
                completion_id: CompletionId::new("T-CDX-01"),
            }]
        );
        assert!(fs::read_to_string(ticket_path)
            .unwrap()
            .contains("phase: review"));
    }

    #[test]
    fn fenced_attempt_and_replacement_publish_one_authoritative_done_record() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);
        let ticket_path = state.config.ticket_dir.join("T-CDX-01.md");
        fs::write(
            &ticket_path,
            "---\nid: T-CDX-01\ntitle: codex-a\ntype: task\nstatus: review\npriority: high\nphase: review\nagent: codex\n---\n\nBody\n",
        )
        .unwrap();
        state.dag =
            Dag::from_tickets(lisa_core::ticket::scan_tickets(&state.config.ticket_dir).unwrap())
                .unwrap();

        codex_slot(&mut state, 1, "T-CDX-01");
        let mut predecessor_thread = Thread::new("T-CDX-01", 1);
        predecessor_thread.client = AgentClient::Codex;
        predecessor_thread.current_phase = Phase::Review;
        state
            .threads
            .insert("T-CDX-01".to_string(), predecessor_thread);
        let predecessor = install_current_attempt(&mut state, "T-CDX-01");

        state.threads.get_mut("T-CDX-01").unwrap().fail();
        let fenced = matches!(
            state.revoke_and_fence_attempt(&"T-CDX-01".to_string()),
            FenceOutcome::Fenced { pane_id: 1 }
        );
        assert!(fenced);
        assert!(state.emit_provenance("T-CDX-01", RunOutcome::TimedOut, fenced));
        state.release_slot_for_ticket(&"T-CDX-01".to_string());
        state.threads.remove("T-CDX-01");

        codex_slot(&mut state, 2, "T-CDX-01");
        let mut replacement_thread = Thread::new("T-CDX-01", 2);
        replacement_thread.client = AgentClient::Codex;
        replacement_thread.current_phase = Phase::Review;
        state
            .threads
            .insert("T-CDX-01".to_string(), replacement_thread);
        let replacement = install_current_attempt(&mut state, "T-CDX-01");
        assert_eq!(replacement.attempt_id, predecessor.attempt_id + 1);

        // The ledger writer is independently fail-closed for Done. Even if a
        // caller presents the predecessor's thread stamp, current authority is
        // still the replacement and no row is appended.
        state.threads.get_mut("T-CDX-01").unwrap().attempt_lease = Some(predecessor.clone());
        assert!(!state.emit_provenance("T-CDX-01", RunOutcome::Done, false));
        state.threads.get_mut("T-CDX-01").unwrap().attempt_lease = Some(replacement.clone());

        // A predecessor callback that arrives after redispatch is rejected at
        // the result publication boundary and cannot append Done provenance.
        state.pending_completions.insert(
            "T-CDX-01".to_string(),
            PendingCompletion {
                completion_key: CompletionGenerationId::new(
                    CompletionId::new("T-CDX-01"),
                    AttemptId::new(predecessor.attempt_id.to_string()),
                    1,
                ),
                correlation: CorrelationId::new(
                    CompletionGenerationId::new(
                        CompletionId::new("T-CDX-01"),
                        AttemptId::new(predecessor.attempt_id.to_string()),
                        1,
                    )
                    .to_string(),
                ),
                deadline: CompletionDeadline::from_unix_millis(u64::MAX),
                is_reconciliation_replay: false,
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Review,
                source: CompletionSource::Artifact,
                authority: CompletionAuthority::Attempt(predecessor.clone()),
                completion_note: None,
            },
        );
        state.handle_completion_result("T-CDX-01", Some(0), vec![b'a'; 40], Vec::new());
        assert!(state.threads.contains_key("T-CDX-01"));
        assert!(!state.pending_completions.contains_key("T-CDX-01"));

        // An admitted completion is a lease-critical section: even a thread
        // over both budget and hard-silence thresholds cannot be reclaimed
        // while its isolated transaction is outstanding.
        state.config.session_timeout_secs = 1;
        state.config.stuck_threshold_secs = 1;
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(10);
        {
            let thread = state.threads.get_mut("T-CDX-01").unwrap();
            thread.started_at = old;
            thread.last_activity = old;
        }
        state.pending_completions.insert(
            "T-CDX-01".to_string(),
            PendingCompletion {
                completion_key: CompletionGenerationId::new(
                    CompletionId::new("T-CDX-01"),
                    AttemptId::new(replacement.attempt_id.to_string()),
                    1,
                ),
                correlation: CorrelationId::new(
                    CompletionGenerationId::new(
                        CompletionId::new("T-CDX-01"),
                        AttemptId::new(replacement.attempt_id.to_string()),
                        1,
                    )
                    .to_string(),
                ),
                deadline: CompletionDeadline::from_unix_millis(u64::MAX),
                is_reconciliation_replay: false,
                prior_phase: Phase::Review,
                prior_status: TicketStatus::Review,
                source: CompletionSource::Artifact,
                authority: CompletionAuthority::Attempt(replacement.clone()),
                completion_note: None,
            },
        );
        state.check_session_timeouts();
        assert!(state.threads.contains_key("T-CDX-01"));
        assert!(replacement.is_current(state.current_leases.get("T-CDX-01")));
        state.pending_completions.remove("T-CDX-01");

        assert!(state.dispatch_completion(CompletionInput::ObservedDone {
            ticket_id: "T-CDX-01".to_string(),
            source_lease: Some(replacement.clone()),
        }));
        lisa_core::ticket::update_ticket_done(&ticket_path).unwrap();
        state.handle_completion_result("T-CDX-01", Some(0), vec![b'b'; 40], Vec::new());
        state.handle_completion_result("T-CDX-01", Some(0), vec![b'b'; 40], Vec::new());

        let records = read_ledger(&ledger);
        assert_eq!(records.len(), 2, "history plus one winning completion");
        assert_eq!(records[0].attempt_lease, predecessor);
        assert_eq!(records[0].outcome, RunOutcome::TimedOut);
        assert!(records[0].fenced);
        assert!(!records[0].authoritative);
        assert_eq!(records[1].attempt_lease, replacement);
        assert_eq!(records[1].outcome, RunOutcome::Done);
        assert!(!records[1].fenced);
        assert!(records[1].authoritative);
        assert_eq!(
            records
                .iter()
                .filter(|record| record.outcome == RunOutcome::Done && record.authoritative)
                .count(),
            1
        );
    }

    #[test]
    fn split_brain_timeline_fences_old_attempt_and_admits_one_winner() {
        use lisa_core::types::Thread;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let tickets_dir = dir.path().join("tickets");
        let work_dir = dir.path().join("work");
        let signal_dir = dir.path().join("signals");
        let attempt_dir = dir.path().join("attempts");
        fs::create_dir_all(&tickets_dir).unwrap();
        fs::create_dir_all(&work_dir).unwrap();
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(
            tickets_dir.join("T-SPLIT.md"),
            "---\nid: T-SPLIT\ntitle: deterministic split brain\ntype: task\nstatus: review\npriority: high\nphase: review\nagent: codex\n---\n",
        )
        .unwrap();

        let tickets = lisa_core::ticket::scan_tickets(&tickets_dir).unwrap();
        let mut state = State {
            dag: Dag::from_tickets(tickets).unwrap(),
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                work_dir: work_dir.clone(),
                client: AgentClient::Codex,
                max_threads: 1,
                wind_down_secs: 0,
                session_timeout_secs: 1,
                stuck_threshold_secs: 1,
                ..PluginConfig::new()
            },
            signal_dir: signal_dir.clone(),
            attempt_dir,
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };
        let ledger = with_ledger(&mut state, &dir);

        // Attempt 1 owns pane 1 but is both over-budget and hard-silent. Pane 2
        // is the sole eligible replacement and already hosts a resident Codex
        // process, so redispatch must wait for an attempt-scoped acknowledgment.
        codex_slot(&mut state, 1, "T-SPLIT");
        state.agent_slots[0].last_client = Some(AgentClient::Codex);
        state
            .agent_slots
            .push(fresh_slot(2, Some(AgentClient::Codex)));
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(10);
        let mut predecessor_thread = Thread::new("T-SPLIT", 1);
        predecessor_thread.client = AgentClient::Codex;
        predecessor_thread.current_phase = Phase::Review;
        predecessor_thread.started_at = old;
        predecessor_thread.last_activity = old;
        state
            .threads
            .insert("T-SPLIT".to_string(), predecessor_thread);
        let predecessor = install_current_attempt(&mut state, "T-SPLIT");
        state.seat_assignments.insert(1, SeatAssignmentState::Owned);
        let predecessor_stage = state.attempt_work_dir(&predecessor);
        fs::create_dir_all(&predecessor_stage).unwrap();
        fs::write(
            predecessor_stage.join("review.md"),
            "predecessor review must remain private\n",
        )
        .unwrap();

        state.check_session_timeouts();

        assert_eq!(
            state.attempt_lifecycle,
            vec![
                AttemptLifecycleEvent::LeaseRevoked {
                    ticket_id: "T-SPLIT".to_string(),
                },
                AttemptLifecycleEvent::PaneFenced {
                    ticket_id: "T-SPLIT".to_string(),
                    pane_id: 1,
                },
                AttemptLifecycleEvent::SlotReleased {
                    ticket_id: "T-SPLIT".to_string(),
                },
            ],
            "the predecessor lease must be revoked and its pane fenced before release"
        );
        assert!(!state.threads.contains_key("T-SPLIT"));
        assert!(!state.current_leases.contains_key("T-SPLIT"));
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Fenced
        );
        assert_eq!(state.agent_slots[0].ticket_id, None);
        assert_eq!(state.agent_slots[0].attempt_lease, None);
        assert_eq!(state.seat_assignment(1), None);
        assert_eq!(state.timeout_alerts.len(), 1);
        let timeout_records = read_ledger(&ledger);
        assert_eq!(timeout_records.len(), 1);
        assert_eq!(timeout_records[0].attempt_lease, predecessor);
        assert_eq!(timeout_records[0].outcome, RunOutcome::TimedOut);
        assert!(timeout_records[0].fenced);
        assert!(!timeout_records[0].authoritative);

        state.schedule_ready_tickets();

        let replacement = state.current_leases["T-SPLIT"].clone();
        assert_eq!(replacement.attempt_id, predecessor.attempt_id + 1);
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Fenced
        );
        assert_eq!(state.agent_slots[0].ticket_id, None);
        assert_eq!(state.agent_slots[1].ticket_id.as_deref(), Some("T-SPLIT"));
        assert_eq!(
            state.agent_slots[1].attempt_lease.as_ref(),
            Some(&replacement)
        );
        assert_eq!(state.threads["T-SPLIT"].pane_id, 2);
        assert_eq!(
            state.threads["T-SPLIT"].attempt_lease.as_ref(),
            Some(&replacement)
        );
        assert_eq!(
            state.agent_slots[1].transition_state,
            TransitionState::WaitingForExit
        );
        exit_then_deliver_fresh_codex(&mut state, 2, &replacement);
        assert_eq!(state.agent_slots[1].transition_state, TransitionState::Idle);
        assert!(matches!(
            state.seat_assignment(2),
            Some(SeatAssignmentState::Delivering {
                generation,
                retries: 0,
                ..
            }) if generation == replacement.attempt_id
        ));
        assert!(
            !state.seat_is_owned(2),
            "a delivered prompt without an acknowledgment is not ownership"
        );
        assert_eq!(
            state
                .agent_slots
                .iter()
                .filter(|slot| slot.ticket_id.as_deref() == Some("T-SPLIT"))
                .count(),
            1,
            "only the replacement pane may reserve the ticket"
        );

        let replacement_thread_clock = state.threads["T-SPLIT"].last_activity;
        let replacement_pane_clock = state.agent_slots[1].last_activity_at;
        let stale_ack = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "late predecessor resume",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-SPLIT",
                    generation: predecessor.attempt_id,
                },
            ),
        });
        fs::write(
            signal_dir.join("pane-1.heartbeat"),
            serde_json::to_string(&predecessor).unwrap(),
        )
        .unwrap();
        fs::write(signal_dir.join("pane-1.ack"), stale_ack.to_string()).unwrap();
        fs::write(signal_dir.join("pane-1.idle"), "late idle").unwrap();
        fs::write(signal_dir.join("pane-1.stopped"), "late stop").unwrap();
        fs::write(signal_dir.join("pane-1.cleared"), "late clear").unwrap();
        fs::write(signal_dir.join("pane-1.error"), "late error").unwrap();

        state.check_heartbeat_signals();
        state.check_codex_ack_signals();
        state.check_idle_signals();
        state.check_transition_signals();
        state.check_error_signals();
        state.check_artifact_advances();

        for suffix in ["heartbeat", "ack", "idle", "stopped", "cleared", "error"] {
            assert!(
                !signal_dir.join(format!("pane-1.{suffix}")).exists(),
                "late {suffix} signal must be consumed without replay"
            );
        }
        assert_eq!(
            state.threads["T-SPLIT"].last_activity,
            replacement_thread_clock
        );
        assert_eq!(
            state.agent_slots[1].last_activity_at,
            replacement_pane_clock
        );
        assert_eq!(state.current_leases.get("T-SPLIT"), Some(&replacement));
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::Fenced
        );
        assert_eq!(state.agent_slots[0].ticket_id, None);
        assert_eq!(state.agent_slots[1].ticket_id.as_deref(), Some("T-SPLIT"));
        assert!(matches!(
            state.seat_assignment(2),
            Some(SeatAssignmentState::Delivering { generation, .. })
                if generation == replacement.attempt_id
        ));
        assert!(!state.seat_is_owned(2));
        assert!(state.error_alerts.is_empty());
        assert!(!state.pending_completions.contains_key("T-SPLIT"));
        assert!(!work_dir.join("T-SPLIT/review.md").exists());
        assert_eq!(
            fs::read_to_string(predecessor_stage.join("review.md")).unwrap(),
            "predecessor review must remain private\n"
        );

        assert!(
            state
                .admit_artifact("T-SPLIT", Some(&predecessor), "review.md")
                .is_err(),
            "a predecessor lease cannot publish its private artifact"
        );
        assert!(!state.dispatch_completion(CompletionInput::ObservedDone {
            ticket_id: "T-SPLIT".to_string(),
            source_lease: Some(predecessor.clone()),
        }));
        assert!(!state.pending_completions.contains_key("T-SPLIT"));
        state.threads.get_mut("T-SPLIT").unwrap().attempt_lease = Some(predecessor.clone());
        assert!(
            !state.emit_provenance("T-SPLIT", RunOutcome::Done, false),
            "a resumed predecessor cannot append authoritative provenance"
        );
        state.threads.get_mut("T-SPLIT").unwrap().attempt_lease = Some(replacement.clone());
        assert_eq!(read_ledger(&ledger).len(), 1);

        assert!(
            !state.acknowledge_codex_assignment(2, &stale_ack.to_string()),
            "the old generation cannot promote the replacement pane"
        );
        assert!(acknowledge_assignment(
            &mut state,
            2,
            "T-SPLIT",
            replacement.attempt_id,
        ));
        assert_eq!(state.seat_assignment(2), Some(SeatAssignmentState::Owned));
        assert_eq!(
            state
                .seat_assignments
                .values()
                .filter(|assignment| **assignment == SeatAssignmentState::Owned)
                .count(),
            1,
            "exactly one physical seat may own the ticket"
        );

        let replacement_stage = state.attempt_work_dir(&replacement);
        fs::create_dir_all(&replacement_stage).unwrap();
        fs::write(
            replacement_stage.join("review.md"),
            "replacement review is authoritative\n",
        )
        .unwrap();
        write_passing_review_disposition(&state, &replacement);
        state.check_artifact_advances();

        assert_eq!(
            fs::read_to_string(work_dir.join("T-SPLIT/review.md")).unwrap(),
            "replacement review is authoritative\n"
        );
        let pending = state.pending_completions.get("T-SPLIT").unwrap();
        assert_eq!(
            pending.authority,
            CompletionAuthority::Attempt(replacement.clone())
        );
        assert_eq!(pending.source, CompletionSource::Artifact);

        lisa_core::ticket::update_ticket_done(tickets_dir.join("T-SPLIT.md")).unwrap();
        let commit_id = vec![b'c'; 40];
        state.handle_completion_result("T-SPLIT", Some(0), commit_id.clone(), Vec::new());
        state.handle_completion_result("T-SPLIT", Some(0), commit_id, Vec::new());

        assert!(!state.pending_completions.contains_key("T-SPLIT"));
        assert!(!state.threads.contains_key("T-SPLIT"));
        assert!(state
            .agent_slots
            .iter()
            .all(|slot| slot.ticket_id.as_deref() != Some("T-SPLIT")));
        assert!(state
            .seat_assignments
            .values()
            .all(|assignment| { *assignment != SeatAssignmentState::Owned }));

        let records = read_ledger(&ledger);
        assert_eq!(
            records.len(),
            2,
            "timeout history plus one winning completion"
        );
        assert_eq!(records[0].attempt_lease, predecessor);
        assert_eq!(records[0].outcome, RunOutcome::TimedOut);
        assert!(records[0].fenced);
        assert!(!records[0].authoritative);
        assert_eq!(records[1].attempt_lease, replacement);
        assert_eq!(records[1].outcome, RunOutcome::Done);
        assert!(!records[1].fenced);
        assert!(records[1].authoritative);
        assert_eq!(
            records
                .iter()
                .filter(|record| record.outcome == RunOutcome::Done && record.authoritative)
                .count(),
            1,
            "only the replacement lease may publish authoritative Done"
        );
    }

    #[test]
    fn artifact_completion_publishes_only_after_verified_commit_result() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, _dir) = codex_state_with_dag();
        let ticket_path = state.config.ticket_dir.join("T-CDX-01.md");
        fs::write(
            &ticket_path,
            "---\nid: T-CDX-01\ntitle: codex-a\ntype: task\nstatus: review\npriority: high\nphase: review\nagent: codex\n---\n\nBody\n",
        )
        .unwrap();
        let tickets = lisa_core::ticket::scan_tickets(&state.config.ticket_dir).unwrap();
        state.dag = Dag::from_tickets(tickets).unwrap();
        let mut thread = Thread::new("T-CDX-01", 1);
        thread.current_phase = Phase::Review;
        state.threads.insert("T-CDX-01".to_string(), thread);
        codex_slot(&mut state, 1, "T-CDX-01");
        state.agent_slots[0].last_client = Some(AgentClient::Codex);
        state
            .last_pane_names
            .insert(1, "codex · T-CDX-01 · codex-a".to_string());
        let lease = install_current_attempt(&mut state, "T-CDX-01");
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();
        fs::write(staged.join("review.md"), "# Review\n").unwrap();
        write_passing_review_disposition(&state, &lease);

        state.check_artifact_advances();

        assert!(state.pending_completions.contains_key("T-CDX-01"));
        assert!(state.threads.contains_key("T-CDX-01"));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-CDX-01"));
        assert!(fs::read_to_string(&ticket_path)
            .unwrap()
            .contains("phase: review"));

        lisa_core::ticket::update_ticket_done(&ticket_path).unwrap();
        state.rebuild_dag();
        assert_eq!(
            state.dag.get_ticket(&"T-CDX-01".to_string()).unwrap().phase,
            Phase::Review,
            "pending Done must be masked from scheduler state"
        );

        state.handle_completion_result("T-CDX-01", Some(0), vec![b'a'; 40], Vec::new());

        assert!(!state.pending_completions.contains_key("T-CDX-01"));
        assert!(!state.threads.contains_key("T-CDX-01"));
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForExit,
            "verified completion must retire the resident Codex TUI"
        );
        assert!(!state.agent_slots[0].has_session);
        assert_eq!(
            state.last_pane_names.get(&1).map(String::as_str),
            Some("codex · idle")
        );
        let ticket = state.dag.get_ticket(&"T-CDX-01".to_string()).unwrap();
        assert_eq!(ticket.phase, Phase::Done);
        assert_eq!(ticket.status, TicketStatus::Done);
    }

    #[test]
    fn completion_journal_reconstructs_restart_states_before_authoritative_provenance() {
        use lisa_core::provenance::ProvenanceLedgerRecord;
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);
        let journal = dir.path().join("completion-journal.jsonl");
        state.completion_journal_path = journal.clone();
        state.restore_completion_journal();
        state.project_root = dir.path().to_path_buf();
        state.git_root = dir.path().to_path_buf();
        state.config.git_root = dir.path().to_path_buf();
        state.config.lisa_bin = Some("/usr/local/bin/lisa".to_string());

        let ticket_id = "T-CDX-01";
        let ticket_path = state.config.ticket_dir.join(format!("{ticket_id}.md"));
        fs::write(
            &ticket_path,
            "---\nid: T-CDX-01\ntitle: codex-a\ntype: task\nstatus: review\npriority: high\nphase: review\nagent: codex\n---\n\nBody\n",
        )
        .unwrap();
        state.dag =
            Dag::from_tickets(lisa_core::ticket::scan_tickets(&state.config.ticket_dir).unwrap())
                .unwrap();
        let mut thread = Thread::new(ticket_id, 1);
        thread.current_phase = Phase::Review;
        thread.client = AgentClient::Codex;
        state.threads.insert(ticket_id.to_string(), thread);
        codex_slot(&mut state, 1, ticket_id);
        let lease = install_current_attempt(&mut state, ticket_id);
        let staged = state.attempt_work_dir(&lease);
        fs::create_dir_all(&staged).unwrap();
        fs::write(staged.join("review.md"), "# Review\n").unwrap();
        write_passing_review_disposition(&state, &lease);

        state.check_artifact_advances();

        let in_flight = state
            .completion_aggregates
            .get(ticket_id)
            .cloned()
            .expect("adapter must retain its durable aggregate");
        let expected_key = CompletionGenerationId::new(
            CompletionId::new(ticket_id),
            AttemptId::new(lease.attempt_id.to_string()),
            1,
        );
        assert_eq!(in_flight.completion_key(), &expected_key);
        assert_eq!(in_flight.prior_phase(), Phase::Review);
        assert_eq!(in_flight.prior_status(), TicketStatus::Review);
        let expected_deadline = state.pending_completions[ticket_id].deadline;
        assert_eq!(
            in_flight.state(),
            &CompletionState::CommandInFlight {
                correlation: CorrelationId::new(expected_key.to_string()),
                deadline: expected_deadline,
            }
        );
        assert!(state.pending_completions.contains_key(ticket_id));
        assert!(
            !ledger.exists(),
            "in-flight state cannot emit Done provenance"
        );

        let journal_body = fs::read_to_string(&journal).unwrap();
        assert_eq!(journal_body.lines().count(), 2);
        assert_eq!(
            journal_body
                .lines()
                .filter(|line| line.contains("\"state\":\"requested\""))
                .count(),
            1
        );
        assert_eq!(
            journal_body
                .lines()
                .filter(|line| line.contains("\"state\":\"command-in-flight\""))
                .count(),
            1
        );

        let mut restarted = State {
            completion_journal_path: journal.clone(),
            ..State::default()
        };
        restarted.restore_completion_journal();
        assert!(restarted.completion_journal_healthy);
        assert_eq!(
            restarted.completion_aggregates.get(ticket_id),
            Some(&in_flight),
            "a fresh plugin state must reconstruct the exact in-flight aggregate"
        );
        assert_eq!(
            restarted.reconciliation_state(ticket_id),
            in_flight.state().clone()
        );

        lisa_core::ticket::update_ticket_done(&ticket_path).unwrap();
        let mut restarted_scan = lisa_core::ticket::scan_tickets(&state.config.ticket_dir).unwrap();
        let scanned = restarted_scan
            .iter_mut()
            .find(|ticket| ticket.id == ticket_id)
            .unwrap();
        assert_eq!(scanned.phase, Phase::Done);
        restarted.mask_completion_transaction(scanned);
        assert_eq!(scanned.phase, Phase::Review);
        assert_eq!(scanned.status, TicketStatus::Review);
        restarted.dag = Dag::from_tickets(restarted_scan).unwrap();
        restarted.config = state.config.clone();
        restarted.permissions_granted = true;
        restarted.slots_discovered = true;
        codex_slot(&mut restarted, 7, ticket_id);
        restarted.agent_slots[0].ticket_id = None;
        restarted.schedule_ready_tickets();
        assert!(
            restarted.threads.is_empty(),
            "an unresolved reconstructed completion must fence replacement scheduling"
        );

        state.rebuild_dag();
        assert_eq!(
            state.dag.get_ticket(&ticket_id.to_string()).unwrap().phase,
            Phase::Review,
            "live journal/pending state must mask Done until confirmation"
        );
        let commit_id = "a".repeat(40);
        state.handle_completion_result(
            ticket_id,
            Some(0),
            commit_id.as_bytes().to_vec(),
            Vec::new(),
        );

        assert!(!state.pending_completions.contains_key(ticket_id));
        let confirmed = state.completion_aggregates.get(ticket_id).unwrap();
        assert_eq!(confirmed.state(), &CompletionState::Confirmed);
        assert_eq!(confirmed.confirmed_commit_id(), Some(commit_id.as_str()));

        let mut confirmed_restart = State {
            completion_journal_path: journal.clone(),
            ..State::default()
        };
        confirmed_restart.restore_completion_journal();
        assert_eq!(
            confirmed_restart.completion_aggregates.get(ticket_id),
            Some(confirmed)
        );
        assert_eq!(
            confirmed_restart.reconciliation_state(ticket_id),
            CompletionState::Confirmed
        );

        let journal_body = fs::read_to_string(&journal).unwrap();
        assert_eq!(journal_body.lines().count(), 3);
        assert_eq!(
            journal_body
                .lines()
                .filter(|line| line.contains("\"state\":\"confirmed\""))
                .count(),
            1
        );
        assert_eq!(
            fs::read_dir(dir.path())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .contains("journal.jsonl.tmp"))
                .count(),
            0,
            "atomic publication must leave no sibling temporary"
        );

        let records = read_mixed_ledger(&ledger);
        assert_eq!(records.len(), 1);
        match &records[0] {
            ProvenanceLedgerRecord::Execution(record) => {
                assert_eq!(record.ticket_id, ticket_id);
                assert_eq!(record.outcome, RunOutcome::Done);
                assert!(record.authoritative);
            }
            other => {
                panic!("completion must retain the legacy execution provenance shape: {other:?}")
            }
        }
    }

    #[test]
    fn lost_result_reload_duplicate_stop_replay_converges_on_single_prior_commit() {
        use lisa_cli::commit_transaction::{complete_ticket, CompleteTicketRequest};
        use lisa_core::provenance::ProvenanceLedgerRecord;
        use lisa_core::types::Thread;
        use std::process::Command;

        fn git(root: &Path, args: &[&str]) -> String {
            let output = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout).unwrap().trim().to_string()
        }

        const TICKET_ID: &str = "T-REPLAY";
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let tickets_dir = root.join("docs/active/tickets");
        let work_dir = root.join("docs/active/work");
        let journal = root.join(".lisa/completion-journal.jsonl");
        let ledger = root.join(".lisa/provenance.jsonl");
        std::fs::create_dir_all(&tickets_dir).unwrap();
        std::fs::create_dir_all(&work_dir).unwrap();
        std::fs::write(
            tickets_dir.join(format!("{TICKET_ID}.md")),
            format!(
                "---\nid: {TICKET_ID}\ntitle: replay\ntype: task\nstatus: review\npriority: critical\nphase: review\n---\n\nReplay fixture\n"
            ),
        )
        .unwrap();
        git(root, &["init", "--quiet"]);
        git(root, &["config", "user.name", "Lisa Test"]);
        git(root, &["config", "user.email", "lisa@example.test"]);
        git(root, &["add", "docs/active/tickets/T-REPLAY.md"]);
        git(root, &["commit", "--quiet", "-m", "base"]);
        let base_commit_count = git(root, &["rev-list", "--count", "HEAD"])
            .parse::<u64>()
            .unwrap();

        let (mut state, lease) = review_timeout_state(
            TICKET_ID,
            tickets_dir,
            work_dir,
            root.to_path_buf(),
            root.to_path_buf(),
            journal.clone(),
        );
        state.attempt_dir = root.join(".lisa/attempts");
        state.ledger_path = ledger.clone();
        state.codex_dir = root.join(".lisa/codex");
        state.claude_dir = root.join(".lisa/claude");
        codex_slot(&mut state, 1, TICKET_ID);
        state.agent_slots[0].attempt_lease = Some(lease.clone());
        std::fs::create_dir_all(state.attempt_work_dir(&lease)).unwrap();
        write_private_review(&state, &lease);

        let initial_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000);
        assert!(state.dispatch_completion_at(
            CompletionInput::Reconcile {
                ticket_id: TICKET_ID.to_string(),
                source_lease: lease.clone(),
            },
            initial_time,
        ));
        let original_pending = state.pending_completions[TICKET_ID].clone();
        let request = || CompleteTicketRequest {
            repo_root: root.to_path_buf(),
            ticket_id: TICKET_ID.to_string(),
            message: format!("Complete {TICKET_ID}"),
            ticket_file: PathBuf::from("docs/active/tickets/T-REPLAY.md"),
            work_dir: PathBuf::from("docs/active/work/T-REPLAY"),
            completion_key: original_pending.completion_key.clone(),
        };

        let first = complete_ticket(request()).unwrap();
        assert_eq!(
            git(root, &["rev-list", "--count", "HEAD"])
                .parse::<u64>()
                .unwrap(),
            base_commit_count + 1
        );
        assert!(
            git(root, &["show", "HEAD:docs/active/tickets/T-REPLAY.md"]).contains("status: done")
        );
        assert_eq!(
            std::fs::read_to_string(&journal).unwrap().lines().count(),
            2,
            "the successful result is deliberately lost before confirmation"
        );

        let mut restarted = State {
            config: state.config.clone(),
            project_root: root.to_path_buf(),
            git_root: root.to_path_buf(),
            attempt_dir: state.attempt_dir.clone(),
            ledger_path: ledger.clone(),
            completion_journal_path: journal.clone(),
            codex_dir: state.codex_dir.clone(),
            claude_dir: state.claude_dir.clone(),
            ..State::default()
        };
        restarted.restore_completion_journal();
        restarted.rebuild_dag();
        let mut thread = Thread::new(TICKET_ID, 7);
        thread.current_phase = Phase::Review;
        thread.client = AgentClient::Codex;
        thread.attempt_lease = Some(lease.clone());
        restarted.threads.insert(TICKET_ID.to_string(), thread);
        restarted
            .current_leases
            .insert(TICKET_ID.to_string(), lease.clone());
        restarted
            .lease_high_water
            .insert(TICKET_ID.to_string(), lease.clone());
        codex_slot(&mut restarted, 7, TICKET_ID);
        restarted.agent_slots[0].attempt_lease = Some(lease.clone());

        assert!(!restarted.dispatch_completion_at(
            CompletionInput::Stopped {
                ticket_id: TICKET_ID.to_string(),
                pane_id: 7,
                source_lease: lease.clone(),
            },
            initial_time + std::time::Duration::from_secs(1),
        ));
        assert!(restarted.dispatch_completion_at(
            CompletionInput::Reconcile {
                ticket_id: TICKET_ID.to_string(),
                source_lease: lease.clone(),
            },
            initial_time + std::time::Duration::from_secs(1),
        ));
        assert_eq!(
            restarted.pending_completions[TICKET_ID].completion_key,
            original_pending.completion_key
        );
        assert_eq!(restarted.launched_completion_effects.len(), 1);
        assert!(!restarted.dispatch_completion_at(
            CompletionInput::Stopped {
                ticket_id: TICKET_ID.to_string(),
                pane_id: 7,
                source_lease: lease.clone(),
            },
            initial_time + std::time::Duration::from_secs(2),
        ));
        assert!(!restarted.dispatch_completion_at(
            CompletionInput::Reconcile {
                ticket_id: TICKET_ID.to_string(),
                source_lease: lease,
            },
            initial_time + std::time::Duration::from_secs(2),
        ));
        assert_eq!(restarted.launched_completion_effects.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&journal).unwrap().lines().count(),
            2,
            "replay must not append duplicate intent or in-flight records"
        );

        let replay = complete_ticket(request()).unwrap();
        assert_eq!(replay.commit_id, first.commit_id);
        assert!(replay.committed_paths.is_empty());
        assert_eq!(
            git(root, &["rev-list", "--count", "HEAD"])
                .parse::<u64>()
                .unwrap(),
            base_commit_count + 1,
            "same-key replay must discover rather than duplicate the completion commit"
        );

        restarted.handle_completion_result(
            TICKET_ID,
            Some(0),
            replay.commit_id.as_bytes().to_vec(),
            Vec::new(),
        );
        assert!(!restarted.pending_completions.contains_key(TICKET_ID));
        assert_eq!(
            restarted.completion_aggregates[TICKET_ID].state(),
            &CompletionState::Confirmed
        );
        assert_eq!(
            std::fs::read_to_string(&journal)
                .unwrap()
                .lines()
                .filter(|line| line.contains("\"state\":\"confirmed\""))
                .count(),
            1
        );
        assert!(!restarted.threads.contains_key(TICKET_ID));
        assert!(restarted.agent_slots[0].ticket_id.is_none());

        let records = read_mixed_ledger(&ledger);
        assert_eq!(records.len(), 1);
        match &records[0] {
            ProvenanceLedgerRecord::Execution(record) => {
                assert_eq!(record.ticket_id, TICKET_ID);
                assert_eq!(record.outcome, RunOutcome::Done);
                assert!(record.authoritative);
            }
            other => panic!("expected one authoritative Done record, got {other:?}"),
        }
    }

    #[test]
    fn reconciliation_deadline_parks_and_ordinary_unpark_restores_eligibility() {
        const TICKET_ID: &str = "T-DEADLINE";
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let tickets_dir = root.join("docs/active/tickets");
        let work_dir = root.join("docs/active/work");
        let journal = root.join(".lisa/completion-journal.jsonl");
        let ledger = root.join(".lisa/provenance.jsonl");
        std::fs::create_dir_all(&tickets_dir).unwrap();
        std::fs::create_dir_all(&work_dir).unwrap();
        let ticket_path = tickets_dir.join(format!("{TICKET_ID}.md"));
        std::fs::write(
            &ticket_path,
            format!(
                "---\nid: {TICKET_ID}\ntitle: deadline\ntype: task\nstatus: review\npriority: critical\nphase: review\n---\n\nDeadline fixture\n"
            ),
        )
        .unwrap();
        let (mut state, lease) = review_timeout_state(
            TICKET_ID,
            tickets_dir,
            work_dir,
            root.to_path_buf(),
            root.to_path_buf(),
            journal.clone(),
        );
        state.attempt_dir = root.join(".lisa/attempts");
        state.ledger_path = ledger.clone();
        std::fs::create_dir_all(state.attempt_work_dir(&lease)).unwrap();
        write_private_review(&state, &lease);
        let initial_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(2_000);
        assert!(state.dispatch_completion_at(
            CompletionInput::Reconcile {
                ticket_id: TICKET_ID.to_string(),
                source_lease: lease.clone(),
            },
            initial_time,
        ));
        let deadline = state.pending_completions[TICKET_ID].deadline;
        assert_eq!(
            deadline.unix_millis(),
            State::completion_time(initial_time).unix_millis()
                + COMPLETION_RECONCILIATION_TIMEOUT_SECS * 1_000
        );

        lisa_core::ticket::update_ticket_done(&ticket_path).unwrap();
        let deadline_time =
            std::time::UNIX_EPOCH + std::time::Duration::from_millis(deadline.unix_millis());
        assert!(state.dispatch_completion_at(
            CompletionInput::Reconcile {
                ticket_id: TICKET_ID.to_string(),
                source_lease: lease.clone(),
            },
            deadline_time,
        ));
        assert!(!state.pending_completions.contains_key(TICKET_ID));
        assert!(matches!(
            state.completion_aggregates[TICKET_ID].state(),
            CompletionState::Rejected {
                retryability: Retryability::ActionRequired,
                ..
            }
        ));
        assert_eq!(
            state.dag.get_ticket(&TICKET_ID.to_string()).unwrap().phase,
            Phase::Review,
            "uncertain Done bytes are restored to Review before parking"
        );
        assert_eq!(
            state.dag.get_ticket(&TICKET_ID.to_string()).unwrap().status,
            TicketStatus::Blocked
        );
        assert!(!state.threads.contains_key(TICKET_ID));
        assert!(matches!(
            parse_review_disposition(
                state
                    .config
                    .work_dir
                    .join(TICKET_ID)
                    .join("review-disposition.json")
            ),
            ReviewDisposition::Block {
                remedy_owner: RemedyOwner::Operator,
                ask,
                unstructured: false,
                ..
            } if ask.starts_with("Lisa could not confirm whether finished work was recorded.")
        ));
        let records = read_mixed_ledger(&ledger);
        let ProvenanceLedgerRecord::ParkingTransition(park) = &records[0] else {
            panic!("expected deadline Park provenance")
        };
        assert_eq!(park.record_type, ParkingTransitionType::Park);
        assert_eq!(state.launched_completion_effects.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&journal).unwrap().lines().count(),
            3
        );

        assert!(!state.dispatch_completion_at(
            CompletionInput::Reconcile {
                ticket_id: TICKET_ID.to_string(),
                source_lease: lease.clone(),
            },
            deadline_time + std::time::Duration::from_secs(60),
        ));
        assert!(!state.dispatch_completion_at(
            CompletionInput::Stopped {
                ticket_id: TICKET_ID.to_string(),
                pane_id: 42,
                source_lease: lease,
            },
            deadline_time + std::time::Duration::from_secs(120),
        ));
        assert_eq!(state.launched_completion_effects.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&journal).unwrap().lines().count(),
            3
        );

        ticket::update_ticket_status(&ticket_path, TicketStatus::Open).unwrap();
        state.rebuild_dag();
        assert_eq!(
            state.reconciliation_state(TICKET_ID),
            CompletionState::Eligible
        );
        state.reconcile_unpark_transitions();
        let records = read_mixed_ledger(&ledger);
        let ProvenanceLedgerRecord::ParkingTransition(unpark) = &records[1] else {
            panic!("expected deadline Unpark provenance")
        };
        assert_eq!(unpark.record_type, ParkingTransitionType::Unpark);

        let mut replacement = Thread::new(TICKET_ID, 43);
        replacement.current_phase = Phase::Review;
        state.threads.insert(TICKET_ID.to_string(), replacement);
        let replacement_lease = install_current_attempt(&mut state, TICKET_ID);
        assert_eq!(replacement_lease.attempt_id, 2);
        std::fs::create_dir_all(state.attempt_work_dir(&replacement_lease)).unwrap();
        write_private_review(&state, &replacement_lease);
        assert!(state.dispatch_completion(CompletionInput::Reconcile {
            ticket_id: TICKET_ID.to_string(),
            source_lease: replacement_lease,
        }));
        assert_eq!(
            state.completion_aggregates[TICKET_ID]
                .completion_key()
                .attempt_id()
                .as_str(),
            "2"
        );
    }

    #[test]
    fn failed_operator_completion_retries_without_early_release_or_duplicate_provenance() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);
        let ticket_path = state.config.ticket_dir.join("T-CDX-01.md");
        fs::write(
            &ticket_path,
            "---\nid: T-CDX-01\ntitle: codex-a\ntype: task\nstatus: review\npriority: high\nphase: review\nagent: codex\n---\n\nBody\n",
        )
        .unwrap();
        let tickets = lisa_core::ticket::scan_tickets(&state.config.ticket_dir).unwrap();
        state.dag = Dag::from_tickets(tickets).unwrap();
        let mut thread = Thread::new("T-CDX-01", 1);
        thread.current_phase = Phase::Review;
        thread.client = AgentClient::Codex;
        state.threads.insert("T-CDX-01".to_string(), thread);
        codex_slot(&mut state, 1, "T-CDX-01");
        state.agent_slots[0].last_client = Some(AgentClient::Codex);
        state
            .last_pane_names
            .insert(1, "codex · T-CDX-01 · codex-a".to_string());
        install_current_attempt(&mut state, "T-CDX-01");
        write_canonical_review_disposition(
            &state,
            "T-CDX-01",
            r#"{"disposition":"pass","reason":null}"#,
        );

        state.open_mark_done_modal();
        state.modal.cursor = state
            .modal
            .ticket_ids
            .iter()
            .position(|ticket_id| ticket_id == "T-CDX-01")
            .unwrap();
        assert!(state.handle_key(KeyWithModifier {
            bare_key: BareKey::Enter,
            key_modifiers: Default::default(),
        }));
        assert!(matches!(
            state.pending_completions.get("T-CDX-01").map(|p| p.source),
            Some(CompletionSource::OperatorRequested(
                OperatorRequestSource::MarkDoneKey
            ))
        ));
        let first_correlation = state.pending_completions["T-CDX-01"]
            .completion_key
            .to_string();
        state.handle_completion_result(
            "T-CDX-01",
            Some(1),
            Vec::new(),
            b"identity unavailable".to_vec(),
        );

        assert!(!state.pending_completions.contains_key("T-CDX-01"));
        assert!(state.threads.contains_key("T-CDX-01"));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-CDX-01"));
        assert_eq!(
            state.last_pane_names.get(&1).map(String::as_str),
            Some("codex · T-CDX-01 · codex-a"),
            "failed completion must retain the assigned pane title"
        );
        assert!(!state
            .dag
            .get_ready_tickets()
            .contains(&"T-CDX-02".to_string()));
        assert!(!ledger.exists(), "failed attempts must not emit provenance");
        assert!(state.activity_log.iter().any(|event| matches!(
            event,
            ActivityEvent::CompletionRejected {
                kind: CompletionRejectionKind::LaunchFailed,
                detail,
                correlation_id,
                ..
            } if detail.contains("identity unavailable")
                && detail.contains("recoverable")
                && !correlation_id.is_empty()
        )));
        assert!(state.modal.open);
        assert!(matches!(
            state.modal.operator_outcome.as_ref(),
            Some(OperatorModalOutcome::Rejected {
                kind: CompletionRejectionKind::LaunchFailed,
                correlation_id,
                detail,
                ..
            }) if correlation_id == &first_correlation
                && detail.contains("identity unavailable")
                && detail.contains("recoverable")
        ));
        assert!(state.handle_key(KeyWithModifier {
            bare_key: BareKey::Enter,
            key_modifiers: Default::default(),
        }));
        assert!(!state.modal.open);

        state.open_mark_done_modal();
        state.modal.cursor = state
            .modal
            .ticket_ids
            .iter()
            .position(|ticket_id| ticket_id == "T-CDX-01")
            .unwrap();
        assert!(state.handle_key(KeyWithModifier {
            bare_key: BareKey::Enter,
            key_modifiers: Default::default(),
        }));
        let retry_correlation = state.pending_completions["T-CDX-01"]
            .completion_key
            .to_string();
        lisa_core::ticket::update_ticket_done(&ticket_path).unwrap();
        state.rebuild_dag();
        assert_eq!(
            state.dag.get_ticket(&"T-CDX-01".to_string()).unwrap().phase,
            Phase::Review
        );
        assert!(!state
            .dag
            .get_ready_tickets()
            .contains(&"T-CDX-02".to_string()));

        state.handle_completion_result("T-CDX-01", Some(0), vec![b'b'; 40], Vec::new());
        assert!(!state.threads.contains_key("T-CDX-01"));
        assert!(state.agent_slots[0].ticket_id.is_none());
        assert_eq!(
            state.last_pane_names.get(&1).map(String::as_str),
            Some("codex · idle")
        );
        assert!(state
            .dag
            .get_ready_tickets()
            .contains(&"T-CDX-02".to_string()));
        assert_eq!(read_ledger(&ledger).len(), 1);
        assert!(state.modal.open);
        assert_eq!(
            state.modal.operator_outcome,
            Some(OperatorModalOutcome::Accepted {
                ticket_id: "T-CDX-01".to_string(),
                correlation_id: retry_correlation,
            })
        );
        assert!(state.handle_key(KeyWithModifier {
            bare_key: BareKey::Esc,
            key_modifiers: Default::default(),
        }));
        assert!(!state.modal.open);

        state.handle_completion_result("T-CDX-01", Some(0), vec![b'b'; 40], Vec::new());
        assert_eq!(read_ledger(&ledger).len(), 1);
    }
}

// wasm32-wasip1 + cdylib produces a reactor module (no entry point).
// Zellij expects a command-style _start export to initialize the WASM instance.
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn __wasm_call_ctors();
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn _start() {
    unsafe {
        __wasm_call_ctors();
    }
}
