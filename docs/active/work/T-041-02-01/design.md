# Design: recorded Review livelock regression

## Decision

Add one public integration test module at
`crates/lisa-core/tests/recorded_livelock_regression.rs`. The module will encode
the field sequence as a typed fixture, replay it through a small driver backed
by `reconcile` and `reduce`, and compare it with a deliberately naive
edge-triggered model.

The production completion module remains unchanged.

## Goals

- Preserve the recorded event order in readable test data.
- Exercise the public completion aggregate contract rather than private helpers.
- Count completion requests and authoritative confirmations independently.
- Prove pending and confirmed states suppress re-requests.
- Prove an existing Review suppresses the fixture's timeout finish-up action.
- Demonstrate that the same trace catches the historical edge-triggered mistake.
- Keep runtime adapter behavior outside this pure-domain ticket.

## Option 1: add a colocated unit test to completion.rs

This would provide direct access to private helpers and follow the module's
existing test placement. It would also modify the settled production source
file, contrary to S-041-02's explicit no-production-change boundary. The
follow-on generated ticket needs a disjoint test module for parallel ownership.

Rejected because the public API is sufficient and integration placement better
proves the contract available to consumers.

## Option 2: add a plugin scheduler regression

This could replay real stop, timeout, pane, and reload behavior. It would be the
closest reproduction of the field incident, but E-041 deliberately stops at
the pure domain contract. E-042 owns plugin adapter wiring, command execution,
journal behavior, operator authority, and the live Arcade-shaped harness.

Rejected because it expands the ticket into explicitly deferred production
integration and would not be a read-only proof of S-041-01.

## Option 3: add a declarative trace integration test

The test defines a local `RecordedEvent` enum with the six incident milestones:
artifact written, phase advanced, stopped, timeout elapsed, reload, and manual
result. A driver retains phase/artifact facts and aggregate state. At every
safe observation after Review, it reconciles durable eligibility.

Chosen because it makes chronology reviewable, consumes only public APIs, and
keeps synthetic adapter observations visibly separate from core state.

## Trace representation

Use a constant or function returning this exact ordered sequence:

1. `ReviewArtifactWritten`
2. `PhaseAdvancedToReview`
3. `StopObserved`
4. `ReviewTimeoutElapsed`
5. `Reloaded`
6. `ManualCompletionConfirmed`

The timeout event name should retain the recorded approximately ten-minute
meaning, either in a variant field or comment. No wall clock or sleeping is
needed; elapsed time is evidence metadata, not reducer input.

## Aggregate-backed driver

The driver starts with:

- phase not yet Review;
- artifact absent;
- exact Pass disposition;
- aggregate Eligible;
- no launched correlation;
- all observation counts zero.

Artifact creation sets the durable artifact fact. It does not request while
the phase is not Review. Phase advancement then invokes level-triggered
reconciliation with the already-present admitted artifact.

When reconciliation returns a launch effect, the driver feeds the matching
Request into `reduce`, verifies the returned effect equals the reconciliation
effect, increments the single request counter, and feeds CommandLaunched with a
stable recorded correlation. Aggregate state becomes CommandInFlight.

Stop, timeout, and reload each invoke reconciliation again. In-flight
reconciliation is recorded as actionable but emits no request. The timeout
increments finish-up only if the artifact is absent; it is present here.

The manual confirming result feeds matching CommandSucceeded into `reduce`.
Only a transition newly entering Confirmed increments authoritative Done.
One final reconciliation verifies Confirmed emits no new request.

## Request accounting

Count a completion Request when the driver successfully applies
`CompletionEvent::Request` and observes the exact launch effect. This is the
aggregate transition requested by the acceptance criterion, not an actual
process invocation.

Track re-requests separately when reconciliation tries to emit after the first
request. The expected value is zero. This makes failure diagnostics more
specific than merely asserting total requests equals one.

## Confirmation accounting

Count authoritative confirmation only when a successful reducer transition
changes CommandInFlight to Confirmed with the matching correlation. Repeated
polling of Confirmed does not increment it. The expected value is exactly one.

## Finish-up accounting

Finish-up is not a core effect. The driver records it as synthetic adapter
output on `ReviewTimeoutElapsed` only when no Review artifact exists. The
recorded artifact is already present, so expected finish-ups are zero.

This does not claim production plugin suppression is already wired. It proves
the intended decision alongside the pure aggregate trace and keeps the honest
boundary explicit in test naming/comments.

## Naive comparison model

The naive stub requests only on `ReviewArtifactWritten` when phase is already
Review. It does not re-check artifact presence on phase advance, stop, timeout,
or reload. On timeout, if no request exists, it emits a finish-up prompt. The
manual result can confirm external completion, but it cannot retroactively
create the missed aggregate request.

For the recorded ordering it should observe:

- zero aggregate requests;
- one authoritative manual confirmation;
- one finish-up prompt.

The test will assert that this observation does not meet the expected contract,
then assert its exact failure shape. This is an executable counterexample, not
a production alternative implementation.

## Error handling in the test

The recorded trace is fixed and every production transition is expected to be
legal. Test helpers may use `expect` with precise invariant messages. Panics
indicate a contract or fixture regression and are appropriate in test code.

All enum matches should remain explicit enough that newly added reconciliation
outcomes cause a compiler-guided update or a clear assertion failure.

## Identity values

Use stable field-derived values such as:

- attempt `T-009-01-01/attempt-1`;
- completion `T-009-01-01/completion-1`;
- correlation `T-009-01-01/manual-result`.

These opaque strings preserve trace attribution without importing scheduler or
lease implementations.

## Assertions

The aggregate replay must assert:

- requests equals 1;
- confirmations equals 1;
- finish-ups equals 0;
- re-requests equals 0;
- final state equals Confirmed.

The naive replay must assert it differs from the expected observation and
specifically exhibits the missed-request plus unwanted-finish-up failure.

## Verification

Run formatting, the focused integration test, all lisa-core tests, workspace
tests, clippy for lisa-core tests, and a diff whitespace check. This ticket does
not own the final release WASM budget gate; T-041-02-03 does.

## Commit boundary

Commit only `crates/lisa-core/tests/recorded_livelock_regression.rs` through
`lisa commit-ticket`. Attempt artifacts, active ticket/provenance changes, and
untracked plugin documentation remain outside the isolated transaction.

