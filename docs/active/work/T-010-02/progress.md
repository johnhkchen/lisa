# T-010-02 Progress

## Step 1: Add TransitionState enum and update AgentSlot
- [x] Added `TransitionState` enum (Idle, WaitingForStop, WaitingForClear) with Default derive
- [x] Added `transition_state` and `transition_started_at` fields to `AgentSlot`
- [x] Replaced `FLUSH_DELAY_SECS` with `STOP_SIGNAL_TIMEOUT_SECS` (60) and `CLEAR_SIGNAL_TIMEOUT_SECS` (30)

## Step 2: Update discover_slots() and remove pending_pane_writes
- [x] Added field initializers in `discover_slots()`
- [x] Removed `pending_pane_writes` field from State
- [x] Removed `flush_pending_pane_writes()` method
- [x] Removed flush call in Timer handler

## Step 3: Modify schedule_ready_tickets() for deferred /clear
- [x] Replaced immediate `/clear` + queue with `WaitingForStop` state + timestamp
- [x] Removed conditional timer arming at bottom of function

## Step 4: Add signal processing methods
- [x] Added `check_transition_signals()` — reads signal_dir for .stopped/.cleared
- [x] Added `handle_stopped_signal()` — WaitingForStop → send /clear → WaitingForClear
- [x] Added `handle_cleared_signal()` — WaitingForClear → send prompt → Idle
- [x] Restructured both handlers to avoid borrow conflicts with log_activity

## Step 5: Add timeout checking
- [x] Added `check_transition_timeouts()` — collects timeout actions then executes

## Step 6: Wire into poll_tick() and update dump_state
- [x] Added `check_transition_signals()` and `check_transition_timeouts()` calls in poll_tick
- [x] Updated `dump_state_to_file()` with per-slot transition state info

## Step 7: Fix existing tests
- [x] Updated all AgentSlot constructions to include new fields
- [x] Added `#[cfg(test)]` no-op for `send_line_to_pane()` to unblock native test builds

## Step 8: Add new tests (10 tests)
- [x] `test_transition_state_default_is_idle`
- [x] `test_check_transition_signals_stopped_advances_state`
- [x] `test_check_transition_signals_stopped_ignored_when_idle`
- [x] `test_check_transition_signals_cleared_advances_state`
- [x] `test_check_transition_signals_cleared_ignored_when_idle`
- [x] `test_check_transition_signals_unknown_pane_ignored`
- [x] `test_check_transition_timeouts_stop_timeout`
- [x] `test_check_transition_timeouts_clear_timeout`
- [x] `test_check_transition_timeouts_within_threshold_no_change`
- [x] `test_check_transition_signals_idle_files_not_consumed`

## Deviation: #[cfg(test)] no-op for send_line_to_pane

Zellij FFI host functions (`write_chars_to_pane_id`, `write_to_pane_id`) are unavailable on native targets. Previously, no test referenced these transitively, so the linker dead-stripped them. With transition signal tests now exercising handlers that call `send_line_to_pane()`, the linker needed these symbols.

Solution: `send_line_to_pane()` is a no-op under `#[cfg(test)]`. The real implementation compiles for WASM. This unblocks both T-010-02 and T-010-03 tests.

## Final verification
- WASM check: `cargo check -p lisa-plugin --target wasm32-wasip1` — pass
- All tests: `cargo test --workspace` — 306 tests pass (105 CLI + 78 core + 123 plugin)
