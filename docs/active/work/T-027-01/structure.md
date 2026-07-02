# T-027-01 — Structure: Provenance Ledger

File-level blueprint. Not code — the shape of the code, with public interfaces,
boundaries, and change ordering.

## Files created

### `crates/lisa-core/src/provenance.rs` (new, ~140 lines incl. tests)

The schema + append I/O, native-testable. Public surface:

```rust
/// The route an adapter resolved to (method, provider, model). model is None
/// until model selection lands (S-026); provider is derived from the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    pub method: String,           // "claude" | "codex" (AgentClient::as_str)
    pub provider: String,         // "anthropic" | "openai"
    pub model: Option<String>,    // None today
}
impl Route {
    /// Derive the route from a resolved client. Both `requested` and `actual`
    /// use this today (requested == actual until routing, T-026-01).
    pub fn from_client(c: AgentClient) -> Route;
}

/// Terminal outcome of a ticket-run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]      // done | failed | timed-out
pub enum RunOutcome { Done, Failed, TimedOut }

/// One append-only ledger record. Timestamps are UTC epoch seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub schema_version: u32,             // = SCHEMA_VERSION
    pub ticket_id: String,
    pub outcome: RunOutcome,
    pub requested: Route,
    pub actual: Route,
    pub started_at: u64,
    pub ended_at: u64,
    pub wall_clock_secs: u64,
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
    pub cost_usd: Option<f64>,
    pub concurrency_at_spawn: usize,
    pub pane_id: u32,
}

pub const SCHEMA_VERSION: u32 = 1;

/// Best-effort token/cost extraction from a Codex `usage` JSON value. Returns
/// (tokens_in, tokens_out, cost_usd), each None when its field is absent.
/// Never fabricates. Public so plugin + tests share one extractor.
pub fn extract_usage(usage: &serde_json::Value)
    -> (Option<u64>, Option<u64>, Option<f64>);

/// Append one record as a single JSON line to `path` (create if absent, create
/// parent dir if absent). True append — never rewrites existing lines.
pub fn append_record(path: &Path, record: &ProvenanceRecord) -> std::io::Result<()>;
```

Internal: a `usize`→epoch helper `system_time_to_epoch(SystemTime) -> u64` (or
reuse the pattern already in `types.rs::system_time_serde`).

### `docs/knowledge/provenance-ledger.md` (new, ~60 lines)

Field table (name · type · nullable · meaning), a worked jq example
(`jq -r 'select(.outcome=="done") | [.ticket_id,.wall_clock_secs] | @tsv'`) and a
duckdb example (`SELECT actual.method, avg(wall_clock_secs) ... FROM
read_json_auto('.lisa/provenance.jsonl') GROUP BY 1`). Referenced from the
`provenance.rs` module doc.

## Files modified

### `crates/lisa-core/src/lib.rs`
Add `pub mod provenance;` (one line, after `pub mod dag;`).

### `crates/lisa-core/src/types.rs`
Add two `#[serde(default)]` fields to `Thread` (struct at line 330):
```rust
#[serde(default)] pub client: AgentClient,
#[serde(default)] pub concurrency_at_spawn: usize,
```
`Thread::new` (line 362) initializes them to `AgentClient::default()` and `0`.
`AgentClient` is already imported (`use crate::client::AgentClient;` line 13).
No signature change → the ~20 `Thread::new(id, pane)` call sites are untouched.
Add one test: `new()` defaults `client == Claude`, `concurrency_at_spawn == 0`,
and the two fields round-trip through serde with older records (missing keys →
defaults).

### `crates/lisa-plugin/src/lib.rs`

1. **Import** (near line 23): add `provenance` to the `lisa_core` uses —
   `use lisa_core::provenance::{self, ProvenanceRecord, Route, RunOutcome};`.

2. **State fields** (near `signal_dir` at line 216): add
   ```rust
   ledger_path: PathBuf,   // /host/.lisa/provenance.jsonl
   codex_dir: PathBuf,     // /host/.lisa/codex (source of Codex usage.json)
   ```
   `State: Default` already exists (tests use `..State::default()`), so empty
   `PathBuf` defaults are transparent to existing tests.

3. **`load()`** (near line 2689 where `signal_dir` is set): add
   ```rust
   self.ledger_path = host.join(".lisa/provenance.jsonl");
   self.codex_dir   = host.join(".lisa/codex");
   ```

4. **Spawn site** (`schedule_ready_tickets`, after `Thread::new` at line 637):
   ```rust
   thread.client = self.config.client;
   thread.concurrency_at_spawn = running_count;   // computed at line 551
   ```

5. **New method `emit_provenance`** (private, alongside the teardown helpers):
   ```rust
   fn emit_provenance(&mut self, ticket_id: &str, outcome: RunOutcome) {
       let Some(t) = self.threads.get(ticket_id) else { return; };
       let route = Route::from_client(t.client);
       let started = system_time_to_epoch(t.started_at);
       let ended   = system_time_to_epoch(SystemTime::now());
       let (tin, tout, cost) = self.read_codex_usage(t.client, ticket_id);
       let record = ProvenanceRecord { /* fields from t + route + usage */ };
       if let Err(e) = provenance::append_record(&self.ledger_path, &record) {
           self.log_activity(ActivityEvent::Error {
               message: format!("provenance write failed for {ticket_id}: {e}"),
           });
       }
   }
   ```
   Reads only — mutates no thread/slot state. `wall_clock_secs = ended - started`
   (saturating). For Claude, `read_codex_usage` returns `(None, None, None)`.

6. **New helper `read_codex_usage`** (private): for `AgentClient::Codex`, read
   `self.codex_dir.join(format!("{ticket_id}.usage.json"))`, parse JSON, pull the
   `.usage` value, and hand it to `provenance::extract_usage`. Any error (missing
   file, bad JSON) → `(None, None, None)`, logged at info. Claude → all `None`.

7. **Six call sites** — insert `self.emit_provenance(&tid, OUTCOME)` immediately
   **before** `self.threads.remove(...)` at each:
   | Site | Location | Outcome |
   |------|----------|---------|
   | `auto_complete_review` | `lib.rs:1358` | `RunOutcome::Done` |
   | `mark_ticket_done` | `lib.rs:2518` | `RunOutcome::Done` |
   | done-sweep in `poll_tick` | `lib.rs:1875` | `RunOutcome::Done` |
   | `check_error_signals` | `lib.rs:1223` | `RunOutcome::Failed` |
   | `check_session_timeouts` | `lib.rs:1718` | `RunOutcome::TimedOut` |
   | `detect_stale_threads` | `lib.rs:1764` | `RunOutcome::Failed` |

   Borrow note: `emit_provenance` takes `&mut self` (it logs). At each site the
   `thread.complete()/fail()` mutable borrow has already ended (separate
   statement), so the added call composes with the existing `release_slot` +
   `remove` sequence without borrow conflict.

8. **Tests** (in the existing `#[cfg(test)] mod tests`): construct `State` with an
   explicit `ledger_path`/`codex_dir` in a tempdir; drive each outcome; assert the
   ledger has one line with the expected `outcome` and `ticket_id`; a second
   run/retry appends a second line (first unchanged); the ticket `.md` file is
   byte-identical before/after emission (frontmatter untouched); a seeded
   `<id>.usage.json` yields non-null tokens for a Codex thread, null for Claude.

## Module boundaries

- `lisa-core::provenance` — pure schema + `std::fs` append + usage extraction.
  Knows nothing of `State`, `Thread`, or the scheduler. Native-testable.
- `lisa-plugin` — owns *when* to emit (the six sites), *where* the files live
  (`/host` paths), and reading the Codex artifact. Builds a `ProvenanceRecord`
  from a `Thread` + config and delegates writing to `lisa-core`.
- `Thread` (lisa-core::types) — gains two spawn-time facts; stays the run's
  single identity object.

## Ordering of changes (each independently compilable/committable)

1. `provenance.rs` + `lib.rs` module decl + its unit tests. (Self-contained;
   `cargo test -p lisa-core` green.)
2. `Thread` fields + `Thread::new` init + serde-default test. (`-p lisa-core`.)
3. Plugin wiring: State fields, `load()`, spawn-site snapshot, `emit_provenance`
   + `read_codex_usage`, six call-site insertions. (`-p lisa-plugin` check +
   `cargo test --workspace`.)
4. `docs/knowledge/provenance-ledger.md`. (Docs; no build impact.)

Steps 1–2 land before 3 so the plugin compiles against a finished `lisa-core`
surface. Step 4 can land anytime.
