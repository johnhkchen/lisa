# T-003-02 Design: Artifact-Phase Advance

## Decision: Scan-then-Update-then-Rebuild (Approach A)

### Why

Approach A scans artifacts, updates ticket files, then rebuilds the DAG — all within a single `poll_tick()`. This gives immediate phase advancement with no delay. Approach B (next-tick) adds a 5s delay for no benefit. Approach C (inside rebuild) mixes concerns.

### Rejected: Scan inside rebuild_dag

Mixing artifact scanning into DAG rebuild violates separation of concerns. `rebuild_dag()` reads ticket files and builds the graph. Artifact scanning is a separate concern that modifies ticket files before the DAG is rebuilt.

### Rejected: Next-tick approach

No reason to delay. The 5s poll interval is already the latency floor; adding another 5s is unnecessary.

## Design

### New Method: `State::check_artifact_advances()`

Called in `poll_tick()` **before** `rebuild_dag()`.

Logic:
1. Iterate over `self.threads` where `thread.status == Running`
2. For each running thread, get its `current_phase`
3. Call `current_phase.artifact_filename()` — if None (Ready/Review/Done), skip
4. Check if artifact exists at `self.config.work_dir / ticket_id / artifact_filename`
5. If artifact exists:
   a. Compute `next_phase = current_phase.next()` (guaranteed Some since we only check phases with artifacts, which are Research through Implement)
   b. Get the ticket from `self.dag` to find its `file_path`
   c. Call `ticket::update_ticket_phase(&file_path, next_phase)`
   d. Log `ActivityEvent::PhaseCompleted { ticket_id, phase: current_phase }`
   e. Log `ActivityEvent::TicketPhaseChanged { ticket_id, old_phase: current_phase, new_phase: next_phase }`
   f. Update `thread.current_phase = next_phase`
   g. If `next_phase == Phase::Review`, call `thread.park()` and log appropriately

### Multi-phase skip handling

If an agent writes multiple artifacts in one interval (e.g., both `research.md` and `design.md`), the scan loop should advance one phase per tick. On the next tick, it will detect the next artifact and advance again. This is simpler and produces clearer activity logs. No need to optimize for a rare case.

### Poll Tick Flow (updated)

```
poll_tick():
  1. check_artifact_advances()   // NEW: scan artifacts, update ticket files
  2. rebuild_dag()               // reads updated ticket files, detects phase changes
  3. (existing) mark Done tickets, update thread phases, free slots
  4. schedule_ready_tickets()
```

### Thread Parking

When `next_phase == Phase::Review`:
- Set `thread.status = ThreadStatus::Parked` via `thread.park()`
- The thread's agent pane remains open but the ticket is parked
- The dashboard will show it in the "Parked (Awaiting Review)" section

### Error Handling

- If `update_ticket_phase()` fails (I/O error), log an `ActivityEvent::Error` and skip that ticket
- If the work directory doesn't exist, `artifact_path.exists()` returns false, so the ticket is naturally skipped

### Test Strategy

- Unit test: create a tempdir with a ticket file and artifact, call the advance logic, verify phase updates
- Since `check_artifact_advances()` accesses `self.dag` and `self.threads`, tests need a populated `State`. This can be tested via the constituent parts: verify `Phase::artifact_filename()` mapping (already tested), verify `update_ticket_phase()` (already tested), and add a new integration test that simulates the full flow
- The actual `check_artifact_advances()` method will be testable on native target since it only uses `std::fs` (no zellij APIs)
