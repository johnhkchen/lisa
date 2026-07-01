---
id: T-022-02
story: S-022
title: error-signal-consumer
type: task
status: open
priority: high
phase: done
depends_on: [T-022-01]
---

## Context

The normalized signal contract is `.heartbeat` / `.stopped` / `.error` +
usage/cost — but the scheduler has **no `.error` consumer today**: failures
surface only indirectly via silence (2× stuck-threshold reclaim,
`detect_stale_threads` / `check_session_timeouts`). The Codex wrapper
(T-023-01) will emit `pane-<id>.error` on `turn.failed` / non-zero exit, and
the scheduler must react promptly instead of waiting ~40 minutes for the
silence clock.

## Acceptance Criteria

- The plugin consumes `pane-<id>.error` in the poll tick (same read-and-delete
  pattern as the other signals; consider ordering relative to
  `check_transition_signals` — error handling should precede transition
  timeouts so a failed pane isn't force-advanced).
- On `.error` for a running thread: the thread is failed, the slot released
  (existing `release_slot_for_ticket` path), and an alert surfaced in the UI —
  mirroring what `check_session_timeouts` does on reclaim (`lib.rs:1558-1571`)
  but immediately.
- `.error` for an idle/unknown pane is consumed harmlessly (logged, no state
  change).
- The normalized contract (which signals exist, who emits them, what the
  scheduler does) is documented in one place — extend the signal table in
  `data/hooks-guide.md` or a contract doc the adapters reference.
- Native tests cover: error on running thread → failed + released; error on
  idle pane → no-op; error file deleted after consumption.

## Notes

- Claude sessions have no `.error` emitter today — that's fine; the consumer
  is adapter-agnostic and simply never fires for Claude panes.
- Body content of signal files is currently ignored by the plugin; keep
  `.error` consistent (presence is the signal), but the wrapper may write the
  error message as body for human debugging.
