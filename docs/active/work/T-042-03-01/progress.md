# Progress: operator-requested authority emission

## Status

Implementation is complete.
Focused, crate-level, workspace, formatting, and WASM checks pass.
The ticket-owned source unit is ready for the isolated Lisa commit.

## Completed: explicit operator source type

Added private `OperatorRequestSource` in `crates/lisa-plugin/src/lib.rs`.
The initial variant is `MarkDoneKey`.
It identifies the dashboard `[d]one` interaction.
It derives the same lightweight traits used by completion diagnostic state.

Changed `CompletionSource::Manual` to
`CompletionSource::OperatorRequested(OperatorRequestSource)`.
Pending completion records now retain the exact operator surface.
The source is inspectable by native tests and Debug-formatted activity.

## Completed: impossible attempt borrowing

Replaced `CompletionInput::Manual { ticket_id, authority }` with
`CompletionInput::OperatorRequested { ticket_id, source }`.
The new input has no authority field.
It cannot carry `CompletionAuthority::Attempt`.
It cannot carry an `AttemptLease`.

The dispatch adapter maps OperatorRequested directly to:

- `CompletionSource::OperatorRequested(source)`;
- `CompletionAuthority::Operator`;
- no review lease;
- stable reducer identity `operator`.

This mapping is independent of `State::threads`.
An active thread and its attempt lease are no longer consulted.
An orphaned ticket follows exactly the same authority mapping.

## Completed: canonical E-040 disposition gate

Extracted `State::passing_review_disposition`.
It parses the canonical `review-disposition.json` for a ticket.
Pass authorizes continuation.
Block maps to the existing `DispositionBlocked` rejection with its reason.
Invalid or missing input maps to fail-closed `DispositionBlocked` detail.

`admit_passing_review` still performs current-attempt artifact admission first.
It now delegates canonical verdict evaluation to the extracted helper.
No attempt fencing or staged artifact publication rule changed.

OperatorRequested calls the canonical helper before invoking the reducer.
It does not admit or copy a private attempt artifact.
This keeps operator authority separate from attempt artifact authority.
A blocked or missing verdict creates no reducer effect.
The existing correlated rejection activity path records the failure.

## Completed: dependency enforcement

The sole completion effect executor retains `Dag::all_dependencies_done`.
No alternate launch path was added.
An operator request that passes disposition but has unmet dependencies produces
the existing `DependencyBlocked` rejection.
No pending completion is inserted and no host command is launched.

Updated executor authority validation.
Operator authority is accepted only for an OperatorRequested source.
Updated completion-result authority validation with the same invariant.
Attempt-driven sources retain current-lease validation unchanged.

## Completed: `[d]one` command construction

Simplified `State::mark_ticket_done`.
It no longer reads `self.threads` or a thread's attempt lease.
It always dispatches `CompletionInput::OperatorRequested`.
Its source is always `OperatorRequestSource::MarkDoneKey`.

The modal selection and filtering logic was not changed.
The modal close timing was not changed because T-042-03-02 owns that behavior.
Thread and agent-slot cleanup timing was not changed.

## Completed: active Review regression

Updated `test_mark_done_keeps_thread_and_slot_until_commit_result`.
The fixture now represents an active Review with canonical Pass evidence.
It installs a real current attempt lease and assigned slot.
It drives the actual `d` key followed by modal Enter confirmation.

The test asserts:

- pending authority is Operator;
- pending source is OperatorRequested(MarkDoneKey);
- the completion generation does not use the installed attempt ID;
- the emitted effect uses reducer identity `operator`;
- the running thread and assigned slot remain pending commit result;
- ticket frontmatter remains Review before commit success.

## Completed: orphaned Review regression

Updated `test_mark_done_without_active_attempt_uses_operator_authority`.
The fixture has no thread and no current attempt.
It supplies canonical Pass disposition evidence.
Calling mark-done creates an operator pending completion and effect.
The test asserts exact MarkDoneKey source and stable `operator` identity.

This directly proves that no live thread is required.

## Completed: refusal regression

Added
`test_operator_requested_refuses_blocked_disposition_and_unmet_dependencies`.

The blocked-disposition half uses an active Review and installed attempt lease.
Its canonical disposition is Block with an actionable reason.
The request produces no pending state and no effect.
The attempt lease remains installed and untouched.
Activity contains a correlated DispositionBlocked event with the reason.

The dependency half uses an orphaned Review with canonical Pass disposition.
It depends on an unfinished Implement ticket.
The request produces no pending state and no effect.
Activity contains a correlated DependencyBlocked event.
Both ticket files remain in Review.

## Completed: retry regression alignment

Renamed the existing failed-manual completion test to failed-operator completion.
Added canonical Pass evidence required by the fail-closed operator gate.
Updated its exact source assertion to OperatorRequested(MarkDoneKey).
The retry, assignment retention, provenance, and single-release assertions remain.

## Verification results

`cargo fmt --all -- --check` passed.

Focused mark-done tests passed:

- 2 passed, 0 failed for `test_mark_done` filter;
- 1 passed, 0 failed for `operator_requested` filter;
- 1 passed, 0 failed for `failed_operator_completion` filter.

`cargo test -p lisa-plugin` passed:

- 359 passed;
- 0 failed;
- 0 ignored.

`cargo test --workspace` passed across all workspace crates and doc tests.
The output included all 359 lisa-plugin tests and the CLI/core suites.
No test failure or warning was reported.

`cargo check -p lisa-plugin --target wasm32-wasip1` passed.
The plugin compiles for its production WASM target.

## Deviations from plan

The active-thread regression was strengthened to drive `handle_key` for `d`
and modal Enter, rather than calling only `mark_ticket_done`.
This more directly covers the acceptance wording about pressing `[d]one`.

No architectural deviation was required.
No lisa-core change was required.
No UI renderer change was required.
No dependency was added.

## Remaining actions

Commit `crates/lisa-plugin/src/lib.rs` through `lisa commit-ticket`.
Confirm no ticket-owned source changes remain.
Write Review artifacts and a pass/block disposition.
Remain on T-042-03-01 for Lisa's completion publication.
