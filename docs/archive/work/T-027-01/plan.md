# T-027-01 — Plan: Provenance Ledger

Ordered, independently verifiable steps. Each step ends green and is committable.
Build/test commands (from `CLAUDE.md`):
`cargo test -p lisa-core` · `cargo test --workspace` ·
`cargo build -p lisa-plugin --target wasm32-wasip1 --release`.

## Step 1 — `lisa-core::provenance` schema + append I/O

- Create `crates/lisa-core/src/provenance.rs` with `Route`, `RunOutcome`,
  `ProvenanceRecord`, `SCHEMA_VERSION = 1`, `Route::from_client`, `extract_usage`,
  `append_record`, and `system_time_to_epoch`.
- `Route::from_client`: Claude → `{method:"claude", provider:"anthropic", model:None}`;
  Codex → `{method:"codex", provider:"openai", model:None}`.
- `extract_usage`: read `input_tokens`|`input`, `output_tokens`|`output`,
  `cost`|`cost_usd`|`total_cost_usd`; each `None` when absent; never fabricate.
- `append_record`: `create_dir_all(parent)`, `OpenOptions::new().create(true)
  .append(true).open(path)`, write `to_string(record) + "\n"`.
- Add `pub mod provenance;` to `crates/lisa-core/src/lib.rs`.

**Unit tests (in `provenance.rs`):**
- `record_serializes_to_one_compact_line` — no `\n` inside, ends with `\n` via
  append; expected keys present; nulls serialize as `null`.
- `outcome_serde_is_kebab` — `TimedOut` → `"timed-out"`.
- `route_from_client` — both clients map as specified.
- `extract_usage_reads_known_fields` and `extract_usage_absent_is_none`.
- `append_creates_then_appends` — two `append_record` calls → file has exactly two
  lines, the first unchanged; each line parses back to the original record.
- `append_creates_missing_parent_dir`.

**Verify:** `cargo test -p lisa-core` green.

## Step 2 — `Thread` spawn-time fields

- Add `#[serde(default)] pub client: AgentClient` and
  `#[serde(default)] pub concurrency_at_spawn: usize` to `Thread`
  (`crates/lisa-core/src/types.rs:330`).
- Initialize both in `Thread::new` (`Claude` default, `0`).

**Unit tests:**
- `new_defaults_run_meta` — `client == Claude`, `concurrency_at_spawn == 0`.
- `thread_deserializes_without_run_meta` — a JSON blob missing both keys
  deserializes with the defaults (backward-compat for the state-dump / older
  serialized threads).

**Verify:** `cargo test -p lisa-core` green. Confirms the ~20 existing
`Thread::new(id, pane)` call sites still compile (signature unchanged).

## Step 3 — Plugin state, paths, and spawn snapshot

- Import: `use lisa_core::provenance::{self, ProvenanceRecord, Route, RunOutcome};`.
- Add `ledger_path: PathBuf` and `codex_dir: PathBuf` to `State` (near
  `signal_dir`, `lib.rs:216`).
- In `load()` (near `lib.rs:2689`): set
  `self.ledger_path = host.join(".lisa/provenance.jsonl");` and
  `self.codex_dir = host.join(".lisa/codex");`.
- At the spawn site after `Thread::new` (`lib.rs:637`):
  `thread.client = self.config.client;`
  `thread.concurrency_at_spawn = running_count;`

**Verify:** `cargo test --workspace` green (no behaviour change yet; State default
gains two empty PathBufs, transparent to `..State::default()` tests).

## Step 4 — `emit_provenance` + `read_codex_usage`

- `fn read_codex_usage(&self, client: AgentClient, ticket_id: &str)
  -> (Option<u64>, Option<u64>, Option<f64>)`:
  - Claude → `(None, None, None)`.
  - Codex → read `self.codex_dir.join(format!("{ticket_id}.usage.json"))`; on any
    error return `(None, None, None)`; else parse JSON, take `["usage"]`, pass to
    `provenance::extract_usage`.
- `fn emit_provenance(&mut self, ticket_id: &str, outcome: RunOutcome)`: read the
  thread (return early if absent); build `ProvenanceRecord` from thread fields +
  `Route::from_client(t.client)` (requested == actual) + usage + `now`; append via
  `provenance::append_record`; on `Err`, `log_activity(ActivityEvent::Error{..})`
  and swallow.
- Guard: if `self.ledger_path` is empty (uninitialized in a unit test that didn't
  set it), skip silently — so unrelated tests that trigger a teardown don't write
  to `/`. (Ledger tests set an explicit path.)

**Verify:** `cargo test --workspace` still green (method compiles, not yet called).

## Step 5 — Wire the six teardown sites

Insert `self.emit_provenance(&<tid>, RunOutcome::X)` immediately before each
`self.threads.remove(...)`:

| Site | Fn | Outcome |
|------|----|---------|
| Review auto-complete | `auto_complete_review` (`~1358`) | `Done` |
| Manual mark-done | `mark_ticket_done` (`~2518`) | `Done` |
| Done-sweep | `poll_tick` (`~1875`) | `Done` |
| `.error` reclaim | `check_error_signals` (`~1223`) | `Failed` |
| Timeout reclaim | `check_session_timeouts` (`~1718`) | `TimedOut` |
| Stale reclaim | `detect_stale_threads` (`~1764`) | `Failed` |

Match each site's `ticket_id` binding (`tid` / `ticket_id` — some are `&TicketId`,
some `&String`; pass `&str`).

**Verify:** `cargo test --workspace` compiles; existing tests green.

## Step 6 — Plugin integration tests

Add to `lib.rs` `#[cfg(test)] mod tests`, each building `State` with an explicit
`ledger_path` + `codex_dir` in a `tempfile::tempdir()`:

- `emits_record_on_review_auto_complete` — seed a Review-phase ticket + running
  thread + `.stopped` handling → one ledger line, `outcome == "done"`,
  `ticket_id` matches.
- `emits_record_on_error_signal` — drive `check_error_signals` with a `pane-N.error`
  → one line, `outcome == "failed"`.
- `emits_record_on_timeout` — force `check_session_timeouts` reclaim → `"timed-out"`.
- `retry_appends_second_record` — emit twice for the same ticket → two lines, first
  unchanged (append-not-rewrite).
- `emission_does_not_touch_ticket_frontmatter` — snapshot the ticket `.md` bytes
  before/after a completion emission; assert equal.
- `codex_usage_flows_into_record` — thread with `client = Codex`, seed
  `<id>.usage.json` with `{"usage":{"input_tokens":10,"output_tokens":5}}` →
  record `tokens_in == Some(10)`, `tokens_out == Some(5)`.
- `claude_record_has_null_tokens` — Claude thread, no artifact → tokens null.

**Verify:** `cargo test --workspace` green;
`cargo build -p lisa-plugin --target wasm32-wasip1 --release` succeeds.

## Step 7 — Schema doc

- Write `docs/knowledge/provenance-ledger.md`: field table, `.lisa/provenance.jsonl`
  location + committable note, jq + duckdb query examples, schema-version note.
- Reference it from the `provenance.rs` module doc comment.

**Verify:** links resolve; `just check` (WASM check + tests) green.

## Step 8 — Runtime verification (append-in-WASI)

- Confirm `OpenOptions::append` works under Zellij's WASI host: run `lisa loop` on a
  throwaway ticket (or the existing repo `.lisa`), let a run complete, and inspect
  `/host/.lisa/provenance.jsonl` for a well-formed line; trigger a second run and
  confirm a second line was **appended** (first line intact).
- If append is not honoured by the host (line missing/overwritten), switch
  `append_record` to read-existing + write-whole (race-free: the plugin is
  single-threaded, one `poll_tick` at a time) and re-verify. Record the outcome in
  `progress.md`.

## Testing strategy summary

- **Unit (native, `lisa-core`):** schema serde, `extract_usage`, `append_record`
  append semantics, `Route::from_client`, `Thread` defaults + backward-compat.
- **Integration (native, `lisa-plugin`):** one record per outcome, append-on-retry,
  frontmatter-untouched, Codex-usage-flows / Claude-null.
- **Manual (WASI runtime):** append actually appends in the real plugin host.
- **Not fatal:** a ledger write error only logs — covered by the swallow path;
  asserted indirectly (a bad `ledger_path` does not panic the teardown).

## Rollback / risk

- All six insertions are additive; reverting them restores exact prior behaviour.
- `Thread`/`State` fields are `#[serde(default)]` / `Default`, so no migration.
- No new dependencies; no change to the signal contract or agent frontmatter.
