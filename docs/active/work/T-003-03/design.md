# Design: T-003-03 completion-reschedule

## Decision Summary

Implement completion detection, failure handling, rescheduling, and clean termination within the existing timer-based polling architecture. No architectural changes to the terminal pane model.

## Approach Evaluation

### Option A: Add `CommandPaneExited` via Command Panes

Switch from terminal pane text injection to `open_command_pane()`. Pane exit fires a real event with exit code.

**Rejected.** Breaks session reuse (T-003-01), requires layout architecture change, and `open_command_pane` creates floating panes not stacked panes. Too much churn for this ticket.

### Option B: PaneUpdate Title Heuristic

Monitor `PaneInfo.title` in PaneUpdate events. When claude exits, the terminal title reverts from something like "claude" to "zsh"/"bash".

**Rejected.** Fragile, terminal-dependent, not all shells update window title, and terminal multiplexers can interfere. No exit code available.

### Option C: Enhanced Polling (Chosen)

Keep the 5-second poll timer. Add: (1) implement phase completion via agent-written frontmatter, (2) failure detection via staleness timeout, (3) rescheduling after state changes, (4) clean termination.

**Chosen.** Builds on existing architecture, reliable, testable, no new Zellij API dependencies.

### Option D: RunCommandResult Wrapper

Use `run_command()` to periodically check if the terminal pane's foreground process is still running.

**Rejected.** Complex, race-prone, and `run_command` output parsing is fragile.

## Detailed Design

### 1. Fix Implement Phase Artifact Detection

**Problem**: `check_artifact_advances()` advances implement → review when `progress.md` appears. But progress.md is a living document created early in the implement phase — it's not a completion signal like research.md or design.md are for their respective phases.

**Fix**: Skip the implement phase in `check_artifact_advances()`. For implement, the agent completes its work and updates the ticket frontmatter to `phase: done` directly. The plugin detects this via `rebuild_dag()` (which already compares `last_phases` snapshots). This approach is:
- Consistent with how agents are instructed (RDSPI workflow says agents update frontmatter)
- Doesn't rely on a file that exists before the phase is complete
- Avoids premature review-parking of active threads

**Alternative considered**: Use a separate completion marker file (e.g., `.implement-done`). Rejected as unnecessary — the agent already writes frontmatter.

### 2. Thread Completion on Ticket Done (AC 1)

When `rebuild_dag()` detects a ticket moved to `phase: done`:

```
thread.status = Running, ticket.phase = Done
  → thread.complete()
  → release_slot_for_ticket(ticket_id)
  → log ThreadExited { exit_code: Some(0) }
```

This is already implemented in `poll_tick()` lines 336-358. No change needed here, just ensure it's robust after the implement artifact fix.

### 3. Failure Detection (AC 2)

Add a staleness-based failure detector. A thread is "stale" when:
- It has been running for longer than `stale_timeout` (configurable, default 30 minutes)
- Its ticket phase has not changed since the last check

On stale detection:
- Mark thread as Failed: `thread.fail()`
- Release the slot: `release_slot_for_ticket(ticket_id)`
- Log error: `ActivityEvent::Error { message: "Thread stale..." }`
- Leave ticket phase unchanged (enables retry on next schedule)
- Remove the thread from `self.threads` so the ticket becomes eligible for rescheduling

**Implementation**: Add a `last_phase_change: HashMap<TicketId, Instant>` to State. Updated when a thread is spawned or its phase changes. Checked in `poll_tick()`.

**Note**: In WASM (wasm32-wasip1), `std::time::Instant` is not available. Use the timer tick count instead — each tick is ~5 seconds. Store the tick count at last phase change and compare against current tick count.

### 4. Rescheduling After State Changes (AC 3)

Already handled: `poll_tick()` calls `schedule_ready_tickets()` on every tick. When a thread completes or fails, its slot is freed, and on the next tick (or same tick), newly-ready tickets are scheduled into freed slots.

No changes needed. The 5-second latency is acceptable.

### 5. Clean Termination (AC 4)

After `rebuild_dag()` in `poll_tick()`, check if all tickets are done:

```rust
let all_done = self.dag.tickets().all(|t| t.phase == Phase::Done);
if all_done && self.threads.values().all(|t| t.status != ThreadStatus::Running) {
    self.log_activity(ActivityEvent::AllTicketsDone);
    // Don't re-arm the timer → loop stops
    return;
}
```

Add `ActivityEvent::AllTicketsDone` variant to log the clean termination.

The dashboard should render a completion message when all tickets are done.

### 6. Retry on Failure

When a thread fails (stale timeout or future exit detection):
- Ticket phase is unchanged
- Thread is removed from `self.threads`
- Slot is released
- On next `schedule_ready_tickets()`, the ticket is still "ready" (or in its current phase, which is startable)
- The scheduler picks it up and assigns it to a free slot
- A new Claude session starts from the ticket's current phase (per RDSPI, artifacts are insurance)

This gives automatic retry without special retry logic.

## Changes By File

| File | Change |
|------|--------|
| `crates/lisa-plugin/src/lib.rs` | Skip implement in `check_artifact_advances()`; add stale thread detection in `poll_tick()`; add termination check; add tick counter to State |
| `crates/lisa-core/src/types.rs` | Add `ActivityEvent::AllTicketsDone` variant |

## What We're NOT Doing

- Not changing the terminal pane / agent slot architecture
- Not adding `CommandPaneExited` event handling (not applicable to terminal panes)
- Not adding `RunCommandResult` wrapper scripts
- Not changing Phase::Implement.artifact_filename() (keep it for reference/documentation)
- Not touching scheduler.rs (it's dead code for the plugin; cleanup is a separate ticket)

## Test Strategy

1. Test stale thread detection: create a thread, advance tick counter past threshold, verify it's marked failed
2. Test clean termination: all tickets done, verify timer is not re-armed
3. Test retry flow: failed thread removed, ticket re-eligible for scheduling
4. Test implement phase skipped in artifact advances: progress.md present but no auto-advance
5. Test rescheduling after completion: done ticket frees slot, dependent ticket is scheduled
