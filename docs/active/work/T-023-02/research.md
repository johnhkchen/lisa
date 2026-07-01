# T-023-02 · Research — Codex adapter

Descriptive map of the code this ticket touches. What exists, where, how it
connects. No solutions here — those are `design.md`.

## The task in one line

Wire the already-built Codex wrapper (`lisa agent-exec`, T-023-01) into the
already-built adapter interface (`AgentAdapter`, T-022-01) as a concrete
`CodexAdapter`, resolvable at spawn, and feed it the absolute `lisa` binary path
so the launch command it types into a pane is `<abs-lisa> agent-exec …`.

## The adapter interface (T-022-01) — `crates/lisa-plugin/src/adapter.rs`

The seam is already shaped for exactly this. Key facts:

- `trait AgentAdapter` (`:125`) with five methods: `launch_command(&SpawnContext)
  -> String`, `reset_strategy() -> ResetStrategy`, `reuse_prompt(&SpawnContext)
  -> String`, `follow_up(&FollowUpContext) -> FollowUp`, `signals() ->
  SignalCapabilities`.
- `SpawnContext<'a>` (`:55`): `ticket_dir: &Path`, `ticket_id: &str`, `pane_id:
  u32`. Paths are already host-relative (caller ran `strip_host_prefix`).
- `FollowUpContext<'a>` (`:62`): adds `work_dir: &Path` and `pane_id` (the doc
  comment on `pane_id` already says it is "Read by a future Codex SpawnCommand
  follow-up (T-023-02)").
- `enum ResetStrategy { ClearHandshake, FreshExec }` (`:73`). `FreshExec` exists,
  `#[allow(dead_code)]`, doc-noted as "native Codex" — this ticket is its first
  live returner.
- `enum FollowUp { TypeIntoPane(String), SpawnCommand(String) }` (`:87`).
  `SpawnCommand` exists, `#[allow(dead_code)]`, doc-noted "codex exec resume".
- `struct SignalCapabilities { idle, awaiting, cleared: bool }` (`:110`) — the
  three *Claude-only* optional signals. Codex emits none of them.
- `ClaudeCodeAdapter` (`:149`) delegates to the free functions; its outputs are
  byte-for-byte the pre-adapter behaviour (the no-op proof).
- `resolve_adapter(ticket: &Ticket) -> Box<dyn AgentAdapter>` (`:184`) currently
  ignores the ticket and always returns Claude. `resolve_adapter_or_native(Option
  <&Ticket>)` (`:193`) is the null-safe wrapper the scheduler actually calls.

The module doc (`:37-43`) already describes the intended Codex mapping:
`launch_command` builds `codex exec`, `reset_strategy` = `FreshExec`, `follow_up`
= `SpawnCommand` wrapping resume, `signals` reports the Claude-only set absent.

## The wrapper (T-023-01) — `crates/lisa-cli/src/agent_exec.rs` + `main.rs`

Already shipped (untracked in the working tree). Relevant contract:

- CLI surface `main.rs:76-104`: `lisa agent-exec <prompt> [--resume] [--codex-bin
  codex] [--cwd .] [--bypass-sandbox] [--codex-arg …] [--signal-dir .lisa/signals]`.
- It reads `LISA_PANE_ID` / `LISA_TICKET_ID` from the **environment** (inherited
  from the pane launch) for signal attribution and the resume key.
- Writes `pane-<id>.{heartbeat,stopped,error}` under `--signal-dir` (default
  `.lisa/signals`, resolved against the pane's cwd).
- Persists `thread_id` under `.lisa/codex/<key>.thread` where `key` = ticket id
  (else `pane-<id>`, else `last`). `--resume` reads it back.
- Degrades safely: no `LISA_PANE_ID` ⇒ runs codex, writes no signals.

So the adapter's job is purely to construct the **shell line** that launches this
subcommand with the right env prefix and prompt — the wrapper does the rest.

## The scheduler consumption sites — `crates/lisa-plugin/src/lib.rs`

Four call sites already route through the resolver (all no-ops for Claude today):

1. **Fresh launch / reuse** — `schedule_ready_tickets` (`:579-617`):
   - `let adapter = resolve_adapter_or_native(self.dag.get_ticket(&ticket_id));`
   - `has_session == false` → `adapter.launch_command(&ctx)` sent via
     `send_line_to_pane`, `has_session = true`.
   - `has_session == true` → `match adapter.reset_strategy()`:
     - `ClearHandshake` → send `/clear`, set `TransitionState::WaitingForClear`,
       stamp `transition_started_at`.
     - `FreshExec` → **`unreachable!("no FreshExec adapter … in the MVP")`** (`:608`).
       This is the arm this ticket must implement.
   - `launch_cmd` is used only for the `SessionLaunch` activity-log `command`
     field (`:655`).

2. **Cleared-signal reuse** — `handle_cleared_signal` (`:1367-1392`): fires only
   for a `WaitingForClear` slot, re-resolves the adapter, calls `reuse_prompt`,
   sends it. **Codex never enters `WaitingForClear`, so this site is inert for
   Codex** — no change needed beyond passing the binary through the resolver.

3. **Clear-timeout fallback** — around `:1465-1473`: same shape as (2), same
   inertness for Codex.

4. **Review finish-up prod** — `check_review_timeouts` (`:1514-1550`):
   - `let follow_up = adapter.follow_up(&FollowUpContext { ticket_dir, work_dir,
     ticket_id, pane_id });`
   - `FollowUp::TypeIntoPane(prompt)` → `send_line_to_pane`.
   - `FollowUp::SpawnCommand(_)` → **`unreachable!("no SpawnCommand … in the MVP")`**
     (`:1539`). The second arm this ticket must implement.

### Signal polling (already Codex-shaped by prior tickets)

- `.heartbeat` / `.stopped`: consumed by the same `read_dir` + `strip_prefix`
  idiom every signal uses; the wrapper emits both. `.stopped` drives
  slot release / completion exactly as for Claude.
- `.error`: consumed by `check_error_signals` (T-022-02) — fails the thread,
  releases the slot, raises a `Failed` alert immediately. The wrapper emits
  `.error` (plus a compat `.stopped`) on failure. **This path is already wired;
  T-023-02 does not touch it.**
- `.idle` / `.awaiting` / `.cleared`: Codex never writes them. `is_pane_awaiting`
  (checks `.awaiting`) and idle detection simply never fire for a Codex pane —
  their absence is self-correct. The `SignalCapabilities` type is *declared* but
  has no behavioural consumer yet (T-022-01 structure `:109-115` deferred it).

### Phase advancement

`check_artifact_advances` advances a ticket's `phase` on **artifact presence
alone** (research.md, design.md, …). This is client-agnostic — it watches the
work dir, not any Claude-specific signal. Codex writing artifacts advances phases
identically. `Phase::Ready → Research` is force-stamped at spawn (`:630-644`).

## The binary-path gap — `crates/lisa-cli/src/loop_cmd.rs`

The Codex launch line must call lisa by **absolute path** (`current_exe()` at
`lisa loop` time — no PATH assumption; the pane shell may not have `lisa` on PATH).
Today nothing carries it:

- `generate_layout(wasm_path, config)` (`:199-248`) emits the plugin block with
  `ticket_dir`, `story_dir`, `work_dir`, `max_threads`, `auto_advance`,
  `review_timeout_secs`, `session_timeout_secs`, `wind_down_secs`. **No binary
  path key.**
- `run_loop` (`:10-86`) has no `current_exe()` capture.
- `PluginConfig` (`crates/lisa-core/src/types.rs:470-514`) + `from_config_map`
  (`:559-619`) parse each of those keys. **No `lisa_bin` field.**
- The plugin's `load` (`lib.rs:2654-2669`) `/host`-prefixes the *relative*
  dir configs. A binary path is an absolute **host** path (the pane shell runs on
  the host, outside the `/host` mount) so it must **not** be prefixed.

## Constraints & assumptions

- **WASM boundary**: adapters return `String`/enums only; the plugin does all
  I/O via `send_line_to_pane`. `SpawnCommand` at the plugin level is still a
  `send_line_to_pane` of a shell line (there is no host `spawn` from WASM) — the
  "spawn" happens in the pane's shell, not the plugin. The enum name reflects
  *intent* (a fresh process vs typing into a live TUI), not a different plugin
  mechanism.
- **Not user-selectable yet**: the resolver must be able to *reach* Codex (a
  test-only path is explicitly acceptable), but production still resolves Claude
  until T-025-01 adds the toggle and T-026-01 reads routing frontmatter.
- **Claude untouched**: `ClaudeCodeAdapter`, the free functions, the FSM, and all
  existing tests must be byte-for-byte unchanged.
- **Shell quoting**: `build_claude_command` (`:58`) already wraps the prompt in
  plain double quotes with no escaping; the prompts contain no `"`/`$`/backtick/
  backslash, so the Codex line can mirror that exposure exactly (no worse, no
  better).
- **Prompt content**: reuse `ticket_prompt` / `finish_up_prompt` verbatim
  (Note in ticket: AGENTS.md substitution is T-025-02; explicit file paths in the
  current prompt already work for Codex).
- **No quota logic** in the adapter (T-026-02).
</content>
</invoke>
