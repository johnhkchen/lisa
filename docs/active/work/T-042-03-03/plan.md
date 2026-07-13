# Plan: operator recovery test matrix

## Objective

Create a cohesive seven-case native regression matrix for the `[d]one`
operator recovery command.

Prove every accepted transition through explicit operator authority and stable
correlation.

Prove every refused transition through its named rejection and matching
correlation.

Keep production behavior unchanged.

## Step 1: register the focused test module

Modify `crates/lisa-plugin/src/lib.rs` inside the existing test module.

Add `mod operator_recovery_matrix;` beside the other focused test modules.

Do not alter production imports or module declarations.

Verification: the new test file is discovered by the Rust compiler.

## Step 2: build the base Review fixture

Create `crates/lisa-plugin/src/tests/operator_recovery_matrix.rs`.

Import the parent test module with `use super::*`.

Create a temporary tickets directory and canonical work directory.

Write one Review-phase ticket with no dependencies.

Scan it into a real DAG.

Configure state paths and write a canonical Pass disposition.

Verification: an orphaned Review appears when the `d` key opens MarkDone.

## Step 3: add shared gesture and assertion helpers

Add an active Review helper with thread, slot, and current lease.

Add a `d` then Enter submission helper.

Add a Pending assertion helper that checks source, authority, effect, modal,
and correlation.

Add a rejection assertion helper that checks structured activity and exact
modal projection.

Verification: helpers compile without exposing new production APIs.

## Step 4: implement active and orphaned Review rows

For active Review, install the attempt and submit from the key handler.

Assert one operator-owned Pending transition.

Assert current attempt records remain untouched.

For orphaned Review, submit without thread or lease.

Assert the same operator-owned Pending transition.

Assert no attempt authority is invented.

Verification: both tests pass independently under focused filters.

## Step 5: implement blocked-disposition row

Overwrite the canonical verdict with a valid Block disposition and explicit
reason.

Submit through `d` and Enter.

Assert no pending transaction or launch effect exists.

Assert `DispositionBlocked`, non-empty operator correlation, and reason detail
in both Activity and modal state.

Verification: the test fails if the operator bypasses Review disposition.

## Step 6: implement stale-attempt row

Create attempt 1 as the thread and slot stamp.

Mint and install attempt 2 only as current scheduler authority.

Submit through `d` and Enter.

Assert operator Pending succeeds and uses `operator`, not either attempt ID.

Assert no stale-lease rejection appears.

Assert stale records and current authority are not mutated by submission.

Verification: the test fails if MarkDone borrows attempt authority.

## Step 7: implement already-pending row

Submit one successful operator request.

Capture its generation correlation.

Open MarkDone again and submit the same ticket.

Assert `AlreadyPending` with the same correlation.

Assert the rejection remains visible and no second effect launches.

Verification: exactly one effect remains recorded.

## Step 8: implement launch-failure row

Configure a non-empty journal path and leave `lisa_bin` absent.

Submit through `d` and Enter.

Assert `LaunchFailed` with non-empty operator correlation and actionable
command-build detail.

Assert no pending entry or effect survives.

Verification: the modal and Activity fields match exactly.

## Step 9: implement successful-recovery row

Start from active Review.

Submit and capture Pending correlation.

Update the real ticket file to phase/status Done.

Feed a successful result with a valid hexadecimal commit-shaped stdout value.

Assert modal Accepted carries the original correlation.

Assert pending transaction, thread, and slot reservation are cleared.

Assert rebuilt DAG state is Done.

Verification: the test fails unless success crosses durable Done verification.

## Step 10: format and focused verification

Run `cargo fmt --all -- --check` after formatting the new source.

Run the focused operator matrix with a test-name or module filter.

If a fixture exposes a production assumption, adjust the fixture and document
the deviation in `progress.md`.

Do not change production logic merely to simplify tests.

Verification: all seven matrix rows pass.

## Step 11: regression verification

Run `cargo test -p lisa-plugin`.

Run `cargo test --workspace` or `just check` for the repository gate.

Inspect failures for unrelated pre-existing worktree artifact pollution before
changing ticket source.

Verification: all relevant native suites and checks pass.

## Step 12: document implementation progress

Write `progress.md` in the private attempt work directory.

Record every completed step and verification command.

Record deviations, if any, before proceeding past them.

Record unrelated worktree entries that were intentionally preserved.

Verification: progress accurately matches the source diff and test results.

## Step 13: commit the meaningful source unit

Use `lisa commit-ticket --ticket-id T-042-03-03`.

Use a test-focused commit message.

Include only `crates/lisa-plugin/src/lib.rs` and
`crates/lisa-plugin/src/tests/operator_recovery_matrix.rs`.

Do not use ordinary Git staging or commit commands.

Verification: the two ticket-owned source paths are clean after the isolated
transaction.

## Step 14: self-review

Inspect the committed diff.

Confirm the seven test names correspond one-for-one with ticket acceptance.

Confirm positive cases assert operator authority and correlation.

Confirm negative cases assert stable kind and correlation.

Confirm no production code changed beyond test-module registration.

Confirm unrelated worktree changes remain untouched.

## Step 15: write Review artifacts

Write `review.md` with change summary, case coverage, verification evidence,
and open concerns.

Write `review-disposition.json` with exactly the valid pass or block shape.

Use pass only if source is committed, all required tests pass, and no critical
gap remains.

Do not edit ticket phase or status.

Stop on the current ticket after both artifacts exist.

## Atomicity

The fixture, helpers, and seven tests form one meaningful test source unit.

The parent module declaration is inseparable from that file because the file
is otherwise not compiled.

They should be committed together in one isolated Lisa transaction.

## Rollback and failure handling

If the focused module cannot compile against private parent items, keep the
same file boundary and add only test-private wrappers in the parent module.

If the launch-failure fixture reaches journal persistence before command
construction, select an earlier deterministic command-build failure.

If workspace checks expose unrelated failures, capture exact evidence and
separate it from ticket-owned regressions.

If a required matrix row reveals a production defect, document the deviation
before making the smallest production fix and update the source-unit plan.

No destructive Git operation is permitted.
