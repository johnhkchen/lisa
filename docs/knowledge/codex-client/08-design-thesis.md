# 08 · Design thesis & target architecture ⭐ (read this first)

> Part of the **Codex client intel packet** (`docs/knowledge/codex-client/`). See [README](./README.md).
> **This is the goal.** Docs 01–07 are the research that led here; this doc states *what lisa is trying to become and why*. If you are overseeing implementation, read this before the epic — it is the north star every ticket serves.
> Status: **priority / north-star.** Written 2026-07-01. Marks decisions (D) and bets/assumptions (A) explicitly so the goal is unambiguous.

---

## 1. The goal in one paragraph

lisa is growing from "a DAG scheduler that runs Claude Code" into **the neutral routing-and-measurement layer for agentic coding work**: run a backlog of dependency-ordered tickets concurrently, **route each ticket to a chosen `(provider, model)`** (e.g. Codex/ChatGPT, Claude/Opus, Claude/Sonnet — in the *same loop*), and **record what actually happened** (which provider/model ran it, what it cost, how long, at what concurrency, with what outcome) so the operator can learn **which routing policies actually win on cost, quality, and throughput.** The Codex work is the first increment; per-pane provider/model routing + execution provenance is the destination.

## 2. Who this is for, and the pain (D)

The user is a developer **mastering "vibe coding" without burning money.** They know the model labs keep specializing on price/quality (the market stays *plural*, not winner-take-all), and that a cheap/open model will eventually do "well enough" what costs $25/MTok today. They can't wait for that workflow to appear, because **no closed-source lab will ever build it** — a vendor's orchestrator always routes to its own models.

The pain lisa removes: **there is no neutral, empirical answer to "which agent should do *this* task."** So teams either over-pay (everything on the expensive model) or guess per-ticket and never learn. lisa turns that guess into measured, improvable policy.

## 3. Why no one else builds this (D — the moat)

- **The labs can't.** A single vendor being a neutral cross-vendor router cannibalizes its own usage — the referee can't be a player. Their orchestration (Codex subagents, Claude agent teams) is single-vendor by construction. Confirmed direction; see [07](./07-ecosystem-viability.md).
- **The tools don't.** The market converged on flat parallel workspaces + human coordination (claude-squad, cmux) or same-task fanout (uzi); the standalone-orchestrator middle is *dying* (vibe-kanban shut down, others archived). No high-traction tool combines **DAG scheduling + cross-vendor routing + provenance.** That triple is lisa's whitespace.
- **API-level routers (OpenRouter et al.) are the wrong granularity.** They route a *model for a completion*. lisa routes an **agent for a task** — a whole multi-turn, tool-using, file-writing session — and measures the real outcome. Nobody does routing-and-measurement at the agent-task level.
- **lisa's unfair advantage on the hard part:** quality is the hard signal to measure, and the **RDSPI workflow already produces gradeable artifacts** (Review phase, `review.md`, tests, structured work products). lisa sees the whole task outcome through a workflow built to yield quality signals — a generic router only sees tokens.

## 4. The bet lisa is making (A — state it plainly)

1. **The model market stays plural** — labs keep specializing, so "which agent for this task" stays a live, valuable question. *(If one agent becomes cheap+dominant, the routing moat shrinks.)*
2. **The interop/neutrality coalition persists** — enough parties "care about not picking a side" (Zed, JetBrains, Google, GitHub, the Linux Foundation / AAIF around MCP + AGENTS.md) that a neutral integration layer stays maintained and grows. lisa *rides* that layer for provider breadth instead of writing a shim per vendor.

The architecture below is designed so lisa **survives either bet being wrong** (see the barbell + graduation rule).

## 5. Target architecture — the three-leg "barbell" (D)

lisa supports **three integration methods behind one normalized signal contract.** Each leg has a distinct, durable job. This is a *stable* portfolio, not a transitional redundancy — because ACP's capabilities are converging but the two frontier labs stay adapter-only (Anthropic declined native ACP; OpenAI ships a competing app-server). See [05](./05-bridging-the-discrepancy.md), [06](./06-off-the-shelf-tooling.md), and the ACP capability/trajectory research summarized in [07](./07-ecosystem-viability.md).

| Leg | Long-term job | Why it earns its keep |
|---|---|---|
| **Native Claude Code** (hooks) | Depth + reliability anchor on the #1 provider | Richest lifecycle (hooks), guaranteed model pinning, per-turn precision, known concurrency, zero adapter-lag. Works today. |
| **Native Codex** (`codex exec --json` wrapper) | Depth on #2 + proof the abstraction is genuinely cross-vendor + ACP fallback | Same control/precision as native Claude; stands in when `codex-acp` is immature. |
| **ACP** (Agent Client Protocol) | Breadth for the long tail + uniform telemetry + the open-model on-ramp | One integration → the ~45–60 registry agents for free; **stable uniform `usage_update` cost across all of them**; the day a cheap/open model ships an ACP adapter, lisa routes to it with zero new code. |

**The load-bearing internal abstraction (D):** all three legs emit the **same normalized signals** (`.heartbeat` / `.stopped` / `.error` + usage/cost) that the scheduler already consumes. This makes it **3× adapter code but 1× scheduler.** Preserve this contract at all costs — it is what keeps three methods sane and the artifact-driven core untouched.

**How it evolves (D):** the structure stays **2 native anchors + ACP-for-N**, growing *only on the ACP leg*. New providers get ACP, never a third native adapter — unless they meet the graduation rule below. Native persists as the flagship-reliability + insurance layer even as ACP matures.

**Graduation rule (D — the standing policy):** *A provider earns a native adapter only when it is run at sustained volume where adapter-lag, per-turn cost precision, or reliability actually bites. Everything else stays on ACP indefinitely.* This is the anti-over-engineering guardrail reborn: **native is earned by usage, not added by default.**

**Why the barbell is antifragile (D):** native covers you if ACP stalls (the real MCP/VS-Code-commoditization threat); ACP + the other native cover you if a lab breaks its CLI. lisa is captive to no single power center — not either lab, not the Zed/JetBrains coalition. For a single-`.wasm` tool betting on a plural, shifting market, that diversification *is* the resilience.

## 6. What the operator gets (D — the felt product)

- **Route per ticket:** `(provider, model)` chosen via ticket frontmatter + a loop-level default with per-ticket override. (Policy routing by task-type/RDSPI-phase is a later evolution.)
- **See it work:** each pane shows the live conversation and its `(provider, model)` + cost-so-far.
- **Learn from it:** after each ticket, provenance captures **which `(provider, model)` ran it, cost/tokens, wall-clock, concurrency-at-run, and outcome/quality**, structured to answer across many runs *which policies win on cost, quality, and scale (incl. 16-agent bursts).*
- **Calibrate deliberately (design idea worth pursuing):** an occasional bake-off mode that fans one representative task across providers to build comparison data — uzi's fanout pattern repurposed from "pick a winner for this task" to "learn a policy for all future tasks." This resolves the tension that frugal operation runs each task once, so you never learn what "good enough" was.

## 7. Sequencing — MVP first, north star always (D)

- **MVP (hackathon):** whole-loop client selection (Codex toggle) — but the client seam is built **per-pane-resolvable from day one** so routing is a later *addition*, not a repaint.
- **North star:** per-pane `(provider, model)` routing + execution provenance + the three-leg adapter portfolio.
- **Do not** let the MVP's simplicity bake in a whole-loop-only assumption. The seam resolves `(method, provider, model)` per ticket at spawn.

## 8. Definition of success (the goal, made checkable)

lisa is delivering on this thesis when, in one loop, an operator can:
1. Run a dependency-ordered backlog with **different tickets on different `(provider, model)`** concurrently.
2. **Watch each pane's conversation** and its provider/model/cost.
3. Afterward, **query provenance** to compare routing policies on cost, quality (via RDSPI artifacts), and behavior under high concurrency.
4. **Add a new provider** that has an ACP adapter with **no lisa code change** — and know from data whether to route real work to it.

## 9. Explicit non-goals (D)

- Not an editor/terminal-surface play (lisa doesn't out-polish cmux/superset; it wins on scheduling + routing + measurement).
- Not a general N-*native*-provider framework — native is bounded by the graduation rule; breadth comes from ACP.
- Not policy-based routing yet (frontmatter + loop-default/override first; type/phase policies later).
- Not changing the artifact-driven core (DAG, phase detection, scheduling) — the normalized signal contract keeps it client-agnostic.

---

**For the implementation overseer:** the actionable breakdown of this goal lives in
[`../../active/epics/E-001-pluggable-agent-client.md`](../../active/epics/E-001-pluggable-agent-client.md)
(stories S-021→S-027, needs-only). This doc is *why*; the epic is *what*; the RDSPI cycle is *how*.
