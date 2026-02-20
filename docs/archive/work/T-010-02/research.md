# T-010-02 Research: Event-driven Transition State Machine

## Problem

When a ticket's phase completes and the agent slot is reused, `schedule_ready_tickets()` immediately sends `/clear` to the pane. But Claude Code may still be streaming output at that moment, so `/clear` arrives as literal text instead of a command. A blind 15-second `FLUSH_DELAY_SECS` timer then sends the new prompt — fragile and unreliable.

## Current Transition Flow (lib.rs:348-397)

1. `schedule_ready_tickets()` finds an idle slot with `has_session = true`
2. **Immediately** sends `/clear` via `send_line_to_pane()` (line 352)
3. Queues the new prompt into `pending_pane_writes` (line 355)
4. Arms a 15s flush timer via `arm_timer(FLUSH_DELAY_SECS)` (line 395)
5. On timer fire, `flush_pending_pane_writes()` sends all queued prompts (lines 178-182)
6. Timer event handler at line 1510-1511 calls `flush_pending_pane_writes()` before poll_tick

### Relevant State Fields

- `pending_pane_writes: Vec<(u32, String)>` — queued commands (State, line 144)
- `pending_timer_count: u32` — timer dedup counter (State, line 148)
- `FLUSH_DELAY_SECS: f64 = 15.0` — constant (line 25)

## Signal Infrastructure (from T-010-01)

T-010-01 scaffolded hooks that produce three signal file types in `.lisa/signals/`:

| Hook | Event | Signal File | Purpose |
|------|-------|-------------|---------|
| `on-idle.sh` | Notification[idle_prompt] | `pane-{id}.idle` | Agent finished and went idle |
| `on-stop.sh` | Stop | `pane-{id}.stopped` | Claude finished responding (ready for input) |
| `on-clear.sh` | SessionStart[clear] | `pane-{id}.cleared` | /clear processed, context cleared |

Currently, only `.idle` signals are consumed by the plugin (in `check_idle_signals()`, line 526). The `.stopped` and `.cleared` signals are written by hooks but **not yet read** by any plugin code.

## Existing Signal Processing Pattern (check_idle_signals, lines 526-700)

- Reads `signal_dir` with `std::fs::read_dir()`
- Filters by `.idle` extension
- Deletes signal file immediately after reading
- Resolves pane ID → ticket ID via agent_slots lookup
- Applies phase-specific logic (Implement = idle alone advances; Research/Design/Structure/Plan = idle + artifact)

This is the established pattern to follow for `.stopped` and `.cleared` processing.

## AgentSlot Structure (lines 69-75)

```rust
struct AgentSlot {
    pane_id: u32,
    ticket_id: Option<TicketId>,
    has_session: bool,
}
```

No transition tracking fields exist. Slots are created in `discover_slots()` (line 250). Released in `release_slot_for_ticket()` (line 285) — sets `ticket_id = None`, keeps `has_session = true`.

## Timer Architecture

- `arm_timer(secs)` calls `set_timeout(secs)` and increments `pending_timer_count`
- `timer_fired()` decrements counter; returns true when counter hits 0
- In `Event::Timer` handler (line 1509): always flushes pane writes, only runs `poll_tick()` when last timer
- `poll_tick()` re-arms with `POLL_INTERVAL_SECS` (5s) at end
- `FLUSH_DELAY_SECS` adds a second concurrent timer (15s)

The dual-timer system exists solely to support the delayed flush. With event-driven transitions, the flush timer becomes unnecessary.

## Constraints

- **WASI sandbox**: `SystemTime::now()` works in wasm32-wasip1 (used elsewhere in the codebase for Thread timestamps)
- **Poll frequency**: 5s poll cycle determines how quickly signal files are detected. Timeout fallbacks must account for this granularity.
- **Stop hook fires every turn**: The `.stopped` signal arrives after every Claude response, not just at phase completion. The state machine must ignore `.stopped` signals when not in `WaitingForStop` state.
- **Signal deletion**: Must delete signal files immediately after reading (same pattern as `.idle`) to prevent re-triggering.
- **No zellij APIs in tests**: Tests can't call `send_line_to_pane()`. Existing tests verify state transitions without invoking host functions.

## Files to Modify

| File | Changes |
|------|---------|
| `crates/lisa-plugin/src/lib.rs` | Add TransitionState enum, AgentSlot fields, signal processing, timeout checking, remove pending_pane_writes/FLUSH_DELAY_SECS |
| `crates/lisa-plugin/src/lib.rs` (state dump) | Update dump_state_to_file to include transition_state (line 1019) |

## Test Surface

Existing tests that touch affected code paths:
- `test_check_idle_signals_*` (6 tests, lines ~3470-3795) — pattern to follow
- `test_check_artifact_advances_*` (3 tests, lines ~2049-2200) — verifies phase advancement
- Schedule-related tests that reference `pending_pane_writes` — will need updating

The `TransitionState` enum and state machine logic are pure state transitions (no zellij calls), so they're fully testable on native.
