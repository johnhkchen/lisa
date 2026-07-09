# T-022-01 · Design — Adapter interface extraction

Goal: extract Claude-specific launch / reuse-reset / follow-up / signal-set
behaviour behind an adapter interface resolved per-ticket at spawn, as a
**provable no-op**. Native Claude is the only implementation; with no opt-in
every existing behaviour is byte-for-byte unchanged.

## Decision summary

Introduce a `trait AgentAdapter` (dynamic dispatch, `Box<dyn AgentAdapter>`) with
one implementation, `ClaudeCodeAdapter`, plus a free resolver
`resolve_adapter(ticket) -> Box<dyn AgentAdapter>` that returns
`ClaudeCodeAdapter` unconditionally in the MVP. The adapter owns four things:
launch command, reset strategy, follow-up action, and a declared signal
capability set. The existing free functions (`build_claude_command`,
`ticket_prompt`, `finish_up_prompt`) stay and become the adapter's
implementation, so the no-op proof (existing tests) holds unmodified.

## What the interface must own (from research)

1. **Launch** — fresh-pane command string.
2. **Reuse/reset** — strategy for a slot that already `has_session`. Claude =
   `/clear` + `.cleared` handshake; Codex = fresh exec. The *applicability of the
   `TransitionState` machine* is adapter-declared, not scheduler-hardcoded.
3. **Follow-up** — a "send follow-up" operation whose mechanism (type-into-TUI vs
   spawn-command) is adapter-owned.
4. **Expected signal set** — which of `.idle`/`.awaiting`/`.cleared` this adapter
   emits, so the scheduler treats absence correctly. (`.heartbeat`/`.stopped`
   are the normalized core; `.error` consumer is T-022-02.)
5. **Selection** — per-ticket resolver at spawn, MVP → native Claude.

## Option A — Enum `Client { Claude }` with `match` at call sites

Two-variant enum, branch on it in `schedule_ready_tickets` etc.

- **Rejected.** The story explicitly says *"an adapter interface (not a
  two-variant enum)."* An enum forces every future leg (Codex, ACP) to edit
  every `match`, and encodes the whole-loop assumption the thesis forbids
  (doc 08 §7). Fails AC 4 (accommodate new adapters without redesign).

## Option B — Full command bus: adapter returns an `Action` for every scheduler event

Adapter exposes `fn on_event(&self, ev, ctx) -> Vec<PaneAction>` and the
scheduler becomes a thin dispatcher; the entire transition FSM moves behind the
trait.

- **Rejected for this ticket.** Maximally future-proof but a large behavioural
  rewrite of the transition machine — precisely the code the no-op tests pin.
  High byte-for-byte risk for zero MVP benefit, and it over-implements before
  Codex exists (violates the graduation/anti-over-engineering guardrail,
  doc 08 §5). The FSM can migrate behind the trait later when a second adapter
  actually needs a different machine; today `reset_strategy()` is enough to
  express applicability.

## Option C (chosen) — Trait with narrow, data-returning operations + resolver

`ClaudeCodeAdapter` implements a small trait; the scheduler calls it at the four
seams and acts on the returned data. The adapter returns **descriptions**
(strings / small enums), never performs host I/O itself — this fits the WASM
constraint (adapter can't pipe; only the scheduler's `send_line_to_pane` /
`run_command` touch the host) and keeps host calls centralized and testable.

### The trait

```rust
/// A pluggable agent client. Each integration *method* (native Claude Code,
/// native Codex `exec` wrapper, future ACP bridge) implements this to supply
/// the behaviour that differs per method, while the scheduler consumes only
/// normalized signals and stays client-agnostic.
///
/// WASM constraint: adapters cannot pipe to subprocesses. Every method returns
/// a *description* (a command string or a small action enum) that the scheduler
/// injects into a pane or hands to Zellij `run_command`. Adapters never do host
/// I/O directly.
///
/// Accommodates without redesign:
/// - native **Codex** (`codex exec --json` wrapper): `launch_command` builds the
///   exec invocation; `reset_strategy` → `FreshExec` (no `/clear`); `follow_up`
///   → `SpawnCommand("codex exec resume …")`; `signals` omits idle/awaiting/cleared.
/// - future **ACP** host-side bridge: same shape; the bridge process writes the
///   normalized signal files, so only `launch_command` + `signals` differ.
trait AgentAdapter {
    /// Command to launch a fresh session in an empty pane.
    fn launch_command(&self, ctx: &SpawnContext) -> String;

    /// How a slot that already has a session is reset before new work.
    fn reset_strategy(&self) -> ResetStrategy;

    /// The next-prompt injection for a *reused* session once it is ready
    /// (post-`.cleared` for Claude). Kept separate from launch because reuse
    /// sends the bare prompt, not the wrapped launch command.
    fn reuse_prompt(&self, ctx: &SpawnContext) -> String;

    /// The follow-up nudge for a parked Review session.
    fn follow_up(&self, ctx: &FollowUpContext) -> FollowUp;

    /// Which optional signals this adapter emits, so the scheduler can treat
    /// absence correctly for non-Claude adapters.
    fn signals(&self) -> SignalCapabilities;
}
```

### Supporting types

```rust
/// Inputs a fresh launch / reuse prompt needs. Host-relative paths (already
/// `strip_host_prefix`-ed by the caller).
struct SpawnContext<'a> { ticket_dir: &'a Path, ticket_id: &'a str, pane_id: u32 }

struct FollowUpContext<'a> { ticket_dir: &'a Path, work_dir: &'a Path, ticket_id: &'a str, pane_id: u32 }

/// How a reused pane is reset before its next prompt.
enum ResetStrategy {
    /// Send `/clear` into the live TUI and wait for `.cleared` (Claude).
    ClearHandshake,
    /// Reuse is a fresh launch; no in-place reset handshake (Codex/ACP).
    FreshExec,
}

/// A follow-up delivery mechanism.
enum FollowUp {
    /// Type the text into the live TUI (Claude).
    TypeIntoPane(String),
    /// Spawn a host command, e.g. `codex exec resume` (Codex/ACP).
    SpawnCommand(String),
}

/// Optional signals beyond the normalized `.heartbeat`/`.stopped`(/`.error`)
/// core. `false` = the scheduler must not wait on / expect this signal.
struct SignalCapabilities { idle: bool, awaiting: bool, cleared: bool }
```

### Native Claude implementation (delegates to existing free functions)

```rust
struct ClaudeCodeAdapter;
impl AgentAdapter for ClaudeCodeAdapter {
    fn launch_command(&self, c: &SpawnContext) -> String {
        build_claude_command(c.ticket_dir, c.ticket_id, c.pane_id)   // unchanged fn
    }
    fn reset_strategy(&self) -> ResetStrategy { ResetStrategy::ClearHandshake }
    fn reuse_prompt(&self, c: &SpawnContext) -> String {
        ticket_prompt(c.ticket_dir, c.ticket_id)                     // unchanged fn
    }
    fn follow_up(&self, c: &FollowUpContext) -> FollowUp {
        FollowUp::TypeIntoPane(finish_up_prompt(c.ticket_dir, c.work_dir, c.ticket_id))
    }
    fn signals(&self) -> SignalCapabilities {
        SignalCapabilities { idle: true, awaiting: true, cleared: true }
    }
}
```

### Resolver (per-ticket, spawn-time)

```rust
/// Resolve the adapter for a ticket at spawn time. MVP: always native Claude.
/// The `_ticket` parameter is the per-pane-resolvable seam (doc 08 §7): S-026
/// will read `(provider, model)` from frontmatter here without changing callers.
fn resolve_adapter(_ticket: &Ticket) -> Box<dyn AgentAdapter> {
    Box::new(ClaudeCodeAdapter)
}
```

## How the scheduler consumes it (no-op wiring)

- **Fresh launch** (`schedule_ready_tickets` `!has_session`): replace
  `build_claude_command(...)` with
  `resolve_adapter(ticket).launch_command(&ctx)`. Native → identical string.
- **Reuse** (`has_session`): branch on `reset_strategy()`. `ClearHandshake` runs
  today's exact code (`/clear`, `WaitingForClear`, stash `reuse_prompt`).
  `FreshExec` is `unreachable`/`todo` in the MVP (no adapter returns it) — kept
  as the documented seam, not dead behaviour.
- **Cleared handler** (`handle_cleared_signal`): the stashed prompt is
  `adapter.reuse_prompt(&ctx)` == `ticket_prompt(...)`. Identical.
- **Follow-up** (`check_review_timeouts`): `match adapter.follow_up(&ctx)` —
  `TypeIntoPane(s)` → `send_line_to_pane(&s, ...)` == today. `SpawnCommand` arm
  is the documented Codex seam (unused in MVP).
- **Signal set**: `SignalCapabilities` is consumed where the scheduler *waits*
  on an optional signal. For native (all `true`), no branch changes behaviour.
  This is mostly a declaration T-022-02 / Codex build on; the MVP wires it into
  at most a guard that is a no-op when `cleared: true`.

## Where the adapter is obtained

Resolve **at each spawn/decision site**, not stored on `State`. Rationale: keeps
`State: Default` (tests build it directly, `AgentSlot` literals untouched), makes
selection genuinely per-ticket (a stored loop-wide adapter would bake the
whole-loop assumption the thesis forbids), and the adapter is zero-sized so
re-resolving per call is free. When S-026 adds `(provider, model)`, resolution
stays a pure function of the ticket.

## No-op argument (why this is provably safe)

1. The three string-producing free functions are **unchanged**; the adapter only
   calls them. `test_build_claude_command*` assert against the free fn → pass.
2. Native returns `ClearHandshake` + all-`true` signals, so every scheduler
   branch taken is the one taken today. Transition tests
   (`test_check_transition_signals_*`, `test_*_skips_when_awaiting`) exercise the
   same path → pass.
3. `follow_up` for native yields `TypeIntoPane(finish_up_prompt(...))` injected
   via the same `send_line_to_pane` → `test_check_review_timeouts_*` pass.
4. No new `State`/`AgentSlot` fields → no test literal changes.

## Rejected extras (guardrail)

- Storing an adapter registry / trait objects on `State` — unneeded, breaks
  `Default`, invites the loop-wide constant.
- Implementing the `.error` consumer here — explicitly T-022-02.
- Implementing `FreshExec`/`SpawnCommand` bodies — explicitly T-023-02 (Codex).
  They exist as documented, unreachable-in-MVP seams so the shape is proven.
