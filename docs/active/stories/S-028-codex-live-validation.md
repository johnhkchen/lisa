---
id: S-028
title: codex-live-validation
status: open
---

## Codex live validation — retire the PROVISIONAL verdicts

The E-001 Codex client shipped fully tested against recorded event streams but
has **never run against a real `codex` binary**: the T-021-01 spike's five
verdicts are all `[PROVISIONAL]` (probes written, never executed), the
T-024-01 live checklist is unrun, and no provenance record with real Codex
tokens exists. This story is the deferred empirical half, parked until the
Codex CLI is installed.

**Start here on codex day:
[`docs/knowledge/codex-day-runbook.md`](../../knowledge/codex-day-runbook.md).**

### Why the tickets are `status: blocked`

Both tickets require the `codex` CLI (pinned target `rust-v0.142.5`) on the
host. `status: blocked` keeps them out of scheduling (`dag.rs::can_start`)
so an intervening loop never spawns an agent that immediately fails the
missing-binary check. Unblocking = flipping `status:` to `open` (runbook
step 2).

### The gate

T-028-01's Q2 (JSONL fidelity under real tool use) is the go/no-go for the
whole wrapper approach; its documented fallback is the app-server integration
(doc 05 Option 2), which is a **human decision** if triggered.

### Tickets

- **T-028-01** — Run the T-021-01 spike harness live; replace all PROVISIONAL verdicts
- **T-028-02** — Live loop validation (T-024-01 checklist) + first real provenance record
