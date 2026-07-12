//! Pluggable agent-client adapter interface.
//!
//! This module owns everything that differs *per integration method* when lisa
//! drives an agent session, so the scheduler in [`crate`] can stay
//! client-agnostic and consume only normalized signals. Today there is exactly
//! one implementation, [`ClaudeCodeAdapter`] (native Claude Code via hooks), and
//! [`resolve_adapter`] returns it unconditionally — with no opt-in, behaviour is
//! byte-for-byte identical to the pre-adapter scheduler.
//!
//! # Why a trait, resolved per ticket
//!
//! The selection seam is a *resolver* ([`resolve_adapter`]) taking the ticket and
//! returning an adapter — **not** a loop-wide constant — so that per-pane
//! `(provider, model)` routing (story S-026) is a later *addition*, not a
//! repaint. The MVP resolves every ticket to native Claude, but the shape is
//! per-pane-resolvable from day one.
//!
//! # WASM constraint
//!
//! Adapters run inside the WASM plugin, which cannot pipe to subprocesses. Every
//! trait method therefore returns a *description* — a command [`String`] or a
//! small action enum — that the scheduler injects into a pane (via
//! `send_line_to_pane`) or hands to Zellij `run_command`. Adapters never perform
//! host I/O themselves. This keeps the host boundary centralized and lets the
//! whole module be unit-tested without a Zellij host.
//!
//! # Accommodating future legs without redesign
//!
//! The trait is shaped so the two other planned integration methods drop in
//! without changing this interface (epic E-001, Decision 4; design thesis §5):
//!
//! - **Native Codex**: launches the official interactive TUI bare, reuses it
//!   through the same `/clear` handshake as Claude, and types
//!   follow-ups into the live composer. Codex lifecycle hooks emit the normalized
//!   `.heartbeat`/`.stopped`/`.cleared` files the scheduler already consumes.
//! - **ACP** (Agent Client Protocol, future): a host-side bridge process writes
//!   the same normalized signal files, so only `launch_command` (launch the
//!   bridge) and `signals` differ; the rest of the shape is unchanged.

use std::path::Path;

use lisa_core::client::AgentClient;
use lisa_core::route::{resolve_route, ResolvedRoute};
use lisa_core::types::Ticket;

use crate::codex_ack::{tag_codex_assignment, CodexAssignmentRef};
use crate::{build_claude_command, finish_up_prompt, shell_quote, ticket_prompt};

/// Inputs needed to construct a launch command or a reuse prompt for a ticket.
///
/// Paths are already host-relative (the caller applies `strip_host_prefix`);
/// adapters must not re-strip them.
pub(crate) struct SpawnContext<'a> {
    pub ticket_dir: &'a Path,
    pub ticket_id: &'a str,
    pub pane_id: u32,
    /// Immutable attempt identity inherited by native lifecycle hooks.
    pub attempt_id: u64,
    /// Private workflow artifact directory for this exact attempt lease.
    pub artifact_dir: &'a Path,
    /// Identity for a delivery awaiting provider acknowledgment.
    pub assignment_generation: Option<u64>,
}

/// Inputs needed to construct a follow-up nudge for a parked Review session.
pub(crate) struct FollowUpContext<'a> {
    pub ticket_dir: &'a Path,
    pub work_dir: &'a Path,
    pub ticket_id: &'a str,
    // Retained for non-interactive/future bridge adapters that spawn a follow-up
    // command rather than typing into a live TUI.
    #[allow(dead_code)]
    pub pane_id: u32,
}

/// How a slot that already has a live session is reset before new work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResetStrategy {
    /// Send `/clear` into the live TUI and wait for the `.cleared` signal before
    /// sending the next prompt (native Claude Code). This is the behaviour the
    /// scheduler's `TransitionState` handshake implements.
    ClearHandshake,
    /// Gracefully exit the resident interactive client, allow the bounded exit
    /// grace, then launch a fresh process for the next ticket. Codex
    /// uses this because its interactive `/clear` hook is not a reliable
    /// unattended delivery boundary.
    ExitThenFresh,
    /// Reuse is a fresh launch; there is no in-place reset handshake, so the
    /// `TransitionState` machine does not apply (headless/ACP bridges).
    #[allow(dead_code)]
    FreshExec,
}

/// A follow-up delivery mechanism for a parked Review session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FollowUp {
    /// Type the text into the live TUI, then submit (native Claude Code).
    TypeIntoPane(String),
    /// Spawn a host command for a headless or bridged follow-up.
    #[allow(dead_code)]
    SpawnCommand(String),
}

/// Which *optional* signals an adapter emits, beyond the normalized
/// `.heartbeat`/`.stopped`(/`.error`) core that every adapter must produce.
///
/// A `false` field tells the scheduler it must not wait on or expect that
/// signal from this adapter. Codex emits `.cleared` but has no equivalent of
/// Claude's `.idle`/`.awaiting` signals.
// Declared here (with the native adapter reporting its set) as the
// "expected-signal-set" seam; the behavioural consumer is the `.error` work in
// T-022-02 and the Codex adapter in T-023-02, which read it to treat the absence
// of Claude-only signals correctly.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SignalCapabilities {
    /// Emits `pane-<id>.idle` (idle-without-artifact detection).
    pub idle: bool,
    /// Emits `pane-<id>.awaiting` (blocked on an `AskUserQuestion`).
    pub awaiting: bool,
    /// Emits `pane-<id>.cleared` after a reset handshake.
    pub cleared: bool,
}

/// How a provider proves it is ready to receive its first prompt after a fresh
/// launch. The scheduler reads this at launch dispatch to pick the bootstrap-
/// readiness path (epic E-037). This is a classification only; the transition
/// that consumes the `Grace` arm lands in T-037-01-02.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadinessMode {
    /// A truthful pre-prompt process-start signal proves readiness (Claude
    /// `SessionStart`): the seat advances `Starting → ReadyForAssignment` on
    /// positive evidence before the first prompt is sent.
    SessionStart,
    /// No truthful pre-prompt readiness hook exists, so a bounded named startup
    /// grace paces the first prompt (Codex 0.144.1 emits `SessionStart` only
    /// *after* the first prompt). Elapsed time paces the send; it never proves
    /// readiness or ownership.
    Grace,
}

/// A pluggable agent client. Each integration *method* (native Claude Code,
/// native Codex TUI, future ACP bridge) implements this to supply the
/// behaviour that differs per method. See the [module docs](self) for how Codex
/// and ACP fit this shape without redesign, and for the WASM constraint that
/// makes every method return a description rather than perform I/O.
pub(crate) trait AgentAdapter {
    /// Bare command to launch a fresh session in an empty pane.
    fn launch_command(&self, ctx: &SpawnContext) -> String;

    /// Complete attempt-specific instructions, persisted before a fresh launch
    /// and typed directly only when reusing an established session.
    fn assignment_text(&self, ctx: &SpawnContext) -> String;

    /// Bounded chat message that refers a ready provider to its complete
    /// assignment file and carries the exact attempt marker acknowledged by
    /// `UserPromptSubmit`.
    fn assignment_reference(&self, ctx: &SpawnContext, assignment_path: &Path) -> String {
        let generation = ctx
            .assignment_generation
            .expect("assignment reference requires an acknowledgment generation");
        let message = format!(
            "Read and follow the complete assignment at {}.",
            assignment_path.display()
        );
        tag_codex_assignment(
            &message,
            CodexAssignmentRef {
                ticket_id: ctx.ticket_id,
                generation,
            },
        )
    }

    /// Slash command that gracefully exits the resident interactive client so
    /// the pane's shell can launch a different provider. Both native clients
    /// support `/exit`; keeping it on the adapter makes that contract explicit
    /// and leaves room for future integrations to override it.
    fn exit_command(&self) -> &'static str {
        "/exit"
    }

    /// How a slot that already has a session is reset before new work. Native
    /// interactive clients share the clear handshake; integrations with a
    /// different transport override this default.
    fn reset_strategy(&self) -> ResetStrategy {
        ResetStrategy::ClearHandshake
    }

    /// The bare next-prompt to inject into a *reused* session once it is ready
    /// (post-`.cleared` for Claude). Separate from [`Self::launch_command`]
    /// because reuse sends the prompt alone, not the wrapped launch invocation.
    fn reuse_prompt(&self, ctx: &SpawnContext) -> String;

    /// The follow-up nudge for a parked Review session. Native interactive
    /// clients type the shared prompt into their live TUI; integrations with a
    /// different delivery mechanism override this default.
    fn follow_up(&self, ctx: &FollowUpContext) -> FollowUp {
        FollowUp::TypeIntoPane(finish_up_prompt(
            ctx.ticket_dir,
            ctx.work_dir,
            ctx.ticket_id,
        ))
    }

    /// Which optional signals this adapter emits (see [`SignalCapabilities`]).
    // No live scheduler consumer in the MVP; wired by T-022-02 / T-023-02.
    #[allow(dead_code)]
    fn signals(&self) -> SignalCapabilities;

    /// Which bootstrap-readiness path this provider uses after a fresh launch
    /// (see [`ReadinessMode`]). The scheduler reads this at launch dispatch.
    fn readiness_mode(&self) -> ReadinessMode;
}

/// Native Claude Code adapter: the depth + reliability anchor leg. Delegates to
/// the existing free functions so its output is identical to the pre-adapter
/// scheduler (the no-op proof).
///
/// `model` is the routed model for this pane (T-026-01), mapped to Claude's
/// `--model` flag; `None` runs Claude's default model.
#[derive(Default)]
pub(crate) struct ClaudeCodeAdapter {
    model: Option<String>,
    /// Absolute `lisa` path (plugin config) exported as `LISA_BIN` on the launch
    /// line so the Stop hook's `capture-usage` is reachable (T-027-02). `None`
    /// omits the var (PATH fallback), keeping the pre-capture launch line.
    lisa_bin: Option<String>,
}

impl ClaudeCodeAdapter {
    /// Build a Claude adapter for the given routed model (`None` = provider
    /// default) and `lisa` binary path (`None` = PATH fallback). The default
    /// `ClaudeCodeAdapter` (via [`Default`]) carries neither, matching the
    /// pre-routing/pre-capture behaviour.
    pub(crate) fn new(model: Option<&str>, lisa_bin: Option<&str>) -> Self {
        Self {
            model: model.map(|s| s.to_string()),
            lisa_bin: lisa_bin.filter(|s| !s.is_empty()).map(|s| s.to_string()),
        }
    }
}

impl AgentAdapter for ClaudeCodeAdapter {
    fn launch_command(&self, ctx: &SpawnContext) -> String {
        build_claude_command(
            ctx.ticket_id,
            ctx.pane_id,
            ctx.attempt_id,
            self.model.as_deref(),
            self.lisa_bin.as_deref(),
        )
    }

    fn assignment_text(&self, ctx: &SpawnContext) -> String {
        ticket_prompt(
            ctx.ticket_dir,
            ctx.ticket_id,
            AgentClient::Claude.context_file(),
            ctx.artifact_dir,
        )
    }

    fn reuse_prompt(&self, ctx: &SpawnContext) -> String {
        self.assignment_text(ctx)
    }

    fn signals(&self) -> SignalCapabilities {
        // Native Claude Code emits the full Claude-only optional set.
        SignalCapabilities {
            idle: true,
            awaiting: true,
            cleared: true,
        }
    }

    fn readiness_mode(&self) -> ReadinessMode {
        // Native Claude Code emits SessionStart on process start: readiness is
        // proven by positive pre-prompt evidence.
        ReadinessMode::SessionStart
    }
}

/// Native Codex adapter: launches the official interactive CLI so Codex panes
/// have the same first-class chat experience as Claude panes.
///
/// Lisa-generated Codex hooks provide liveness, stop, startup, and exact prompt
/// acknowledgment signals. A completed Codex TUI is exited and freshly
/// launched for the next ticket: this avoids making unattended delivery depend
/// on the version-sensitive interactive `/clear` hook. The new process still
/// receives its assignment only after the provider-aware startup grace and
/// becomes owned only after the tagged `UserPromptSubmit` acknowledgment.
/// `--dangerously-bypass-hook-trust` is scoped to this invocation because lisa
/// generated the hook definitions. Agent command execution uses Codex's
/// full-access/no-approval mode, matching the existing Claude
/// `--dangerously-skip-permissions` contract and allowing the RDSPI workflow to
/// commit through `.git`.
pub(crate) struct CodexAdapter {
    lisa_bin: String,
    /// The routed model for this pane (T-026-01), mapped to Codex's top-level
    /// `--model <m>` option. `None` runs Codex's default model.
    model: Option<String>,
}

impl CodexAdapter {
    /// Build a Codex adapter. `lisa_bin` is the absolute `lisa` path from the
    /// plugin config; `None` (older layout / `current_exe()` failure) falls back
    /// to the bare `lisa`, resolved on the pane shell's PATH. `model` is the
    /// routed model (`None` = codex default).
    pub(crate) fn new(lisa_bin: Option<&str>, model: Option<&str>) -> Self {
        Self {
            lisa_bin: lisa_bin
                .filter(|s| !s.is_empty())
                .unwrap_or("lisa")
                .to_string(),
            model: model.map(|s| s.to_string()),
        }
    }

    /// The ` --model <m>` fragment for the interactive Codex line, or empty when
    /// no model is routed.
    fn model_flag(&self) -> String {
        match &self.model {
            Some(m) => format!(" --model {}", shell_quote(m)),
            None => String::new(),
        }
    }

    /// Attach scheduler identity only when this reused Codex delivery is
    /// waiting for positive acknowledgment.
    fn assignment_prompt(&self, ctx: &SpawnContext) -> String {
        let prompt = self.assignment_text(ctx);
        match ctx.assignment_generation {
            Some(generation) => tag_codex_assignment(
                &prompt,
                CodexAssignmentRef {
                    ticket_id: ctx.ticket_id,
                    generation,
                },
            ),
            None => prompt,
        }
    }

    /// The full interactive shell line for a fresh Codex TUI. The env prefix
    /// gives lifecycle hooks pane/ticket attribution and tells `capture-usage`
    /// to parse the Codex transcript into `.lisa/codex/`. Assignment text is
    /// deliberately absent and arrives through chat only after SessionStart.
    fn interactive_line(&self, ctx: &SpawnContext) -> String {
        format!(
            "LISA_BIN={bin} LISA_AGENT_CLIENT=codex LISA_PANE_ID={pane} LISA_TICKET_ID={ticket} LISA_ATTEMPT_ID={attempt} \
             codex --dangerously-bypass-approvals-and-sandbox \
             --dangerously-bypass-hook-trust{model} || \
             {{ mkdir -p .lisa/signals; date -u +%Y-%m-%dT%H:%M:%SZ > \
             .lisa/signals/pane-{pane}.error; }}",
            bin = shell_quote(&self.lisa_bin),
            pane = ctx.pane_id,
            ticket = shell_quote(ctx.ticket_id),
            attempt = ctx.attempt_id,
            model = self.model_flag(),
        )
    }
}

impl AgentAdapter for CodexAdapter {
    fn launch_command(&self, ctx: &SpawnContext) -> String {
        self.interactive_line(ctx)
    }

    fn assignment_text(&self, ctx: &SpawnContext) -> String {
        ticket_prompt(
            ctx.ticket_dir,
            ctx.ticket_id,
            AgentClient::Codex.context_file(),
            ctx.artifact_dir,
        )
    }

    fn reuse_prompt(&self, ctx: &SpawnContext) -> String {
        self.assignment_prompt(ctx)
    }

    fn reset_strategy(&self) -> ResetStrategy {
        ResetStrategy::ExitThenFresh
    }

    fn signals(&self) -> SignalCapabilities {
        SignalCapabilities {
            idle: false,
            awaiting: false,
            cleared: false,
        }
    }

    fn readiness_mode(&self) -> ReadinessMode {
        // Codex 0.144.1 emits SessionStart only after the first prompt creates a
        // thread, so no truthful pre-prompt readiness hook exists: a bounded
        // startup grace paces the first prompt (E-037).
        ReadinessMode::Grace
    }
}

/// Map a resolved route to its adapter.
///
/// `lisa_bin` is the absolute `lisa` path exported to lifecycle hooks so usage
/// capture does not depend on the pane shell's PATH. The
/// route's `model` (T-026-01) is threaded into the adapter so the launch line
/// carries the right model flag.
fn adapter_for_route(route: &ResolvedRoute, lisa_bin: Option<&str>) -> Box<dyn AgentAdapter> {
    let model = route.model.as_deref();
    match route.agent {
        AgentClient::Claude => Box::new(ClaudeCodeAdapter::new(model, lisa_bin)),
        AgentClient::Codex => Box::new(CodexAdapter::new(lisa_bin, model)),
    }
}

/// Resolve the adapter **and the route** for a ticket at spawn time, given the
/// loop-level default client and the absolute `lisa` binary path (Codex leg).
///
/// This is the per-pane routing seam (S-026 / T-026-01): the ticket's
/// `(agent, model)` frontmatter is resolved against the loop default via
/// [`resolve_route`], and the resulting [`ResolvedRoute`] is returned alongside
/// the adapter so the caller can surface a substituted fallback (log +
/// dashboard) and record requested-vs-actual in provenance. With no routing
/// hint and a Claude default this resolves byte-for-byte to the pre-routing
/// native path.
pub(crate) fn resolve_adapter(
    ticket: &Ticket,
    default_client: AgentClient,
    lisa_bin: Option<&str>,
) -> (Box<dyn AgentAdapter>, ResolvedRoute) {
    let route = resolve_route(ticket, default_client);
    let adapter = adapter_for_route(&route, lisa_bin);
    (adapter, route)
}

/// Resolve the adapter and route for an optional ticket, falling back to the
/// loop default when the ticket is momentarily absent from the DAG (e.g. a
/// mid-rebuild `.cleared`/timeout). An absent ticket has no routing hint, so the
/// route is the un-substituted loop default.
pub(crate) fn resolve_adapter_or_native(
    ticket: Option<&Ticket>,
    default_client: AgentClient,
    lisa_bin: Option<&str>,
) -> (Box<dyn AgentAdapter>, ResolvedRoute) {
    match ticket {
        Some(t) => resolve_adapter(t, default_client, lisa_bin),
        None => {
            let route = ResolvedRoute {
                agent: default_client,
                model: None,
                requested_agent: None,
                substituted: false,
                note: None,
            };
            let adapter = adapter_for_route(&route, lisa_bin);
            (adapter, route)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_ctx<'a>(dir: &'a Path, id: &'a str, pane: u32) -> SpawnContext<'a> {
        SpawnContext {
            ticket_dir: dir,
            ticket_id: id,
            pane_id: pane,
            attempt_id: 1,
            artifact_dir: Path::new(".lisa/attempts/T-042-01/1/work"),
            assignment_generation: None,
        }
    }

    #[test]
    fn native_launch_matches_free_fn() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 7);
        assert_eq!(
            ClaudeCodeAdapter::default().launch_command(&ctx),
            build_claude_command("T-042-01", 7, 1, None, None)
        );
    }

    #[test]
    fn native_launch_with_model_maps_flag() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 7);
        let cmd = ClaudeCodeAdapter::new(Some("opus"), None).launch_command(&ctx);
        assert!(cmd.contains("--model 'opus'"), "got: {cmd}");
        assert_eq!(
            cmd,
            build_claude_command("T-042-01", 7, 1, Some("opus"), None)
        );
    }

    #[test]
    fn native_launch_exports_lisa_bin_when_set() {
        // T-027-02: a threaded lisa path becomes LISA_BIN so the Stop hook's
        // capture-usage is reachable; without one the launch line is unchanged.
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 7);
        let with_bin = ClaudeCodeAdapter::new(None, Some("/abs/lisa")).launch_command(&ctx);
        assert!(
            with_bin.starts_with("LISA_BIN='/abs/lisa' LISA_PANE_ID=7"),
            "got: {with_bin}"
        );
        let without = ClaudeCodeAdapter::new(None, None).launch_command(&ctx);
        assert!(!without.contains("LISA_BIN="), "got: {without}");
        assert!(without.starts_with("LISA_PANE_ID=7"), "got: {without}");
    }

    #[test]
    fn native_reuse_prompt_matches_free_fn() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 7);
        assert_eq!(
            ClaudeCodeAdapter::default().reuse_prompt(&ctx),
            ticket_prompt(
                dir,
                "T-042-01",
                AgentClient::Claude.context_file(),
                ctx.artifact_dir,
            )
        );
    }

    #[test]
    fn native_follow_up_is_type_into_pane() {
        let ticket_dir = Path::new("docs/active/tickets");
        let work_dir = Path::new("docs/active/work");
        let ctx = FollowUpContext {
            ticket_dir,
            work_dir,
            ticket_id: "T-042-01",
            pane_id: 7,
        };
        assert_eq!(
            ClaudeCodeAdapter::default().follow_up(&ctx),
            FollowUp::TypeIntoPane(finish_up_prompt(ticket_dir, work_dir, "T-042-01"))
        );
    }

    #[test]
    fn native_reset_is_clear_handshake() {
        assert_eq!(
            ClaudeCodeAdapter::default().reset_strategy(),
            ResetStrategy::ClearHandshake
        );
    }

    #[test]
    fn native_clients_support_common_exit_command() {
        assert_eq!(ClaudeCodeAdapter::default().exit_command(), "/exit");
        assert_eq!(CodexAdapter::new(None, None).exit_command(), "/exit");
    }

    #[test]
    fn native_signals_all_true() {
        assert_eq!(
            ClaudeCodeAdapter::default().signals(),
            SignalCapabilities {
                idle: true,
                awaiting: true,
                cleared: true,
            }
        );
    }

    #[test]
    fn resolver_returns_claude_default_for_unrouted_ticket() {
        let ticket = Ticket::new("T-001", "example");
        let (adapter, route) = resolve_adapter(&ticket, AgentClient::Claude, None);
        assert_eq!(adapter.reset_strategy(), ResetStrategy::ClearHandshake);
        assert_eq!(route.agent, AgentClient::Claude);
        assert!(!route.substituted);
    }

    #[test]
    fn resolver_or_native_handles_missing_ticket() {
        let (adapter, route) = resolve_adapter_or_native(None, AgentClient::Claude, None);
        assert_eq!(adapter.reset_strategy(), ResetStrategy::ClearHandshake);
        assert_eq!(route.agent, AgentClient::Claude);
        assert!(!route.substituted);
    }

    #[test]
    fn resolver_codex_resolves_native_codex_adapter() {
        // Codex uses a fresh process boundary between tickets instead of the
        // version-sensitive interactive /clear hook.
        let ticket = Ticket::new("T-002", "codex-example");
        assert_eq!(
            resolve_adapter(&ticket, AgentClient::Codex, Some("/abs/lisa"))
                .0
                .reset_strategy(),
            ResetStrategy::ExitThenFresh
        );
        assert_eq!(
            resolve_adapter_or_native(None, AgentClient::Codex, Some("/abs/lisa"))
                .0
                .reset_strategy(),
            ResetStrategy::ExitThenFresh
        );
    }

    // --- Per-ticket routing (T-026-01) --------------------------------------

    #[test]
    fn ticket_agent_overrides_loop_default() {
        // A `agent: codex` ticket under a Claude loop default resolves to Codex.
        let mut ticket = Ticket::new("T-003", "routed-codex");
        ticket.agent = Some("codex".to_string());
        let (adapter, route) = resolve_adapter(&ticket, AgentClient::Claude, Some("/abs/lisa"));
        assert_eq!(adapter.reset_strategy(), ResetStrategy::ExitThenFresh);
        assert!(adapter
            .launch_command(&spawn_ctx(Path::new("docs/active/tickets"), "T-003", 1))
            .contains(" codex "));
        assert_eq!(route.agent, AgentClient::Codex);
        assert!(!route.substituted);
    }

    #[test]
    fn invalid_ticket_agent_falls_back_and_is_flagged() {
        // Invalid route → loop default (Codex here), substituted flag + note.
        let mut ticket = Ticket::new("T-004", "bad-route");
        ticket.agent = Some("gpt".to_string());
        let (adapter, route) = resolve_adapter(&ticket, AgentClient::Codex, Some("/abs/lisa"));
        assert_eq!(adapter.reset_strategy(), ResetStrategy::ExitThenFresh);
        assert_eq!(route.agent, AgentClient::Codex);
        assert!(route.substituted);
        assert!(route.note.unwrap().contains("gpt"));
    }

    #[test]
    fn mixed_route_resolves_heterogeneous_adapters_in_one_loop() {
        // The core S-026 promise: two tickets, same loop default, resolve to
        // different adapters concurrently by their own frontmatter.
        let mut a = Ticket::new("T-005", "on-codex");
        a.agent = Some("codex".to_string());
        a.model = Some("gpt-5".to_string());
        let b = Ticket::new("T-006", "on-default"); // no hint → default

        let (adapter_a, route_a) = resolve_adapter(&a, AgentClient::Claude, Some("/abs/lisa"));
        let (adapter_b, route_b) = resolve_adapter(&b, AgentClient::Claude, Some("/abs/lisa"));

        assert_eq!(adapter_a.reset_strategy(), ResetStrategy::ExitThenFresh);
        assert_eq!(adapter_b.reset_strategy(), ResetStrategy::ClearHandshake);
        assert_eq!(route_a.agent, AgentClient::Codex);
        assert_eq!(route_a.model.as_deref(), Some("gpt-5"));
        assert_eq!(route_b.agent, AgentClient::Claude);

        // The model flows into the launched command for the Codex pane.
        let dir = Path::new("docs/active/tickets");
        let cmd_a = adapter_a.launch_command(&spawn_ctx(dir, "T-005", 1));
        assert!(cmd_a.contains("--model 'gpt-5'"), "got: {cmd_a}");
    }

    // --- CodexAdapter (T-023-02) --------------------------------------------

    #[test]
    fn codex_launch_command_shape() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 7);
        let cmd = CodexAdapter::new(Some("/abs/lisa"), None).launch_command(&ctx);
        assert!(cmd.starts_with(
            "LISA_BIN='/abs/lisa' LISA_AGENT_CLIENT=codex LISA_PANE_ID=7 LISA_TICKET_ID='T-042-01' LISA_ATTEMPT_ID=1 codex "
        ));
        assert!(cmd.contains("--dangerously-bypass-approvals-and-sandbox"));
        assert!(cmd.contains("--dangerously-bypass-hook-trust"));
        assert!(cmd.contains(".lisa/signals/pane-7.error"));
        assert!(!cmd.contains("Read the ticket"));
        assert!(!cmd.contains("AGENTS.md"));
        assert!(!cmd.contains("LISA_ASSIGNMENT"));
    }

    #[test]
    fn provider_assignment_text_uses_its_context_file_while_launch_is_bare() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 1);

        let codex = CodexAdapter::new(Some("/abs/lisa"), None);
        assert!(codex.assignment_text(&ctx).contains("AGENTS.md"));
        assert!(!codex.assignment_text(&ctx).contains("CLAUDE.md"));
        assert!(!codex.launch_command(&ctx).contains("AGENTS.md"));

        let claude = ClaudeCodeAdapter::default();
        assert!(claude.assignment_text(&ctx).contains("CLAUDE.md"));
        assert!(!claude.assignment_text(&ctx).contains("AGENTS.md"));
        assert!(!claude.launch_command(&ctx).contains("CLAUDE.md"));
    }

    #[test]
    fn codex_launch_with_model_emits_flag() {
        // A routed model (T-026-01) is forwarded to the interactive CLI.
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 7);
        let cmd = CodexAdapter::new(Some("/abs/lisa"), Some("gpt-5")).launch_command(&ctx);
        assert!(cmd.contains("--model 'gpt-5' ||"), "got: {cmd}");
        // No model → no flag (pre-routing wrapper line).
        let bare = CodexAdapter::new(Some("/abs/lisa"), None).launch_command(&ctx);
        assert!(!bare.contains("--model"), "got: {bare}");
    }

    #[test]
    fn codex_reuse_is_bare_prompt_for_live_tui() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 3);
        let a = CodexAdapter::new(Some("/abs/lisa"), None);
        assert_eq!(
            a.reuse_prompt(&ctx),
            ticket_prompt(
                dir,
                "T-042-01",
                AgentClient::Codex.context_file(),
                ctx.artifact_dir,
            )
        );
        assert!(!a.reuse_prompt(&ctx).contains(" codex "));
    }

    #[test]
    fn pending_delivery_tags_reuse_and_bounded_reference_but_not_launch() {
        let dir = Path::new("docs/active/tickets");
        let mut ctx = spawn_ctx(dir, "T-042-01", 3);
        ctx.assignment_generation = Some(17);
        let adapter = CodexAdapter::new(Some("/abs/lisa"), None);

        let reuse = adapter.reuse_prompt(&ctx);
        assert!(reuse.contains(r#"LISA_ASSIGNMENT {"ticket_id":"T-042-01","generation":17}"#));

        let launch = adapter.launch_command(&ctx);
        assert!(!launch.contains("LISA_ASSIGNMENT"));
        let reference = adapter.assignment_reference(
            &ctx,
            Path::new(".lisa/attempts/T-042-01/1/work/assignment.md"),
        );
        assert!(reference.contains("assignment.md"));
        assert!(reference.contains(r#"LISA_ASSIGNMENT {"ticket_id":"T-042-01","generation":17}"#));
    }

    #[test]
    fn codex_reset_exits_before_fresh_launch() {
        assert_eq!(
            CodexAdapter::new(Some("/abs/lisa"), None).reset_strategy(),
            ResetStrategy::ExitThenFresh
        );
    }

    #[test]
    fn codex_follow_up_is_typed_into_live_tui() {
        let ticket_dir = Path::new("docs/active/tickets");
        let work_dir = Path::new("docs/active/work");
        let ctx = FollowUpContext {
            ticket_dir,
            work_dir,
            ticket_id: "T-042-01",
            pane_id: 9,
        };
        assert_eq!(
            CodexAdapter::new(Some("/abs/lisa"), None).follow_up(&ctx),
            FollowUp::TypeIntoPane(finish_up_prompt(ticket_dir, work_dir, "T-042-01"))
        );
    }

    #[test]
    fn codex_signals_do_not_require_clear_handshake() {
        assert_eq!(
            CodexAdapter::new(Some("/abs/lisa"), None).signals(),
            SignalCapabilities {
                idle: false,
                awaiting: false,
                cleared: false,
            }
        );
    }

    #[test]
    fn claude_reports_session_start_readiness() {
        // Claude proves readiness via a pre-prompt SessionStart signal.
        assert_eq!(
            ClaudeCodeAdapter::default().readiness_mode(),
            ReadinessMode::SessionStart
        );
    }

    #[test]
    fn codex_reports_grace_readiness() {
        // Codex has no truthful pre-prompt readiness hook, so it paces the first
        // prompt behind a bounded startup grace.
        assert_eq!(
            CodexAdapter::new(Some("/abs/lisa"), None).readiness_mode(),
            ReadinessMode::Grace
        );
    }

    #[test]
    fn codex_new_falls_back_to_bare_lisa() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-001", 1);
        // None and empty both degrade to a PATH-resolved `lisa` for hooks.
        for bin in [None, Some("")] {
            let cmd = CodexAdapter::new(bin, None).launch_command(&ctx);
            assert!(cmd.starts_with("LISA_BIN='lisa' "), "cmd: {cmd}");
        }
    }
}
