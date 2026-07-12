# Plan: T-039-02-03

## Objective

Add a post-refactor regression suite that locks the typed signal-ingestion
contract, scheduler poll interleaving, and current-attempt admission while
retaining the predecessor characterization suite unchanged.

## Step 1: establish the source baseline

1. Record `git status --short`.
2. Confirm only Lisa-managed ticket/provenance files are already dirty.
3. Record the current hash of the characterization file.
4. Run the existing characterization module before editing.
5. Run the existing signal module tests before editing.
6. Treat any baseline failure as a blocker rather than masking it with new tests.

Verification:

- Existing characterization tests pass.
- Existing signal unit tests pass.
- No ticket-owned source path is dirty.

## Step 2: create the regression module skeleton

1. Create `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`.
2. Import the parent test namespace.
3. Import local filesystem and time utilities.
4. Define a pane ID and ticket ID unique to the suite.
5. Add a temporary signal-directory state fixture.
6. Add a running current-attempt fixture using established parent helpers.
7. Add a vector normalization helper only if multi-record assertions need it.
8. Add `mod signal_ingestion_regression;` beside the characterization declaration.

Verification:

- The new empty/skeleton module compiles under `cargo test -p lisa-plugin`.
- No production visibility changes are introduced.

## Step 3: lock the typed request/record contract

1. Write one valid heartbeat lease record.
2. Request `Heartbeats` and assert exact `Heartbeat` output.
3. Repeat for process start and shell readiness.
4. Use the same typed lease value for clear comparisons.
5. Write a raw provider acknowledgement payload that is not lease JSON.
6. Assert exact raw payload preservation.
7. Write awaiting and error records with arbitrary bodies.
8. Assert pane-only variants with no payload representation.
9. Write pane-scoped and legacy idle records.
10. Assert both `IdleTarget` variants.
11. Write stopped and cleared records.
12. Assert both transition variants.
13. Normalize only the two multi-record result vectors.
14. Assert recognized paths are consumed.

Verification:

- Every `SignalRequest` appears in the test.
- Every `SignalRecord` variant appears in expected output.
- Both `IdleTarget` variants appear.
- Raw provider payload equality is exact.
- Directory order cannot cause flakes.

## Step 4: lock recognition and deletion distinctions

1. Create `pane-seven.heartbeat`.
2. Ingest heartbeats and assert the malformed strict path remains.
3. Create a valid pane heartbeat with invalid lease JSON.
4. Assert it produces no record and is deleted.
5. Create a valid pane acknowledgement with arbitrary raw text.
6. Assert it produces a raw record and is deleted.
7. Create a legacy acknowledgement path.
8. Assert it remains because legacy naming belongs only to idle.
9. Create `pane-seven.idle`.
10. Assert it is deleted without producing a record.
11. Create `pane-seven.stopped`.
12. Assert it is deleted without producing a record.
13. Create an unrelated idle record during transition ingestion.
14. Assert the unrelated record remains.

Verification:

- Strict invalid-name retention is explicit.
- Strict recognized malformed-payload consumption is explicit.
- Broad malformed-target consumption is explicit.
- Legacy compatibility does not spread beyond idle.
- Request selectivity is explicit.

## Step 5: lock ingestion versus current-attempt admission

1. Build a state with one bound Codex slot and running thread.
2. Install one current `AttemptLease` in all authority locations.
3. Set both attention and awaiting gates for the pane.
4. Save the initial slot/thread activity facts.
5. Construct a same-ticket lease with a different attempt ID.
6. Serialize the stale lease to a heartbeat file.
7. Call `signal::ingest` directly.
8. Assert it returns a typed stale `Heartbeat` record.
9. Assert the direct-ingestion file is deleted.
10. Recreate the stale file.
11. Call `check_heartbeat_signals`.
12. Assert no activity or gate effect is admitted.
13. Write the exact current lease.
14. Call the consumer again.
15. Assert pane and thread activity advance.
16. Assert both gates clear.

Verification:

- Typed deserialization is not currency admission.
- Scheduler currency rejection is observable.
- Current lease effects remain observable.
- Both stale and current paths remain one-shot.

## Step 6: lock the complete poll interleaving

1. Read `lib.rs` with `include_str!`.
2. Slice the `poll_tick` method body.
3. Define the ordered expected calls.
4. Include heartbeat and awaiting first.
5. Include `deliver_ready_assignments` before process start.
6. Include process start, shell ready, and acknowledgement.
7. Include `check_artifact_advances` before idle.
8. Include idle, transition, and error.
9. Include transition timeout after error.
10. Include acknowledgement timeout after transition timeout.
11. Walk the source remainder with sequential `split_once`.
12. Emit a named panic on missing or reordered calls.

Verification:

- Reordering consumers fails.
- Moving delivery across process-start ingestion fails.
- Moving artifact advancement across idle ingestion fails.
- Moving timeouts before their admitting signals fails.
- Comment-only changes do not fail.

## Step 7: run focused tests

1. Run `cargo fmt --all`.
2. Run the new regression module by name.
3. Run `cargo test -p lisa-plugin signal::tests`.
4. Run the retained characterization module by name.
5. Inspect any failure against the researched contract.
6. Fix only ticket-owned test code unless a real production defect is proven.

Verification:

- New regression tests all pass.
- Existing boundary tests all pass.
- Existing characterization tests all pass unchanged.

## Step 8: run broad validation

1. Run `cargo test --workspace`.
2. Run `cargo clippy --workspace --all-targets -- -D warnings`.
3. Run `cargo fmt --all -- --check`.
4. Run `just check` for the repository's combined native/WASM gate.
5. Run `git diff --check`.
6. Record command outcomes in `progress.md`.

Verification:

- Full workspace suite is green.
- Clippy is green with warnings denied.
- Formatting is clean.
- WASM check succeeds through repository automation.
- No whitespace errors exist.

## Step 9: inspect scope and preservation

1. Review the new regression file in full.
2. Review the `lib.rs` diff.
3. Confirm the characterization file has no diff.
4. Compare its hash with the baseline hash.
5. Confirm no production code changed.
6. Confirm no ticket phase/status edit was made manually.
7. Confirm the ordinary Git index has no entries.
8. Write completed steps and deviations to `progress.md`.

Verification:

- Source diff contains exactly the intended test unit.
- Characterization is retained byte-for-byte.
- Existing unrelated worktree changes remain untouched.

## Step 10: commit the meaningful source unit

Use Lisa's isolated transaction with exact paths:

```text
lisa commit-ticket \
  --ticket-id T-039-02-03 \
  --message "T-039-02-03: lock signal ingestion regressions" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/tests/signal_ingestion_regression.rs
```

Do not run `git add`, `git add -A`, ordinary `git commit`, or any command that
uses the ordinary index for ticket work.

Verification:

- The command succeeds and reports a commit.
- Both ticket-owned paths are clean afterward.
- The ordinary index remains empty.
- Lisa-owned provenance/ticket changes remain outside the source commit.

## Step 11: final verification and review

1. Inspect the created commit and exact paths.
2. Re-run focused regression tests if the isolated commit changes the worktree.
3. Confirm all ticket-owned source files are clean.
4. Complete `progress.md` with commit and validation results.
5. Write `review.md` in the attempt-private work directory.
6. Summarize files, behavior locked, test coverage, and concerns.
7. Remain on this ticket after writing Review.
8. Do not publish artifacts or start another ticket.

## Planned commit units

One meaningful source unit is expected. The new test module depends on its module
declaration, so committing both exact paths together is atomic and reviewable.
No production unit or manifest unit is planned.

## Deviation policy

If a new regression exposes a production mismatch, document it in `progress.md`
before modifying production source. Add the exact new path to the isolated commit
only when the correction is within this ticket's stated ingestion boundary. If
the mismatch belongs to excluded failure, timeout, or publication behavior, stop
and report it rather than expanding scope.
