# Structure — T-051-03-01 late-usage-join

Blueprint of file-level changes. Six edited files, no new files, no deletions.
Ordering is bottom-up: core types → pure attribution → plugin sweep → CLI
surface → docs.

## 1. `crates/lisa-core/src/provenance.rs` (MODIFY)

The schema + corrected-view home; both writers (plugin) and readers (CLI) share
it.

- Bump `SCHEMA_VERSION`: `8` → `9`.
- New discriminator enum:
  ```rust
  #[derive(…kebab-case)]
  pub enum UsageCorrectionType { UsageCorrection }
  ```
- New record:
  ```rust
  pub struct UsageCorrectionRecord {
      pub schema_version: u32,
      #[serde(default)] pub seal: CompletionSeal,
      pub record_type: UsageCorrectionType,   // "usage-correction"
      pub ticket_id: String,
      pub attempt_lease: AttemptLease,         // owner's exact attempt
      pub method: String,                      // "claude" | "codex" — disambiguates source_line
      pub session_id: String,
      pub pane_id: u32,
      pub source_line: u64,                    // 1-based line in captures.jsonl (idempotency)
      pub captured_at: u64,
      pub tokens_in: u64,                      // non-null: a correction only exists with real tokens
      pub tokens_out: u64,
      pub occurred_at: u64,
  }
  ```
- `ProvenanceLedgerRecord`: add `UsageCorrection(UsageCorrectionRecord)` **before**
  the fallback `Execution` variant (disjoint required `record_type` keeps it
  distinguishable in the untagged enum).
- `pub fn append_usage_correction_record(path, record) -> io::Result<()>` via the
  existing `append_serialized`.
- **Corrected view** (pure, no I/O):
  ```rust
  pub struct TicketUsage { pub tokens_in: Option<u64>, pub tokens_out: Option<u64>,
                           pub correction_count: usize }
  pub fn correct_usage<'a>(records: impl IntoIterator<Item = &'a ProvenanceLedgerRecord>)
      -> std::collections::BTreeMap<String, TicketUsage>
  ```
  Fold: seed each ticket from its authoritative `Done` Execution row (raw tokens,
  possibly `None`); layer corrections by `ticket_id` — if `correction_count > 0`,
  tokens = summed corrections (override), else raw fallback. Saturating adds.
- `pub fn usage_gap<'a>(records) -> Vec<String>`: authoritative-`Done` ticket_ids
  whose corrected `tokens_in` is still `None`, sorted. (Countable gap, AC #3.)
- Tests: correction row serializes single-line with `record_type`; round-trips;
  mixed-ledger replay keeps a correction distinct from Execution; `correct_usage`
  overrides raw with corrections, sums multiple corrections, falls back to raw
  legacy tokens, and leaves capture-never `None`; `usage_gap` counts only null
  completed tickets.

## 2. `crates/lisa-plugin/src/ownership.rs` (MODIFY — reign rewrite)

Replace the closed-window `owner_at` with the reign model.

- ```rust
  pub(crate) enum ReignSource<'a> {
      Completed(&'a ProvenanceRecord),   // durable, attributable
      Live { ticket_id: &'a str },       // in-flight thread, pending
  }
  pub(crate) struct Reign<'a> { pub pane_id: u32, pub started_at: u64, pub source: ReignSource<'a> }
  pub(crate) enum ReignOutcome<'a> { Attributed(&'a ProvenanceRecord), Pending, Unowned }
  pub(crate) fn reign_owner_at<'a>(reigns: &'a [Reign<'a>], pane_id: u32, captured_at: u64)
      -> ReignOutcome<'a>
  ```
- Algorithm: over reigns on `pane_id` with `started_at ≤ captured_at`, take the
  greatest `started_at`; if two entries tie on it with different tickets →
  `Unowned` (fail closed); winner `Completed(r)` → `Attributed(r)`; winner
  `Live` → `Pending`; empty → `Unowned`.
- Remove `owner_at` and its two tests; add reign tests covering: post-`ended_at`
  rest capture → owner; recycled pane splits by successor `started_at`; live
  successor → `Pending`; pre-first-occupant → `Unowned`; ambiguous tie →
  `Unowned`; duplicate-identity retries → owner.

## 3. `crates/lisa-plugin/src/quarantine.rs` (MODIFY)

- `pub(crate) enum DrainOutcome { Drained, Absent }`
- `pub(crate) fn drain(provider_dir, session_id, source_line) -> io::Result<DrainOutcome>`:
  rewrite the session file without that `source_line` (atomic temp-write +
  rename); delete the file when it becomes empty; `Absent` when the line/file
  isn't present.
- `pub(crate) fn count_quarantined(provider_dir) -> usize`: total quarantined
  rows across all session files under `quarantine/` (terminal-countable signal).
- Tests: drain removes exactly the target line and keeps siblings; drain of the
  last row deletes the file; drain of an absent line is `Absent` and non-mutating;
  `count_quarantined` sums across sessions.

## 4. `crates/lisa-plugin/src/lib.rs` (MODIFY — core wiring)

- `emit_provenance_with_note`: delete the `read_usage` call and the
  token-rebuild; the terminal row is appended with `tokens_in/out/cost_usd = None`
  always. (Row null by construction.)
- **Delete** `fn read_usage`.
- **Add** `fn sweep_usage_captures(&mut self)`:
  1. Load ledger once → `prior_records: Vec<ProvenanceRecord>` (Execution) and
     `already_corrected: HashSet<(String /*method*/, u64 /*source_line*/)>` from
     existing `UsageCorrection` rows.
  2. Build live reigns from `self.threads` (each thread's `pane_id`,
     `started_at`→epoch, `ticket_id`).
  3. For each client dir (claude, codex) with a `captures.jsonl`: enumerate rows
     (1-based `source_line`), parse `CaptureRecord`, build the `Reign` slice
     (completed rows for this pane + live threads for this pane), resolve
     `reign_owner_at`:
     - `Attributed(rec)` and `(method, source_line) ∉ already_corrected` → append
       a `UsageCorrectionRecord`; on success add to the set and log an
       `ActivityEvent::Info`. Then `quarantine::drain(...)` for this capture
       (best-effort; log drains).
     - `Attributed(rec)` already corrected → still attempt `drain` (idempotent
       cleanup of a late-drained straggler).
     - `Pending` → skip.
     - `Unowned` → `self.quarantine_capture(client, source_line, &capture)`
       (existing, idempotent).
  4. Guard checked-`u64` source-line conversion exactly as the old code did.
- Keep `fn quarantine_capture` unchanged.
- Call site: in the poll body add `self.sweep_usage_captures();` immediately after
  `self.retire_resting_sessions();` (`~lib.rs:7577`).
- **Tests to rewrite** (inline-attribution → correction mechanism):
  `provenance_codex_usage_flows_into_record`,
  `provenance_recycled_pane_attributes_capture_sums_to_each_ticket`,
  `provenance_field_repro_*`, `provenance_claude_usage_flows_into_record` — assert
  tokens land as correction rows and the corrected view sums per ticket, with
  original rows untouched.
  `provenance_claude_record_has_null_tokens` — now the *steady* state; keep.
  `provenance_unattributable_capture_is_quarantined_by_session_and_visible` —
  drive via the sweep.
- **New sweep tests** (the three AC fixtures):
  - `sweep_joins_capture_after_completion_via_correction` — complete a ticket
    (null row), land its capture after `ended_at`, sweep → one correction, raw row
    bytes unchanged, corrected view reports the ticket's tokens.
  - `sweep_quarantines_unowned_then_drains_when_attribution_arrives` — capture
    with no owning record → quarantined; add the covering Execution row → sweep
    drains it to a correction; a second unowned capture with no future owner stays
    quarantined and countable.
  - `sweep_leaves_capture_never_null_and_countable_gap` — completed ticket with no
    capture → no correction, corrected `None`, `usage_gap` counts it; no zero.

## 5. `crates/lisa-cli/src/status.rs` (MODIFY — reader surface)

- Add `fn print_token_usage(ledger_path: &Path)` (and a testable
  `fn token_usage_lines(records) -> Vec<String>`): read
  `.lisa/provenance.jsonl` → `Vec<ProvenanceLedgerRecord>`, call
  `correct_usage` + `usage_gap`, render a "Token usage" block:
  - per-ticket joined totals (tickets with `Some` tokens), sorted;
  - an aggregate line (`Joined: N tickets, X in / Y out`);
  - the gap line (`Not yet joined: M completed tickets` — omitted when 0).
  Brand voice: plain, host-facing, no jargon.
- Call from `run_status` after the config summary block (`~:169`), before waves.
- Tests: lines reflect the corrected view (corrections override raw), gap counts
  null completed tickets, empty ledger prints an empty/"nothing yet" state.

## 6. `docs/knowledge/provenance-ledger.md` (MODIFY)

- Correct the stale current version to **9**; add a "Usage-correction record
  shape" section (field table + example line) and a "Corrected view" subsection
  explaining corrections override raw row tokens, the capture-never null rule, and
  the gap count. Add a jq example summing corrected per-ticket tokens. Extend the
  Versioning section: v9 adds the usage-correction row and the late join.

## Interfaces & boundaries

- `lisa-core` owns the schema and the pure corrected-view fold — the single
  definition both the plugin writer and CLI reader depend on. No plugin/CLI types
  leak into core.
- `ownership.rs` stays pure (borrows records/thread ids; no `State`), so it is
  unit-testable in isolation; the plugin constructs `Reign`s from its live state.
- `quarantine.rs` owns its on-disk format including drain; the plugin only calls
  append/drain/count.
- The sweep is best-effort and non-fatal (log-and-swallow), matching every other
  ledger write in the poll loop.

## Change ordering

1 (core record + view) → 2 (reign) → 3 (quarantine drain) → 4 (sweep + wiring +
tests) → 5 (status) → 6 (doc). Each of 1–3 compiles and tests green on its own;
4 depends on 1–3; 5 depends on 1; 6 is docs.
