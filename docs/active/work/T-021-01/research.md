# T-021-01 Research — codex-wrapper-spike

Map of (a) the lisa signal surface a Codex wrapper must satisfy, and (b) the five
empirical unknowns this spike settles, grounded in the code that consumes signals
today and in the pinned intel packet. Descriptive only — verdicts live in
`design.md`.

## What this spike is

A **spike**, not a build. Deliverable = a written verdict + evidence per unknown,
produced with throwaway stub scripts, pinned to codex **`rust-v0.142.5`**. The
driving-model decision (host-side wrapper translating `codex exec --json` JSONL
into signal files) is already made in the epic Decisions and
`docs/knowledge/codex-client/05-bridging-the-discrepancy.md`; this spike confirms
the mechanics that decision rests on. Interactive TUI driving and interactive
hooks are **refuted** in doc 04 and are out of scope.

## The signal contract the wrapper must satisfy (codebase reality)

lisa's scheduler never cares *how* signal files appear — only that they appear in
`.lisa/signals/`, correctly attributed to a pane. Today Claude Code hooks write
them; for Codex a wrapper writes them instead. The consumers, all in
`crates/lisa-plugin/src/lib.rs`:

- **`State.signal_dir`** (`lib.rs:204`) — the `.lisa/signals/` directory, scanned
  every poll tick.
- **`check_heartbeat_signals()`** (`lib.rs:785`) — consumes `pane-<id>.heartbeat`,
  bumps liveness, and clears the `awaiting_human` flag. Deletes the file.
- **`check_awaiting_signals()`** (`lib.rs:828`) — consumes `pane-<id>.awaiting`,
  flags the pane blocked on `AskUserQuestion`.
- **`check_idle_signals()`** (`lib.rs:870`) — consumes `pane-<id>.idle` (legacy
  `{ticket}.idle` also handled).
- **`check_transition_signals()`** (`lib.rs:1085`) — consumes `pane-<id>.stopped`
  (drives `WaitingForStop → /clear`) and `pane-<id>.cleared`
  (drives `WaitingForClear → send prompt`).

**File naming is fixed:** `pane-<pane_id>.<suffix>`, suffix ∈
`{heartbeat, awaiting, idle, stopped, cleared}`. Files are **deleted on read**.
The wrapper must emit exactly these names to be understood with zero scheduler
changes.

### Critical finding: `.error` has no consumer today

Doc 05's event→signal table maps codex failures to a `.error` signal, but a
`grep` for `.error` across `lib.rs` returns **nothing** — no `check_error_signals`,
no reader. So on the current scheduler the wrapper's only failure channel is
`pane-<id>.stopped` (a failed turn still "stops" the pane). Adding a real `.error`
consumer is **T-023 work, not this spike**; the spike should note that mapping
`turn.failed`/non-zero-exit to `.error` is aspirational until a consumer exists.

## How lisa launches the agent today (the env premise)

- **`build_claude_command()`** (`lib.rs:53`) launches
  `LISA_PANE_ID=<n> LISA_TICKET_ID=<id> claude --dangerously-skip-permissions "<prompt>"`.
  The pane id is injected as an **environment variable on the launched command**.
- The Codex reframe (doc 05 §15) depends on this: the wrapper lisa launches
  inherits `LISA_PANE_ID` directly, and so does any child codex spawns — giving
  deterministic pane attribution with no hook↔pane correlation problem. **Q1
  exists to verify this inheritance survives the wrapper → codex → tool-shell
  chain.**
- **`finish_up_prompt()`** (`lib.rs:63`) + `build_notify_command()` (`lib.rs:315`)
  are lisa's "nudge a stuck Review pane" mechanism. Its Codex analog is a fresh
  turn on the same thread via `codex exec resume` — **Q5**.
- The prompt is passed as a **CLI argument**, not typed into a TUI. This is why
  observability is achievable without the refuted keystroke injection: the wrapper
  can render from the JSON stream (doc 05 §Observability). **Q3** decides how.

## The five unknowns (from doc 05 §"Empirical unknowns" + the review pass)

1. **Env inheritance** — does a wrapper-launched `codex exec` child (and the shell
   codex spawns for a tool call) see `LISA_PANE_ID=7`? Expected `[M]`; the whole
   attribution model rests on it.
2. **`--json` fidelity under a real ticket** — with MCP/tools active, are
   `turn.*`/`item.*` events complete (#15451)? Do item statuses misreport at turn
   end (#14691)? Does the anchor rule (turn events + exit code authoritative,
   `item.*` best-effort) hold?
3. **In-pane rendering** — with stdout piped and stderr inspected, what does codex
   render on stderr under `--json`, at what granularity? Partial assistant text or
   only completed messages? Picks **tee-stderr vs. render-from-JSON** for T-023-01.
4. **Directory trust headless** — does a fresh `CODEX_HOME` block
   `codex exec -a never` on an untrusted repo? What must `lisa doctor` pre-seed
   (`[projects.<path>].trust_level = "trusted"`) and/or which flag bypasses it?
   (open bug #14345.)
5. **Follow-up via resume** — does `codex exec resume <thread_id>` reliably
   continue a completed session with new instructions (the `finish_up_prompt`
   analog for T-023-02)?

## Environment reality (constraint that shapes this spike)

**Codex is not installed on the current machine** (`which codex` → not found;
`~/.codex` absent; `CODEX_HOME` unset). This spike is empirical by definition, so
the honest deliverable from *this* host is:

- A **runnable harness** (`harness/`) — one stub probe per unknown, plus a driver
  and an RDSPI-style fixture prompt — that captures event streams, exit codes, env
  dumps, and stderr/stdout separation.
- **Provisional verdicts** derived from the pinned intel (doc 02/04/05 + the cited
  `openai/codex` issues), each explicitly flagged as requiring one run of the
  harness on a host with `rust-v0.142.5` to become authoritative.

The harness embodies the exact method each verdict needs, so promoting a verdict
from provisional → confirmed is a single `bash run-all.sh` once codex is present.

## Assumptions & constraints

- Codex facts are pinned to `rust-v0.142.5`; hooks are the most version-volatile
  surface (README caveat), but this spike deliberately avoids hooks entirely.
- `-a never` + `-s workspace-write` is the assumed autonomous mode; under it Codex
  never pauses for human input, so `.idle`/`.awaiting` never occur on the Codex
  path (doc 05 §reframe) — the wrapper need only produce `.heartbeat`/`.stopped`
  (and, once a consumer exists, `.error`).
- No production code may merge; stubs live under `docs/active/work/T-021-01/harness/`
  and are clearly labelled spike-only.
- If Q2 fails badly (events dropped under tools), the fallback is the app-server
  (doc 05 Option 2) — a **human decision**, not something to design around here.
