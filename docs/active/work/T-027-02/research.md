# T-027-02 Research — Cost/Token Capture Per Adapter

Descriptive map of the code and the external surfaces this ticket touches. No
solutions here — see design.md.

## 1. Where tokens/cost land in the ledger

The single construction+append point is `Plugin::emit_provenance`
(`crates/lisa-plugin/src/lib.rs:1420-1459`). It reads spawn-time facts off the
live `Thread` (`client`, `started_at`, `concurrency_at_spawn`, `pane_id`),
builds a `ProvenanceRecord`, then layers usage on top:

```rust
let (tokens_in, tokens_out, cost_usd) = self.read_codex_usage(client, ticket_id);
let record = ProvenanceRecord { tokens_in, tokens_out, cost_usd, ..record };
```

`emit_provenance` is called at every terminal site **before** the thread is
removed: Done at lib.rs:1403 / 2006 / 2650; Failed at lib.rs:1267 (`.error`) and
1894 (stale reclaim); TimedOut at lib.rs:1847. It already runs for **Claude**
runs — it just gets all-`None` back today.

`ProvenanceRecord` (schema owner `crates/lisa-core/src/provenance.rs:78-100`) has
nullable `tokens_in`, `tokens_out`, `cost_usd`. `SCHEMA_VERSION = 1`. The schema
already anticipates this ticket (comment at provenance.rs:29-31, 95).

## 2. The Codex capture path (the pattern to mirror)

Codex is already wired end-to-end:

- **Writer** — `crates/lisa-cli/src/agent_exec.rs`. `Translator::observe`
  captures `turn.completed.usage` (agent_exec.rs:115-124, `extract_usage`
  196-202: event-level `usage` or nested `turn.usage`). At child exit,
  `persist_run_artifacts` (398-422) writes
  `.lisa/codex/<key>.usage.json` with shape
  `{ key, thread_id, success, usage }`. The dir/key are built in
  `run_agent_exec` (agent_exec.rs:492-496): `key = LISA_TICKET_ID` (falling back
  to `pane-<id>`, then `last`).
- **Reader** — `Plugin::read_codex_usage` (lib.rs:1467-1490). Guards
  `if client != AgentClient::Codex { return (None,None,None) }`, reads
  `self.codex_dir.join("<ticket>.usage.json")` (`.lisa/codex`, set lib.rs:2827),
  parses, and hands the nested `usage` object to
  `provenance::extract_usage`.
- **Normalizer** — `provenance::extract_usage`
  (`provenance.rs:113-128`) reads whichever of `input_tokens|input`,
  `output_tokens|output`, `cost_usd|cost|total_cost_usd` is present. Tolerant of
  the provisional Codex shape; never fabricates.

The artifact-file indirection is deliberate: a `.usage.json` survives the plugin
deleting the `.stopped` signal on read, and it is written by the agent-side
process, read by the plugin write-after.

## 3. How a Claude session is launched — and what it does NOT carry

`build_claude_command` (lib.rs:66-86):

```
LISA_PANE_ID=<n> LISA_TICKET_ID=<t> claude --dangerously-skip-permissions[ --model M] "<prompt>"
```

Key facts:
- **No `--settings` flag.** Hooks are loaded from the project-root
  `.claude/settings.local.json` that `lisa init` writes
  (`settings_local_json()`, `templates.rs:149-217`; written init.rs:388-429).
- `LISA_PANE_ID` and `LISA_TICKET_ID` are **both** exported into the session env
  and are inherited by every hook subprocess. These are the correlation keys — a
  Claude Stop hook already has the same `LISA_TICKET_ID` the Codex artifact is
  keyed by. No new correlation plumbing is required.
- The `lisa` binary path is **not** exported to Claude sessions today. The Codex
  adapter threads an absolute `lisa` path (config `lisa_bin`) because a pane
  shell may lack `lisa` on PATH (adapter.rs:245-250); the Claude launch line has
  no equivalent.

## 4. The Claude hooks that exist today (all POSIX `sh`)

From `templates.rs`, scaffolded by init.rs:340-350 into `.lisa/hooks/`:

| Hook (event) | Script | Writes |
|---|---|---|
| PostToolUse | `on-heartbeat.sh` (58-68) | `pane-<id>.heartbeat` mtime |
| Stop | `on-stop.sh` (28-38) | `pane-<id>.stopped` |
| SessionStart[clear] | `on-clear.sh` (42-52) | `pane-<id>.cleared` |
| Notification[idle_prompt] | `on-idle.sh` (14-24) | `pane-<id>.idle` |
| PreToolUse[AskUserQuestion] | inline (142) | `pane-<id>.awaiting` |

Every script is trivial: `mkdir -p .lisa/signals`, guard on `$LISA_PANE_ID`,
write a timestamp. **None of them read stdin.** Claude Code pipes the hook
payload JSON to the command's stdin — for Stop that payload includes
`session_id`, `transcript_path`, `cwd`, `stop_hook_active`. Today the Stop hook
discards it.

The ticket's constraint (Notes): the **heartbeat (PostToolUse)** must stay
trivial — no per-tool-call payload processing beyond the mtime bump. The Stop
hook fires once per turn boundary, not per tool call.

## 5. The external Claude cost/token surface (what the Stop payload buys)

Claude Code exposes per-session usage only through the **transcript JSONL** at
`transcript_path` (under `~/.claude/projects/<slug>/<uuid>.jsonl`). Each
assistant line carries `message.usage`:

```json
{"type":"assistant","message":{"model":"claude-...","usage":{
  "input_tokens":4,"cache_creation_input_tokens":0,
  "cache_read_input_tokens":14791,"output_tokens":123}}}
```

Observations:
- Usage is **per assistant message**; a session total is the sum across the
  transcript. The transcript is cumulative and append-only, so reading it at the
  final Stop yields the whole-run total (and each earlier Stop yields a running
  total — last-write-wins is the final total).
- There is **no reliable dollar-cost field** in current transcripts. `costUSD`
  existed on some historical entries but is not dependable; cost is derived
  downstream from tokens × model pricing.
- The four token fields are distinct input classes: fresh `input_tokens`,
  `cache_creation_input_tokens`, `cache_read_input_tokens`, and `output_tokens`.
  "tokens_in" must decide which classes it sums — a fidelity question (§7).

### WASM boundary constraint

The plugin runs in Zellij's WASI sandbox with the host mounted at `/host`
(`strip_host_prefix`, lib.rs:111-117). It can read `/host/.lisa/**` (that is how
it reads signals and the Codex artifact) but **cannot** reach
`~/.claude/projects/...` — that path is outside the `/host` mount. Any transcript
parsing therefore has to happen host-side, and the result deposited under
`.lisa/` for the plugin to read. This is the same reason the Codex usage capture
lives in the `lisa agent-exec` subprocess, not the plugin.

## 6. Comparability of Codex vs Claude counts

- Codex `turn.completed.usage` fields are provisional (`input_tokens` /
  `output_tokens`, possibly cached-inclusive — the wrapper reads them verbatim).
- Claude splits cached vs fresh input explicitly.
- Neither is guaranteed to mean the same thing per token. The schema already
  keeps `requested`/`actual` route on every record, so cross-provider queries can
  always segment by `actual.method` — raw counts per provider are the honest unit;
  a normalized cross-provider token is not defensible without a mapping neither
  vendor publishes.

## 7. Constraints and assumptions carried into design

1. **Never fabricate.** Missing → `null`. (provenance.rs doc, AC.)
2. **Write-after.** No mid-run writes racing the agent's frontmatter; capture
   must not add per-tool-call payload processing (heartbeat stays trivial).
3. **Mirror, don't reinvent.** Codex already defines the artifact→plugin-read
   contract; Claude should reuse the same shape and the same reader spine.
4. **`lisa` reachability from a Stop hook is not guaranteed** (PATH), and the
   binary path is not currently exported to Claude sessions — an open plumbing
   question for design.
5. **`extract_usage` already tolerates a `usage` object** with token/cost fields —
   whatever writes the Claude artifact should emit that same shape.
