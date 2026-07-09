---
id: T-028-01
story: S-028
title: spike-harness-live-run
type: spike
status: open
priority: high
phase: ready
depends_on: []
---

## Context

> **Unblocked 2026-07-09:** codex-cli **0.144.0** is installed and
> authenticated (ChatGPT login). That drifts from the pinned research target
> `rust-v0.142.5` — the probes are the drift detector; record the observed
> drift per the AC. See `docs/knowledge/codex-day-runbook.md` (steps 0–3).

The T-021-01 spike produced a turnkey probe harness
(`docs/active/work/T-021-01/harness/run-all.sh`, probes q1–q5) but no codex
binary was available, so every verdict in
`docs/active/work/T-021-01/design.md` is tagged `[PROVISIONAL]`. This ticket
executes the harness against a real codex install and replaces the
provisional verdicts with empirical ones.

## Acceptance Criteria

- `run-all.sh` executed on a host with the codex CLI; evidence captured under
  `harness/out/` (or transcribed if out/ stays untracked).
- Every `[PROVISIONAL]` tag in `docs/active/work/T-021-01/design.md` replaced
  with a PASS/FAIL verdict + evidence pointer, noting the exact codex version
  run (if it differs from `rust-v0.142.5`, record observed drift).
- **Q2 is the gate:** a clear go/no-go on `codex exec --json` fidelity under
  real tool use. On no-go, STOP and escalate — the app-server fallback
  (doc 05 Option 2) is a human decision; do not proceed to T-028-02.
- Q3's rendering verdict either confirms the wrapper's render-from-JSON mode
  or files a follow-up ticket to switch it to tee-stderr.
- Q4's trust findings reconciled with doctor's `pregrant_codex_trust_in`
  behaviour (does the pre-seed actually unblock a fresh `CODEX_HOME`?).

## Notes

- The harness is spike-scaffolding: it may be deleted after verdicts are
  transcribed (per its README), but only once this ticket is done.
- Version drift found here feeds updates to `docs/knowledge/codex-client/`
  docs 02/04/05.
