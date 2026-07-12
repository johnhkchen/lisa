---
id: T-031-03
story: S-031
title: provider-contract-live-regression
type: bug
status: open
priority: critical
phase: done
agent: codex
depends_on: [T-030-02, T-031-02]
---

## Context

Atomic scheduler behavior also requires an agent contract that does not stage
files early or independently publish completion. The current RDSPI workflow
says to commit incrementally during Implement, while the repository-safety
contract requires one final per-ticket completion commit carrying code, work
artifacts, and the phase transition. Align the generated workflow, provider
prompts, and runtime behavior, then exercise the exact Codex-seat failure from
the field report.

This is the convergence ticket for E-002. It depends on both the safe-upgrade
track and the scheduler transaction so template/prompt changes cannot race and
the live test validates the shipped contract rather than an intermediate one.

## Acceptance Criteria

- The bundled RDSPI workflow and all Claude/Codex initial, reuse, and finish-up
  prompts describe one consistent ticket-level commit contract: agents do not
  leave files staged, do not manually set phase/status, and do not move to the
  next ticket before lisa confirms the completion commit.
- Any agent-side git operation required during a ticket uses the isolated lisa
  transaction boundary; ordinary-index `git add`, `git add -A`, and staged
  handoff between commands are not part of the generated workflow.
- Existing project customizations to `docs/knowledge/rdspi-workflow.md` remain
  protected when the bundled contract changes, using S-030's ownership rules.
- A checked-in end-to-end harness runs at least five Codex-routed tickets on a
  reused seat, with one dependency edge and a foreign staged file present
  throughout completion.
- The harness proves that every ticket's Done frontmatter is in its completion
  commit; ticket code and work artifacts are committed; no loop-owned changes
  remain staged or unstaged; the foreign staged file remains uncommitted; and
  the dependent starts only after its prerequisite commit exists.
- The same harness or an equivalent focused case covers a mixed Claude/Codex
  loop so the invariant is provider-neutral.
- The regression records enough commit-tree, index, activity, and provenance
  evidence to diagnose a future failure without relying on dashboard glances.
- User-facing setup/workflow documentation explains the new atomicity guarantee
  and recovery behavior after commit failure.
- `lisa validate`, focused tests, the workspace test suite, WASM release build,
  and plugin Clippy all pass.

## Notes

- Keep the live test repository outside the lisa source tree and commit only
  the reusable harness plus recorded assertions/fixtures.
- The vend restore commits and `.lisa/provenance.jsonl` are evidence sources,
  not runtime dependencies.
