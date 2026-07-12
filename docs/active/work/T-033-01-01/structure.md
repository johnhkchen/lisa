# Structure: recycled-seat assignment state model

## Change boundary

This ticket modifies one source file:

- `crates/lisa-plugin/src/lib.rs`

It creates the six RDSPI artifacts under:

- `docs/active/work/T-033-01-01/`

It does not modify:

- `docs/active/tickets/T-033-01-01.md`;
- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-plugin/src/ui.rs`;
- `crates/lisa-plugin/src/pane_name.rs`;
- `crates/lisa-core/src/types.rs`;
- hook templates or CLI code.

## Scheduler enum

Add a private enum near `TransitionState` and the other per-slot lifecycle types:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeatAssignmentState {
    AssignedPendingAck,
    Owned,
    Recovering,
}
```

Responsibilities:

- name the scheduler’s assignment-truth states;
- remain independent of reset/exit transport state;
- support direct equality assertions in native tests;
- give later tickets stable promotion and recovery targets.

The enum is not serialized and is not public API.

`Recovering` may be unused by production code in this ticket. If the compiler warns,
annotate that variant narrowly with `#[allow(dead_code)]` and explain that T-033-01-04
owns the transition into it. Do not suppress warnings for the whole enum or module.

## State storage

Add a field to `State` adjacent to `agent_slots`:

```rust
seat_assignments: HashMap<u32, SeatAssignmentState>,
```

The key is the physical terminal pane ID, matching `AgentSlot.pane_id` and future UI
projection. The map has an entry only while a ticket is assigned.

Invariants:

1. No entry means unassigned.
2. `Owned` is the only state that reports ownership.
3. `AssignedPendingAck` retains `ticket_id` but reports not owned.
4. `Recovering` retains the recovery ticket reservation but reports not owned.
5. Normal slot release removes the entry.
6. Transport transitions do not implicitly promote assignment state.

`State` continues to derive `Default`; `HashMap::default` supplies the empty map.

## Query helpers

Add private methods in the slot-management section before slot selection:

```rust
fn seat_assignment(&self, pane_id: u32) -> Option<SeatAssignmentState>
fn seat_is_owned(&self, pane_id: u32) -> bool
```

`seat_assignment` copies the enum from the map.

`seat_is_owned` is implemented as an exact match against:

```rust
Some(SeatAssignmentState::Owned)
```

The helpers establish the semantic boundary for current tests and later dashboard
projection. No caller should infer ownership from `ticket_id`, `has_session`,
`ThreadStatus::Running`, or `TransitionState::Idle`.

## Schedule-time classification

Within `schedule_ready_tickets`, capture the pre-mutation residency fact immediately
after slot selection:

```rust
let reused_seat = self.agent_slots[slot_idx].has_session;
```

This value must be captured before the recycle branch sets `has_session = false`.

After the scheduler writes `ticket_id` and `last_client`, calculate:

```rust
let assignment_state =
    if route.agent == AgentClient::Codex && reused_seat {
        SeatAssignmentState::AssignedPendingAck
    } else {
        SeatAssignmentState::Owned
    };
```

Insert the state keyed by `pane_id`.

Ordering requirements:

1. Slot selection establishes the candidate.
2. The old residency fact is captured.
3. Existing launch/reset/recycle behavior runs unchanged.
4. The slot is reserved with `ticket_id` and incoming provider.
5. Assignment state is inserted in the same scheduling operation.
6. Thread creation proceeds unchanged.

This preserves all existing command construction and activity logging.

## Fresh path

For `has_session == false`:

- the adapter launch command remains unchanged;
- `has_session` becomes true as before;
- Codex and Claude assignments enter `Owned`;
- the slot’s ticket, client, thread, and pane title remain unchanged.

This explicitly threads fresh-slot bookkeeping into the new model.

## Same-provider reuse path

For `has_session == true` and a compatible resident provider:

- reset strategy selection remains unchanged;
- `ClearHandshake` still sends `/clear` and enters `WaitingForClear`;
- Claude enters `Owned` immediately;
- Codex enters `AssignedPendingAck`;
- neither `.cleared` nor clear timeout promotes the assignment.

This is the primary acceptance path.

## Cross-provider recycle path

For `SlotSelection::Recycle`:

- the resident adapter still supplies `/exit`;
- `WaitingForExit` and exit grace remain unchanged;
- the incoming provider remains stamped in `last_client`;
- incoming Codex enters `AssignedPendingAck`;
- incoming Claude enters `Owned`;
- exit-grace launch preserves the map entry.

Capturing residency before `has_session = false` is required for correct classification.

## Transition signal handling

No structural edits are required in:

- `check_transition_signals`;
- `handle_stopped_signal`;
- `handle_cleared_signal`.

Their lack of assignment-state writes is intentional. The tests will establish that
transport completion does not equal assignment acknowledgment for recycled Codex.

## Transition timeout handling

The normal timeout branches preserve `seat_assignments`:

- `WaitingForExit` with a ticket launches and retains existing state;
- `WaitingForStop` changes only transport state;
- `WaitingForClear` sends the prompt and changes only transport state.

The exceptional missing-ticket `WaitingForExit` branch must remove the pane’s entry
because it explicitly abandons the pending assignment and restores an empty shell.

No acknowledgment timeout is added in this ticket.

## Release integration

In `release_slot_for_ticket`, when a matching slot is found:

1. save the pane ID for existing rename/log behavior;
2. clear `slot.ticket_id` as before;
3. remove `seat_assignments[pane_id]`;
4. retain resident session and provider state as before;
5. establish cooldown and rename the pane as before.

Because borrowing `agent_slots` and `seat_assignments` simultaneously may constrain
the loop, capture the released pane ID and remove from the map after the slot borrow
ends. This follows the method’s existing deferred rename pattern.

## Test placement

All tests remain in the `#[cfg(test)] mod tests` section of `lib.rs`.

### Primary acceptance test

Add a clearly named scheduler test using `pane_name_schedule_state` with:

- incoming ticket agent `codex`;
- loop default Claude or Codex as already supported;
- resident `last_client = Codex`;
- `has_session = true` from the helper.

Call `schedule_ready_tickets` and assert:

- slot remains assigned to the expected ticket;
- transport state is `WaitingForClear`;
- assignment is `AssignedPendingAck`;
- `seat_is_owned` is false.

### Fresh bookkeeping test

Use the same scheduling fixture with no resident client/session.
Assert fresh Codex assignment is `Owned` and `seat_is_owned` is true.

### Claude compatibility test

Use a reused Claude resident session and Claude ticket.
Assert transport state remains `WaitingForClear`, assignment is `Owned`, and the
ownership predicate is true.

### Timeout preservation tests

Extend the clear-timeout fixture or add a focused test that seeds:

- a Codex ticket;
- `WaitingForClear` past timeout;
- `AssignedPendingAck` in the map.

After `check_transition_timeouts`, assert transport state becomes `Idle` while the
assignment remains pending and not owned.

Extend the exit-grace launch test similarly. After launch, assert pending remains and
ownership remains false.

### Release cleanup test

Seed an assigned slot and assignment entry, call `release_slot_for_ticket`, and assert:

- `ticket_id` is absent;
- assignment query is absent;
- ownership predicate is false;
- resident-session behavior is unchanged.

## Verification boundary

Run formatting first, then focused plugin tests, then the full workspace tests.
Run `cargo clippy -p lisa-plugin --all-targets -- -D warnings` if the repository’s
existing lint baseline permits. Always run `git diff --check` on the ticket-owned
source and artifacts.

Before Review:

- commit `crates/lisa-plugin/src/lib.rs` through `lisa commit-ticket`;
- include only the exact source path in that implementation commit;
- confirm the source path is clean afterward;
- confirm it is not staged in the ordinary index;
- leave work artifacts for Lisa’s completion transaction.

## Expected final shape

The scheduler will have two independent per-pane lifecycle axes:

```text
AgentSlot.transition_state
    reset/exit transport progress

State.seat_assignments[pane_id]
    assignment acknowledgment truth
```

That structure enables T-033-01-02 to detect acknowledgment, T-033-01-03 to perform
the exact pending-to-owned transition, T-033-01-04 to enter recovery on its own
deadline, and S-033-02 to project the named state without terminal-text inference.
