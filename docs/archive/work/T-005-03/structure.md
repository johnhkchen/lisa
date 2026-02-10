# Structure: T-005-03 fix-phase-change-detection

## File Modified

`crates/lisa-plugin/src/lib.rs` — the only file touched.

## Change 1: `rebuild_dag()` — fix phase change detection

**Location**: Lines 164-176 (the phase comparison loop inside the `Ok(dag)` arm).

**Current**:
```rust
for ticket in dag.tickets() {
    if let Some(&old_phase) = self.last_phases.get(&ticket.id) {
        if old_phase != ticket.phase {
            self.log_activity(ActivityEvent::TicketPhaseChanged { ... });
            changed = true;
        }
    }
}
```

**New**:
```rust
for ticket in dag.tickets() {
    match self.last_phases.get(&ticket.id) {
        Some(&old_phase) => {
            if old_phase != ticket.phase {
                self.log_activity(ActivityEvent::TicketPhaseChanged {
                    ticket_id: ticket.id.clone(),
                    old_phase,
                    new_phase: ticket.phase,
                });
                changed = true;
            }
        }
        None => {
            // New ticket or first rebuild — treat non-Ready as a change
            // so downstream slot-release logic runs.
            if ticket.phase != Phase::Ready {
                changed = true;
            }
        }
    }
}
```

**Interface**: No public API changes. `rebuild_dag()` still returns `bool`.

## Change 2: `poll_tick()` — unconditional done-ticket detection and slot release

**Location**: Lines 493-530 (the `if changed { ... }` block).

**Current structure**:
```
let changed = self.rebuild_dag();
if changed {
    // done-ticket detection + slot release
    // thread phase sync
}
self.schedule_ready_tickets();
```

**New structure**:
```
let changed = self.rebuild_dag();

// UNCONDITIONAL: always check for done tickets and release slots
// (done-ticket detection code, moved out of if-block)

// UNCONDITIONAL: always sync thread phases
// (thread phase sync code, moved out of if-block)

if changed {
    // Retained only for future phase-change-specific logging if needed
    // (currently empty — the logging already happens in rebuild_dag)
}

self.sweep_stale_slots();

self.schedule_ready_tickets();
```

The `if changed` block becomes empty and can be removed entirely. The done-ticket detection and thread phase sync code are literally moved out, unchanged in logic.

## Change 3: New method `sweep_stale_slots()`

**Location**: New method on `impl State`, added after `schedule_ready_tickets()` (around line 298).

**Signature**:
```rust
fn sweep_stale_slots(&mut self)
```

**Behavior**:
1. Iterate `self.agent_slots`
2. For each slot with a `ticket_id`, check if that ticket is Done in `self.dag`
3. If Done, call `self.release_slot_for_ticket(&ticket_id)` and log a warning
4. No return value — purely side-effectful cleanup

**Called from**: `poll_tick()`, after thread phase sync and before `schedule_ready_tickets()`.

## Change 4: New tests

Three new test functions added to `mod tests` at the bottom of lib.rs.

### `test_done_ticket_detected_on_first_poll`
- State: `last_phases` empty, DAG with one ticket at `phase: done`, one running thread for it, one occupied slot
- Action: Run the done-ticket detection logic (extracted or via rebuild_dag + inline detection)
- Assert: Thread is completed, slot is released, activity log has ThreadExited

### `test_done_ticket_detected_between_polls`
- State: `last_phases` has ticket at `Phase::Research`, DAG rebuilt with ticket at `Phase::Done`, running thread, occupied slot
- Action: Run done-ticket detection
- Assert: Thread completed, slot released

### `test_sweep_stale_slots`
- State: One slot with `ticket_id = Some("T-001")`, DAG has T-001 at `phase: done`, no thread for T-001 (thread was already cleaned up)
- Action: Call `sweep_stale_slots()`
- Assert: Slot released (`ticket_id = None`), activity log has warning message containing "stale"

## Module Boundaries

No changes to:
- `lisa-core` (types, ticket, dag) — no API changes needed
- `scheduler.rs` — not used by the current `State` implementation for this logic
- `ui.rs` — rendering is unaffected

All changes are internal to `State` methods in `lib.rs`. No public interface changes.

## Ordering

1. Fix `rebuild_dag()` detection (independent)
2. Refactor `poll_tick()` to move logic out of `if changed` (depends on understanding the block)
3. Add `sweep_stale_slots()` (independent of 1-2)
4. Add tests (depends on 1-3)

Steps 1 and 3 can be done in any order. Step 2 is a pure code motion refactor. Step 4 validates all three.
