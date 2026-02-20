# T-010-03 Research: Auto-complete Review tickets on Stop signal

## What exists

### Signal infrastructure (from T-010-01, done)

T-010-01 added hook scaffolding for `Stop` and `SessionStart[clear]` events:
- `on-stop.sh` writes `.lisa/signals/pane-{id}.stopped` when Claude finishes responding
- `on-clear.sh` writes `.lisa/signals/pane-{id}.cleared` after `/clear` processes
- Both hooks are scaffolded by `lisa init` and wired into `settings.local.json`

### Signal processing (NOT yet implemented — T-010-02 is still `ready`)

T-010-02 defines the event-driven transition state machine with:
- `TransitionState` enum: `Idle`, `WaitingForStop`, `WaitingForClear`
- `check_transition_signals()` to read `.stopped` and `.cleared` signal files
- `handle_stopped_signal()` and `handle_cleared_signal()` methods
- Per-slot `transition_state` and `transition_started_at` fields on `AgentSlot`

**Key finding:** T-010-02 has NOT been implemented yet (phase: ready). T-010-03 depends on T-010-01 only, but the ticket's design references `handle_stopped_signal()` from T-010-02. This means T-010-03 either needs to:
1. Wait for T-010-02 to implement the transition state machine first, OR
2. Implement its own `.stopped` signal processing independently

Since T-010-03's `depends_on` only lists T-010-01, it can proceed. But the implementation must handle `.stopped` signals even without the T-010-02 transition state machine in place.

### Current signal processing (`lib.rs`)

The existing signal processing only handles `.idle` signals in `check_idle_signals()` (lines 526-685):
- Reads `.lisa/signals/pane-{id}.idle` files
- Deletes signal files immediately
- Resolves pane ID → ticket ID via `agent_slots`
- Advances phases based on current phase + artifact presence
- Parks threads when advancing to Review

There is **no** processing of `.stopped` or `.cleared` signals currently. No `TransitionState` enum exists. No `check_transition_signals()` function exists.

### AgentSlot structure (`lib.rs:69-75`)

```rust
struct AgentSlot {
    pane_id: u32,
    ticket_id: Option<TicketId>,
    has_session: bool,
}
```

No `transition_state` field — that's part of T-010-02's design.

### Review phase handling

When a ticket reaches Review:
1. `check_artifact_advances()` or `check_idle_signals()` updates the ticket frontmatter to `phase: review`
2. The thread is parked: `thread.park()` → `status = ThreadStatus::Parked`
3. The slot remains occupied (ticket_id is still set)

To mark a ticket done from Review, the user currently must:
- Press `[d]` to open the mark-done modal
- Select the ticket
- Press Enter to confirm

### `mark_ticket_done()` (`lib.rs:1268-1318`)

This is the manual completion path:
1. Updates ticket frontmatter: `phase: done`
2. Updates ticket status: `status: done`
3. Logs `TicketPhaseChanged` event
4. Marks thread as completed
5. Releases slot
6. Removes thread
7. Rebuilds DAG and schedules ready tickets

### Thread status when parked in Review

When parked:
- `thread.status == ThreadStatus::Parked`
- `thread.current_phase == Phase::Review`
- The agent pane is idle (Claude finished responding)

### The Stop hook fires on every turn

The `.stopped` signal fires every time Claude finishes a response turn, not just when all work is done. During active RDSPI phases (Research, Design, etc.), there will be many `.stopped` signals as the agent iterates. Only in Review phase does a `.stopped` signal mean "the agent is done."

### Ticket file update functions (`ticket.rs`)

Available:
- `update_ticket_phase(path, Phase)` — updates `phase:` in frontmatter
- `update_ticket_status(path, TicketStatus)` — updates `status:` in frontmatter

Both read the file, update the specific YAML field, and write back.

## Boundaries and constraints

1. **No TransitionState yet**: T-010-02 hasn't added the transition state machine. T-010-03 needs to process `.stopped` signals but cannot rely on `TransitionState::Idle` checks. For now, the simplest approach is: if a `.stopped` signal arrives and the slot's ticket is in Review phase, auto-complete.

2. **False positive risk is low for Review**: The Stop hook fires on every turn completion. But once a ticket is in Review phase, the thread is parked and the agent should not be actively working. A `.stopped` signal in Review means the agent's last response finished — which is exactly when we want to auto-complete.

3. **Race condition**: A `.stopped` signal might arrive from the agent's last Implement-phase response right as the plugin advances the ticket to Review. The signal file persists on disk, so the next poll cycle would pick it up. The check must verify the ticket is actually in Review phase at processing time, not at signal-write time.

4. **Existing `.idle` signal flow**: The idle signal flow already handles phase advancement and Review parking. The `.stopped` signal processing is a separate concern — it runs alongside idle signal processing.

5. **File path resolution**: Ticket file paths are stored in `ticket.file_path` via the DAG. The DAG is rebuilt each poll cycle from `scan_tickets()`. The file path includes the `/host/` prefix in WASI context.

6. **Thread cleanup pattern**: From `mark_ticket_done()`: complete thread → release slot → remove thread → rebuild DAG → schedule. The auto-complete path should follow the same pattern.

## Summary

The core task is straightforward: add `.stopped` signal processing to `poll_tick()` that auto-completes Review-phase tickets. The main complexity is that T-010-02's transition state machine doesn't exist yet, so we process `.stopped` signals with a simpler check: if the slot's ticket is in Review phase and the thread is Parked, auto-complete it. When T-010-02 lands later, `handle_stopped_signal()` will be extended to also handle transition states — the Review auto-complete becomes one case in that function.
