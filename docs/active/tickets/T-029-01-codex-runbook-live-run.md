---
id: T-029-01
story: S-029
title: codex-runbook-live-run
type: spike
status: open
priority: high
phase: ready
depends_on: []
---

## Context

The one board item for codex-day: execute
`docs/knowledge/codex-day-runbook.md` end-to-end against the installed codex
CLI (0.144.0 observed 2026-07-09; pinned research target `rust-v0.142.5` —
record all drift). Preflight and step 2 are already done: the workspace is
`0.4.0-rc.1`, a fresh `lisa` is installed, and this ticket replaces the
archived S-028 pair (T-028-01 → T-028-02, never run) with their scope
unchanged.

## Acceptance Criteria

- Doctor (runbook step 1): `lisa doctor` green with the codex client
  selected; the trust pre-seed (`pregrant_codex_trust_in`) verified to
  unblock a fresh `CODEX_HOME` on the installed version (re-verifies
  bug #14345 against 0.144.0).
- Spike harness (runbook step 3): `docs/active/work/T-021-01/harness/run-all.sh`
  executed; every `[PROVISIONAL]` tag in
  `docs/active/work/T-021-01/design.md` replaced with an empirical
  PASS/FAIL verdict + evidence pointer, exact codex version noted.
- **Q2 is the hard gate:** if `--json` drops events under real tool use
  (#15451-class), STOP — do not run the live loop; surface the app-server
  fallback (codex-client doc 05 Option 2) as a human decision.
- Q3's verdict either confirms the wrapper's render-from-JSON mode or files
  a follow-up ticket to switch to tee-stderr (do not rework inline).
- Live loop (runbook step 4): `docs/active/work/T-024-01/validate-codex-loop.sh`
  then `lisa loop` on the dry-run project; PASS/FAIL recorded per checklist
  rows 1–8 from durable artifacts, not dashboard glances. Row 8 re-tested as
  a true mixed loop — one Claude ticket + one Codex ticket via `agent:`
  frontmatter in the same loop — per the runbook's stale-scope correction.
- Provenance (runbook step 5): `.lisa/provenance.jsonl` contains a Codex
  record with real `turn.completed.usage` tokens and a Claude record from
  the mixed run (schema: `docs/knowledge/provenance-ledger.md`).
- Write-back: any drift (version, event names, trust behaviour) recorded in
  `docs/knowledge/codex-client/` docs 02/04/05; every checklist FAIL filed
  as a `type: bug` ticket; the runbook status log updated with the outcome.

## Notes

- Keep the dry-run project outside this repo (`/tmp/lisa-codex-dryrun` per
  the script); commit only recorded results.
- Row 6 (review-timeout finish-up) needs a deliberately stalled Review
  session — the checklist describes the forcing technique.
- The harness is spike-scaffolding and may be deleted after verdicts are
  transcribed, but only once this ticket is done.
- Known template gap to fold into the write-back: `LISA_GITIGNORE`
  (`templates.rs`) ignores only `signals/`, but `.lisa/claude/` and
  `.lisa/codex/` are runtime state too — an interactive session's Stop hook
  committed a stray `last.usage.json` here (fixed for this repo in
  `.lisa/.gitignore`, 2026-07-09).
