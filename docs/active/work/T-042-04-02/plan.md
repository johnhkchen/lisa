# Plan: restart reconstruction and lost-result fixtures

## Objective

Add two independently selectable real-adapter fixtures to the predecessor's
nested-repository regression harness.

Prove both rediscover the original completion commit.

Prove both create one authoritative Done result.

Prove reconstructed CommandInFlight retains its deadline and ends Confirmed.

Keep all production code and contracts unchanged.

## Step 1: establish the source baseline

Inspect the exact current test module.

Confirm its module registration remains present in `src/lib.rs`.

Confirm the ordinary Git index has no staged ticket-owned source.

Record unrelated modified and untracked paths so they can be preserved.

Verification:

- `hostile_order_regression.rs` is committed at the predecessor baseline;
- `src/lib.rs` already declares the module;
- no ticket-owned source change exists before implementation.

## Step 2: add the shared lost-result prefix fixture

Edit `hostile_order_regression.rs` with `apply_patch`.

Add private `LostResultFixture` after transaction request construction.

Give it owned Scenario, original pending completion, original effect, and first
commit ID fields.

Implement `new` with the real artifact adapter and CLI transaction.

Make construction stop before adapter result delivery.

Assert the journal remains exactly Requested plus CommandInFlight.

Assert the repository has exactly one new completion commit.

Assert no provenance exists yet.

Verification:

- fixture construction models a genuinely lost result;
- it does not synthesize journal records;
- it derives Git argv from the real adapter;
- its repository uses `games/midsummer` below the Git root.

## Step 3: add exact restart reconstruction helper

Implement `restart_in_flight`.

Use the existing Scenario restart constructor.

Assert restoration health.

Assert absence of live pending memory.

Assert exact completion generation identity.

Assert exact CommandInFlight correlation.

Assert exact retained absolute deadline.

Assert the durable Done bytes are masked to Review in the rebuilt DAG.

Verification:

- the fresh adapter derives authority from the journal;
- no process-local pending entry is carried over;
- reconstructed state matches the pre-restart aggregate exactly.

## Step 4: add deterministic replay helper

Implement `replay_time` from the stored deadline minus one millisecond.

Implement `start_replay` using `dispatch_completion_at`.

Dispatch a real typed Reconcile input with the current attempt lease.

Assert one replay launch.

Assert the replay effect equals the initial effect.

Assert generation, correlation, and deadline are unchanged.

Assert replay pending state is named as reconciliation replay.

Assert the journal remains at two records.

Verification:

- replay is inside the original bounded window;
- replay does not mint a new generation;
- replay does not append new intent or in-flight evidence.

## Step 5: add shared exactly-once convergence helper

Implement `converge`.

Call `complete_ticket` again from real adapter-derived argv.

Assert it returns the first commit ID.

Assert it reports no committed paths.

Assert Git commit count remains baseline plus one.

Deliver the result through `handle_completion_result`.

Assert CommandInFlight ends Confirmed.

Assert the aggregate records the first commit ID.

Assert the journal has exactly one transition of each successful state.

Assert the ledger has one authoritative Done execution record.

Verification:

- Git idempotency is established with commit identity and count;
- adapter terminal state is durable and named;
- authority is emitted once only after correlated confirmation.

## Step 6: add plugin restart reconstruction fixture test

Create a test with a name containing `fixture` and restart reconstruction.

Build the shared lost-result prefix.

Restart and assert exact CommandInFlight reconstruction.

Start one replay and converge on the prior commit.

Construct another fresh state after confirmation.

Assert Confirmed reconstructs with the same prior commit ID.

Assert reconciliation state is Confirmed.

Verification:

- a focused test filter selects the case;
- both in-flight and terminal restart states are proven;
- exactly one authoritative Done exists.

## Step 7: add lost-result/duplicate-Stop fixture test

Create a test with a name containing `fixture` and duplicate Stop.

Build the shared lost-result prefix.

Restart and capture original journal bytes.

Submit repeated Stop observations before replay.

Assert they create no pending invocation and no launch effect.

Start one replay.

Submit repeated Stop and Reconcile observations while it is pending.

Assert they create no second launch and no journal append.

Converge on the first commit.

Submit a duplicate successful result after confirmation.

Assert journal, ledger, and commit count remain unchanged.

Verification:

- duplicate Stop is inert both before and during replay;
- duplicate Reconcile is inert while pending;
- duplicate command result is inert after confirmation;
- one prior commit and one authoritative Done remain.

## Step 8: format and run focused tests

Run `cargo fmt --all` as a mechanical formatting operation.

Run:

`cargo test -p lisa-plugin --lib fixture --no-fail-fast`.

If the filter catches unrelated tests, inspect the exact test list.

Run:

`cargo test -p lisa-plugin --lib hostile_order_regression --no-fail-fast`.

Resolve only test-code defects within this ticket's scope.

If production behavior violates acceptance, document the failure and block.

Verification:

- both new focused fixtures pass;
- both predecessor hostile-order cases continue to pass;
- no sleep or network dependency exists.

## Step 9: inspect the implementation diff

Review the exact test-file diff.

Check that production files are unchanged.

Check that assertions cover generation, correlation, deadline, commit identity,
commit count, Confirmed state, and authoritative ledger count.

Run `git diff --check`.

Run `cargo fmt --all -- --check`.

Verification:

- the change is test-only;
- no whitespace error exists;
- no acceptance boundary is asserted indirectly when direct evidence exists.

## Step 10: run broader pre-commit verification

Run the entire plugin native library:

`cargo test -p lisa-plugin --lib --no-fail-fast`.

This catches shared private-helper assumptions and test interference.

Verification:

- all plugin library tests pass;
- temporary repositories clean themselves up;
- no global mutable state leaks between fixtures.

## Step 11: commit the meaningful source unit

Locate the repository-built Lisa binary if PATH Lisa lacks `commit-ticket`.

Run Lisa's isolated transaction with:

- ticket ID `T-042-04-02`;
- message `test(plugin): add restart replay fixtures`;
- exact include `crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

Do not run ordinary `git add`.

Do not run ordinary `git commit`.

Inspect the resulting commit with `git show` or `git diff-tree`.

Verification:

- the commit contains exactly one source path;
- the ordinary index remains untouched;
- the source file is clean after commit.

## Step 12: run full repository verification

Run:

`cargo test --workspace --no-fail-fast`.

Run:

`just check`.

Run:

`cargo fmt --all -- --check`.

Run:

`git diff --check`.

Verification:

- all workspace tests pass;
- WASM target checking passes;
- repository formatting remains valid;
- no ticket-owned source is left dirty.

## Step 13: document progress

Write private `progress.md`.

Record implemented fixture structure.

Record each focused acceptance assertion.

Record commit ID and exact included path.

Record all verification command outcomes.

Record any plan deviation before proceeding if one occurred.

Do not publish progress directly to shared work paths.

## Step 14: review and disposition

Inspect final commit and worktree status.

Write private `review.md` summarizing source, coverage, evidence, and limitations.

Write `review-disposition.json` with the exact valid pass or block shape.

Use pass only if all acceptance evidence and verification are green.

Use block with a non-empty actionable reason for any unresolved failure.

Remain on this ticket after both Review artifacts exist.

Do not edit ticket phase or status.

Do not start the dependent live-seat ticket.
