# Design: generated completion invariant properties

## Decision summary

Add a black-box integration test in `lisa-core` driven by
`proptest-state-machine`.

Model adapter observations as generated transitions, drive the existing
completion reducer and reconciler as the system under test, and assert safety
invariants after every generated step.

Give every generated trace a convergence probe after each observation so an
admitted passing Review cannot remain eligible without either one live effect
or one authoritative Done.

Keep production completion code unchanged.

## Option 1: independent permutation property

One option is to generate a vector containing the six named observations and
shuffle it with a proptest strategy.

The property could fold this vector through a bespoke simulator.

This directly describes permutation and is compact.

It would use `proptest`, but would not meaningfully use the required
`proptest-state-machine` abstraction.

It also makes shrinking constrained by the fixed multiset rather than by the
semantic validity of model transitions.

It is rejected as the primary test form.

## Option 2: reducer events only

Another option is to generate only `CompletionEvent` values and apply
`reduce`.

This closely tests the production reducer.

However, Review-before-phase, stop, poll, reload, timeout, and manual recovery
are not reducer event variants.

Mapping all of them to arbitrary reducer callbacks would erase the durable
input/reconciliation behavior central to the reported livelock.

It would duplicate existing reducer legality tests without proving the named
adapter-ordering contract.

It is rejected as incomplete.

## Option 3: high-level adapter model over the pure domain

The chosen option defines generated high-level observations:

- ObservePassingReview.
- ObserveBlockedReview.
- EnterReviewPhase.
- Stop.
- Poll.
- DuplicateResult.
- Reload.
- Timeout.
- ManualRecovery.

These transitions update a small harness representing durable adapter facts
and then invoke production `reconcile` and `reduce` where appropriate.

The reference machine tracks only abstract authority and cardinality.

The system under test tracks concrete completion domain values and calls the
actual public functions.

This separates the oracle from implementation details while retaining the
real transition behavior.

It naturally allows arbitrary repetitions and orderings.

It also lets proptest-state-machine shrink a failing trace.

## Admission semantics

Observing Review content is not itself admission.

The harness records an observed disposition separately from phase readiness.

When both Review content and Review phase are present, the adapter admits the
artifact for the fixed current attempt and completion identity.

Therefore Review-before-phase and phase-before-Review converge to the same
durable inputs.

A Block disposition remains durable but never authorizes a launch.

Pass can replace a previously observed Block only as an explicit newly
generated observation.

This models a corrected review artifact without treating Block as completion.

## Reconciliation semantics

Every observation is followed by one level-triggered reconciliation pass.

This represents the scheduler's opportunity to notice durable facts after
polls, reloads, stop processing, and timeouts.

When reconciliation returns `Effect`, the harness applies a Request through
the reducer and counts the returned launch effect.

The resulting Requested state owns one live effect.

The harness then records CommandLaunched with a stable correlation, producing
CommandInFlight.

Repeated reconciliation while Requested, in flight, Confirmed, or blocked
cannot create another effect.

This makes the liveness statement testable after every generated step instead
of only for traces that happen to end with Poll.

## Result semantics

`DuplicateResult` presents a success result for the stable correlation.

If the aggregate is CommandInFlight, the production reducer accepts it and the
harness increments authoritative Done once.

If the aggregate is already Confirmed or is in another state, the reducer
rejects the callback and no Done is counted.

Repeated duplicate results therefore exercise terminal idempotence.

The reference model independently treats the first success for live passing
work as authoritative and later successes as duplicates.

## Reload semantics

Reload reconstructs state from durable harness facts.

Confirmed remains Confirmed because authoritative Done is durable.

An unresolved live command remains CommandInFlight with its correlation.

An admitted pass with no pending command reconstructs Eligible and is
immediately reconciled.

Blocked or unadmitted input reconstructs Eligible but cannot emit an effect.

This design tests that reload does not erase a completion obligation or invent
another live command.

## Timeout semantics

Timeout is observational when no command is live.

For a live in-flight command, production reconciliation returns
`CommandInFlightActionRequired` rather than LaunchCompletion.

The harness records that bounded intervention was requested.

It does not count Done and does not create a second effect.

This directly exercises the state meaning of unresolved work.

## Manual recovery semantics

Manual recovery confirms a currently live correlated command by feeding the
actual `CommandSucceeded` event to the reducer.

If no command is live, manual recovery is a no-op.

This gives manual recovery the same authoritative result gate as an ordinary
completion callback.

Repeated manual recovery cannot increment Done after Confirmed.

This reflects the recorded field path without granting an uncorrelated manual
action extra authority.

## Stop and Poll semantics

Stop records agent inactivity but does not discard Review artifacts or
completion state.

Poll records scheduler observation and invokes reconciliation.

Because every generated observation ends with reconciliation, Stop-before-Poll
and Poll-before-Stop both retain the same durable obligation.

The explicit transitions remain in the generated vocabulary, allowing traces
and shrunk failures to show the named ordering.

## Reference state

The reference state contains:

- Whether phase Review has been reached.
- The latest observed disposition.
- Whether artifact admission exists.
- Whether one completion effect is live.
- Whether authoritative Done exists.
- Counts for effects and authoritative Done.

Its transition strategy selects from all named observations in every state.

No precondition is required because duplicates and premature observations are
part of the behavior under test.

The pure reference apply function updates durable facts and expected
cardinality without invoking production code.

## System-under-test state

The SUT contains the same adapter facts plus concrete `CompletionState`.

It stores stable attempt, completion, and correlation identities.

It counts launch effects returned by production decisions.

It counts accepted success transitions to Confirmed.

It stores the maximum simultaneous live-effect count for direct assertion.

Reload creates a semantically equivalent concrete state from durable facts.

## Properties

After every transition, admitted Pass implies exactly one of:

- a live completion command exists; or
- authoritative Done has been confirmed.

An admitted Block implies authoritative Done is false and its count is zero.

The live effect count is never greater than one.

The authoritative Done count is never greater than one.

The concrete SUT cardinalities match the independent reference state.

Confirmed concrete state matches authoritative Done.

Any live effect matches CommandInFlight concrete state.

## Dependency versions

Use current compatible releases explicitly in core dev-dependencies:

- `proptest = "1.10"`.
- `proptest-state-machine = "0.8"`.

Both remain test-only.

The workspace lockfile will capture the exact resolved graph.

## Test execution budget

Use the state-machine macro with a moderate sequence range long enough to
exercise repetitions and reorderings.

Set a local proptest case count if ordinary defaults make workspace execution
too costly.

The test has no filesystem or process I/O, so hundreds of generated sequences
remain cheap.

The deterministic unit tests remain valuable for exact rejection variants;
the property test complements rather than replaces them.

## Rejected production changes

Do not add phase, poll, stop, reload, or timeout variants to
`CompletionEvent`.

Those values belong to adapters and would couple the pure aggregate to the
scheduler vocabulary.

Do not expose test-only counters from production.

Do not weaken correlation checks to accommodate manual recovery.

Do not make blocked dispositions enter Confirmed for trace convenience.

## Completion criteria

The focused integration test must compile and pass repeatedly.

The full workspace suite must pass.

Formatting and diff checks must be clean for owned files.

The manifest, lockfile, and integration test must be committed through the
isolated Lisa transaction with exact includes.

Unrelated worktree changes must remain untouched.
