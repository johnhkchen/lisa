# T-026-02 · Research — provider-aware concurrency

Descriptive map of the concurrency machinery relevant to making limits
provider-aware and stress-validating ~16 mixed-provider agents. What exists,
where, how it connects. No solutions here.

## 1. The one knob today: a single global `max_threads`

`PluginConfig.max_threads: usize` (`types.rs:528`, default `2` at
`types.rs:590`). It is the **only** concurrency limit in the system. There is no
per-provider notion anywhere — grep for `per_provider` / `provider_cap` returns
nothing.

Resolution chain: `.lisa.toml [scheduling].max_threads` → `resolve_config`
(`config.rs:129-131`, precedence `--max-threads` CLI > file > default) →
emitted into the layout plugin block as the `max_threads` key
(`loop_cmd.rs:251`) → parsed back by `PluginConfig::from_config_map`
(`types.rs:639-643`). Validation rejects `0` (`config.rs:240`).

## 2. Where the cap is enforced: `schedule_ready_tickets`

`crates/lisa-plugin/src/lib.rs:544-716`. Per ready ticket, in order:

1. Skip if a live thread already exists for it (`:554-569`).
2. **Global cap gate** (`:571-581`):
   ```rust
   let running_count = self.threads.values()
       .filter(|t| t.status == ThreadStatus::Running).count();
   if running_count >= self.config.max_threads { unscheduled += 1; continue; }
   ```
   `running_count` counts **all** running threads regardless of provider.
3. `find_idle_slot()` (`:584`).
4. Skip if the target pane is awaiting a human (`:600-603`).
5. **Resolve route + adapter** (`:611-615`): `resolve_adapter_or_native(ticket,
   self.config.client, lisa_bin)` → `(Box<dyn AgentAdapter>, ResolvedRoute)`.
   Note this happens **after** the cap gate — the provider is not yet known when
   the cap is checked.
6. Launch or reuse (`:622-659`), store `route`, `client`, and
   `concurrency_at_spawn = running_count` on the new `Thread` (`:677-684`).

Key consequence: because the route is resolved *after* the cap check, a
per-provider cap needs the provider known earlier. `resolve_route`
(`route.rs:120`) is pure and cheap, so it can move up.

## 3. Slots are provider-blind terminals; reuse picks reset strategy from the *new* ticket

`AgentSlot` (`lib.rs:118-137`): `pane_id`, `ticket_id: Option`, `has_session:
bool`, `transition_state`, `cooldown_until`, `last_activity_at`. **No field
records which agent client last ran in the pane.**

`find_idle_slot()` (`lib.rs:504-514`) returns the first slot that is
unassigned, past cooldown, and (if it had a session) signal-silent for the
wind-down window. It is provider-agnostic — any idle slot serves any ticket.

Reuse path (`:623-659`): `adapter.reset_strategy()` is read from the **new
ticket's** adapter:
- `ClearHandshake` (native Claude): sends `/clear` into the pane, waits for a
  `.cleared` signal. Assumes a Claude REPL is live in the pane.
- `FreshExec` (native Codex): types a fresh `codex exec` wrapper line into the
  pane's **shell** (codex exits after each run, leaving a shell prompt).

Since a Claude session *stays running* for reuse but a Codex session *exits*,
the pane's real state depends on which provider last ran there — but the reset
strategy is chosen from the incoming ticket. Mixed-provider reuse can therefore
mismatch: a Codex ticket reusing a pane that still has a live Claude REPL would
`FreshExec`-type a shell command into Claude's input; a Claude ticket reusing a
Codex-vacated shell would `/clear` a bare shell and wait for a `.cleared` that
never comes (falls back via `check_transition_timeouts`). This is the primary
mixed-provider correctness hazard, and it is invisible to the single global cap.

## 4. Layout pre-creates 2× panes

`loop_cmd.rs:216-225`: `let pane_count = config.max_threads * 2;` — one terminal
pane per slot. Comment: "Extra idle panes absorb new tickets while finishing
panes wind down." So physical slots = `2 × max_threads`; the *cap* still admits
at most `max_threads` running threads (§2). To reach 16 concurrent, `max_threads
= 16` → 32 panes.

## 5. Thread carries the resolved client already

`Thread.client: AgentClient` (`types.rs:383`) is set at spawn to `route.agent`
(`lib.rs:682`). So a per-provider running count is already computable today:
`threads.values().filter(|t| t.status == Running && t.client == p).count()`.
`Thread.route: Option<ResolvedRoute>` (`types.rs:399`) carries requested+actual.

## 6. Signal directory: read every tick, once per signal kind

`poll_tick` (`lib.rs:1959-2078`) is the per-cadence heart. It calls, in order,
functions that each `std::fs::read_dir(&self.signal_dir)` independently:
`check_heartbeat_signals` (`:877`), `check_awaiting_signals` (`:920`),
`check_idle_signals` (`:965`), `check_transition_signals` (`:1178`),
`check_error_signals` (`:1230`). That is **≥5 full directory scans per tick**,
each O(files in dir), each also `remove_file`-ing matches. Signal files are
`pane-<id>.{heartbeat,awaiting,idle,stopped,cleared,error}` plus `<ticket>.idle`.
At 32 panes with the PostToolUse hook writing a `.heartbeat` per tool call, the
dir churns fast; the ticket note explicitly asks to measure poll-tick cost with
~32 panes' worth of signal files before assuming it is fine.

Codex adapter reports `signals()` all-false (idle/awaiting/cleared)
(`adapter.rs:321-329`) — only core `.heartbeat`/`.stopped`/`.error` appear for
Codex panes. So the idle/awaiting/cleared machinery is Claude-only; on a Codex
pane those signals simply never arrive (the scans still run, just find nothing).

## 7. Commit serialization is a documented convention, not enforced code

`/host/.lisa-commit.lock` appears only as a **path logged** in diagnostics
(`lib.rs:2867`, `diagnostics.rs:20-30`). No `flock`/lock acquisition exists in
the Rust code (grep confirms). `rdspi-workflow.md:109-111` states "commit
serialization is handled via file locking — agents do not need to coordinate,"
but the injected workflow gives agents no concrete `flock` command. In practice
the real serializer is **git's own `.git/index.lock`**: concurrent
`git commit`s fail-visibly ("unable to lock index") rather than corrupt, and the
agent retries. This matters for the stress target: at high N, index-lock
contention is a real (soft, visible) ceiling factor, not a silent deadlock.

## 8. Provenance consumes concurrency-at-spawn (global)

`emit_provenance` writes a `ProvenanceRecord` at teardown (`provenance.rs:80-100`)
carrying `requested`/`actual` routes, `concurrency_at_spawn` (the global running
count at spawn, `:98`), and `pane_id`. Per-provider concurrency is **not** stored
directly, but because each record carries `actual.agent`, the ledger can be
queried to reconstruct per-provider concurrency after the fact (T-027-01's
"concurrency-at-run" interpretation). This is the downstream consumer the ticket
says our findings must feed.

## 9. Per-provider reality: separate auth + rate-limit pools

`doctor.rs` checks exactly one binary per loop (`check_claude` / `check_codex`,
selected by `AgentClient`), and Codex needs directory-trust pre-seeding
(`pregrant_codex_trust`, Codex-only). Claude and Codex authenticate
independently and rate-limit independently. A single global cap of N cannot
protect either pool: 16 all-Claude threads hammer one provider's limits, while
8+8 splits the load. This is the whole motivation for a per-provider sub-cap.

## 10. Constraints & assumptions

- The plugin runs in WASM: no threads, no subprocess; work is `read_dir` + file
  I/O on `/host`. Cheap but not free at 32 panes × 5 scans/tick.
- Adapters are pure "command describers" (`adapter.rs`), so all launch/reuse
  decisions are testable natively without a Zellij host.
- Tests must be native, no live agents (acceptance criterion 4) — everything
  above (cap gating, slot selection, config parse) is reachable in `#[cfg(test)]`
  as the existing `test_concurrency_cap_respects_max_threads` (`lib.rs:6452`) and
  `test_find_idle_slot_*` (`:7556`) demonstrate.
- Global `max_threads` must remain the hard ceiling; a per-provider cap is a
  *sub-limit*, and defaults must leave single-provider loops byte-for-byte
  unchanged (acceptance criterion 1).
