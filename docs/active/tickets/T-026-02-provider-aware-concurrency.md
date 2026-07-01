---
id: T-026-02
story: S-026
title: provider-aware-concurrency
type: task
status: open
priority: medium
phase: ready
depends_on: [T-026-01]
---

## Context

Mixing providers must not silently break: providers have separate auth and
rate-limit pools, and today there is only one global `max_threads`
(`types.rs:481`; layout pre-creates 2× panes, `loop_cmd.rs:199`). The explicit
stress target from the epic is **~16 concurrent mixed-provider agents** —
find what breaks (rate limits, pane/slot limits, commit serialization,
signal-file contention) and establish the realistic ceiling.

## Acceptance Criteria

- Concurrency limits are provider-aware: at minimum an optional per-provider
  cap alongside the global `max_threads`, enforced at spawn-time slot
  assignment; sensible defaults keep single-provider loops unchanged.
- A stress validation at high concurrency (target 16 mixed agents, or the
  documented ceiling if lower): scheduler stays correct (no cross-pane signal
  misattribution, no commit-serialization deadlock, no slot leaks), and
  failures degrade to visible alerts, not silent stalls.
- Findings written up: what actually broke, at what N, per provider — feeding
  the provenance "concurrency-at-run" interpretation (T-027-01) and the epic's
  open question 8.
- Tests for the per-provider cap logic (native, no live agents).

## Notes

- Signal-dir churn at high N is the plugin's own surface — measure poll-tick
  cost with ~32 panes' worth of signal files before assuming it's fine.
