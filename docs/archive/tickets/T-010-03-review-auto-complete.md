---
id: T-010-03
title: Auto-complete Review tickets on Stop signal
type: feature
phase: done
status: done
priority: medium
story: S-010
depends_on: [T-010-01]
created: 2026-02-11
---

# T-010-03: Auto-complete Review tickets on Stop signal

## Objective

When an agent stops in the Review phase (detected via `.stopped` signal), auto-mark the ticket as `phase: done` and release the slot. This eliminates the manual step of pressing `[d]` to mark review tickets done.

## Current Behavior

File: `crates/lisa-plugin/src/lib.rs:577-613`

When a ticket advances to Review phase (either via artifact or Implement→Review idle signal), the thread is **parked**:

```rust
if let Some(thread) = self.threads.get_mut(&ticket_id) {
    thread.current_phase = Phase::Review;
    thread.last_phase_change = std::time::SystemTime::now();
    thread.park();  // ← Thread is parked
}
```

**Current user flow:**
1. Agent finishes Implement phase
2. Plugin advances ticket to Review, parks thread
3. Slot remains occupied, agent pane is idle
4. **User must manually press `[d]` to mark ticket done**
5. Or user must prompt the agent to mark the ticket done

This is tedious and error-prone.

## New Behavior

When a `.stopped` signal arrives for a ticket in Review phase (and the slot is **not** in a transition state), auto-mark the ticket as done.

**Detection logic:**
- `.stopped` signal for a pane
- Ticket assigned to that pane is in `Phase::Review`
- Slot is in `TransitionState::Idle` (not mid-transition to a new ticket)

**Action:**
- Update ticket frontmatter: `phase: done` and `status: Done`
- Mark thread as Completed
- Release the slot
- Log the auto-completion event

**User override:** The `[d]` hotkey still works for manual control (if user wants to mark done before the agent stops, or reset a ticket).

## Implementation Tasks

### 1. Add Review auto-complete logic to signal processing

File: `crates/lisa-plugin/src/lib.rs`

In `handle_stopped_signal()` (from T-010-02), add logic to check for Review phase:

```rust
fn handle_stopped_signal(&mut self, pane_id: u32) {
    let slot = match self.agent_slots.iter_mut().find(|s| s.pane_id == pane_id) {
        Some(s) => s,
        None => return,
    };

    // Case 1: Slot is waiting for stop signal (mid-transition)
    if slot.transition_state == TransitionState::WaitingForStop {
        send_line_to_pane("/clear", PaneId::Terminal(pane_id));
        slot.transition_state = TransitionState::WaitingForClear;
        slot.transition_started_at = Some(SystemTime::now());

        self.log_activity(ActivityEvent::Info {
            message: format!("Pane {} ready, sent /clear", pane_id),
        });
        return;
    }

    // Case 2: Slot is idle and ticket is in Review phase (auto-complete)
    if slot.transition_state == TransitionState::Idle {
        if let Some(ticket_id) = &slot.ticket_id {
            if let Some(ticket) = self.dag.get_ticket(ticket_id) {
                if ticket.phase == Phase::Review {
                    self.auto_complete_review_ticket(ticket_id.clone(), pane_id);
                }
            }
        }
    }
}
```

### 2. Add `auto_complete_review_ticket()` function

File: `crates/lisa-plugin/src/lib.rs`

```rust
fn auto_complete_review_ticket(&mut self, ticket_id: TicketId, pane_id: u32) {
    let ticket_file = self.config.ticket_dir.join(format!("{}.md", ticket_id));

    // Update phase to Done
    if let Err(e) = ticket::update_ticket_phase(&ticket_file, Phase::Done) {
        self.log_activity(ActivityEvent::Error {
            message: format!("Failed to update {} phase to done: {}", ticket_id, e),
        });
        return;
    }

    // Update status to Done
    if let Err(e) = ticket::update_ticket_status(&ticket_file, lisa_core::types::TicketStatus::Done) {
        self.log_activity(ActivityEvent::Error {
            message: format!("Failed to update {} status to done: {}", ticket_id, e),
        });
        return;
    }

    // Mark thread as completed
    if let Some(thread) = self.threads.get_mut(&ticket_id) {
        thread.complete();
    }

    // Release slot
    self.release_slot_for_ticket(&ticket_id);

    // Log the auto-completion
    self.log_activity(ActivityEvent::PhaseChange {
        ticket_id: ticket_id.clone(),
        from: Phase::Review,
        to: Phase::Done,
    });

    self.log_activity(ActivityEvent::Info {
        message: format!("Auto-completed {} (Review → Done) for pane {}", ticket_id, pane_id),
    });

    // Remove thread tracking
    self.threads.remove(&ticket_id);
}
```

### 3. Prevent false positives (agent pausing mid-work)

The `Stop` hook fires on every turn completion, not just when work is done. To avoid false positives:

**Safety check:** Only auto-complete if:
- Ticket is in Review phase (not Research/Design/Structure/Plan/Implement)
- Slot is in `Idle` transition state (not mid-transition)
- Thread status is `Running` or `Parked` (not already `Completed`)

This is already handled by the logic above (checking `ticket.phase == Phase::Review` and `slot.transition_state == TransitionState::Idle`).

**Rationale:** Review is the final phase before Done. If the agent stops in Review, it's done with its work. Earlier phases may have legitimate pauses (waiting for user input, thinking between tool calls), but Review is special.

### 4. Update dashboard to show auto-complete events

File: `crates/lisa-plugin/src/ui.rs`

The activity log already displays `PhaseChange` and `Info` events, so auto-completions will appear automatically. No UI changes needed.

### 5. Tests

Add tests for:
- `.stopped` signal in Review phase triggers auto-complete
- Ticket frontmatter is updated (`phase: done`, `status: Done`)
- Thread is marked as Completed and removed
- Slot is released
- `.stopped` signal in other phases (Research, Implement, etc.) does NOT trigger auto-complete
- `.stopped` signal during transition (`WaitingForStop` state) does NOT trigger auto-complete

## Acceptance Criteria

- [ ] `.stopped` signal in Review phase auto-marks ticket as done
- [ ] Slot is released after auto-complete
- [ ] Thread is removed from tracking
- [ ] Activity log shows "Auto-completed {ticket_id} (Review → Done)"
- [ ] `.stopped` in non-Review phases does not auto-complete
- [ ] `.stopped` during transitions (WaitingForStop) does not auto-complete
- [ ] Manual `[d]` hotkey still works for override
- [ ] All tests pass

## Files Modified

- `crates/lisa-plugin/src/lib.rs`

## Notes

- Review phase is special because it's the final pre-Done phase. Agents in earlier phases may pause for legitimate reasons (AskUserQuestion, waiting for tool results, etc.), but Review indicates the agent has finished its work and is just waiting for human approval.
- The `[d]` hotkey remains functional for manual control, but most Review tickets should auto-complete without intervention.
- If the agent stops in Review but the ticket is NOT actually done, the user can press `[r]` to reset it back to Ready (existing functionality).
