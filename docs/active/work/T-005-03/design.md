# Design: T-005-03 fix-phase-change-detection

## Problem Summary

Two bugs interact to stall the scheduler:
1. `rebuild_dag()` only detects phase changes for tickets already in `last_phases`
2. `poll_tick()` gates slot release behind `if changed`, so when detection fails, slots are never freed

## Option A: Fix detection + move slot release unconditionally

**Approach**: Fix `rebuild_dag()` to detect new/first-seen tickets as changed, AND move the done-ticket-detection + slot-release logic outside `if changed` in `poll_tick()`.

**Pros**:
- Addresses both bugs directly
- The "find done tickets → release slots" logic is semantically correct to run every tick — it's cheap (iterates threads + DAG lookup) and idempotent
- `if changed` block can remain for phase-sync logging only

**Cons**:
- Slightly more work per tick (done-ticket scan runs every 5s regardless)

**Assessment**: The per-tick cost is negligible (iterates `self.threads`, which is bounded by `max_threads`). This is the correct fix.

## Option B: Fix detection only, keep gating

**Approach**: Fix `rebuild_dag()` detection so `changed` is always correct, keep `if changed` gating.

**Pros**:
- Minimal code change
- No extra per-tick work

**Cons**:
- Still fragile: if `rebuild_dag()` ever misses a change for any reason, slots get stuck again
- No defense in depth

**Assessment**: Rejected. The gating is the root cause of the severity — even if detection is perfect, gating critical slot-release logic behind an optimization flag is architecturally wrong.

## Option C: Replace `if changed` with periodic unconditional sweep

**Approach**: Remove `if changed` entirely, always run all logic.

**Pros**:
- Simplest possible fix
- No way for detection bugs to cause stalls

**Cons**:
- Loses the phase-sync logging context (we still want to know WHEN phases changed)
- Slightly less efficient (phase sync logging runs every tick)

**Assessment**: Rejected. We want to keep `changed` for logging, just not for gating critical operations.

## Decision: Option A + Safety Sweep

**Chosen approach**: Option A with an additional safety sweep (`sweep_stale_slots()`).

### Fix 1: `rebuild_dag()` detection

In the phase comparison loop, treat missing entries as changes when the ticket is in a non-default state:

```rust
for ticket in dag.tickets() {
    match self.last_phases.get(&ticket.id) {
        Some(&old_phase) if old_phase != ticket.phase => {
            // Existing: phase changed
            changed = true;
            // log...
        }
        None if ticket.phase != Phase::Ready => {
            // New: ticket appeared in non-default phase
            changed = true;
            // log as new ticket...
        }
        _ => {} // No change
    }
}
```

The `!= Phase::Ready` guard avoids triggering `changed` on first load when all tickets are in the default state, which would be noisy. On first load with done tickets, those will be detected.

### Fix 2: `poll_tick()` unconditional slot release

Move the "find done tickets → complete thread → release slot" block outside `if changed`:

```rust
let changed = self.rebuild_dag();

// Always check for done tickets — unconditionally release slots
let done_tickets: Vec<TicketId> = self.threads.iter()
    .filter(|(_, t)| t.status == ThreadStatus::Running)
    .filter(|(tid, _)| self.dag.get_ticket(tid).map(|t| t.phase == Phase::Done).unwrap_or(false))
    .map(|(tid, _)| tid.clone())
    .collect();

for ticket_id in &done_tickets {
    if let Some(thread) = self.threads.get_mut(ticket_id) {
        thread.complete();
    }
    self.release_slot_for_ticket(ticket_id);
    self.log_activity(ActivityEvent::ThreadExited { ... });
}

// Also always sync thread phases
for (tid, thread) in &mut self.threads {
    if thread.status == ThreadStatus::Running {
        if let Some(ticket) = self.dag.get_ticket(tid) {
            if thread.current_phase != ticket.phase {
                thread.current_phase = ticket.phase;
                thread.last_phase_change = std::time::SystemTime::now();
            }
        }
    }
}

if changed {
    // Keep for phase-change logging only (optional)
}
```

### Fix 3: Safety sweep `sweep_stale_slots()`

After scheduling, verify no slot holds a ticket that is Done:

```rust
fn sweep_stale_slots(&mut self) {
    let stale: Vec<(u32, TicketId)> = self.agent_slots.iter()
        .filter_map(|slot| {
            let tid = slot.ticket_id.as_ref()?;
            let is_done = self.dag.get_ticket(tid)
                .map(|t| t.phase == Phase::Done)
                .unwrap_or(false);
            if is_done { Some((slot.pane_id, tid.clone())) } else { None }
        })
        .collect();

    for (pane_id, ticket_id) in stale {
        self.release_slot_for_ticket(&ticket_id);
        self.log_activity(ActivityEvent::Error {
            message: format!("Slot #{} held stale ticket {}, releasing", pane_id, ticket_id),
        });
    }
}
```

Called at the end of `poll_tick()`, after `schedule_ready_tickets()`.

### Test Strategy

Three new tests, all buildable with the existing test infrastructure:
1. **First poll with done tickets**: Build state with `last_phases` empty, tickets at Done, threads running → call the done-detection logic → verify slots released
2. **Ticket transitions to done between polls**: Build state with `last_phases` showing a ticket at Research, update ticket to Done in DAG → verify detected and released
3. **Safety sweep catches stale slot**: Build state with a slot assigned to a Done ticket but no thread → verify sweep releases it

All tests avoid zellij host functions (no `schedule_ready_tickets()`).
