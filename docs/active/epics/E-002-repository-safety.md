---
id: E-002
title: repository-safety
status: open
priority: critical
stories: [S-030, S-031]
---

# E-002: Repository Safety — non-destructive upgrades and atomic completion

## Intent

Make `lisa init` upgrades and `lisa loop` completion safe in a shared,
actively developed repository. Lisa must not silently replace user-owned
content, expose an ignored secret, publish completion without committing it,
or let one ticket's staged files leak into another process's commit.

This epic comes from a 0.3.0 → 0.4.0-rc.5 field run in the vend repository on
2026-07-11. The Codex seat completed all five assigned tickets, but the run
also exposed two repository-integrity gaps:

- the upgrade replaced committed additions in
  `docs/knowledge/rdspi-workflow.md` and removed `hooks/ntfy-topic` from
  `.lisa/.gitignore`, making a `0600` notification secret visible to git;
- ticket completion and git publication were separate operations, leaving five
  Codex tickets marked `phase: done` only in the working tree, while staged
  loop artifacts were swept into an unrelated commit during the run.

Both failures were caught before damage. The goal here is prevention, not a
better post-hoc warning.

## Safety invariants

1. An upgrade never overwrites an existing file whose content has diverged
   from the version lisa previously installed.
2. Lisa-managed ignore files are monotonic: upgrades may add required rules
   but never remove an existing rule.
3. Every mutating init run reports exactly which files it created or changed
   and which candidate updates it skipped for safety.
4. A ticket becomes schedulably `done` only after the commit containing its
   final code, work artifacts, and done transition succeeds.
5. Lisa never exposes ticket staging in the repository's shared index between
   operations and never includes another process's staged entries in a ticket
   commit.
6. A failed completion transaction leaves the ticket non-done, retains its
   slot for recovery, and does not unblock dependents.

## Scope

- Ownership-aware behavior for all files created or updated by `lisa init`.
- Append-only `.lisa/.gitignore` upgrades, including preservation of secret
  paths and project-specific rules.
- Human-auditable mutation summaries for real and dry-run initialization.
- A serialized, isolated git transaction for ticket completion.
- Integration of that transaction into every automatic completion path and
  both Claude and Codex seat contracts.
- Regression coverage for concurrent repository activity and a real Codex
  loop closeout.

## Non-goals

- General-purpose merging of arbitrary project files.
- Taking exclusive ownership of project documentation or agent configuration.
- Replacing git with a lisa-specific source-control model.
- Solving conflicts between tickets that are missing dependency edges.

## Stories

- **S-030 — Non-destructive init and upgrade:** preserve local content,
  append ignore rules, and report every mutation.
- **S-031 — Atomic ticket completion:** bind the done transition to an isolated
  successful commit and prevent shared-index leakage.

## Field evidence

The vend repository can supply the restore commits and diffs for both upgrade
clobbers. Its `.lisa/provenance.jsonl` contains the five Codex-seat execution
legs. Use equivalent checked-in fixtures for automated regression tests; do
not make the vend repository a test dependency.
