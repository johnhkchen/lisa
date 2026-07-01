# T-023-01 Design — agent-exec wrapper

Options and decisions, grounded in `research.md`. Each decision names what was
rejected and why.

## Decision 1 — Where the code lives: `lisa-cli` subcommand (not a generated script)

**Chosen:** a new `lisa agent-exec` subcommand, module `crates/lisa-cli/src/agent_exec.rs`.

**Rejected — generated shell script** (the Claude-hook shape): the repo's own
`.lisa/hooks/` demonstrates the stale-generated-script failure mode (E-001
Decision 5, T-021-01 review). A script can't do real JSON parsing without `jq`
(not guaranteed present), and it drifts out of sync with the plugin it feeds.
Host-side Rust is versioned atomically with the plugin.

**Rejected — separate binary**: extra build/packaging surface; `lisa` is already
the single carried artifact. A subcommand reuses the existing clap plumbing.

## Decision 2 — Parser posture: defensive `serde_json::Value`, not rigid structs

The Codex JSON shape is `[PROVISIONAL]` (T-021-01 never ran against live codex).
The dominant design risk is a rigid `#[derive(Deserialize)]` struct that fails to
parse the moment the real schema differs by one field name or nesting level.

**Chosen:** parse each line to `serde_json::Value`, key decisions on the string
`type` field with **prefix matching** (`item.` → heartbeat; `turn.completed` →
done; `turn.failed`/`error` → error; `thread.started` → capture id). Extract
`thread_id`, `usage`, and render text with best-effort `.get()` chains that
tolerate missing/renamed fields (try several candidate keys). A line that isn't
JSON, or an event whose `type` we don't recognise, is **rendered raw / ignored for
signals** — never a hard error.

**Rejected — typed enum with `#[serde(tag = "type")]`**: cleaner *if* the schema
were confirmed, but a single unrecognised variant makes serde reject the whole
line, dropping a possibly-terminal event. Unacceptable against an unconfirmed,
version-volatile surface. When T-021-01's harness is run and the schema is pinned,
a typed layer can be introduced behind the same translation API without touching
callers — the defensive core stays as the fallback.

**Rejected — regex over raw text**: loses the structure we need for `usage`/
`thread_id` and rendering; the stream is genuine JSON, so parse it as JSON.

## Decision 3 — Anchor rule via a two-phase translator (stream + finalize)

Per doc 05 / T-021-01 Q2: item statuses are best-effort; the authoritative
done/failed decision is `turn.completed`/`turn.failed` **crossed with the process
exit code**. This means the terminal signal cannot be decided mid-stream.

**Chosen:** split translation into two operations on a `Translator` struct:

- `observe(&mut self, event: &Value) -> StreamEffect` — called per line during
  streaming. Returns `{ heartbeat: bool, render: Option<String> }`. Side-effects
  onto the struct: capture `thread_id`, `usage`, note `saw_turn_completed` /
  `saw_turn_failed` / `error_message`. `item.*` → `heartbeat = true`.
- `finalize(&self, exit_success: bool) -> Outcome` — called once after the child
  exits. Applies the anchor rule: `saw_turn_completed && exit_success` → `Success`;
  otherwise → `Failure { message }`. Returns the signal set to write.

This makes the whole JSONL→signal mapping a **pure function of (event lines, exit
code)** — exactly the unit-testable core the AC demands, with zero IO.

**Rejected — decide `.stopped` the instant `turn.completed` arrives**: ignores the
exit-code cross-check (a turn can "complete" then the process die non-zero, #14691
territory), violating the anchor rule.

## Decision 4 — Failure path: write `.error` **and** compat `.stopped`

`research.md` established there is **no `.error` consumer in today's plugin**. Doc
05's `turn.failed`/non-zero → `.error` mapping is aspirational until T-023-02 adds
the reader.

**Chosen:** on failure the wrapper writes **both**:
- `pane-<id>.error` — canonical, per the AC and doc 05; the file T-023-02's
  consumer will read; also the durable provenance/debug record of the failure.
- `pane-<id>.stopped` — compatibility, so today's scheduler (which only knows
  `.stopped`) still advances instead of hanging forever on a dead pane.

On success: only `pane-<id>.stopped`. The presence of `.error` is what distinguishes
the two for the future consumer. Documented in structure.md as the explicit
resolution of T-021-01 review Open-concern #1, with a note that T-023-02 may gate
off the compat `.stopped` once it reads `.error`.

**Rejected — write only `.error`**: strands the scheduler (no consumer). **Rejected
— write only `.stopped`**: loses the failure distinction and the provenance record,
and gives T-023-02 nothing to build its consumer against.

## Decision 5 — Rendering: render-from-JSON, chunked

Per T-021-01 Q3. The wrapper prints readable lines derived from the same events it
parses for signals — one read loop, both outputs (doc 05 §Observability). A small
`render_event()` maps:

- `agent_message` completed → the message text (the assistant's reply).
- `command_execution` started → `$ <command>`.
- `file_change` → `~ <path>` per changed file.
- `reasoning` → a dim `· <summary>` (best-effort; skipped if absent).
- `mcp_tool_call` → `→ <server>.<tool>`.
- `turn.completed` → a `— done (<usage summary>)` line.
- errors → `✗ <message>`.
- unknown item/event → compact single-line JSON, so nothing is silently swallowed.

**Rejected — tee-stderr**: bets on unverified stderr richness (T-021-01 Q3); no
granularity gain since `exec` is completed-item coarse anyway. Kept as a documented
future flip if an empirical run shows stderr is rich+stable.

**Rejected — no rendering (silent headless)**: violates the S-023 in-pane
observability requirement — the pane would show nothing.

## Decision 6 — thread_id persistence: per-ticket file under `.lisa/codex/`

`--resume` (T-023-02's finish-up path) needs to find the prior thread. It's keyed
by ticket because finish-up resumes *the ticket's* session.

**Chosen:** persist to `.lisa/codex/<ticket_id>.thread` (single line = thread_id),
written when `thread.started` is seen. `--resume` reads
`.lisa/codex/<LISA_TICKET_ID>.thread`; if present → `codex exec resume <id> …`; if
absent → fall back to `codex exec resume --last …` (T-021-01 Q5 fallback). No
`LISA_TICKET_ID` and `--resume` → error out clearly (can't locate a thread).

**Rejected — persist beside the signal (`pane-<id>.thread`)**: pane ids are
recycled across tickets; keying by pane would resume the wrong session.

## Decision 7 — usage/provenance capture: per-run artifact

**Chosen:** on `turn.completed`, capture the raw `usage` `Value`; at finalize,
write it to `.lisa/codex/<ticket_id>.usage.json` alongside `thread_id`, ticket id,
and success flag. Defensive: store the whole `usage` object verbatim (don't
hardcode inner field names — placement is unconfirmed, T-027-02 will read it).
Falls back to `pane-<id>.usage.json` if no ticket id.

**Rejected — inline into the `.stopped` file**: the plugin deletes `.stopped` on
read, destroying the provenance record. A separate artifact survives.

## Decision 8 — CLI shape

```
lisa agent-exec [OPTIONS] <PROMPT>
  --resume                 resume this ticket's persisted thread (else --last)
  --codex-bin <BIN>        [default: codex]
  --cwd <DIR>              working tree passed to codex -C [default: .]
  --bypass-sandbox         use --dangerously-bypass-approvals-and-sandbox
                           instead of the default -a never -s workspace-write
  --codex-arg <ARG>        extra flag passed through to codex exec (repeatable)
  --signal-dir <DIR>       [default: .lisa/signals] (override for tests)
```

Standard flags the wrapper always supplies: `exec --json --skip-git-repo-check
-C <cwd>` plus the sandbox flags (default `-a never -s workspace-write`). Env
`LISA_PANE_ID` / `LISA_TICKET_ID` are read from the environment (inherited), not
CLI args — matching how the hooks and `build_claude_command` pass attribution.

**Trust:** unattended runs need the repo pre-seeded as trusted (T-021-01 Q4); that
seeding is **T-025-01 doctor's** job. `--bypass-sandbox` is the explicit escape
hatch (also disables sandbox, so not the default).

## Decision 9 — Sync IO, no async runtime

**Chosen:** `std::process::Command` with piped stdout, a `BufReader` line loop,
`child.wait()` for exit. No tokio. The stream is line-oriented and single-consumer;
async buys nothing and would add a heavy dependency against the zero-dep ethos.

**Rejected — tokio/async streams**: unjustified dependency and complexity.

## Summary of the decided shape

A defensive, two-phase, pure translator core (`Translator::observe`/`finalize`)
wrapped by a thin IO shell (`run_agent_exec`) that spawns codex, streams lines
through the translator, writes `.heartbeat` per item event, `.stopped`(+`.error`
on failure) at the end per the anchor rule, renders each event to stdout, and
persists `thread_id`/`usage` per ticket. Zero new dependencies. Degrades to
"run + render, no signals" when `LISA_PANE_ID` is unset.
</content>
