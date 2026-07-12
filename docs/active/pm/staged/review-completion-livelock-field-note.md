---
title: review-completion-livelock
status: chained
agent: codex
observed_at: 2026-07-12
evidence_repo: /Users/johnchen/swe/repos/arcade/games/midsummer
evidence_ticket: T-009-01-01
materialized_epics: [E-041, E-042]
chain_after: T-040-03-04
---

# Review completion livelock — Arcade field note

## Observed sequence

1. The Codex attempt wrote the private Review artifact and reported its turn complete.
2. Lisa advanced the ticket only as far as `phase: review`; it did not create a pending or
   successful completion transaction.
3. At 16:21:39 PDT Lisa sent the Review-timeout prompt asking the agent to finish
   `.lisa/attempts/T-009-01-01/1/work/review.md`.
4. The agent found the file already present and complete, revalidated it, added an explicit
   critical-issues section, and stopped again at 16:21:55 PDT.
5. The ticket remained in Review. The operator's dashboard `[d]one` path did not recover it.
6. Lisa later relaunched the same ticket. The agent directly ran `lisa complete-ticket` from the
   Arcade repository root, producing completion commit `f64df75` at 16:34:25 PDT.

The direct CLI completion proves the ticket and published artifacts were committable. It does not
prove the scheduler's normal artifact, stopped-signal, timeout, or manual-done paths worked.

## What is known

- The artifact existed and was non-empty when the timeout prompt arrived.
- Lisa itself considered the ticket to be in Review.
- The same attempt identity (`T-009-01-01`, attempt 1) was retained across the incident.
- A direct `lisa complete-ticket` succeeded and marked both `status` and `phase` done.
- The fallback `[d]one` control was not an effective recovery path.

## What is not yet proven

The retained evidence does not identify which guard rejected or skipped `request_completion`.
Candidate boundaries include thread status filtering, slot transition state, lease authority,
artifact admission, or loss of a pending completion result. The fix ticket must instrument and
test the actual rejection rather than assuming that another timeout or prompt will heal it.

## Required behavior

- Once a current attempt's Review is admitted, polling is level-triggered: every safe subsequent
  poll can observe the unmet completion obligation until it is pending, done, or visibly blocked.
- Stop, idle, timeout, and pane relaunch ordering cannot make that obligation disappear.
- `[d]one` routes through the same completion transaction and either succeeds or displays a named
  reason; it never fails silently.
- Retries are idempotent: exactly one completion commit and one authoritative Done outcome.
- A future structured `block` disposition remains blocked; reliability must not bypass E-040's
  explicit pass gate.

## Regression shape

Create a current leased Codex thread whose private `review.md` exists at the Implement-to-Review
edge. Exercise polling, a stopped signal arriving in an inconvenient slot transition state, the
Review timeout/relaunch path, and `[d]one`. Before the fix this must reproduce the stranded Review;
after the fix it must converge on one pending completion and one successful terminal transaction,
or a durable operator-visible rejection when completion is intentionally made to fail.
