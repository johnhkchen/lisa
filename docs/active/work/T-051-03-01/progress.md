# Progress — T-051-03-01 late-usage-join

All six plan steps complete. Every gate green: `cargo fmt --check`, `cargo clippy
--workspace --all-targets` (no warnings), `cargo test --workspace` (23 suites, 0
failures), and the `wasm32-wasip1` release build.

## Steps executed (each committed via `lisa commit-ticket`)

1. **Core: correction record + corrected view** — `crates/lisa-core/src/provenance.rs`.
   `SCHEMA_VERSION` 8→9; `UsageCorrectionType`, `UsageCorrectionRecord`, the
   `ProvenanceLedgerRecord::UsageCorrection` variant, `append_usage_correction_record`,
   `TicketUsage`, `correct_usage`, `usage_gap`. Bumped the three `":8"` assertions
   to `":9"`. Commit `1d51e07`.

2. **Reign attribution** — `crates/lisa-plugin/src/ownership.rs`. Replaced the
   closed-window `owner_at` with `Reign`/`ReignSource`/`ReignOutcome`/`reign_owner_at`
   (reign-until-next-occupant; live threads → `Pending`; ambiguity → `Unowned`).
   Commit `f24d618`; later hardened to prefer a completed record over the same
   ticket's resting thread on a start tie (commit `ffbea71`).

3. **Quarantine drain + count** — `crates/lisa-plugin/src/quarantine.rs`.
   `DrainOutcome`, `drain` (temp-write + rename, delete-when-empty),
   `count_quarantined` (`#[cfg(test)]` — the on-disk rows are the durable count).
   Commit `c7d650f`.

4. **Sweep + wiring + test migration** — `crates/lisa-plugin/src/lib.rs`.
   `emit_provenance_with_note` now writes null tokens always; `read_usage` deleted;
   `sweep_usage_captures` added and called after `retire_resting_sessions` in the
   poll. Migrated the five inline-attribution tests to the correction mechanism
   and added the AC fixtures (join-after-completion + byte-untouched,
   quarantine→drain + terminal-stays, capture-never null + gap, resting-session
   join). Commit `010d96d`.

5. **Status surface** — `crates/lisa-cli/src/status.rs`. `token_usage_lines`
   (pure) + `print_token_usage`, wired into `run_status`; reads only the corrected
   view; prints per-ticket totals, an aggregate, and the gap count. Commit
   `98875f1`.

6. **Docs** — `docs/knowledge/provenance-ledger.md`. Corrected the stale current
   version to 9, added the usage-correction record shape, the corrected-view rule,
   a jq example, and the v9 versioning note. Commit `b883d34`.

Formatting/clippy fixups landed in `ede4ff4`.

## Deviations from the plan

- **One extra hardening pass (not in the original plan):** running `lisa status`
  from source against the real ledger surfaced the resting-session timing — a
  finished session still sits in `self.threads` when its capture lands, so its
  Done row and its live thread share a `started_at`. Added a same-ticket tie
  preference for the completed record in `reign_owner_at` (order-independent) plus
  two tests. This prevents the winning capture from stalling at `Pending`.
- **`count_quarantined` gated `#[cfg(test)]`** — no production surface counts
  quarantine (the ledger-derived gap on `lisa status` is the operator signal), so
  gating it keeps the non-test build warning-free without inventing a spammy
  per-poll log.

## Verified end-to-end

`lisa status` (built from source) renders the Token usage block against the real
`.lisa/provenance.jsonl`: 4 legacy tickets surface via the raw-row fallback,
13.58M/187K aggregate, and 138 completed tickets counted as the not-yet-joined
gap — no fabricated zeros. This exercises `correct_usage` + `usage_gap` on real
mixed-ledger data.
