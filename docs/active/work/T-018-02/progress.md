# Progress: T-018-02 session-timeout-enforcement

## Completed

- [x] Step 1: Added `ActivityEvent::SessionTimedOut` variant to `crates/lisa-core/src/types.rs`
- [x] Step 2: Added `AlertType::TimedOut` variant to `crates/lisa-plugin/src/ui.rs` with render match arm
- [x] Step 3: Added `timeout_alerts` field to `State` struct (derives Default, no manual init needed)
- [x] Step 4: Implemented `check_session_timeouts()` method on `State`
- [x] Step 5: Wired `check_session_timeouts()` into `poll_tick()` between `evaluate_health()` and `detect_stale_threads()`
- [x] Step 6: Handled `SessionTimedOut` in both `activity_event_to_ui_entry()` and `format_activity_event()`
- [x] Step 7: Included `timeout_alerts` in `to_ui_state()` as `HealthAlert` entries
- [x] Step 8: Added timeout alert cleanup in `schedule_ready_tickets()` on reschedule
- [x] Step 9: All tests pass — 395 total (151 CLI + 96 core + 148 plugin), WASM compiles clean

## Tests Added

| Test | Status |
|------|--------|
| `test_check_session_timeouts_expired` | Pass |
| `test_check_session_timeouts_not_expired` | Pass |
| `test_check_session_timeouts_disabled` | Pass |
| `test_session_timed_out_event_to_ui` | Pass |
| `test_to_ui_state_includes_timeout_alerts` | Pass |

## Deviations from Plan

- Had to add `SessionTimedOut` handling in `format_activity_event()` (used for state snapshots) in addition to `activity_event_to_ui_entry()`. This was missed in the plan but caught by the compiler (exhaustive match).
- No deviations in architecture or approach.
