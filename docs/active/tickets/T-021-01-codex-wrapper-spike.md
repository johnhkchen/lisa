---
id: T-021-01
story: S-021
title: codex-wrapper-spike
type: spike
status: open
priority: high
phase: done
depends_on: []
---

## Context

Verify, on the pinned Codex version (`rust-v0.142.5`), the empirical unknowns
the `codex exec --json` wrapper decision rests on. The driving model is already
decided (epic Decisions + doc 05); this spike confirms mechanics, it does not
re-open interactive TUI driving or interactive hooks (both refuted in doc 04).

This is a **spike**: the deliverable is a written verdict per unknown, using
stub scripts. No production code.

## Questions to answer

1. **Env inheritance** — launch `LISA_PANE_ID=7 <stub>` where the stub dumps
   env and runs `codex exec` with a prompt like "run `env | grep LISA`". Does
   the wrapper child (and codex's shell) see the var?
2. **`--json` fidelity** — run one real RDSPI-style ticket prompt through
   `codex exec --json`; log every event + exit code. Are `turn.*`/`item.*`
   events complete under active MCP/tools (#15451)? Do item statuses misreport
   at turn end (#14691)? Confirm the anchor rule (turn events + exit code
   authoritative) holds.
3. **In-pane rendering** — with stdout piped and stderr inherited in a Zellij
   pane: what does codex render on stderr under `--json`, and at what
   granularity? Does exec emit partial assistant text or only completed
   messages? (Chunked is acceptable — this picks tee-stderr vs.
   render-from-JSON for T-023-01.)
4. **Directory trust headless** — does a fresh `CODEX_HOME` block
   `codex exec -a never` on an untrusted repo? Confirm the pre-seeding
   (`projects.<path>.trust_level = "trusted"`) and/or flags doctor must apply
   (open bug #14345 context).
5. **Follow-up via resume** — after an exec run completes, does
   `codex exec resume <thread_id>` reliably continue the session with new
   instructions (the `finish_up_prompt` analog for T-023-02)?

## Acceptance Criteria

- A `design.md` (or findings artifact) with a verdict + evidence per question
  (captured event streams, env dumps, exit codes), pinned to the exact codex
  version tested.
- A go/no-go on the wrapper approach, and any event-mapping corrections to
  doc 05's table that the real stream contradicts.
- The tee-stderr vs. render-from-JSON recommendation for T-023-01.
- No production code merged; stubs kept clearly separate.

## Notes

- Unknowns list from doc 05 §"Empirical unknowns to settle in the spike" plus
  the review pass recorded in the epic's Decisions section.
- If `--json` fidelity fails badly (Q2), the fallback surface is the
  app-server (doc 05 Option 2) — flag for a human decision rather than
  designing around it.
