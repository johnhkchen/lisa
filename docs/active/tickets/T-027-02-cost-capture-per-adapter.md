---
id: T-027-02
story: S-027
title: cost-capture-per-adapter
type: task
status: open
priority: medium
phase: done
depends_on: [T-027-01]
---

## Context

Make the ledger's cost/token fields real for both natives. Codex is largely
wired (wrapper captures `turn.completed.usage`); the **Claude-side cost signal
is the open question** (epic open question 7): what is obtainable per run —
hook payloads, transcript files, `/cost`-equivalent surfaces — without
perturbing the run or violating the artifact-driven contract. The moat
argument leans on cross-provider comparison; this ticket is what makes the
cost axis of that comparison honest.

## Questions to answer (investigation phase)

1. What per-session token/cost data does Claude Code expose that a hook script
   or post-run read can capture (Stop-hook payload fields, transcript path
   JSONL, session files)? On which surface is it stable?
2. Attribution: can captured usage be tied to the pane/ticket with the same
   `LISA_PANE_ID` correlation the other signals use?
3. Comparability: are Claude and Codex token counts commensurable enough to
   record raw, or does the ledger need provider-native units + a normalized
   field (record raw always; normalize only if defensible)?

## Acceptance Criteria

- A findings write-up answering the questions with evidence on the current
  Claude Code version.
- Claude runs populate ledger tokens/cost where obtainable via the chosen
  mechanism (hook-side capture writing an artifact the plugin reads
  write-after, or documented as unobtainable with fields remaining null —
  never fabricated).
- Codex usage capture verified end-to-end into the ledger (closing the
  T-023-01 → T-027-01 plumbing with a real run).
- Ledger schema docs updated with per-provider fidelity caveats (what each
  provider's numbers do and don't include).

## Notes

- Respect write-after: no mid-run writes that could race the agent; capture
  must not add hooks that fire per-tool-call payload processing beyond what
  exists (heartbeat script stays trivial).
