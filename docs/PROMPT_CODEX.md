# PROMPT_CODEX.md — handoff prompt: understand the Codex / multi-provider task

Paste the block below into a capable agent to have it deeply understand (and
critically pressure-test) the Codex + per-pane-routing task before any
implementation. It is self-contained — no prior conversation context required.

Authoritative goal doc: [`knowledge/codex-client/08-design-thesis.md`](./knowledge/codex-client/08-design-thesis.md).
Epic: [`active/epics/E-001-pluggable-agent-client.md`](./active/epics/E-001-pluggable-agent-client.md).

---

```
You are taking over a design-stage task on "lisa" and your first job is to
UNDERSTAND it deeply and critically — not to write implementation code yet.

## What lisa is
lisa is a Zellij WASM plugin (Rust) that does DAG-driven concurrent task
scheduling for the RDSPI workflow: it manages Claude Code sessions — spawning,
tracking, and scheduling them based on ticket dependencies — and carries between
projects as a single .wasm with zero project-specific deps.

Repo: /Users/johnchen/swe/repos/lisa  (branch: main; the docs below are
uncommitted working-tree changes). Build/test per CLAUDE.md:
  cargo build -p lisa-plugin --target wasm32-wasip1 --release
  cargo build -p lisa-cli --release
  cargo test --workspace   (or: just check)

## The task
Over several sessions we researched and designed adding OpenAI's Codex CLI as a
second agent client, and the goal has since grown into a north star:
**per-pane (provider, model) routing + execution provenance, delivered over a
three-leg adapter portfolio — native Claude Code + native Codex + ACP — behind
one normalized signal contract.** Your job is to absorb this, verify it against
the real codebase, pressure-test it, and surface what's underspecified — so you
(or the next agent) can plan implementation with confidence.

## Read, in this order (these are the authority)
1. docs/knowledge/codex-client/08-design-thesis.md  <- the GOAL. Start here.
2. docs/active/epics/E-001-pluggable-agent-client.md <- the needs/stories (S-021->S-027).
3. docs/knowledge/codex-client/README.md            <- index of the intel packet.
4. Then packet docs 01-07 as needed:
   01 = how lisa couples to Claude Code today (with file:line)
   02 = Codex CLI capabilities
   03 = mechanism mapping (lisa need -> Codex equivalent -> fit -> risk)
   04 = adversarially-verified risk ledger
   05 = how the discrepancy is bridged (codex exec --json wrapper, etc.)
   06 = off-the-shelf tooling we can reuse
   07 = ecosystem viability (traction/backing/philosophy)

## Verify against real code — don't trust the docs blindly
Doc 01 anchors every claim to file:line. Confirm the load-bearing ones yourself,
especially: crates/lisa-plugin/src/lib.rs (build_claude_command ~L53, the /clear
reuse handshake ~L555-620, ENTER_DELAY_SECS ~L83, the signal-file consumption),
the .lisa/hooks/ scripts + how .claude/settings.local.json wires them,
crates/lisa-core/src/types.rs PluginConfig, crates/lisa-cli/src/doctor.rs, and
crates/lisa-cli/src/templates.rs. The RDSPI workflow is docs/knowledge/rdspi-workflow.md.

## Locked decisions — understand them; do NOT re-open unless you find them broken
- North star = per-pane (provider, model) routing + execution provenance (S-026/S-027).
- MVP first: a whole-loop Codex toggle may ship first, but the client seam MUST be
  per-pane-resolvable from day one (no whole-loop-only assumption baked in).
- Three integration methods behind ONE adapter interface: native Claude Code (hooks),
  native Codex (codex exec --json wrapper), and ACP. All adapters emit the SAME
  normalized signals (.heartbeat/.stopped/.error + usage/cost); scheduler stays
  client-agnostic. This preserves the artifact-driven core untouched.
- Graduation rule: breadth comes from ACP; a provider earns a NATIVE adapter only
  when run at sustained volume where lag/precision/reliability bites. Native is
  earned by usage, not added by default.
- The "drive a live TUI by keystroke injection + interactive hooks" path was
  REFUTED for Codex (doc 04); the resolution is a host-side wrapper that translates
  codex exec --json into lisa's signal files (doc 05).
- Two explicit bets (doc 08): the model market stays plural; the interop coalition
  (ACP/MCP/AGENTS.md) persists. The architecture must survive either bet failing.

## Known open questions (already logged in the epic; sharpen, don't rediscover)
Hook->pane attribution & env inheritance; interactive-hook reliability (#17532);
codex exec --json fidelity/edge cases; routing frontmatter schema; provenance
metric fidelity (what cost/quality signals are actually obtainable); behavior at
~16 concurrent mixed-provider agents; ACP headless/concurrency at scale +
model-selection maturity.

## Deliverable (demonstrate understanding; no implementation code yet)
1. A concise restatement, in your own words, of the goal and why lisa is uniquely
   positioned to deliver it (the moat) — proving you grasp the intent, not just the mechanics.
2. A verification pass: which of doc 01's key code claims you confirmed, and any
   drift between the docs and the actual code.
3. A critical read: challenge the two bets and the three-leg architecture. Where is
   the thesis strong, where is it fragile or underspecified, what would you push back on?
4. The riskiest unknowns ranked, and the cheapest experiments (the S-021 spike) to
   retire them — e.g. does a Codex hook/wrapper child inherit LISA_PANE_ID; does
   codex exec --json render usably in a pane; ACP headless concurrency.
5. A proposed decomposition from MVP -> north star (sequence of shippable increments),
   staying at the "what/why" altitude — flag anything you'd want a human decision on
   before planning the "how".

Work from evidence: read the code and the docs, verify claims, cite file:line and
doc sections. If something in the docs is wrong, say so plainly.
```
