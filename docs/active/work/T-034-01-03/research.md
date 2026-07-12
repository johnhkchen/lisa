# T-034-01-03 Research — revoke and fence before reschedule

## Scope

This ticket covers the scheduler boundary used when an attempt has exceeded a
configured budget and has also been silent for the hard-silence interval.

The required safety sequence is:

1. invalidate the timed-out attempt's authority;
2. fence the physical pane that hosted it;
3. release the ticket so the DAG can dispatch a successor.

The ticket consumes the attempt-lease type from `T-034-01-01` and the dispatch
lease registry/stamps from `T-034-01-02`.

Per-surface rejection of stale acknowledgements, liveness, artifacts,
completion, and provenance belongs to S-034-02 and is outside this ticket.

## Core attempt-lease contract

`crates/lisa-core/src/types.rs` defines `AttemptLease` with:

- `ticket_id: TicketId`;
- `attempt_id: u64`.

`AttemptLease::mint(ticket_id, previous)` starts at attempt 1 and checked-
increments a same-ticket predecessor.

Minting rejects a predecessor for another ticket and rejects `u64::MAX`
exhaustion.

`AttemptLease::is_current(current)` compares the complete value against an
optional authoritative lease.

An absent authoritative lease rejects every candidate. That behavior is the
existing representation available for revocation.

`Thread` has an optional `attempt_lease` field. `None` represents legacy or
manually constructed state and is not a grant of authority.

## Scheduler lease state

`crates/lisa-plugin/src/lib.rs` owns all mutable scheduler state relevant to
this ticket.

`State::current_leases` is currently documented as the latest lease minted for
each ticket.

Dispatch uses the map twice:

- as the predecessor passed to `AttemptLease::mint`;
- as the authoritative value stamped onto the new slot and thread.

The entry deliberately survives `release_slot_for_ticket` today so a later
dispatch can mint a strictly higher attempt ID.

This means one map currently serves two lifetimes:

- high-water history, which must survive release;
- current authority, which must end before release.

Those lifetimes diverge at revocation. Removing the entry correctly invalidates
the old lease but loses the predecessor needed for monotonic redispatch.

Keeping the entry preserves monotonicity but leaves the old attempt valid in
the only current-authority registry.

The state therefore lacks a representation for “attempt N was the latest
minted value, but no attempt is currently authorized.”

## Physical and logical lease stamps

Every admitted dispatch copies one lease into three places:

- `State::current_leases[ticket_id]`;
- `AgentSlot::attempt_lease`;
- `Thread::attempt_lease`.

The registry is the scheduler-owned authority source.

The slot stamp identifies the provider-neutral attempt occupying a physical
pane.

The thread stamp identifies the logical run represented in scheduler state.

`release_slot_for_ticket` clears the slot stamp and ticket reservation.

The caller removes the thread separately, so the thread stamp disappears when
the logical thread record is removed.

Release currently does not invalidate the registry entry.

## Dispatch ordering

`State::schedule_ready_tickets` computes ready tickets and excludes any ticket
already present in `threads`.

It applies global capacity, provider capacity, slot availability, and awaiting-
human gates before minting.

It mints before pane rename, process input, seat reservation, acknowledgement
state, or thread insertion.

After minting, it stores the lease in `current_leases`, then copies it to the
chosen `AgentSlot` and new `Thread`.

The current predecessor map survives release, so the prior dispatch test can
release attempt 1 and dispatch attempt 2.

Any revocation change must retain that monotonic behavior.

## Release behavior

`State::release_slot_for_ticket` finds the matching physical slot and clears:

- `slot.ticket_id`;
- `slot.attempt_lease`;
- the pane-keyed `seat_assignments` entry.

For an ordinary completed resident session, release keeps `has_session = true`
and records a cooldown. The resident TUI can later be reused through its
adapter reset path.

Release renames the pane to an idle name and logs an informational event.

Release itself does not remove the logical thread; all callers own that step.

Release itself can make the slot eligible for later scheduling after cooldown.

The scheduler does not call `schedule_ready_tickets` inside release, but several
callers schedule later in the same control flow or on the next poll.

Central release is therefore the last shared boundary at which current
authority can be invalidated for every rescheduling path.

## Hard-silence timeout paths

`State::check_session_timeouts` enforces configured global and per-phase
budgets.

Exceeding a budget alone only emits a warning. Reclamation additionally
requires silence for `2 * stuck_threshold_secs`.

Panes marked as awaiting a human are exempt from reclamation.

For each reclaimable attempt, the current order is:

1. mark the thread failed;
2. emit timed-out provenance;
3. release the slot;
4. remove the thread;
5. publish timeout alert/activity state.

The method's documentation explicitly says the provider process is not killed.

`State::detect_stale_threads` independently uses the same doubled stuck
threshold as a hard inactivity timeout.

Its current reclaim order is also fail, provenance, release, remove, and log.

It likewise leaves the provider process alive.

Both paths can therefore make the ticket and pane available while the old
process remains capable of filesystem activity.

## Other release callers

Release is also called from:

- successful completion publication;
- stale slot cleanup;
- adapter-reported error handling;
- orphan-thread audit;
- manual ticket reset;
- tests and harnesses.

Some paths intentionally preserve a resident session for provider reuse.

Those paths do not all require physical termination, but none should leave a
released attempt authoritative if the same ticket can later dispatch again.

A timeout-specific fence cannot by itself establish the broader invariant that
release precedes rescheduling only after revocation.

## Existing pane transition states

`AgentSlot::transition_state` uses the private `TransitionState` enum.

The existing named states are:

- `Idle`;
- `WaitingForStop`;
- `WaitingForClear`;
- `WaitingForExit`.

`find_slot_for_client` only selects slots whose transition state is `Idle`.

The waiting states are bounded by timer-driven fallback logic.

There is no terminal state meaning a pane has been permanently disqualified
from future scheduler use.

A closed pane left represented as `Idle` would be selectable even though the
Zellij pane no longer exists.

## Available pane-control boundary

The project depends on `zellij-tile` 0.43.1 through its plugin prelude.

That API exposes `close_terminal_pane(terminal_pane_id)`.

Closing a terminal pane terminates the hosted process tree at the Zellij
boundary and is stronger than typing a provider-specific graceful-exit command.

The call is fire-and-forget. The plugin does not receive a completion result
for the close request.

The scheduler's slot discovery currently runs once. It does not reconcile
closed pane IDs or automatically create replacement terminal panes.

Consequently, a closed pane must remain disqualified in scheduler state; it
cannot safely become an idle reusable slot.

Capacity may shrink until an operator or later recovery feature replaces the
pane. That is bounded and explicit rather than an automatic retry loop.

## Test infrastructure

Plugin unit tests construct `State` and `AgentSlot` directly in `lib.rs`.

`test_check_session_timeouts_expired` already drives the hard-silence budget
path and asserts thread removal, slot release, activity, and alert creation.

Its fixture currently has no attempt lease and therefore cannot prove the new
authority invariant.

`dispatch_mints_and_stamps_strictly_new_attempt_lease` drives the real scheduler
twice and proves monotonic redispatch.

Native unit tests should not depend on executing a Zellij host command.

Post-state assertions alone can prove that a lease is absent and a pane is in a
fenced state, but they cannot prove the relative order of revocation, fencing,
and release.

An internal test observation seam is needed if the acceptance test must assert
strict ordering rather than infer it from final state.

## Constraints and invariants

- The ticket frontmatter phase and status are Lisa-owned and must not be edited.
- Ticket source changes must be committed through `lisa commit-ticket` with
  exact paths.
- The ordinary Git index and unrelated dirty files must remain untouched.
- Attempt IDs must remain strictly increasing after revocation and redispatch.
- An absent current authority must reject the old attempt lease.
- The pane must be fenced before its slot is released.
- A fenced pane must never be selected for later work.
- Fencing must reach a named bounded state without retrying forever.
- Awaiting-human exemptions must continue to suppress hard-silence reclaim.
- Ordinary successful completion may continue to reuse a resident provider
  session, but release must still end that ticket attempt's lease authority.
- Provider-specific acknowledgement generations remain distinct from the
  provider-neutral attempt lease.
- Stale-event admission and durable restart semantics remain later-story work.

## Research conclusion

The unsafe behavior is concentrated in `crates/lisa-plugin/src/lib.rs`.

The scheduler needs separate current-authority and high-water lease storage,
because revocation and monotonic redispatch have different retention needs.

Release needs to invalidate current authority centrally so no release caller
can expose a still-valid prior lease to rescheduling.

The two hard-silence reclaim paths need an explicit pane fence before release.

The existing transition state machine needs a terminal named state so a closed
pane is never treated as reusable.

The scheduler test needs direct ordering evidence in addition to final-state
assertions for lease invalidation, physical fence state, thread removal, and a
strictly higher successor dispatch on another eligible pane.
