# Plan: T-005-04 Thread Lifecycle Cleanup

## Step 1: Add `audit_threads()` method

Add a new method on `impl State` after `detect_stale_threads()`.

```rust
fn audit_threads(&mut self) {
    let orphaned: Vec<TicketId> = self.threads.keys()
        .filter(|tid| {
            self.dag.get_ticket(tid)
                .map(|t| t.phase == Phase::Done)
                .unwrap_or(true)
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

**Verify**: `cargo check -p lisa-plugin --target wasm32-wasip1`

## Step 2: Modify `poll_tick()` — remove completed threads + call audit

In the done-ticket detection loop (around line 579), add `self.threads.remove(ticket_id)` after `self.release_slot_for_ticket(ticket_id)`.

After `self.sweep_stale_slots()`, add `self.audit_threads()`.

**Verify**: `cargo check -p lisa-plugin --target wasm32-wasip1`

## Step 3: Modify `mark_ticket_done()` — remove thread

After `self.release_slot_for_ticket(&tid)` (around line 752), add `self.threads.remove(&tid)`.

**Verify**: `cargo check -p lisa-plugin --target wasm32-wasip1`

## Step 4: Modify `schedule_ready_tickets()` — defensive guard

Replace the existing `contains_key` check with:

```rust
if let Some(thread) = self.threads.get(&ticket_id) {
    if thread.status == lisa_core::types::ThreadStatus::Completed {
        self.threads.remove(&ticket_id);
        // Fall through to scheduling
    } else {
        self.log_activity(ActivityEvent::Info {
            message: format!("Skipping {}: thread already exists", ticket_id),
        });
        continue;
    }
}
```

Note: Need to extract the status before removing to avoid borrow issues:
```rust
let is_completed = self.threads.get(&ticket_id)
    .map(|t| t.status == lisa_core::types::ThreadStatus::Completed)
    .unwrap_or(false);
if self.threads.contains_key(&ticket_id) {
    if is_completed {
        self.threads.remove(&ticket_id);
    } else {
        self.log_activity(...);
        continue;
    }
}
```

**Verify**: `cargo check -p lisa-plugin --target wasm32-wasip1`

## Step 5: Fix existing tests

The existing test `test_done_ticket_detected_on_first_poll` (line 1956) asserts `state.threads.get("T-001").unwrap().status == ThreadStatus::Completed`. After our change, the thread is removed, so this assertion needs updating to `assert!(!state.threads.contains_key("T-001"))`.

Same for `test_done_ticket_detected_between_polls` (line 2014).

The test `test_rescheduling_conditions_after_completion` (line 1564) manually calls `thread.complete()` and checks preconditions — this test should also verify thread removal.

**Verify**: `cargo test --workspace`

## Step 6: Add new tests

### test_completed_thread_removed_from_hashmap
- Create a DAG with a done ticket and a Running thread for it
- Run the done-ticket detection logic
- Assert: `!state.threads.contains_key("T-001")`
- Assert: slot released

### test_completed_thread_dependent_gets_scheduled
- T-001 done with Running thread, T-002 ready depends on T-001
- Run done detection
- Assert: T-001 thread gone, T-002 ready and no thread blocking it

### test_defensive_guard_removes_completed_thread
- Create ticket with a Completed thread in self.threads
- Check that the defensive guard in schedule_ready_tickets logic removes it
- Since schedule_ready_tickets calls zellij APIs, test the guard logic directly

### test_audit_threads_removes_done_ticket_thread
- Thread for a ticket that's phase: done in the DAG
- Run audit_threads()
- Assert: thread removed, error logged

### test_audit_threads_removes_missing_ticket_thread
- Thread for a ticket ID that doesn't exist in the DAG
- Run audit_threads()
- Assert: thread removed, error logged

### test_audit_threads_keeps_active_thread
- Running thread for a ticket in research phase
- Run audit_threads()
- Assert: thread stays

**Verify**: `cargo test --workspace` — all tests pass

## Step 7: Final verification

- `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM build clean
- `cargo test --workspace` — all tests pass
- Verify no new warnings
