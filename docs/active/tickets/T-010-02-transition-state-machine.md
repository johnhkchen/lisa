---
id: T-010-02
title: Event-driven transition state machine
type: feature
phase: done
status: done
priority: high
story: S-010
depends_on: [T-010-01]
created: 2026-02-11
---

# T-010-02: Event-driven transition state machine

## Objective

Replace the blind-fire `/clear` + fixed delay system with an event-driven handshake state machine. Use `.stopped` and `.cleared` signals to gate each step of the session transition.

## Current Behavior (Broken)

File: `crates/lisa-plugin/src/lib.rs:348-397`

```rust
if self.agent_slots[slot_idx].has_session {
    send_line_to_pane("/clear", PaneId::Terminal(pane_id));  // IMMEDIATE
    let prompt = ticket_prompt(&host_ticket_dir, &ticket_id);
    self.pending_pane_writes.push((pane_id, prompt));        // DEFERRED 15s
}

// Later (15 seconds)...
if !self.pending_pane_writes.is_empty() {
    self.arm_timer(FLUSH_DELAY_SECS);  // 15 second blind guess
}
```

**Problem:** `/clear` is sent immediately when the idle signal is detected, but Claude is still streaming its final idle message. `/clear` arrives during generation and gets treated as literal text.

## New Behavior (Event-driven)

### State Machine

Add per-slot transition state tracking:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransitionState {
    Idle,              // No transition pending
    WaitingForStop,    // Phase complete, waiting for .stopped signal
    WaitingForClear,   // /clear sent, waiting for .cleared signal
}

struct AgentSlot {
    pane_id: u32,
    ticket_id: Option<TicketId>,
    has_session: bool,
    transition_state: TransitionState,  // NEW
    transition_started_at: Option<SystemTime>,  // NEW (for timeouts)
}
```

### Transition Flow

1. **Phase completion detected** (artifact + idle signal, as-is)
   - Assign ticket to slot (as-is)
   - Set `slot.transition_state = WaitingForStop`
   - Set `slot.transition_started_at = Some(SystemTime::now())`
   - **Do NOT send `/clear` yet**

2. **`.stopped` signal detected** (new)
   - Check if slot is in `WaitingForStop` state
   - If yes:
     - Send `/clear` to pane
     - Set `slot.transition_state = WaitingForClear`
     - Reset `slot.transition_started_at = Some(SystemTime::now())`
   - Delete signal file

3. **`.cleared` signal detected** (new)
   - Check if slot is in `WaitingForClear` state
   - If yes:
     - Send the new prompt to pane (immediately, no queue)
     - Set `slot.transition_state = Idle`
     - Clear `slot.transition_started_at = None`
   - Delete signal file

### Timeout Fallbacks

To prevent stalls if hooks fail:

```rust
const STOP_SIGNAL_TIMEOUT_SECS: u64 = 60;   // If no .stopped after 60s, send /clear anyway
const CLEAR_SIGNAL_TIMEOUT_SECS: u64 = 30;  // If no .cleared after 30s, send prompt anyway
```

On each poll tick, check for timed-out transitions:

```rust
fn check_transition_timeouts(&mut self) {
    let now = SystemTime::now();
    for slot in &mut self.agent_slots {
        if let Some(started) = slot.transition_started_at {
            let elapsed = now.duration_since(started).unwrap_or_default().as_secs();

            match slot.transition_state {
                TransitionState::WaitingForStop if elapsed > STOP_SIGNAL_TIMEOUT_SECS => {
                    // Fallback: send /clear anyway
                    self.log_activity(ActivityEvent::Warning {
                        message: format!("Stop signal timeout for pane {}, sending /clear anyway", slot.pane_id),
                    });
                    send_line_to_pane("/clear", PaneId::Terminal(slot.pane_id));
                    slot.transition_state = TransitionState::WaitingForClear;
                    slot.transition_started_at = Some(now);
                }
                TransitionState::WaitingForClear if elapsed > CLEAR_SIGNAL_TIMEOUT_SECS => {
                    // Fallback: send prompt anyway
                    self.log_activity(ActivityEvent::Warning {
                        message: format!("Clear signal timeout for pane {}, sending prompt anyway", slot.pane_id),
                    });
                    if let Some(ticket_id) = &slot.ticket_id {
                        let prompt = ticket_prompt(&self.config.ticket_dir, ticket_id);
                        send_line_to_pane(&prompt, PaneId::Terminal(slot.pane_id));
                    }
                    slot.transition_state = TransitionState::Idle;
                    slot.transition_started_at = None;
                }
                _ => {}
            }
        }
    }
}
```

## Implementation Tasks

### 1. Add `TransitionState` enum to types

File: `crates/lisa-core/src/types.rs` or `crates/lisa-plugin/src/lib.rs`

Define the enum and add fields to `AgentSlot` struct (lines 72-76 in lib.rs):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransitionState {
    Idle,
    WaitingForStop,
    WaitingForClear,
}

struct AgentSlot {
    pane_id: u32,
    ticket_id: Option<TicketId>,
    has_session: bool,
    transition_state: TransitionState,
    transition_started_at: Option<SystemTime>,
}
```

### 2. Modify `schedule_ready_tickets()` to defer `/clear`

File: `crates/lisa-plugin/src/lib.rs:348-355`

**Before:**
```rust
if self.agent_slots[slot_idx].has_session {
    send_line_to_pane("/clear", PaneId::Terminal(pane_id));  // ← REMOVE
    let prompt = ticket_prompt(&host_ticket_dir, &ticket_id);
    self.pending_pane_writes.push((pane_id, prompt));        // ← REMOVE
}
```

**After:**
```rust
if self.agent_slots[slot_idx].has_session {
    // Set state to WaitingForStop; actual /clear happens when .stopped signal arrives
    self.agent_slots[slot_idx].transition_state = TransitionState::WaitingForStop;
    self.agent_slots[slot_idx].transition_started_at = Some(SystemTime::now());
}
```

### 3. Add signal processing for `.stopped` and `.cleared`

File: `crates/lisa-plugin/src/lib.rs`

Add new function `check_transition_signals()` (similar pattern to `check_idle_signals()`):

```rust
fn check_transition_signals(&mut self) {
    let entries = match std::fs::read_dir(&self.signal_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry_result in entries {
        let entry = match entry_result { Ok(e) => e, Err(_) => continue };
        let path = entry.path();
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        // Delete signal file immediately
        let _ = std::fs::remove_file(&path);

        // Parse pane-{id}.stopped or pane-{id}.cleared
        if let Some(rest) = filename.strip_prefix("pane-") {
            if let Some(id_str) = rest.strip_suffix(".stopped") {
                let pane_id: u32 = match id_str.parse() { Ok(p) => p, Err(_) => continue };
                self.handle_stopped_signal(pane_id);
            } else if let Some(id_str) = rest.strip_suffix(".cleared") {
                let pane_id: u32 = match id_str.parse() { Ok(p) => p, Err(_) => continue };
                self.handle_cleared_signal(pane_id);
            }
        }
    }
}

fn handle_stopped_signal(&mut self, pane_id: u32) {
    let slot = match self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
        Some(s) => s,
        None => return,
    };

    if slot.transition_state == TransitionState::WaitingForStop {
        send_line_to_pane("/clear", PaneId::Terminal(pane_id));
        slot.transition_state = TransitionState::WaitingForClear;
        slot.transition_started_at = Some(SystemTime::now());

        self.log_activity(ActivityEvent::Info {
            message: format!("Pane {} ready, sent /clear", pane_id),
        });
    }
}

fn handle_cleared_signal(&mut self, pane_id: u32) {
    let slot = match self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
        Some(s) => s,
        None => return,
    };

    if slot.transition_state == TransitionState::WaitingForClear {
        if let Some(ticket_id) = &slot.ticket_id {
            let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);
            let prompt = ticket_prompt(&host_ticket_dir, ticket_id);
            send_line_to_pane(&prompt, PaneId::Terminal(pane_id));

            self.log_activity(ActivityEvent::Info {
                message: format!("Context cleared for pane {}, sent prompt for {}", pane_id, ticket_id),
            });
        }

        slot.transition_state = TransitionState::Idle;
        slot.transition_started_at = None;
    }
}
```

### 4. Add timeout checking to `poll_tick()`

File: `crates/lisa-plugin/src/lib.rs:814`

Add call to `check_transition_timeouts()` in the poll cycle:

```rust
fn poll_tick(&mut self) {
    self.check_artifact_advances();
    self.check_transition_signals();     // NEW: process .stopped and .cleared
    self.check_idle_signals();
    self.check_transition_timeouts();    // NEW: fallback for missing signals

    // ... rest of poll_tick
}
```

### 5. Remove `FLUSH_DELAY_SECS` and `pending_pane_writes`

File: `crates/lisa-plugin/src/lib.rs`

- Remove `const FLUSH_DELAY_SECS: f64 = 15.0;` (line 25)
- Remove `pending_pane_writes: Vec<(u32, String)>` from `State` struct (line 141-144)
- Remove `flush_pending_pane_writes()` function (lines 177-182)
- Remove call to `flush_pending_pane_writes()` from `update()` Timer event (line 1510)
- Remove conditional timer arming (lines 392-397)

### 6. Initialize new fields in `AgentSlot` construction

File: `crates/lisa-plugin/src/lib.rs:250-283`

In `discover_slots()`:

```rust
slots.push(AgentSlot {
    pane_id: pane.pane_id,
    ticket_id: None,
    has_session: false,
    transition_state: TransitionState::Idle,         // NEW
    transition_started_at: None,                     // NEW
});
```

### 7. Tests

Add tests for:
- State machine transitions (Idle → WaitingForStop → WaitingForClear → Idle)
- `.stopped` signal processing
- `.cleared` signal processing
- Timeout fallbacks
- Signal files are deleted after processing

## Acceptance Criteria

- [ ] No more immediate `/clear` send in `schedule_ready_tickets()`
- [ ] `TransitionState` tracks per-slot transition progress
- [ ] `.stopped` signal triggers `/clear` only when in `WaitingForStop` state
- [ ] `.cleared` signal triggers prompt only when in `WaitingForClear` state
- [ ] Timeout fallbacks prevent indefinite stalls (60s for Stop, 30s for Clear)
- [ ] `FLUSH_DELAY_SECS` constant removed
- [ ] `pending_pane_writes` mechanism removed
- [ ] All tests pass

## Files Modified

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-core/src/types.rs` (if `TransitionState` goes there)

## Notes

- The `Stop` hook fires on every turn completion, so `.stopped` signals will be frequent. Only act on them when `transition_state == WaitingForStop`.
- Timeout fallbacks preserve the legacy blind-fire behavior if hooks fail, preventing total breakage.
- Signal files are deleted immediately after reading (same pattern as `.idle` signals).
