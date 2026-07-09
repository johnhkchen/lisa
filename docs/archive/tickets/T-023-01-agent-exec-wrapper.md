---
id: T-023-01
story: S-023
title: agent-exec-wrapper
type: feature
status: open
priority: high
phase: done
depends_on: [T-021-01]
---

## Context

Build the Codex wrapper as a subcommand of `lisa-cli` (epic Decision 5 — not a
generated script): lisa types
`LISA_PANE_ID=<n> LISA_TICKET_ID=<t> <lisa> agent-exec …` into a pane; the
wrapper runs `codex exec --json -a never -s workspace-write …`, consumes the
JSONL event stream, writes lisa's signal files, and renders a chunked
human-readable conversation view to the pane (its stdout IS the pane).

Event → signal mapping (doc 05 Option 1, corrected by T-021-01 findings):

| Codex event | lisa signal |
|---|---|
| `thread.started` | record `thread_id` (for `resume`) |
| `item.started/updated/completed` | `.heartbeat` |
| `turn.completed` + exit 0 | `.stopped` |
| `turn.failed` / top-level error / non-zero exit | `.error` |

Turn events + process exit are authoritative; item statuses are best-effort
heartbeat only (#14691). Schema reference: `@openai/codex-sdk` events; shape
reference: takopi; renderer reference: codex-trace (doc 06 Tier 1).

## Acceptance Criteria

- A `lisa agent-exec` (name flexible) subcommand that takes the prompt (and
  the codex flags it needs), runs `codex exec --json`, and:
  - writes `pane-$LISA_PANE_ID.heartbeat` on item events (mtime bump),
    `.stopped` on successful turn completion + exit, `.error` on failure —
    exactly the files the plugin already polls in `.lisa/signals/`;
  - renders the conversation to stdout per the T-021-01 rendering verdict
    (tee-stderr or render-from-JSON), chunked output acceptable;
  - persists the `thread_id` where a follow-up invocation can find it
    (for `agent-exec --resume`, used by T-023-02's finish-up path);
  - exposes captured `turn.completed.usage` for provenance (T-027-01) — e.g.
    written alongside the signal or to a per-run artifact.
- Degrades safely: missing `LISA_PANE_ID` → still runs codex and renders, just
  writes no signals (mirrors the hook scripts' `[ -n "$LISA_PANE_ID" ]` guard).
- Unit tests over the JSONL→signal translation using recorded event streams
  from the spike (no live codex needed in CI).

## Notes

- The wrapper is host-side Rust in `lisa-cli` — real JSON parsing, versioned
  atomically with the plugin (the stale-generated-script failure mode the
  repo's own `.lisa/hooks/` exhibits is the thing this avoids).
- Trust/flags for unattended runs per the T-021-01 trust verdict; doctor-side
  pre-seeding lands in T-025-01.
