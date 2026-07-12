---
id: S-031
title: atomic-ticket-completion
type: story
status: open
priority: critical
tickets: [T-031-01, T-031-02, T-031-03]
---

# S-031: Atomic ticket completion

## Outcome

For every agent provider, `phase: done` means that completion is durably
committed. The final ticket code, RDSPI work artifacts, status/phase transition,
and any completion metadata form one serialized git transaction. Lisa does not
release the seat, schedule a dependent, or report all tickets done until that
transaction succeeds.

The transaction must coexist with unrelated tools using the same repository.
Ticket work must not remain staged in the shared index, and pre-existing staged
entries belonging to a human or another process must remain untouched and must
not enter the ticket commit.

## Field regression

On the E-068 vend loop, the Codex seat completed five tickets but left all five
`phase: done` transitions uncommitted. During the same run, an unrelated
`git commit` swept 14 loop-staged files into the wrong commit. Presweep detected
the residue after the fact; this story makes both states unrepresentable during
normal operation.

## Tickets

- **T-031-01 — Isolated commit transaction:** provide a serialized commit
  mechanism that never leaks through or consumes the shared index.
- **T-031-02 — Gate completion on commit success:** route every done transition
  through the transaction before releasing scheduler state.
- **T-031-03 — Provider contract and live regression:** align Claude/Codex
  instructions with ticket-level atomicity and prove the invariant end to end.

## Done when

A mixed-provider loop can complete while a foreign staged entry is present:
each ticket has one final completion commit containing its code, work artifacts,
and frontmatter transition; the foreign entry remains staged but uncommitted;
the ordinary working tree has no loop-owned residue; and dependents start only
after their prerequisite completion commits exist.
