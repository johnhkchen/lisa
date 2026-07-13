# Structure: restart reconstruction and lost-result fixtures

## Change summary

Modify one existing ticket-owned source file:

`crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

Do not create, delete, or modify production source files.

Do not change `crates/lisa-plugin/src/lib.rs` module registration.

Do not add static fixture files.

The existing test module remains the story-level real-adapter harness.

## Existing module organization

The module currently begins with private imports and fixture constants.

`NestedRepo` owns a temporary real Git repository.

`Scenario` owns the configured adapter and attempt authority.

Small helpers decode adapter argv and create the CLI transaction request.

Two acceptance tests cover passing and blocked hostile order.

The new fixture layer will sit between helper functions and acceptance tests.

This keeps setup types first, derived fixture types second, and tests last.

## Existing types retained

`NestedRepo` remains unchanged.

It continues providing:

- repository lifetime;
- project-root derivation;
- file creation;
- Git command execution;
- Git output decoding;
- commit counting.

`Scenario` remains the authoritative nested-topology setup.

It continues providing:

- primary and dependent ticket creation;
- passing or blocked Review disposition setup;
- adapter configuration;
- thread and slot setup;
- current attempt installation;
- review aging;
- operator input;
- finish-up assertions;
- fresh-state restart construction.

`transaction_request` remains the command-contract boundary.

It continues asserting Git root and nested repository-relative paths.

## New private type: `LostResultFixture`

Add one private struct inside the test module.

It owns a `Scenario`.

It owns the cloned original `PendingCompletion`.

It owns the original typed `EffectCommand`.

It owns the first completion transaction's commit ID.

All fields remain private to the module.

No production interface is exposed.

The type represents the shared prefix of both requested fixture cases.

That prefix ends after Git commit success but before adapter result delivery.

The durable journal is therefore still CommandInFlight.

The repository ticket is therefore already Done.

The provenance ledger therefore does not yet exist.

## `LostResultFixture::new`

Construct a passing `Scenario`.

Call `check_artifact_advances` on the real adapter.

Require the primary thread to be in Review.

Require exactly one launch effect.

Capture the original pending completion and effect.

Assert the aggregate uses the original key.

Assert aggregate state is exact CommandInFlight.

Assert correlation and deadline match pending state.

Assert the journal has two records.

Build the CLI request through `transaction_request`.

Call the real `complete_ticket` transaction.

Store the returned commit ID.

Assert the repository has baseline plus one commit.

Assert the completion commit has the fixture baseline as parent.

Assert committed primary ticket bytes are Done.

Assert no authoritative provenance exists before result confirmation.

Return the owned fixture.

## `LostResultFixture::restart_in_flight`

Create a fresh adapter through `Scenario::restart`.

Assert journal restoration is healthy.

Assert there is no live pending completion.

Assert the reconstructed aggregate key is the original key.

Assert the reconstructed state is exact CommandInFlight.

Assert the restored correlation and deadline equal the original values.

Assert `reconciliation_state` returns the same in-flight state.

Assert rebuilt DAG masks durable Done back to prior Review authority.

Assert the ticket status matches the prior state stored in the aggregate.

Return the fresh State for observation-specific actions.

## `LostResultFixture::replay_time`

Return a deterministic `SystemTime` inside the stored bound.

Read the absolute deadline from the original pending completion.

Subtract one millisecond with saturation.

Add that duration to `UNIX_EPOCH`.

Do not read wall-clock time.

Do not sleep.

This helper makes retained deadline semantics visible to both tests.

## `LostResultFixture::start_replay`

Accept a mutable restarted State.

Dispatch `CompletionInput::Reconcile` with the current lease.

Use `replay_time` as explicit adapter time.

Require the dispatch to launch.

Require exactly one replay effect.

Require the effect to equal the original typed effect.

Require one pending completion for the primary ticket.

Require its generation to equal the original generation.

Require its correlation to equal the original correlation.

Require its deadline to equal the original deadline.

Require `is_reconciliation_replay` to be true.

Require journal line count to remain two.

This method does not execute Git or confirm a result.

## `LostResultFixture::converge`

Accept a mutable restarted State.

Build the same CLI request from the original completion key.

Call the real `complete_ticket` transaction again.

Require returned commit ID to equal the stored first commit.

Require returned committed paths to be empty.

Require commit count to remain baseline plus one.

Deliver the returned commit ID through `handle_completion_result`.

Require pending completion to be removed.

Require aggregate state to be Confirmed.

Require aggregate confirmed commit ID to equal the first commit.

Require journal line count to be three.

Require exactly one Requested record.

Require exactly one CommandInFlight record.

Require exactly one Confirmed record.

Decode the provenance ledger.

Require exactly one Execution record.

Require the record ticket ID to be the primary.

Require outcome Done and authoritative true.

Require repository commit count still baseline plus one.

This method is the shared exactly-once terminal assertion.

## New focused test: restart reconstruction

Name the test for plugin restart reconstruction and prior-commit convergence.

Create `LostResultFixture`.

Call `restart_in_flight`.

Call `start_replay`.

Call `converge`.

Create a second fresh State through `Scenario::restart`.

Do not restore thread release expectations into this final state.

Assert its restored aggregate is Confirmed.

Assert its confirmed commit ID is the stored prior commit.

Assert its reconciliation state is Confirmed.

This second restart establishes durable terminal reconstruction.

## New focused test: lost result and duplicate Stop

Name the test for lost result, duplicate Stop, and prior-commit convergence.

Create `LostResultFixture`.

Call `restart_in_flight`.

Record journal bytes before new observations.

Call `handle_stopped_signal` twice before replay.

Require no pending completion and no launch effect.

Require journal bytes unchanged.

Call `start_replay`.

Call `handle_stopped_signal` twice while replay is pending.

Dispatch a second Reconcile one millisecond later.

Require all duplicate observations to return or remain no-op.

Require only one launch effect.

Require journal bytes still contain only the original two records.

Call `converge`.

Capture final journal and ledger contents.

Deliver the same successful result again.

Require journal and ledger bytes remain identical.

Require commit count remains baseline plus one.

## Existing tests

Keep `passing_review_hostile_order_converges_once_and_schedules_dependent`.

Keep `blocked_review_hostile_order_has_no_completion_side_effects`.

The broad passing test continues proving the entire hostile sequence.

The broad blocked test continues proving disposition refusal.

The new focused cases make two durable sub-sequences independently selectable.

Do not weaken existing assertions to make the helper fit.

## Imports and visibility

Reuse `super::*` for private plugin adapter types.

Reuse current CLI and provenance imports.

Reuse current standard library imports.

No new crate dependency is needed.

No item needs `pub`, `pub(crate)`, or test-only production exposure.

All new symbols remain private to the test module.

## Commit unit

The single meaningful source unit is the modified test module.

Commit exactly:

`crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

Use `lisa commit-ticket` with ticket `T-042-04-02`.

Do not include the private RDSPI artifacts in the source commit.

Do not include Lisa-managed ticket or provenance changes.

Do not include the unrelated untracked plugin docs tree.

## Verification boundaries

Focused filter:

`cargo test -p lisa-plugin --lib fixture --no-fail-fast`.

Module filter:

`cargo test -p lisa-plugin --lib hostile_order_regression --no-fail-fast`.

Plugin library:

`cargo test -p lisa-plugin --lib --no-fail-fast`.

Workspace:

`cargo test --workspace --no-fail-fast`.

Project check:

`just check`.

Formatting:

`cargo fmt --all -- --check`.

Repository hygiene:

`git diff --check` and exact-path status inspection.
