---
id: T-025-02
story: S-025
title: agents-md-and-docs
type: task
status: open
priority: medium
phase: done
depends_on: [T-023-02, T-025-01]
---

## Context

Codex auto-loads `AGENTS.md`, not `CLAUDE.md`; Claude Code still reads
`CLAUDE.md` (doc 06 §AGENTS.md — emit both). And the toggle needs
documentation: prerequisites, trust setup, what the wrapper does, and the
zero-regression promise for projects that never opt in.

## Acceptance Criteria

- `lisa init` (and the templates in `templates.rs`) generate an `AGENTS.md`
  with content equivalent to the generated `CLAUDE.md` (shared source of
  truth — one template rendered to both, or a pointer file — so they cannot
  drift), including the RDSPI workflow reference.
- The Codex ticket prompt references `AGENTS.md` where the Claude prompt
  references `CLAUDE.md` (`ticket_prompt`, plugin `lib.rs:34`).
- README / setup guide document: the client toggle, Codex prerequisites
  (binary, version pinning caveat, trust pre-seeding), wrapper behaviour in
  the pane, and that Claude remains the default with unchanged behaviour.
- Docs do not imply support for providers beyond the two natives (breadth is
  the ACP leg, explicitly future).
- `lisa validate` accepts a project with both context files.

## Notes

- Version-pinning caveat belongs here: Codex's surface drifts (doc 04);
  document the tested codex version and that `lisa doctor` reports the
  installed one.
