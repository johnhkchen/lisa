---
id: S-026
title: per-pane-routing
status: open
---

## Per-pane provider + model routing (the north star)

Building on the per-pane-resolvable seam (S-022), run different tickets on
different `(provider, model)` combinations within one loop — e.g. Codex/gpt-x,
Claude/opus, Claude/sonnet×2 concurrently. See
[08 · Design thesis](../../knowledge/codex-client/08-design-thesis.md) §6–8.

### Needs (from the epic + decisions)

- Each ticket resolves `(provider, model)` at spawn via **ticket frontmatter**
  plus a **loop-level default with per-ticket override** (confirmed product
  decision; policy routing by type/phase is explicitly later).
- **Invalid/unavailable route → fall back to the loop default** (epic
  Decision 3), never fail the ticket; the substitution is surfaced in the
  dashboard and recorded in provenance (requested vs. actual route).
- The dashboard surfaces each pane's `(provider, model)`.
- Concurrency is provider-aware enough that mixing providers doesn't silently
  break (separate auth/rate-limit pools); ~16 concurrent mixed-provider agents
  is the explicit stress target.

### Tickets

- **T-026-01** — Routing frontmatter + loop default/override + fallback + dashboard surfacing
- **T-026-02** — Provider-aware concurrency + mixed-provider stress validation
