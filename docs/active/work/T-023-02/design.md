# T-023-02 · Design — Codex adapter

Options, tradeoffs, decisions — grounded in `research.md`. The interface
(T-022-01) and the wrapper (T-023-01) already exist; this ticket fills the two
`unreachable!` seams and threads one new piece of config.

## Decision 1 — Adapter shape: `CodexAdapter { lisa_bin }`

The adapter is a thin string-builder, mirroring `ClaudeCodeAdapter`. It holds the
one thing the launch line needs that the `SpawnContext` does not carry: the
absolute `lisa` binary path.

- `launch_command` → `LISA_PANE_ID=<n> LISA_TICKET_ID=<t> <lisa> agent-exec "<ticket_prompt>"`
- `reuse_prompt` → **identical to `launch_command`** (a fresh exec for the new
  ticket; there is no "bare prompt" mode because there is no live TUI).
- `reset_strategy` → `FreshExec`.
- `follow_up` → `SpawnCommand(LISA_PANE_ID=<n> LISA_TICKET_ID=<t> <lisa> agent-exec --resume "<finish_up_prompt>")`.
- `signals` → all `false` (Codex emits none of `.idle`/`.awaiting`/`.cleared`).

**Why reuse the prompt free functions** (`ticket_prompt`, `finish_up_prompt`)
verbatim: the ticket's Note says to, and Codex reads explicitly-named file paths
regardless (epic Intel B.7). AGENTS.md substitution is T-025-02, not here.

**Why hold `lisa_bin` on the struct** rather than threading it through every
`SpawnContext`: it is invariant per loop (captured once at `lisa loop` time), not
per-spawn. Putting it on the adapter keeps `SpawnContext`/`FollowUpContext`
unchanged — zero churn to the T-022-01 interface and its tests. Rejected
alternative: add `lisa_bin` to `SpawnContext` — would touch the Claude call sites
and every context literal for a value Claude never reads.

**Shell quoting**: mirror `build_claude_command` exactly — wrap the prompt in
plain double quotes, no escaping. The prompts contain no `"`/`$`/backtick/`\`, so
the exposure is identical to today's Claude line (Decision: match the existing
bar, do not invent new quoting the Claude path lacks).

## Decision 2 — Resolution: a provider seam, Codex reachable but not selected

The AC: Codex must be *resolvable via the T-022-01 resolver*, still **not**
user-selectable (T-025-01 owns the toggle), and a *test-only resolution path is
fine*.

Chosen shape:

```
enum Provider { Claude, Codex }
fn provider_for(_ticket: &Ticket) -> Provider { Provider::Claude }   // the seam
pub(crate) fn build_adapter(p: Provider, lisa_bin: Option<&str>) -> Box<dyn AgentAdapter>
pub(crate) fn resolve_adapter(ticket, lisa_bin) -> build_adapter(provider_for(ticket), lisa_bin)
```

- Production `provider_for` returns `Claude` unconditionally → **byte-for-byte
  the current behaviour** (the no-op guarantee holds). T-025-01/T-026-01 change
  only `provider_for` (read a config toggle / ticket frontmatter) — no caller
  moves.
- `build_adapter(Provider::Codex, …)` is the test-only resolution path: tests
  drive it directly to exercise the real `CodexAdapter`, satisfying "resolvable
  via the resolver" without exposing a user switch.

Rejected: make `resolve_adapter` sniff a ticket field now. That is T-026-01's job
and would ship a half-built toggle. Rejected: a global mutable "provider" — the
design thesis §7 mandates *per-pane-resolvable*, which `provider_for(ticket)`
already is.

`resolve_adapter` and `resolve_adapter_or_native` grow one parameter,
`lisa_bin: Option<&str>`, passed at all four lib.rs sites as
`self.config.lisa_bin.as_deref()`.

## Decision 3 — Fill the two `unreachable!` arms

1. **`FreshExec` reuse arm** (`schedule_ready_tickets`): send
   `adapter.launch_command(&ctx)` into the pane immediately and leave
   `transition_state` = `Idle`. No `/clear`, no `WaitingForClear`, no
   `transition_started_at`. This is the whole of "the `WaitingForStop`/
   `WaitingForClear` machinery must not engage for Codex panes" — because the
   machinery is only *armed* in the `ClearHandshake` arm, skipping it is
   sufficient; `handle_cleared_signal` and `check_transition_timeouts` stay inert
   for Codex panes (they act only on `WaitingForClear` slots).

2. **`SpawnCommand` follow-up arm** (`check_review_timeouts`): `send_line_to_pane
   (&cmd, PaneId::Terminal(pane_id))`. At the plugin level a "spawn" is still a
   typed shell line — the finished exec left the pane's shell at its prompt, so
   typing `agent-exec --resume "…"` re-launches codex. The enum name reflects
   *intent* (fresh process vs typing into a live TUI), not a distinct plugin
   mechanism (the WASM plugin cannot host-spawn; only `send_line_to_pane` exists
   for pane I/O).

## Decision 4 — Do **not** gate signal readers on `SignalCapabilities`

The research flagged that skipping `.idle`/`.awaiting`/`.cleared` readers for
Codex panes would require caching `SignalCapabilities` on each `AgentSlot` (the
readers iterate by pane, not ticket; there is no per-slot adapter today).

**Decision: skip this.** It is unnecessary and out of scope:

- Codex **never writes** `.idle`/`.awaiting`/`.cleared`, so those readers find no
  files for a Codex pane and are already inert. Gating would add a per-slot field
  (breaking `AgentSlot` literals in many tests) to prevent reads that already
  no-op.
- `signals()` returning all-`false` remains the **declaration** of the expected
  set (matching T-022-01's explicit deferral of a live consumer, structure
  `:109-115`). Keeping it declarative preserves the no-op guarantee for Claude
  and the AC ("expected-signal declarations") is met by the declaration itself.
- The one machinery the ticket explicitly calls out — `WaitingForStop`/
  `WaitingForClear` — is handled structurally by Decision 3.1, not by signal
  gating.

This keeps `State`/`AgentSlot` field-stable → every `State::default()` /
`AgentSlot { … }` test literal is untouched.

## Decision 5 — Config key `lisa_bin`, absolute host path, never `/host`-prefixed

`lisa loop` captures `std::env::current_exe()` and emits `lisa_bin "<abs>"` in the
layout's plugin block. `PluginConfig` gains `lisa_bin: Option<String>`, parsed in
`from_config_map`. Unlike `ticket_dir`/`work_dir`, it is **not** `/host`-prefixed
in `load` — the pane's shell runs on the host (outside the WASI mount), so the
path must be the real host path `current_exe` returns.

- `Option`, not a defaulted string: absence is meaningful. If `current_exe`
  fails or an older layout omits the key, `CodexAdapter::new(None)` falls back to
  the bare `lisa` (PATH lookup) — degrade-safely, never panic. Production never
  hits this (Claude is resolved), so the fallback only matters for a
  Codex-forcing test / future toggle.

Rejected: reuse `project_root` + a hardcoded `/lisa` — the binary is not in the
project root. Rejected: a `codex_bin` key too — the wrapper already defaults
`--codex-bin codex` and doctor pre-seeds it (T-025-01); the adapter stays free of
that concern.

## What is deliberately **not** changed

- `ClaudeCodeAdapter`, the three free functions, the `TransitionState` FSM, every
  signal reader, `ui.rs`, `lisa-core` types beyond the one `lisa_bin` field.
- No new dependency. `Box<dyn AgentAdapter>` is WASM-safe `alloc` (already used).

## Risks

- **R1 — Codex JSON shape is `[PROVISIONAL]`** (inherited from T-023-01). The
  adapter constructs the *launch line*, which is schema-independent, so this
  ticket is insulated; the residual lives in the wrapper's parser and is a
  documented downstream reconcile (T-023-01 review Open-concern #1).
- **R2 — `current_exe()` on exotic setups** may return a symlink/temp path. It is
  the documented "no PATH assumption" source the ticket names; the `None`
  fallback covers total failure. Acceptable.
- **R3 — end-to-end Codex run needs a live codex** (not in CI). Same reality as
  T-023-01; the pane-level run is a manual verification, string construction is
  unit-tested.
</content>
