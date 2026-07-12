---
id: T-030-02
story: S-030
title: append-only-ignore-and-mutation-report
type: bug
status: open
priority: critical
phase: done
agent: codex
depends_on: [T-030-01]
---

## Context

The 0.4.0-rc.5 template added `.lisa/claude/` and `.lisa/codex/`, but replacing
the entire existing `.lisa/.gitignore` also deleted `hooks/ntfy-topic`. That
made a `0600` secret created by lisa's notification setup visible as an
untracked file. Init already prints a planned action list, but the safety
contract needs an exact, unmistakable record of what a real run actually wrote
and what it declined to overwrite.

Make `.lisa/.gitignore` upgrades append-only and make init's mutation outcome
auditable. Build on the ownership policy from T-030-01 rather than adding a
second competing update classifier.

## Acceptance Criteria

- Updating an existing `.lisa/.gitignore` preserves every existing line and
  adds only missing lisa-required rules; it never deletes, reorders, or rewrites
  a project rule.
- Re-running init is idempotent: required rules are not duplicated, including
  when the file lacks a trailing newline or uses harmless surrounding spacing.
- A fixture containing `hooks/ntfy-topic` retains that rule and the corresponding
  secret remains ignored according to `git check-ignore` after upgrade.
- Both dry-run and real init output distinguish creates, updates, no-ops, and
  safety skips. After a real run, the command prints the exact set of files it
  created or modified rather than only a generic completion message.
- Reported mutations match the filesystem write set; skipped and unchanged
  files are never reported as rewritten.
- Regression tests cover the vend-style workflow customization and secret
  ignore rule in the same upgrade fixture.
- CLI documentation states the ownership and append-only contracts and tells
  operators to inspect the reported files before their next commit.
- Focused init tests and the full CLI test suite pass.
