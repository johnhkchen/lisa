# Review: operator recovery test matrix

## Disposition

The implementation is ready to complete.

The ticket acceptance criterion is satisfied by seven explicit passing tests.

No critical issue remains open.

The recommended review disposition is Pass.

## Change summary

This ticket adds a cohesive native regression matrix for the `[d]one` operator
recovery command.

The matrix covers every lifecycle position named by the story and ticket.

Every row drives the real `d` then Enter keyboard path.

Accepted requests assert explicit operator authority and stable correlation.

Rejected requests assert exact stable kind, correlation, and actionable detail.

The terminal success row crosses durable Done verification before acceptance.

No production completion behavior changed.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

Added one declaration inside the existing native test module:
`mod operator_recovery_matrix;`.

No production line in this file changed.

No public or private production API changed.

No host command, reducer, scheduler, journal, or UI behavior changed.

### `crates/lisa-plugin/src/tests/operator_recovery_matrix.rs`

Added a 326-line focused native test module.

Added one real temporary Review-ticket fixture.

Added an active Review thread/slot/lease helper.

Added a real key-gesture helper for `d` then Enter.

Added a stable expected operator-correlation helper.

Added a shared accepted-Pending assertion helper.

Added a shared named-rejection assertion helper.

Added seven named tests corresponding one-for-one with acceptance.

## Matrix coverage

### Active Review

Test:
`active_review_accepts_explicit_operator_recovery_with_correlation`.

Creates a running Review thread, assigned Codex slot, and current attempt.

Submits from the real key handler.

Asserts the pending source is `OperatorRequested(MarkDoneKey)`.

Asserts authority is `CompletionAuthority::Operator`.

Asserts the completion attempt identity is `operator`.

Asserts exactly one launch effect.

Asserts the modal remains Pending with exact correlation.

Asserts active scheduler, thread, and slot attempt records remain unchanged.

Asserts operator correlation is not the attempt correlation.

### Orphaned Review

Test:
`orphaned_review_accepts_operator_recovery_without_attempt_authority`.

Starts with no thread and no current lease.

Submits from the real key handler.

Asserts the same explicit operator-owned Pending transition.

Asserts no thread or attempt authority is created as a side effect.

This proves recovery fires without a live agent.

### Blocked disposition

Test:
`blocked_disposition_rejects_operator_recovery_with_name_and_correlation`.

Writes a canonical valid Block disposition with an actionable reason.

Submits from the real key handler.

Asserts there is no pending transaction or launch effect.

Asserts exact `DispositionBlocked` rejection kind.

Asserts exact stable operator generation correlation.

Asserts Activity and modal detail contain the disposition reason.

This proves operator authority cannot bypass the Review verdict.

### Stale attempt

Test:
`stale_attempt_records_do_not_override_explicit_operator_authority`.

Leaves thread and slot records stamped with attempt 1.

Installs checked successor attempt 2 as current scheduler authority.

Submits from the real key handler while adjacent records remain stale.

Asserts an explicit operator-owned Pending transition is accepted.

Asserts the operator identity is independent of both numeric attempts.

Asserts current and stale attempt records are not mutated.

Asserts no `StaleLease` rejection is emitted.

This directly guards the story's no-silent-attempt-borrowing requirement.

### Already pending

Test:
`already_pending_operator_recovery_rejects_with_name_and_same_correlation`.

Creates one accepted pending operator request.

Submits a duplicate from a fresh MarkDone selection.

Asserts exact `AlreadyPending` rejection kind.

Asserts the rejection uses the original pending generation correlation.

Asserts Activity and modal projections match.

Asserts no second launch effect is recorded.

### Launch failure

Test:
`launch_failure_rejects_operator_recovery_with_name_and_correlation`.

Selects production-like command-build error handling in native test mode.

Leaves `lisa_bin` unavailable for a deterministic launch failure.

Submits from the real key handler.

Asserts exact `LaunchFailed` kind.

Asserts stable operator correlation and actionable missing-binary detail.

Asserts Activity and modal fields match exactly.

Asserts no pending transaction or launch effect survives.

### Successful operator recovery

Test:
`successful_operator_recovery_accepts_the_original_correlation_and_releases`.

Starts from an active Review and accepted Pending request.

Updates the real temporary ticket to durable Done.

Feeds a zero exit and valid hexadecimal commit-shaped output.

Asserts Accepted uses the original Pending correlation.

Asserts Accepted remains visibly open for acknowledgement.

Asserts pending state is removed.

Asserts active thread removal and slot release.

Asserts the rebuilt DAG reports phase and status Done.

## Correlation coverage

The shared expected correlation is built with the production helper.

It uses completion ID `T-OPERATOR`.

It uses attempt identity `operator`.

It uses completion generation 1.

Every accepted Pending case compares its actual key to this value.

Every immediate rejection compares its structured correlation to this value.

The duplicate case also compares rejection correlation to the original live
Pending correlation.

The success case compares Accepted correlation to the original Pending
correlation.

No case accepts a merely non-empty opaque correlation where exact identity is
available.

## Rejection coverage

The ticket specifically names blocked disposition, already-pending, and launch
failure as matrix rejection cases.

All three assert the stable `CompletionRejectionKind` enum value.

All three assert a stable operator correlation.

All three assert actionable detail.

All three assert the modal mirrors the structured Activity event exactly.

The stale-attempt row is intentionally an accepted operator transition, not a
stale rejection.

That behavior proves explicit operator authority does not borrow the stale
attempt lease.

Existing attempt-driven tests continue to cover legitimate `StaleLease`
rejection for non-operator sources.

## Test results

Focused command:
`cargo test -p lisa-plugin operator_recovery_matrix -- --nocapture`.

Focused result: 7 passed, 0 failed, 364 filtered out.

Plugin command: `cargo test -p lisa-plugin`.

Plugin result: 371 passed, 0 failed.

Repository command: `just check`.

WASM result: `cargo check -p lisa-plugin --target wasm32-wasip1` passed.

Workspace result: all unit, integration, and doc tests passed.

Formatting command: `cargo fmt --all -- --check` passed.

Diff hygiene command: `git diff --check` passed.

## Commit and worktree review

Source commit:
`8ecf773f02077455f63c0a0f891d84a330823398`.

Commit message: `test(plugin): cover operator recovery matrix`.

The commit was created with `lisa commit-ticket`.

It includes exactly the two ticket-owned source paths.

It contains one parent-module insertion and one new 326-line test file.

Both ticket-owned source paths are clean after the commit.

The ordinary Git index is empty.

Existing Lisa-owned provenance and ticket mutations remain outside the source
commit.

Unrelated untracked plugin-relative disposition artifacts were not touched.

## Scope assessment

The story's honest boundary specifies native UI/adapter tests over the stubbed
executor.

The implementation stays within that boundary.

It does not launch a live Codex or Claude provider.

It does not spend provider tokens.

It does not duplicate Story B's restart-persistence coverage.

It does not duplicate Story D's live disposable-seat field gate.

It does not change E-040 disposition semantics.

It does not change E-041 reducer semantics.

## Open concerns and limitations

The successful recovery row stubs the external completion transaction result
with commit-shaped bytes after writing durable Done locally.

This is intentional and matches the story boundary.

Real Git transaction behavior remains covered by existing completion
transaction and plugin integration tests.

Live provider-seat behavior remains explicitly assigned to the downstream
field gate, not this ticket.

The test module accesses production-private state through the parent test
module.

That coupling is appropriate for an adapter/UI lifecycle matrix but means
internal refactors may require fixture updates.

Some scenario mechanics overlap predecessor regressions.

The duplication is deliberate: this module is the auditable story barrier and
keeps the seven required cases visible together.

No TODO, skipped test, ignored test, or known flaky timing dependency was
introduced.

No human action is required before completion.

## Final assessment

The implementation proves `[d]one` behavior across all seven named lifecycle
positions.

Positive rows prove explicit accepted operator transitions with correlation.

Negative rows prove named correlated rejections.

Successful recovery proves correlation continuity through durable acceptance
and resource release.

All verification gates pass.

The source is committed through the required isolated transaction.

The ticket is ready for Lisa's completion process.
