# T-027-01 — Review: Provenance Ledger

Handoff for a human reviewer. What changed, how it's tested, and what needs
attention.

## What was built

An append-only JSONL ledger at `.lisa/provenance.jsonl`: the plugin appends one
record per ticket-run at teardown (write-after), across all six terminal sites,
with `(method, provider, model)` requested+actual, timestamps, wall-clock, Codex
tokens/cost (nullable), concurrency-at-spawn, and outcome. Schema is versioned and
documented. All five acceptance criteria are implemented.

## Files

**Created**
- `crates/lisa-core/src/provenance.rs` — schema (`ProvenanceRecord`, `Route`,
  `RunOutcome`, `SCHEMA_VERSION=1`), `Route::from_client`, `extract_usage`,
  `system_time_to_epoch`, `append_record`. Pure + native-testable; no scheduler
  knowledge. 7 unit tests.
- `docs/knowledge/provenance-ledger.md` — field table, nullability notes, jq +
  duckdb query examples, append/versioning semantics.

**Modified**
- `crates/lisa-core/src/lib.rs` — `pub mod provenance;`.
- `crates/lisa-core/src/types.rs` — `Thread` gains `#[serde(default)] client:
  AgentClient` and `concurrency_at_spawn: usize`; `Thread::new` initializes both.
  2 tests (defaults, serde back-compat for old state dumps).
- `crates/lisa-plugin/src/lib.rs` — `State.ledger_path` + `State.codex_dir` (set
  in `load()`); provenance import; spawn-site snapshot of client + concurrency;
  `emit_provenance` + `read_codex_usage` methods; six teardown-site calls; 7
  integration tests.

## How each acceptance criterion is met

1. **Record on completion & terminal failure** — `emit_provenance(tid, outcome)`
   is called at all six teardown sites (research §1): three `Done` sites
   (`auto_complete_review`, `mark_ticket_done`, poll_tick done-sweep), `Failed`
   for `.error` reclaim and stale-silence, `TimedOut` for session/phase timeout.
   Record carries id, requested+actual route, started/ended/wall-clock,
   tokens/cost (nullable), `concurrency_at_spawn`, `pane_id`, outcome.
2. **Append-only, non-fatal** — `append_record` opens `create(true).append(true)`;
   retries append a new line (test `append_creates_then_appends`,
   `provenance_retry_appends_not_rewrites`). A write error logs an
   `ActivityEvent::Error` and is swallowed (test `provenance_noop_when_ledger_unset`
   covers the guard path; the error branch is a pure log-and-continue).
3. **Codex cost/tokens flow; Claude null** — `read_codex_usage` reads
   `.lisa/codex/<ticket>.usage.json` (the wrapper's existing artifact) and
   best-effort-extracts tokens/cost; Claude returns all `None`
   (`provenance_codex_usage_flows_into_record`, `provenance_claude_record_has_null_tokens`).
4. **Documented, versioned, jq/duckdb-queryable** — `schema_version` field +
   `docs/knowledge/provenance-ledger.md` with a field table and worked queries.
5. **Native tests** — emitted on completion and on failure; append-not-rewrite;
   frontmatter untouched (`provenance_does_not_touch_ticket_frontmatter`).

## Test coverage

- **`cargo test -p lisa-core` → 140 passed, 0 failed.** Covers the load-bearing
  logic: record serde (single compact line, kebab outcomes, null fields,
  round-trip), `Route::from_client`, `extract_usage` (known fields + alternate
  names + absent → None), `append_record` append semantics + missing-parent-dir,
  epoch conversion, and both `Thread` field tests.
- **7 plugin integration tests written** for the six-site wiring and the
  frontmatter/append/usage ACs. **Not yet executed** — see the blocker below.

### Coverage gaps / not yet run

- **Plugin integration tests are unrun** because the `lisa-plugin` crate currently
  does not compile (blocker below). They are written against the stable
  `emit_provenance` surface and don't touch the adapter API, so they should pass
  once the crate builds.
- **WASI append not runtime-verified (plan Step 8).** `append_record` uses
  `OpenOptions::append`, which native tests exercise directly, but I could not
  drive the real plugin under Zellij's WASI host (the crate won't build). If the
  host does not honour the append flag, the documented fallback is
  read-existing + rewrite-whole (race-free: the plugin is single-threaded). This
  needs a manual `lisa loop` check before relying on the ledger in production.

## Critical issue for human attention — concurrent same-file collision

Sibling ticket **T-026-01 (routing-frontmatter)** is editing the **same shared
working tree** concurrently and left it non-compiling: it rewrote the plugin's
adapter resolver to return a **`(Box<dyn AgentAdapter>, ResolvedRoute)` tuple** and
added a `model` param to `build_claude_command`/`ClaudeCodeAdapter`, but the
adapter call sites in `lib.rs` (626, 643, 650, 1506, 1598, 1653) and `adapter.rs`
are only half-updated. `cargo test -p lisa-plugin` fails with 13 errors — **all in
`adapter.rs` / adapter call sites; none reference any provenance symbol** (verified
by grepping the error set). My spawn snapshot reads `self.config.client`, not the
adapter, so my code is independent of the tuple change.

**This is the RDSPI "two tickets edit the same file → missing dependency edge"
case:** T-026-01 and T-027-01 both modify `lib.rs`, but no dep edge exists (T-027-01
`depends_on: [T-023-02]`). I deliberately did **not** touch the sibling's in-flight
`adapter.rs`/`build_claude_command` work. **A human should decide the merge/commit
order** — likely land T-026-01's adapter rewrite first, then this ticket; both
crates then compile. My changes are additive and conflict-free with the sibling
except by coexisting in `lib.rs`.

## Open concerns / follow-ups (non-blocking)

- **Wire `requested` vs `actual` to real routing (integrates with T-026-01).**
  Today `requested == actual == Route::from_client(thread.client)` — correct per
  the ticket ("before routing lands, requested == actual == loop default"). Once
  T-026-01's `ResolvedRoute` (with `requested_agent`, `substituted`, `model`) is
  wired at spawn, snapshot it onto the `Thread` so the ledger records real
  fallbacks and the `model` field. **No schema change needed** — both routes and a
  nullable `model` already exist.
- **Peak concurrency** is not recorded (only at-spawn). The ticket's "and/or"
  makes this sufficient; peak can be added later as an additive nullable field.
- **Cost fidelity** is deferred to T-027-02 as intended (Claude cost investigation
  + Codex cost-field confirmation). The ledger already carries nullable `cost_usd`.

## Risk assessment

Low, for this ticket's own surface. All new fields are `#[serde(default)]` /
`Default` (no migration); all six emission calls are additive one-liners
(reverting restores exact prior behaviour); no new dependencies; the signal
contract and agent frontmatter are untouched. The only real risk is the
cross-ticket `lib.rs` collision, which is a coordination/merge-order concern, not
a defect in this code.
