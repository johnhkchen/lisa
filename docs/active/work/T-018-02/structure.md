# Structure: T-018-02 session-timeout-enforcement

## Files Modified

### 1. `crates/lisa-core/src/types.rs`

**Add `ActivityEvent::SessionTimedOut` variant** (after `FinishUpPromptSent`):
```rust
SessionTimedOut {
    ticket_id: TicketId,
    elapsed_secs: u64,
    phase: Phase,
}
```

No other changes to types.rs. ThreadStatus remains as-is.

### 2. `crates/lisa-plugin/src/lib.rs`

**Add field to `State` struct:**
```rust
/// Recent session timeouts for dashboard display.
/// Entries: (ticket_id, elapsed_secs, phase_at_timeout).
/// Cleared when ticket is rescheduled.
timeout_alerts: Vec<(TicketId, u64, Phase)>,
```

**Add `check_session_timeouts()` method to `impl State`:**
- Guarded by `session_timeout_secs == 0` (disabled check)
- Iterates running threads, compares `now - started_at` against timeout
- For each timed-out thread: logs `SessionTimedOut`, calls `fail()`, releases slot, removes thread, appends to `timeout_alerts`
- Collects ticket IDs first to avoid borrow conflicts (same pattern as `detect_stale_threads`)

**Add call in `poll_tick()`** — insert between `evaluate_health()` and `detect_stale_threads()`:
```rust
self.check_session_timeouts();
```

**Update `schedule_ready_tickets()`** — when a ticket is scheduled, remove it from `timeout_alerts` (the timeout is no longer relevant if it gets a new session).

**Update `to_ui_state()`** — convert `timeout_alerts` into `HealthAlert`s with `AlertType::TimedOut`.

**Update `activity_event_to_ui_entry()`** — handle the new `SessionTimedOut` variant for the activity log display.

**Update `State::default()`** — initialize `timeout_alerts: Vec::new()`.

### 3. `crates/lisa-plugin/src/ui.rs`

**Add `AlertType::TimedOut` variant:**
```rust
TimedOut,
```

**Update `render_attention_banner()`** — add match arm for `AlertType::TimedOut`:
```rust
AlertType::TimedOut => ("⏱ TIMEOUT", YELLOW),
```

### 4. No new files created. No files deleted.

## Module Boundaries

- `types.rs` (core): Only adds the new ActivityEvent variant. No logic changes.
- `lib.rs` (plugin): Contains all enforcement logic. Owns `timeout_alerts` state.
- `ui.rs` (plugin): Only rendering changes. Receives data via `PluginState`.

## Ordering

1. Add `ActivityEvent::SessionTimedOut` to types.rs (other code depends on it)
2. Add `AlertType::TimedOut` to ui.rs (lib.rs references it)
3. Add `timeout_alerts` field and `check_session_timeouts()` to lib.rs
4. Wire into `poll_tick()`, `to_ui_state()`, `schedule_ready_tickets()`, `activity_event_to_ui_entry()`
5. Tests

## Test Strategy

- Unit test: `check_session_timeouts` with a thread past the timeout — verify thread removed, slot released, activity logged, timeout_alerts populated
- Unit test: `check_session_timeouts` with a fresh thread — verify no change
- Unit test: `check_session_timeouts` with `session_timeout_secs = 0` — verify disabled (no action)
- Unit test: `to_ui_state` includes timeout alerts from `timeout_alerts` vec
- Unit test: `activity_event_to_ui_entry` handles `SessionTimedOut`
- Existing tests remain unchanged
