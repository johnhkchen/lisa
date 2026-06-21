---
id: T-018-02
story: S-018
title: session-timeout-enforcement
type: task
status: open
priority: high
phase: done
depends_on: [T-018-01]
---

## Context

Make the plugin enforce the configured session timeout. When an agent session exceeds the timeout, lisa should reclaim the slot gracefully and log the event.

## Acceptance Criteria

- Plugin tracks session start time for each active thread (wall-clock, not CPU)
- During `poll_tick`, check each active session against `session_timeout_secs`
- When a session exceeds the timeout:
  - Log an `ActivityEvent` with timeout details: ticket ID, elapsed time, phase at timeout
  - Mark the thread as timed out (new state or reuse existing error state)
  - Free the scheduling slot so other tickets can be picked up
  - Do NOT kill the Claude Code process — it may still be doing useful work. Just stop waiting for it.
- Dashboard UI shows timed-out sessions distinctly (e.g., "T-024-01 [TIMEOUT 32m]")
- If a timed-out session later produces artifacts (idle signal, phase transition), lisa should recognize and handle it gracefully (either ignore or re-acquire)

## Implementation notes

- Session start time should be captured when the zellij pane is spawned (in `schedule_ready_tickets`)
- The timeout check runs in the existing `poll_tick` loop — no new timers needed
- Consider: should a timed-out ticket be retried automatically? Probably not in v1 — just free the slot and let the operator decide
