# Progress: operator recovery test matrix

## Status

Implementation is complete.

All seven required matrix cases are present and passing.

Focused plugin tests pass.

The complete plugin test suite passes.

The repository `just check` gate passes.

The source unit is ready for the isolated Lisa commit.

## Completed: project and workflow intake

Read `CLAUDE.md` as the project source of truth.

Read `AGENTS.md` and followed its pointer to project context.

Read the complete private assignment for T-042-03-03.

Read the ticket and parent story.

Read the complete RDSPI workflow definition.

Confirmed that all six phases must run continuously.

Confirmed that phase artifacts belong in the private attempt directory.

Confirmed that source commits must use `lisa commit-ticket` with exact paths.

Confirmed that ticket phase and status are Lisa-owned.

## Completed: Research

Mapped the `[d]` key path through `open_mark_done_modal` and Enter handling.

Mapped `mark_ticket_done` to typed `CompletionInput::OperatorRequested`.

Mapped operator input to `CompletionAuthority::Operator`.

Mapped stable operator completion identity and correlation generation.

Mapped disposition, dependency, reducer, authority, and launch gates.

Mapped `PendingCompletion` and modal Pending/Accepted/Rejected projections.

Mapped the durable success boundary in `handle_completion_result`.

Mapped predecessor tests and identified their scattered coverage.

Identified stale attempt records as the missing isolated story case.

Identified focused child test modules as the repository organization pattern.

Wrote `research.md` in the private attempt directory.

Lisa admitted the artifact without manual ticket frontmatter edits.

## Completed: Design

Evaluated relying on existing scattered tests.

Evaluated expanding and renaming predecessor tests in place.

Evaluated a single table-driven test with scenario branching.

Selected a focused module with shared fixtures and seven named tests.

Selected the actual `d` then Enter key path for every row.

Defined accepted-request assertions over authority, source, effect, modal, and
correlation.

Defined rejection assertions over exact kind, correlation, detail, and modal
projection.

Defined stale-attempt semantics as accepted operator recovery independent of
attempt authority.

Defined successful recovery at verified durable Done result handling.

Wrote `design.md` in the private attempt directory.

Lisa admitted the artifact without direct shared-path writes.

## Completed: Structure

Defined one modified file: `crates/lisa-plugin/src/lib.rs`.

The only change in that file is the focused test-module declaration.

Defined one new file:
`crates/lisa-plugin/src/tests/operator_recovery_matrix.rs`.

Defined constants, fixture builder, active-attempt helper, gesture helper,
Pending assertion helper, and rejection assertion helper.

Defined one named test per acceptance-matrix row.

Defined one isolated commit containing both inseparable source paths.

Wrote `structure.md` in the private attempt directory.

Lisa admitted the artifact through its normal phase handling.

## Completed: Plan

Sequenced module registration before focused compilation.

Sequenced fixture and helper creation before scenario tests.

Sequenced active and orphaned positive cases first.

Sequenced blocked, stale, duplicate, launch-failure, and success cases next.

Defined focused, plugin-wide, and repository-wide verification gates.

Defined exact source include paths for the isolated commit.

Defined review and disposition completion requirements.

Wrote `plan.md` in the private attempt directory.

Lisa admitted the artifact and advanced into implementation.

## Completed: test module registration

Added `mod operator_recovery_matrix;` to the existing parent test module.

Placed it beside the existing focused signal test modules.

No production module registration changed.

No production imports changed.

No public API changed.

## Completed: base fixture

Created a real temporary `T-OPERATOR` Review ticket.

Created temporary ticket and canonical work directories.

Scanned the ticket into a real `Dag`.

Configured the plugin state with those paths.

Wrote canonical Pass disposition evidence by default.

Kept the base state free of thread, slot, and attempt authority.

Used the base state directly for orphaned Review.

## Completed: shared helpers

Added an active Review helper with a Codex thread and assigned slot.

Reused the parent `install_current_attempt` helper for exact stamping.

Added a gesture helper that sends bare `d` then Enter.

The gesture helper verifies modal selection before submission.

Added a stable expected operator-correlation helper.

Added a Pending helper that proves operator authority and source.

The Pending helper proves exactly one inert launch effect.

The Pending helper proves modal correlation equals pending generation.

Added a rejection helper that locates structured Activity evidence.

The rejection helper proves modal and Activity fields are identical.

## Completed: active Review row

Created running Review thread, assigned slot, and current attempt.

Submitted through the real key handler.

Asserted operator-owned Pending with stable correlation.

Asserted the active lease remains installed on scheduler, thread, and slot.

Asserted the operator correlation is not the attempt correlation.

## Completed: orphaned Review row

Started with no thread and no current lease.

Submitted through the real key handler.

Asserted the same operator-owned Pending transition.

Asserted submission does not manufacture attempt authority.

## Completed: blocked disposition row

Replaced Pass with a valid Block disposition and actionable reason.

Submitted through the real key handler.

Asserted no pending transaction and no launch effect.

Asserted exact `DispositionBlocked` kind.

Asserted stable operator correlation and matching reason detail.

Asserted rejection remains visible in the modal.

## Completed: stale attempt row

Created attempt 1 as the thread and slot stamp.

Minted attempt 2 as checked successor.

Installed attempt 2 as current scheduler authority only.

Submitted while thread and slot remained stale at attempt 1.

Asserted operator-owned Pending rather than `StaleLease`.

Asserted current and stale records were not mutated by submission.

Asserted no stale-lease activity was emitted.

## Completed: already-pending row

Submitted the first operator request and captured its correlation.

Opened a fresh MarkDone selection over the live pending transaction.

Submitted the duplicate through Enter.

Asserted exact `AlreadyPending` kind.

Asserted rejected correlation equals original Pending correlation.

Asserted exactly one launch effect remains.

## Completed: launch-failure row

Configured a non-empty completion journal path.

Kept `lisa_bin` unconfigured to trigger deterministic command-build failure.

Submitted through the real key handler.

Asserted exact `LaunchFailed` kind and operator correlation.

Asserted actionable missing-binary detail.

Asserted no pending transaction or launch effect survived.

## Completed: successful recovery row

Started from active Review and submitted operator recovery.

Captured the Pending correlation.

Updated the real ticket file to durable Done.

Fed a zero exit and valid 40-character hexadecimal result.

Asserted Accepted uses the original correlation.

Asserted pending state cleared.

Asserted thread removal and slot release.

Asserted rebuilt DAG phase and status are Done.

## Verification results

`cargo fmt --all` completed successfully.

`cargo test -p lisa-plugin operator_recovery_matrix -- --nocapture` passed.

Focused result: 7 passed, 0 failed, 364 filtered out.

`cargo test -p lisa-plugin` passed.

Plugin result: 371 passed, 0 failed.

`just check` passed.

The WASM target check passed for `wasm32-wasip1`.

The full workspace test suite passed.

All workspace doc tests passed.

`cargo fmt --all -- --check` passed.

`git diff --check` passed.

The ordinary Git index is empty.

## Deviations

There were no production defects requiring a code change.

There were no changes to the planned source file boundaries.

The stale-attempt test explicitly preserves stale thread and slot records while
installing a newer current lease; this is the concrete realization of the
planned lifecycle case.

The workspace already contained Lisa-owned provenance and ticket mutations.

It also contained unrelated untracked plugin-relative disposition artifacts.

Those paths were left untouched and excluded from ticket ownership.

## Completed: isolated source commit

Committed the meaningful source unit through `lisa commit-ticket`.

Commit: `8ecf773f02077455f63c0a0f891d84a330823398`.

Message: `test(plugin): cover operator recovery matrix`.

The isolated transaction included exactly
`crates/lisa-plugin/src/lib.rs` and
`crates/lisa-plugin/src/tests/operator_recovery_matrix.rs`.

The commit contains 327 inserted lines across those two paths.

No ordinary Git staging or commit command was used.

Post-commit inspection shows both ticket-owned source paths are clean.

The ordinary Git index remains empty.

## Remaining

Write `review.md` and `review-disposition.json`.

Remain on T-042-03-03 after Review artifacts are complete.
