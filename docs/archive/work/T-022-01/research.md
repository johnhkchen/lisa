# T-022-01 · Research — Adapter interface extraction

Descriptive map of the Claude-Code-specific behaviour in the plugin and the seams
the adapter interface must own. No solutions proposed here.

## Scope anchor

The whole plugin lives in `crates/lisa-plugin/src/lib.rs` (7568 lines) — the
`scheduler.rs`/`ui.rs` split in CLAUDE.md is aspirational; only `ui.rs` is split
out today. All state, scheduling, and signal handling is in `lib.rs`. This
matters: the refactor happens inside one large module, and the existing unit
tests live in the same file (`#[cfg(test)] mod tests`) and reach directly into
private items (`State`, `AgentSlot`, `TransitionState`, free functions). Those
tests are the no-op proof, so whatever the adapter interface becomes, those
items and free functions must keep compiling and passing **unmodified**.

## The five Claude-specific behaviours (per the ticket)

### 1. Launch — command construction for a fresh pane
- `build_claude_command(ticket_dir, ticket_id, pane_id) -> String` (`lib.rs:53`).
  Produces `LISA_PANE_ID={} LISA_TICKET_ID={} claude --dangerously-skip-permissions "{prompt}"`.
- The prompt body comes from `ticket_prompt(ticket_dir, ticket_id)` (`lib.rs:34`),
  which is **workflow content** (RDSPI instructions), not client-specific. Only
  the *wrapper* (env vars + `claude` binary + `--dangerously-skip-permissions`
  flag + quoting) is Claude-specific.
- Called once, at `schedule_ready_tickets` `lib.rs:582`, in the `!has_session`
  (fresh pane) branch.
- Proven by `test_build_claude_command*` (`lib.rs:3144`, `3160`, `3172`) which
  assert the exact prefix, env vars, ticket path, and RDSPI reference. These are
  the byte-for-byte anchors.

### 2. Reuse / reset — the `/clear` → `.cleared` handshake
- When a slot already `has_session`, reuse does **not** relaunch. Instead
  (`schedule_ready_tickets` `lib.rs:568-586`): send `/clear` into the live TUI,
  set `transition_state = WaitingForClear`, stamp `transition_started_at`, and
  stash the next prompt (`ticket_prompt`) to send once cleared.
- The state machine is `TransitionState` (`lib.rs:129`): `Idle` →
  `WaitingForStop` → `WaitingForClear` → `Idle`. It gates `/clear` and prompt
  sends on hook-written signal files rather than blind timers.
- `check_transition_signals` (`lib.rs:1085`) scans `pane-<id>.stopped` /
  `pane-<id>.cleared`, deleting each after read.
  - `.stopped` → `handle_stopped_signal` (`lib.rs:1129`): if `WaitingForStop`,
    send `/clear`, advance to `WaitingForClear`; if `Idle` + ticket in Review,
    `auto_complete_review`.
  - `.cleared` → `handle_cleared_signal` (`lib.rs:1246`): if `WaitingForClear`,
    send the stashed `ticket_prompt` and return to `Idle`.
- `check_transition_timeouts` (`lib.rs:1289`) is the fallback: after
  `STOP_SIGNAL_TIMEOUT_SECS`=60 / `CLEAR_SIGNAL_TIMEOUT_SECS`=90 **and** the pane
  has been quiet for `wind_down_secs`, it forces the next step anyway.
- **Codex relevance:** for Codex, reuse = a fresh `codex exec`, so the whole
  `/clear`/`.cleared` handshake is inapplicable. The applicability of this state
  machine is therefore something an adapter must declare — today it is
  hardcoded into the scheduler.

### 3. Follow-up injection — `finish_up_prompt` typed into the live TUI
- `finish_up_prompt(ticket_dir, work_dir, ticket_id) -> String` (`lib.rs:63`)
  builds a "please finish your review.md" nudge.
- `check_review_timeouts` (`lib.rs:1368`) finds running Review threads past
  `review_timeout_secs`, quiet for `wind_down_secs`, not already in
  `finish_up_sent`, not `awaiting_human`, and **types the prompt into the pane**
  via `send_line_to_pane` (`lib.rs:1403`).
- The mechanism — keystrokes into a live TUI — is Claude-specific. Under
  `codex exec` there is no persistent TUI to type into; the analog is a spawned
  `codex exec resume` command (epic Decision 4). So the interface needs a
  "send follow-up" operation whose *mechanism* is adapter-owned.

### 4. Expected signal set — which signals this adapter emits
- Signals consumed today, all as files in `signal_dir` (`.lisa/signals/`),
  named `pane-<id>.<kind>`, deleted after read:
  - `.heartbeat` — `check_heartbeat_signals` (`lib.rs:779`): liveness; clears
    `awaiting_human` and `notified_attention`.
  - `.awaiting` — `check_awaiting_signals` (`lib.rs:815`): pane blocked on an
    `AskUserQuestion`; suppresses all injection into that pane.
  - `.idle` — `check_idle_signals` (`lib.rs:862`): idle-without-artifact alerts;
    also legacy `{ticket_id}.idle` naming.
  - `.stopped` / `.cleared` — transition machine (above).
- Per the normalized contract (S-022, doc 08 §5): `.heartbeat`/`.stopped`/`.error`
  + usage/cost are the cross-vendor core; `.idle`/`.awaiting`/`.cleared` are
  **Claude-only**. The scheduler must be able to treat *absence* of the
  Claude-only signals correctly for a non-Claude adapter — e.g. never expect a
  `.cleared` from an adapter that resets by fresh exec.
- **Gap (owned by sibling T-022-02, not here):** there is **no `.error` signal
  consumer** today. The normalized contract requires one. T-022-01 must leave
  room for it in the signal-capability description but not implement it.

### 5. Selection seam — per-ticket adapter resolution at spawn
- Today there is no seam: `schedule_ready_tickets` hardcodes `build_claude_command`.
- Requirement: resolve the adapter **per ticket at spawn time** — a resolver
  function taking the ticket and returning the adapter — even though the MVP
  resolves every ticket to native Claude unconditionally. No whole-loop-only
  constant (doc 08 §7, epic S-022 needs).

## State & data structures involved

- `State` (`lib.rs:156`): the plugin god-struct (`#[derive(Default)]`). Holds
  `agent_slots`, `threads`, `config: PluginConfig`, `signal_dir`, sets like
  `awaiting_human`, `finish_up_sent`. Tests build it via `State::default()` with
  struct-update syntax, so any new field must be `Default`-able.
- `AgentSlot` (`lib.rs:95`): `pane_id`, `ticket_id`, `has_session`,
  `transition_state`, `transition_started_at`, `cooldown_until`,
  `last_activity_at`. Constructed literally in `discover_slots` (`lib.rs:438`)
  and in ~20 test sites — adding a required field here is expensive (touches
  every test literal), so prefer not to.
- `PluginConfig` (`lisa-core/src/types.rs:470`): dirs, `max_threads`,
  `wind_down_secs`, timeouts. No client/provider field today.
- `send_line_to_pane` (`lib.rs:276`): the single injection primitive; has a
  built-in `awaiting_human` guard. Both the launch command and reuse `/clear`
  and prompts go through it.

## Constraints & assumptions

- **WASM sandbox:** adapters run inside the plugin. They may only *write shell
  commands into panes* and *read signal files*; the only host-process capability
  is Zellij's fire-and-forget `run_command_with_env_variables_and_cwd`
  (`lib.rs:361`, used for `on-notify`). No persistent stdio pipe. So an adapter's
  "operations" reduce to producing strings/commands the scheduler injects, plus
  declaring which signal files to expect (doc 06 §"The constraint that filters
  everything").
- **No Codex here:** T-023-02 implements Codex; T-022-01 is a pure refactor.
- **No-op proof = existing tests unmodified:** `test_build_claude_command*`,
  `test_check_transition_signals_*`, `test_*_skips_when_awaiting`,
  `test_check_review_timeouts_*`. If any needs editing, it is not a no-op.
- **Host-path handling:** commands run on the host, so paths are passed through
  `strip_host_prefix` (`lib.rs:89`) before command construction. The adapter must
  receive already-host-relative paths (as today) — path stripping is not an
  adapter concern.
- **Native Claude is the anchor leg** (doc 08 §5); the interface must
  accommodate a native-Codex `exec` wrapper and a future ACP host-side bridge
  without redesign — documented in the trait doc comment (AC 4).

## Call-site inventory (what the refactor must touch or preserve)

| Behaviour | Call site | Free fn today |
|---|---|---|
| Fresh launch | `schedule_ready_tickets` `:582` | `build_claude_command` |
| Reuse reset | `schedule_ready_tickets` `:575` | `/clear` literal |
| Reuse prompt | `schedule_ready_tickets` `:579`, `handle_cleared_signal` `:1267` | `ticket_prompt` |
| Follow-up | `check_review_timeouts` `:1402` | `finish_up_prompt` |
| Transition FSM | `check_transition_signals/_timeouts` | `TransitionState` |
| Signal scan | `check_*_signals` | file suffixes |

The seam is narrow: launch string, reset strategy, follow-up mechanism, and a
declared signal set. Everything else (DAG, phase detection, thread bookkeeping,
`send_line_to_pane`) is already client-agnostic and must stay untouched.
