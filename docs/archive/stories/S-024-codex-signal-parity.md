---
id: S-024
title: codex-signal-parity
status: open
---

## Codex signal parity: the scheduler treats a Codex pane correctly end-to-end

Codex sessions must feed the scheduler the signals it already consumes, with
correct per-pane attribution — and the scheduler/UI must never misread a Codex
pane's differing semantics:

- `.stopped` arrives **once per exec run** (one turn), not once per Claude
  turn; verify auto-complete-Review and transition handling still behave.
- `.idle`/`.awaiting` never occur for Codex (autonomous headless — confirmed in
  [04](../../knowledge/codex-client/04-risks-and-open-questions.md)); the
  awaiting/attention machinery must be a defined no-op, and phase advancement
  must ride `check_artifact_advances` (artifact presence alone), which already
  works independent of idle signals.
- Stuck detection: `item.*`-driven heartbeats must keep the stuck-detector
  honest through long tool calls; `.error` must surface failures promptly
  rather than waiting for 2× stuck-threshold silence.
- Wrapper/trust setup for unattended runs is generated or doctored by lisa the
  way `.claude/` hook setup is today.

This story is the end-to-end validation of the contract the wrapper (T-023-01)
and adapter (T-023-02) implement — a real multi-ticket loop run on Codex with
transitions, liveness, failure, and review-completion exercised.

### Tickets

- **T-024-01** — End-to-end Codex loop parity validation (transitions, liveness, failure paths)
