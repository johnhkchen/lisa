# Research — T-051-03-01 late-usage-join

Descriptive map of the capture → attribution → ledger machinery this ticket touches.
No solutions here.

## The problem, stated in code terms

Usage capture is now durable and, by design, **late**: `retire_resting_sessions`
(the poll cycle, `lib.rs:7577`) rests a finished session before ending it so its
Stop-hook `capture-usage` can flush. The terminal `ProvenanceRecord` for that
ticket is written earlier, at completion detection, in `emit_provenance_with_note`
(`lib.rs:6654`). So the capture row always lands in `captures.jsonl` **after** the
ledger row is already written — `tokens_in`/`tokens_out` are null by construction.
Both 0718 field legs show all-null token columns despite healthy capture files.

## Producer side (CLI) — unchanged by this ticket

- `crates/lisa-cli/src/capture_usage.rs` — `lisa capture-usage`, invoked by both
  native TUI `Stop` hooks. `capture_usage_from` (`:154`) reads a `StopPayload`
  (`transcript_path`, `session_id`), reads `LISA_PANE_ID`, sums transcript usage,
  and appends a `CaptureRecord` to `.lisa/<client>/captures.jsonl` (`:233`).
  Empty/unreadable transcripts append a marker to `no-captures.jsonl` — never a
  fabricated zero.
- `crates/lisa-core/src/capture.rs` — `CaptureRecord { pane_id: u32,
  session_id: String, captured_at: u64, input_tokens: u64, output_tokens: u64 }`
  (`:16`) and `append_capture_record` (`:35`, append-only). The capture knows
  the pane and provider session it observed, and when — **not** the ticket.

## The ledger and its schema

- `crates/lisa-core/src/provenance.rs`. `SCHEMA_VERSION = 8` (`:42`). (The doc
  `docs/knowledge/provenance-ledger.md` still says 6 — stale.)
- `ProvenanceRecord` (`:151`) is the terminal execution row: `ticket_id`,
  `attempt_lease: AttemptLease {ticket_id, attempt_id}`, `outcome`,
  `authoritative`, `pane_id`, `started_at`, `ended_at`, and the nullable
  `tokens_in/out/cost_usd` (`:179`–`:186`, doc: "never fabricated").
- `ProvenanceLedgerRecord` (`:299`) is an **untagged** enum replaying a mixed
  ledger: `NoteAcknowledgment | AssignmentTransition | ParkingTransition |
  TriageTransition | ProposalAction | Execution`. Non-execution shapes carry a
  required disjoint `record_type` discriminator; `Execution` is the fallback
  (no discriminator). New shapes are added the same way and must stay disjoint.
- `append_record`/`append_*_record` (`:343`+) all funnel through
  `append_serialized` — one compact JSON line, true append, parent dirs created.
  There is **no update/patch path**; published rows are immutable by construction.

## Consumer side (plugin) — where this ticket lives

All scheduler logic is in `crates/lisa-plugin/src/lib.rs` (~24k lines). There is
no `scheduler.rs`. The "pane×time attempt history" is **not** an in-memory
struct; it is reconstructed from durable `Execution` rows in the ledger plus the
record currently being completed, in `crates/lisa-plugin/src/ownership.rs`.

### Attribution today: `owner_at` (`ownership.rs:17`)

```rust
pub(crate) fn owner_at<'a>(records, pane_id, captured_at) -> Option<&'a str>
```
Returns the unique `ticket_id` whose `[started_at, ended_at]` (inclusive) covers
`captured_at` on `pane_id`. Overlap by two different tickets fails closed to
`None`. **Critical limitation for this ticket:** the window is closed at
`ended_at`. A capture that lands *after* the ticket's `ended_at` (exactly the
rest-before-retire case) is **not** covered — so the winning capture can never be
attributed by this window.

### The scan: `read_usage` (`lib.rs:6735`)

Called synchronously from `emit_provenance_with_note` (`:6711`) at completion:
1. Read all of `.lisa/<client>/captures.jsonl`.
2. Rebuild `prior_records: Vec<ProvenanceRecord>` from the ledger (Execution rows
   only; assignment/parking/triage/proposal/note excluded — `:6749`).
3. Per capture on the current pane with `captured_at <= current.ended_at`
   (`:6772`): resolve `owner_at(prior_records + current, pane, captured_at)`.
   - owner == current ticket → sum tokens.
   - owner == another ticket → `continue` (silently skipped — **orphaned**).
   - `None` → `quarantine_capture` (`:6796`).
4. Return summed totals, stamped onto the row before the single append.

No cursor/offset — the whole file is rescanned each completion. Idempotency for
quarantine is delegated to the quarantine module (dedup by `source_line`);
attribution just re-sums.

**The two gaps this creates:**
- The current ticket's own capture isn't present yet at completion → row null.
- A later completion rescans and sees the earlier ticket's now-present capture,
  but `owner != current` → silently skipped forever. Tokens never reach the
  ledger.

### Quarantine (`quarantine.rs`)

- `QuarantinedCaptureRecord { source_line: u64, capture: CaptureRecord }` (`:16`).
- `session_path(provider_dir, session_id)` → `<dir>/quarantine/<encoded>.jsonl`
  (`:37`), per-session buckets, path-safe percent-encoding.
- `append(provider_dir, source_line, capture)` (`:66`) is idempotent by
  `source_line`: identical row → `AlreadyPresent`, differing content at the same
  line → hard error.
- `quarantine_capture` (`lib.rs:6819`) logs an `ActivityEvent::Warning` on
  `Appended`. **There is no drain / re-examination path anywhere** — quarantine is
  write-only and terminal today.

### Session linkage

The scheduler does **not** track the provider `session_id` for an attempt (no
`LISA_SESSION`, no session-per-ticket in the plugin). `session_id` exists only on
the capture and is used **only** for quarantine bucketing. Attribution is purely
`pane_id × captured_at` against ledger windows.

### Poll cycle hook point

The poll body (`lib.rs` ~`:7560`–`7616`) runs `sweep_stale_slots`,
`retire_resting_sessions` (`:7577`), `audit_threads`, `schedule_ready_tickets`,
`request_world_recheck`, then logs a `PollSummary`. This is the natural place to
run a periodic capture pass. Live pane occupancy is available via `self.threads:
HashMap<String, Thread>`, each `Thread` carrying `pane_id`, `started_at`
(`SystemTime`), `client`, and `attempt_lease: Option<AttemptLease>`.

## Readers — nothing surfaces tokens today

- `crates/lisa-cli/src/status.rs` — no token/usage/cost reads at all. Reads the
  ledger only for parked remedies and notes; delegates a run summary to
  `run_summary.rs`.
- `crates/lisa-cli/src/run_summary.rs` — parses only `ticket_id` and `outcome`
  from a byte-offset segment; never touches tokens.
- `crates/lisa-plugin/src/ui.rs` — does not read the ledger for usage.

So the token columns currently have **no consumer**: values written to rows are
never summed or displayed. AC #3/#4 therefore require adding a reader surface,
not just fixing the writer.

## Existing tests that pin current behavior (to be revisited)

In `lib.rs` `#[cfg(test)]`: `provenance_codex_usage_flows_into_record` (`:22363`),
`provenance_recycled_pane_attributes_capture_sums_to_each_ticket` (`:22400`, whose
comment "captures after A's closed interval must remain pending for B" encodes the
now-obsolete closed-window assumption), `provenance_field_repro_*` (`:22516`),
`provenance_unattributable_capture_is_quarantined_by_session_and_visible`
(`:22795`), `provenance_claude_record_has_null_tokens` (`:22914`),
`provenance_claude_usage_flows_into_record` (`:22943`). Pure-unit tests in
`ownership.rs` (`:88`, `:109`) and `quarantine.rs` (`:132`, `:162`).

## Constraints and assumptions surfaced

- Append-only, no mutation of published rows: the join must be a **new** record.
- Never fabricate: a missing capture stays null (`capture-never`), and that gap
  must be countable.
- Attribution key is pane×time; `session_id` is available for quarantine only.
- The winning capture's `captured_at` is strictly **after** the owner's
  `ended_at` (rest-before-retire), so any fix must reason about a pane "reign"
  that extends past `ended_at` until the next occupant, and must not misattribute
  a capture produced by a *currently live but not-yet-recorded* successor.
- `WASM` build + `cargo test --workspace` + clippy + fmt are the CI gates.
