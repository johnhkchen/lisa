# Plan: T-003-03 completion-reschedule

## Step 1: Add `AllTicketsDone` to ActivityEvent

**File**: `crates/lisa-core/src/types.rs`

Add `AllTicketsDone` variant to the `ActivityEvent` enum after `DagRecomputed`. No fields needed.

**Verify**: `cargo check -p lisa-core` passes.

## Step 2: Add new State fields to lib.rs

**File**: `crates/lisa-plugin/src/lib.rs`

Add to the `State` struct:
- `tick_count: u64` — incremented each `poll_tick()` call
- `last_activity_tick: HashMap<TicketId, u64>` — records when each thread last had activity
- `terminated: bool` — whether all tickets are done

Update the `Default` derive to manual impl (or init these fields with defaults — u64 is 0, HashMap is empty, bool is false — so `Default` derive still works since these all implement Default).

**Verify**: `cargo check -p lisa-plugin --target wasm32-wasip1` passes.

## Step 3: Skip implement phase in `check_artifact_advances()`

**File**: `crates/lisa-plugin/src/lib.rs`

In `check_artifact_advances()`, add an early `continue` when `current_phase == Phase::Implement`. Add a comment explaining that progress.md is a tracking document, not a completion signal.

**Verify**: Existing test `test_check_artifact_advances_implement_to_review_parks_thread` now needs updating — it expects implement → review via progress.md. Update or replace this test:
- New behavior: progress.md does NOT trigger implement → review
- Add test: `test_implement_phase_skipped_in_artifact_advances` — create state with implement phase + progress.md, run check, verify no phase change

## Step 4: Add `detect_stale_threads()` method

**File**: `crates/lisa-plugin/src/lib.rs`

Add a constant `STALE_THRESHOLD_TICKS: u64 = 360` (30 min at 5s/tick).

Implement `detect_stale_threads()`:
1. Collect running threads whose `last_activity_tick` is older than threshold
2. For each stale thread: `thread.fail()`, release slot, remove from threads map, log error
3. The ticket phase is untouched — it remains schedulable for retry

**Test**: `test_stale_thread_detection` — create a thread, set tick_count past threshold, verify thread is removed and slot released.

## Step 5: Add `check_all_done()` method

**File**: `crates/lisa-plugin/src/lib.rs`

Returns true when:
- DAG is not empty
- All tickets in DAG have `phase == Phase::Done`
- No threads are still running

**Test**: `test_check_all_done_true` — all tickets done, no running threads → true.
**Test**: `test_check_all_done_false_running_thread` — all tickets done but thread still running → false.
**Test**: `test_check_all_done_false_not_all_done` — some tickets not done → false.

## Step 6: Modify `poll_tick()` to integrate new behavior

**File**: `crates/lisa-plugin/src/lib.rs`

Update `poll_tick()`:
1. Increment `self.tick_count`
2. Call `check_artifact_advances()` (existing)
3. Call `detect_stale_threads()` (new)
4. Call `rebuild_dag()` (existing)
5. If changed: update `last_activity_tick` for threads whose phase changed, then existing done-ticket handling and phase sync
6. Call `schedule_ready_tickets()` (existing)
7. Check `self.check_all_done()` — if true, log `AllTicketsDone`, set `self.terminated = true`, return without re-arming timer
8. Re-arm timer (existing)

**Test**: `test_poll_tick_terminates_when_all_done` — set up state where all tickets are done, call poll_tick logic, verify terminated is set.

## Step 7: Record activity ticks on spawn and phase change

**File**: `crates/lisa-plugin/src/lib.rs`

In `schedule_ready_tickets()`: after inserting a new thread, also insert into `last_activity_tick` with current `tick_count`.

In `poll_tick()` phase-change detection block: when a thread's phase changes, update `last_activity_tick`.

**Verify**: `cargo check -p lisa-plugin --target wasm32-wasip1` passes.

## Step 8: Handle `AllTicketsDone` in UI conversion

**File**: `crates/lisa-plugin/src/lib.rs`

In `activity_event_to_ui_entry()`, add a match arm for `ActivityEvent::AllTicketsDone` that returns an activity entry with a completion message.

Update `render()` to show a completion banner when `self.terminated` is true.

**Verify**: existing UI tests pass, add test for the new event conversion.

## Step 9: Run full test suite

Run `cargo test --workspace`. Fix any test failures. Ensure WASM check passes with `cargo check -p lisa-plugin --target wasm32-wasip1`.

Target: all existing tests pass + 5-7 new tests.

## Test Summary

| Test | Verifies |
|------|----------|
| `test_implement_phase_skipped_in_artifact_advances` | progress.md doesn't trigger implement advance |
| `test_stale_thread_detection` | Stale threads are failed and removed |
| `test_stale_thread_not_stale_yet` | Threads within threshold are not affected |
| `test_check_all_done_true` | All done + no running = true |
| `test_check_all_done_false_running_thread` | Running thread = false |
| `test_check_all_done_empty_dag` | Empty DAG = false |
| `test_all_tickets_done_event_conversion` | AllTicketsDone → UI entry |
| `test_rescheduling_after_completion` | Done ticket frees slot, dependent gets scheduled |
