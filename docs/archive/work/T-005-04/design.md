# Design: T-005-04 Thread Lifecycle Cleanup

## Approach: Immediate Removal After Completion

### Decision

Remove completed threads from `self.threads` immediately when they are marked as `Completed`, rather than accumulating them and cleaning up periodically. This is the simplest approach that addresses all the acceptance criteria.

### Rationale

**Option A: Immediate removal (chosen)**
- Remove thread from `self.threads` right after `thread.complete()` + `release_slot_for_ticket()`
- Simple, deterministic, no new data structures
- The `last_health` cleanup in `evaluate_health()` already handles orphaned entries via `retain()`
- Activity log already captures thread lifecycle events — no history is lost

**Option B: Move to `completed_threads: HashSet<TicketId>`**
- Preserves a record of which tickets completed
- Adds a new field to State
- The DAG already tracks `phase: done` — duplicating this information adds complexity
- The `Scheduler` struct in scheduler.rs uses this pattern, but it also uses a separate `parked` set — neither is needed when the DAG is the source of truth

**Option C: Periodic batch cleanup**
- Add a `cleanup_completed_threads()` method called in `poll_tick()`
- Lets completed threads linger for one poll cycle
- Unnecessarily complex for no benefit — a thread marked Completed has no useful work left

Option A wins because it's the simplest and most correct. The DAG is the single source of truth for ticket state. Threads are ephemeral tracking of active work. Once work ends, the thread should be gone.

### Rejected: completed_threads set for history

The ticket mentions "or move to a separate `completed_threads` set for history." This is unnecessary because:
1. The activity log already records `ThreadExited` events
2. The DAG tracks `phase: done` for all completed tickets
3. No code path reads completed thread history
4. Adding state that nothing reads violates YAGNI

## Changes

### 1. Remove completed threads in poll_tick()

After the existing done-ticket detection loop (line 579-588), add removal:

```rust
for ticket_id in &done_tickets {
    if let Some(thread) = self.threads.get_mut(ticket_id) {
        thread.complete();
    }
    self.release_slot_for_ticket(ticket_id);
    self.threads.remove(ticket_id);  // ← NEW
    self.log_activity(ActivityEvent::ThreadExited { ... });
}
```

### 2. Remove completed thread in mark_ticket_done()

After the existing completion block (line 749-752):

```rust
if let Some(thread) = self.threads.get_mut(&tid) {
    thread.complete();
}
self.release_slot_for_ticket(&tid);
self.threads.remove(&tid);  // ← NEW
```

### 3. Defensive guard in schedule_ready_tickets()

Add a secondary check: if a thread exists but is Completed, remove it and proceed with scheduling. This catches any edge case where thread removal was missed.

```rust
if self.threads.contains_key(&ticket_id) {
    if self.threads.get(&ticket_id)
        .map(|t| t.status == ThreadStatus::Completed)
        .unwrap_or(false)
    {
        self.threads.remove(&ticket_id);
        // Fall through to scheduling
    } else {
        continue;
    }
}
```

### 4. Thread audit in poll_tick()

Add a new method `audit_threads()` that checks for orphaned thread entries — threads whose ticket_id doesn't match an active (non-done) ticket in the DAG. This runs each poll cycle after DAG rebuild.

```rust
fn audit_threads(&mut self) {
    let orphaned: Vec<TicketId> = self.threads.keys()
        .filter(|tid| {
            self.dag.get_ticket(tid)
                .map(|t| t.phase == Phase::Done)
                .unwrap_or(true)  // missing from DAG = orphaned
        })
        .cloned()
        .collect();
    for tid in orphaned {
        self.log_activity(ActivityEvent::Error {
            message: format!("Orphaned thread for {} — removing", tid),
        });
        self.release_slot_for_ticket(&tid);
        self.threads.remove(&tid);
    }
}
```

This is the strongest safety net: any thread for a done or missing ticket gets cleaned up, with a log warning for visibility.

### 5. Verify last_health cleanup

The existing code in `evaluate_health()` line 497-498:
```rust
self.last_health.retain(|tid, _| self.threads.contains_key(tid));
```

This already handles cleanup when threads are removed. No change needed.

### 6. Verify detect_stale_threads()

The existing code at line 528:
```rust
self.threads.remove(&ticket_id);
```

Already removes failed threads. No change needed.

## Testing Strategy

### Test 1: Completed thread removed, dependent scheduled
- Create T-001 (done) and T-002 (ready, depends on T-001)
- Add a Running thread for T-001
- Run the done-ticket detection logic
- Assert T-001's thread is removed from `self.threads`
- Assert T-002 shows as ready and has no thread (can be scheduled)

### Test 2: Defensive guard in schedule_ready_tickets
- Create a ticket with a Completed thread in `self.threads`
- The ticket should still be schedulable despite having a stale thread entry
- Assert the Completed thread is removed and a new thread would be created

### Test 3: Thread audit detects orphaned entries
- Add threads for tickets that are either done or missing from the DAG
- Run `audit_threads()`
- Assert orphaned threads are removed and warnings logged

### Test 4: Stale Running thread for Done ticket cleaned up
- Add a Running thread for a ticket that's already `phase: done` in the DAG
- Run the done-ticket detection + audit
- Assert the thread is cleaned up

## Ordering

1. Add `audit_threads()` method
2. Modify `poll_tick()` to remove completed threads and call audit
3. Modify `mark_ticket_done()` to remove completed thread
4. Modify `schedule_ready_tickets()` with defensive guard
5. Add tests
