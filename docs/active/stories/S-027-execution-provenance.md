---
id: S-027
title: execution-provenance
status: open
---

## Execution provenance & routing-policy telemetry

Record what actually happened per ticket-run so routing policies can be
evaluated empirically — which policies yield the best results, which are most
cost-effective, and which hold up under high concurrency. This is the
measurement half of the north star
([08 · Design thesis](../../knowledge/codex-client/08-design-thesis.md) §6–8).

### Decided shape (epic Decision 2)

- **Append-only JSONL ledger at `.lisa/provenance.jsonl`** — one record per
  completed ticket-run, written by the plugin **after** completion
  (write-after; never races the agent, never touches the agent-owned ticket
  frontmatter). `.lisa/` only gitignores `signals/`, so the ledger is
  committable learning data.
- Record: ticket id, `(method, provider, model)` — requested **and** actual
  (so Decision-3 fallbacks are visible in the data) — started/ended,
  wall-clock, tokens/cost where obtainable, concurrency-at-run, outcome.
- Provenance lands **before** per-pane routing: the whole-loop toggle already
  produces cross-provider data loop-by-loop.

### Open: cost-signal fidelity per adapter

Codex cost comes from `turn.completed.usage` in the wrapper's event stream.
The **Claude-side cost signal is an open question** (epic open question 7) —
what is obtainable from hooks/transcripts without perturbing the run, and what
does the ledger record when cost is unobtainable (nullable, never fabricated).

### Tickets

- **T-027-01** — Provenance ledger: write-after JSONL records at completion
- **T-027-02** — Usage/cost capture per adapter (Codex usage wiring + Claude cost investigation)
