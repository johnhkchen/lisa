# Design: T-018-02 session-timeout-enforcement

## Decision Summary

Add a `check_session_timeouts()` method to `State` that runs in `poll_tick()`. When a running thread's `started_at` exceeds `session_timeout_secs`, log a new `SessionTimedOut` activity event, mark the thread as `Failed` (reuse existing variant), release the slot, and remove the thread. Add `AlertType::TimedOut` to the UI for distinct dashboard display.

## Options Considered

### Option A: New `ThreadStatus::TimedOut` variant

Add `ThreadStatus::TimedOut` alongside Running/Parked/Completed/Failed.

- Pros: Semantically precise. Easy to distinguish timeouts from real failures in logs and UI.
- Cons: Breaks existing `match` exhaustiveness across the codebase. Many places check `ThreadStatus::Running` or `ThreadStatus::Failed` — each would need to handle `TimedOut`. `health()` and `is_attention_needed()` need updates. More invasive diff.

### Option B: Reuse `ThreadStatus::Failed` + dedicated `ActivityEvent`

Keep the 4 existing ThreadStatus variants. Use `Failed` for timed-out threads (they did fail — they exceeded the time limit). Add `ActivityEvent::SessionTimedOut` for rich logging. Add `AlertType::TimedOut` in the UI layer only.

- Pros: Minimal type-level changes. The thread is removed from `State.threads` immediately after marking failed, so the status value is transient. What matters is the activity log and the UI alert.
- Cons: If inspecting a thread's status at the exact moment of timeout, you can't distinguish timeout from crash. But since the thread is removed in the same function call, this window doesn't exist in practice.

### Option C: Don't mark failed at all, just remove

Skip `thread.fail()` and go straight to removing the thread and releasing the slot.

- Pros: Simplest code path.
- Cons: Breaks the pattern established by `detect_stale_threads()` which marks failed first. If any observer reads the thread between fail and remove, they'd see a consistent state.

## Decision: Option B

Reuse `ThreadStatus::Failed`. The thread is ephemeral — it's removed from the map immediately after being marked. The distinction between "timed out" and "crashed" lives in the activity log (`SessionTimedOut` vs `ThreadExited`) and the UI alert (`AlertType::TimedOut` vs `AlertType::Failed`). This avoids a disruptive enum variant addition that would touch many match arms.

## New Types

### `ActivityEvent::SessionTimedOut`

```rust
SessionTimedOut {
    ticket_id: TicketId,
    elapsed_secs: u64,
    phase: Phase,
}
```

Captures the ticket, how long it ran, and what phase it was in when timed out. This satisfies the acceptance criteria: "Log an ActivityEvent with timeout details: ticket ID, elapsed time, phase at timeout."

### `AlertType::TimedOut` (UI only)

```rust
AlertType::TimedOut
```

Renders distinctly from Failed and Stuck in the attention banner. Display format: `"⏱ TIMEOUT"` with detail like `"T-024-01 ran for 32m, timed out in implement"`.

## `check_session_timeouts()` Logic

```
if session_timeout_secs == 0: return (disabled)
for each running thread:
    elapsed = now - thread.started_at
    if elapsed >= session_timeout_secs:
        log SessionTimedOut event
        thread.fail()
        release_slot_for_ticket(ticket_id)
        remove thread from map
```

Placement in `poll_tick()`: after `evaluate_health()` and before `detect_stale_threads()`. Rationale: session timeout is a broader check than per-phase staleness. If a thread is timed out by session timeout, `detect_stale_threads()` shouldn't also try to handle it (and won't, since the thread is already removed).

## Dashboard Display

The acceptance criteria say: `"T-024-01 [TIMEOUT 32m]"`. This will be shown in the attention banner via `AlertType::TimedOut`. The thread won't appear in the active threads section (it's removed), but the alert persists in the activity log.

However, alerts are computed from current thread state in `to_ui_state()`. Since timed-out threads are removed, we need a different approach: store timeout alerts in a separate vec or include them directly as `HealthAlert`s added during `check_session_timeouts()`.

Better approach: Add `timeout_alerts: Vec<HealthAlert>` to `State` and populate it during `check_session_timeouts()`. Include these in `to_ui_state()` alongside the health-computed alerts. Clear stale timeout alerts after a reasonable display period or when the ticket is rescheduled.

Simpler approach: Since `activity_log` already captures `SessionTimedOut` events, the `to_ui_state()` method can scan recent activity for timeout events and generate alerts from them. This avoids new state.

Simplest approach: Just add to the existing `alerts` computation in `to_ui_state()`. But this requires threads to still exist...

**Final decision**: Add a `timeout_alerts: Vec<(TicketId, u64, Phase)>` field to State. Populated during `check_session_timeouts()`. Converted to `HealthAlert`s in `to_ui_state()`. Cleared when the ticket is rescheduled or after N polls. This gives persistent visibility without keeping stale threads around.

## Late Artifact Handling

When a timed-out session later produces artifacts:
- Thread is already removed, so `check_artifact_advances()` won't process it
- If the ticket is later rescheduled, `check_artifact_advances()` will find existing artifacts and advance through them immediately (this already works — tested in `test_check_artifact_advances_all_at_once`)
- No special handling needed for v1

## What Was Rejected

- **Option A** (new ThreadStatus::TimedOut): Too invasive for the benefit. The status is transient.
- **Option C** (remove without marking): Breaks the fail-then-remove pattern.
- **Automatic retry**: Ticket explicitly says not for v1.
- **Killing the process**: Ticket says "Do NOT kill the Claude Code process."
