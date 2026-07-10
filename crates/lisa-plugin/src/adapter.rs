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
//! - **Native Codex** (`codex exec --json` wrapper, ticket T-023-02):
//!   [`AgentAdapter::launch_command`] builds the `codex exec` invocation;
//!   [`AgentAdapter::reset_strategy`] returns [`ResetStrategy::FreshExec`] because
//!   Codex reuse is a fresh exec, not a `/clear` handshake;
//!   [`AgentAdapter::follow_up`] returns [`FollowUp::SpawnCommand`] wrapping
//!   `codex exec resume` (there is no live TUI to type into); and
//!   [`AgentAdapter::signals`] reports that `.idle`/`.awaiting`/`.cleared` are not
//!   emitted (they are Claude-only), so the scheduler treats their absence
//!   correctly.
//! - **ACP** (Agent Client Protocol, future): a host-side bridge process writes
//!   the same normalized signal files, so only `launch_command` (launch the
//!   bridge) and `signals` differ; the rest of the shape is unchanged.

use std::path::Path;

use lisa_core::client::AgentClient;
use lisa_core::route::{resolve_route, ResolvedRoute};
use lisa_core::types::Ticket;

use crate::{build_claude_command, finish_up_prompt, ticket_prompt};

/// Inputs needed to construct a launch command or a reuse prompt for a ticket.
///
/// Paths are already host-relative (the caller applies `strip_host_prefix`);
/// adapters must not re-strip them.
pub(crate) struct SpawnContext<'a> {
    pub ticket_dir: &'a Path,
    pub ticket_id: &'a str,
    pub pane_id: u32,
}

/// Inputs needed to construct a follow-up nudge for a parked Review session.
pub(crate) struct FollowUpContext<'a> {
    pub ticket_dir: &'a Path,
    pub work_dir: &'a Path,
    pub ticket_id: &'a str,
    // Read by a future Codex SpawnCommand follow-up (T-023-02) to attribute the
    // resumed exec to a pane; the native TUI follow-up does not need it.
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
    /// Reuse is a fresh launch; there is no in-place reset handshake, so the
    /// `TransitionState` machine does not apply (native Codex, ACP).
    // Consumed by T-023-02 (Codex); no adapter returns it in the MVP.
    #[allow(dead_code)]
    FreshExec,
}

/// A follow-up delivery mechanism for a parked Review session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FollowUp {
    /// Type the text into the live TUI, then submit (native Claude Code).
    TypeIntoPane(String),
    /// Spawn a host command, e.g. `codex exec resume …` (native Codex, ACP).
    // Consumed by T-023-02 (Codex); no adapter returns it in the MVP.
    #[allow(dead_code)]
    SpawnCommand(String),
}

/// Which *optional* signals an adapter emits, beyond the normalized
/// `.heartbeat`/`.stopped`(/`.error`) core that every adapter must produce.
///
/// A `false` field tells the scheduler it must not wait on or expect that
/// signal from this adapter. `.idle`/`.awaiting`/`.cleared` are Claude-only; a
/// Codex/ACP adapter reports `false` for the ones it does not emit so the
/// scheduler treats their absence correctly (the `.error` consumer itself is
/// ticket T-022-02).
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

/// A pluggable agent client. Each integration *method* (native Claude Code,
/// native Codex `exec` wrapper, future ACP bridge) implements this to supply the
/// behaviour that differs per method. See the [module docs](self) for how Codex
/// and ACP fit this shape without redesign, and for the WASM constraint that
/// makes every method return a description rather than perform I/O.
pub(crate) trait AgentAdapter {
    /// Command to launch a fresh session in an empty pane.
    fn launch_command(&self, ctx: &SpawnContext) -> String;

    /// How a slot that already has a session is reset before new work.
    fn reset_strategy(&self) -> ResetStrategy;

    /// The bare next-prompt to inject into a *reused* session once it is ready
    /// (post-`.cleared` for Claude). Separate from [`Self::launch_command`]
    /// because reuse sends the prompt alone, not the wrapped launch invocation.
    fn reuse_prompt(&self, ctx: &SpawnContext) -> String;

    /// The follow-up nudge for a parked Review session.
    fn follow_up(&self, ctx: &FollowUpContext) -> FollowUp;

    /// Which optional signals this adapter emits (see [`SignalCapabilities`]).
    // No live scheduler consumer in the MVP; wired by T-022-02 / T-023-02.
    #[allow(dead_code)]
    fn signals(&self) -> SignalCapabilities;
}

/// Native Claude Code adapter: the depth + reliability anchor leg. Delegates to
/// the existing free functions so its output is identical to the pre-adapter
/// scheduler (the no-op proof).
///
/// `model` is the routed model for this pane (T-026-01), mapped to Claude's
/// `--model` flag; `None` runs Claude's default model — the pre-routing launch
/// line, byte-for-byte.
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

impl Default for ClaudeCodeAdapter {
    fn default() -> Self {
        Self {
            model: None,
            lisa_bin: None,
        }
    }
}

impl AgentAdapter for ClaudeCodeAdapter {
    fn launch_command(&self, ctx: &SpawnContext) -> String {
        build_claude_command(
            ctx.ticket_dir,
            ctx.ticket_id,
            ctx.pane_id,
            self.model.as_deref(),
            self.lisa_bin.as_deref(),
        )
    }

    fn reset_strategy(&self) -> ResetStrategy {
        ResetStrategy::ClearHandshake
    }

    fn reuse_prompt(&self, ctx: &SpawnContext) -> String {
        // Native Claude reads CLAUDE.md.
        ticket_prompt(
            ctx.ticket_dir,
            ctx.ticket_id,
            AgentClient::Claude.context_file(),
        )
    }

    fn follow_up(&self, ctx: &FollowUpContext) -> FollowUp {
        FollowUp::TypeIntoPane(finish_up_prompt(
            ctx.ticket_dir,
            ctx.work_dir,
            ctx.ticket_id,
        ))
    }

    fn signals(&self) -> SignalCapabilities {
        // Native Claude Code emits the full Claude-only optional set.
        SignalCapabilities {
            idle: true,
            awaiting: true,
            cleared: true,
        }
    }
}

/// Native Codex adapter: drives the `lisa agent-exec` wrapper (T-023-01), which
/// runs `codex exec --json` and writes lisa's signal files. Unlike the Claude
/// leg there is no live TUI and no `/clear` handshake — every session is a fresh
/// `agent-exec` invocation typed into the pane's shell, so:
///
/// - [`Self::launch_command`] and [`Self::reuse_prompt`] both return the *same*
///   full wrapper line (there is no "bare prompt" mode — a reused pane is at its
///   shell prompt, not inside a running agent).
/// - [`Self::reset_strategy`] is [`ResetStrategy::FreshExec`], so the scheduler's
///   `WaitingForClear` machinery never engages for a Codex pane.
/// - [`Self::follow_up`] is [`FollowUp::SpawnCommand`] wrapping
///   `agent-exec --resume` (re-enters the persisted thread with the finish-up
///   prompt) rather than typing into a live TUI.
/// - [`Self::signals`] reports the Claude-only optional signals as absent.
///
/// `lisa_bin` is the absolute path to the `lisa` binary, captured at `lisa loop`
/// time via `current_exe()` and threaded through the plugin config — the pane
/// shell may not have `lisa` on its PATH. It falls back to the bare `lisa` when
/// the layout omits it (see [`CodexAdapter::new`]).
pub(crate) struct CodexAdapter {
    lisa_bin: String,
    /// The routed model for this pane (T-026-01), forwarded to the wrapper as
    /// `--model <m>` (which the wrapper passes to `codex exec`). `None` runs
    /// codex's default model.
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

    /// The ` --model <m>` fragment for the `agent-exec` line, or empty when no
    /// model is routed (the pre-routing wrapper line).
    fn model_flag(&self) -> String {
        match &self.model {
            Some(m) => format!(" --model {}", m),
            None => String::new(),
        }
    }

    /// The full wrapper shell line for a fresh (or reused) run of `ticket_id` on
    /// `pane_id`. The env prefix mirrors `build_claude_command` so the wrapper
    /// inherits `LISA_PANE_ID`/`LISA_TICKET_ID` for signal attribution and the
    /// resume key. The prompt is wrapped in plain double quotes exactly as the
    /// Claude line is — the RDSPI prompts contain no shell metacharacters.
    fn agent_exec_line(&self, ctx: &SpawnContext) -> String {
        format!(
            "LISA_PANE_ID={pane} LISA_TICKET_ID={ticket} {bin} agent-exec{model} \"{prompt}\"",
            pane = ctx.pane_id,
            ticket = ctx.ticket_id,
            bin = self.lisa_bin,
            model = self.model_flag(),
            // Codex auto-loads AGENTS.md, not CLAUDE.md.
            prompt = ticket_prompt(
                ctx.ticket_dir,
                ctx.ticket_id,
                AgentClient::Codex.context_file()
            ),
        )
    }
}

impl AgentAdapter for CodexAdapter {
    fn launch_command(&self, ctx: &SpawnContext) -> String {
        self.agent_exec_line(ctx)
    }

    fn reset_strategy(&self) -> ResetStrategy {
        // Reuse is a fresh exec — the finished codex process leaves the pane's
        // shell at its prompt, so there is nothing to `/clear`.
        ResetStrategy::FreshExec
    }

    fn reuse_prompt(&self, ctx: &SpawnContext) -> String {
        // Identical to launch: reuse types a fresh wrapper command for the new
        // ticket, not a bare prompt into a live TUI.
        self.agent_exec_line(ctx)
    }

    fn follow_up(&self, ctx: &FollowUpContext) -> FollowUp {
        // Re-enter the persisted thread (`--resume`) with the finish-up prompt.
        // The pane shell re-launches codex; there is no live TUI to type into.
        let cmd = format!(
            "LISA_PANE_ID={pane} LISA_TICKET_ID={ticket} {bin} agent-exec --resume{model} \"{prompt}\"",
            pane = ctx.pane_id,
            ticket = ctx.ticket_id,
            bin = self.lisa_bin,
            model = self.model_flag(),
            prompt = finish_up_prompt(ctx.ticket_dir, ctx.work_dir, ctx.ticket_id),
        );
        FollowUp::SpawnCommand(cmd)
    }

    fn signals(&self) -> SignalCapabilities {
        // The Codex `exec -a never` path emits only the core
        // `.heartbeat`/`.stopped`/`.error` set — none of the Claude-only signals.
        SignalCapabilities {
            idle: false,
            awaiting: false,
            cleared: false,
        }
    }
}

/// Map a resolved route to its adapter.
///
/// `lisa_bin` is the absolute `lisa` path (plugin config) the Codex adapter needs
/// to build `<lisa> agent-exec …` lines; it is unused by the Claude arm. The
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
        }
    }

    #[test]
    fn native_launch_matches_free_fn() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 7);
        assert_eq!(
            ClaudeCodeAdapter::default().launch_command(&ctx),
            build_claude_command(dir, "T-042-01", 7, None, None)
        );
    }

    #[test]
    fn native_launch_with_model_maps_flag() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 7);
        let cmd = ClaudeCodeAdapter::new(Some("opus"), None).launch_command(&ctx);
        assert!(cmd.contains("--model opus"), "got: {cmd}");
        assert_eq!(
            cmd,
            build_claude_command(dir, "T-042-01", 7, Some("opus"), None)
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
            with_bin.starts_with("LISA_BIN=/abs/lisa LISA_PANE_ID=7"),
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
            ticket_prompt(dir, "T-042-01", AgentClient::Claude.context_file())
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
        // T-023-02: a Codex selection now resolves to the real CodexAdapter,
        // identified by its FreshExec reset strategy (no /clear handshake).
        let ticket = Ticket::new("T-002", "codex-example");
        assert_eq!(
            resolve_adapter(&ticket, AgentClient::Codex, Some("/abs/lisa"))
                .0
                .reset_strategy(),
            ResetStrategy::FreshExec
        );
        assert_eq!(
            resolve_adapter_or_native(None, AgentClient::Codex, Some("/abs/lisa"))
                .0
                .reset_strategy(),
            ResetStrategy::FreshExec
        );
    }

    // --- Per-ticket routing (T-026-01) --------------------------------------

    #[test]
    fn ticket_agent_overrides_loop_default() {
        // A `agent: codex` ticket under a Claude loop default resolves to Codex.
        let mut ticket = Ticket::new("T-003", "routed-codex");
        ticket.agent = Some("codex".to_string());
        let (adapter, route) = resolve_adapter(&ticket, AgentClient::Claude, Some("/abs/lisa"));
        assert_eq!(adapter.reset_strategy(), ResetStrategy::FreshExec);
        assert_eq!(route.agent, AgentClient::Codex);
        assert!(!route.substituted);
    }

    #[test]
    fn invalid_ticket_agent_falls_back_and_is_flagged() {
        // Invalid route → loop default (Codex here), substituted flag + note.
        let mut ticket = Ticket::new("T-004", "bad-route");
        ticket.agent = Some("gpt".to_string());
        let (adapter, route) = resolve_adapter(&ticket, AgentClient::Codex, Some("/abs/lisa"));
        assert_eq!(adapter.reset_strategy(), ResetStrategy::FreshExec);
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

        assert_eq!(adapter_a.reset_strategy(), ResetStrategy::FreshExec);
        assert_eq!(adapter_b.reset_strategy(), ResetStrategy::ClearHandshake);
        assert_eq!(route_a.agent, AgentClient::Codex);
        assert_eq!(route_a.model.as_deref(), Some("gpt-5"));
        assert_eq!(route_b.agent, AgentClient::Claude);

        // The model flows into the launched command for the Codex pane.
        let dir = Path::new("docs/active/tickets");
        let cmd_a = adapter_a.launch_command(&spawn_ctx(dir, "T-005", 1));
        assert!(cmd_a.contains("--model gpt-5"), "got: {cmd_a}");
    }

    // --- CodexAdapter (T-023-02) --------------------------------------------

    #[test]
    fn codex_launch_command_shape() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 7);
        let cmd = CodexAdapter::new(Some("/abs/lisa"), None).launch_command(&ctx);
        assert!(cmd.starts_with("LISA_PANE_ID=7 LISA_TICKET_ID=T-042-01 /abs/lisa agent-exec "));
        assert!(cmd.contains("agent-exec \""));
        // The wrapped prompt is the shared RDSPI ticket prompt, verbatim — with
        // Codex's context file (AGENTS.md).
        assert!(cmd.contains(&ticket_prompt(
            dir,
            "T-042-01",
            AgentClient::Codex.context_file()
        )));
        assert!(cmd.contains("AGENTS.md"));
    }

    #[test]
    fn codex_prompt_references_agents_not_claude() {
        // Parity: the Codex launch line points the agent at AGENTS.md, the Claude
        // launch line at CLAUDE.md — never crossed.
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 1);

        let codex_cmd = CodexAdapter::new(Some("/abs/lisa"), None).launch_command(&ctx);
        assert!(codex_cmd.contains("AGENTS.md"));
        assert!(!codex_cmd.contains("CLAUDE.md"));

        let claude_cmd = ClaudeCodeAdapter::default().launch_command(&ctx);
        assert!(claude_cmd.contains("CLAUDE.md"));
        assert!(!claude_cmd.contains("AGENTS.md"));
    }

    #[test]
    fn codex_launch_with_model_emits_flag() {
        // A routed model (T-026-01) is forwarded to the wrapper as `--model`,
        // placed between `agent-exec` and the quoted prompt.
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 7);
        let cmd = CodexAdapter::new(Some("/abs/lisa"), Some("gpt-5")).launch_command(&ctx);
        assert!(cmd.contains("agent-exec --model gpt-5 \""), "got: {cmd}");
        // No model → no flag (pre-routing wrapper line).
        let bare = CodexAdapter::new(Some("/abs/lisa"), None).launch_command(&ctx);
        assert!(!bare.contains("--model"), "got: {bare}");
    }

    #[test]
    fn codex_reuse_equals_launch_no_handshake() {
        // Reuse-without-handshake: a reused Codex pane types the same fresh
        // wrapper line as a launch (not a bare prompt into a live TUI).
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-042-01", 3);
        let a = CodexAdapter::new(Some("/abs/lisa"), None);
        assert_eq!(a.reuse_prompt(&ctx), a.launch_command(&ctx));
    }

    #[test]
    fn codex_reset_is_fresh_exec() {
        assert_eq!(
            CodexAdapter::new(Some("/abs/lisa"), None).reset_strategy(),
            ResetStrategy::FreshExec
        );
    }

    #[test]
    fn codex_follow_up_is_spawn_resume() {
        let ticket_dir = Path::new("docs/active/tickets");
        let work_dir = Path::new("docs/active/work");
        let ctx = FollowUpContext {
            ticket_dir,
            work_dir,
            ticket_id: "T-042-01",
            pane_id: 9,
        };
        match CodexAdapter::new(Some("/abs/lisa"), None).follow_up(&ctx) {
            FollowUp::SpawnCommand(cmd) => {
                assert!(cmd.contains("LISA_PANE_ID=9 LISA_TICKET_ID=T-042-01 /abs/lisa"));
                assert!(cmd.contains("agent-exec --resume \""));
                assert!(cmd.contains(&finish_up_prompt(ticket_dir, work_dir, "T-042-01")));
            }
            other => panic!("expected SpawnCommand, got {:?}", other),
        }
    }

    #[test]
    fn codex_signals_all_false() {
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
    fn codex_new_falls_back_to_bare_lisa() {
        let dir = Path::new("docs/active/tickets");
        let ctx = spawn_ctx(dir, "T-001", 1);
        // None and empty both degrade to the bare `lisa` (PATH lookup).
        for bin in [None, Some("")] {
            let cmd = CodexAdapter::new(bin, None).launch_command(&ctx);
            assert!(cmd.contains(" lisa agent-exec \""), "cmd: {cmd}");
        }
    }
}
