---
id: T-031-01
story: S-031
title: isolated-commit-transaction
type: bug
status: open
priority: critical
phase: done
agent: codex
depends_on: [T-029-02]
---

## Context

Lisa serializes commits with a lock, but a shared repository index is still a
cross-process mailbox. During the vend loop, the Codex seat left 14 files staged
between its own operations and an unrelated `git commit` swept them into the
wrong commit. Locking commit calls alone does not protect files staged before
the lock or prevent a ticket commit from consuming entries staged by somebody
else.

Introduce one provider-neutral commit transaction that lisa can use for ticket
completion. The transaction must serialize the full stage-and-commit operation,
isolate ticket-owned staging from the normal repository index, and provide a
clear success/failure result for the scheduler. The RDSPI cycle should decide
the implementation mechanism after mapping the existing plugin permissions,
host command execution, and git constraints.

## Acceptance Criteria

- The transaction's critical section spans all preparation, staging, commit,
  verification, and cleanup required for one ticket completion.
- Ticket-owned paths are never visible as staged entries in the repository's
  ordinary index, including while the transaction is in progress.
- Entries already staged in the ordinary index remain byte-for-byte equivalent
  before and after the transaction and cannot enter the ticket commit.
- The transaction can include modified and untracked ticket code, the ticket's
  `docs/active/work/<ticket-id>/` artifacts, and its ticket frontmatter without
  broad `git add -A` behavior.
- A process-level regression test creates a foreign staged entry, completes a
  ticket transaction, and proves via the resulting tree and index that neither
  side captured or altered the other's content.
- Commit failure, lock failure, and cleanup failure return actionable errors;
  none is reported as success, and the ordinary index remains usable.
- The mechanism is provider-neutral and does not depend on Claude- or
  Codex-specific hooks.
- Focused tests and the relevant workspace test suites pass.

## Notes

- This ticket supplies the transaction boundary. T-031-02 integrates it with
  scheduler state transitions.
- T-029-02 is an explicit dependency because both tickets may change plugin
  timer/scheduler internals on the active board.
