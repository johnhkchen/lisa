# Design: total completion reducer

## Goals

The reducer must make every state/event pair explicit, preserve asynchronous
correlation, emit at most one effect, and remain a pure value transformation.
The design must not anticipate the plugin adapter or the next ticket's durable
eligibility model.

## Option 1: permissive event folding

One approach is to accept expected forward edges and return the existing state
unchanged for every other event. This is compact and makes replay superficially
idempotent.

It hides ordering errors, stale results, and duplicate launch acknowledgements.
It also contradicts the requirement that illegal edges yield named rejection
outcomes. Rejected.

## Option 2: catch-all rejection

The forward transitions could be listed first, followed by a single wildcard
that returns a generic failure. This is common in small reducers and would be
total at runtime.

The ticket explicitly rules out a catch-all that hides a state. A wildcard
also permits a newly added state or event to compile without a deliberate
decision, defeating exhaustiveness as change detection. Rejected.

## Option 3: exhaustive state-oriented reducer

Match the state first, then match every event within each state. This makes all
five states visible in the outer match and all five events visible in every
inner match. Adding either a state or event becomes a compiler error until its
behavior is chosen.

This is verbose—25 arms—but the matrix is the domain contract and remains easy
to inspect. Chosen.

## Illegal-transition representation

The inherited rejection enum names business and adapter failures, but it lacks
a truthful outcome for an event that cannot apply in the current lifecycle
state. Reusing `StaleLease`, `AlreadyPending`, or `LaunchFailed` for arbitrary
ordering errors would attach false meaning and, for most callback events, the
required attempt/completion payload is not present.

Add `UnexpectedEvent { state, event }`, with static names, for invalid matrix
edges. Add `CorrelationMismatch { expected, actual }` for results addressed to
a different in-flight command. Both are independently matchable and avoid
string parsing. They extend rather than replace the five required domain
outcomes.

Static state/event labels avoid recursively embedding `CompletionState` inside
`CompletionRejection` (a state can itself contain a rejection). Small private
label helpers use exhaustive matches with no wildcard.

## Accepted transitions

### Eligible

`Request` transitions to `Requested` and returns exactly one
`LaunchCompletion` effect containing the event's attempt and completion IDs.
Every command lifecycle event is unexpected because no request has been
accepted.

### Requested

`CommandLaunched` transitions to `CommandInFlight` with the supplied mandatory
correlation and no effect.

`CommandLaunchFailed` is an accepted fact about the emitted launch request. It
transitions to `Rejected`, wraps the source in
`CompletionRejection::LaunchFailed`, marks it retryable, and emits no effect.

A second `Request` returns `AlreadyPending` with the incoming completion ID.
Result events are unexpected because launch acknowledgement has not established
an in-flight correlation.

### CommandInFlight

Matching `CommandSucceeded` transitions to `Confirmed` with no effect.
Matching `CommandFailed` transitions to `Rejected`, preserving the failure as
`LaunchFailed` and preserving the event's retryability, with no effect.

Either result with a different correlation returns `CorrelationMismatch` and
does not construct a transition. Request returns `AlreadyPending`. A second
launch acknowledgement or pre-launch failure is unexpected.

### Rejected

When retryability is `Retryable`, `Request` starts a fresh request and emits one
launch effect. When retryability is `ActionRequired`, `Request` returns the
stored rejection unchanged. This makes the retry policy operational without
inventing eligibility inputs.

All command lifecycle events are unexpected because no command is represented
as live in this state.

### Confirmed

Request returns `AlreadyPending`: completion has already been authoritatively
resolved and must not emit another command. All command lifecycle events are
unexpected. This keeps confirmation terminal.

## Error versus rejected state

`Err` means the presented event was refused. `Ok(Transition { state:
Rejected, .. })` means a valid lifecycle fact moved the aggregate into a
rejected domain state. Thus a launch/command failure arriving at its expected
state is an accepted transition, while an action-required retry request returns
the retained reason as `Err`.

This distinction lets callers update aggregate state only for accepted facts
and report refused inputs without guessing from booleans.

## Purity and effect cardinality

`reduce` consumes state and event and allocates only owned return values. It has
no adapter parameter, callback, global, filesystem access, clock, randomness,
process execution, or scheduler reference.

Only two arms emit an effect: initial request and retryable request. Each
constructs `Some(EffectCommand::LaunchCompletion { .. })`; every other accepted
arm uses `None`. A vector is not introduced.

## Testing decision

Use module-local unit tests with exact whole-value equality. Tests cover every
legal edge, including both retryability branches and both matching command
results. A compact matrix test enumerates every illegal non-request edge and
asserts its exact `UnexpectedEvent`. Separate tests assert duplicate request,
correlation mismatch, and retained action-required rejection variants.

The tests call the reducer directly. No mock runtime is needed because there is
no runtime boundary in this ticket.

## Rejected extensions

- Do not add durable eligibility inputs; T-041-01-03 owns reconciliation.
- Do not store attempt/completion IDs in state; that changes the predecessor's
  settled vocabulary and belongs with durable idempotency design.
- Do not execute effect commands in lisa-core.
- Do not add serde or a wire format.
- Do not add proptest here; exhaustive matrix unit tests directly satisfy this
  ticket, while broader generated sequences belong to later epic work.
- Do not modify lisa-plugin completion callers.

## Compatibility

Adding rejection variants is source-compatible for construction but
intentionally asks exhaustive downstream matches to handle invalid lifecycle
outcomes. No downstream source currently imports this module, so there is no
repository migration. No dependency or manifest change is needed.
