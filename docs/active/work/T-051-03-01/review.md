# Review — T-051-03-01 late-usage-join

## What changed

The provenance ledger's token columns become true via an **append-only late
join** instead of an at-completion stamp that rest-before-retire made always-null.

- **`crates/lisa-core/src/provenance.rs`** — `SCHEMA_VERSION` 8→9. New
  `UsageCorrectionRecord` (+ `UsageCorrectionType`) and the untagged
  `ProvenanceLedgerRecord::UsageCorrection` variant, `append_usage_correction_record`,
  and the pure corrected-view fold `correct_usage` → `BTreeMap<ticket, TicketUsage>`
  plus `usage_gap`.
- **`crates/lisa-plugin/src/ownership.rs`** — replaced the closed-window
  `owner_at` with reign attribution (`reign_owner_at`): an occupant reigns from
  its `started_at` until the pane's next occupant, so a post-`ended_at` rest
  capture attributes to its owner. Live threads bound an open reign
  (`Pending`); a same-ticket start tie prefers the durable record; different-ticket
  ties fail closed (`Unowned`).
- **`crates/lisa-plugin/src/quarantine.rs`** — `drain` (temp-write + rename,
  delete-when-empty) and a `#[cfg(test)]` `count_quarantined`.
- **`crates/lisa-plugin/src/lib.rs`** — `emit_provenance_with_note` now writes
  null tokens by construction; `read_usage` removed; `sweep_usage_captures`
  (run each poll after `retire_resting_sessions`) reconciles
  `captures.jsonl` against the durable ledger: attribute → append a correction
  (idempotent by `(method, source_line)`) and drain any quarantine straggler;
  live-owned → skip; unowned → quarantine by session id.
- **`crates/lisa-cli/src/status.rs`** — a "Token usage" block on `lisa status`
  reading only the corrected view: per-ticket totals, an aggregate, and the gap
  count.
- **`docs/knowledge/provenance-ledger.md`** — usage-correction shape, corrected-view
  rule, jq example, schema-v9 note (and corrected the stale "current version 6").

## Test coverage

- **Core (`provenance.rs`):** correction serialize/round-trip, mixed-ledger
  distinctness, `correct_usage` override/sum/legacy-fallback/null, `usage_gap`.
- **Ownership:** rest-capture attribution, recycled-pane split, live→Pending,
  pre-first/other-pane Unowned, ambiguous tie Unowned, duplicate-identity retries,
  and the resting-session completed-over-live tie (both orders).
- **Quarantine:** drain-target-only, drain-last-deletes-file, drain-absent
  non-mutating, count across buckets.
- **Sweep (integration, `lib.rs`):** the three AC fixtures — join-after-completion
  with byte-for-byte untouched original row; quarantine→drain with the terminal
  capture staying counted; capture-never null + countable gap — plus recycled-pane
  per-ticket sums, the migrated field-repro (7 recycles + unowned quarantine +
  rescan idempotency), and the resting-session join. The migrated inline tests now
  assert the correction mechanism; the null-by-construction test stays.
- **Status:** corrected-view reading (corrections override raw), gap counting,
  empty-ledger state.

Gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets` (0 warnings),
`cargo test --workspace` (23 suites, 0 failures), `wasm32-wasip1` release build —
all green. Manually verified `lisa status` renders the Token usage block against
the real ledger (4 legacy tickets via fallback, 138-ticket gap, no fabricated
zeros).

## Acceptance criteria → evidence

1. Correction joins after completion, row bytes untouched —
   `sweep_correction_leaves_original_row_bytes_untouched`,
   `provenance_codex_usage_flows_into_record`.
2. Unowned quarantines by session id; late-attributable drains; unattributable
   stays quarantined and countable —
   `sweep_drains_quarantine_when_attribution_arrives_and_keeps_terminal`.
3. capture-never stays null, gap countable on `lisa status` —
   `sweep_leaves_capture_never_null_and_gap_counts_it`,
   `token_usage_counts_the_capture_never_gap`.
4. Per-ticket totals read from the corrected view, not the raw row —
   `token_usage_reads_the_corrected_view_not_the_raw_row`, `correct_usage`.
5. WASM + workspace tests + clippy + fmt green — see Gates above.

## Open concerns / limitations

- **Sweep cost per poll.** The sweep re-reads both `captures.jsonl` files and the
  whole ledger every ~5s poll and is O(captures × execution-rows). The old
  `read_usage` did the same work per completion; here it recurs per poll. Fine at
  solo scale; a future optimization could gate on capture-file mtime or a
  processed cursor. Not a correctness issue (idempotent by `(method, source_line)`
  and the corrected view *overrides* rather than adds).
- **Legacy non-null rows.** Pre-0.4.4 rows with inline tokens surface via the
  corrected-view fallback (used only when a ticket has no corrections), so
  historical data is preserved without backfill (backfill is explicitly out of
  slice).
- **Terminal quarantine count is not CLI-surfaced.** `count_quarantined` is
  test-only; the operator-facing gap number on `lisa status` is the ledger-derived
  count of completed-but-unjoined tickets. The unattributable-capture count lives
  as persisted quarantine rows (countable, per AC #2) but is not printed. Left as a
  deliberate scope boundary.
- **Cost and Claude cache-split** remain `None`/out of scope per the story.
