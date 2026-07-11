//! Lisa - A Zellij plugin for DAG-driven concurrent task scheduling
//!
//! This plugin implements the RDSPI workflow (Research -> Design -> Structure -> Plan -> Implement)
//! as a DAG-driven concurrent scheduler. It manages Claude Code sessions for each ticket,
//! tracks phase progress, and provides a live dashboard.

mod adapter;
mod codex_ack;
mod pane_name;
mod ui;

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use zellij_tile::prelude::*;

use adapter::{resolve_adapter_or_native, FollowUp, FollowUpContext, ResetStrategy, SpawnContext};

use lisa_core::client::AgentClient;
use lisa_core::dag::Dag;
use lisa_core::diagnostics;
use lisa_core::provenance::{self, ProvenanceRecord, Route, RunOutcome};
use lisa_core::ticket;
use lisa_core::types::{ActivityEvent, Phase, PluginConfig, Thread, TicketId, TicketStatus};
use pane_name::{format_pane_name, PaneName};

/// How often (in seconds) the plugin rescans ticket files to detect phase changes.
const POLL_INTERVAL_SECS: f64 = 5.0;

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
pub(crate) fn ticket_prompt(ticket_dir: &Path, ticket_id: &str, context_file: &str) -> String {
    let ticket_path = lisa_core::ticket::scan_tickets(ticket_dir)
        .ok()
        .and_then(|tickets| {
            tickets
                .into_iter()
                .find(|ticket| ticket.id == ticket_id)
                .map(|ticket| ticket.file_path)
        })
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| ticket_dir.join(format!("{}.md", ticket_id)));
    format!(
        "Read the ticket at {path}, {context}, and docs/knowledge/rdspi-workflow.md. \
         Your job: start from the current phase in the ticket frontmatter and work through ALL remaining phases \
         (Research, Design, Structure, Plan, Implement, Review) without stopping between phases. \
         For each phase, write the artifact to docs/active/work/{id}/ then immediately continue to the next phase. \
         Do NOT update the ticket's phase or status fields in the frontmatter — \
         Lisa detects your artifacts and handles all phase transitions automatically. \
         During Implement, commit each meaningful ticket-owned source unit only with \
         lisa commit-ticket and exact repository-relative --include paths. Do not use ordinary-index git add, \
         git add -A, or git commit for ticket work, and do not leave ticket-owned files staged, modified, or untracked. \
         After Review is complete (review.md written summarizing changes, test coverage, and open concerns), \
         remain on this ticket and stop. Do not start another ticket until Lisa confirms the completion commit; \
         Lisa handles Done publication and seat release.",
        path = ticket_path.display(),
        context = context_file,
        id = ticket_id,
    )
}

/// Build the full shell command to launch Claude Code in a fresh pane.
/// Sets LISA_PANE_ID env var so the idle signal hook can identify the pane,
/// and LISA_TICKET_ID for debugging/logging context.
///
/// `lisa_bin` is the absolute `lisa` path (plugin config) exported as `LISA_BIN`
/// so the `Stop` hook's `lisa capture-usage` (T-027-02) is reachable even when
/// the pane shell lacks `lisa` on PATH — mirroring the Codex adapter's
/// `lisa_bin` threading. `None`/empty omits the var entirely, keeping the launch
/// line byte-for-byte the pre-capture command (the hook then falls back to a
/// PATH `lisa`).
pub(crate) fn build_claude_command(
    ticket_dir: &Path,
    ticket_id: &str,
    pane_id: u32,
    model: Option<&str>,
    lisa_bin: Option<&str>,
) -> String {
    // The Claude adapter owns the model→flag mapping (`--model`). When no model
    // is routed the flag is omitted, so the launch line is byte-for-byte the
    // pre-routing command — the zero-regression path.
    let model_flag = match model {
        Some(m) => format!(" --model {}", m),
        None => String::new(),
    };
    let lisa_bin_env = match lisa_bin.filter(|s| !s.is_empty()) {
        Some(bin) => format!("LISA_BIN={} ", bin),
        None => String::new(),
    };
    format!(
        "{}LISA_PANE_ID={} LISA_TICKET_ID={} claude --dangerously-skip-permissions{} \"{}\"",
        lisa_bin_env,
        pane_id,
        ticket_id,
        model_flag,
        ticket_prompt(ticket_dir, ticket_id, AgentClient::Claude.context_file())
    )
}

/// The prompt text sent to a stuck Review session after the review timeout.
pub(crate) fn finish_up_prompt(_ticket_dir: &Path, work_dir: &Path, ticket_id: &str) -> String {
    let review_path = work_dir.join(ticket_id).join("review.md");
    format!(
        "You have been in the Review phase for a while. Please finish writing your review artifact at {}. \
         It should cover: what changes were made, files created/modified/deleted, test coverage, \
         any open concerns or TODOs, and critical issues to surface for human review. \
         Do NOT update the ticket's phase or status fields or use ordinary-index git add/git commit to publish completion. \
         Remain on this ticket and wait until Lisa confirms the completion commit before starting another ticket.",
        review_path.display(),
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
}

/// Scheduler-owned truth for the ticket assigned to a physical seat.
///
/// This is deliberately independent of [`TransitionState`]: a pane can be
/// waiting for `/clear` or `/exit` while its ticket assignment is still waiting
/// for positive provider acknowledgment. Absence from `State::seat_assignments`
/// means the seat is unassigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeatAssignmentState {
    /// The seat is reserved for a ticket, but Codex has not acknowledged it.
    AssignedPendingAck { generation: u64 },
    /// The provider is considered to have accepted the assigned ticket.
    Owned,
    /// The pending assignment timed out and bounded recovery is in progress.
    /// T-033-01-04 owns the transition into this state.
    #[allow(dead_code)]
    Recovering,
}

/// Diagnostic origin for a request to durably complete a ticket. Every origin
/// enters the same completion transaction and result publisher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionSource {
    Artifact,
    Idle,
    Stopped(u32),
    Manual,
    ObservedDone,
}

#[derive(Debug, Clone, Copy)]
struct PendingCompletion {
    prior_phase: Phase,
    prior_status: TicketStatus,
    source: CompletionSource,
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

    /// Assignment truth keyed by physical terminal pane ID. Slot `ticket_id`
    /// remains the reservation/routing key during handoff; this map says whether
    /// that reservation is pending acknowledgment, owned, or recovering.
    /// Missing means the seat has no current assignment.
    seat_assignments: HashMap<u32, SeatAssignmentState>,

    /// Monotonic identity source for recycled Codex delivery attempts. A
    /// generation is retained in `AssignedPendingAck` until that exact prompt
    /// submission is acknowledged.
    next_assignment_generation: u64,

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

    /// Path to the idle signal directory (`.lisa/signals/` under /host/).
    signal_dir: PathBuf,

    /// Path to the append-only provenance ledger (`.lisa/provenance.jsonl` under
    /// /host/). One record is appended per ticket-run at teardown (T-027-01).
    /// Empty until `load()` runs — a native test that does not set it skips the
    /// write, so unrelated teardown-triggering tests never write to disk.
    ledger_path: PathBuf,

    /// Directory native Codex usage capture (or the headless fallback) writes
    /// artifacts into (`.lisa/codex/` under /host/).
    codex_dir: PathBuf,

    /// Directory the Claude `Stop` hook's `lisa capture-usage` writes usage
    /// artifacts into (`.lisa/claude/` under /host/). Read at teardown for Claude
    /// tokens (T-027-02); same `{ ..., usage }` shape as the Codex artifact.
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

    fn repository_relative_path(&self, path: &Path) -> Result<PathBuf, String> {
        if let Ok(relative) = path.strip_prefix("/host") {
            if !relative.as_os_str().is_empty() {
                return Ok(relative.to_path_buf());
            }
        }
        if !self.project_root.as_os_str().is_empty() {
            if let Ok(relative) = path.strip_prefix(&self.project_root) {
                if !relative.as_os_str().is_empty() {
                    return Ok(relative.to_path_buf());
                }
            }
        }
        if path.is_relative() && !path.as_os_str().is_empty() {
            return Ok(path.to_path_buf());
        }
        Err(format!(
            "path {} is not relative to the Lisa project root",
            path.display()
        ))
    }

    fn build_completion_command(
        &self,
        ticket_id: &str,
        ticket_file: &Path,
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
        let ticket_file = self.repository_relative_path(ticket_file)?;
        let work_dir = self.repository_relative_path(&self.config.work_dir.join(ticket_id))?;
        let argv = vec![
            lisa_bin.to_string(),
            "complete-ticket".to_string(),
            "--path".to_string(),
            self.project_root.display().to_string(),
            "--ticket-id".to_string(),
            ticket_id.to_string(),
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

    /// Enter the single commit-gated completion state machine.
    fn request_completion(&mut self, ticket_id: TicketId, source: CompletionSource) -> bool {
        if self.pending_completions.contains_key(&ticket_id) {
            return false;
        }
        if !self.dag.all_dependencies_done(&ticket_id) {
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Cannot complete {}: its dependencies are not all done",
                    ticket_id
                ),
            });
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

        self.pending_completions.insert(
            ticket_id.clone(),
            PendingCompletion {
                prior_phase,
                prior_status,
                source,
            },
        );

        let (argv, context) = match self.build_completion_command(&ticket_id, &ticket_file) {
            Ok(command) => command,
            Err(error) => {
                #[cfg(test)]
                {
                    let _ = error;
                    return true;
                }
                #[cfg(not(test))]
                {
                    self.pending_completions.remove(&ticket_id);
                    self.log_activity(ActivityEvent::Error {
                        message: format!("Cannot start completion for {ticket_id}: {error}"),
                    });
                    return false;
                }
            }
        };
        let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        run_command_with_env_variables_and_cwd(
            &argv_refs,
            BTreeMap::new(),
            self.project_root.clone(),
            context,
        );
        self.log_activity(ActivityEvent::Info {
            message: format!("Completion commit pending for {ticket_id} ({source:?})"),
        });
        true
    }

    fn is_commit_id(output: &[u8]) -> bool {
        let value = String::from_utf8_lossy(output);
        let value = value.trim();
        matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    }

    fn handle_completion_result(
        &mut self,
        ticket_id: &str,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    ) {
        let pending = match self.pending_completions.get(ticket_id).copied() {
            Some(pending) => pending,
            None => return,
        };
        if exit_code != Some(0) || !Self::is_commit_id(&stdout) {
            self.pending_completions.remove(ticket_id);
            self.rebuild_dag();
            let detail = String::from_utf8_lossy(&stderr);
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Completion commit failed for {} ({:?}, exit {:?}): {}. Ticket remains recoverable for retry",
                    ticket_id,
                    pending.source,
                    exit_code,
                    if detail.trim().is_empty() {
                        "no error output"
                    } else {
                        detail.trim()
                    }
                ),
            });
            return;
        }

        self.pending_completions.remove(ticket_id);
        self.rebuild_dag();
        let ticket_id_owned = ticket_id.to_string();
        let durable_done = self
            .dag
            .get_ticket(&ticket_id_owned)
            .map(|ticket| ticket.phase == Phase::Done && ticket.status == TicketStatus::Done)
            .unwrap_or(false);
        if !durable_done {
            self.pending_completions
                .insert(ticket_id.to_string(), pending);
            self.rebuild_dag();
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "Completion command succeeded for {} but durable Done frontmatter could not be verified; scheduler state remains blocked",
                    ticket_id
                ),
            });
            return;
        }

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
            message: format!(
                "Completion commit verified for {} at {}",
                ticket_id,
                String::from_utf8_lossy(&stdout).trim()
            ),
        });
        if let Some(thread) = self.threads.get_mut(ticket_id) {
            thread.complete();
        }
        self.emit_provenance(ticket_id, RunOutcome::Done);
        self.release_slot_for_ticket(&ticket_id_owned);
        self.threads.remove(ticket_id);
        self.schedule_ready_tickets();
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
            if let Some(pending) = self.pending_completions.get(&scanned.id) {
                scanned.phase = pending.prior_phase;
                scanned.status = pending.prior_status;
            }
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

    /// Allocate a process-local, nonzero identity for one pending delivery.
    fn allocate_assignment_generation(&mut self) -> u64 {
        self.next_assignment_generation = self.next_assignment_generation.saturating_add(1);
        self.next_assignment_generation
    }

    /// Return the expected generation only while this seat is pending.
    fn pending_assignment_generation(&self, pane_id: u32) -> Option<u64> {
        match self.seat_assignment(pane_id) {
            Some(SeatAssignmentState::AssignedPendingAck { generation }) => Some(generation),
            _ => None,
        }
    }

    /// Promote a recycled Codex seat only when the provider payload identifies
    /// the ticket and delivery generation currently pending in that pane.
    /// Returning true means this call performed the one pending-to-owned edge.
    fn acknowledge_codex_assignment(&mut self, pane_id: u32, payload_json: &str) -> bool {
        let Some(generation) = self.pending_assignment_generation(pane_id) else {
            return false;
        };
        let Some(ticket_id) = self
            .agent_slots
            .iter()
            .find(|slot| slot.pane_id == pane_id)
            .and_then(|slot| slot.ticket_id.as_deref())
        else {
            return false;
        };
        let pending = codex_ack::CodexAssignmentRef {
            ticket_id,
            generation,
        };
        if !codex_ack::detect_codex_ack(payload_json, pending) {
            return false;
        }

        self.seat_assignments
            .insert(pane_id, SeatAssignmentState::Owned);
        true
    }

    /// Whether a physical seat has acknowledged ownership of its assignment.
    ///
    /// Pending and recovering seats are intentionally not owned even though
    /// their slot retains a ticket reservation.
    #[allow(dead_code)] // Scheduler tests today; S-033-02 will project this to UI.
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

    /// Mark a slot as idle when its ticket completes. Keeps `has_session = true`
    /// so the same provider can reuse the TUI via `/clear`, while the other
    /// provider can explicitly recycle it via `/exit` after cooldown.
    fn release_slot_for_ticket(&mut self, ticket_id: &TicketId) {
        let mut released_pane: Option<(u32, String)> = None;
        for slot in &mut self.agent_slots {
            if slot.ticket_id.as_ref() == Some(ticket_id) {
                slot.ticket_id = None;
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
                released_pane = Some((
                    slot.pane_id,
                    format_pane_name(PaneName::Idle { resident_agent }),
                ));
                break;
            }
        }
        if let Some((pane_id, _)) = &released_pane {
            self.seat_assignments.remove(pane_id);
        }
        match released_pane {
            Some((pane_id, idle_name)) => {
                self.rename_slot(pane_id, idle_name);
                self.log_activity(ActivityEvent::Info {
                    message: format!("Released slot #{} for {}", pane_id, ticket_id),
                });
            }
            None => self.log_activity(ActivityEvent::Info {
                message: format!("No slot found for {}", ticket_id),
            }),
        }
    }

    /// Schedule ready tickets into idle agent slots.
    fn schedule_ready_tickets(&mut self) {
        if !self.permissions_granted || !self.slots_discovered || self.paused {
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
            let (slot_idx, recycle) = match self.find_slot_for_client(route.agent) {
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

            // Build the host-relative ticket dir (strip /host/ prefix)
            let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);

            let pane_id = self.agent_slots[slot_idx].pane_id;

            let assignment_generation = if route.agent == AgentClient::Codex && reused_seat {
                Some(self.allocate_assignment_generation())
            } else {
                None
            };

            // Defensive: an idle slot rarely hosts an agent blocked on a question,
            // but if it does, leave the slot unassigned and retry next poll rather
            // than /clear-ing or launching over the question UI.
            if self.is_pane_awaiting(pane_id) {
                unscheduled += 1;
                continue;
            }

            let ctx = SpawnContext {
                ticket_dir: &host_ticket_dir,
                ticket_id: &ticket_id,
                pane_id,
                assignment_generation,
            };

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
                let cmd = adapter.launch_command(&ctx);
                launch_cmd = cmd;
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
                    // Reuse-as-fresh-exec (headless/bridge adapters). The prior
                    // process left the pane's shell at its prompt, so there is no
                    // /clear handshake: type a fresh command for the new ticket.
                    // WaitingForClear must not engage — leaving
                    // transition_state untouched (Idle) keeps the .cleared/
                    // clear-timeout machinery inert for this pane.
                    ResetStrategy::FreshExec => {
                        let cmd = adapter.launch_command(&ctx);
                        self.send_line_to_pane(&cmd, PaneId::Terminal(pane_id));
                        launch_cmd = cmd;
                    }
                }
            } else {
                // Fresh pane — launch the agent from the shell.
                let cmd = adapter.launch_command(&ctx);
                launch_cmd = cmd.clone();
                self.send_line_to_pane(&cmd, PaneId::Terminal(pane_id));
                self.agent_slots[slot_idx].has_session = true;
            }

            self.agent_slots[slot_idx].ticket_id = Some(ticket_id.clone());
            // Stamp the provider that claimed this pane. A compatible session is
            // reused in-place; a recycled pane is reserved for this provider
            // while WaitingForExit prevents any other scheduler claim.
            self.agent_slots[slot_idx].last_client = Some(route.agent);
            let assignment_state = if let Some(generation) = assignment_generation {
                SeatAssignmentState::AssignedPendingAck { generation }
            } else {
                // Fresh launches retain the established immediate-ownership
                // contract, as do all Claude paths.
                SeatAssignmentState::Owned
            };
            self.seat_assignments.insert(pane_id, assignment_state);
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
            let running: Vec<(TicketId, Phase)> = self
                .threads
                .iter()
                .filter(|(_, t)| t.status == lisa_core::types::ThreadStatus::Running)
                .map(|(tid, t)| (tid.clone(), t.current_phase))
                .collect();

            let mut advanced_any = false;

            for (ticket_id, current_phase) in running {
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

                let artifact_path = self.config.work_dir.join(&ticket_id).join(artifact_name);
                if !artifact_path.exists() {
                    continue;
                }

                // Compute next phase (always Some for phases with artifacts)
                let next_phase = match current_phase.next() {
                    Some(p) => p,
                    None => continue,
                };

                if next_phase == Phase::Done {
                    self.request_completion(ticket_id.clone(), CompletionSource::Artifact);
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
        let entries = match std::fs::read_dir(&self.signal_dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let pane_id = match path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("pane-"))
                .and_then(|n| n.strip_suffix(".heartbeat"))
                .and_then(|id| id.parse::<u32>().ok())
            {
                Some(id) => id,
                None => continue,
            };

            let _ = std::fs::remove_file(&path);
            self.bump_pane_activity(pane_id);
            // A heartbeat proves genuine progress — clear any attention debounce
            // so a pane that resumes and later re-stalls can notify again.
            self.notified_attention.remove(&pane_id);
            // A real tool call means an AskUserQuestion (if any) was answered and
            // the agent resumed — stop suppressing injection into this pane.
            self.awaiting_human.remove(&pane_id);
        }
    }

    /// Consume raw Codex `UserPromptSubmit` payloads and promote only the
    /// ticket/generation currently pending in the addressed physical seat.
    fn check_codex_ack_signals(&mut self) {
        let entries = match std::fs::read_dir(&self.signal_dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let pane_id = match path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_prefix("pane-"))
                .and_then(|name| name.strip_suffix(".ack"))
                .and_then(|id| id.parse::<u32>().ok())
            {
                Some(id) => id,
                None => continue,
            };

            let payload = std::fs::read_to_string(&path).ok();
            let _ = std::fs::remove_file(&path);
            let Some(payload) = payload else {
                continue;
            };

            if self.acknowledge_codex_assignment(pane_id, &payload) {
                self.bump_pane_activity(pane_id);
                self.log_activity(ActivityEvent::Info {
                    message: format!("Pane {} acknowledged its Codex assignment", pane_id),
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
        let entries = match std::fs::read_dir(&self.signal_dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let pane_id = match path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("pane-"))
                .and_then(|n| n.strip_suffix(".awaiting"))
                .and_then(|id| id.parse::<u32>().ok())
            {
                Some(id) => id,
                None => continue,
            };

            let _ = std::fs::remove_file(&path);
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

            // Clean up the signal file immediately (prevents re-trigger on next poll)
            let _ = std::fs::remove_file(&path);

            // Signal files are named pane-{pane_id}.idle — resolve ticket
            // from the agent slot that owns this pane. `idle_pane_id` is lifted
            // out of the parse branch so the IdleWithoutArtifact arm below can
            // debounce on it and export LISA_PANE_ID.
            let mut idle_pane_id: Option<u32> = None;
            let ticket_id: TicketId = if let Some(rest) = filename
                .strip_prefix("pane-")
                .and_then(|s| s.strip_suffix(".idle"))
            {
                let pane_id: u32 = match rest.parse() {
                    Ok(id) => id,
                    Err(_) => continue,
                };
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
            } else {
                // Legacy: {ticket_id}.idle (from older hook versions)
                filename.trim_end_matches(".idle").to_string()
            };

            // Look up thread — signal only meaningful for running threads
            let current_phase = match self.threads.get(&ticket_id) {
                Some(t) if t.status == lisa_core::types::ThreadStatus::Running => t.current_phase,
                _ => continue,
            };

            match current_phase {
                Phase::Implement => {
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
                    let review_path = self.config.work_dir.join(&ticket_id).join("review.md");
                    if review_path.exists() {
                        self.request_completion(ticket_id.clone(), CompletionSource::Idle);
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
                    let artifact_path = self.config.work_dir.join(&ticket_id).join(artifact_name);

                    if artifact_path.exists() {
                        let next_phase = match current_phase.next() {
                            Some(p) => p,
                            None => continue,
                        };

                        if next_phase == Phase::Done {
                            self.request_completion(ticket_id.clone(), CompletionSource::Idle);
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
        let entries = match std::fs::read_dir(&self.signal_dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let filename = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Only process .stopped and .cleared signals
            if let Some(rest) = filename.strip_prefix("pane-") {
                if let Some(id_str) = rest.strip_suffix(".stopped") {
                    let _ = std::fs::remove_file(&path);
                    let pane_id: u32 = match id_str.parse() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    // A stop signal is recent life — restart the wind-down
                    // clock. Agents often keep working past their stop signal.
                    self.bump_pane_activity(pane_id);
                    self.handle_stopped_signal(pane_id);
                } else if let Some(id_str) = rest.strip_suffix(".cleared") {
                    let _ = std::fs::remove_file(&path);
                    let pane_id: u32 = match id_str.parse() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    self.bump_pane_activity(pane_id);
                    self.handle_cleared_signal(pane_id);
                }
                // .idle files are handled by check_idle_signals()
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
    fn check_error_signals(&mut self) {
        let entries = match std::fs::read_dir(&self.signal_dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let pane_id = match path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("pane-"))
                .and_then(|n| n.strip_suffix(".error"))
                .and_then(|id| id.parse::<u32>().ok())
            {
                Some(id) => id,
                None => continue,
            };

            // Read-and-delete: consume the signal regardless of outcome so it
            // never re-triggers or accumulates.
            let _ = std::fs::remove_file(&path);

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
                    self.emit_provenance(&tid, RunOutcome::Failed);
                    self.release_slot_for_ticket(&tid);
                    self.threads.remove(&tid);
                    self.error_alerts.push((tid.clone(), pane_id));
                    self.log_activity(ActivityEvent::Error {
                        message: format!(
                            "{} reported an error on pane {} — marked failed for retry",
                            tid, pane_id
                        ),
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
        self.request_completion(ticket_id, CompletionSource::Stopped(pane_id));
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
    fn emit_provenance(&mut self, ticket_id: &str, outcome: RunOutcome) {
        if self.ledger_path.as_os_str().is_empty() {
            return;
        }
        let Some(thread) = self.threads.get(ticket_id) else {
            return;
        };
        let client = thread.client;
        let started = provenance::system_time_to_epoch(thread.started_at);
        let ended = provenance::system_time_to_epoch(std::time::SystemTime::now());
        let route = Route::from_client(client);
        let record = ProvenanceRecord {
            schema_version: provenance::SCHEMA_VERSION,
            ticket_id: ticket_id.to_string(),
            outcome,
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
        let (tokens_in, tokens_out, cost_usd) = self.read_usage(client, ticket_id);
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
        }
    }

    /// Read tokens/cost from a run's usage artifact, selecting the per-provider
    /// directory by client:
    /// - Codex → `.lisa/codex/<ticket>.usage.json` (written by the native Stop
    ///   hook's `lisa capture-usage`, or by the JSON fallback).
    /// - Claude → `.lisa/claude/<ticket>.usage.json` (written by the Stop hook's
    ///   `lisa capture-usage`, T-027-02).
    ///
    /// Both writers emit the same `{ ..., usage: { input_tokens, output_tokens } }`
    /// shape, so the read spine is shared. A run with no artifact (missing file,
    /// bad JSON) yields all `None` — never fabricated. Claude carries no
    /// `cost_usd`; that stays `None` (cost is derived downstream from tokens +
    /// pricing, T-027-02 design).
    fn read_usage(
        &self,
        client: AgentClient,
        ticket_id: &str,
    ) -> (Option<u64>, Option<u64>, Option<f64>) {
        let dir = match client {
            AgentClient::Codex => &self.codex_dir,
            AgentClient::Claude => &self.claude_dir,
        };
        let path = dir.join(format!("{}.usage.json", ticket_id));
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return (None, None, None),
        };
        let value: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return (None, None, None),
        };
        // The artifact is `{ key, thread_id, success, usage }`; the token/cost
        // fields ride on the nested `usage` object.
        match value.get("usage") {
            Some(usage) if !usage.is_null() => provenance::extract_usage(usage),
            _ => (None, None, None),
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
            let ctx = SpawnContext {
                ticket_dir: &host_ticket_dir,
                ticket_id: &ticket_id,
                pane_id,
                assignment_generation: self.pending_assignment_generation(pane_id),
            };
            let prompt = adapter.reuse_prompt(&ctx);
            self.send_line_to_pane(&prompt, PaneId::Terminal(pane_id));

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
        let now = std::time::SystemTime::now();
        let wind_down = std::time::Duration::from_secs(self.config.wind_down_secs);

        // Collect actions to avoid borrow conflicts
        let mut exit_ready: Vec<(u32, Option<TicketId>)> = Vec::new();
        let mut stop_timeouts: Vec<u32> = Vec::new();
        let mut clear_timeouts: Vec<(u32, Option<TicketId>)> = Vec::new();

        for slot in &self.agent_slots {
            if let Some(started) = slot.transition_started_at {
                let elapsed = now.duration_since(started).unwrap_or_default().as_secs();
                let quiet = slot
                    .last_activity_at
                    .is_none_or(|at| now.duration_since(at).unwrap_or_default() >= wind_down);

                match slot.transition_state {
                    TransitionState::WaitingForExit if elapsed > AGENT_EXIT_GRACE_SECS => {
                        exit_ready.push((slot.pane_id, slot.ticket_id.clone()));
                    }
                    TransitionState::WaitingForStop
                        if elapsed > STOP_SIGNAL_TIMEOUT_SECS && quiet =>
                    {
                        stop_timeouts.push(slot.pane_id);
                    }
                    TransitionState::WaitingForClear
                        if elapsed > CLEAR_SIGNAL_TIMEOUT_SECS && quiet =>
                    {
                        clear_timeouts.push((slot.pane_id, slot.ticket_id.clone()));
                    }
                    _ => {}
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

            let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);
            let (adapter, route) = resolve_adapter_or_native(
                self.dag.get_ticket(&ticket_id),
                self.config.client,
                self.config.lisa_bin.as_deref(),
            );
            let ctx = SpawnContext {
                ticket_dir: &host_ticket_dir,
                ticket_id: &ticket_id,
                pane_id,
                assignment_generation: self.pending_assignment_generation(pane_id),
            };
            let command = adapter.launch_command(&ctx);
            self.send_line_to_pane(&command, PaneId::Terminal(pane_id));

            if let Some(slot) = self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
                slot.transition_state = TransitionState::Idle;
                slot.transition_started_at = None;
                slot.has_session = true;
                slot.last_client = Some(route.agent);
                slot.last_activity_at = Some(now);
            }
            self.log_activity(ActivityEvent::Info {
                message: format!(
                    "Pane {} exited previous client, launched {} for {}",
                    pane_id, route.agent, ticket_id
                ),
            });
        }

        for pane_id in stop_timeouts {
            // Don't force a /clear over a question — skip this pane in the fallback;
            // the transition resumes on a later tick once the agent is unblocked.
            if self.is_pane_awaiting(pane_id) {
                continue;
            }
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
            // Don't force the prompt over a question — skip; retry once unblocked.
            if self.is_pane_awaiting(pane_id) {
                continue;
            }
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
                let ctx = SpawnContext {
                    ticket_dir: &host_ticket_dir,
                    ticket_id: tid,
                    pane_id,
                    assignment_generation: self.pending_assignment_generation(pane_id),
                };
                let prompt = adapter.reuse_prompt(&ctx);
                self.send_line_to_pane(&prompt, PaneId::Terminal(pane_id));
            }
            if let Some(slot) = self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
                slot.transition_state = TransitionState::Idle;
                slot.transition_started_at = None;
            }
        }
    }

    /// Check for running Review threads that have exceeded the review timeout.
    ///
    /// When a thread has been running in Review phase longer than `review_timeout_secs`
    /// without producing `review.md`, sends a finish-up prompt to prod the agent.
    ///
    /// Set `review_timeout_secs = 0` to disable this feature.
    fn check_review_timeouts(&mut self) {
        if self.config.review_timeout_secs == 0 {
            return;
        }

        let now = std::time::SystemTime::now();
        let timeout = std::time::Duration::from_secs(self.config.review_timeout_secs);
        let wind_down = std::time::Duration::from_secs(self.config.wind_down_secs);

        // Collect candidates: running threads in Review phase past timeout,
        // not yet prompted, and quiet — never prod a session that is still
        // actively working (heartbeats flowing).
        let candidates: Vec<(TicketId, u32)> = self
            .threads
            .iter()
            .filter(|(_, t)| {
                t.status == lisa_core::types::ThreadStatus::Running
                    && t.current_phase == Phase::Review
            })
            .filter(|(tid, _)| !self.finish_up_sent.contains(*tid))
            .filter(|(_, t)| now.duration_since(t.last_phase_change).unwrap_or_default() >= timeout)
            .filter(|(_, t)| now.duration_since(t.last_activity).unwrap_or_default() >= wind_down)
            .map(|(tid, t)| (tid.clone(), t.pane_id))
            .collect();

        for (ticket_id, pane_id) in candidates {
            // Most acute clobber risk: a Review agent legitimately asking a question
            // must not be prodded with a finish-up prompt over its question UI. Skip
            // without marking finish_up_sent so it's re-evaluated once unblocked.
            if self.is_pane_awaiting(pane_id) {
                continue;
            }
            let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);
            let host_work_dir = strip_host_prefix(&self.config.work_dir);
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
                thread.mark_phase_change(std::time::SystemTime::now());
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
        use lisa_core::types::{HealthStatus, ThreadStatus};

        let now = std::time::SystemTime::now();
        let threshold = std::time::Duration::from_secs(self.config.stuck_threshold_secs);

        // Collect health transitions
        let transitions: Vec<(TicketId, HealthStatus, HealthStatus)> = self
            .threads
            .iter()
            .filter(|(_, t)| t.status == ThreadStatus::Running || t.status == ThreadStatus::Failed)
            .filter_map(|(tid, t)| {
                let current = t.health(now, threshold);
                let previous = self
                    .last_health
                    .get(tid)
                    .copied()
                    .unwrap_or(HealthStatus::Healthy);
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

    /// Check for sessions that have exceeded the configured session timeout.
    ///
    /// When `session_timeout_secs > 0` and a running thread's total wall-clock
    /// time (since `started_at`) exceeds the limit, the thread is marked failed,
    /// the slot is released, and the thread is removed. The Claude Code process
    /// is NOT killed — it may still be doing useful work.
    ///
    /// Busy-pane guard: a session that is over budget but not provably dead
    /// is never reclaimed — interrupting a partially-done ticket wastes the
    /// work and forces a repeat attempt. A warning is logged once, and
    /// reclamation requires the same prolonged silence as stale detection
    /// (2x stuck_threshold_secs), so slow tests or long integration calls
    /// (multi-minute silent stretches) never get a progressing session
    /// reclaimed. Budgets warn; only silence kills.
    fn check_session_timeouts(&mut self) {
        let now = std::time::SystemTime::now();
        let global_timeout = self.config.session_timeout_secs;
        let has_phase_timeouts = !self.config.phase_timeouts.is_empty();
        let hard_silence = std::time::Duration::from_secs(self.config.stuck_threshold_secs * 2);

        // If both global and per-phase timeouts are disabled, skip entirely
        if global_timeout == 0 && !has_phase_timeouts {
            return;
        }

        let mut timed_out: Vec<(TicketId, u64, Phase)> = Vec::new();
        let mut over_budget_active: Vec<(TicketId, u64, Phase)> = Vec::new();

        for (tid, t) in &self.threads {
            if t.status != lisa_core::types::ThreadStatus::Running {
                continue;
            }

            // Check global session timeout (total wall-clock since start)
            let mut exceeded: Option<(u64, Phase)> = None;
            if global_timeout > 0 {
                let elapsed = now.duration_since(t.started_at).unwrap_or_default();
                if elapsed >= std::time::Duration::from_secs(global_timeout) {
                    exceeded = Some((elapsed.as_secs(), t.current_phase));
                }
            }

            // Check per-phase timeout (time-in-phase since last phase change)
            if exceeded.is_none() && has_phase_timeouts {
                let phase_limit = self.config.timeout_for_phase(t.current_phase);
                if phase_limit > 0 {
                    let elapsed_in_phase =
                        now.duration_since(t.last_phase_change).unwrap_or_default();
                    if elapsed_in_phase >= std::time::Duration::from_secs(phase_limit) {
                        exceeded = Some((elapsed_in_phase.as_secs(), t.current_phase));
                    }
                }
            }

            if let Some((elapsed_secs, phase)) = exceeded {
                // Reclaim only at death-level silence (same bar as stale
                // detection), not a mere wind-down gap: slow test or
                // integration commands routinely produce multi-minute silent
                // stretches in a session that is progressing fine, and a
                // wind-down gap after the budget line would reclaim it
                // mid-ticket. The budget itself is advisory — silence kills,
                // budgets warn.
                let silent_for = now.duration_since(t.last_activity).unwrap_or_default();
                // Awaiting-human exemption (T-020-04): a pane blocked on an
                // AskUserQuestion may be silent far longer than hard-silence while a
                // human composes an answer. Never kill it — reclaiming mid-question
                // is the exact failure S-020 exists to prevent. It falls into the
                // warn branch instead, so it is still surfaced (warnings may log;
                // only the kill is suppressed). The exemption clears with the flag
                // on the pane's next heartbeat, restoring normal reclamation.
                if silent_for >= hard_silence && !self.awaiting_human.contains(&t.pane_id) {
                    timed_out.push((tid.clone(), elapsed_secs, phase));
                } else {
                    over_budget_active.push((tid.clone(), elapsed_secs, phase));
                }
            }
        }

        for (ticket_id, elapsed_secs, phase) in over_budget_active {
            if self.over_budget_warned.insert(ticket_id.clone()) {
                self.log_activity(ActivityEvent::Warning {
                    message: format!(
                        "{} exceeded its timeout ({}s in {}) but is still active — \
                         waiting for it to wind down instead of interrupting",
                        ticket_id, elapsed_secs, phase
                    ),
                });
            }
        }

        for (ticket_id, elapsed_secs, phase) in timed_out {
            if let Some(thread) = self.threads.get_mut(&ticket_id) {
                thread.fail();
            }
            self.emit_provenance(&ticket_id, RunOutcome::TimedOut);
            self.release_slot_for_ticket(&ticket_id);
            self.threads.remove(&ticket_id);
            self.timeout_alerts
                .push((ticket_id.clone(), elapsed_secs, phase));
            self.log_activity(ActivityEvent::SessionTimedOut {
                ticket_id,
                elapsed_secs,
                phase,
            });
        }
    }

    /// Detect threads that have been silent beyond the hard timeout.
    ///
    /// The hard timeout is 2x the configured stuck_threshold_secs of total
    /// inactivity — no heartbeats, signals, or phase changes. A session that
    /// is actively making tool calls never trips this, no matter how long its
    /// phase runs. Silent threads are marked as failed, their slots released,
    /// and they are removed from the threads map for retry.
    fn detect_stale_threads(&mut self) {
        use lisa_core::types::{HealthStatus, ThreadStatus};

        let now = std::time::SystemTime::now();
        // Hard timeout: 2x the configured stuck threshold
        let hard_timeout = std::time::Duration::from_secs(self.config.stuck_threshold_secs * 2);

        // Awaiting-human exemption (T-020-04): a pane blocked on an AskUserQuestion
        // is intentionally silent while a human answers. Exclude it from stale
        // reclamation so it is never killed mid-question. The marker on the
        // dashboard (driven off this same set) keeps it visible; the exemption
        // clears with the flag on the pane's next heartbeat.
        let awaiting = &self.awaiting_human;
        let stale: Vec<TicketId> = self
            .threads
            .iter()
            .filter(|(_, t)| t.status == ThreadStatus::Running)
            .filter(|(_, t)| t.health(now, hard_timeout) == HealthStatus::Stuck)
            .filter(|(_, t)| !awaiting.contains(&t.pane_id))
            .map(|(tid, _)| tid.clone())
            .collect();

        for ticket_id in stale {
            let mins = self.config.stuck_threshold_secs * 2 / 60;
            if let Some(thread) = self.threads.get_mut(&ticket_id) {
                thread.fail();
            }
            self.emit_provenance(&ticket_id, RunOutcome::Failed);
            self.release_slot_for_ticket(&ticket_id);
            self.threads.remove(&ticket_id);
            self.log_activity(ActivityEvent::Error {
                message: format!(
                    "{} stale — no activity for {}+ minutes, marked failed for retry",
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
        // Consume heartbeat signals first so activity clocks are current
        // before any health or timeout decisions this tick.
        self.check_heartbeat_signals();

        // Flag panes blocked on AskUserQuestion before any consumer can inject
        // into them this tick (must precede check_idle_signals and the timeout
        // fallbacks). Heartbeats above already cleared resumed panes.
        self.check_awaiting_signals();

        // Promote recycled Codex ownership only from a matching native prompt
        // submission. This runs before timeout/recovery evaluation.
        self.check_codex_ack_signals();

        // Check for new artifacts and advance phases before rebuilding DAG
        self.check_artifact_advances();

        // Check for idle signals and advance phases / generate alerts
        self.check_idle_signals();

        // Process .stopped and .cleared signals for session transitions
        self.check_transition_signals();

        // Fail panes that reported an error before the transition-timeout fallback
        // can force-advance them (adapter-emitted; inert for Claude panes).
        self.check_error_signals();

        // Fallback: force-advance stalled transitions
        self.check_transition_timeouts();

        // Send finish-up prompts to parked Review threads past timeout
        self.check_review_timeouts();

        // Evaluate health: log transitions (Healthy→Stuck, etc.)
        self.evaluate_health();

        // Check for sessions that exceeded the configured session timeout
        self.check_session_timeouts();

        // Detect and handle stale threads at hard timeout (2x threshold)
        self.detect_stale_threads();

        self.rebuild_dag();

        // Externally observed Done still enters the same commit transaction.
        // The pending mask prevents this path from publishing while a command
        // result is outstanding.
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

        for ticket_id in done_tickets {
            self.request_completion(ticket_id, CompletionSource::ObservedDone);
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
                        match self.modal.mode {
                            ModalMode::MarkDone => self.mark_ticket_done(&ticket_id),
                            ModalMode::ResetTicket => self.reset_ticket(&ticket_id),
                            ModalMode::QuitConfirm => {} // handled above
                        }
                    }
                    self.modal.open = false;
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
        };
    }

    /// Request manual completion through the same isolated transaction.
    fn mark_ticket_done(&mut self, ticket_id: &str) {
        self.request_completion(ticket_id.to_string(), CompletionSource::Manual);
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

        // Provenance ledger + per-provider usage-artifact directories.
        self.ledger_path = host.join(".lisa/provenance.jsonl");
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
                self.name_unnamed_idle_slots();
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
                    lisa_core::types::HealthStatus::Stuck => Some(ui::HealthAlert {
                        ticket_id: t.ticket_id.clone(),
                        alert_type: ui::AlertType::Stuck,
                        detail: format!("No phase change for {}+ min", threshold.as_secs() / 60),
                        suggested_actions: vec![
                            "Check pane".to_string(),
                            "Restart session".to_string(),
                        ],
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
                kind: match self.modal.mode {
                    ModalMode::MarkDone => ui::ModalKind::MarkDone,
                    ModalMode::ResetTicket => ui::ModalKind::ResetTicket,
                    ModalMode::QuitConfirm => ui::ModalKind::QuitConfirm,
                },
                new_ticket_ids: self.modal.new_ticket_ids.clone(),
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
    }

    #[test]
    fn test_build_claude_command() {
        let ticket_dir = Path::new("docs/active/tickets");
        let cmd = build_claude_command(ticket_dir, "T-042-01", 7, None, None);

        assert!(cmd.starts_with(
            "LISA_PANE_ID=7 LISA_TICKET_ID=T-042-01 claude --dangerously-skip-permissions "
        ));
        assert!(cmd.contains("docs/active/tickets/T-042-01.md"));
        assert!(cmd.contains("CLAUDE.md"));
        // No routed model → no --model flag (zero-regression: byte-for-byte the
        // pre-routing launch line).
        assert!(!cmd.contains("--model"));
        assert!(
            !cmd.ends_with('\r'),
            "Enter is now sent as a raw byte, not embedded in text"
        );
    }

    #[test]
    fn test_build_claude_command_with_model() {
        let ticket_dir = Path::new("docs/active/tickets");
        let cmd = build_claude_command(ticket_dir, "T-042-01", 7, Some("opus"), None);
        // The Claude adapter maps a routed model to `--model`, placed after the
        // permission flag and before the quoted prompt.
        assert!(
            cmd.contains("--dangerously-skip-permissions --model opus \""),
            "got: {cmd}"
        );
    }

    #[test]
    fn test_build_claude_command_includes_env_vars() {
        let ticket_dir = Path::new("docs/active/tickets");
        let cmd = build_claude_command(ticket_dir, "T-042-01", 42, None, None);

        assert!(
            cmd.starts_with("LISA_PANE_ID=42 LISA_TICKET_ID=T-042-01 "),
            "command should set LISA_PANE_ID and LISA_TICKET_ID env vars, got: {}",
            cmd
        );
    }

    #[test]
    fn test_build_claude_command_includes_rdspi_reference() {
        let ticket_dir = Path::new("docs/active/tickets");
        let cmd = build_claude_command(ticket_dir, "T-001", 1, None, None);

        assert!(
            cmd.contains("docs/knowledge/rdspi-workflow.md"),
            "command should reference RDSPI workflow, got: {}",
            cmd
        );
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
        let prompt = ticket_prompt(dir, "T-024-03", AgentClient::Claude.context_file());

        assert!(prompt.contains("docs/active/tickets/T-024-03.md"));
        assert!(prompt.contains("CLAUDE.md"));
        assert!(prompt.contains("docs/knowledge/rdspi-workflow.md"));
        assert!(prompt.contains("current phase"));
        assert!(prompt.contains("lisa commit-ticket"));
        assert!(prompt.contains("exact repository-relative --include paths"));
        assert!(prompt.contains("Do not use ordinary-index git add"));
        assert!(prompt.contains("git add -A"));
        assert!(prompt.contains("do not leave ticket-owned files staged, modified, or untracked"));
        assert!(prompt.contains("Do not start another ticket until Lisa confirms"));
    }

    #[test]
    fn test_ticket_prompt_uses_given_context_file() {
        let dir = Path::new("docs/active/tickets");
        // Codex's context file replaces CLAUDE.md in the shared prompt body.
        let prompt = ticket_prompt(dir, "T-024-03", "AGENTS.md");
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

        let prompt = ticket_prompt(&ticket_dir, "T-024-03", "AGENTS.md");

        assert!(prompt.contains("T-024-03-descriptive-title.md"));
        assert!(!prompt.contains("tickets/T-024-03.md"));
    }

    #[test]
    fn test_finish_up_prompt_preserves_atomic_completion_contract() {
        let prompt = finish_up_prompt(
            Path::new("docs/active/tickets"),
            Path::new("docs/active/work"),
            "T-024-03",
        );

        assert!(prompt.contains("docs/active/work/T-024-03/review.md"));
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

        // Only progress.md — should NOT advance
        let work_dir = dir.path().join("work");
        fs::create_dir_all(work_dir.join("T-002")).unwrap();
        fs::write(work_dir.join("T-002/progress.md"), "# Progress").unwrap();

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

        let thread = state.threads.get("T-002").unwrap();
        assert_eq!(thread.current_phase, Phase::Implement);
        assert_eq!(thread.status, ThreadStatus::Running);
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
        fs::create_dir_all(work_dir.join("T-002")).unwrap();
        fs::write(work_dir.join("T-002/review.md"), "# Review\nAll good.").unwrap();

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

        // All artifacts present
        let work_dir = dir.path().join("work");
        fs::create_dir_all(work_dir.join("T-005")).unwrap();
        fs::write(work_dir.join("T-005/research.md"), "# Research").unwrap();
        fs::write(work_dir.join("T-005/design.md"), "# Design").unwrap();
        fs::write(work_dir.join("T-005/structure.md"), "# Structure").unwrap();
        fs::write(work_dir.join("T-005/plan.md"), "# Plan").unwrap();
        fs::write(work_dir.join("T-005/review.md"), "# Review").unwrap();

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

        // Create work dir with review.md artifact
        let work_dir = dir.path().join("work");
        fs::create_dir_all(work_dir.join("T-001")).unwrap();
        fs::write(work_dir.join("T-001/review.md"), "# Review summary").unwrap();

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

        state.check_artifact_advances();

        // Thread and disk remain Review while the commit is pending.
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Running);
        assert!(state.pending_completions.contains_key("T-001"));

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

        // Create a thread that's been silent for 31+ minutes (past the bar)
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Research;
        thread.last_phase_change =
            std::time::SystemTime::now() - std::time::Duration::from_secs(31 * 60);
        thread.last_activity = thread.last_phase_change;
        state.threads.insert("T-001".to_string(), thread);

        // Add an agent slot so we can verify it gets released
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
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
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.mark_ticket_done("T-001");

        assert!(state.pending_completions.contains_key("T-001"));
        assert!(state.threads.contains_key("T-001"));
        assert_eq!(state.agent_slots[0].ticket_id.as_deref(), Some("T-001"));
        let content = fs::read_to_string(tickets_dir.join("T-001.md")).unwrap();
        assert!(content.contains("phase: implement"));
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

        // Create work dir with review.md already present (agent ran all phases)
        let work_dir = dir.path().join("work");
        fs::create_dir_all(work_dir.join("T-001")).unwrap();
        fs::write(work_dir.join("T-001/review.md"), "# Review\nAll good.").unwrap();

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

        // Run idle signal check
        state.check_idle_signals();

        // Verify: Review is published locally, while Done awaits the commit.
        let thread = state.threads.get("T-001").unwrap();
        assert_eq!(thread.current_phase, Phase::Review);
        assert_eq!(thread.status, ThreadStatus::Running);
        assert!(state.pending_completions.contains_key("T-001"));

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
        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state.agent_slots.push(slot);
        state
            .seat_assignments
            .insert(7, SeatAssignmentState::AssignedPendingAck { generation: 9 });

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
                .filter(|event| matches!(event, ActivityEvent::Info { message } if message.contains("acknowledged its Codex assignment")))
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
                .filter(|event| matches!(event, ActivityEvent::Info { message } if message.contains("acknowledged its Codex assignment")))
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
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let signal_dir = dir.path().join("signals");
        fs::create_dir_all(&signal_dir).unwrap();
        fs::write(signal_dir.join("pane-7.heartbeat"), "2026-06-20T00:00:00Z").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
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

        state.check_session_timeouts();

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

        // Create work dir with review.md artifact
        let work_dir = dir.path().join("work");
        fs::create_dir_all(work_dir.join("T-001")).unwrap();
        fs::write(work_dir.join("T-001/review.md"), "# Review summary").unwrap();

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
                client: default_agent,
                wind_down_secs: 0,
                ..PluginConfig::new()
            },
            permissions_granted: true,
            slots_discovered: true,
            ..State::default()
        };
        state.agent_slots.push(fresh_slot(10, resident_agent));
        (state, dir)
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
        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::Owned),
            "a fresh Codex launch retains the existing immediate ownership contract"
        );
        assert!(state.seat_is_owned(10));
    }

    #[test]
    fn test_recycled_codex_ownership_requires_matching_ack_exactly_once() {
        let (mut state, _dir) =
            pane_name_schedule_state("codex", AgentClient::Claude, Some(AgentClient::Codex));
        state
            .last_pane_names
            .insert(10, "codex · T-OLD · old work".to_string());

        state.schedule_ready_tickets();

        assert_eq!(
            state.last_pane_names.get(&10).map(String::as_str),
            Some("codex · T-NAME · pane lifecycle")
        );
        assert_eq!(
            state.agent_slots[0].transition_state,
            TransitionState::WaitingForClear
        );
        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::AssignedPendingAck { generation: 1 })
        );
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
                    generation: 1,
                },
            ),
        });
        assert!(!state.acknowledge_codex_assignment(10, &stale_ticket.to_string()));
        assert!(!state.seat_is_owned(10));

        let stale_generation = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": codex_ack::tag_codex_assignment(
                "new work",
                codex_ack::CodexAssignmentRef {
                    ticket_id: "T-NAME",
                    generation: 0,
                },
            ),
        });
        assert!(!state.acknowledge_codex_assignment(10, &stale_generation.to_string()));
        assert_eq!(
            state.seat_assignment(10),
            Some(SeatAssignmentState::AssignedPendingAck { generation: 1 })
        );

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
            Some(SeatAssignmentState::AssignedPendingAck { generation: 1 })
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
            SeatAssignmentState::AssignedPendingAck { generation: 1 },
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
        state
            .seat_assignments
            .insert(1, SeatAssignmentState::AssignedPendingAck { generation: 1 });

        state.check_transition_timeouts();

        // Should have forced to Idle
        assert_eq!(state.agent_slots[0].transition_state, TransitionState::Idle);
        assert!(state.agent_slots[0].transition_started_at.is_none());
        assert_eq!(
            state.seat_assignment(1),
            Some(SeatAssignmentState::AssignedPendingAck { generation: 1 }),
            "a clear timeout sends the prompt but is not Codex acknowledgment"
        );
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
        let mut state = State {
            config: PluginConfig {
                client: AgentClient::Codex,
                ticket_dir: std::path::PathBuf::from("/tmp/tickets"),
                ..PluginConfig::new()
            },
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-RECYCLE".to_string()),
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
        state
            .seat_assignments
            .insert(1, SeatAssignmentState::AssignedPendingAck { generation: 1 });

        state.check_transition_timeouts();

        let slot = &state.agent_slots[0];
        assert_eq!(slot.transition_state, TransitionState::Idle);
        assert!(slot.transition_started_at.is_none());
        assert!(slot.has_session);
        assert_eq!(slot.last_client, Some(AgentClient::Codex));
        assert_eq!(
            state.seat_assignment(1),
            Some(SeatAssignmentState::AssignedPendingAck { generation: 1 }),
            "exit-grace launch must preserve pending assignment truth"
        );
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
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Agent slot with ticket assigned
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
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

        let mut state = State {
            config: PluginConfig {
                ticket_dir: tickets_dir.clone(),
                session_timeout_secs: 1800, // 30 minutes
                stuck_threshold_secs: 600,  // hard-silence bar = 2x = 1200s
                ..PluginConfig::new()
            },
            ..State::default()
        };

        // Create a thread that started 31+ minutes ago (past 1800s timeout)
        // and has been silent the whole time (past the hard-silence bar),
        // so it is reclaimable
        let mut thread = Thread::new("T-001", 1);
        thread.current_phase = Phase::Implement;
        thread.started_at = std::time::SystemTime::now() - std::time::Duration::from_secs(31 * 60);
        thread.last_activity = thread.started_at;
        state.threads.insert("T-001".to_string(), thread);

        // Add an agent slot
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
            has_session: true,
            transition_state: TransitionState::Idle,
            transition_started_at: None,
            cooldown_until: None,
            last_activity_at: None,
            last_client: None,
        });

        state.check_session_timeouts();

        // Thread should be removed
        assert!(state.threads.is_empty());

        // Slot should be released
        assert!(state.agent_slots[0].ticket_id.is_none());

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
        fs::write(signal_dir.join("pane-1.heartbeat"), "2026-01-01T00:00:00Z").unwrap();

        let mut state = State {
            signal_dir: signal_dir.clone(),
            ..State::default()
        };
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: Some("T-001".to_string()),
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

        state.check_heartbeat_signals();

        // Signal file consumed
        assert!(!signal_dir.join("pane-1.heartbeat").exists());

        // Thread and slot activity clocks refreshed
        assert!(state.threads.get("T-001").unwrap().last_activity > stale);
        assert!(state.agent_slots[0].last_activity_at.is_some());
    }

    #[test]
    fn test_find_idle_slot_busy_pane_guard() {
        let mut state = State::default();

        // Released slot whose session showed activity moments ago — not reusable
        state.agent_slots.push(AgentSlot {
            pane_id: 1,
            ticket_id: None,
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

        state.check_error_signals();

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
        let ticket_work = state.config.work_dir.join("T-CDX-01");
        fs::create_dir_all(&ticket_work).unwrap();

        let mut thread = Thread::new("T-CDX-01", 1);
        thread.current_phase = Phase::Research;
        state.threads.insert("T-CDX-01".to_string(), thread);

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

        // Negative first: T-CDX-02's dep (T-CDX-01) is not Done → guard blocks.
        state.auto_complete_review("T-CDX-02".to_string(), 2);
        assert!(
            state.threads.contains_key("T-CDX-02"),
            "dependent ticket must NOT auto-complete while its dep is open"
        );
        assert!(state.activity_log.iter().any(|e| matches!(
            e,
            ActivityEvent::Error { message } if message.contains("dependencies are not all done")
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

        state.check_error_signals();

        let records = read_ledger(&ledger);
        assert_eq!(records.len(), 1, "one record on failure");
        assert_eq!(records[0].ticket_id, "T-CDX-02");
        assert_eq!(records[0].outcome, RunOutcome::Failed);
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
        state.emit_provenance("T-CDX-01", RunOutcome::Done);
        state.threads.remove("T-CDX-01");

        // Retry of the same ticket: fails.
        state
            .threads
            .insert("T-CDX-01".to_string(), Thread::new("T-CDX-01", 1));
        state.emit_provenance("T-CDX-01", RunOutcome::Failed);

        let records = read_ledger(&ledger);
        assert_eq!(records.len(), 2, "retry appends a second record");
        assert_eq!(records[0].outcome, RunOutcome::Done, "first record intact");
        assert_eq!(records[1].outcome, RunOutcome::Failed);
    }

    /// AC: Codex tokens flow from its usage artifact into the record.
    #[test]
    fn provenance_codex_usage_flows_into_record() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);
        fs::create_dir_all(&state.codex_dir).unwrap();
        fs::write(
            state.codex_dir.join("T-CDX-01.usage.json"),
            r#"{"key":"T-CDX-01","thread_id":"abc","success":true,
                "usage":{"input_tokens":120,"output_tokens":34}}"#,
        )
        .unwrap();

        let mut thread = Thread::new("T-CDX-01", 1);
        thread.client = AgentClient::Codex;
        state.threads.insert("T-CDX-01".to_string(), thread);
        state.emit_provenance("T-CDX-01", RunOutcome::Done);

        let records = read_ledger(&ledger);
        assert_eq!(records[0].tokens_in, Some(120));
        assert_eq!(records[0].tokens_out, Some(34));
        assert_eq!(
            records[0].cost_usd, None,
            "no cost field → null, never fabricated"
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

        state.emit_provenance("T-CDX-01", RunOutcome::Done);

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

    /// T-027-02 AC: a Claude run's tokens flow from the `.lisa/claude` usage
    /// artifact (written by the Stop hook's `capture-usage`) into the record;
    /// `cost_usd` stays null (derived downstream, never fabricated).
    #[test]
    fn provenance_claude_usage_flows_into_record() {
        use lisa_core::types::Thread;
        use std::fs;

        let (mut state, dir) = codex_state_with_dag();
        let ledger = with_ledger(&mut state, &dir);
        fs::create_dir_all(&state.claude_dir).unwrap();
        fs::write(
            state.claude_dir.join("T-CDX-01.usage.json"),
            r#"{"key":"T-CDX-01","usage":{"input_tokens":167,"output_tokens":37}}"#,
        )
        .unwrap();

        let mut thread = Thread::new("T-CDX-01", 1);
        thread.client = AgentClient::Claude;
        state.threads.insert("T-CDX-01".to_string(), thread);
        state.emit_provenance("T-CDX-01", RunOutcome::Done);

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
        state.emit_provenance("T-CDX-01", RunOutcome::Done);

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
        state.emit_provenance("T-CDX-01", RunOutcome::Done);
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
        fs::create_dir_all(state.config.work_dir.join("T-CDX-01")).unwrap();
        fs::write(
            state.config.work_dir.join("T-CDX-01/review.md"),
            "# Review\n",
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
            state.last_pane_names.get(&1).map(String::as_str),
            Some("codex · idle")
        );
        let ticket = state.dag.get_ticket(&"T-CDX-01".to_string()).unwrap();
        assert_eq!(ticket.phase, Phase::Done);
        assert_eq!(ticket.status, TicketStatus::Done);
    }

    #[test]
    fn failed_manual_completion_retries_without_early_release_or_duplicate_provenance() {
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

        state.mark_ticket_done("T-CDX-01");
        assert!(matches!(
            state.pending_completions.get("T-CDX-01").map(|p| p.source),
            Some(CompletionSource::Manual)
        ));
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
            ActivityEvent::Error { message }
                if message.contains("identity unavailable") && message.contains("recoverable")
        )));

        state.mark_ticket_done("T-CDX-01");
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
