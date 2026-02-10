# Structure: T-005-04 Thread Lifecycle Cleanup

## File Changes

Only one file is modified: `crates/lisa-plugin/src/lib.rs`

No new files, no new crates, no new dependencies.

## Modifications to lib.rs

### New Method: `audit_threads(&mut self)`

**Location**: Add as a method on `impl State`, after `detect_stale_threads()` (around line 536).

**Purpose**: Safety net that removes any thread whose ticket is done or missing from the DAG. Logs a warning for each orphaned thread found.

**Signature**:
```rust
fn audit_threads(&mut self)
```

**Behavior**:
- Collect ticket IDs of threads where `dag.get_ticket(tid)` returns `phase == Done` or `None`
- For each: release slot, remove from `self.threads`, log `ActivityEvent::Error` warning
- Skip threads with `status == Running` that have a non-done ticket (these are legitimate active work)

### Modified: `poll_tick(&mut self)`

**Location**: Line 579-588 (done-ticket detection loop)

**Change**: After `self.release_slot_for_ticket(ticket_id)`, add `self.threads.remove(ticket_id)`. The `thread.complete()` call can be dropped since the thread is about to be removed — but keep it for the activity log event ordering (complete before exit).

Actually, we need to restructure slightly. Currently:
```rust
for ticket_id in &done_tickets {
    if let Some(thread) = self.threads.get_mut(ticket_id) {
        thread.complete();
    }
    self.release_slot_for_ticket(ticket_id);
    self.log_activity(ActivityEvent::ThreadExited { ... });
}
```

Change to:
```rust
for ticket_id in &done_tickets {
    if let Some(thread) = self.threads.get_mut(ticket_id) {
        thread.complete();
    }
    self.release_slot_for_ticket(ticket_id);
    self.threads.remove(ticket_id);
    self.log_activity(ActivityEvent::ThreadExited { ... });
}
```

**Location**: After `self.sweep_stale_slots()` (line 603), add `self.audit_threads()`.

The call order in poll_tick becomes:
1. `check_artifact_advances()` — advance phases from artifacts
2. `evaluate_health()` — log health transitions
3. `detect_stale_threads()` — remove threads past hard timeout
4. `rebuild_dag()` — rescan ticket files
5. Done-ticket detection loop — mark complete, release slot, **remove thread**
6. Phase sync loop — sync running thread phases with DAG
7. `sweep_stale_slots()` — release orphaned slots
8. **`audit_threads()`** — remove orphaned threads (new)
9. `schedule_ready_tickets()` — schedule newly-ready work
10. Poll summary log
11. `check_all_done()` — termination check

### Modified: `mark_ticket_done(&mut self, ticket_id: &str)`

**Location**: Line 749-752

**Change**: Add `self.threads.remove(&tid)` after `self.release_slot_for_ticket(&tid)`.

```rust
if let Some(thread) = self.threads.get_mut(&tid) {
    thread.complete();
}
self.release_slot_for_ticket(&tid);
self.threads.remove(&tid);
```

### Modified: `schedule_ready_tickets(&mut self)`

**Location**: Line 269-276

**Change**: Replace the simple `contains_key` guard with a check that also handles Completed threads defensively.

```rust
// Skip tickets that already have an active thread
if let Some(thread) = self.threads.get(&ticket_id) {
    if thread.status == lisa_core::types::ThreadStatus::Completed {
        // Stale completed thread — remove and proceed with scheduling
        self.threads.remove(&ticket_id);
    } else {
        self.log_activity(ActivityEvent::Info {
            message: format!("Skipping {}: thread already exists", ticket_id),
        });
        continue;
    }
}
```

Note: The `self.threads.get()` borrow must end before `self.threads.remove()`. Use a let-binding to extract the status first.

## Module Boundaries

All changes are internal to the `State` struct in `lib.rs`. No changes to:
- `scheduler.rs` — separate module with its own lifecycle (not used by State)
- `ui.rs` — already filters by thread status
- `lisa-core` types — Thread, ThreadStatus, etc. are unchanged
- `lisa-cli` — no changes

## Test Structure

All new tests go in the existing `#[cfg(test)] mod tests` block at the bottom of lib.rs.

### `test_completed_thread_removed_after_done_detection`
- Setup: DAG with done ticket, Running thread
- Execute: done-ticket detection logic (same pattern as existing tests)
- Assert: thread removed from HashMap, slot released

### `test_completed_thread_dependent_scheduled`
- Setup: T-001 done with Running thread, T-002 ready depending on T-001, idle slot
- Execute: done detection + verify scheduling preconditions
- Assert: T-001 thread removed, T-002 has no thread, T-002 is ready

### `test_defensive_guard_removes_completed_thread`
- Setup: ticket with Completed thread in self.threads
- Execute: the scheduling guard logic
- Assert: Completed thread removed

### `test_audit_threads_removes_orphaned`
- Setup: thread for ticket that's done in DAG, thread for ticket not in DAG
- Execute: `audit_threads()`
- Assert: both threads removed, warnings logged

### `test_audit_threads_keeps_active`
- Setup: Running thread for active (non-done) ticket
- Execute: `audit_threads()`
- Assert: thread stays

### `test_mark_done_removes_thread`
- Setup: ticket with Running thread, call mark_ticket_done
- Assert: thread removed from self.threads
