# Review: T-018-02 session-timeout-enforcement

## Summary of Changes

Added session timeout enforcement to the Lisa WASM plugin. When a running agent session exceeds `session_timeout_secs` (configured via `.lisa.toml`, parsed in T-018-01), the plugin marks the thread as failed, releases the scheduling slot, logs a `SessionTimedOut` activity event, and shows a `⏱ TIMEOUT` alert on the dashboard.

### Files Modified

| File | Change |
|------|--------|
| `crates/lisa-core/src/types.rs` | Added `ActivityEvent::SessionTimedOut { ticket_id, elapsed_secs, phase }` variant |
| `crates/lisa-plugin/src/ui.rs` | Added `AlertType::TimedOut` variant, render match arm (`"⏱ TIMEOUT"`, yellow) |
| `crates/lisa-plugin/src/lib.rs` | Added `timeout_alerts` field to `State`, `check_session_timeouts()` method, wired into `poll_tick()`, handled in `activity_event_to_ui_entry()` and `format_activity_event()`, included in `to_ui_state()`, cleanup on reschedule |

### Lines Changed

Approximately 120 lines added across 3 files. No files created or deleted (other than work artifacts).

## Acceptance Criteria Check

| Criterion | Status |
|-----------|--------|
| Plugin tracks session start time for each active thread | Done — `Thread::started_at` (pre-existing) is used |
| During `poll_tick`, check each active session against `session_timeout_secs` | Done — `check_session_timeouts()` called in `poll_tick()` |
| Log an `ActivityEvent` with timeout details: ticket ID, elapsed time, phase | Done — `SessionTimedOut { ticket_id, elapsed_secs, phase }` |
| Mark the thread as timed out | Done — `thread.fail()` then remove from map |
| Free the scheduling slot | Done — `release_slot_for_ticket()` called |
| Do NOT kill the Claude Code process | Done — no process management; `has_session` stays true |
| Dashboard UI shows timed-out sessions distinctly | Done — `AlertType::TimedOut` renders as `"⏱ TIMEOUT"` in attention banner |
| Timed-out session later produces artifacts → handle gracefully | Done — thread removed so no re-processing; on reschedule, existing artifacts are detected by `check_artifact_advances()` |

## Test Coverage

- **395 total tests**, all passing (up from 390 before this ticket)
- **5 new tests** added:
  - `test_check_session_timeouts_expired` — thread past timeout is removed, slot released, event logged, timeout_alerts populated
  - `test_check_session_timeouts_not_expired` — fresh thread is unaffected
  - `test_check_session_timeouts_disabled` — `session_timeout_secs = 0` means no enforcement
  - `test_session_timed_out_event_to_ui` — `SessionTimedOut` event renders as Warning with elapsed time and phase
  - `test_to_ui_state_includes_timeout_alerts` — dashboard includes timeout alerts from `timeout_alerts` vec
- **WASM compilation**: `cargo check -p lisa-plugin --target wasm32-wasip1` passes

### Coverage Gaps

- No integration test for the full `poll_tick()` → `check_session_timeouts()` → `schedule_ready_tickets()` cycle. This would require mocking Zellij host functions. Existing unit tests cover each step individually.
- No test for `timeout_alerts` cleanup on reschedule (the `retain` call in `schedule_ready_tickets`). This is hard to test because `schedule_ready_tickets` calls `write_chars_to_pane_id` (a Zellij host function). The logic is trivial (one-line retain).

## Design Decisions

1. **Reused `ThreadStatus::Failed`** instead of adding `TimedOut` variant. The thread is removed immediately after marking, so the status is transient. The distinction lives in `ActivityEvent::SessionTimedOut` and `AlertType::TimedOut`.

2. **`timeout_alerts` vec on State** for persistent dashboard visibility. Alerts survive the thread being removed and are cleared when the ticket is rescheduled.

3. **Placement in `poll_tick()`**: between `evaluate_health()` and `detect_stale_threads()`. Session timeout fires before per-phase staleness, so a timed-out thread isn't double-handled.

## Open Concerns

1. **No automatic retry**: Per the ticket, timed-out tickets are not retried in v1. The operator must manually re-trigger (e.g., reset the ticket to ready phase). This is the correct behavior for now.

2. **`timeout_alerts` growth**: Alerts accumulate until the ticket is rescheduled. In a long-running session with many timeouts, this could grow. In practice, this is bounded by the number of tickets. Could add a cap or age-out in the future if needed.

3. **T-018-03 dependency**: Per-phase timeout overrides (the stretch goal from S-018) build on this foundation. The `check_session_timeouts()` method checks total session time; per-phase would need to check `last_phase_change` with phase-specific limits.
