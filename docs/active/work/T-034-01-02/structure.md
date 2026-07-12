# T-034-01-02 Structure — mint lease on dispatch

## Change inventory

Modify two source files:

- `crates/lisa-core/src/types.rs`
- `crates/lisa-plugin/src/lib.rs`

Create the remaining workflow artifacts under:

- `docs/active/work/T-034-01-02/`

No source files or modules are created or deleted. No ticket frontmatter is
modified by this work.

## `crates/lisa-core/src/types.rs`

### `Thread` lease stamp

Add an optional public field to `Thread`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub attempt_lease: Option<AttemptLease>,
```

Place it after the pane identity and before lifecycle phase/time fields. This
keeps execution identity close to ticket/pane identity and before mutable run
state.

The field means: the provider-neutral attempt lease stamped by the scheduler
when this thread was dispatched. `None` means the record was constructed
outside dispatch or predates lease support; it must not be interpreted as an
authorized attempt.

### `Thread::new`

Initialize `attempt_lease` to `None`.

`Thread::new` remains a general constructor used heavily by tests and helper
paths. It must not mint authority because it has no scheduler predecessor
registry and cannot prove dispatch admission.

### Core tests

Extend `test_thread_run_meta_defaults` to assert the new thread is unstamped.

Extend `test_thread_deserializes_without_run_meta` to assert an older serialized
thread receives `attempt_lease == None` through serde defaulting.

No change is required to the existing `AttemptLease` unit tests or public
exports; `types` is already public from `lisa-core`.

## `crates/lisa-plugin/src/lib.rs`

### Imports

Add `AttemptLease` to the existing `lisa_core::types` import list used by the
plugin implementation.

No new module dependency or crate dependency is needed.

### `AgentSlot`

Add:

```rust
attempt_lease: Option<AttemptLease>,
```

Place it adjacent to `ticket_id`. Together they describe which ticket attempt
is assigned to the physical pane. A populated ticket must receive the matching
lease during scheduler dispatch. An idle slot has neither.

Update every direct `AgentSlot` construction in production discovery and test
fixtures to initialize `attempt_lease: None` unless the fixture explicitly
models an assigned attempt.

Most constructors are repeated test literals. The compiler will identify every
required update because the private struct uses complete literals.

### `State`

Add:

```rust
current_leases: HashMap<TicketId, AttemptLease>,
```

Place it after `threads`, since both maps are keyed by ticket and jointly
describe active run identity. `State::default()` initializes it empty through
the collection default.

The map is the scheduler-owned latest minted lease and redispatch high-water
record. It is not removed by ordinary release in this ticket.

### `schedule_ready_tickets`

After slot selection and the awaiting-human guard, mint a lease:

```rust
let attempt_lease = match AttemptLease::mint(
    ticket_id.clone(),
    self.current_leases.get(&ticket_id),
) { ... };
```

On success, insert a clone into `current_leases` before pane lifecycle effects.

On error:

- log an `ActivityEvent::Error` containing the ticket and error;
- increment `unscheduled`;
- continue to the next ready ticket.

When reserving the selected slot, set:

```rust
self.agent_slots[slot_idx].attempt_lease = Some(attempt_lease.clone());
```

When building the thread, set:

```rust
thread.attempt_lease = Some(attempt_lease);
```

The registry, slot, and thread must all derive from this one local value. No
second call to `mint` is allowed within one dispatch.

The existing Codex `assignment_generation` allocation remains unchanged and
independent.

### `release_slot_for_ticket`

When clearing a matching slot's `ticket_id`, also clear:

```rust
slot.attempt_lease = None;
```

Do not remove `current_leases[ticket_id]`. That retained value is the
predecessor needed by redispatch and is explicitly asserted in the scheduler
test.

No change is made to seat acknowledgment removal, cooldown, pane renaming, or
thread removal ownership.

### Scheduler test helper

`fresh_slot` initializes `attempt_lease` to `None`; all tests using it inherit
the new valid idle-slot shape.

Other direct slot literals throughout the test module receive the same field.
Fixtures that start with `ticket_id: Some(...)` but are unrelated to dispatch
may remain unstamped because they model legacy/manually assembled state and do
not pass through the new authority seam.

### New scheduler acceptance test

Add a test near existing scheduler/pane lifecycle tests:

`dispatch_mints_and_stamps_strictly_new_attempt_lease`

Use `pane_name_schedule_state` so the test exercises:

- a real scanned ticket and DAG;
- real slot admission;
- the real scheduling method;
- the existing fresh/reuse lifecycle machinery.

First-dispatch assertions:

- `current_leases["T-NAME"].attempt_id == 1`;
- `current_leases["T-NAME"].ticket_id == "T-NAME"`;
- `threads["T-NAME"].attempt_lease` equals current;
- `agent_slots[0].attempt_lease` equals current.

Release assertions:

- call `release_slot_for_ticket`;
- remove the thread to model lifecycle caller behavior;
- slot ticket and lease are both absent;
- current/high-water entry still equals the first lease.

Redispatch assertions:

- call `schedule_ready_tickets` again;
- new current attempt is greater than the first;
- new current is exactly attempt 2;
- new thread and slot carry the new current;
- the old lease no longer validates as current;
- the new lease does validate as current.

If immediate cooldown proves timing-sensitive, explicitly set the slot's
`cooldown_until` to the past before redispatch. This is test setup, not a
production behavior change.

## Interface effects

The only public interface change is the optional `Thread::attempt_lease` field.
This is additive and serde-compatible.

`AgentSlot`, `State` fields, and scheduler methods remain private. No CLI,
configuration, adapter, prompt, signal, UI, or provenance interface changes.

## State relationship

During an active dispatch:

```text
State.current_leases[ticket]
             │ exact clone
             ├──────────────> Thread.attempt_lease
             └──────────────> AgentSlot.attempt_lease
```

After release:

```text
State.current_leases[ticket] = retained predecessor
Thread                         = removed by lifecycle caller
AgentSlot.attempt_lease        = None
AgentSlot.ticket_id            = None
```

On redispatch, the retained predecessor is passed to `AttemptLease::mint` and
all active records receive the successor.

## Ordering constraints

1. Add the core `Thread` field and defaults.
2. Add plugin imports and storage fields.
3. Update all `AgentSlot` literals until compilation succeeds.
4. Add dispatch mint/store/stamp behavior.
5. Clear the seat stamp on release.
6. Add the scheduler acceptance test.
7. Format and run focused tests.
8. Run workspace verification.
9. Commit only the two ticket-owned source files via Lisa.
10. Write Review after confirming no ticket-owned source remains dirty.

## Boundaries preserved

- `AttemptLease::mint` remains the sole increment contract.
- Scheduler state, not core types, owns mutable lease authority.
- Provider acknowledgment state continues to own delivery generations.
- Release continues to own physical seat cleanup.
- Later tickets own revocation, fencing, and stale-surface rejection.
- Lisa owns ticket phase/status transitions and final artifact publication.
