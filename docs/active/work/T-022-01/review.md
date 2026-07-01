# T-022-01 · Review — Adapter interface extraction

Handoff document. What changed, how it was proven a no-op, test coverage, and
open concerns. This is a **provable no-op refactor**: with no opt-in, native
Claude is the only resolvable adapter and every existing behaviour is
byte-for-byte unchanged.

## What changed

### Created

- **`crates/lisa-plugin/src/adapter.rs`** (~250 lines incl. tests) — the seam:
  - `trait AgentAdapter` — `launch_command`, `reset_strategy`, `reuse_prompt`,
    `follow_up`, `signals`. Extensive module + trait doc comments show how a
    native-Codex `exec` wrapper and a future ACP bridge fit **without redesign**
    (AC 4).
  - Support types: `SpawnContext`, `FollowUpContext`, `ResetStrategy`
    (`ClearHandshake`/`FreshExec`), `FollowUp` (`TypeIntoPane`/`SpawnCommand`),
    `SignalCapabilities` (`idle`/`awaiting`/`cleared`).
  - `ClaudeCodeAdapter` — the sole implementation; delegates to the existing free
    functions so its output is identical to the pre-adapter code.
  - `resolve_adapter(&Ticket)` — the per-ticket, spawn-time selection seam; MVP
    ignores the ticket and returns native Claude. `resolve_adapter_or_native`
    handles the DAG-miss fallback.

### Modified

- **`crates/lisa-plugin/src/lib.rs`**
  - `mod adapter;` + import of the seam types.
  - `ticket_prompt` / `build_claude_command` / `finish_up_prompt` → `pub(crate)`
    (visibility only; **not moved**, so the string-anchor tests are unchanged).
  - Four call sites rewired through a resolved adapter:
    - `schedule_ready_tickets` fresh launch → `adapter.launch_command`.
    - `schedule_ready_tickets` reuse → `match adapter.reset_strategy()`
      (`ClearHandshake` = today's `/clear` handshake; `FreshExec` = `unreachable!`).
    - `handle_cleared_signal` + `check_transition_timeouts` clear-timeout →
      `adapter.reuse_prompt`.
    - `check_review_timeouts` → `match adapter.follow_up()` (`TypeIntoPane` =
      today's `send_line_to_pane`; `SpawnCommand` = `unreachable!`).

### Deleted

- None.

## How the no-op is proven

1. **Free functions unchanged.** The three string producers keep identical
   signatures and bodies; the adapter only calls them. `test_build_claude_command`,
   `_includes_env_vars`, `_includes_rdspi_reference` assert against the free
   function and pass unmodified.
2. **Native takes today's branches.** `ClaudeCodeAdapter::reset_strategy()` is
   always `ClearHandshake` and `follow_up()` is always `TypeIntoPane`, so every
   scheduler code path executed is the one executed before the refactor. The
   `FreshExec` / `SpawnCommand` arms are `unreachable!` because no resolvable
   adapter returns them in the MVP.
3. **No state shape change.** No fields added to `State` or `AgentSlot`, so all
   `State::default()` / `AgentSlot { .. }` test literals are untouched.
4. **Transition + review tests unchanged and green:**
   `test_check_transition_signals_stopped_advances_state`,
   `_cleared_advances_state`, `test_stopped_signal_skips_when_awaiting`,
   `test_cleared_signal_skips_when_awaiting`,
   `test_transition_timeouts_skip_when_awaiting`,
   `test_check_review_timeouts_sends_prompt_after_timeout`, `_idempotent`.

## Test coverage

- **New (7, in `adapter::tests`):** native adapter output equals each free
  function (`native_launch_matches_free_fn`, `native_reuse_prompt_matches_free_fn`,
  `native_follow_up_is_type_into_pane`); `native_reset_is_clear_handshake`;
  `native_signals_all_true`; `resolver_returns_claude_for_any_ticket`;
  `resolver_or_native_handles_missing_ticket`.
- **Regression surface:** the whole existing plugin suite (177) exercises the
  rewired call sites through the native adapter and is green.
- **Full workspace:** 175 (core) + 106 (cli) + 184 (plugin) = **465 passing, 0
  failing.** WASM release build succeeds.

### Coverage gaps (acknowledged)

- **No end-to-end pane drive.** The tests assert command *strings* and state
  transitions, not that Zellij injects them — consistent with the existing suite
  (host calls are unmockable in native tests). Behaviour parity rests on the
  branch-identity argument above, not on a live pane test.
- **`FreshExec` / `SpawnCommand` arms are `unreachable!`, so untested by
  construction.** They are compile-checked, documented seams; their behaviour is
  T-023-02's responsibility. If a future refactor makes a non-Claude adapter
  resolvable *before* those arms are implemented, the `unreachable!` becomes a
  live panic — see concern C1.
- **`signals()` has no behavioural consumer yet**, so only its return value is
  tested, not any scheduler reaction to a `false` capability.

## Open concerns / TODOs

- **C1 — `unreachable!` guards depend on the resolver.** They are correct only
  while `resolve_adapter` returns native Claude unconditionally. The two tickets
  that add a non-Claude adapter (T-023-02) **must** implement the `FreshExec` and
  `SpawnCommand` arms in the same change that makes such an adapter resolvable.
  This coupling is documented at each `unreachable!` and in the module docs, but
  it is a real ordering constraint a reviewer of the Codex work should enforce.
- **C2 — `.error` consumer still absent.** Deliberately out of scope (sibling
  T-022-02). `SignalCapabilities` leaves room for it but the normalized contract
  is not yet complete; the scheduler still has no `.error` handling.
- **C3 — `signals()` is declared, not consumed.** The "expected-signal-set" AC is
  satisfied by the declaration + docs, but the scheduler does not yet branch on
  it. That wiring lands with T-022-02 / the Codex adapter; until then a
  hypothetical adapter reporting `cleared: false` would still be driven through
  the `ClearHandshake` path if its `reset_strategy()` said so — the two are
  independent knobs today.
- **C4 — Adapter is re-resolved per call, not cached.** `ClaudeCodeAdapter` is
  zero-sized so this is free now, but if a future adapter carries per-ticket
  config (e.g. a model id), resolution cost/identity should be revisited at the
  spawn site (S-026 concern, not this ticket).

## Critical issues for human attention

None blocking. The change is a mechanical, test-backed no-op. The one thing a
human overseer should carry forward is **C1**: the `unreachable!` seams are a
deliberate tripwire that the Codex ticket must retire as it lands — they should
never ship reachable-but-panicking.
