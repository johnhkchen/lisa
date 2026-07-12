# Research — T-037-01-01 provider-readiness-capability

## Ticket in one line

Expose, at the `AgentAdapter` boundary, whether a provider proves bootstrap
readiness via a pre-prompt process-start signal (Claude / `SessionStart`) or
needs a bounded startup grace to pace its first prompt (Codex). The scheduler
must *read* that mode at launch dispatch — a settled classification only, with
no seat-state behavior change in this ticket.

## Why this exists (epic/story grounding)

- `docs/active/epic/E-037.md`: Codex 0.144.1 emits `SessionStart` only *after*
  the first prompt creates a thread. E-035 gates the first prompt on a positive
  pre-prompt `SessionStart`, so a bare Codex launch waits for a signal that can
  never arrive → it sits in `Starting` forever. Claude emits `SessionStart` on
  process start and succeeds. The fix is *provider-aware* readiness.
- `docs/active/stories/S-037-01.md` explicitly waves this ticket alone because
  "both the grace transition and the tests depend on the settled readiness-mode
  shape, and it lives in `adapter.rs`, disjoint from the state machine."
  T-037-01-02 lands the grace-pacing transition; T-037-01-03 adds the
  delayed-send / prompt-miss tests. Both of those edit the `lib.rs` `Starting`
  region; **this** ticket must not.

## The adapter boundary — `crates/lisa-plugin/src/adapter.rs`

The `AgentAdapter` trait (lines 125–176) is the per-integration-method seam. It
already carries a capability-descriptor method precedent:

- `fn signals(&self) -> SignalCapabilities` (line 175) — a `Copy` struct of
  bool fields (`idle`, `awaiting`, `cleared`) telling the scheduler which
  *optional* signals a provider emits. `ClaudeCodeAdapter` returns all-true
  (lines 242–249); `CodexAdapter` returns `{idle:false, awaiting:false,
  cleared:true}` (lines 362–368).
- `fn reset_strategy(&self) -> ResetStrategy` (line 162) — another pure
  classification the scheduler branches on (`ClearHandshake` vs `FreshExec`).

`SignalCapabilities` and `ResetStrategy` are the exact shape to mirror: a small
`Copy` classification enum returned by a trait method, one variant per adapter.
`readiness_mode()` belongs beside `signals()`.

Two adapter impls exist and are the only two:
- `ClaudeCodeAdapter` (lines 185–250) — depth/reliability anchor. Uses
  `ResetStrategy::ClearHandshake`. Its launch line runs `claude` with hooks that
  emit `SessionStart` on process start (the positive pre-prompt evidence).
- `CodexAdapter` (lines 265–369) — native Codex TUI. `interactive_line`
  (lines 316–329) launches `codex` bare; the module comment on `interactive_line`
  notes "Assignment text is deliberately absent and arrives through chat only
  after SessionStart" — the very assumption E-037 says is false for Codex.

`resolve_adapter` / `resolve_adapter_or_native` (lines 395–428) return a
`Box<dyn AgentAdapter>` per ticket route. The scheduler always holds an adapter
at dispatch, so a new trait method is reachable everywhere a launch happens.

## The state machine — `crates/lisa-plugin/src/lib.rs` (DO NOT MUTATE HERE)

`SeatAssignmentState` (lines 280–336) is the bootstrap-readiness machine, keyed
by physical `pane_id` in `State::seat_assignments: HashMap<u32,
SeatAssignmentState>` (line 433). Relevant states:

- `Starting { generation, start_deadline: Option<SystemTime>, relaunches }`
  (284–292) — fresh process launched, exact process-start signal not yet
  observed. `start_deadline` is `None` until the launcher is actually submitted.
- `ReadyForAssignment { generation }` (301) — set by `acknowledge_process_start`
  (1387–1415) *only* on the exact current attempt lease's process-start signal.
- `Delivering { generation, ack_deadline, retries }` (304–308) — bounded chat
  reference submitted, awaiting `UserPromptSubmit`.
- Terminal: `Owned`, `StartupFailed`, `DeliveryFailed`, `RecoveryFailed`.

Key transition sites (all owned by 02/03, not this ticket):
- `acknowledge_process_start` (1387) — `Starting → ReadyForAssignment` on
  positive `SessionStart` evidence. This is Claude's path; it is the path E-037
  says must stay unchanged.
- `deliver_ready_assignments` (1499) / `deliver_assignment_to_pane` (1429) —
  `ReadyForAssignment → Delivering`.
- `expire_*` deadline sweep (2127+) — `Starting`/`Delivering` timeouts →
  `StartupFailed` / retry.

## Launch dispatch — where the scheduler must read the mode

`schedule_ready_tickets` is the primary dispatch loop. Relevant anchors:

- Adapter+route resolved once per ticket: `let (adapter, route) =
  resolve_adapter_or_native(...)` (line 2468).
- `SpawnContext` built (2578–2585); assignment persisted (2590).
- Launch branches: recycle/cross-provider exit (2618), same-process reuse
  (2666), fresh pane (2713).
- **Fresh-launch `Starting` insertion (2747–2765):** `fresh_launch = recycle ||
  !reused_seat || reset_strategy == FreshExec`; if `fresh_launch`, insert
  `SeatAssignmentState::Starting { generation, start_deadline: None, relaunches:
  0 }`. This is the canonical "launch dispatch" site.
- A second fresh `Starting` is set in the post-exit recovery relaunch path
  (3996–4003), inside a different method, keyed by `recovery_generation`.

The scheduler holds `adapter` at both sites, so `adapter.readiness_mode()` is
readable there without re-resolving.

## How capability reads are observed/tested today

- Adapter-level: `adapter.rs` `#[cfg(test)]` module (430+) asserts each method's
  per-adapter value directly, e.g. `native_signals_all_true` (528),
  `codex_signals_include_clear_handshake` (736), `codex_reset_is_clear_handshake`
  (712). A `readiness_mode` pair of tests slots in identically.
- Scheduler-level: tests drive `state.schedule_ready_tickets()` then assert
  `state.seat_assignment(pane)` (e.g. `test_pane_title_fresh_launch_uses_actual_
  fallback_route`, 9582–9608, asserts `Starting {..}` after a Codex dispatch).
  Helper `pane_name_schedule_state(requested_agent, default_agent, resident)`
  (9414–9448) builds a one-ticket state with pane 10. This is the harness a
  "scheduler reads the mode at dispatch" test reuses — it needs an *observable*
  record of the read.

## Constraints / assumptions

- **Disjoint from the state machine.** No new field on `SeatAssignmentState`, no
  change to any existing transition. 02 owns the `Starting` region.
- **No behavior change.** Identical seat states, deadlines, launch lines, logs.
  The read must be observationally inert beyond its own recorded classification.
- **The read must be observable** for a native test to assert "the scheduler
  reads that mode at launch dispatch" — implies recording it somewhere the test
  can inspect (a per-pane map is the natural, machine-disjoint choice; mirrors
  how `seat_assignments` is keyed by `pane_id`).
- WASM constraint: adapters return descriptions, never perform I/O; a pure enum
  classifier honors that.
- Only two adapters exist; `AgentClient` has exactly `Claude` and `Codex`
  variants, so the mode enum is a closed two-variant set today.
