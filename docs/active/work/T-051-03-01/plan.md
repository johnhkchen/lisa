# Plan — T-051-03-01 late-usage-join

Six ordered steps, each committed atomically via `lisa commit-ticket`. Steps 1–3
compile and test independently; 4 wires them together; 5 adds the reader; 6 is
docs. Gate after each: `cargo test -p <crate>`; full `just check` + WASM build
before Review.

## Step 1 — Core: correction record + corrected view (`lisa-core`)

- `crates/lisa-core/src/provenance.rs`:
  - Bump `SCHEMA_VERSION` 8 → 9.
  - Add `UsageCorrectionType`, `UsageCorrectionRecord`, the
    `ProvenanceLedgerRecord::UsageCorrection` variant (before `Execution`),
    `append_usage_correction_record`.
  - Add `TicketUsage`, `correct_usage`, `usage_gap`.
- Tests (same module): single-line serialize with `record_type`; round-trip;
  mixed-ledger replay distinguishes correction from execution; `correct_usage`
  override/sum/legacy-fallback/null; `usage_gap` counts only null completed.
- Verify: `cargo test -p lisa-core`.
- Commit: `--include crates/lisa-core/src/provenance.rs` —
  "feat(provenance): usage-correction record and corrected token view (schema 9)".

## Step 2 — Plugin: reign-based attribution (`ownership.rs`)

- Replace `owner_at` with `Reign`/`ReignSource`/`ReignOutcome`/`reign_owner_at`.
- Rewrite the module tests for the reign outcomes (post-ended_at, recycled split,
  live→Pending, pre-first→Unowned, ambiguous→Unowned, retry duplicate-identity).
- Verify: `cargo test -p lisa-plugin ownership` (compiles only after Step 4
  removes the old `owner_at` caller; if the crate won't build standalone yet,
  fold this commit's verification into Step 4's gate — keep the commit boundary).
- Commit: `--include crates/lisa-plugin/src/ownership.rs` —
  "feat(ownership): reign-until-next-occupant pane attribution".

## Step 3 — Plugin: quarantine drain + count (`quarantine.rs`)

- Add `DrainOutcome`, `drain` (temp-write + rename, delete-when-empty),
  `count_quarantined`.
- Tests: drain target only; drain-last deletes file; drain-absent is `Absent` and
  non-mutating; count sums across sessions.
- Verify: `cargo test -p lisa-plugin quarantine`.
- Commit: `--include crates/lisa-plugin/src/quarantine.rs` —
  "feat(quarantine): drainable holding area with terminal count".

## Step 4 — Plugin: the sweep + wiring + test migration (`lib.rs`)

- `emit_provenance_with_note`: drop the `read_usage` call; row tokens always
  `None`.
- Delete `read_usage`; add `sweep_usage_captures` (ledger load + already-corrected
  set + live reigns + per-capture Attributed/Pending/Unowned handling with drain).
- Call `self.sweep_usage_captures();` after `retire_resting_sessions` in the poll.
- Migrate the inline-attribution tests to the correction mechanism; keep the
  null-by-construction test; drive the quarantine-visibility test via the sweep.
- Add the three AC fixtures (join-after-completion, quarantine→drain +
  terminal-stays, capture-never null + gap).
- Verify: `cargo test -p lisa-plugin` (this is also where Step 2's ownership tests
  first build).
- Commit: `--include crates/lisa-plugin/src/lib.rs` —
  "feat(scheduler): late-join usage captures as append-only corrections".

## Step 5 — CLI: corrected token surface on `lisa status` (`status.rs`)

- Add `token_usage_lines` (pure) + `print_token_usage`; call from `run_status`
  after the config summary.
- Tests: lines reflect corrected view; gap counts null completed; empty ledger
  handled.
- Verify: `cargo test -p lisa-cli`.
- Commit: `--include crates/lisa-cli/src/status.rs` —
  "feat(status): surface per-ticket tokens from the corrected view".

## Step 6 — Docs: schema v9 + correction/corrected-view (`provenance-ledger.md`)

- Fix stale version; add usage-correction record shape, corrected-view rule,
  gap-count, a jq example; extend Versioning with v9.
- Commit: `--include docs/knowledge/provenance-ledger.md` —
  "docs(provenance): document usage-correction record and corrected view".

## Testing strategy

- **Unit (pure):** `correct_usage`/`usage_gap` (core), `reign_owner_at`
  (ownership), `drain`/`count_quarantined` (quarantine), `token_usage_lines`
  (status). These pin each mechanism in isolation.
- **Integration (plugin sweep):** the three AC fixtures exercise the full path —
  capture file → sweep → correction/quarantine/drain → corrected view — against a
  real tempdir ledger, asserting original row bytes are untouched.
- **Regression:** the migrated inline-attribution tests confirm the mechanism
  moved, not vanished; the null-by-construction and quarantine-visibility tests
  stay green.

## Verification criteria (maps to Acceptance Criteria)

1. AC #1 ← `sweep_joins_capture_after_completion_via_correction`: correction row
   present, raw row bytes identical, corrected view reports tokens.
2. AC #2 ← `sweep_quarantines_unowned_then_drains_when_attribution_arrives`:
   unowned → quarantined by session id; covering row → drains to a correction; a
   never-attributable capture stays quarantined and is counted.
3. AC #3 ← `sweep_leaves_capture_never_null_and_countable_gap` + status test: null
   stays null (no zero), `usage_gap` counts it, `lisa status` prints the count.
4. AC #4 ← status `token_usage_lines` tests read from `correct_usage`, never the
   raw first-write row.
5. AC #5 ← final `just check` (fmt + clippy + `cargo test --workspace`) and
   `cargo build -p lisa-plugin --target wasm32-wasip1 --release` all green.

## Risks & mitigations

- **Test churn in `lib.rs`** is the largest surface — mechanical, bounded to the
  named tests; do it in Step 4 alongside the sweep so the crate builds once.
- **Reign standalone build (Step 2)**: the old `owner_at` caller lives in `lib.rs`;
  removing `owner_at` breaks the crate until Step 4. Keep the commits separate for
  reviewability but treat Step 4's `cargo test -p lisa-plugin` as the gate that
  covers Steps 2+3+4.
- **Sweep cost each poll**: bounded by capture-file + ledger size (the old
  `read_usage` already did this per completion); early-return when no
  `captures.jsonl` exists.
- **Double-count**: prevented by the `(method, source_line)` idempotency set and
  by the corrected view *overriding* (not adding to) raw tokens when corrections
  exist.
