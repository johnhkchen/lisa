# Codex client intel packet

Research + design intel for **E-001 — adding OpenAI's Codex CLI and growing toward
per-pane provider/model routing** in `lisa loop`.

> ⭐ **START HERE: [08 · Design thesis & target architecture](./08-design-thesis.md)** —
> the *goal* this whole packet serves. Docs 01–07 are the research trail; doc 08 is
> the north star (per-pane `(provider, model)` routing + execution provenance, over a
> three-leg adapter portfolio: native Claude Code + native Codex + ACP). If you are
> overseeing implementation, read 08, then the [epic](../../active/epics/E-001-pluggable-agent-client.md).

**Status:** intel + design goal. Docs 01–07 contain no implementation plan (the
*how* is left to the epic's tickets and their RDSPI cycles); doc 08 states the
*what* and *why*.

> **Implementation update (2026-07-09):** the wrapper recommendation below is
> retained as the historical decision record, but it no longer describes Lisa's
> default Codex path. A fresh spike against Codex 0.144.0 verified the native TUI
> contract that the original 0.142.5 research treated as too risky: positional
> prompt launch, inherited `LISA_PANE_ID` in Stop/SessionStart hooks, `/clear`
> delivery, and delayed text-then-Enter follow-ups. Lisa now launches the native
> TUI and keeps `agent-exec` as a headless fallback. See
> [09 · Native-TUI parity decision](./09-native-tui-parity.md).

> **Implementation update (2026-07-18, 0.4.4-rc.8):** the `/clear` session-reuse
> handshake these docs describe as Lisa's current Claude coupling (01 §7–§8, 05,
> and the reuse mentions in 09) is no longer how either provider hands a pane
> between tickets. Both native clients now end their session after each ticket —
> `/exit`, a bounded grace, then a fresh launch — because in-place reuse kept the
> launch line's `LISA_TICKET_ID`/`LISA_ATTEMPT_ID` environment alive across
> tickets, leaving every ticket after the first under a stale identity. The
> `/clear` handshake text below stands as the historical record of the surface
> that research mapped.

Compiled 2026-07-01 from a multi-agent sweep: 5 readers over the lisa repo, 5 web
researchers over official Codex docs + `openai/codex` issues, 6 adversarial
verifiers on the highest-risk correspondences, and a synthesis pass. Codex facts
are pinned to stable **`rust-v0.142.5`**.

## Read in this order

| Doc | What it gives you |
|---|---|
| [01 · lisa ↔ Claude Code coupling](./01-lisa-claude-coupling.md) | Every load-bearing Claude-Code touchpoint in lisa, with exact `file:line`. The surface a second client must satisfy. |
| [02 · Codex CLI capabilities](./02-codex-capabilities.md) | Reference intel on what Codex actually exposes: hooks, `codex exec`, lifecycle/slash-commands, flags, `config.toml`, `AGENTS.md`, version drift. Confidence-tagged. |
| [03 · Mechanism mapping](./03-mechanism-mapping.md) | **The crown jewel.** Each lisa need → closest Codex equivalent → fit (1:1 / shim / partial / gap) → parity risk. Start with its executive table. |
| [04 · Risks & open questions](./04-risks-and-open-questions.md) | The 6 adversarially-verified claims (3 refuted, 2 confirmed, 1 uncertain) and what a spike still must settle. |
| [05 · Bridging the discrepancy](./05-bridging-the-discrepancy.md) | **The resolution.** The mechanisms available to close the gap (`codex exec --json` wrapper, app-server, `notify`, quiescence), ranked with trade-offs. |
| [06 · Off-the-shelf tooling](./06-off-the-shelf-tooling.md) | What we can **reuse**: SDKs/wrappers/renderers, orchestrator prior art, and **ACP** as a possible unified Claude+Codex client layer. Tactical vs. strategic tiers. |
| [07 · Ecosystem viability](./07-ecosystem-viability.md) | **Strategic read.** Traction/backing/philosophy: what's a safe bet (MCP, AGENTS.md), the dying orchestrator middle, and lisa's DAG+hooks whitespace. |
| [08 · Design thesis & target architecture](./08-design-thesis.md) | ⭐ **The goal.** The pain, the moat, the bet, and the three-leg adapter portfolio (native Claude + native Codex + ACP) + graduation rule. Read first. |
| [09 · Native-TUI parity decision](./09-native-tui-parity.md) | **The 2026-07-09 reversal.** Root cause of the JSON-pane disparity, why the wrapper was originally chosen, live 0.144.0 evidence, and the remaining asymmetries. |

> **ACP note (updated by later research):** docs 06/07 rated ACP "watch, don't bet."
> The capability/trajectory deep-dive **upgrades** it — headless-viable today, with
> **stable uniform `usage_update` cost reporting**, and model-selection/cost/remote-transport
> gaps already shipped or merged. ACP is now the **third primary method** (breadth leg),
> with native Claude/Codex as the depth anchors. Governance (no foundation yet) and
> vendor posture (Anthropic/OpenAI adapter-only) remain the caveats. See [08 §5](./08-design-thesis.md).

Related planning doc: [`../../active/epics/E-001-pluggable-agent-client.md`](../../active/epics/E-001-pluggable-agent-client.md).

## The one-paragraph takeaway

lisa drives Claude Code by **typing into a live TUI pane** and reacting to
**signal files that Claude Code hooks write** on lifecycle events. The binary
swap is trivial (one function, `build_claude_command`), but the two pillars that
matter — *driving the TUI by keystroke injection* and *the interactive hook
signal contract* — **do not cleanly survive the port to Codex**. Verification
**refuted** reliable TUI keystroke injection (Codex's paste-burst heuristic),
**refuted** reliable interactive `Stop`/`PostToolUse` hook delivery (issue
#17532; no heartbeat cadence), and left env-based pane↔hook correlation
**undocumented**. Codex's own supported automation surface is the **headless
`codex exec` + `--json` + `resume`** model, not TUI driving. So "full parity"
via lisa's current architecture is not a given — the central open question the
epic must resolve is **interactive-TUI-pane driving vs. a `codex exec` redesign**,
and that is exactly what the gating spike exists to answer.

**But the discrepancy is bridgeable** — see [05](./05-bridging-the-discrepancy.md). The
fix is to change *who emits lisa's signal files*: a thin wrapper lisa launches
(inheriting `LISA_PANE_ID`) translates Codex's machine-readable `codex exec --json`
event stream into the same signal files lisa already consumes — no hooks, no TUI
scraping, deterministic pane attribution. The autonomous headless path also
*dissolves* the two hardest signals (`.idle`/`.awaiting` never occur when Codex
doesn't pause for a human; `.cleared` is moot when reuse is a fresh `exec`), so
the surface lisa needs from Codex collapses to `.heartbeat` / `.stopped` /
`.error`. The scheduler that consumes the signals stays unchanged.

See [03 §3 genuine gaps](./03-mechanism-mapping.md) and [04](./04-risks-and-open-questions.md)
for the problem, and [05](./05-bridging-the-discrepancy.md) for the resolution options.

## Confidence & freshness caveats

- Codex's **hooks subsystem is the most version-volatile surface**; every
  hook-dependent mapping inherits that risk. Re-verify against the installed
  Codex version before relying on any hook payload/field name.
- Confidence tags (`[H]/[M]/[L]`) are carried inline in docs 02 and 03.
- Facts are current as of **2026-07-01 / Codex `rust-v0.142.5`**.
