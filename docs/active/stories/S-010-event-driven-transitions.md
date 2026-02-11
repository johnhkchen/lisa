---
id: S-010
title: Event-driven session transitions
type: story
status: Active
created: 2026-02-11
---

# S-010: Event-driven session transitions

## Problem

Lisa currently uses a blind-fire timing approach for session transitions:
1. Detects idle signal (from `idle_prompt` notification hook)
2. Immediately sends `/clear` to the pane
3. Waits 15 seconds (hardcoded `FLUSH_DELAY_SECS`)
4. Sends the new prompt

**This fails because:**
- The `idle_prompt` notification fires BEFORE Claude Code finishes generating its final idle message
- `/clear` arrives while Claude is still streaming (the "✢ Garnishing…" phase)
- Since Claude is actively generating, `/clear` is treated as literal user input, not a command
- The new prompt arrives 15s later into an un-cleared context
- Result: bloated context, confused agent, wasted tokens

**Root causes:**
1. `idle_prompt` fires at the START of the idle prompt flow, not after rendering completes
2. `/clear` has zero delay (sent immediately)
3. `FLUSH_DELAY_SECS` (15s) is a fixed guess that can't match variable generation times
4. No handshake/readiness verification (Zellij can't read pane output)

## Solution

Replace blind-fire timing with an **event-driven handshake** using Claude Code's `Stop` and `SessionStart[clear]` hooks:

### Hook Events

| Hook Event | When it fires | Signal file | Purpose |
|-----------|--------------|-------------|---------|
| `Stop` | When Claude finishes responding | `.stopped` | Pane is at input prompt, ready for `/clear` |
| `SessionStart[clear]` | After `/clear` is processed | `.cleared` | Context is cleared, ready for new prompt |
| `Notification[idle_prompt]` | After 60s idle (legacy) | `.idle` | Fallback/phase detection |

### Transition State Machine

```
[Phase complete] → WaitingForStop
  ↓
[.stopped signal] → send /clear → WaitingForClear
  ↓
[.cleared signal] → send prompt → Idle
```

**Per-slot state tracking:**
- `Idle` — no transition pending
- `WaitingForStop` — phase complete, waiting for agent to finish generating
- `WaitingForClear` — `/clear` sent, waiting for context clear confirmation

**Timeouts:** If signals don't arrive within reasonable timeframes (60s for Stop, 30s for Clear), fall back to legacy behavior to prevent stalls.

### Review→Done Auto-transition

Currently, when an agent finishes the Implement phase and advances to Review, the thread is parked and waits indefinitely for the user to press `[d]` to mark it done.

**New behavior:** When a `.stopped` signal arrives for a ticket in Review phase (and the slot is not in a transition state), auto-mark the ticket as `phase: done` and release the slot. This eliminates the manual step.

**User override:** The `[d]` hotkey still works for manual control.

## Tickets

- **T-010-01:** Add Stop and SessionStart hook scaffolding (CLI)
- **T-010-02:** Event-driven transition state machine (plugin)
- **T-010-03:** Auto-complete Review tickets on Stop signal (plugin)

## Dependencies

```
T-010-01 (hook scaffolding)
  ├── T-010-02 (transition state machine)
  └── T-010-03 (review auto-complete)
```

## Success Criteria

1. No more `/clear` treated as literal text (verified by checking session transcripts)
2. No more hardcoded timing delays (`FLUSH_DELAY_SECS` removed)
3. Review tickets auto-complete without manual intervention
4. Graceful fallback if hooks fail (timeout-based legacy behavior)
5. All tests pass

## References

- Claude Code hooks documentation: https://code.claude.com/docs/en/hooks
- GitHub issue #12048: Feature request for `WaitingForInput` notification
- Root cause analysis: conversation thread 2026-02-11
