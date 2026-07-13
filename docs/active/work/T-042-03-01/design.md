# Design: operator-requested authority emission

## Goal

Represent `[d]one` as an explicit operator request at the adapter boundary.
The type must make attempt authority impossible for this input.
The request must name its operator surface for auditability.
Active and orphaned Reviews must follow the same authority path.
The request must fail closed on E-040 Block/Invalid/missing disposition.
The existing dependency gate must remain mandatory.

## Constraints

The pure E-041 reducer contract is out of scope.
The adapter remains the only effect executor.
Attempt fencing must not be weakened.
An operator request must not admit an active attempt's private artifacts.
Modal persistence and richer confirmation are owned by the next ticket.
The implementation should remain a small plugin-local change.

## Option 1: always construct Operator in `mark_ticket_done`

The smallest behavioral patch would keep `CompletionInput::Manual` unchanged.
`mark_ticket_done` would always pass `Some(CompletionAuthority::Operator)`.
This would stop borrowing the thread lease.
It would work without a thread.

The input type would still permit future callers to pass Attempt or None.
The authority invariant would live only in one constructor's implementation.
The event would still carry only the generic unit source Manual.
It would not meet the explicit `OperatorRequested` event-shape requirement well.
It also would not address the disposition bypass without another conditional.
This option is rejected because it leaves the illegal state representable.

## Option 2: add operator authority to lisa-core

The pure reducer could gain an `OperatorRequested` event variant.
It could carry a source enum and distinguish attempt and operator requests.
This would make the domain contract directly aware of authority.

The reducer has no access to canonical disposition files or the scheduler DAG.
Encoding disposition and dependencies would expand its input contract.
Story S-042-03 explicitly excludes changes to the E-041 reducer contract.
That change would affect generated reducer tests and downstream tickets.
This option is rejected as a boundary and scope violation.

## Option 3: plugin-local `OperatorRequested` adapter event

Replace `CompletionInput::Manual` with `CompletionInput::OperatorRequested`.
The variant carries only `ticket_id` and an operator source enum.
It carries no authority field and no attempt lease.
Dispatch maps it unconditionally to `CompletionAuthority::Operator`.
Dispatch maps it to the stable reducer identity `operator`.

This makes attempt borrowing impossible at the adapter input boundary.
It preserves the pure reducer unchanged.
It makes the source explicit and extensible if another operator surface appears.
It is selected as the primary design.

## Auditable source shape

Introduce `OperatorRequestSource` in the plugin module.
Its first variant is `MarkDoneKey`.
The name identifies the dashboard `[d]one` interaction.
The enum derives Debug, Clone, Copy, PartialEq, and Eq.

`CompletionSource` changes from unit `Manual` to
`OperatorRequested(OperatorRequestSource)`.
Pending completion state therefore retains both operator authority and source.
Debug-formatted activity around command launch includes the source.
Tests can inspect the exact source without parsing text.

The source enum is intentionally plugin-private.
No serialization is required by this ticket.
Durable journal behavior is Story B and already uses completion identities.
The later UI ticket can consume the retained source if needed.

## Disposition enforcement

Operator authority is independent of attempt authority.
It therefore must not call `admit_artifact` with the active attempt lease.
It also must not use the unleased admission fallback when a lease exists.
Instead it consumes the canonical admitted `review-disposition.json`.

Extract canonical verdict evaluation into a helper.
The helper parses the existing canonical disposition path.
Pass returns success.
Block returns `DispositionBlocked` with the authored reason.
Invalid returns `DispositionBlocked` with invalid-detail context.

`admit_passing_review` retains its current artifact-admission step.
After admission, it delegates canonical verdict evaluation to the helper.
This preserves all attempt-based behavior.

The OperatorRequested dispatch branch calls only canonical verdict evaluation.
Missing canonical evidence parses as Invalid and fails closed.
No attempt artifact is copied or claimed by the operator.
The rejection uses the existing correlated activity mechanism.

## Ordering of checks

The adapter creates the stable operator completion correlation first.
It checks the canonical disposition before reducing the request.
A blocking disposition therefore produces no reducer effect.
This mirrors attempt requests, which validate review evidence before reduction.

After Pass, the reducer sees a normal typed Request.
The returned inert effect reaches `execute_completion_effect`.
The executor performs its existing duplicate and dependency checks.
Unmet dependencies return `DependencyBlocked` before pending state or launch.

Disposition-first ordering gives the explicit Review decision priority.
Separate tests will isolate disposition and dependency failures.
Both failures remain named and correlated.

## Executor authority rule

The executor currently admits Operator only for `CompletionSource::Manual`.
Update that match to accept Operator only for
`CompletionSource::OperatorRequested(_)`.
No scheduler source can use operator authority.
No operator source can use attempt authority because its input has no such field.

Result validation uses the same source condition.
Pending operator results remain valid only for OperatorRequested sources.
The change preserves stale-attempt fencing for every attempt-driven source.

## Test design

Update the active-thread mark-done regression.
Provide canonical Pass disposition evidence.
Install an active attempt and thread as before.
Assert pending authority is Operator.
Assert pending source is MarkDoneKey.
Assert the launched effect identity is `operator`, not attempt `1`.
Retain thread, slot, and ticket-state assertions.

Retain and strengthen the orphaned Review test.
Provide canonical Pass disposition evidence.
Assert the same authority and exact source with no thread.

Add a focused refusal test covering two tickets.
One Review has canonical Block disposition and an active attempt/thread.
Calling `mark_ticket_done` must yield no pending state or effect.
Activity must contain a correlated DispositionBlocked reason.

The other Review has canonical Pass disposition but depends on an unfinished ticket.
Calling `mark_ticket_done` must yield no pending state or effect.
Activity must contain a correlated DependencyBlocked reason.
Both cases prove operator authority does not bypass reducer/adapter eligibility.

## Compatibility

No public crate API changes.
No serialized schema changes.
No CLI changes.
No WASM dependency changes.
Existing automatic completion paths retain attempt authority.
Existing completion command arguments retain stable `operator` identity.
Existing modal eligibility behavior remains unchanged.

## Rejected extensions

Do not keep the modal open in this ticket.
Do not add success banners or new renderer variants.
Do not expand to stale, pending, and launch-failure matrix coverage.
Those are explicitly assigned to T-042-03-02 and T-042-03-03.
Do not persist operator source in a new journal schema.
Do not modify lisa-core's reducer or disposition types.

## Decision

Use a plugin-local `CompletionInput::OperatorRequested` variant.
Make its only authority mapping `CompletionAuthority::Operator`.
Carry `OperatorRequestSource::MarkDoneKey` through pending source state.
Evaluate the canonical E-040 disposition before reducer dispatch.
Preserve dependency enforcement in the sole effect executor.
Prove active, orphaned, blocked-disposition, and unmet-dependency behavior natively.
