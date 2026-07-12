---
id: T-030-01
story: S-030
title: ownership-aware-init-planning
type: bug
status: open
priority: critical
phase: done
agent: codex
depends_on: []
---

## Context

`plan_init_actions` currently treats several differing template-backed files
as unconditional `UpdateFile` actions. That makes an upgrade capable of
replacing committed project additions with the current bundled template. The
vend 0.3.0 → 0.4.0-rc.5 upgrade demonstrated this on
`docs/knowledge/rdspi-workflow.md`; the same ownership ambiguity applies to
other existing template-backed files considered by init.

Establish and implement an ownership-aware update policy across the complete
init action set. Lisa may replace content only when it can establish that the
existing file is an unmodified lisa-installed version. A locally modified or
unclassifiable existing file must be preserved and surfaced as a safety skip.
Structured merges may continue where they already preserve unrelated content.

## Acceptance Criteria

- Every file path considered by `plan_init_actions` has an explicit tested
  policy: create-if-absent, replace-if-proven-pristine, format-aware merge, or
  preserve-if-present.
- A workflow fixture containing committed project additions is byte-for-byte
  unchanged after both planning and a real init run.
- Equivalent regression coverage protects locally modified lisa hook scripts
  and any other plain-text template targets from silent replacement.
- An existing file that matches a known prior lisa template can still receive
  the current safe template update; a current template remains a no-op.
- If prior ownership cannot be established, init chooses preservation and
  emits a specific skip reason instead of guessing.
- Unreadable or malformed existing files are never replaced as a fallback.
- Fresh initialization behavior remains compatible, and focused init tests plus
  the full CLI test suite pass.

## Notes

- Do not solve this with a path-specific exception for the workflow file. The
  regression exposed a missing ownership contract, not a one-file bug.
- Keep project files as the source of truth. Any lisa metadata used to recognize
  installed templates must not make deletion or corruption of that metadata
  authorize an overwrite.
