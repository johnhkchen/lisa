# 06 · Off-the-shelf tooling — what we can reuse

> Part of the **Codex client intel packet** (`docs/knowledge/codex-client/`). See [README](./README.md).
> Generated 2026-07-01 from three verified landscape surveys (Codex SDKs/wrappers/renderers, multi-agent CLI orchestrators, cross-agent abstraction protocols). Every repo below was confirmed via a fetched page or real search result; unconfirmed names are marked **unverified**. Facts current as of ~Codex `rust-v0.142.5`.
> **Options intel, not an implementation plan.** It answers "what can we reuse instead of hand-rolling?" and how each fits lisa's constraints.

## The constraint that filters everything: lisa is a Zellij WASM plugin

lisa doesn't spawn agents as long-lived subprocesses it pipes to. It **writes a shell command into a pre-created pane** and **reads signal files** (`.lisa/signals/pane-<n>.*`); its only host-process capability is Zellij's fire-and-forget `RunCommand` (used today for the `on-notify` hook, `lib.rs:361`) — **not** a persistent bidirectional stdio pipe. See [01](./01-lisa-claude-coupling.md).

Consequence for reuse: anything that requires a **resident JSON-RPC stdio client** (ACP, app-server SDKs) must run as a **host-side helper process** that lisa launches into a pane, which then relays lifecycle back to the plugin via signal files / `zellij pipe`. In-WASM Rust crates for those protocols don't fit directly. A **host-side wrapper that writes signal files**, by contrast, drops into lisa's existing model with almost no plugin change — it's simply the Codex analog of Claude's hook scripts.

---

## Two tiers of reuse

### Tier 1 — Tactical: a thin `codex exec --json` wrapper (fits lisa today)

lisa types `codex-wrapper … codex exec --json …` into the pane instead of `claude …`; the wrapper (inheriting `LISA_PANE_ID`) parses the JSONL stream, writes `.heartbeat`/`.stopped`/`.error`, and renders the conversation to the pane (its stdout *is* the pane). The plugin's signal-consuming core is unchanged. This is [05](./05-bridging-the-discrepancy.md)'s Option 1, and the wrapper *is* the Codex equivalent of `on-stop.sh`/`on-heartbeat.sh`.

**Finding: there is no published *Rust* crate that parses `codex exec --json` into typed lifecycle events** (codex-core/codex-exec are unpublished; the Rust crates that exist target the app-server protocol). So the parser is hand-written — but it's thin, and three off-the-shelf pieces de-risk it:

| Reuse | What it gives | License | Verdict |
|---|---|---|---|
| **`@openai/codex-sdk`** event schema ([npm](https://www.npmjs.com/package/@openai/codex-sdk)) | The authoritative turn/item event taxonomy (`turn.started/completed`, `item.started/updated/completed`, item types) — mirror it in Rust. Confirms `exec --json` is exactly how OpenAI's own SDK drives Codex. | Apache-2.0 (official) | **Reference — the schema to copy** |
| **`banteg/takopi`** runner model ([repo](https://github.com/banteg/takopi)) | Best template for collapsing Codex JSONL → a minimal typed lifecycle (`started` / `action{phase}` / `completed` + resume token) covering turn-complete, liveness, error. | MIT | **Reference — copy the shape** |
| **`PixelPaw-Labs/codex-trace`** ([repo](https://github.com/PixelPaw-Labs/codex-trace)) | Rust renderer that turns Codex JSONL into readable turns w/ tool calls + **live tail** — closest thing to "render the conversation in a pane." | MIT | **Reference — study the renderer** |
| **`codex app-server generate-json-schema`** | Emits version-matched schema from the installed binary — generate Rust types locally rather than trusting a stale third-party crate. | official | **Reuse if types wanted** |

Also worth reading, both **Rust + closest to lisa's stack**:
- **`ishefi/zellaude`** ([repo](https://github.com/ishefi/zellaude), MIT) — **useful pattern reference, not a scheduler.** ⚠️ Correction (see [07 §4](./07-ecosystem-viability.md)): zellaude is a Claude-Code-aware **status-bar widget** (~90★, stale-ish), *not* an orchestrator. But it is a Rust **Zellij WASM plugin** using lisa's exact wiring — *hook events → thin bash bridge → `zellij pipe` → in-memory plugin state* — so its detection/bridge path is worth studying for wiring a Codex wrapper's signals into the plugin. Study the pattern; don't expect a fork-able DAG engine.
- **`vibe-kanban`'s `executors` crate** ([repo](https://github.com/BloopAI/vibe-kanban), Apache-2.0, Rust) — subprocess/PTY + **stream-json normalization across 10+ agent CLIs** + process-exit status. The most directly reusable code for a multi-CLI, non-hook detection path (though the project is sunsetting — read it, don't depend on it).

### Tier 2 — Strategic: ACP as a unified client layer (bigger bet, both agents)

**Agent Client Protocol (ACP)** ([spec](https://agentclientprotocol.com), [repo](https://github.com/zed-industries/agent-client-protocol), Apache-2.0) is a JSON-RPC 2.0 "LSP for coding agents": `initialize` → `session/new`/`session/load` → `session/prompt` (returns a stop reason) → `session/update` (streaming message/tool-call/plan notifications) → tool-call permission requests → `session/cancel`. It is the one **verified, standards-track** abstraction that normalizes **both** of lisa's targets behind a single contract:

- **`claude-agent-acp`** ([repo](https://github.com/agentclientprotocol/claude-agent-acp), Apache-2.0) — mature, Anthropic-linked, ~2.2k★.
- **`codex-acp`** ([repo](https://github.com/agentclientprotocol/codex-acp), Apache-2.0) — wraps the Codex **app-server**; community-maintained, smaller/younger (~79★) — validate against the pinned Codex version.
- Reusable **Rust** libs: `agent-client-protocol` + `agent-client-protocol-schema` on crates.io — lisa could be an ACP *client* natively.

**Independent corroboration:** `mco-org/mco` (MCO/Hive) ([repo](https://github.com/mco-org/mco)) already drives Claude Code + Codex + others through a uniform adapter with exactly two transports — a **"shim"** (stdout parsing) and an **"acp"** (ACP JSON-RPC) transport. That's precisely the Tier-1-vs-Tier-2 choice lisa faces, already built — strong signal the design space is real and ACP is the clean path.

**The catch for lisa (the WASM constraint):** ACP adapters are subprocesses speaking stdio JSON-RPC. lisa can't host a resident ACP client *inside* the WASM plugin; it would run a **host-side ACP bridge** launched into a pane, relaying `session/update`/stop-reason to the plugin via signal files / `zellij pipe`. That's a substantially bigger component than a bash wrapper, and it changes lisa's driving model for **both** agents (not just Codex). Powerful — one contract for Claude *and* Codex, richer than hooks, future-proof — but **over-scoped for a hackathon Codex toggle** and it touches the currently-solid Claude path.

---

## Prior art worth studying (regardless of path)

From the orchestrator survey — liveness-detection patterns, ranked by relevance:

- **`ishefi/zellaude`** — Zellij WASM + hook-bridge over `zellij pipe`. lisa's exact stack. *(study first)*
- **`overstory`** ([repo](https://github.com/jayminwest/overstory), archived) — out-of-band structured `worker_done` message bus + tiered watchdog; the strongest "know it's done, don't guess from the screen" design — relevant to lisa's DAG scheduler.
- **`awslabs/cli-agent-orchestrator`** ([repo](https://github.com/awslabs/cli-agent-orchestrator), Apache-2.0, active) — MCP-callback completion keyed to a per-agent `CAO_TERMINAL_ID` env var across 9 CLIs; clean multi-CLI signaling model.
- **`codex-orchestrator`** ([repo](https://github.com/kingbootoshi/codex-orchestrator)) — `--no-alt-screen` + tail-scan + reading Codex's own `~/.codex/sessions/*.jsonl`; concrete Codex idle/done tricks.
- **`ntm`** ([repo](https://github.com/Dicklesworthstone/ntm)) & **`codex-yolo`** ([repo](https://github.com/codex-yolo/codex-yolo)) — the fallback scraping recipes (5-state machine + ANSI-strip; 8-prompt regex + cooldown) for whenever structured signals aren't available.

Liveness clusters into four families across the field: **(1) hooks → pipe/state-file** (all Zellij plugins; accurate but Claude-only), **(2) capture-pane scraping + regex/state-machine** (CLI-agnostic, fragile), **(3) out-of-band structured signaling** (mail bus / MCP callbacks / file claims), **(4) heuristic composites** (heartbeat + git-commit + lock + context-%). lisa today is family (1) for Claude; the Codex wrapper puts it in a robust variant of (3) — structured events from `exec --json` written to signal files.

**Ecosystem note:** no verified **Zellij-native DAG scheduler** exists — every other DAG/kanban orchestrator is web-UI (vibe-kanban, sunsetting) or tmux (overstory, archived). lisa occupies an open niche; the active neighbors are awslabs/CAO and the Zellij status plugins.

---

## Operational aside: `AGENTS.md`

`AGENTS.md` is now a Linux-Foundation standard that **Codex reads natively**, but **Claude Code still reads `CLAUDE.md`** (not AGENTS.md) as of mid-2026 ([ref](https://bestagent.dev/claude-md-vs-agents-md-2026/); Claude feature request open). Since lisa already generates `CLAUDE.md`, the cheap fix when spawning Codex is to also emit `AGENTS.md` (or symlink `AGENTS.md → CLAUDE.md`) so each agent finds its context file. Orthogonal to the client toggle, but needed for parity.

---

## Where the intel points

- **For the hackathon → Tier 1.** Hand-write the thin `exec --json` wrapper (schema from `@openai/codex-sdk`, shape from `takopi`, renderer from `codex-trace`), wire it into the plugin using `zellaude`'s hook-bridge-over-`zellij pipe` pattern. Fits lisa's WASM model with near-zero core change, keeps the Claude path untouched, and there's no Rust crate that would save the small parser effort anyway.
- **Tier 2 (ACP) is the deliberate future bet** — genuinely attractive as a *unified* Claude-and-Codex layer with a Rust crate and both adapters, and MCO proves the pattern — but it's a host-side-bridge rearchitecture that also disturbs the working Claude path. Note it in the epic as the strategic option; don't take it on for the toggle.
- **Study `zellaude` and `vibe-kanban/executors` first** regardless — they're the two Rust repos closest to the pieces lisa needs.

## Sources

Inline above. Primary: [ACP](https://agentclientprotocol.com) · [claude-agent-acp](https://github.com/agentclientprotocol/claude-agent-acp) · [codex-acp](https://github.com/agentclientprotocol/codex-acp) · [MCO](https://github.com/mco-org/mco) · [@openai/codex-sdk](https://www.npmjs.com/package/@openai/codex-sdk) · [takopi](https://github.com/banteg/takopi) · [codex-trace](https://github.com/PixelPaw-Labs/codex-trace) · [zellaude](https://github.com/ishefi/zellaude) · [vibe-kanban](https://github.com/BloopAI/vibe-kanban) · [overstory](https://github.com/jayminwest/overstory) · [awslabs/CAO](https://github.com/awslabs/cli-agent-orchestrator) · [codex-orchestrator](https://github.com/kingbootoshi/codex-orchestrator) · [ntm](https://github.com/Dicklesworthstone/ntm) · [codex-yolo](https://github.com/codex-yolo/codex-yolo).
