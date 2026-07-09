---
id: S-021
title: codex-parity-spike
status: open
---

## Spike: prove the Codex wrapper control surface (gates the rest)

Before committing to the Codex client build, confirm on the pinned Codex version
(`rust-v0.142.5`) that the `codex exec --json` wrapper approach satisfies lisa's
signal contract. The driving-model decision is already made (host-side wrapper
translating the JSONL event stream into signal files — see
[05 · Bridging the discrepancy](../../knowledge/codex-client/05-bridging-the-discrepancy.md)
and the epic's Decisions section); this spike verifies the empirical unknowns
that decision rests on. Interactive TUI driving and interactive hooks were
adversarially refuted ([04](../../knowledge/codex-client/04-risks-and-open-questions.md))
and are NOT re-litigated here.

### Unknowns to settle (from doc 05 §"Empirical unknowns" + the review pass)

1. **Env inheritance** — does a wrapper-launched `codex exec` child see
   `LISA_PANE_ID`?
2. **`--json` fidelity under a real RDSPI ticket** — events not dropped under
   MCP/tools (#15451); item-status bugs (#14691); exit-code behaviour.
3. **In-pane rendering** — what renders on stderr under `--json`, and at what
   granularity? (Chunked output is acceptable per product decision; this picks
   tee-stderr vs. render-from-JSON.)
4. **Directory trust headless** — does a fresh `CODEX_HOME` block
   `codex exec -a never`? What pre-seeding does doctor need?
5. **Follow-up mechanics** — does `codex exec resume <thread_id>` reliably
   continue with new instructions (the `finish_up_prompt` analog)?

### Scope

- Stub scripts + a written verdict per unknown; no production code.
- Output feeds T-023-01 (wrapper) and T-023-02 (adapter) directly.

### Tickets

- **T-021-01** — Spike: verify `codex exec --json` wrapper mechanics on the pinned version
