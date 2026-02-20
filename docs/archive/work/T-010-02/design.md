# T-010-02 Design: Event-driven Transition State Machine

## Decision 1: Where to Define TransitionState

### Option A: In `lisa-core/src/types.rs`

Place the enum alongside Phase, ThreadStatus, etc.

**Pros:** Shared types live together. Could be reused by CLI or tests.
**Cons:** TransitionState is plugin-internal scheduling state, not a core domain concept. The core crate has no zellij dependency and shouldn't model plugin-specific state machines.

### Option B: In `lisa-plugin/src/lib.rs` (Chosen)

Define `TransitionState` at the top of lib.rs, next to `AgentSlot`.

**Pros:** TransitionState is purely plugin scheduling logic — it governs how pane commands are sequenced. Keeps it colocated with AgentSlot which it extends. No cross-crate coupling.
**Cons:** None meaningful.

**Decision: Option B.** TransitionState is plugin-internal. Define it adjacent to AgentSlot in lib.rs.

## Decision 2: Signal Processing Architecture

### Option A: Extend check_idle_signals() to handle all signal types

Add `.stopped` and `.cleared` handling inside the existing `check_idle_signals()` method.

**Pros:** Single read_dir call. Less code.
**Cons:** Conflates three different concerns. The function name becomes misleading. `.idle` signals drive phase advancement; `.stopped`/`.cleared` drive transition sequencing — different responsibilities.

### Option B: Separate check_transition_signals() function (Chosen)

New `check_transition_signals()` reads signal_dir, filters for `.stopped` and `.cleared`, and handles each.

**Pros:** Clear separation of concerns. `check_idle_signals()` stays focused on phase advancement. `check_transition_signals()` handles transition sequencing. Each is independently testable.
**Cons:** Two read_dir calls per poll cycle on the same directory.

**Decision: Option B.** The cost of two read_dir calls is negligible. Clarity wins. Follow the ticket's proposed design.

## Decision 3: Timeout Mechanism

### Option A: Dedicated timeout timer

Arm a separate timer for each transition, similar to FLUSH_DELAY_SECS approach.

**Pros:** Precise timing.
**Cons:** Reintroduces timer complexity we're removing. Multiple concurrent timers create the same problems we have now.

### Option B: Check timeouts on each poll tick (Chosen)

Store `transition_started_at: Option<SystemTime>` on AgentSlot. On each 5s poll_tick, iterate slots and check elapsed time against threshold constants.

**Pros:** Piggybacks on existing 5s poll cycle. No new timers. Simple loop. Timeout resolution is 5s which is fine for 30-60s thresholds.
**Cons:** Up to 5s extra latency on timeout. Acceptable for fallback behavior.

**Decision: Option B.** Use poll-tick-based timeout checking. Timeout constants: 60s for WaitingForStop, 30s for WaitingForClear (matching ticket spec).

## Decision 4: What to Remove

The pending_pane_writes mechanism exists solely to delay prompt sends after /clear. With event-driven transitions, this is fully replaced.

Remove:
- `FLUSH_DELAY_SECS` constant (line 25)
- `pending_pane_writes` field (State, line 144)
- `flush_pending_pane_writes()` method (lines 178-182)
- The `flush_pending_pane_writes()` call in Timer handler (line 1511)
- The conditional timer arming in `schedule_ready_tickets()` (lines 393-396)
- The `pending_pane_writes` line in `dump_state_to_file()` (line 1019)

Keep:
- `arm_timer()` / `timer_fired()` / `pending_timer_count` — still used for poll cycle management

## Decision 5: Prompt Dispatch on .cleared

When `.cleared` is received for a slot in `WaitingForClear` state:
- Send the prompt immediately via `send_line_to_pane()`
- No queueing needed — the `.cleared` signal confirms the pane is ready

The prompt is built from the slot's `ticket_id` plus `self.config.ticket_dir`, same as today.

## Decision 6: State Machine Interaction with Idle Signals

The `.stopped` signal fires on every Claude response. The `.idle` signal fires when the agent actually goes idle (idle_prompt notification). These are independent events.

Scenario: A slot is in `WaitingForStop` state. An `.idle` signal also arrives for the same pane.
- The `.idle` signal is processed by `check_idle_signals()` which looks up the ticket and applies phase logic.
- But the slot's `ticket_id` has already been reassigned by `schedule_ready_tickets()` to the new ticket.
- So the `.idle` signal won't match any running thread — it will be ignored (correct behavior).

No interaction conflict. The two signal processing paths are independent.

## API Summary

### New Types (lib.rs)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TransitionState {
    #[default]
    Idle,
    WaitingForStop,
    WaitingForClear,
}
```

### Modified Structs

```rust
struct AgentSlot {
    pane_id: u32,
    ticket_id: Option<TicketId>,
    has_session: bool,
    transition_state: TransitionState,          // NEW
    transition_started_at: Option<SystemTime>,  // NEW
}
```

### New Constants

```rust
const STOP_SIGNAL_TIMEOUT_SECS: u64 = 60;
const CLEAR_SIGNAL_TIMEOUT_SECS: u64 = 30;
```

### New Methods

```rust
fn check_transition_signals(&mut self)    // Process .stopped and .cleared files
fn handle_stopped_signal(&mut self, pane_id: u32)
fn handle_cleared_signal(&mut self, pane_id: u32)
fn check_transition_timeouts(&mut self)   // Fallback on poll tick
```

### Modified Methods

```rust
fn schedule_ready_tickets(&mut self)  // WaitingForStop instead of immediate /clear
fn poll_tick(&mut self)               // Add check_transition_signals + check_transition_timeouts
fn discover_slots(&mut self, ...)     // Initialize new fields
fn dump_state_to_file(&self)          // Include transition_state
```

### Removed

```rust
const FLUSH_DELAY_SECS: f64 = 15.0;
fn flush_pending_pane_writes(&mut self)
// pending_pane_writes field
// flush call in Timer handler
// timer arming in schedule_ready_tickets
```
