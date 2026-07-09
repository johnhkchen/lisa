# T-027-01 — Design: Provenance Ledger

Decisions, with the options weighed and rejected, grounded in `research.md`.

## Problem restated

Emit one append-only JSONL record per ticket-run at each of the six teardown
sites, after the run ends, without touching agent frontmatter or racing the
agent. The record's data lives in three places today: the `Thread` (`started_at`,
`pane_id`), `self.config` (the client), and — for Codex — an on-disk
`.lisa/codex/<key>.usage.json` artifact the plugin does not yet read.

## Decision 1 — Record type + I/O live in `lisa-core`

**Chosen:** a new `crates/lisa-core/src/provenance.rs` owning `ProvenanceRecord`,
`Route`, `RunOutcome`, and `append_record(path, &record)`.

- **Why:** the same precedent as `client.rs` — "one place both readers share, not
  two ad-hoc parsers." The plugin writes it; a future `lisa` CLI query command
  reads it. Keeping the struct + serde + append in `lisa-core` makes the schema
  a single source of truth and lets it be unit-tested natively (the plugin
  crate compiles to WASM; `lisa-core` tests run native like `agent_exec.rs`'s
  tempdir tests).
- **Rejected — put it in `lisa-plugin`:** only the plugin writes today, but the
  ledger is explicitly "committable learning data" meant to be queried; binding
  the schema to the WASM crate blocks a native CLI reader and native tests of
  the append path.
- **Rejected — a new crate:** violates the zero-extra-dependency ethos; the type
  is ~60 lines. `lisa-core` already re-exports its modules via `lib.rs`.

No new dependencies: `serde`, `serde_json`, `SystemTime` serde are all in-tree.

## Decision 2 — Emit via an additive `emit_provenance` call, not a teardown refactor

All six sites (research §1) share `fail()/complete()` → `release_slot_for_ticket`
→ `threads.remove`. Two ways to inject the ledger write:

**Chosen:** add one method `State::emit_provenance(&mut self, ticket_id,
outcome)` and call it at each site **immediately before** `self.threads.remove()`
(while the thread is still readable). The method reads the thread + config +
Codex artifact, builds the record, appends, and logs on error. It mutates no
thread/slot state.

- **Why:** minimal, surgical change to delicate scheduler control flow — six
  one-line insertions, no reordering of the existing `complete/release/remove`
  dance. Record-building and append stay centralized in one function, so the
  schema is written in exactly one place and no site can format a record
  differently. The thread is present at call time (removal is the next line), so
  all fields are readable.
- **Rejected — a centralized `finalize_run(tid, outcome)` that folds in
  `release_slot_for_ticket` + `remove`:** more DRY, but rewrites the teardown at
  six ordering-sensitive sites (e.g. site 5 pushes `timeout_alerts` between
  `fail()` and `remove`; site 4 pushes `error_alerts`). The refactor risk
  outweighs the duplication saved — the duplicated part (`release`+`remove`) is
  two trivial lines, and the non-trivial part (record building) is already
  centralized by the chosen approach.

Outcome per site is passed explicitly: sites 1–3 → `Done`; site 4 (`.error`) →
`Failed`; site 5 (timeout) → `TimedOut`; site 6 (stale silence) → `Failed`.

## Decision 3 — Snapshot run-metadata onto `Thread` at spawn

The record needs values known only at spawn: the resolved client and the
concurrency at spawn. Neither is on the thread today (research §2, §3).

**Chosen:** add two `#[serde(default)]` fields to `Thread`:
```rust
#[serde(default)] pub client: AgentClient,        // resolved adapter's client
#[serde(default)] pub concurrency_at_spawn: usize, // running threads at spawn
```
`Thread::new` leaves them at defaults (`Claude`, `0`); the real spawn site
(`lib.rs:637`) sets them: `thread.client = self.config.client;` and
`thread.concurrency_at_spawn = running_count;` (that count is already computed one
line up at `lib.rs:551`, exclusive of the new thread).

- **Why `Thread`, not a side map:** the thread *is* the run's identity and already
  carries `started_at`; adding the two missing spawn-time facts keeps all run
  provenance in one struct that lives exactly as long as the run. `#[serde(default)]`
  keeps the state-dump/`Deserialize` backward-compatible and leaves the ~20
  `Thread::new(id, pane)` test call sites untouched (signature unchanged).
- **Why `client` snapshot (vs. read `self.config.client` at teardown):** identical
  value today (config is loop-fixed), but snapshotting is correct when per-pane
  routing (S-026) makes the resolved client differ per ticket — the ledger then
  records what *actually* ran without re-deriving. Forward-compatible for free.
- **Concurrency = at-spawn, not peak:** the ticket says "running count at spawn
  **and/or** peak." At-spawn is a single cheap capture at the exact line the count
  is already computed; peak-during-run would need a `State`-level max updated
  every poll and adds cost for marginal signal now. At-spawn satisfies the AC;
  peak can be added later as another nullable field without a schema break.

`Route { method, provider, model }` is derived from the client at record time:
`method = client.as_str()` (`"claude"|"codex"`), `provider` = `anthropic`|`openai`,
`model = None`. **`requested` and `actual` are both this route today** (research §4)
— populated from day one so the schema is stable when routing splits them.

## Decision 4 — Read Codex usage from the existing artifact at teardown

**Chosen:** for a Codex run, `emit_provenance` reads
`/host/.lisa/codex/<ticket_id>.usage.json` (the file the wrapper already writes,
research §5), best-effort-extracts token/cost fields, and stores them nullable.
Claude runs store `null` tokens/cost (deferred to T-027-02).

- **Why:** honours write-after — the wrapper produces the artifact when the run
  ends; the plugin reads it after. No new IPC, no change to the signal contract,
  the wrapper stays purely a signal producer. The `key` is `LISA_TICKET_ID` = the
  ticket id (research §5), so the path is deterministic from the thread.
- **Rejected — wrapper appends to the ledger directly:** two writers to one
  append-only file (wrapper on the host + plugin) invites interleaving and
  duplicate records, and puts ledger schema knowledge in two crates. The plugin
  is the single writer by Decision 2 of the epic.
- **Extraction is defensive** (the Codex `usage` shape is provisional): read
  `input_tokens`|`input` → `tokens_in`; `output_tokens`|`output` → `tokens_out`;
  `cost`|`cost_usd`|`total_cost_usd` → `cost_usd`; any missing → `None`. Never
  fabricate. A missing/parse-failing file → all `None`, logged at info, not fatal.

## Decision 5 — Append with `OpenOptions::append`, non-fatal on error

**Chosen:** `append_record` serializes the record to a single compact JSON line
(`serde_json::to_string`, no pretty) + `\n`, ensures the parent dir exists, and
opens `OpenOptions::new().create(true).append(true)` to write. Failures return
`io::Error`; the plugin caller logs an `ActivityEvent::Error` and swallows it —
never fatal to the loop (AC).

- **Why append-mode:** true append-only semantics; retries/resets each add a line
  and nothing rewrites history (AC). The plugin is single-threaded (one
  `poll_tick` at a time), so there is no concurrent-writer race within lisa.
- **WASI verification:** `OpenOptions::append` maps to WASI `fd_write` on a file
  opened with the append right; native tests cover it directly. Whether Zellij's
  WASI host honours the append flag is verified in Implement by driving the
  plugin and inspecting the file (the fallback, if it does not, is
  read-existing + rewrite-whole, race-free here because writes are sequential).
  Documented as a plan step, not assumed away.

## Decision 6 — Versioned schema, documented in a knowledge doc

`schema_version: u32 = 1` is the first field. The field table + a jq and a duckdb
example live in a new `docs/knowledge/provenance-ledger.md` (committed, discoverable
alongside `rdspi-workflow.md`), and the module doc in `provenance.rs` points to it.

- **Why a knowledge doc:** the ticket wants it "queryable across runs with
  jq/duckdb" — worked examples belong in prose, not a code comment. Versioning
  from v1 lets T-027-02 (cost fidelity) and S-026 (routing splits `requested` ≠
  `actual`) evolve readers without guessing.

## Record shape (v1)

```json
{
  "schema_version": 1,
  "ticket_id": "T-027-01",
  "outcome": "done",
  "requested": { "method": "codex", "provider": "openai", "model": null },
  "actual":    { "method": "codex", "provider": "openai", "model": null },
  "started_at": 1719800000,
  "ended_at": 1719800600,
  "wall_clock_secs": 600,
  "tokens_in": 12000,
  "tokens_out": 3400,
  "cost_usd": null,
  "concurrency_at_spawn": 3,
  "pane_id": 2
}
```
Timestamps are UTC epoch seconds (matching `system_time_serde`). `outcome` ∈
`{done, failed, timed-out}`. Nullable fields serialize as JSON `null`.

## Test strategy (native)

- `provenance.rs` unit tests: record serializes to one line with expected fields;
  `RunOutcome`/`Route` serde values; `append_record` creates the file, appends a
  second record without rewriting the first, and tolerates a missing parent dir.
- Plugin tests (native, like existing `lib.rs` tests): a run at each outcome emits
  exactly one record with the right `outcome`; a retry appends a second; the
  emission never mutates the ticket file (frontmatter untouched); Codex usage is
  read from a seeded `.lisa/codex/<id>.usage.json`, Claude yields null tokens.
