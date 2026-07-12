---
id: T-031-02
story: S-031
title: gate-done-on-commit
type: bug
status: open
priority: critical
phase: done
agent: codex
depends_on: [T-031-01]
---

## Context

Automatic completion currently writes `phase: done` and `status: done` before
the transition is durably committed. The scheduler can then release the seat,
unblock dependents, emit provenance, and terminate the loop even though the
done transition exists only in the working tree. The E-068 vend run ended with
five Codex tickets in exactly that state.

Route every completion path through the isolated transaction from T-031-01.
The final code and work artifacts plus the status/phase transition must be
committed before in-memory state publishes Done. This applies to artifact/idle
auto-advance, review auto-completion, timeout finish-up, manual mark-done, and
any other path that can transition a ticket to Done.

## Acceptance Criteria

- There is one completion state machine used by every path that can move a
  ticket from a non-done phase to Done; no call site writes Done independently.
- The final commit contains the ticket's outstanding code changes, all six
  ticket work artifacts, and its `phase: done` / `status: done` frontmatter.
- In-memory phase, thread completion, provenance outcome, slot release,
  dependent scheduling, all-done notification, and loop termination occur only
  after the commit is verified successful.
- If the transaction fails, the ticket remains non-done on disk and in memory,
  its dependents remain blocked, and the seat remains recoverable with an
  actionable activity error.
- A successful transition cannot leave the done frontmatter or any other
  loop-owned ticket changes uncommitted.
- Regression tests cover automatic review completion, review timeout/finish-up,
  manual mark-done, a reused Codex seat, and a dependent ticket waiting at the
  boundary.
- Existing provenance emits exactly once for the eventual successful outcome,
  not for failed attempts.
- Focused plugin tests, the workspace test suite, the WASM release build, and
  plugin Clippy pass.

## Notes

- Treat the order as an invariant: prepare final ticket content → commit the
  isolated transaction → publish Done → move on.
- Do not retain a compatibility path that publishes Done and asks an agent to
  commit afterward.
