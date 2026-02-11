# T-010-02 Structure: Event-driven Transition State Machine

## File: `crates/lisa-plugin/src/lib.rs`

This is the only file modified. All changes are in this file.

### New Types (after line 83, before `State`)

```rust
/// Per-slot state machine for session transitions.
/// Gates /clear and prompt sends on hook-generated signal files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TransitionState {
    #[default]
    Idle,
    WaitingForStop,
    WaitingForClear,
}
```

### Modified: AgentSlot struct (lines 69-75)

Add two fields:

```rust
struct AgentSlot {
    pane_id: u32,
    ticket_id: Option<TicketId>,
    has_session: bool,
    transition_state: TransitionState,          // NEW
    transition_started_at: Option<SystemTime>,  // NEW
}
```

### New Constants (after POLL_INTERVAL_SECS, replacing FLUSH_DELAY_SECS)

```rust
const STOP_SIGNAL_TIMEOUT_SECS: u64 = 60;
const CLEAR_SIGNAL_TIMEOUT_SECS: u64 = 30;
```

Remove: `const FLUSH_DELAY_SECS: f64 = 15.0;` (line 25)

### Removed: pending_pane_writes Field (State struct, line 141-144)

Delete the field and its doc comment. Also delete `flush_pending_pane_writes()` method (lines 177-182).

### Modified: discover_slots() (line 258)

Add field initializers:

```rust
self.agent_slots.push(AgentSlot {
    pane_id: pane.id,
    ticket_id: None,
    has_session: false,
    transition_state: TransitionState::Idle,
    transition_started_at: None,
});
```

### Modified: schedule_ready_tickets() (lines 348-397)

Replace the `has_session` branch (lines 348-355):

**Before:** Send /clear immediately, queue prompt in pending_pane_writes.
**After:** Set `transition_state = WaitingForStop`, set `transition_started_at = Some(SystemTime::now())`. Do not send /clear or prompt.

Remove the conditional timer arming (lines 393-396).

### New: check_transition_signals() Method

Insert after `check_idle_signals()`. Same structure: read signal_dir, filter by extension, delete file, dispatch.

Handles two signal types:
- `pane-{id}.stopped` → calls `handle_stopped_signal(pane_id)`
- `pane-{id}.cleared` → calls `handle_cleared_signal(pane_id)`

### New: handle_stopped_signal(pane_id: u32) Method

Finds slot by pane_id. If `transition_state == WaitingForStop`:
1. Send `/clear` to pane
2. Set `transition_state = WaitingForClear`
3. Reset `transition_started_at = Some(SystemTime::now())`
4. Log Info event

Otherwise: no-op (signal from routine Stop events).

### New: handle_cleared_signal(pane_id: u32) Method

Finds slot by pane_id. If `transition_state == WaitingForClear`:
1. Build prompt from slot.ticket_id + config.ticket_dir
2. Send prompt to pane
3. Set `transition_state = Idle`
4. Clear `transition_started_at = None`
5. Log Info event

Otherwise: no-op.

### New: check_transition_timeouts() Method

Iterates `agent_slots`. For each slot with `transition_started_at`:
- `WaitingForStop` + elapsed > 60s → force send /clear, move to WaitingForClear, log Warning
- `WaitingForClear` + elapsed > 30s → force send prompt, move to Idle, log Warning

### Modified: poll_tick() (lines 814-908)

Add two calls after `check_idle_signals()`:

```rust
self.check_transition_signals();    // Process .stopped and .cleared
self.check_transition_timeouts();   // Fallback for missing signals
```

### Modified: Event::Timer handler (lines 1509-1518)

Remove `self.flush_pending_pane_writes();` call (line 1511).

### Modified: dump_state_to_file()

Replace `pending_pane_writes` line (1019) with transition state info per slot.

### Test Additions

New tests (following existing check_idle_signals pattern):

1. `test_transition_state_default` — TransitionState::Idle is default
2. `test_handle_stopped_signal_waiting` — WaitingForStop → sends /clear → WaitingForClear
3. `test_handle_stopped_signal_idle_noop` — Idle state ignores .stopped
4. `test_handle_cleared_signal_waiting` — WaitingForClear → sends prompt → Idle
5. `test_handle_cleared_signal_idle_noop` — Idle state ignores .cleared
6. `test_check_transition_signals_stopped` — File-based: .stopped file processed, deleted
7. `test_check_transition_signals_cleared` — File-based: .cleared file processed, deleted
8. `test_check_transition_timeouts_stop` — 60s timeout forces /clear
9. `test_check_transition_timeouts_clear` — 30s timeout forces prompt
10. `test_transition_signals_unknown_pane` — Unknown pane ID ignored

Note: Tests for handle_stopped_signal and handle_cleared_signal can't directly verify `send_line_to_pane()` calls (zellij host function), but can verify state transitions and log entries — same pattern as existing scheduling tests.

## Ordering of Changes

1. Add TransitionState enum + AgentSlot fields + constants
2. Update discover_slots() to initialize new fields
3. Modify schedule_ready_tickets() to set WaitingForStop instead of immediate /clear
4. Add check_transition_signals(), handle_stopped_signal(), handle_cleared_signal()
5. Add check_transition_timeouts()
6. Update poll_tick() to call new methods
7. Remove FLUSH_DELAY_SECS, pending_pane_writes, flush_pending_pane_writes(), flush call in Timer handler
8. Update dump_state_to_file()
9. Add tests
