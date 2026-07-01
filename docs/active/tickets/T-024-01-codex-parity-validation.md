---
id: T-024-01
story: S-024
title: codex-parity-validation
type: task
status: open
priority: medium
phase: implement
depends_on: [T-023-02]
---

## Context

End-to-end validation that a Codex loop gets the same treatment a Claude loop
gets: correct transitions, liveness, failure handling, and review completion —
and that the scheduler/UI never misreads Codex's differing semantics
(`.stopped` once per run; no `.idle`/`.awaiting`; heartbeats from `item.*`
events). This is S-024's "full parity is the bar" made checkable, in the
spirit of the T-020-05 gate-harness precedent.

## Acceptance Criteria

- A scripted/reproducible validation run: a small multi-ticket DAG executed on
  the Codex adapter, verifying:
  - phases advance on artifacts through all RDSPI phases;
  - `.stopped` at run end triggers Review auto-completion with dependencies
    respected;
  - a long tool-free stretch does not false-trip stuck detection while the
    wrapper is emitting heartbeats — and a genuinely hung run IS reclaimed;
  - a forced failure (`turn.failed`/non-zero exit) fails the thread promptly
    via `.error` and releases the slot;
  - the review-timeout finish-up path works via `agent-exec --resume`;
  - the dashboard shows sane states throughout (no phantom "awaiting").
- Mixed loop sanity: one Claude pane + one Codex pane in the same loop, both
  completing, signals correctly attributed per pane.
- Findings written up; any contract violations become bugs blocking S-025's
  "documented toggle" claim.

## Notes

- Where live-codex steps can't run in CI, encode them as a documented manual
  checklist plus recorded-stream native tests (same split T-023-01 uses).
