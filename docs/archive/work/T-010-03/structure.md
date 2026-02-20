# T-010-03 Structure: Auto-complete Review tickets on Stop signal

## Files modified

### `crates/lisa-plugin/src/lib.rs` (only file)

All changes are in this file.

## Changes

### 1. New method: `check_stopped_signals(&mut self)`

Location: After `check_idle_signals()` (after line 685), before `evaluate_health()`.

Pattern mirrors `check_idle_signals()`:
- Read `self.signal_dir` directory entries
- Match files named `pane-{id}.stopped`
- Delete signal file immediately
- Resolve pane_id → slot → ticket_id
- If ticket is in Review phase AND thread exists and is not Completed:
  - Call `auto_complete_review(ticket_id)`
- Otherwise: ignore (signal consumed, no action)

### 2. New method: `auto_complete_review(&mut self, ticket_id: TicketId)`

Location: After `check_stopped_signals()`.

Extracted helper to keep `check_stopped_signals()` focused on signal parsing. Follows the same pattern as `mark_ticket_done()` (lines 1268-1318):

1. Get ticket file path from DAG
2. `ticket::update_ticket_phase(&file_path, Phase::Done)`
3. `ticket::update_ticket_status(&file_path, TicketStatus::Done)`
4. Log `TicketPhaseChanged { from: Review, to: Done }`
5. Log `Info { message: "Auto-completed {ticket_id} (Review → Done)" }`
6. `thread.complete()` if thread exists
7. `self.release_slot_for_ticket(&ticket_id)`
8. `self.threads.remove(&ticket_id)`
9. `self.rebuild_dag()`
10. `self.schedule_ready_tickets()`

### 3. Add `check_stopped_signals()` call to `poll_tick()`

Location: `poll_tick()` (line 814), after `check_idle_signals()` call.

```
self.check_artifact_advances();
self.check_idle_signals();
self.check_stopped_signals();    // NEW
self.evaluate_health();
```

### 4. Tests (in `mod tests` at bottom of file)

New tests:
- `test_check_stopped_signals_review_auto_complete` — Full integration: create temp dir with signal file, ticket in Review, parked thread. Verify auto-complete.
- `test_check_stopped_signals_non_review_ignored` — Ticket in Implement phase. Signal consumed, no auto-complete.
- `test_check_stopped_signals_no_ticket_on_slot_ignored` — Slot exists but no ticket assigned. Signal consumed.
- `test_check_stopped_signals_completed_thread_ignored` — Thread already Completed. Signal consumed, no re-complete.

## Module boundaries

No new modules. No changes to `lisa-core`. No changes to `ui.rs`.

The only public interface change: none. `check_stopped_signals()` and `auto_complete_review()` are private methods on `State`.

## Ordering

1. Add `check_stopped_signals()` method
2. Add `auto_complete_review()` method
3. Wire into `poll_tick()`
4. Add tests
5. Verify all existing tests still pass
