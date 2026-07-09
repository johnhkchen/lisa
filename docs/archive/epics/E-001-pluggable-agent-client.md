---
id: E-001
title: pluggable-agent-client
status: open
priority: high
stories: [S-021, S-022, S-023, S-024, S-025, S-026, S-027, S-028]
---

# E-001: Pluggable Agent Client — add Codex, grow toward per-pane provider/model routing

> **Archived 2026-07-09.** All build waves (0–5) shipped; the one outstanding
> piece — live validation against a real `codex` binary (S-028) — is carried
> forward by **S-029 / T-029-01** on the active board
> (`docs/active/stories/S-029-codex-integration.md`).

> ⭐ **PRIORITY / NORTH STAR.** The goal, rationale, and target architecture are in
> **[08 · Design thesis & target architecture](../../knowledge/codex-client/08-design-thesis.md)
> — read it first.** This epic is the *what*; the thesis is the *why*; the RDSPI
> cycle is the *how*. Every story below serves the thesis: **per-pane
> `(provider, model)` routing + execution provenance, over a three-leg adapter
> portfolio (native Claude Code + native Codex + ACP) behind one normalized
> signal contract.**

## Intent

Today `lisa loop` drives the Anthropic **Claude Code** CLI exclusively. The
near-term need is the ability to **toggle the agent client to OpenAI's Codex CLI**
for a hackathon centred on Codex, without disturbing the existing Claude-based
workflow.

The **north star** (product direction, confirmed) is bigger: **per-pane
provider + model routing** — route one ticket to Codex/ChatGPT, another to
Claude/Opus, two more to Claude/Sonnet, within a single loop — plus **execution
provenance** so lisa can later answer *which routing policies actually yield the
best results, which are most cost-effective, and which survive extreme
concurrency (e.g. 16 agents bursting a well-planned task).* The Codex toggle is
the **MVP first increment** toward that; the client seam must be built
per-pane-resolvable from day one so routing is a later addition, not a repaint.

This document is **intel + needs only**. It states *what must be true* for Codex
to be a first-class client, and captures the research that establishes what is
feasible. It intentionally does **not** prescribe an implementation approach —
the how is left to each ticket's RDSPI cycle.

> **Detailed intel lives in the [Codex client intel packet](../../knowledge/codex-client/README.md)**
> (coupling map, Codex capability reference, mechanism mapping, verified-claim
> ledger). The Intel A/B/C sections below are the summary; the packet is the
> authority. **Key finding from the deeper research:** lisa's current
> "drive a live TUI pane by typing + react to interactive hook signals"
> architecture does **not** cleanly survive the port to Codex — verification
> refuted reliable TUI keystroke injection and reliable interactive hook
> delivery. Codex's supported automation path is headless `codex exec`. So the
> central decision the spike (S-021) must settle is **interactive-pane driving
> vs. a `codex exec` redesign**; "full parity via the current architecture" is
> not assumed.
>
> **Resolution path exists** (see [05 · Bridging the discrepancy](../../knowledge/codex-client/05-bridging-the-discrepancy.md)):
> change *who emits lisa's signal files* — a thin wrapper lisa launches (inheriting
> `LISA_PANE_ID`) translates Codex's `codex exec --json` event stream into the same
> signal files the scheduler already consumes. Autonomous headless execution also
> dissolves `.idle`/`.awaiting` (Codex never pauses for a human) and `.cleared`
> (reuse = fresh `exec`), collapsing the needed signal surface to `.heartbeat` /
> `.stopped` / `.error`. This reframes S-021 from "is parity possible?" to
> "confirm the wrapper mechanics on the pinned Codex version."

## Constraints & non-goals (product decisions already made)

- **Full parity is the bar.** A Codex loop must get the same treatment a Claude
  loop gets today: session reuse, stuck/liveness detection, and automatic phase
  progression — not a degraded "launch-and-hope" mode.
- **Whole-loop toggle is the MVP; per-pane routing is the north star.** The
  first shipped increment may select the client once per `lisa loop` run, but the
  client abstraction **must resolve `(provider, model)` per ticket at spawn
  time** (not thread one choice through the loop), so per-pane routing is a
  config/UI addition later rather than a rearchitecture. *(This supersedes the
  earlier "per-ticket client is out of scope" decision.)*
- **Three integration methods, one adapter interface, one signal contract.**
  lisa integrates via an **adapter interface**, not a hardcoded provider set. The
  three primary methods are **native Claude Code (hooks)**, **native Codex
  (`exec --json` wrapper)**, and **ACP** (breadth for the long tail + the
  open-model on-ramp). All adapters emit the **same normalized signals**
  (`.heartbeat`/`.stopped`/`.error` + usage/cost) so the scheduler stays
  client-agnostic (3× adapter code, 1× scheduler). Model-level selection within a
  provider is a first-class goal. *(This supersedes the earlier "exactly two
  providers, no N-provider framework" guardrail — see
  [08 · Design thesis §5](../../knowledge/codex-client/08-design-thesis.md).)*
- **Graduation rule (anti-over-engineering guardrail, reborn).** Breadth comes
  from ACP; a provider earns a **native** adapter only when run at sustained
  volume where adapter-lag, per-turn cost precision, or reliability actually
  bites. Native is **earned by usage, not added by default** — so lisa doesn't
  balloon into a bespoke shim per vendor.
- **Zero regression for existing projects.** Claude Code stays the default. A
  project that never opts in must behave exactly as it does today — same
  commands, same signals, same docs.
- **The artifact-driven core stays untouched.** DAG computation, phase
  detection, and scheduling already react only to files written under
  `docs/active/work/`. That contract ("agent writes artifacts; lisa reacts") is
  client-agnostic and must remain so.

---

## Intel A — How lisa is coupled to Claude Code today

Grounded in the current code, the coupling lives in a small number of places.
This is the surface any second client has to satisfy.

| Concern | Where | Current Claude-specific behaviour |
|---|---|---|
| Launch command | `crates/lisa-plugin/src/lib.rs:53` (`build_claude_command`) | `LISA_PANE_ID=… LISA_TICKET_ID=… claude --dangerously-skip-permissions "<prompt>"` — the **only** place the binary + flags are hard-wired |
| Permission bypass | same | `--dangerously-skip-permissions` |
| Session reuse | `lib.rs:570` | Types the `/clear` slash command into the live TUI pane, then waits for a `.cleared` signal before sending the next prompt |
| Input injection | `ENTER_DELAY_SECS`, `lib.rs:83` | Types characters into the pane TUI, then presses Enter after a tuned delay |
| Liveness / stop / idle | `.lisa/hooks/*.sh` (`on-stop`, `on-clear`, `on-idle`, `on-heartbeat`) wired via `.claude/settings.local.json` | Claude Code **hooks** fire on lifecycle events and write signal files (`pane-N.stopped`, `.cleared`, heartbeats). The scheduler reads these files to drive transitions, detect stuck sessions, and gate reuse |
| Pane identity | env var | `LISA_PANE_ID` is set at launch and read back by the hook scripts to name the signal file for the right pane |
| Dependency check | `crates/lisa-cli/src/doctor.rs:87` | `doctor` checks `claude --version` |
| Project context | prompt + `templates.rs` | Prompt tells the agent to read `CLAUDE.md`; lisa generates `CLAUDE.md` |

**Takeaway:** launching a different binary is trivial (one function). The real
substance is the **signal/hook nervous system** — lisa doesn't watch the process,
it watches files that the agent's hooks emit. A second client is only "full
parity" if it can emit the same signals.

---

## Intel B — Codex CLI capabilities (research, July 2026)

Findings from official OpenAI Codex docs (`developers.openai.com/codex`) and the
`openai/codex` repo. Confidence noted per item.

1. **Hooks system exists and is close to Claude Code's.** *(High)* Codex loads
   lifecycle hooks from `hooks.json` or inline `[hooks]` tables in
   `config.toml`. Documented events include **`Stop`** (turn complete — the
   signal a scheduler needs), **`SessionStart`** (with a `source` of `startup` /
   `resume` / `clear`), `PreToolUse`, `PostToolUse`, `PermissionRequest`,
   `UserPromptSubmit`, `PreCompact`/`PostCompact`. Each hook runs a shell command
   that receives a **JSON payload on stdin** and may return JSON on stdout.
   - There is **no dedicated `idle` event**; `Stop` is the turn boundary and
     `PermissionRequest` is the approval-waiting signal.
   - Hooks have a **trust model**; `--dangerously-bypass-hook-trust` runs them
     without stored trust for that invocation. *(Medium on exact field names.)*
   - Known risk: issue **#17532** reports repo-local `.codex/config.toml` hooks
     may not fire in interactive sessions on some versions. **Must be validated
     on our pinned version.**
2. **Lightweight `notify` also exists** *(High)* — `notify = [...]` in config
   fires on **`agent-turn-complete` only**, passing a single JSON arg. Weaker
   than the hooks system; hooks are preferred for parity.
3. **`/clear` equivalent exists.** *(High)* In an interactive session `/clear`
   (and `/new`) reset context without restarting the process — a direct analog.
   Caveat: `/clear` is **disabled while a task is running**; reset only happens
   between turns (which matches how lisa already sequences reuse).
4. **Headless mode exists** *(High)* — `codex exec "<prompt>"` runs
   non-interactively and exits; `codex exec resume --last`/`resume <ID>` continues
   a session. This is an *alternative* driving model to the interactive TUI pane.
5. **Permission/sandbox bypass** *(High)* — `--dangerously-bypass-approvals-and-sandbox`
   (alias `--yolo`) is the analog to Claude's `--dangerously-skip-permissions`;
   `--sandbox danger-full-access` and `--ask-for-approval never` are the granular
   equivalents.
6. **Initial prompt on launch** *(High)* — `codex "<prompt>"` (interactive,
   pre-filled) or `codex exec "<prompt>"` (headless).
7. **Project context file is `AGENTS.md`, not `CLAUDE.md`.** *(High)* Codex
   auto-loads `AGENTS.md` walking root→cwd. An explicitly-named file in the
   prompt is still read regardless.
8. **TUI keystroke-injection caveats** *(Medium)* — driving the TUI by injecting
   keys works but is sensitive to paste framing and submit timing; sending text
   and Enter separately is the community-recommended pattern (which lisa already
   does). Headless `exec` avoids TUI fragility entirely but is a different
   lifecycle.

---

## Intel C — Parity map (Claude mechanism → Codex equivalent)

| lisa need | Claude Code | Codex equivalent | Parity risk |
|---|---|---|---|
| Launch w/ prompt | `claude "<p>"` | `codex "<p>"` (TUI) or `codex exec "<p>"` (headless) | Low |
| Bypass permissions | `--dangerously-skip-permissions` | `--dangerously-bypass-approvals-and-sandbox` | Low |
| "Turn complete" signal | Stop hook → `on-stop.sh` → `.stopped` | `Stop` hook → stdin JSON → write `.stopped` | **Medium** (hook wiring differs; stdin JSON vs env; #17532) |
| Context reset for reuse | type `/clear`, await `.cleared` | type `/clear`; `SessionStart` source=`clear` → `.cleared` | Medium |
| Heartbeat / liveness | `on-heartbeat.sh` | `PostToolUse` hook as heartbeat proxy | Medium (no native heartbeat) |
| Idle / awaiting-input | `on-idle.sh` | No `idle`; `PermissionRequest` only (and moot under `--yolo`) | **Open question** |
| Pane identity in hook | `LISA_PANE_ID` env inherited by hook process | Depends on Codex hook subprocess inheriting launch env | **Open question** |
| Project context | `CLAUDE.md` | `AGENTS.md` | Low |
| Dependency check | `claude --version` | `codex --version` | Low |

---

## Needs (organised as candidate stories)

Each item is a **requirement**, not a task list. Acceptance is "this is true,"
not "these steps were done."

### S-021 — Spike: prove Codex full-parity control surface *(gates the rest)*
The team needs confidence, before committing to a client design, that Codex can
satisfy lisa's signal contract on our pinned Codex version. Needs answered:
- Whether a Codex `Stop` hook reliably fires at turn completion in an
  interactive pane session (re: issue #17532), and can write a signal file lisa
  reads.
- Whether the hook subprocess can identify **which pane** it belongs to (env
  inheritance of `LISA_PANE_ID`, or an alternative correlation key such as
  `cwd`/`session_id` from the stdin payload).
- Whether `/clear` + `SessionStart(source=clear)` round-trips a `.cleared`
  signal for session reuse.
- Whether a `PostToolUse` (or other) hook is an adequate heartbeat/liveness
  proxy, and what — if anything — fills the missing `idle`/awaiting role.
- A recommendation on driving model (**interactive TUI pane** vs **`codex exec`
  + resume**) with the trade-offs for parity, recorded as the decision input for
  S-023.

### S-022 — Adapter interface & per-pane-resolvable selection (no behaviour change)
lisa needs a single **adapter interface** where each integration method supplies
its own launch, reuse, permission-bypass, and signal behaviour — the seam that
makes the three-leg portfolio and per-pane routing possible. Needs:
- An **adapter interface** (not a two-variant enum): native Claude Code is the
  first adapter; the interface must accommodate native Codex and an ACP adapter
  without redesign.
- Selection resolves **`(method, provider, model)` per ticket at spawn** (the MVP
  may set it loop-wide, but the seam must be per-pane-resolvable from day one —
  no whole-loop-only assumption baked in).
- All adapters converge on the **same normalized signal contract**
  (`.heartbeat`/`.stopped`/`.error` + usage/cost) so the scheduler is unchanged.
- With no opt-in, the resolved adapter is native Claude and **every existing
  behaviour is byte-for-byte unchanged** (provable as a no-op refactor).

> **Build order** (see [06 · Off-the-shelf tooling](../../knowledge/codex-client/06-off-the-shelf-tooling.md)
> and [08 · Design thesis §5](../../knowledge/codex-client/08-design-thesis.md)):
> the tactical Codex adapter is a host-side **`codex exec --json` wrapper** writing
> lisa's signal files (schema from `@openai/codex-sdk`, shape from `takopi`,
> renderer from `codex-trace`, wiring from `ishefi/zellaude`). **ACP** is the third
> primary method (breadth + uniform `usage_update` cost + open-model on-ramp),
> reached via `claude-agent-acp` / `codex-acp` as a host-side bridge. Native and
> ACP are **complementary legs**, not alternatives — natives for depth/reliability
> on the flagships, ACP for breadth; each is a fallback for the others.

### S-023 — Codex client: launch & session lifecycle parity
With the driving model chosen in S-021, Codex sessions need to launch, reset, and
reuse panes to the same standard as Claude. Needs:
- Codex launches per ticket with the correct prompt and permission-bypass flag.
- Session reuse resets context between tickets without restarting the process (or
  the S-021-approved equivalent).
- Input injection into a Codex session is reliable (no premature submit / paste
  corruption).

### S-024 — Codex signal & hook parity
Codex sessions need to feed the same scheduler signals Claude sessions do. Needs:
- Turn-complete, context-cleared, and liveness/heartbeat signals reach lisa in
  the format the scheduler already consumes, correctly attributed per pane.
- The awaiting/attention state has a defined behaviour for Codex (even if that
  definition is "not applicable under full-auto"), so the UI/scheduler never
  misreads a Codex pane.
- Hook setup for Codex is generated/guided by lisa the way `.claude/` hook setup
  is today, including any trust-bypass required for unattended runs.

### S-025 — Config toggle, environment doctoring, and docs
Users need a discoverable, safe way to opt into Codex and to know their
environment is ready. Needs:
- A documented way to select the client (e.g. a `.lisa.toml` field and/or a
  `lisa loop` flag), defaulting to Claude.
- `lisa doctor` checks the dependencies for the **selected** client (Codex's CLI
  when Codex is chosen) instead of unconditionally requiring `claude`.
- Codex projects get the right project-context file (`AGENTS.md`) with equivalent
  content to the `CLAUDE.md` lisa generates today.
- README / setup guide document the toggle, the Codex prerequisites, and the hook
  setup, without implying support for clients beyond these two.

### S-026 — Per-pane provider + model routing (the north star)
Building on the per-pane-resolvable seam from S-022, lisa needs to run different
tickets on different `(provider, model)` combinations within one loop. Needs:
- Each ticket resolves to a `(provider, model)` at spawn; different panes in the
  same loop can run different combinations concurrently (e.g. Codex/gpt-x,
  Claude/opus, Claude/sonnet×2).
- The routing decision is expressed via **ticket frontmatter** (e.g.
  `agent:` / `model:`) **and** a **loop-level default with per-ticket override**
  (confirmed product decision). Policy-based routing (by ticket type or RDSPI
  phase) is a **later** evolution, explicitly not required now.
- Heterogeneous panes must not confuse the scheduler — signal consumption is
  already client-agnostic (normalized signal files), so this is a spawn-time
  concern; the dashboard should surface each pane's `(provider, model)`.
- Concurrency/quota is provider-aware enough that mixing providers doesn't
  silently break (separate auth/rate-limit pools); handling extreme bursts (e.g.
  16 concurrent agents) is an explicit stress target.

### S-027 — Execution provenance & routing-policy telemetry
lisa needs to **record what actually happened** so routing policies can be
evaluated empirically. Needs:
- After a ticket completes, its **frontmatter (or a work artifact) captures which
  `(provider, model)` executed it** — and, where feasible, cost/token, wall-clock,
  concurrency-at-run, and outcome/quality signals.
- The captured data is structured enough to answer, across many runs: *which
  routing policies yield the best results, which are most cost-effective, and which
  hold up under extreme concurrency.*
- Provenance capture must not perturb the run itself (write-after, not
  write-during in a way that races the agent) and must respect the "don't touch
  the agent's own frontmatter phase/status fields" rule the prompt enforces.

> **Ecosystem fit** (see [07 · Ecosystem viability](../../knowledge/codex-client/07-ecosystem-viability.md)):
> per-pane cross-vendor routing + provenance sits squarely in lisa's **whitespace** —
> DAG/dependency scheduling with hooks-based liveness is largely unoccupied, and the
> labs' own orchestration (Anthropic agent teams, Codex subagents) is single-vendor.
> Cross-*vendor* routing with policy telemetry is a thing they are unlikely to ship.

---

## Decisions (2026-07-01, post-review — answers to the planning questions)

Recorded after a verification pass over docs 01–08 and the code. These are
product decisions; the *how* stays with each ticket's RDSPI cycle.

1. **Pane rendering bar: chunked output is acceptable.** No token-by-token
   streaming requirement, so `codex exec --json` + a chunked in-pane view
   suffices; the app-server (doc 05 Option 2) is not needed for this epic.
   The spike still picks tee-stderr vs. render-from-JSON on the pinned version.
2. **Provenance lives in an append-only JSONL ledger, not ticket frontmatter.**
   `.lisa/provenance.jsonl` (lisa-owned state dir; only `signals/` is
   gitignored, so the ledger is committable learning data). One record
   appended per ticket-run at completion (write-after): ticket id,
   `(method, provider, model)`, started/ended, wall-clock, tokens/cost where
   obtainable, concurrency-at-run, outcome. Rationale: respects the
   don't-touch-frontmatter rule, supports multiple runs per ticket
   (retries/resets) via append semantics, never races the agent, and stays
   queryable across many runs with standard tools. A human-readable mirror in
   `docs/active/work/<ticket>/` may be added later; the ledger is the source
   of truth.
3. **Invalid/unavailable `(provider, model)` at spawn: fall back to the loop
   default** (never fail the ticket), and surface the substitution in the
   dashboard and in the provenance record (record what actually ran, plus the
   requested route so fallbacks are visible in the data).
4. **Sequencing: Codex first, adapter-shaped.** Ship the native Codex leg
   before any ACP work, but S-022's adapter interface is designed so an ACP
   adapter is an addition, not a redesign. The interface must own the pieces
   that differ per method — launch command, session reuse/reset, follow-up
   injection (the `finish_up_prompt` path has no live-TUI analog under
   `codex exec`; its equivalent is `codex exec resume`), expected signal set
   (`.idle`/`.awaiting` are Claude-only), and usage/cost extraction — while
   the scheduler consumes only the normalized signals. The scheduler also
   needs the (currently nonexistent) `.error` consumer as part of this work.
5. **Wrapper distribution: a `lisa` subcommand, not a generated script.** The
   `codex exec --json` wrapper ships inside the existing `lisa-cli` binary
   (e.g. a hidden `lisa agent-exec` subcommand the plugin launches into the
   pane). Grounds: (a) no published Rust crate parses `exec --json` — the
   parser is hand-written either way (doc 06), and JSONL parsing + rendering
   in generated POSIX `sh` would add a `jq` dependency and real fragility;
   (b) generated scripts drift — the repo's own `.lisa/hooks/` is already a
   stale generation vs. `templates.rs`, which is the failure mode a
   compiled-in wrapper cannot have; (c) versioning is atomic: the wrapper,
   plugin, and CLI ship as one artifact via the existing embed pattern
   (`build.rs` → `PLUGIN_WASM`), so signal semantics can never skew across
   components; (d) `lisa loop` execs zellij itself, so it can pass its own
   absolute binary path (`std::env::current_exe()`) into the plugin config —
   the pane invocation is deterministic with no PATH assumption; (e) prior
   art agrees — mco, awslabs/CAO, and takopi all ship adapters inside the
   orchestrator binary rather than as loose scripts (doc 06). Claude's hook
   scripts stay as they are (they run *inside* Claude Code's hook system,
   which a subcommand cannot replace).

> **Post-implementation note (2026-07-01):** S-021→S-027 shipped without a live
> `codex` binary on the host — all empirical verdicts are `[PROVISIONAL]` and
> the live-loop checklist is unrun. **S-028 (codex-live-validation)** holds the
> deferred empirical half as `status: blocked` tickets; start at
> [`docs/knowledge/codex-day-runbook.md`](../../knowledge/codex-day-runbook.md)
> when the Codex CLI is installed.

## Open questions to resolve (mostly during S-021)

1. **Hook → pane attribution.** Does a Codex hook subprocess inherit
   `LISA_PANE_ID`, or must we correlate via `cwd`/`session_id` from the stdin
   payload? (Determines the whole signal-plumbing shape.)
2. **Interactive-hook reliability.** Is issue #17532 present on our pinned Codex
   version? If interactive hooks are unreliable, does that force the `codex exec`
   driving model — and what does that cost in reuse/liveness parity?
3. **Heartbeat fidelity.** Is `PostToolUse` frequent enough to keep the
   stuck-detector honest, or do long tool-free stretches risk false "stuck"?
4. **Idle/attention semantics.** Under `--yolo` there are no approval prompts;
   is there any Codex state lisa should surface as "awaiting", or is that state
   simply Claude-only?
5. **Version pinning.** Codex flags/config have shifted across versions (e.g.
   `--full-auto` deprecation, `/approvals` removal). What Codex version does the
   epic target, and how do we guard against drift?
6. **Routing schema (S-026).** What exactly do the frontmatter fields look like
   (`agent`/`model`? a combined `route`?), and how does per-ticket override
   compose with the loop default? How is an invalid/unavailable `(provider,
   model)` handled at spawn?
7. **Provenance fidelity (S-027).** Which metrics are actually obtainable per run
   — cost/tokens (Codex `turn.completed.usage`; Claude equivalent?), wall-clock,
   concurrency-at-run, outcome/quality — and where do they live (ticket
   frontmatter vs a separate telemetry artifact) to stay queryable across runs?
8. **Extreme concurrency (S-026).** What breaks at ~16 concurrent mixed-provider
   agents — auth/rate limits, pane/slot limits, commit serialization, signal-file
   contention — and what's the realistic ceiling?

## Explicitly out of scope

- A third **native** adapter (native = Claude Code and Codex only, per the
  graduation rule; additional providers arrive via the ACP leg, which is
  in scope as the third integration *method*, not a third native shim).
- Changing the artifact-driven core (DAG, phase detection, scheduling).
- **Policy-based** routing (by ticket type / RDSPI phase) — a later evolution;
  S-026 covers only frontmatter + loop-default/override routing.

> Note: *per-ticket client selection* and *model selection within a provider*
> were previously out of scope but are now **in scope** as the S-026 north star.

## Source notes

Codex intel drawn from `developers.openai.com/codex` (hooks, config-reference,
noninteractive, cli/reference, slash-commands, agents-md) and `openai/codex`
issues #11808, #17532. Confidence levels recorded inline in Intel B.
