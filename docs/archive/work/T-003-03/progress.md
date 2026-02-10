# Progress: T-003-03 completion-reschedule

## Completed

### Step 1: Add `AllTicketsDone` to ActivityEvent
- Added `AllTicketsDone` variant to `ActivityEvent` in `crates/lisa-core/src/types.rs`

### Step 2: Add State fields
- Added `terminated: bool` to `State` struct in `crates/lisa-plugin/src/lib.rs`
- Removed planned `tick_count` and `last_activity_tick` — not needed because T-003-02 already added `Thread::health()` with `last_phase_change` and `HealthStatus`

### Step 3: Skip implement in `check_artifact_advances()`
- Added `continue` for `Phase::Implement` in `check_artifact_advances()`
- Rationale: `progress.md` is a living tracking document created early in implement, not a completion signal
- Updated existing test from `test_check_artifact_advances_implement_to_review_parks_thread` to `test_check_artifact_advances_implement_skipped`

### Step 4: Add `detect_stale_threads()`
- Uses `Thread::health()` with 30-minute threshold to detect stuck threads
- Stale threads: `fail()`, release slot, remove from threads map (enables retry)
- Logs error with ticket ID

### Step 5: Add `check_all_done()`
- Returns true when DAG is non-empty, all tickets at Phase::Done, and no running threads

### Step 6: Modify `poll_tick()`
- Calls `detect_stale_threads()` after artifact checks
- Calls `check_all_done()` after scheduling
- If all done: logs `AllTicketsDone`, sets `terminated = true`, returns without re-arming timer

### Step 7: Handle `AllTicketsDone` in UI
- `activity_event_to_ui_entry()`: converts to PhaseCompleted with ticket_id "all"
- `render()`: shows "All tickets done. Lisa loop complete." when terminated

### Step 8: Tests
8 new tests added (40 total in lisa-plugin, up from 32):
- `test_check_all_done_true`
- `test_check_all_done_false_not_all_done`
- `test_check_all_done_false_running_thread`
- `test_check_all_done_empty_dag`
- `test_detect_stale_threads`
- `test_stale_thread_not_stale_yet`
- `test_all_tickets_done_event_conversion`
- `test_rescheduling_conditions_after_completion`

### Step 9: Full test suite
- 97 tests pass (57 lisa-core + 40 lisa-plugin)
- WASM check passes

## Deviations from Plan

1. **No tick counter**: Used existing `Thread::health()` + `HealthStatus` (from T-003-02) instead of tick-based staleness detection. Simpler, more accurate (uses real wall clock time).

2. **Rescheduling test**: Can't directly test `schedule_ready_tickets()` because it calls `write_chars_to_pane_id()` (zellij host function). Tested preconditions instead (slot freed, DAG shows ready, no existing thread).
