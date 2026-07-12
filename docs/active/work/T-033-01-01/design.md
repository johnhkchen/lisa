# Design: recycled-seat assignment state model

## Decision summary

Add an explicit `SeatAssignmentState` enum to the scheduler with these variants:

- `AssignedPendingAck`
- `Owned`
- `Recovering`

Store the state in `State` as a pane-keyed map. Absence from the map means the
physical slot has no current assignment. Keep `AgentSlot.ticket_id` as the
reservation/routing key and keep `TransitionState` as the reset transport machine.

At schedule time:

- a reassigned Codex pane with an existing session enters `AssignedPendingAck`;
- fresh Codex assignments enter `Owned` under the current contract;
- every Claude assignment enters `Owned`, preserving current behavior;
- cross-provider recycling into Codex also begins pending because the physical seat
  was reassigned and acceptance is not yet acknowledged.

Current `.cleared` and transition-timeout paths preserve the assignment state.
Release removes the pane’s assignment state. Later tickets will add acknowledgment
promotion and recovery transitions without changing this initial classification.

## Design goals

- Represent the ticket’s three named states explicitly.
- Make “owned” a precise scheduler query rather than an inference from `ticket_id`.
- Preserve ticket reservation while acknowledgment is outstanding.
- Preserve existing capacity, DAG, thread, prompt, and pane-title behavior.
- Avoid coupling acknowledgment state to reset transport state.
- Keep Claude’s externally observable behavior unchanged.
- Provide a narrow seam for the detector and timeout tickets.
- Avoid broad changes to shared core types or dashboard rendering.
- Keep all logic unit-testable inside the existing scheduler test module.

## Non-goals

- Detecting a Codex acknowledgment.
- Defining lifecycle fixture formats.
- Promoting a pending assignment to owned.
- Starting the acknowledgment deadline.
- Entering recovery because an acknowledgment deadline elapsed.
- Launching the fresh-session recovery fallback.
- Rendering assignment state in the dashboard.
- Changing pane titles or provider routing.
- Changing Claude reset commands, hooks, or timing.

## Option 1: treat `ticket_id` as ownership

Continue using `AgentSlot.ticket_id.is_some()` as the ownership predicate and add
comments explaining that some assignments are not acknowledged yet.

Advantages:

- No code changes.
- No additional state synchronization.

Disadvantages:

- Directly violates the ticket’s semantic requirement.
- Cannot report pending assignment as not owned.
- Leaves later acknowledgment gating nowhere to store its result.
- Cannot distinguish recovery from ordinary reservation.
- Maintains the exact open-loop ambiguity that motivated the epic.

Decision: rejected.

## Option 2: delay `ticket_id` and `Thread` creation until acknowledgment

Reserve the pane through transport state alone, then bind the ticket and create the
thread only after Codex acknowledgment.

Advantages:

- Existing `ticket_id` could retain a strict owned meaning.
- Fewer concepts might appear in a fully redesigned scheduler.

Disadvantages:

- Transition handlers currently use `ticket_id` to construct the correct prompt.
- Timeouts need the ticket ID to relaunch or recover the same assignment.
- The DAG could reschedule a ticket that has no thread record yet.
- Provider and global capacity accounting would ignore in-flight assignments.
- Pane naming and signal attribution need the pending ticket immediately.
- It broadens the ticket into a scheduler transaction redesign.
- It would change Claude behavior even though Claude is out of scope.

Decision: rejected.

## Option 3: add assignment variants to `TransitionState`

Extend the transport enum with `AssignedPendingAck`, `Owned`, and `Recovering`.

Advantages:

- State stays on the physical slot.
- No second state container is needed.

Disadvantages:

- A slot can be `WaitingForClear` and `AssignedPendingAck` simultaneously.
- A cross-provider slot can be `WaitingForExit` and pending simultaneously.
- Later recovery can have its own launch transport step.
- Combining the axes creates a product enum and many artificial variants.
- Existing timeout matches would become ambiguous.
- Claude transport behavior would be touched throughout the scheduler.

Decision: rejected. Transport and assignment are orthogonal state machines.

## Option 4: add a required assignment field to `AgentSlot`

Add `assignment_state: Option<SeatAssignmentState>` directly to each slot.

Advantages:

- Slot data is colocated in one struct.
- It is difficult to have an assignment entry for an unknown pane.
- Later UI projection can read the field directly.

Disadvantages:

- `AgentSlot` is private but instantiated in many scheduler tests.
- Every literal would require mechanical modification unrelated to behavior.
- The resulting diff would obscure the contract change.
- A defaulted constructor migration would itself be a larger refactor.
- Existing code already stores pane-keyed scheduler metadata in `State` maps.

Decision: viable, but rejected for this narrow ticket because of disproportionate
fixture churn. It can be reconsidered in a future cohesive slot-model refactor.

## Option 5: store assignment state in a pane-keyed scheduler map

Add `seat_assignments: HashMap<u32, SeatAssignmentState>` to `State`.

Advantages:

- Makes assignment truth explicitly scheduler-owned.
- Absence cleanly represents an unassigned slot.
- `State::default()` initializes it safely.
- Existing `AgentSlot` literals remain unchanged.
- The scheduler can query by pane, matching future dashboard projection.
- Later tickets can promote or recover a single assignment atomically.
- Transport handlers can preserve it without additional writes.
- Release can clear it at the same point it clears `ticket_id`.
- The diff stays local to the actual lifecycle boundary.

Disadvantages:

- Slot reservation and assignment state live in separate containers.
- Incorrect cleanup could leave a stale entry.
- Callers need a helper rather than directly reading a slot field.

Mitigations:

- Centralize normal removal in `release_slot_for_ticket`.
- Clear missing-ticket recycle entries in its explicit abandonment path.
- Add tests for schedule, timeout preservation, and release.
- Provide one ownership predicate to prevent callers from inferring ownership.

Decision: selected.

## State semantics

### No map entry

- The slot is not assigned to a ticket.
- It may be an empty shell or may retain an idle resident agent session.
- It must report not owned.

### `AssignedPendingAck`

- The scheduler reserved the slot and ticket.
- Prompt delivery/reset may still be in progress or may have completed.
- No positive Codex ticket-acceptance evidence has been recorded.
- The slot must report not owned.
- Current transport timeouts do not promote this state.

### `Owned`

- Under the current contract, the assignment is considered accepted.
- Fresh assignments remain immediately owned.
- Claude assignments remain immediately owned.
- Later work will restrict recycled Codex promotion to matching acknowledgment.
- The ownership predicate returns true only for this variant.

### `Recovering`

- The pending assignment failed to acknowledge within its future deadline.
- The scheduler is performing a bounded recovery for the same ticket.
- The slot must report not owned.
- This ticket only makes the state representable.
- T-033-01-04 owns entry, fallback behavior, and terminal failure handling.

## Classification rule

Capture whether the selected slot already has a resident session before schedule-time
mutations. The initial state is:

```text
incoming provider = Codex AND selected slot has_session
    -> AssignedPendingAck
otherwise
    -> Owned
```

This covers the defect’s same-provider Codex reuse path. It also treats a physical
cross-provider recycle into Codex conservatively as pending. A completely fresh pane
continues to be owned immediately, preserving existing initial-launch behavior and
keeping the story scoped to reassignment.

Claude always falls into `Owned`, regardless of fresh, same-provider, or cross-provider
selection. No Claude adapter, hook, transition command, or timer changes.

## Ownership query

Add private scheduler helpers:

- `seat_assignment(pane_id) -> Option<SeatAssignmentState>`
- `seat_is_owned(pane_id) -> bool`

The first is the precise state query for tests and future projection. The second
defines ownership once: only `Some(Owned)` is true. Neither `ticket_id`, `ThreadStatus`,
nor transport `Idle` should be used as an ownership proxy after this change.

## Lifecycle integration

### Discovery

No entry is inserted. Newly discovered slots are unassigned.

### Scheduling

Determine the initial assignment state from incoming provider and preexisting session.
Insert it after the slot has been reserved for the ticket, in the same scheduling unit.
Thread creation, capacity accounting, and ticket phase advancement remain unchanged.

### Clear signal and clear timeout

Both paths send the pending ticket prompt and return transport state to `Idle`.
Neither path changes assignment state. This is the essential “no implicit promotion”
property needed before acknowledgment detection exists.

### Exit timeout launch

The path launches the incoming provider after the old provider’s exit grace period.
It updates session residency and transport state only. The existing assignment state
survives, so recycled Codex still reports not owned after launch.

### Release

When `release_slot_for_ticket` clears the matching slot’s `ticket_id`, also remove that
pane from the assignment map. Cooldown and resident-session behavior remain unchanged.

### Abandoned exit transition

When `WaitingForExit` has no ticket, restore the empty shell and remove any assignment
entry for the pane. This protects against stale ownership metadata.

## Test design

Add or extend scheduler tests to prove:

- same-provider recycled Codex becomes `AssignedPendingAck`;
- it reports not owned;
- fresh Codex remains `Owned`;
- reused Claude remains `Owned`;
- Claude still follows `WaitingForClear` exactly as before;
- clear timeout preserves a pending Codex assignment;
- exit-grace launch preserves a pending Codex assignment;
- release removes assignment state and reports not owned.

Tests should use existing one-ticket scheduling fixtures where host calls are already
supported. Timeout tests may seed the map directly because this ticket tests state
threading, not live hook production.

## Compatibility and risk

- The enum is private to the plugin, so no serialized format changes.
- `State` is not persisted, so no migration is needed.
- Thread and DAG semantics stay unchanged.
- Provider capacity continues counting the pending thread as running.
- Pane titles continue showing the assigned ticket; S-033-02 will add state display.
- The primary risk is map/slot drift on an overlooked teardown path.
- Central release coverage and the missing-ticket exit cleanup reduce that risk.
- Full plugin tests provide broad lifecycle regression coverage.

## Final decision

Use a private pane-keyed `SeatAssignmentState` map in scheduler `State`, classify
reassigned Codex seats as pending at scheduling time, define ownership as exactly the
`Owned` variant, preserve assignment state through current transport completion and
timeouts, and clear it whenever the slot assignment is released or abandoned. This is
the smallest durable contract on which the acknowledgment and recovery tickets can build.
