# T-010-02 Plan: Event-driven Transition State Machine

## Step 1: Add TransitionState enum and update AgentSlot

**Changes:**
- Add `TransitionState` enum (Idle, WaitingForStop, WaitingForClear) with Default derive, placed after `ModalMode` enum
- Add `transition_state: TransitionState` and `transition_started_at: Option<SystemTime>` to `AgentSlot`
- Replace `FLUSH_DELAY_SECS` constant with `STOP_SIGNAL_TIMEOUT_SECS` and `CLEAR_SIGNAL_TIMEOUT_SECS`

**Verification:** `cargo check -p lisa-plugin --target wasm32-wasip1` compiles

## Step 2: Update discover_slots() and remove pending_pane_writes

**Changes:**
- Add field initializers in `discover_slots()` for new AgentSlot fields
- Remove `pending_pane_writes` field from State
- Remove `flush_pending_pane_writes()` method
- Remove `FLUSH_DELAY_SECS` constant
- Remove flush call in Timer handler (line 1511)

**Verification:** `cargo check -p lisa-plugin --target wasm32-wasip1` compiles

## Step 3: Modify schedule_ready_tickets() for deferred /clear

**Changes:**
- In the `has_session` branch: replace immediate `/clear` + queue with setting `transition_state = WaitingForStop` and `transition_started_at = Some(SystemTime::now())`
- Remove the conditional timer arming at bottom of function (`if !self.pending_pane_writes.is_empty()`)

**Verification:** `cargo check -p lisa-plugin --target wasm32-wasip1` compiles

## Step 4: Add signal processing methods

**Changes:**
- Add `check_transition_signals()` — reads signal_dir, filters `.stopped`/`.cleared`, dispatches
- Add `handle_stopped_signal(pane_id)` — if WaitingForStop, send /clear, move to WaitingForClear
- Add `handle_cleared_signal(pane_id)` — if WaitingForClear, send prompt, move to Idle

**Verification:** `cargo check -p lisa-plugin --target wasm32-wasip1` compiles

## Step 5: Add timeout checking

**Changes:**
- Add `check_transition_timeouts()` — iterate slots, check elapsed vs thresholds, force-advance

**Verification:** `cargo check -p lisa-plugin --target wasm32-wasip1` compiles

## Step 6: Wire into poll_tick() and update dump_state

**Changes:**
- Add `self.check_transition_signals()` and `self.check_transition_timeouts()` calls in `poll_tick()`, after `check_idle_signals()`
- Update `dump_state_to_file()` to show transition_state per slot instead of pending_pane_writes count

**Verification:** `cargo check -p lisa-plugin --target wasm32-wasip1` compiles

## Step 7: Fix existing tests

**Changes:**
- Update any tests that reference `pending_pane_writes` or `FLUSH_DELAY_SECS`
- Update AgentSlot construction in tests to include new fields

**Verification:** `cargo test --workspace` passes

## Step 8: Add new tests

**Tests:**
- `test_transition_state_default` — verify Default is Idle
- `test_check_transition_signals_stopped_file` — .stopped file triggers state change from WaitingForStop to WaitingForClear, file deleted
- `test_check_transition_signals_cleared_file` — .cleared file triggers state change from WaitingForClear to Idle, file deleted
- `test_check_transition_signals_idle_ignored` — .stopped/.cleared files ignored when slot is Idle
- `test_check_transition_timeouts_stop_timeout` — 60s+ elapsed forces WaitingForStop → WaitingForClear
- `test_check_transition_timeouts_clear_timeout` — 30s+ elapsed forces WaitingForClear → Idle
- `test_check_transition_timeouts_no_timeout` — within threshold, no change
- `test_check_transition_signals_unknown_pane` — unknown pane ID in signal file, no crash

**Verification:** `cargo test --workspace` passes with all new and existing tests

## Testing Strategy

- State machine transitions: verify enum values change correctly, timestamps set/cleared
- Signal file processing: use tempdir, write signal files, call method, assert file deleted + state changed
- Timeout fallbacks: set `transition_started_at` to past time, call check_transition_timeouts, verify forced advance
- Cannot verify `send_line_to_pane()` calls directly (zellij host fn) — verify state + log entries instead
- All existing tests must continue to pass after pending_pane_writes removal
