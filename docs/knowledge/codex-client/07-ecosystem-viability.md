# 07 · Ecosystem viability — traction, backing, philosophy

> Part of the **Codex client intel packet** (`docs/knowledge/codex-client/`). See [README](./README.md).
> Generated 2026-07-01 from three verified viability passes (standards/protocols, orchestrator traction+philosophy, big-player direction). GitHub metrics were pulled live from the REST API on 2026-07-01; adoption/funding figures from secondary sources are flagged. Cites inline.
> **Strategic intel, not an implementation plan.** Answers: what's gaining traction, what's niche, what philosophies are in play, and what has enough backing to keep on our radar.

## TL;DR

- **Build on the model-lab CLIs (Claude Code, Codex) and the Linux-Foundation interop stack.** MCP and AGENTS.md are foundation-governed safe bets. ACP is real but not yet foundation-governed and both big agent vendors keep it at arm's length — **watch, don't bet the architecture on it**.
- **The standalone-orchestrator middle is dying** (vibe-kanban/bloop shut down, uzi stale ~13mo, overstory & mco rebooted as early cloud/closed successors). Money and star-growth are flowing to **agent-native editors/terminals** (cmux, superset, gastown) and to **hooks-based extension ecosystems** (oh-my-codex).
- **The labs are absorbing orchestration** — Anthropic shipped experimental *agent teams* (file-locked task list + dependency resolution + mailbox), OpenAI shipped Codex *subagent delegation*. Both overlap lisa's core.
- **lisa's whitespace is real and largely unoccupied: DAG/dependency scheduling + hooks-based liveness in a terminal multiplexer.** oh-my-codex proves hooks are the right liveness bet; ntm/overstory/CAO prove protocol-coordination has demand; **no high-traction tool combines dependency-DAG scheduling with hook liveness.** Opportunity — and risk (few validators, no proven audience yet).
- **Anthropic explicitly leaves the Zellij niche open** (#31901 open, no maintainer response; #26572 pluggable pane backend). That's a defensible seam for lisa.

---

## 1. Standards / protocols — what's safe to build on

| Standard | Layer | Backing / governance | Momentum | Verdict |
|---|---|---|---|---|
| **MCP** | agent → tools | **Linux Foundation / AAIF** (donated by Anthropic, Dec 2025); every major vendor | Dominant tool standard (~10k+ servers; ~97M installs *[secondary]*) | **Safe bet** — but it's the *tool* layer, not orchestration |
| **AGENTS.md** | project context file | **Linux Foundation / AAIF** (donated by OpenAI) | 60k+ repos *[secondary]*; read by Codex/Cursor/Gemini/Copilot/VS Code | **Safe bet (low-cost)** — but Claude Code still defaults to `CLAUDE.md`; emit both |
| **ACP** (Agent Client Protocol) | editor/orchestrator → agent | **Zed + JetBrains** (interim two-vendor BDFL; *not* in a foundation) | Accelerating from a small base (~3.6k★, v1.17, 5-lang SDKs, ACP Registry Jan 2026); Google/Gemini native launch partner | **Emerging — watch, don't bet** |

**The ACP caveat that matters most for lisa:** ACP standardizes *editor↔agent*, and both of lisa's targets are **adapter-only, not first-party**:
- **Anthropic closed** the Claude Code ACP request **as "not planned"** (#6686); Claude Code works over ACP only via a Zed-maintained adapter.
- **OpenAI** connects via the community `codex-acp` bridge; no primary endorsement.

So adopting ACP as lisa's unified layer would put a **Zed-maintained-adapter dependency on the very Claude path that currently works**. Two triggers to re-evaluate: (a) ACP moves to a neutral foundation; (b) Anthropic or OpenAI ship *native* ACP. Until then, ACP is a documented future option, not a foundation to build on. *(This tempers [06](./06-off-the-shelf-tooling.md)'s Tier-2.)*

**Framing correction:** MCP vs ACP is not a competition — **MCP = agent-to-tools, ACP = editor-to-agent**; they compose. Neither is an *orchestration/scheduling* protocol (agent-to-agent is only on MCP's 2026 roadmap). lisa's DAG layer isn't something any of these standards provides.

---

## 2. Big-player direction — where the backing actually is

- **OpenAI / Codex — orchestration via its *own* protocol.** The **App Server** (JSON-RPC, backward-compatible) is the strategic centerpiece powering CLI/IDE/web; OpenAI *deliberately rejected MCP as the client-integration layer* (couldn't do streaming diffs/approvals). Native **subagent delegation** shipped. Interop: MCP for tools, ACP via community bridge only. Near-weekly releases; even an `/import` from Claude Code (migration play).
- **Anthropic / Claude Code — first-party orchestration, interop = MCP only.** Shipped **subagents** and experimental **agent teams** (peer instances sharing a *file-locked task list with dependency resolution* + mailbox, behind `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`). Platform play = **Claude Agent SDK → hosted Managed Agents**. No ACP. **Multiplexer support is tmux/iTerm2 only — Zellij explicitly unsupported; #31901 open with no maintainer response.**
- **Editors coalescing on a two-protocol stack:** MCP (tools, near-universal) + ACP (editor-hosts-agent). **JetBrains** is the strongest ACP backer (co-governs; Junie GA); **Google** pro-interop (Gemini CLI reference impl); **Microsoft/VS Code** is the ACP holdout (standardized agent mode on MCP; declined native ACP host), though Copilot CLI added ACP *outbound*.
- **Foundations — Linux Foundation / AAIF is the center of gravity** (Dec 2025; co-founded by Anthropic, Block, OpenAI; founding donations MCP, goose, AGENTS.md; A2A a separate LF project). Platinum: AWS, Anthropic, Block, Bloomberg, Cloudflare, Google, Microsoft, OpenAI.
- **Funding / consolidation — two poles, dying middle.** Mega-rounds/acquisitions cluster at the model labs and big coding-agent platforms (Cursor→SpaceX/xAI $60B *[press]*; Windsurf split across OpenAI/Google/Cognition; OpenAI bought Astral/uv). **Standalone orchestrators are the shutdown cohort:** bloop/Vibe Kanban and Terragon shut down; Conductor/Charlie tiny.

**Whose direction not to fight:** the lab CLIs (durable substrate) and the LF interop stack (MCP first, AGENTS.md, ACP for editor hosting). But assume the labs keep absorbing orchestration — so lisa's defensible value must be what they *won't* ship: the **Zellij/multiplexer backend Anthropic left open**, **cross-vendor scheduling across Claude *and* Codex**, and **DAG/dependency logic above any single lab's native teams**.

---

## 3. Orchestrator traction & philosophy (GitHub API, 2026-07-01)

| Tool | Stars | Status | Backing | Philosophy cluster |
|---|--:|---|---|---|
| **oh-my-codex / OMX** | 31.5k | **Active (breakout)** | solo, big community *(no LICENSE — legal risk)* | **Hooks-based** Codex extension/agent-teams |
| claude-flow / ruflo | 62k | Active (contested) | solo (rUv) | Swarm meta-harness, MCP; inflated-metrics skepticism |
| **vibe-kanban** | 27k | **Sunsetting** | bloop (company folded) | Kanban web-UI + worktree |
| **cmux** | 23k | Active (rising) | Manaflow (startup) | **Agent-native GUI terminal** + worktree |
| gastown | 16k | Active (rising) | S. Yegge → org | Flat workspace manager |
| **superset** | 12k | Active (rising) | startup | **Agent-native editor** + worktree |
| claude-squad | 8k | Active but **cooling** | smtg-ai | tmux session mgr + worktree + scraping |
| **awslabs CAO** | 780 | Active | **AWS Labs** | Supervisor/worker, **protocol/message-bus**, multi-CLI |
| ntm | 373 | Active | solo | tmux control plane, Agent Mail + REST/WS, graph triage |
| uzi | 579 | **Stale (~13mo)** | Devflow Inc (deprioritized) | **Fanout + pick-a-winner** |
| overstory → **Warren** | 1.3k→104 | **Archived → early successor** | solo | Supervisor/worker, SQLite msg-bus → cloud sandbox |
| mco → **Hive** | 421 | **Superseded** | solo (Hive = BSL-1.1) | Neutral protocol, supervisor/worker |
| codex-orchestrator | 318 | Active (slowing) | solo | Claude-delegates-to-Codex, tmux, scraping |
| ccswarm | 146 | Active (low) | solo (Rust) | Supervisor/worker, worktree |
| **zellaude** | 90 | Stale-ish | solo | **Status-bar widget, NOT an orchestrator** |
| codex-yolo | 16 | Negligible | solo | Parallel Codex tmux + scraping auto-approve |

**Design "religions" (the axes):**
- **Coordination:** fanout+pick-a-winner (uzi) · kanban (vibe-kanban) · supervisor/worker (CAO, overstory, mco/Hive, ccswarm, claude-flow) · **flat parallel workspaces + human coordination** (claude-squad, cmux, superset, gastown — the *dominant commercial* pattern) · **DAG/dependency scheduling — rare; lisa's niche.**
- **Isolation:** git-worktree is the settled default; **cloud-sandbox** is the emerging frontier (Warren, CAO-enterprise).
- **Liveness:** capture-pane **scraping** (pragmatic majority) · **hooks** (oh-my-codex, *lisa*) · **structured protocol/message-bus** (overstory, ntm, CAO, mco/Hive, claude-flow).
- **Surface:** terminal multiplexer (tmux/Zellij — *lisa*) → web UI → **purpose-built agent-native editor/terminal** (where money + star-growth are going).

**Who's winning / converging:** the market has converged on **flat parallel worktree workspaces with polished surfaces and human-driven coordination**; the surface is moving up-stack (tmux wrappers commoditizing → web → agent-native editors). oh-my-codex is the breakout by **riding Codex's growth as a hooks/extension ecosystem** rather than wrapping it. **True DAG scheduling + hooks liveness remains unoccupied at scale.**

---

## 4. What this means for lisa

1. **The Codex-client bet is well-aimed.** oh-my-codex's breakout validates lisa's **hooks/structured-signal liveness** philosophy over scraping, and confirms Codex is a growth substrate worth supporting.
2. **Keep the client layer hand-rolled + signal-file-normalized (Tier 1), not ACP (Tier 2), for now** — ACP's governance/vendor-commitment gap makes it a watch-item, and lisa's value isn't the transport, it's the scheduling.
3. **Lean into the whitespace, not the crowded surface.** Don't try to out-polish cmux/superset as an editor; double down on **DAG/dependency scheduling + cross-vendor (Claude *and* Codex) routing** — the thing the labs and the popular tools don't do. Per-pane provider/model routing (the new [E-001](../../active/epics/E-001-pluggable-agent-client.md) north star) sits *exactly* in this whitespace.
4. **Study the protocol-coordination minority for durability:** awslabs/CAO (institutional, message-bus) and overstory's archived "Agent Mail" + tiered-watchdog design are the best open references for liveness/coordination that survives a maintainer leaving.
5. **Corrections to earlier packet docs:** `zellaude` is a **status-bar widget, not an orchestrator** — still a useful reference for the *Zellij-WASM + hook-bridge-over-`zellij pipe`* wiring pattern (which is lisa's exact stack), but not a fork-worthy scheduler as [06](./06-off-the-shelf-tooling.md) implied. Note also `oh-my-codex` has **no license** (don't copy code) and `Hive` is **BSL-1.1**.
6. **Watch, don't adopt (radar):** oh-my-codex (hook API + agent-teams design), cmux/superset (UX bar), awslabs/CAO (durable coordination), Warren (cloud-sandbox leading indicator), and the two ACP triggers (foundation move; native lab support).

---

## Radar shortlist (confidence)

| Keep on radar | Why | Confidence |
|---|---|---|
| **MCP + AGENTS.md** | Foundation-governed interop; adopt AGENTS.md now (emit alongside CLAUDE.md) | High |
| **Codex App Server** | OpenAI's strategic centerpiece; the upgrade path if lisa ever needs live streaming/steering | High |
| **oh-my-codex** | Breakout validating hooks-liveness + ride-the-base-agent; watch its hook API *(unlicensed)* | High |
| **awslabs/CAO** | Institutional message-bus supervisor/worker; durable-coordination reference | Medium |
| **ACP** | Winning cross-editor standard, but governance + lab-commitment gap; re-eval on the two triggers | Medium |
| **cmux / superset / gastown** | The surface-quality bar users will compare lisa against | Medium |
| **Warren (+ overstory design)** | Leading indicator for the cloud-sandbox pivot | Low-Med |

## Sources

Inline above; GitHub REST API queried 2026-07-01 for all repos. Primary: [LF/AAIF press](https://www.linuxfoundation.org/press/linux-foundation-announces-the-formation-of-the-agentic-ai-foundation) · [ACP governance](https://agentclientprotocol.com/community/governance) · [Anthropic ACP #6686](https://github.com/anthropics/claude-code/issues/6686) · [Claude Zellij #31901](https://github.com/anthropics/claude-code/issues/31901) · [Codex App Server](https://developers.openai.com/codex/app-server) · [Anthropic agent teams](https://code.claude.com/docs/en/agent-teams) · [vibe-kanban #3408](https://github.com/BloopAI/vibe-kanban/issues/3408) · [oh-my-codex](https://github.com/Yeachan-Heo/oh-my-codex) · [cmux](https://github.com/manaflow-ai/cmux) · [superset](https://github.com/superset-sh/superset) · [awslabs CAO](https://github.com/awslabs/cli-agent-orchestrator) · [warren](https://github.com/jayminwest/warren).
