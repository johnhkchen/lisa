# Design: clock-injected deadline evaluator

## Goal

Introduce one evaluator that owns deadline clock sampling and policy eligibility,
while preserving the six existing state-machine actions and exemptions.

## Option 1: inject `now` into each existing method

Each method could gain an `_at(SystemTime)` variant and retain its current loop.

Advantages:

- Minimal code movement.
- Straightforward deterministic tests.
- Low risk to stateful actions.

Disadvantages:

- Deadline traversal remains scattered across six methods.
- No common evaluator exists.
- Per-policy results remain implicit local vectors.
- This satisfies clock injection but not the centralization requested by the ticket.

Decision: reject as incomplete.

## Option 2: evaluator owns `State` and executes all effects

A generic evaluator could borrow `&mut State`, traverse every collection, and
perform recovery, pane I/O, fencing, provenance, warnings, and logging.

Advantages:

- One object would visibly run the complete deadline subsystem.
- Policies and effects would reside together.

Disadvantages:

- The module would need access to most private state-machine internals.
- It would tightly couple time policy to adapters, pane I/O, leases, and provenance.
- Mutable borrowing across traversal and effects would remain complex.
- Moving large action bodies risks changing characterized behavior.
- Testing pure timing would still require constructing the full plugin state.

Decision: reject because it centralizes unrelated effects and increases risk.

## Option 3: clock-injected eligibility evaluator with typed actions

Create a `deadline` module containing:

- a `Clock` trait returning `SystemTime`;
- a production `SystemClock`;
- a `DeadlineEvaluator<C>` that samples its clock;
- typed input views for each policy;
- typed action/candidate values for each policy;
- one evaluation method per policy.

Existing `State` wrappers construct input views, ask the evaluator for actions,
then apply the existing stateful effects in their existing order.

Advantages:

- All six traversals and comparisons live behind one abstraction.
- Clock injection is uniform and deterministic.
- Per-policy actions remain explicit rather than flattened into a generic timeout.
- Stateful behavior stays near the state machine that owns it.
- The characterization tests can remain unchanged.
- Pure evaluator tests can pin exact boundaries and exemptions cheaply.

Disadvantages:

- Input views duplicate selected fields from state objects.
- The evaluator API has several policy-specific types.
- `State` retains action loops, so centralization is deliberately limited to evaluation.

Decision: choose Option 3.

## Clock contract

`Clock` exposes `fn now(&self) -> SystemTime`.

`SystemClock` is a zero-sized production implementation.

Tests use a fixed clock implementation local to the test module. The evaluator
is generic rather than trait-object based, avoiding allocation and dynamic dispatch.

The evaluator samples the clock once per method call. Every comparison within a
policy evaluation therefore observes the same instant.

## Policy inputs and actions

Acknowledgement input carries pane ID, copied seat state, and absolute deadline.
Its action is an expired seat containing pane ID and original state.

Transition input carries pane ID, optional ticket ID, transition state/start,
last activity, and awaiting-human status. Its action is one of exit-ready,
stop-timeout, or clear-timeout. Awaiting-human suppression belongs in evaluation
for stop and clear, matching the requested preservation of exemptions.

Review input carries ticket ID, pane ID, running/Review eligibility, prior-prompt
status, phase clock, activity clock, and awaiting-human status. Its action is a
finish-up candidate.

Health input carries ticket ID, current computed state fields, and prior health.
The evaluator returns health transitions and initial observations separately so
the state layer can preserve logging behavior.

Session input carries ticket identity, pane, status eligibility, completion
exclusion, global/phase clocks and limits, activity, and awaiting-human status.
Its action distinguishes reclaim from advisory warning and includes elapsed time
and phase for the existing event payloads.

Stale input carries ticket identity, pane, status/completion eligibility,
activity, and awaiting-human status. Its action identifies reclaimable tickets.

## Boundary semantics

- Acknowledgement fires when `now >= absolute deadline`.
- Transition preserves strict whole-second `>` thresholds.
- Transition exit ignores activity and awaiting-human.
- Transition stop/clear require quietness and are awaiting-human exempt.
- Review, health, session, and stale use inclusive duration thresholds.
- Future timestamps yield zero elapsed duration.
- Global session budget retains precedence over phase budget.
- Awaiting-human session overrun remains advisory, not destructive.
- Awaiting-human stale sessions remain excluded.
- Awaiting-human health remains observable as stuck.

## State integration

Each current wrapper remains available with its existing name and signature.
Production wrappers instantiate `DeadlineEvaluator::new(SystemClock)`.

Where deterministic integration tests need direct control, private `_with` or
`_at` helpers accept an evaluator or timestamp without changing existing callers.
Acknowledgement retains its existing `_at(now)` signature by wrapping a fixed
one-shot clock or calling an evaluator method that accepts a sampled instant.

Action application retains existing ordering. This is significant because
earlier actions can mutate seat state, remove threads, revoke leases, or alter
slots before later effects run.

## Testing strategy

- Keep all T-039-04-01 characterization tests textually unchanged.
- Add pure evaluator tests using a fixed clock.
- Cover every policy at the exact boundary appropriate to that policy.
- Cover active and awaiting-human exemptions in evaluator tests.
- Confirm health deliberately ignores awaiting-human for observation.
- Confirm session returns a warning action for exempt overruns.
- Confirm stale returns no action for exempt threads.
- Run focused plugin tests, full plugin tests, and workspace tests.

## Compatibility

No configuration fields change.
No serialized state changes.
No public crate API is required.
No dependency is added.
No timeout duration changes.
No effect/event payload changes.
