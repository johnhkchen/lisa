---
id: T-028-02
story: S-028
title: live-loop-validation
type: task
status: blocked
priority: high
phase: ready
depends_on: [T-028-01]
---

## Context

> **BLOCKED until the Codex CLI is installed** and T-028-01's Q2 gate passes.
> Unblock by setting `status: open` — see
> `docs/knowledge/codex-day-runbook.md` (steps 4–5).

Run the live half of the Codex parity validation that CI cannot reach: the
T-024-01 checklist (`docs/active/work/T-024-01/checklist.md`, rows 1–8) via
`validate-codex-loop.sh`, plus the first provenance record carrying real
Codex token usage — closing T-027-02's deferred "live Codex e2e" concern.

## Acceptance Criteria

- `validate-codex-loop.sh` run; a real `lisa loop` executed against the
  dry-run project on the Codex client; PASS/FAIL recorded per checklist row
  from durable artifacts (ticket files, `.lisa/signals/`, `.lisa/codex/`),
  not dashboard observation.
- **Row 8 re-scoped and re-tested:** the checklist's "mixed loop not
  achievable — client is loop-wide" note predates T-026-01's routing
  frontmatter. Test the original AC: one Claude ticket + one Codex ticket
  (via `agent:` frontmatter) in the same loop, completing with correct
  per-pane attribution and per-pane `(provider, model)` on the dashboard.
- `.lisa/provenance.jsonl` contains a Codex record with real
  `turn.completed.usage` tokens (schema per
  `docs/knowledge/provenance-ledger.md`), and a Claude record from the mixed
  run for comparison.
- Failures become tickets; the checklist is updated with the recorded
  results and the stale scope note corrected.

## Notes

- The review-timeout finish-up row (row 6) needs a deliberately stalled
  Review session — the checklist describes the forcing technique.
- Keep the dry-run project out of this repo (`/tmp/lisa-codex-dryrun` per the
  script); commit only the recorded results.
