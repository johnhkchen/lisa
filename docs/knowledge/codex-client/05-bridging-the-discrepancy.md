# 05 · Bridging the discrepancy — what we can use

> Part of the **Codex client intel packet** (`docs/knowledge/codex-client/`). See [README](./README.md).
> Generated 2026-07-01 from three parallel research passes (headless `codex exec`, Codex programmatic protocols, wrapper/liveness patterns). Codex facts pinned to **`rust-v0.142.5`**; confidence tags `[H]/[M]/[L]` inline.
> **Options intel, not an implementation plan.** It names the mechanisms available to close the gap and their trade-offs. The choice is the spike's ([S-021](../../active/epics/E-001-pluggable-agent-client.md)) to make.

## The discrepancy, restated

lisa drives Claude Code by **typing into a live TUI pane** and reacting to **signal files that Claude Code hooks write** on lifecycle events (`.stopped`, `.cleared`, `.heartbeat`, `.idle`, `.awaiting`). Doc [04](./04-risks-and-open-questions.md) established that for Codex this path is unreliable: TUI keystroke injection is refuted (paste-burst heuristic), interactive hook delivery is refuted (#17532, no heartbeat cadence), and pane↔hook env correlation is undocumented.

## The reframe that resolves it

**Don't make Codex fit lisa's Claude-shaped control path. Change who produces lisa's signal files.**

lisa's scheduler doesn't care *how* `.lisa/signals/pane-<n>.*` files appear — only that they do, correctly attributed to a pane. Today Claude Code's hooks write them. For Codex, a **thin wrapper that lisa launches** can write them instead, translating Codex's *machine-readable* output into the exact same files. Because lisa already sets `LISA_PANE_ID=<n>` on the launched command, the wrapper (and any child it spawns) **inherits that env var directly** — sidestepping the entire "does a Codex hook know which pane it's in" problem `[H]`.

Two consequences shrink the problem further:

- **The autonomous headless path dissolves `.idle` and `.awaiting`.** Under `--yolo` / `codex exec` with `-a never`, Codex **never pauses for human input** (confirmed `[H]`, and there is no "awaiting input" event by design). lisa's two hardest-to-map signals — "waiting on a question" / "idle awaiting input" — simply don't occur. The scheduler's awaiting/attention machinery becomes a no-op for the Codex client rather than a gap to fill.
- **Session reuse via `/clear` becomes moot.** lisa sends `/clear` between tickets to reset Claude's context in a reused pane. The natural Codex analog is a **fresh `codex exec` per ticket** (or `codex exec resume` if continuity is ever wanted), so the `.cleared` handshake isn't needed on the Codex path.

**Net: the signal surface lisa actually needs from Codex collapses to three — `.heartbeat`, `.stopped`, `.error` — all cleanly derivable without hooks.**

---

## The mechanisms available (ranked)

### Option 1 — Wrapper around `codex exec --json`  *(primary candidate, unattended)*

lisa launches a thin wrapper (with `LISA_PANE_ID` in env) that runs
`codex exec --json -a never -s workspace-write [--skip-git-repo-check] -C <dir> "<prompt>"`,
reads the newline-delimited JSON event stream on stdout, and writes signal files. `[H]`

**Event → signal mapping** (`exec` uses dot-form event names):

| Codex event (stdout JSONL) | lisa signal |
|---|---|
| `thread.started` (`thread_id`) | record thread id (for optional `resume`) |
| `turn.started` | working / clear idle |
| `item.started` / `item.updated` / `item.completed` (esp. `command_execution` streaming) | **`.heartbeat`** (bump mtime per event) |
| `turn.completed` (`usage`) + **process exit 0** | **`.stopped`** (turn complete; exit is authoritative) |
| `turn.failed` (`error.message`) / top-level `error` / non-zero exit | **`.error`** |

- **Pros:** fully machine-readable; deterministic pane attribution via env; no dependence on hooks or TUI parsing; matches what CI-grade Codex orchestrators actually use; `codex exec resume <thread_id|--last>` re-feeds context headlessly for multi-turn. `[H]`
- **Cons:** headless — **no interactive TUI in the pane** (observability handled separately, see below); a few version-dependent `--json` bugs to design around: abandoned/wrong-status items at turn end (#14691 → treat `turn.completed`/`turn.failed` as authoritative, not item statuses), `--image` hangs (#5773 → avoid), `--json` silently ignored under some active MCP/tools (#15451). `[M]`
- **Anchor rule:** derive *done/failed* from `turn.completed`/`turn.failed` **+ process exit**; treat `item.*` as best-effort heartbeat. `[H]`

### Option 2 — `codex app-server` (JSON-RPC 2.0)  *(the "deep integration" upgrade path)*

A resident, bidirectional JSON-RPC protocol over stdio (JSONL) — **the interface OpenAI's own VS Code extension is built on**, i.e. the officially-blessed embedding surface. `[H]`

- **Event vocabulary** (slash-form): turn lifecycle `turn/started`, `turn/completed`, `turn/diff/updated`, `turn/plan/updated`; tool activity `item/started` + `item/completed` (`commandExecution`/`fileChange`/`mcpToolCall`/`webSearch`/…); streaming deltas `item/agentMessage/delta`, `item/commandExecution/outputDelta`; accounting `thread/tokenUsage/updated`; **approvals as answerable server→client requests** (`item/commandExecution/requestApproval`, `item/fileChange/requestApproval`, `item/tool/requestUserInput`). `[H]`
- **Session model:** long-lived, multi-turn, resumable/forkable (`thread/start` → repeated `turn/start`; `thread/resume`; `thread/fork`; `turn/steer` to inject into an in-flight turn; `turn/interrupt`). Context reset = `thread/compact/start` or fork/new thread (no literal `/clear`). `[H]`
- **Pros:** richest lifecycle contract; live-steerable resident process; approvals recoverable programmatically (so it could even support a supervised, non-`--yolo` mode later). `[H]`
- **Cons:** heaviest to adopt (lisa would speak JSON-RPC, a bigger departure than a wrapper); **no back-compat guarantee** — schema is version-pinned (regenerate with `codex app-server generate-ts` / `generate-json-schema` per version); WebSocket transport is experimental/unsupported; much of the surface is `experimentalApi`-gated. `[M]`
- **When it's worth it:** only if lisa later wants live steering/interrupt or supervised approvals. For the hackathon's "run tickets autonomously" goal, Option 1 delivers the same signals with far less surface area.

`codex mcp-server` (the `codex` + `codex-reply` MCP tools) is a third, simpler protocol, but its documented lifecycle/streaming vocabulary is **thin** — usable if lisa already spoke MCP, but weaker than both above for progress tracking. `[M]`

### Option 3 — `notify` program + env attribution  *(the interactive-pane companion)*

Codex's legacy `notify = [...]` config runs an external program on **`agent-turn-complete`**, and — crucially — it **fires in both interactive TUI and headless modes** (it predates, and is independent of, the unreliable hooks system). `[H]`

- A `notify` script writes `pane-$LISA_PANE_ID.stopped` (env inherited from launch `[M]` — verify). Payload arg (JSON on `argv[1]`): `type`, `thread-id`, `turn-id`, `cwd`, `input-messages`, `last-assistant-message` — though `cwd`'s presence is version-uncertain (#4005 closed not-planned vs. current docs listing it — **verify**). `[M]`
- **What it covers:** a clean turn-complete (`.stopped`) even while the human watches a live TUI. **What it can't:** it does **not** fire on approval/user-input pauses (#11808, #12524, both closed not-planned) — so it cannot produce `.idle`/`.awaiting`. `[H]`
- **Role:** the turn-complete signal for an *observable* interactive pane, paired with Option 4 for liveness.

### Option 4 — Output quiescence + PID + safer keystroke injection  *(TUI fallback)*

Only if lisa keeps the full-screen TUI for human observability/takeover:

- **Liveness by quiescence:** dump the pane every N s (Zellij pane dump / `tmux capture-pane`), hash it; unchanged for ~20 min → stuck. Coarser than JSONL heartbeat (a long compile looks "quiet"; a spinner looks "alive"). `[M]`
- **PID monitoring:** process gone → `.error`/`.stopped`; low CPU ≠ stuck (Codex idles at ~0% awaiting the API). Use as a floor, not the detector. `[M]`
- **Safer injection (if driving input at all):** `[tui].disable_paste_burst = true`; send text and Enter as **separate** writes with a short delay; wait for the composer prompt before sending; 2 s per-pane cooldown to avoid double-submits. Supervised-reliable, **not** unattended-reliable. `[H]` techniques, `[M]` overall.

---

## Observability: seeing the conversation in the pane

**Stated requirement:** lisa must be able to *show the Codex conversation inside the
pane*, even without interacting with it directly. This reshapes — but does not
overturn — the recommendation.

What the requirement does:

1. **Rules out silent headless.** Piping `codex exec --json` to a wrapper renders
   nothing human in the pane; we now owe the pane a readable view. `[H]`
2. **Does *not* reopen the injection problem.** `codex exec "<prompt>"` takes the
   prompt as a **command-line argument** — no keystroke injection, no paste-burst
   race, no startup-screen timing. Observability is achievable *without* the
   fragile TUI-driving that [04](./04-risks-and-open-questions.md) refuted. `[H]`
3. **Coexists with machine-grade signals** — the wrapper reads the JSONL stream
   **once** and, per event, both updates the signal file *and* prints a human line.
   One read loop → both a readable pane and reliable `.heartbeat`/`.stopped`/`.error`.
   No trade-off between the two. `[H]`

The one rendering decision it forces (three shapes):

| Shape | How the pane shows the conversation | Cost | Catch |
|---|---|---|---|
| **Tee stderr** | Codex's own human progress (stderr) flows to the pane while the wrapper consumes stdout JSON (launch codex with stdout piped, stderr inherited) | Cheapest | Unverified whether `--json` keeps *rich* human output on stderr or only a spinner `[M]` |
| **Render-from-JSON** | Wrapper prints clean lines from `item`/`turn` events (agent messages, commands, file changes) | Moderate — a small renderer; lisa already renders a dashboard | Coarser granularity than a live TUI |
| **Native TUI (watch-only)** | The real `codex` TUI — richest view for free | — | Reintroduces startup injection + coarser signals (`notify` + quiescence, no JSONL heartbeat) |

**The live-ness split — the only thing that would justify app-server (Option 2):**
`codex exec --json` is coarser-grained — you largely get an assistant message when
its `item` *completes*, so the pane updates in chunks. The **app-server** streams
`item/agentMessage/delta` + reasoning deltas — a live "watch it type/think" view. `[H]`
So:

- *"See the conversation, chunked/adequate"* → `exec --json` + render-from-JSON suffices.
- *"See it stream token-by-token like the official client"* → that specific goal is
  what earns app-server's heavier JSON-RPC integration.

**Effect on the wrapper's role:** signal-translator → **signal-translator + renderer**.
The engine choice (Option 1) is unchanged.

**New spike question this adds:** in `codex exec --json`, what still renders on
stderr and at what granularity, and does exec emit *partial* assistant text or only
*completed* messages? That single answer picks tee vs. render-from-JSON vs.
escalate-to-app-server.

## Comparison at a glance

| | Opt 1 · `exec --json` wrapper | Opt 2 · app-server | Opt 3 · `notify` | Opt 4 · quiescence/inject |
|---|---|---|---|---|
| Reliable turn-complete | ✅ stream + exit | ✅ `turn/completed` | ✅ turn-complete only | ⚠️ inferred |
| Heartbeat/liveness | ✅ `item.*` | ✅ `item/*` | ❌ | ⚠️ quiescence |
| Error/failure | ✅ | ✅ | ❌ | ⚠️ PID |
| Waiting-on-human | n/a (autonomous) | ✅ approval requests | ❌ | ⚠️ regex on pane |
| Pane attribution | ✅ env (deterministic) | ✅ per-connection | ✅ env `[M]` | ✅ pane id |
| Interactive TUI view | ❌ headless | ❌ headless | ✅ | ✅ |
| Adoption cost | low (thin wrapper) | high (JSON-RPC client) | low (script) | medium |
| Depends on flaky hooks? | ❌ | ❌ | ❌ | ❌ |

---

## Where the intel points

- **For the hackathon's autonomous "run tickets" goal *with* in-pane observability → Option 1 (wrapper around `codex exec --json`), extended so the wrapper also renders a human view.** It cleanly yields `.heartbeat` / `.stopped` / `.error` with rock-solid env-based pane attribution, needs no hooks, no TUI scraping, and — because the prompt is passed as a CLI argument — **no keystroke injection**. The same one-pass read loop that produces signals also prints the conversation to the pane (tee stderr, or render-from-JSON). Preserves lisa's artifact-driven core verbatim; multi-turn continuity, if ever needed, is `codex exec resume`.
- **Option 2 (app-server)** becomes worth its heavier JSON-RPC cost *only if* a **token-by-token live-streaming** conversation view is a hard requirement (its `item/agentMessage/delta` stream), or lisa later wants live steering/interrupt or supervised (non-`--yolo`) approvals. For a chunked-but-readable pane view, Option 1's renderer suffices.
- **Native watch-only TUI (Option 3 `notify` + Option 4 quiescence/PID)** gives the richest view for free but pays for it with startup keystroke injection and coarser liveness — worth it only if the *exact* Codex TUI presentation (not just the conversation content) is what's wanted.

This maps onto the whole-loop toggle cleanly: the client choice selects *which signal producer lisa launches* (Claude hooks vs. a Codex wrapper), leaving the scheduler that consumes the signals unchanged.

## Empirical unknowns to settle in the spike (S-021)

Cheap to confirm with stub scripts on the pinned `rust-v0.142.5`; do **not** design around them until verified:

1. **Env inheritance:** does a wrapper-launched `codex exec` reader — and a `notify` child — actually see `LISA_PANE_ID`? (Expected `[M]`; a stub that dumps `env` confirms.)
2. **`notify` payload `cwd`:** present on this version or not (#4005 vs. docs)? Dump `argv[1]`.
3. **Directory-trust in `exec`:** does a fresh `CODEX_HOME` block headless with `-a never`, or is trust bypassed? (Force with `--dangerously-bypass-approvals-and-sandbox` if needed.) `[M]`
4. **Exit-code contract:** confirm 0/non-zero behaviour empirically — the docs don't formalize it, so anchor on the event stream, not exit values. `[M]`
5. **`--json` fidelity under real tickets:** verify item/turn events aren't dropped when MCP/tools are active (#15451) or output is large (#10141/#14691).
6. **In-pane rendering (from the observability requirement):** in `codex exec --json`, what still renders on **stderr** and at what granularity, and does exec emit *partial* assistant text or only *completed* messages? Decides tee-stderr vs. render-from-JSON vs. escalate-to-app-server.

## Sources

Headless `exec`/`--json`: developers.openai.com/codex/noninteractive, /cli/reference; SDK `events.ts`; issues #10141, #14691, #15451, #5773. app-server: developers.openai.com/codex/app-server, repo `codex-rs/app-server/README.md`; MCP: /guides/agents-sdk. `notify`/injection/prior-art: /codex/config-advanced, /config-reference; issues #11808, #12524, #4005; PR #18914; codex-yolo, danielvaughan orchestration writeups. Full URLs inline above and in the per-topic research.

---

## Write-back: live-run verdicts (2026-07-11, codex-cli 0.144.1) — T-029-01

The S-021 empirical unknowns (the numbered list above) were run live for the
first time. Evidence: `docs/active/work/T-021-01/design.md` (verdicts) +
`docs/active/work/T-029-01/progress.md`.

- **#5 `--json` fidelity — RESOLVED PASS.** A real RDSPI ticket produced a
  terminal `turn.completed` agreeing with exit 0, with `command_execution` and
  `file_change` items present; **#15451 did not reproduce** with builtin tools.
  Anchor rule confirmed: `turn.completed`/`turn.failed` + exit are authoritative.
- **#6 rendering — RESOLVED render-from-JSON.** Under `--json`, stderr is a
  spinner-only status line (~39 bytes); stdout emits **completed-only** items,
  **no `*delta*` events**. Deltas remain app-server-only. tee-stderr rejected.
- **#1 env inheritance — PASS** (`LISA_PANE_ID` reaches the tool-shell).
- **#3 directory trust in `exec` — no block on 0.144.1** with the logged-in home
  (`-s workspace-write`, forced tool call ran, exit 0). #14345 affects the
  **native TUI** path, not `exec`; `lisa doctor`'s pre-seed is retained for the
  TUI (verified writing `trust_level="trusted"`).
- **Option-1 event map / usage shape — no drift.** Dot-form event names
  (`thread.started`, `turn.started`, `turn.completed`, `item.completed`), item
  types `agent_message`/`command_execution`/`file_change`, and
  `turn.completed.usage:{input_tokens, cached_input_tokens, output_tokens,
  reasoning_output_tokens}` all match. No `item.updated` observed.

**CLI-surface drift (0.144.1):** (a) `-a/--ask-for-approval` is **top-level
only** — rejected after `exec`; (b) `codex exec` **blocks reading stdin** unless
`</dev/null` (native TUI unaffected); (c) `codex exec resume` **rejects `-C`,
`-s`, `--skip-git-repo-check`** (inherits session cwd/sandbox). (c) breaks the
shipped `lisa agent-exec --resume` argv (diagnostics/headless path only — the
loop uses the native TUI). See docs 02 and 04 for the per-claim updates.
