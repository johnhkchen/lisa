---
id: T-027-01
story: S-027
title: provenance-ledger
type: feature
status: open
priority: medium
phase: done
depends_on: [T-023-02]
---

## Context

The measurement half of the north star: an **append-only JSONL ledger at
`.lisa/provenance.jsonl`** (epic Decision 2), one record per completed
ticket-run, written by the plugin **after** completion — write-after, never
racing the agent, never touching agent-owned ticket frontmatter. `.lisa/`
gitignores only `signals/`, so the ledger is committable learning data. Lands
before per-pane routing: the whole-loop toggle already yields cross-provider
data loop-by-loop.

## Acceptance Criteria

- On ticket completion (Done via review auto-complete or manual mark-done) and
  on terminal failure (reclaim/`.error`), the plugin appends one JSON record:
  ticket id, `(method, provider, model)` requested and actual, started/ended
  timestamps, wall-clock, tokens/cost where available (nullable — never
  fabricated), concurrency-at-run (running thread count at spawn and/or peak),
  and outcome (done / failed / timed-out).
- Append-only semantics: retries/resets of the same ticket produce additional
  records; nothing rewrites history. Write failures are logged, never fatal to
  the loop.
- Codex cost/tokens flow from the wrapper's captured `turn.completed.usage`
  (T-023-01) into the record; Claude records carry null cost until T-027-02.
- The record schema is documented (field table + example) so it's queryable
  across runs with jq/duckdb; schema carries a version field.
- Native tests: record emitted on completion and on failure; append-not-
  rewrite; agent frontmatter untouched.

## Notes

- Before routing lands (T-026-01), requested == actual == the loop default;
  populate both fields from day one so the schema doesn't change.
- A human-readable mirror in `docs/active/work/<ticket>/` is explicitly
  deferred; the ledger is the source of truth.
