---
id: T-026-01
story: S-026
title: routing-frontmatter
type: feature
status: open
priority: medium
phase: done
depends_on: [T-024-01, T-025-01]
---

## Context

Per-pane routing: each ticket resolves `(provider, model)` at spawn from
**ticket frontmatter** with the **loop-level default (T-025-01) as fallback**
— different panes in the same loop run different combinations concurrently.
Invalid or unavailable routes **fall back to the loop default** (epic
Decision 3), never fail the ticket, and the substitution is surfaced.

## Acceptance Criteria

- Routing frontmatter fields defined and parsed in lisa-core's ticket parser
  (settle the exact schema — `agent:`/`model:` vs. combined — as part of this
  ticket's Design phase; epic open question 6), ignored harmlessly by older
  lisa versions (unknown-field tolerance already exists).
- The spawn-time resolver (T-022-01 seam) resolves ticket frontmatter →
  loop default → native Claude, in that order; resolution happens per ticket,
  concurrently heterogeneous panes work in one loop.
- Invalid/unavailable route → loop default, with the substitution logged,
  shown in the dashboard, and passed to provenance (requested vs. actual —
  T-027-01 records both).
- The dashboard/thread table surfaces each pane's `(provider, model)`.
- Tests: frontmatter parsing (inline + multiline), resolution precedence,
  fallback behaviour, and a mixed-route scheduling test.

## Notes

- Model selection within a provider rides the same field(s); the Claude
  adapter passes `--model`, the Codex wrapper passes its model flag —
  adapter-owned mapping, resolver stays vocabulary-only.
- Policy-based routing (by type/phase) is explicitly out of scope.
