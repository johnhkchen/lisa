# T-023-01 Research — agent-exec wrapper

Descriptive map of the codebase surfaces this ticket touches. What exists, where,
how it connects. No solutions here — those are for Design.

## What the ticket is

Build a `lisa agent-exec` subcommand inside `lisa-cli`. lisa types
`LISA_PANE_ID=<n> LISA_TICKET_ID=<t> <lisa> agent-exec …` into a fresh pane; the
subcommand runs `codex exec --json -a never -s workspace-write …`, consumes the
JSONL event stream on stdout, writes lisa's signal files, renders a human-readable
conversation to its own stdout (which *is* the pane), persists the `thread_id` for
`resume`, and exposes `turn.completed.usage` for provenance.

This is the **Codex-side signal producer** — the analog of Claude Code's hook
scripts, but host-side Rust versioned atomically with the plugin (epic E-001
Decision 5; the stale-generated-script failure mode is precisely what we avoid).

## The signal contract (the thing we must reproduce byte-for-byte)

Claude's path writes signal files via four POSIX `sh` hooks in `templates.rs`
(`ON_IDLE_HOOK`, `ON_STOP_HOOK`, `ON_CLEAR_HOOK`, `ON_HEARTBEAT_HOOK`, lines
14–68). Every one is the same shape:

```sh
SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"
if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.<kind>"
fi
```

Established facts:

- **Path:** `.lisa/signals/pane-<pane_id>.<kind>`, relative to the pane's CWD
  (the working tree). `<kind>` ∈ `idle | stopped | cleared | heartbeat`.
- **Content is informational only.** The plugin never reads file *contents* — it
  reads **mtime** (heartbeat freshness) and **existence** (stopped/cleared/idle).
  Confirmed by reading the consumers (below). So the wrapper can write any
  content; a timestamp keeps parity with the hooks.
- **The `[ -n "$LISA_PANE_ID" ]` guard** is the degrade-safely contract: no pane
  id → no signal files, everything else still runs. The wrapper must mirror this.

## How the plugin consumes signals (`crates/lisa-plugin/src/lib.rs`)

- `check_heartbeat_signals()` (l.785): scans `pane-<id>.heartbeat`, calls
  `bump_pane_activity(id)`, then **deletes the file**. Also clears attention/
  awaiting debounce. Only existence+mtime matter; content ignored.
- `check_transition_signals()` (l.1085): scans `pane-<id>.stopped` and
  `pane-<id>.cleared`, **deletes**, then drives the transition state machine
  (`handle_stopped_signal`, `handle_cleared_signal`). `.stopped` in `Idle` +
  Review phase → `auto_complete_review` (marks ticket Done). `.stopped` in
  `WaitingForStop` → sends `/clear`.
- `check_idle_signals()` (l.870): scans `pane-<id>.idle`.
- **No `.error` reader exists anywhere in the plugin.** Grep confirms: `.error`
  appears only in unrelated `scan_result.errors` contexts. This is the single
  most important constraint carried from T-021-01's review (Open concern #1).

Consequence for this ticket: doc 05 maps `turn.failed`/non-zero exit → `.error`,
but on today's plugin a `.error` file is written into a directory nobody reads.
A failed codex turn that writes *only* `.error` would leave the scheduler waiting
forever for a `.stopped` that never comes. The wrapper's failure path must account
for this. (The plugin-side `.error` consumer is T-023-02's scope, not ours.)

## The CLI surface we extend (`crates/lisa-cli/src/`)

- `main.rs`: clap `Cli` with a `Commands` enum (Init, Validate, Status,
  SetupGuide, HooksGuide, Doctor, Version, Loop). Each arm resolves a path and
  calls a module fn returning `Result<_, String>`, `eprintln!`+`exit(1)` on error.
  We add one arm: `AgentExec { … }`.
- Module convention: one file per command (`doctor.rs`, `status.rs`, `init.rs`,
  `loop_cmd.rs`). We add `agent_exec.rs`, `mod agent_exec;` in `main.rs`.
- `Cargo.toml`: deps already include `serde`, `serde_json`, `toml`, `clap` v4
  (derive), dev-dep `tempfile`. **No new dependency is needed** — JSONL parsing is
  `serde_json`, process spawning is `std::process`, line reading is
  `std::io::BufRead`. This preserves lisa's zero-extra-dependency ethos.

## How lisa launches the pane command today (`lisa-plugin/src/lib.rs`)

`build_claude_command()` (l.53) formats
`LISA_PANE_ID={} LISA_TICKET_ID={} claude --dangerously-skip-permissions "{}"`.
For Codex, T-023-02 will swap `claude …` for `<lisa> agent-exec …`, inheriting the
same env-var prefix. The absolute lisa path comes from `current_exe()` captured at
`lisa loop` time and threaded through the layout config (S-023 mechanics). **That
plumbing is T-023-02's job.** T-023-01 only builds the subcommand that gets typed.

## The Codex event stream (from the intel packet, `[PROVISIONAL]`)

Per doc 05 §Option 1 and T-021-01's design.md, `codex exec --json` emits
newline-delimited JSON on stdout with **dot-form** event type names:

| Event `type` | Carries | Maps to |
|---|---|---|
| `thread.started` | `thread_id` | record for `resume` |
| `turn.started` | — | working / clear idle |
| `item.started` / `item.updated` / `item.completed` | an `item` (agent_message, command_execution, file_change, reasoning, mcp_tool_call…) | **`.heartbeat`** (bump mtime) |
| `turn.completed` | `usage` | **`.stopped`** (with exit 0) |
| `turn.failed` | `error.message` | **`.error`** |
| top-level `error` | message | **`.error`** |

**Anchor rule (T-021-01 Q2, [H] confidence in principle):** derive done/failed
from `turn.completed`/`turn.failed` **plus process exit code** — item statuses are
best-effort heartbeat only (#14691: items can carry stale/abandoned status at turn
end). Exit code is authoritative; the terminal turn event must agree with it.

**Schema is unconfirmed.** T-021-01 did not run against a live `rust-v0.142.5`
(codex was not installed on the host). The exact JSON *shape* — field nesting,
`item` discriminator key (`item_type` vs `type`), where `usage` rides, event-name
casing — is `[PROVISIONAL]`. Grep of the harness confirms probes were written but
never executed. **This is the dominant constraint on the parser design:** it must
be defensive (key on string prefixes, tolerate unknown shapes) rather than assume
a rigid struct that a schema drift would break.

## Rendering (T-021-01 Q3 verdict)

Recommendation: **render-from-JSON** (print clean lines from item/turn events),
not tee-stderr. Rationale carried from the spike: `exec --json`'s stderr richness
is unverified and version-volatile; `exec` is coarse-grained (completed-item
granularity, no token deltas), so both approaches show the *same* chunking anyway;
rendering from the JSON we already parse removes the stderr dependency (one read
loop → both signals and pane view). Chunked output is explicitly acceptable
(S-023, Decision 1). The verdict "flips to tee-stderr if stderr turns out rich and
stable" is deferred to an empirical run and is **not** a blocker for this ticket.

## Resume / follow-up (T-021-01 Q5)

`codex exec resume <thread_id>` continues a completed session with new
instructions, carrying context. The wrapper persists `thread_id` from
`thread.started`; a follow-up invocation (`--resume`) re-feeds it. `--last` is the
documented fallback if per-thread capture proves flaky. This is what T-023-02's
review finish-up path will call.

## Provenance (T-027-01/02 dependency)

`turn.completed.usage` must be captured where a later provenance ledger can find
it — "alongside the signal or to a per-run artifact" (AC). Exact `usage` placement
in the JSON is one of the unconfirmed schema facts; the capture must be defensive
(store whatever `usage` object appears, don't hardcode inner fields yet).

## Testing surface

AC requires **unit tests over the JSONL→signal translation using recorded event
streams** — no live codex in CI. This mandates a **pure, IO-free translation
core** that takes event lines (+ a simulated exit code) and yields signal
decisions / render lines, tested against fixture JSONL. The process-spawn +
file-write shell is the untested-in-CI outer layer.

## Constraints & assumptions surfaced

1. Signal-file semantics are mtime/existence, not content — verified in consumers.
2. No `.error` consumer today — failure path can't rely on it alone.
3. Codex JSON shape is `[PROVISIONAL]` — parser must be defensive, not rigid.
4. Zero new dependencies available and expected.
5. Pane attribution is env-based (`LISA_PANE_ID`), deterministic, inherited — the
   wrapper reads env, does not need pane-detection logic.
6. The pane-launch plumbing (absolute lisa path, typing the command) is T-023-02.
7. `.idle`/`.cleared`/`.awaiting` do **not** occur on the autonomous codex path
   (doc 05: `-a never` never pauses; fresh `exec` per ticket, no `/clear`) — the
   wrapper only ever needs to produce `.heartbeat`, `.stopped`, `.error`.
</content>
</invoke>
