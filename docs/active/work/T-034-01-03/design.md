# T-034-01-03 Design — revoke and fence before reschedule

## Decision summary

Separate lease high-water history from current authority, revoke current
authority in the shared release boundary, and make hard-silence reclamation
close and permanently disqualify the old terminal pane before release.

Add a terminal `TransitionState::Fenced` state. A fenced slot is retained for
diagnostics but is never eligible for scheduling, transition fallback, or
resident-session reuse.

Both hard-silence paths use one helper with the following ordered contract:

1. remove the ticket's exact current lease;
2. close the terminal pane and mark its slot `Fenced`;
3. call `release_slot_for_ticket`;
4. remove the logical thread and publish the existing outcome.

Record these lifecycle boundaries under `cfg(test)` so a scheduler test can
assert their strict order without making production state an event journal.

## Goals

- Make a timed-out lease fail `is_current` before ticket release.
- Terminate the old provider process before ticket release.
- Prevent the closed pane ID from ever becoming scheduler-eligible.
- Preserve strict attempt-ID monotonicity across revocation and redispatch.
- Cover both budget-plus-silence and pure stale-silence reclamation.
- Make ordering explicit and directly testable.
- Preserve existing completion/session-reuse behavior outside hard timeouts.
- Avoid unbounded fence retries or ambiguous intermediate ownership.

## Non-goals

- Replacing a closed terminal pane automatically.
- Persisting lease high-water state across plugin restart.
- Adding leases to signal file payloads.
- Rejecting stale acknowledgement, heartbeat, artifact, completion, or
  provenance events at their individual ingestion sites.
- Unifying attempt IDs with Codex delivery generations.
- Changing timeout thresholds or awaiting-human exemptions.
- Redesigning normal `/clear` and `/exit` reuse transitions.

## Lease storage options

### Option 1 — remove `current_leases` on timeout

Revocation can be represented by removing the current map entry.

Advantages:

- smallest timeout-path mutation;
- directly uses `AttemptLease::is_current(None)` semantics;
- old lease immediately becomes invalid.

Disadvantages:

- dispatch has no predecessor after release;
- the next attempt restarts at attempt 1;
- violates the prerequisite monotonic redispatch contract;
- makes a stale attempt indistinguishable from a newly minted attempt after
  revocation.

Rejected because it fixes authority by breaking identity monotonicity.

### Option 2 — mint a tombstone successor during revocation

Revocation could mint attempt N+1 and leave it in `current_leases` without
assigning it to a pane or thread.

Advantages:

- attempt N stops validating;
- one map remains sufficient;
- later dispatch can mint N+2.

Disadvantages:

- the registry claims an unassigned attempt is current;
- state no longer means what its name says;
- later surface gates could accept a fabricated tombstone if it leaked;
- every timeout consumes two attempt IDs;
- absence can no longer represent no authority.

Rejected because a fake current lease violates the state contract.

### Option 3 — store a revoked flag beside the lease

Replace the map value with an enum such as `Current(AttemptLease)` or
`Revoked(AttemptLease)`.

Advantages:

- preserves one per-ticket registry;
- retains high-water history;
- models revocation explicitly.

Disadvantages:

- every validation and mint caller must interpret the enum correctly;
- the core `is_current(Option<&AttemptLease>)` helper no longer matches the
  scheduler authority representation directly;
- later code can accidentally extract the revoked lease as current;
- combines history and authority behind branching rather than separating their
  lifetimes.

Viable, but not selected.

### Option 4 — separate high-water and current maps

Add `lease_high_water: HashMap<TicketId, AttemptLease>` and retain
`current_leases` exclusively for presently authorized attempts.

Dispatch mints from `lease_high_water`, then inserts the same lease into both
maps before launching the pane.

Revocation removes only `current_leases[ticket]`.

Release also removes current authority idempotently, while high-water history
survives for redispatch.

Advantages:

- each map has one meaning;
- absent current entry directly uses the core validation contract;
- monotonic redispatch remains intact;
- revocation is a simple idempotent removal;
- later surface gates query one unambiguous authority map;
- release can enforce the global no-valid-lease-before-reschedule invariant.

Disadvantages:

- duplicates one small lease value per active ticket;
- requires updating the prior dispatch test and comments;
- consistency during dispatch depends on inserting the same minted value into
  both maps.

Selected because it makes authority and history explicit and independently
correct.

## Revocation placement options

### Timeout callers only

Each hard-timeout path can remove `current_leases` before fencing.

This proves the immediate acceptance scenario but leaves other release callers
able to make a ticket reschedulable while its lease remains current.

Rejected as insufficient for “no code path reschedules” safety.

### `release_slot_for_ticket` only

The shared release function can revoke at its first line.

This covers every release caller, but a timeout sequence would revoke only when
release begins. The pane fence must occur strictly before release, and the
acceptance test asks for revocation before the fence as well.

Using release alone therefore cannot express the full ordered timeout boundary.

### Explicit timeout revocation plus idempotent release revocation

The timeout helper removes current authority, fences the pane, then calls
release. Release independently removes current authority at entry.

Advantages:

- timeout ordering is explicit;
- every other release path is fail-closed;
- repeated removal is harmless;
- no caller can rely on release retaining authority.

Selected. The duplication is deliberate defense at two different contracts:
the timeout sequence and the general release invariant.

## Pane fencing options

### Provider-specific graceful exit

Resolve the adapter and send `/exit` or another provider command.

Advantages:

- may preserve the terminal pane for reuse;
- follows existing cross-provider recycle mechanics.

Disadvantages:

- provider acknowledgement is not guaranteed;
- exit uses a grace period and fallback launch semantics intended for healthy
  sessions;
- a silent or wedged TUI may ignore the command;
- a timeout fence cannot depend on the process cooperating;
- creates a retry/timer state rather than a terminal bounded result.

Rejected for hard fencing.

### Send interrupt bytes and return the pane to idle

Send Ctrl-C and mark the slot as a shell.

Advantages:

- keeps capacity;
- simple host interaction.

Disadvantages:

- Ctrl-C may cancel only a child tool, not the agent TUI;
- process state after the interrupt is unknown;
- immediate reuse can type into the wrong process;
- no positive termination observation exists.

Rejected because it is not a reliable process fence.

### Close the Zellij terminal pane

Call `close_terminal_pane(pane_id)` and disqualify its slot record.

Advantages:

- termination occurs at the pane/process container boundary;
- provider behavior is irrelevant;
- the operation has no retry loop;
- the old pane ID cannot host a later scheduler attempt;
- matches the preservation goal better than leaving a timed-out writer alive.

Disadvantages:

- reduces available scheduler capacity;
- current one-shot slot discovery does not create a replacement;
- the host call is fire-and-forget;
- unit tests need a host-free wrapper.

Selected. Capacity loss is an explicit bounded failure mode. Automatically
rebuilding layout capacity is a separate recovery concern.

## Fence state options

### Remove the `AgentSlot` record

Removing the slot would prevent selection.

This loses the named terminal state required by the ticket and makes the pane's
fate less visible in state-level tests and diagnostics.

Rejected.

### Add `TransitionState::Fenced`

The private transition enum already gates slot eligibility. Add a terminal
variant representing a pane that Lisa closed and will never reuse.

The fence operation sets:

- `transition_state = Fenced`;
- `transition_started_at = None`;
- `has_session = false`;
- `last_client = None`;
- no cooldown or pending acknowledgement.

Release clears the ticket and lease stamps but preserves `Fenced` and avoids
renaming the closed pane.

Selected because it is named, terminal, bounded, and naturally excluded by the
existing `Idle`-only selection predicate.

## Shared timeout fence helper

Introduce `revoke_and_fence_attempt(&mut self, ticket_id) -> FenceOutcome`.

The helper resolves the assigned slot and validates that its lease matches the
current authority before mutation where possible.

The helper removes `current_leases[ticket_id]` first.

It then closes and marks the matching slot `Fenced`, removes pane-specific
pending transition/input state, and returns a named outcome.

The outcome distinguishes at least:

- `Fenced { pane_id }`;
- `AlreadyFenced { pane_id }`;
- `NoAssignedPane`.

All outcomes are terminal for one invocation; none schedule a retry.

For the normal hard-timeout case, the state is `Fenced` before release begins.

If scheduler state is already inconsistent and no pane is found, authority is
still revoked and release continues. The missing pane is logged rather than
turning the timeout scan into an infinite retry.

## Pending-input cleanup

`send_line_to_pane` queues deferred Enter keypresses.

A pane closed during fencing must not receive a later queued Enter request.

The fence helper removes pending Enter records for that pane.

It also clears:

- `seat_assignments[pane_id]`;
- awaiting-human state;
- attention debounce state.

The normal awaiting-human guard prevents reaching the fence while a question
is active, but cleanup keeps the terminal fence self-contained.

## Release behavior after this change

At entry, `release_slot_for_ticket` removes `current_leases[ticket_id]`.

For an ordinary non-fenced slot, existing behavior remains:

- preserve the resident session;
- establish cooldown;
- rename to idle;
- remove seat assignment.

For a fenced slot:

- clear ticket and slot lease;
- keep `transition_state = Fenced`;
- keep `has_session = false`;
- use no cooldown;
- do not rename the already-closed terminal pane;
- remove seat assignment;
- log a fenced release.

The high-water lease is never removed by release.

## Ordering test design

Add a private test-only lifecycle trace to `State`.

The trace enum records:

- `LeaseRevoked(ticket_id)`;
- `PaneFenced { ticket_id, pane_id }`;
- `SlotReleased(ticket_id)`.

Production builds contain neither the field nor recording storage.

The acceptance test constructs a real current lease, stamps it on a timed-out
thread and first slot, and provides a second eligible slot.

After `check_session_timeouts`, it asserts the trace order exactly:

1. lease revoked;
2. pane fenced;
3. slot released.

It also asserts:

- the old lease fails against `current_leases.get(ticket)`;
- high-water still equals the old lease;
- the first slot is terminal `Fenced` and unassigned;
- the old thread is gone;
- the ticket is in the existing named `TimedOut` alert/outcome state.

Then it invokes the real scheduler and proves the second slot receives a lease
whose attempt ID is strictly greater than the fenced attempt.

The test thereby covers both strict ordering and end-to-end rescheduling.

Add a focused stale-detection assertion that the shared helper also fences its
pane, preventing divergence between the two hard-silence paths.

## Failure and compatibility behavior

Lease revocation is idempotent. Missing current authority does not block
physical fencing.

Pane fencing is idempotent at the state level. An already fenced slot does not
issue another close request.

No fence retry timer is created.

Dispatch remains fail-closed if minting from high-water fails.

Existing serialized public types do not change.

All new enums and maps are private plugin implementation details.

Normal successful completion keeps provider reuse and cooldown behavior.

Existing direct test fixtures without leases remain supported; release simply
has no current authority to remove.

## Decision outcome

The selected design gives the scheduler three explicit truths:

- `lease_high_water` says the greatest attempt ever minted in this process;
- `current_leases` says which attempt, if any, currently owns a ticket;
- `TransitionState::Fenced` says a timed-out physical pane is permanently
  disqualified.

The timeout pipeline moves through those truths in one direction and finishes
without a retry loop: current authority absent, old pane closed/fenced, slot
released, logical thread removed, ticket available for a strictly newer
attempt on another eligible pane.
