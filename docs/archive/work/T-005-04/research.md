# Research: T-005-04 Thread Lifecycle Cleanup

## Problem Statement

Completed threads are never removed from `State::threads` in `poll_tick()`. This causes two classes of bugs:

1. **Unbounded growth**: Completed threads accumulate in the HashMap over the lifetime of a session.
2. **Stale Running threads block rescheduling**: If phase-change detection fails (T-005-03's artifact detection misses), a thread stays `Running` for a ticket that's actually done. `schedule_ready_tickets()` skips any ticket with a thread entry (line 271), regardless of thread status.

## Current Thread Lifecycle in lib.rs

### Creation (schedule_ready_tickets, line 308-312)
```rust
let mut thread = Thread::new(ticket_id.clone(), pane_id);
if let Some(ticket) = self.dag.get_ticket(&ticket_id) {
    thread.current_phase = ticket.phase;
}
self.threads.insert(ticket_id.clone(), thread);
```
Thread starts as `Running` with the ticket's current phase.

### Phase sync (poll_tick, line 591-600)
Running threads have their `current_phase` synced with DAG state each tick. This also resets `last_phase_change` when a change is detected (for stuck detection).

### Completion (poll_tick, line 566-588)
When a Running thread's ticket has `phase == Done` in the DAG:
```rust
thread.complete();                      // status → Completed
self.release_slot_for_ticket(ticket_id); // slot freed
self.log_activity(ThreadExited { ... });
```
**The thread stays in `self.threads`.** Never removed.

### Stale/Failed removal (detect_stale_threads, line 506-536)
When a Running thread exceeds the hard timeout (2x `stuck_threshold_secs`):
```rust
thread.fail();                          // status → Failed
self.release_slot_for_ticket(&ticket_id);
self.threads.remove(&ticket_id);        // ← REMOVED here
```
Failed threads ARE removed. This is the only place threads are removed during normal operation.

### mark_ticket_done (line 708-757)
Manual "mark done" from the keyboard modal:
```rust
thread.complete();                      // status → Completed
self.release_slot_for_ticket(&tid);
```
**Thread stays in `self.threads`.** Not removed.

## Scheduling Guard (schedule_ready_tickets, line 269-276)

```rust
for ticket_id in ready {
    if self.threads.contains_key(&ticket_id) {
        continue;  // ← skips regardless of thread status
    }
    ...
}
```

This is correct for Running threads (don't double-schedule), but wrong for Completed threads. A Completed thread for ticket X means X finished successfully. If X's ticket file somehow reverts to a non-done phase (manual edit, test scenario), X could never be rescheduled because the stale Completed thread blocks it.

## Health Tracking (evaluate_health, line 456-498)

```rust
// Clean up last_health for threads that no longer exist
self.last_health.retain(|tid, _| self.threads.contains_key(tid));
```

This cleanup already works — it removes `last_health` entries when threads are removed from `self.threads`. But since Completed threads are never removed, their `last_health` entries persist too (though they're benign since `health()` returns `Healthy` for non-Running threads).

## UI State (to_ui_state, line 871-981)

The `to_ui_state()` method filters threads by status:
- `active_threads`: only `Running` threads
- `parked_threads`: only `Parked` threads
- `alerts`: only `Running` or `Failed` threads

Completed threads are invisible in the UI but still consume memory.

## check_all_done (line 539-546)

```rust
fn check_all_done(&self) -> bool {
    !self.dag.is_empty()
        && self.dag.tickets().all(|t| t.phase == Phase::Done)
        && !self.threads.values().any(|t| t.status == ThreadStatus::Running)
}
```

Checks for `Running` threads specifically — Completed threads don't block termination. Correct behavior.

## sweep_stale_slots (line 340-368)

Releases slots assigned to done tickets, but doesn't touch `self.threads`. This is a slot-level safety net, not a thread-level one.

## Scheduler Module (scheduler.rs)

The `Scheduler` struct in scheduler.rs has its own `threads` HashMap and a `cleanup_completed_threads()` method (line 598-609) that removes Completed and Failed threads. **However, the `Scheduler` is not used by `State` in lib.rs.** `State` manages threads directly. The `Scheduler` is a standalone module with its own lifecycle management — it's used for the `open_command_pane_floating` path, not the current `write_chars_to_pane_id` path.

The `Scheduler::cleanup_completed_threads()` pattern is exactly what's needed in `State`:
```rust
pub fn cleanup_completed_threads(&mut self) {
    let completed_ids: Vec<TicketId> = self.threads
        .iter()
        .filter(|(_, t)| t.status == ThreadStatus::Completed || t.status == ThreadStatus::Failed)
        .map(|(id, _)| id.clone())
        .collect();
    for id in completed_ids {
        self.threads.remove(&id);
    }
}
```

## Thread Audit: Orphaned Thread Detection

The ticket requests a periodic audit in `poll_tick()` that warns if a thread's ticket_id doesn't match an active (non-done) ticket in the DAG. This would catch:
- Threads for tickets manually deleted from disk
- Threads for tickets whose frontmatter was corrupted
- Threads that somehow survived past their ticket's done transition

Currently no such audit exists. The closest is `sweep_stale_slots()` which checks slots, not threads.

## Summary of Gaps

| Behavior | Current | Needed |
|---|---|---|
| Remove Completed threads from `self.threads` | Never | After marking complete in `poll_tick()` |
| Remove Completed threads from `mark_ticket_done` | Never | After marking complete |
| Defensive guard in `schedule_ready_tickets()` | Skips all threads | Remove Completed threads and proceed |
| Thread audit for orphaned entries | None | Log warning if thread's ticket is done/missing |
| Clean up `last_health` on thread removal | Works (via retain) | Already correct |
| Clean up Failed threads | Works (in `detect_stale_threads`) | Already correct |

## Files to Modify

- `crates/lisa-plugin/src/lib.rs` — all changes are in this file
  - `poll_tick()`: remove Completed threads after marking them (line ~579-588)
  - `mark_ticket_done()`: remove Completed thread after marking (line ~749-752)
  - `schedule_ready_tickets()`: defensive guard for Completed threads (line ~271)
  - New: thread audit method, called from `poll_tick()`

## Existing Test Coverage

| Test | What it covers |
|---|---|
| `test_detect_stale_threads` | Failed thread removed, slot released |
| `test_stale_thread_not_stale_yet` | Running thread below threshold stays |
| `test_rescheduling_conditions_after_completion` | Preconditions for scheduling after completion (but doesn't test thread removal) |
| `test_done_ticket_detected_on_first_poll` | Thread marked Completed, slot released (but thread stays in HashMap) |
| `test_done_ticket_detected_between_polls` | Same as above with phase transition |
| `test_sweep_stale_slots_releases_done_ticket` | Stale slot released (no thread involved) |

## Test Gaps

- No test verifies that a Completed thread is removed from `self.threads`
- No test verifies that after removal, a dependent ticket gets scheduled (end-to-end)
- No test verifies the defensive guard in `schedule_ready_tickets()`
- No test for thread audit / orphaned thread detection
