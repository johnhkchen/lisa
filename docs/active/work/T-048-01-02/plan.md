# Plan — T-048-01-02 park-instead-of-churn

## Baseline

1. Record the ordinary worktree status and preserve unrelated changes.
2. Run focused current provenance tests.
3. Run a plugin test build or focused scheduler tests to establish compilation.
4. Do not modify or commit ticket/frontmatter and shared work artifacts.

## Step 1 — extend blocked-work provenance

Modify `crates/lisa-core/src/provenance.rs`.

1. Advance the schema constant to 5.
2. Add `Retry` to `ParkingTransitionType`.
3. Add optional retry count and retry limit fields.
4. Add the defaulted `recheck_eligible` field.
5. Broaden comments to describe retry/park/unpark records.
6. Update all existing record literals.
7. Add retry and world-recheck serialization assertions.
8. Add legacy schema-4 replay coverage for absent additive fields.

Verification:

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
cargo check -p lisa-core
```

Commit this meaningful contract unit with Lisa and exact include path:

```text
lisa commit-ticket --ticket-id T-048-01-02 \
  --message "feat(core): record bounded block retries" \
  --include crates/lisa-core/src/provenance.rs
```

## Step 2 — add pure scheduler policy and state

Modify `crates/lisa-plugin/src/lib.rs`.

1. Import remedy owner and parking provenance types.
2. Add the two-retry fixed policy constant.
3. Add the private `ReviewBlockAction` result.
4. Add the pure decision function.
5. Add the per-loop `agent_block_retries` state map.
6. Add a table-style unit test for every owner and boundary count.

Verification:

```text
cargo test -p lisa-plugin review_block_policy --no-fail-fast
```

Keep this unit uncommitted until the effectful scheduler method and regression
tests compile together, since the private API has no standalone production use.

## Step 3 — implement retry and park effects

Continue in `crates/lisa-plugin/src/lib.rs`.

1. Add a validated parking-transition append helper.
2. Add `apply_review_block_policy` after current Review artifact admission.
3. For agent retry, append the exact n/limit row before release.
4. Release slot and remove thread while leaving phase/status open.
5. For park, write blocked status before releasing authority.
6. Append park rows with owner, final bound metadata, and world recheck marker.
7. Keep canonical structured block artifacts untouched.
8. Rebuild the DAG after successful park writes.
9. Integrate the method into polling before completion/timeouts.

Verification criteria:

- a block is processed at most once for one live attempt;
- a failed status write leaves the attempt and seat intact;
- parked tickets have no thread and no assigned slot;
- blocked status excludes them from ready scheduling;
- no completion or Done transition is launched.

## Step 4 — implement unpark provenance reconciliation

Continue in `crates/lisa-plugin/src/lib.rs`.

1. Replay latest parking transition per ticket from the mixed ledger.
2. Identify latest Park plus current open/non-Done ticket status.
3. Append one Unpark row with the prior park interval start and current end.
4. Preserve owner, attempt, retry, and recheck facts.
5. Clear per-loop retry state for the reopened ticket.
6. Call reconciliation after poll DAG rebuild and during initial load.
7. Keep scheduling independent of ledger success.

Verification criteria:

- repeated reconciliation appends no duplicate Unpark;
- malformed/unreadable ledger does not block scheduling;
- status open alone restores DAG eligibility;
- duration uses saturating epoch arithmetic.

## Step 5 — add production-shaped scheduler regressions

Continue in the existing `lib.rs` test module.

1. Add fixture helpers for temporary tickets and current Review attempts.
2. Recreate two occupied seats with operator/world blocks and two queued ready
   tickets.
3. Apply the production policy and scheduling methods.
4. Assert both external blocks become durably blocked.
5. Assert neither external block is reseated on repeated scheduler calls.
6. Assert both seats go to the queued tickets.
7. Assert exact Park rows and world-only recheck eligibility.
8. Exercise an agent block through two retries and a third-attempt park.
9. Assert exact Retry 1/2, Retry 2/2, Park 2/2 ledger order.
10. Reopen the agent ticket, reconcile, and assert Unpark plus scheduling.

Focused verification:

```text
cargo test -p lisa-plugin park_instead_of_churn --no-fail-fast
cargo test -p lisa-plugin agent_owned_block --no-fail-fast
cargo test -p lisa-plugin review_block_policy --no-fail-fast
```

## Step 6 — format and commit scheduler unit

1. Run `cargo fmt --all`.
2. Inspect the exact diff for only the two owned source files.
3. Re-run core provenance and focused plugin tests.
4. Commit the scheduler implementation and tests with Lisa:

```text
lisa commit-ticket --ticket-id T-048-01-02 \
  --message "feat(plugin): park blocked reviews without occupying seats" \
  --include crates/lisa-plugin/src/lib.rs
```

If formatting changes the already committed core path, inspect and commit only
the intentional follow-up through Lisa with that exact path.

## Step 7 — workspace verification

Run:

```text
cargo check --workspace
cargo test --workspace --no-fail-fast
```

Also inspect:

```text
git status --short
git diff -- crates/lisa-core/src/provenance.rs crates/lisa-plugin/src/lib.rs
git diff --cached --name-only
```

The ticket-owned source paths must be clean and unstaged. Unrelated worktree
changes must remain untouched.

## Step 8 — progress and Review

Maintain `progress.md` throughout implementation with:

- baseline results;
- completed units;
- exact Lisa commit IDs and include paths;
- test results;
- deviations and rationale;
- remaining work.

After source and tests are complete, write `review.md` covering:

- files changed;
- runtime ordering and durable authority;
- retry/park/unpark behavior;
- test coverage;
- compatibility and open concerns;
- confirmation that check execution remains out of scope.

Write exact passing disposition only if all required work is ready:

```json
{"disposition":"pass","reason":null}
```

Otherwise write a block disposition with a non-empty actionable reason.

Remain on T-048-01-02 after both Review artifacts are present.
