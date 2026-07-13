---
id: S-029
title: codex-integration
type: story
status: done
priority: high
tickets: [T-029-01, T-029-02, T-029-03]
---

## Codex integration — first live run

Everything E-001 built for the Codex client is CI-green but empirically
unproven: no `codex` binary had ever run on this host when the epic shipped
(see `docs/archive/epics/E-001-pluggable-agent-client.md`). This story is the
integration test that turns "should work" into "observed working" — one
consolidated ticket that executes the codex-day runbook end-to-end.

**Start here:
[`docs/knowledge/codex-day-runbook.md`](../../knowledge/codex-day-runbook.md).**

Supersedes S-028 (archived 2026-07-09 with the rest of the E-001 board, never
run; scope carried over unchanged, its two tickets consolidated into one).
The live instruments stay in `docs/active/work/T-021-01/` (probe harness) and
`docs/active/work/T-024-01/` (checklist + `validate-codex-loop.sh`); sweep
those to `docs/archive/work/` only after this story completes.

### The gate

Q2 — `codex exec --json` fidelity under real tool use — remains the go/no-go
for the whole wrapper approach. Its documented fallback is the app-server
integration (codex-client doc 05 Option 2), which is a **human decision** if
triggered, not something to design around.

### Environment (observed 2026-07-09)

codex-cli **0.144.0**, logged in via ChatGPT. The pinned research target was
`rust-v0.142.5`, so drift is live — the probes double as the drift detector.

### Tickets

- **T-029-01** — Execute the codex-day runbook: probes → live loop → provenance
- **T-029-02** — Fix the timer race that drops reused-session Codex prompts
