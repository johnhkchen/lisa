# Design: auditable operator recovery matrix

## Decision summary

Add one focused native test module containing the seven named operator recovery
cases.

Drive every case through the real `[d]` then Enter key path.

Use a shared single-ticket Review fixture and explicit scenario mutations.

Assert accepted requests through operator-owned pending effects and modal
correlation.

Assert terminal success through the durable result boundary and Accepted modal
state.

Assert refused requests through exact stable rejection kind, non-empty matching
correlation, and actionable detail.

Do not change production completion behavior.

## Goals

The matrix should read as a direct executable version of story acceptance.

Each required lifecycle position should have a clearly named test.

The input gesture should be consistent across cases.

Positive cases should prove operator authority rather than merely a boolean.

Negative cases should prove named correlated evidence rather than absence of
effects alone.

The success case should cross the same durable verification boundary used in
production.

The fixture should remain native, deterministic, and token-free.

## Option 1: rely on the existing scattered tests

The predecessors already left broad operator coverage.

This option has no implementation cost.

It does not provide a directly reviewable seven-row matrix.

The stale-attempt lifecycle position is not isolated.

Several rows call `mark_ticket_done` directly rather than the key handler.

Future changes could preserve each local assertion while breaking the common
operator gesture.

This option does not satisfy the barrier nature of the ticket and is rejected.

## Option 2: rename and expand tests in `lib.rs`

Existing tests could be renamed to match the matrix rows.

Missing assertions and stale-attempt coverage could be added in place.

This would reuse current fixtures with minimal duplication.

The relevant tests are separated by thousands of lines.

Reviewers would still need to reconstruct the matrix manually.

Changing predecessor tests also makes ownership and intent less clear.

This option is viable but rejected in favor of a cohesive module.

## Option 3: parameterize all rows in one table-driven test

A case enum could build and mutate fixtures in a loop.

Common assertions would be compact.

The cases have materially different terminal actions.

Some end Pending, some reject immediately, one rejects on result, and one
confirms after disk mutation.

A single parameterized function would hide scenario-specific invariants behind
branching helper logic.

Rust test failures would report loop positions unless extra diagnostics were
carefully added.

This option is rejected because individual named tests are more auditable.

## Option 4: focused module with shared helpers

Create `src/tests/operator_recovery_matrix.rs`.

Register it from the existing parent test module.

Build one base Review fixture on real temporary ticket and work paths.

Add small helpers for active attempts, stale attempt records, key submission,
accepted request assertions, and rejection lookup.

Give each of the seven story cases its own `#[test]` function.

This option provides direct names, shared mechanics, and scenario-specific
assertions.

It is selected.

## Base fixture

The base fixture creates one `T-OPERATOR` ticket.

The ticket begins with status and phase both Review.

The fixture scans the ticket into a real `Dag`.

The state's ticket and work directories point into a retained `TempDir`.

The canonical disposition is Pass by default.

The completion journal path remains empty for normal stubbed launch behavior.

No thread, slot, or current lease exists in the base fixture.

That base directly represents orphaned Review.

## Gesture helper

The submission helper sends bare `d` through `State::handle_key`.

It asserts that the MarkDone modal opens and selects the fixture ticket.

It then sends Enter through the same handler.

It does not call `mark_ticket_done` directly.

This keeps every row attached to the operator-visible command.

## Accepted request assertions

An accepted-for-execution row must have one live pending entry.

The pending authority must equal `CompletionAuthority::Operator`.

The source must equal `OperatorRequested(MarkDoneKey)`.

The completion key attempt identity must equal `operator`.

Exactly one inert `LaunchCompletion` effect must be recorded.

The modal must remain open in Pending state.

The Pending correlation must equal the pending completion generation string.

These assertions distinguish accepted transition from silent no-op.

## Rejection assertions

A rejection helper finds `ActivityEvent::CompletionRejected` for the fixture
ticket and expected kind.

It asserts a non-empty correlation.

It asserts the expected actionable detail fragment.

For key-driven immediate rejections, it also checks modal Rejected state.

The modal kind, correlation, and detail must equal the activity event values.

This verifies one projection rather than two independently invented messages.

## Case semantics

### Active Review

Add a running Review thread, assigned slot, and current attempt.

Submit through the gesture.

Expect operator-owned Pending and one effect.

Assert the attempt lease remains installed and was not borrowed.

### Orphaned Review

Use the base fixture with no thread, slot, or lease.

Submit through the gesture.

Expect the same operator-owned Pending and correlation.

### Blocked disposition

Replace canonical Pass with a Block document containing a known reason.

Submit through the gesture.

Expect no pending entry or effect.

Expect `DispositionBlocked` and the operator correlation.

### Stale attempt

Add an active Review thread and slot stamped with attempt 1.

Advance scheduler current authority to attempt 2 without updating those stale
records.

Submit through the gesture.

Expect operator-owned Pending, not `StaleLease`.

Assert the pending identity is operator and neither attempt ID.

This proves the recovery command does not borrow stale attempt authority.

### Already pending

Submit once and capture its correlation.

Open a fresh MarkDone modal and submit again.

Expect `AlreadyPending` with the same stable generation correlation.

Expect no second launch effect.

### Launch failure

Configure a non-empty journal path so test mode uses production command-build
failure handling.

Leave `lisa_bin` unconfigured.

Submit through the gesture.

Expect immediate `LaunchFailed`, no pending entry, and no effect.

Expect the modal to retain exact rejection correlation and detail.

### Successful recovery

Start from an active Review and submit successfully.

Capture the Pending correlation.

Mutate the real ticket file to durable Done using the core ticket helper.

Feed a zero exit and a valid hexadecimal commit-shaped stdout value.

Expect Accepted with the same correlation.

Expect pending, thread, and slot ownership to clear.

Expect the rebuilt DAG to report Done.

## Compatibility and risks

The change adds tests only.

No serialized type, host call, or production state transition changes.

The principal risk is coupling to private plugin state.

That coupling is intentional for this native adapter/UI barrier.

Helpers assert domain-visible identities rather than incidental log ordering.

The launch-failure row intentionally selects the same test-mode branch used by
existing production-like completion fixtures.

The success row avoids a real Git transaction, consistent with the story's
stubbed-executor boundary.

Full plugin and workspace tests guard against fixture side effects.
