# Progress: T-005-04 Thread Lifecycle Cleanup

## Completed

### Step 1: Added `audit_threads()` method
- New method on `State` after `detect_stale_threads()`
- Removes threads whose ticket is `phase: Done` or missing from DAG
- Logs `ActivityEvent::Error` with "Orphaned thread" message for each removal
- Also releases the associated agent slot

### Step 2: Modified `poll_tick()` — remove completed threads + call audit
- Added `self.threads.remove(ticket_id)` in the done-ticket detection loop (after `release_slot_for_ticket`)
- Added `self.audit_threads()` call after `sweep_stale_slots()` and before `schedule_ready_tickets()`

### Step 3: Modified `mark_ticket_done()` — remove thread
- Added `self.threads.remove(&tid)` after `release_slot_for_ticket`

### Step 4: Modified `schedule_ready_tickets()` — defensive guard
- Replaced simple `contains_key` check with two-path logic
- If thread is `Completed`: remove it and fall through to scheduling
- Otherwise: skip with existing log message

### Step 5: Updated existing tests
- `test_done_ticket_detected_on_first_poll`: now asserts thread is removed (not just Completed)
- `test_done_ticket_detected_between_polls`: same update
- `test_rescheduling_conditions_after_completion`: now includes thread removal step

### Step 6: Added 6 new tests
- `test_completed_thread_removed_dependent_scheduled` — end-to-end: done detection removes thread, dependent is ready
- `test_defensive_guard_removes_completed_thread` — stale Completed thread cleaned up by guard
- `test_audit_threads_removes_done_ticket_thread` — audit removes thread for done ticket
- `test_audit_threads_removes_missing_ticket_thread` — audit removes thread for ticket not in DAG
- `test_audit_threads_keeps_active_thread` — audit preserves threads for active tickets
- `test_mark_done_removes_thread` — mark-done flow removes thread

## Verification
- `cargo check -p lisa-plugin --target wasm32-wasip1` — clean (no new warnings)
- `cargo test --workspace` — 182 tests pass (was 176, added 6 new)
- No new warnings introduced in lib.rs
