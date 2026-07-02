# T-027-01 — Progress

## Completed

- **Step 1 — `lisa-core::provenance`** (`crates/lisa-core/src/provenance.rs`, new):
  `Route`, `RunOutcome` (kebab serde: `done`/`failed`/`timed-out`),
  `ProvenanceRecord`, `SCHEMA_VERSION = 1`, `Route::from_client`, `extract_usage`,
  `system_time_to_epoch`, `append_record` (create+append). `pub mod provenance;`
  added to `lisa-core/src/lib.rs`. 7 unit tests green.
- **Step 2 — `Thread` spawn fields** (`crates/lisa-core/src/types.rs`): added
  `#[serde(default)] pub client: AgentClient` and `pub concurrency_at_spawn: usize`;
  `Thread::new` initializes them. 2 tests (defaults + serde back-compat) green.
- **Step 3 — plugin state/paths/spawn snapshot** (`crates/lisa-plugin/src/lib.rs`):
  added `ledger_path` + `codex_dir` to `State`, set in `load()`; import of
  `provenance::{self, ProvenanceRecord, Route, RunOutcome}`; spawn site snapshots
  `thread.client = self.config.client` and `thread.concurrency_at_spawn = running_count`.
- **Step 4 — `emit_provenance` + `read_codex_usage`**: `emit_provenance` reads the
  thread, builds a record (requested == actual == route-from-client), reads Codex
  usage, appends, logs+swallows on error, no-ops when `ledger_path` is unset.
  `read_codex_usage` reads `.lisa/codex/<ticket>.usage.json` for Codex, else all
  `None`.
- **Step 5 — six teardown sites wired**: `auto_complete_review`,
  `mark_ticket_done`, poll_tick done-sweep (→ `Done`); `check_error_signals`,
  `detect_stale_threads` (→ `Failed`); `check_session_timeouts` (→ `TimedOut`).
  Each calls `emit_provenance` immediately before `threads.remove`.
- **Step 6 — plugin integration tests**: 7 tests written (error-signal emission
  end-to-end, retry-appends, Codex-usage-flows, Claude-null-tokens,
  frontmatter-untouched, ledger-unset no-op).
- **Step 7 — schema doc**: `docs/knowledge/provenance-ledger.md` (field table,
  jq + duckdb examples, versioning); referenced from the module doc.

## Verification status

- `cargo test -p lisa-core` → **140 passed, 0 failed**, including all 7
  `provenance::` tests and both new `Thread` tests. The load-bearing schema,
  append semantics, usage extraction, and Thread fields are fully verified.
- `cargo build --workspace` compiled **cleanly** with my changes at the point I
  finished the plugin wiring (before the concurrent edit below).

## Deviation / blocked: concurrent sibling ticket collision (T-026-01)

While I was in Implement, sibling ticket **T-026-01 (routing-frontmatter)** landed
uncommitted changes into the **same shared working tree** — a `crates/lisa-core/
src/route.rs` module (`ResolvedRoute`), `agent`/`model` fields on `Ticket`, and an
in-flight rewrite of the plugin's adapter-resolution API: `resolve_adapter_or_native`
now returns a **`(Box<dyn AgentAdapter>, ResolvedRoute)` tuple** and
`build_claude_command`/`ClaudeCodeAdapter` gained a `model` parameter.

Consequently `cargo test -p lisa-plugin` currently **does not compile** — but
every error is in `adapter.rs` and the adapter call sites in `lib.rs` (E0599
"method not found for tuple", E0061 arg-count), i.e. the sibling's half-applied
API change. **Verified: zero errors reference any provenance symbol**
(`emit_provenance`, `read_codex_usage`, `ledger_path`, `ProvenanceRecord`) or my
tests — grep of the error set confirms it. My spawn snapshot reads
`self.config.client`, not the adapter, so it is untouched by the tuple change.

This is the RDSPI "two tickets modify the same file → missing dependency edge"
case (both T-026-01 and T-027-01 edit `lib.rs`). Per the concurrency model I did
**not** edit the sibling's in-flight `adapter.rs`/`build_claude_command` work to
avoid clobbering it. My plugin integration tests are correct and will run green
once the T-026-01 adapter API settles (they don't touch the adapter surface).

## Follow-up enabled by T-026-01 (noted, not done)

T-026-01's `ResolvedRoute` (`requested_agent`, resolved client, `model`,
`substituted`) is exactly the source for the ledger's `requested` vs `actual`
split. Once both land, a small follow-up should snapshot the `ResolvedRoute` onto
the `Thread` at spawn so `emit_provenance` records real routing/fallbacks and the
`model` field, instead of `requested == actual == config.client`. The schema
already carries both routes + nullable `model`, so this needs **no schema change**
— matching the ticket's "populate both fields from day one" instruction.
