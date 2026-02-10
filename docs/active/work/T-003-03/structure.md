# Structure: T-003-03 completion-reschedule

## Files Modified

### 1. `crates/lisa-core/src/types.rs`

**Add `AllTicketsDone` variant to `ActivityEvent`**

```rust
// In ActivityEvent enum, add:
/// All tickets have reached done phase
AllTicketsDone,
```

No other changes to types.rs. The Thread, Phase, and ThreadStatus types already have everything needed.

### 2. `crates/lisa-plugin/src/lib.rs`

**Add tick counter to State struct**

```rust
pub struct State {
    // ... existing fields ...

    /// Tick counter (incremented each poll_tick call, ~5s per tick).
    tick_count: u64,

    /// Tick at which each thread last had a phase change.
    /// Used for staleness detection.
    last_activity_tick: HashMap<TicketId, u64>,

    /// Whether the loop has terminated (all tickets done).
    terminated: bool,
}
```

**Modify `check_artifact_advances()`**

Skip the implement phase. Implement → done is handled by the agent writing frontmatter directly, not by artifact detection.

```rust
fn check_artifact_advances(&mut self) {
    // ... existing collection of running threads ...

    for (ticket_id, current_phase) in running {
        // Skip implement phase - progress.md is a living document,
        // not a completion signal. The agent sets phase: done directly.
        if current_phase == Phase::Implement {
            continue;
        }

        // ... rest of existing logic unchanged ...
    }
}
```

**Add `detect_stale_threads()` method**

```rust
/// Default staleness threshold in ticks (~5s each).
/// 360 ticks = ~30 minutes.
const STALE_THRESHOLD_TICKS: u64 = 360;

fn detect_stale_threads(&mut self) {
    let stale: Vec<TicketId> = self.threads.iter()
        .filter(|(_, t)| t.status == ThreadStatus::Running)
        .filter(|(tid, _)| {
            let last_tick = self.last_activity_tick.get(*tid).copied().unwrap_or(0);
            self.tick_count - last_tick > STALE_THRESHOLD_TICKS
        })
        .map(|(tid, _)| tid.clone())
        .collect();

    for ticket_id in stale {
        if let Some(thread) = self.threads.get_mut(&ticket_id) {
            thread.fail();
        }
        self.release_slot_for_ticket(&ticket_id);
        self.threads.remove(&ticket_id);
        self.log_activity(ActivityEvent::Error {
            message: format!("{} stale — no phase change for {} ticks, marked failed",
                ticket_id, STALE_THRESHOLD_TICKS),
        });
    }
}
```

**Add `check_all_done()` method**

```rust
fn check_all_done(&self) -> bool {
    !self.dag.is_empty()
        && self.dag.tickets().all(|t| t.phase == Phase::Done)
        && self.threads.values().all(|t| t.status != ThreadStatus::Running)
}
```

**Modify `poll_tick()`**

Add staleness check, termination check, and update tick counter.

```rust
fn poll_tick(&mut self) {
    self.tick_count += 1;

    // Check for new artifacts and advance phases
    self.check_artifact_advances();

    // Detect stale threads (failure handling)
    self.detect_stale_threads();

    let changed = self.rebuild_dag();

    if changed {
        // Update last_activity_tick for threads whose phase changed
        for (tid, thread) in &self.threads {
            if thread.status == ThreadStatus::Running {
                if let Some(ticket) = self.dag.get_ticket(tid) {
                    if thread.current_phase != ticket.phase {
                        self.last_activity_tick.insert(tid.clone(), self.tick_count);
                    }
                }
            }
        }

        // Mark done tickets' threads as complete
        // (existing logic, unchanged)
        let done_tickets: Vec<TicketId> = ...;
        for ticket_id in &done_tickets {
            // ... existing completion logic ...
        }

        // Update thread phases
        // (existing logic, unchanged)
    }

    // Schedule newly ready tickets
    self.schedule_ready_tickets();

    // Check for clean termination
    if self.check_all_done() {
        self.log_activity(ActivityEvent::AllTicketsDone);
        self.terminated = true;
        // Don't re-arm the timer
        return;
    }

    // Re-arm the timer
    set_timeout(POLL_INTERVAL_SECS);
}
```

**Modify `schedule_ready_tickets()`**

Record `last_activity_tick` when spawning new threads.

```rust
fn schedule_ready_tickets(&mut self) {
    // ... existing logic ...

    // After creating thread record:
    self.last_activity_tick.insert(ticket_id.clone(), self.tick_count);

    // ... rest unchanged ...
}
```

**Modify `render()` to show termination state**

```rust
fn render(&mut self, rows: usize, cols: usize) {
    if self.terminated {
        println!("All tickets done. Lisa loop complete.");
        return;
    }
    // ... existing render logic ...
}
```

**Update `activity_event_to_ui_entry()` to handle new variant**

```rust
ActivityEvent::AllTicketsDone => {
    return Some(ui::ActivityEntry {
        timestamp,
        activity: ui::ActivityType::PhaseCompleted {
            ticket_id: String::new(),
            phase: ui::Phase::Done,
        },
    });
}
```

## Module Boundaries

- **types.rs** (lisa-core): Only adds the `AllTicketsDone` variant. No interface changes.
- **lib.rs** (lisa-plugin): All behavioral changes. New methods are private to State. No public API changes.
- **scheduler.rs**: Untouched. Dead code for the plugin; not in scope.
- **ui.rs**: Untouched. The existing UI types and rendering handle all needed states.
- **dag.rs**: Untouched. `get_ready_tickets()` and `tickets()` already provide what's needed.
- **ticket.rs**: Untouched. `update_ticket_phase()` already exists.

## Ordering

1. Add `AllTicketsDone` to types.rs (no dependencies)
2. Add new State fields (tick_count, last_activity_tick, terminated)
3. Modify `check_artifact_advances()` to skip implement
4. Add `detect_stale_threads()`
5. Add `check_all_done()`
6. Modify `poll_tick()` to wire everything together
7. Modify `schedule_ready_tickets()` to record activity ticks
8. Update `activity_event_to_ui_entry()` for new variant
9. Update `render()` for termination
10. Add tests
