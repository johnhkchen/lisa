# Plan: T-018-02 session-timeout-enforcement

## Step 1: Add `ActivityEvent::SessionTimedOut` to types.rs

Add the new variant to the `ActivityEvent` enum in `crates/lisa-core/src/types.rs`:
```rust
SessionTimedOut {
    ticket_id: TicketId,
    elapsed_secs: u64,
    phase: Phase,
}
```

**Verification**: `cargo check --workspace` passes. No tests needed for the enum variant itself.

## Step 2: Add `AlertType::TimedOut` to ui.rs

Add `TimedOut` variant to `AlertType` enum in `crates/lisa-plugin/src/ui.rs`.

Add match arm in `render_attention_banner()`:
```rust
AlertType::TimedOut => ("⏱ TIMEOUT", YELLOW),
```

**Verification**: `cargo check -p lisa-plugin --target wasm32-wasip1` passes.

## Step 3: Add `timeout_alerts` field to State and Default impl

In `crates/lisa-plugin/src/lib.rs`:
- Add `timeout_alerts: Vec<(TicketId, u64, Phase)>` to `State` struct
- Initialize to `Vec::new()` in `Default` impl

**Verification**: `cargo check -p lisa-plugin --target wasm32-wasip1` passes.

## Step 4: Implement `check_session_timeouts()`

Add method to `impl State`:
1. Early return if `self.config.session_timeout_secs == 0`
2. Compute `now` and `timeout` Duration
3. Collect timed-out ticket IDs (to avoid borrow conflicts)
4. For each: compute elapsed, log `SessionTimedOut`, `thread.fail()`, release slot, remove thread, push to `timeout_alerts`

**Verification**: Write test `test_check_session_timeouts_expired` — create a thread with `started_at` set to 31 minutes ago (past 1800s default). Assert thread removed, slot released, activity log has `SessionTimedOut`.

## Step 5: Wire into `poll_tick()`

Add `self.check_session_timeouts();` call between `evaluate_health()` and `detect_stale_threads()`.

**Verification**: Existing tests still pass. The new check is a no-op for threads within timeout.

## Step 6: Handle `SessionTimedOut` in `activity_event_to_ui_entry()`

Add match arm to convert `SessionTimedOut` to a UI activity entry with appropriate text like:
```
"T-024-01 timed out after 32m (in implement phase)"
```

**Verification**: Write test `test_session_timed_out_event_to_ui`.

## Step 7: Include timeout alerts in `to_ui_state()`

In the `to_ui_state()` method, convert `self.timeout_alerts` entries to `HealthAlert`s with `AlertType::TimedOut` and append to the alerts vec.

**Verification**: Write test `test_to_ui_state_includes_timeout_alerts`.

## Step 8: Clear timeout alerts on reschedule

In `schedule_ready_tickets()`, when a ticket is scheduled, remove it from `timeout_alerts`:
```rust
self.timeout_alerts.retain(|(tid, _, _)| tid != &ticket_id);
```

**Verification**: Write test `test_check_session_timeouts_not_expired` (fresh thread, no timeout).
Write test `test_check_session_timeouts_disabled` (session_timeout_secs = 0, no action).

## Step 9: Run full test suite

`cargo test --workspace` — all tests pass including new ones.
`cargo check -p lisa-plugin --target wasm32-wasip1` — WASM compilation passes.

## Test Summary

| Test | What it verifies |
|------|-----------------|
| `test_check_session_timeouts_expired` | Thread past timeout is removed, slot released, event logged |
| `test_check_session_timeouts_not_expired` | Fresh thread is unaffected |
| `test_check_session_timeouts_disabled` | `session_timeout_secs = 0` means no enforcement |
| `test_session_timed_out_event_to_ui` | ActivityEvent renders correctly |
| `test_to_ui_state_includes_timeout_alerts` | Dashboard shows timeout alerts |
