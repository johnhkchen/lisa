# T-034-01-02 Research — mint lease on dispatch

## Ticket boundary

This ticket connects the attempt-lease value introduced by `T-034-01-01` to
the plugin scheduler's dispatch path. The required observable behavior is that
every accepted dispatch of a ticket records a fresh current lease and places
that same lease on the assigned pane/thread. Releasing and redispatching the
ticket must produce a strictly higher attempt ID.

The ticket does not own timeout fencing, stale-event rejection, completion
gating, or durable restart semantics. Those are assigned to later tickets in
S-034-01, S-034-02, and S-034-03.

## Core lease contract

`crates/lisa-core/src/types.rs` defines `AttemptLease` near `TicketId`.

An `AttemptLease` contains:

- an owned `ticket_id: TicketId`;
- an `attempt_id: u64`;
- derived clone, equality, hash, and serde behavior.

`AttemptLease::mint(ticket_id, previous)` returns attempt 1 when `previous` is
absent. With a predecessor for the same ticket, it uses checked addition and
returns the next attempt. It rejects a predecessor for another ticket and
rejects exhaustion at `u64::MAX`.

`AttemptLease::is_current` compares the complete lease to an optional
authoritative lease. Validation therefore depends on both ticket and attempt,
not only the numeric generation.

The core type deliberately does not own a registry. The caller must retain the
previous lease to preserve monotonicity across dispatches.

## Thread representation

`Thread` also lives in `crates/lisa-core/src/types.rs`. It is the shared active
run record consumed by the plugin and UI. It currently stores ticket, pane,
phase, activity clocks, status, provider, concurrency-at-spawn, and route.

`Thread::new` initializes a running thread with provider defaults and no route.
The scheduler fills provider, route, concurrency, and phase after construction.

`Thread` is serialized and has explicit serde defaults for fields added after
the original state shape. Tests verify that older JSON without run metadata
still deserializes. Any lease field added to `Thread` must preserve that
compatibility because persisted or fixture-created threads may predate leases.

There is no current attempt identity on `Thread`. Ticket ID alone cannot
distinguish two executions of the same ticket.

## Plugin state boundary

`State` in `crates/lisa-plugin/src/lib.rs` owns the DAG, threads, physical agent
slots, seat-assignment state, transition state, and scheduler configuration.
It derives `Default`, so newly added maps naturally begin empty.

`threads: HashMap<TicketId, Thread>` is keyed only by ticket. A dispatch inserts
one active thread and the scheduler refuses another while a non-completed
thread exists.

`agent_slots: Vec<AgentSlot>` represents physical terminal panes. Each slot has
a `pane_id`, optional assigned `ticket_id`, resident-session facts, transition
state, cooldown, activity time, and provider affinity. `AgentSlot` is private
to the plugin and is constructed in slot discovery and test helpers.

`seat_assignments: HashMap<u32, SeatAssignmentState>` represents provider
acceptance of an assignment. It distinguishes pending acknowledgment, owned,
recovering, and recovery-failed. It is keyed by pane ID and intentionally
separate from the lifecycle transition state.

`next_assignment_generation` and generations inside `SeatAssignmentState` are
Codex delivery identities. They exist only for reused Codex seats and may mint a
second generation for the same scheduler dispatch during bounded recovery.
Fresh Codex launches and all Claude paths have immediate ownership and no such
generation. This generation is therefore not provider-neutral attempt identity.

## Dispatch path

`State::schedule_ready_tickets` is the single scheduling admission path.

For each DAG-ready ticket it:

1. rejects a ticket with an active thread;
2. resolves its provider and route;
3. enforces global and provider concurrency caps;
4. selects a compatible or recyclable idle slot;
5. rejects a pane currently awaiting human input;
6. builds the provider spawn context;
7. renames the pane and starts fresh, reuse, or recycle lifecycle input;
8. assigns the ticket and provider to the `AgentSlot`;
9. creates the provider acknowledgment state;
10. constructs and inserts the `Thread`;
11. clears stale alerts and logs the launch.

The last pre-dispatch gates are slot selection and the awaiting-human check.
After those gates, pane renaming and input are externally observable side
effects. Lease minting must occur after admission is known but before these
effects so a failed mint cannot launch an unleased attempt.

The scheduler currently has no fallible state allocation at this seam.
`AttemptLease::mint` introduces only two exceptional cases: cross-ticket state
corruption and numeric exhaustion. Both should fail closed for that ticket.

## Release and redispatch

`State::release_slot_for_ticket` finds the assigned `AgentSlot`, clears its
`ticket_id`, retains the resident session, starts cooldown, removes the
pane-keyed seat assignment, renames the pane idle, and logs the release.

Thread removal is performed by callers after completion or reclamation rather
than by `release_slot_for_ticket` itself. Once the thread is absent and the DAG
still considers the ticket ready, a later scheduler poll can dispatch it again.

Release currently discards all pane assignment identity. Strictly increasing
redispatch requires the scheduler to retain a per-ticket high-water lease even
after the thread and seat stamp are cleared. Removing the only lease on release
would cause redispatch to mint attempt 1 again.

The next story ticket will revoke and fence before release. A retained
high-water record is compatible with revocation if current authority and mint
history are later separated or represented explicitly. This ticket only needs
the latest minted predecessor and current dispatch authority.

## Existing test seams

Plugin tests are colocated in `crates/lisa-plugin/src/lib.rs`. Helpers create
temporary ticket directories, scan them into a real `Dag`, construct `State`
with permissions and slot discovery enabled, and install `AgentSlot` fixtures.

`pane_name_schedule_state` creates one ready ticket and one physical pane, then
calls the real `schedule_ready_tickets`. Existing tests inspect slot ticket ID,
thread provider, pane name, and `SeatAssignmentState` after dispatch.

Scheduler tests can call host-facing methods because test builds use local
stubs around Zellij operations. This makes the real dispatch path testable
without extracting a second scheduling implementation.

A redispatch test must release the slot, remove the old thread, ensure the slot
is immediately eligible, and call the same scheduler method again. With
`wind_down_secs = 0`, cooldown and quiet-period checks permit reuse.

## Relevant ownership and repository state

The implementation surface is concentrated in:

- `crates/lisa-core/src/types.rs` for the thread stamp;
- `crates/lisa-plugin/src/lib.rs` for scheduler registry, seat stamp, minting,
  release behavior, and scheduler coverage.

The working tree contains unrelated user changes, including CLI and hook work.
Neither target source file is currently modified. Ticket commits must include
only the exact source paths owned by this ticket and must use
`lisa commit-ticket` rather than the ordinary Git index.

## Constraints and assumptions

- Attempt IDs are monotonic per ticket, not globally.
- The first dispatch in a plugin process uses attempt 1.
- Scheduler-lifetime retention is sufficient for this slice; restart durability
  is not specified by the ticket or core helper.
- A dispatch lease must be provider-neutral and exist for fresh and reused
  Claude and Codex assignments alike.
- Provider acknowledgment generations remain a separate transport contract.
- An idle seat should not retain an active assignment stamp.
- The scheduler must retain the previous per-ticket lease after release so the
  next dispatch can mint a successor.
- A lease mint error must prevent pane input, seat assignment, and thread
  insertion for that ticket.
- Existing serialized `Thread` values must continue to deserialize.
- Ticket phase/status frontmatter is managed by Lisa and is not edited here.

## Research conclusion

The codebase already has one authoritative dispatch seam and all three required
state layers. What is missing is a per-ticket scheduler lease registry, a
provider-neutral stamp on the physical seat, and a compatible stamp on the
shared thread record. The registry must survive release; the seat stamp must be
cleared on release; and all three values must originate from one successful
mint immediately before dispatch side effects.
