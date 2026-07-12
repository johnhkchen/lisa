# T-034-02-01 Design — bind Codex ack to lease

## Decision summary

Use `AttemptLease::attempt_id` as the generation encoded in every Codex
assignment acknowledgement marker.

Before promotion, require the pane's stamped `AttemptLease` to validate against
`State::current_leases` for the reserved ticket.

When E-033 starts its one fresh-session recovery, mint and install a successor
lease before abandoning the old delivery. The recovery prompt then carries the
successor attempt ID.

Remove the independent process-global acknowledgement counter.

Make `seat_is_owned` part of the production acknowledgement guard and remove
its obsolete `#[allow(dead_code)]`.

## Goals

- make the Codex marker carry scheduler attempt authority;
- reject prior-attempt acknowledgement after redispatch;
- reject acknowledgement when current authority is absent;
- reject inconsistent slot, ticket, and lease stamps;
- preserve exact-once pending-to-owned promotion;
- preserve E-033's bounded one-shot fresh recovery;
- keep Claude behavior unchanged;
- avoid changing the hook payload schema;
- make tests explicitly compare current and prior leases.

## Non-goals

- gating heartbeat or other lifecycle signals;
- changing artifact admission;
- gating completion or commit transactions;
- publishing lease-aware provenance;
- persisting lease high-water state across plugin restarts;
- changing fresh Codex or Claude immediate-ownership policy;
- adding another acknowledgement or recovery protocol.

## Option 1 — retain the delivery counter and add a lease check

Keep `next_assignment_generation` for marker generation, then separately check
the slot lease against `current_leases` before promotion.

Advantages:

- smallest effect on E-033 recovery tests;
- preserves the existing global delivery sequence;
- prior delivery acknowledgements remain distinguishable within one lease.

Disadvantages:

- the marker still does not carry the attempt lease generation;
- hook evidence cannot identify the authority it is claiming;
- acceptance can pass only through scheduler-side coincidence, not transport
  identity;
- later stale-signal work would need a second attempt identity transport;
- the story explicitly says to reuse the existing marker as attempt transport.

Decision: rejected. A hidden lease check does not bind the acknowledgement to
the lease carried by the event.

## Option 2 — use the lease for initial delivery but retain a recovery counter

Set reused-seat initial generation to `attempt_id`, while a recovery prompt
continues to allocate from `next_assignment_generation`.

Advantages:

- scheduler redispatch acknowledgements carry lease identity;
- recovery retains a distinct marker without changing lease authority;
- smaller than reminting during recovery.

Disadvantages:

- the meaning of the same marker field changes by assignment state;
- a recovery acknowledgement is no longer sourced from the current lease;
- numeric collisions between the two sources are possible;
- consumers cannot infer whether generation means attempt or delivery;
- the ticket's invariant would have an undocumented exception.

Decision: rejected. Authority-bearing fields need one stable meaning.

## Option 3 — reuse one lease generation across original and recovery prompts

Use the current lease attempt ID for both the reused-session prompt and its
fresh fallback.

Advantages:

- no additional lease mint during recovery;
- every marker is sourced from the current lease;
- removes the delivery counter completely.

Disadvantages:

- a delayed acknowledgement from the abandoned reused session is
  indistinguishable from acceptance by the fresh fallback;
- that would undo E-033's delivery fence;
- the scheduler could display the recovery seat as owned before the replacement
  process accepts its prompt.

Decision: rejected. The epic requires preserving E-033's bounded acknowledged
fallback, including stale original-delivery rejection.

## Option 4 — mint a successor lease for fresh recovery

Use the dispatch lease attempt ID for the original Codex marker. If that prompt
times out, mint a successor from `lease_high_water`, install it as current, and
stamp the same lease on the slot and logical thread before sending `/exit`.

The recovery state and fresh prompt use the successor attempt ID.

Advantages:

- every marker generation has exactly one meaning: lease attempt ID;
- the old acknowledgement is stale as soon as recovery begins;
- the fresh process is truthfully represented as a new execution attempt;
- current authority, physical seat, logical thread, and hook evidence agree;
- existing high-water and `AttemptLease::mint` contracts are reused;
- no additional hook field or handshake is introduced.

Disadvantages:

- recovery changes more scheduler state than before;
- mint failure must terminate recovery safely;
- tests that assumed process-global delivery numbering require updates;
- future provenance will observe recovery as a distinct attempt when it binds
  to leases.

Decision: selected. A fresh fallback process is a replacement execution
attempt, so giving it a successor lease is the coherent authority model.

## Dispatch generation

Lease minting already happens after admission gates and before provider input.

Move acknowledgement-generation derivation to after successful minting.

For a reused Codex seat:

```text
assignment_generation = attempt_lease.attempt_id
```

For fresh Codex and every Claude assignment, retain `None` because those paths
remain immediately `Owned` and do not emit a pending-ack marker.

The `SpawnContext` already accepts `Option<u64>`, so adapter interfaces and
marker serialization remain unchanged.

## Current-lease validation

Promotion must assemble one candidate from the assigned slot, not from loosely
related maps.

The guard order is:

1. reject an already owned seat;
2. require pending or recovering assignment generation;
3. require a slot with the addressed pane and a ticket reservation;
4. require an `attempt_lease` stamp on that same slot;
5. require slot lease ticket identity to equal the reservation;
6. require assignment generation to equal slot lease `attempt_id`;
7. require slot lease `is_current(current_leases[ticket])`;
8. require the payload marker to match ticket and generation;
9. promote to `Owned`.

Every missing or inconsistent fact returns false without mutation.

Validation happens before parsing success can affect assignment state.

Duplicate acknowledgement remains inert because `Owned` has no active
generation and the explicit production ownership guard rejects it.

## Recovery lease replacement

`begin_assignment_recovery` already transitions state before sending `/exit`.

Extend that safety boundary to mint and install the successor first.

For a valid reserved slot:

1. read the ticket ID;
2. mint from `lease_high_water[ticket]`;
3. insert the successor into `lease_high_water`;
4. replace `current_leases[ticket]` with the successor;
5. replace the slot stamp;
6. replace the logical thread stamp when the thread exists;
7. enter `Recovering` with `generation = successor.attempt_id`;
8. abandon the old TUI and continue the existing bounded fallback.

The old lease becomes invalid at step 4, before provider input.

Mint failure cannot fall back to a fabricated or saturated generation. It
enters the existing actionable recovery-failed state and emits an error.

A missing reservation likewise fails recovery without granting new authority.

## State cleanup

Delete `State::next_assignment_generation`.

Delete `allocate_assignment_generation`.

Keep the `generation` fields in `SeatAssignmentState`; they now cache the
attempt ID expected by the acknowledgement state machine.

Keeping the number in assignment state preserves deadline transitions and
provides a direct consistency check against the slot lease.

## Dead-code allowance

Use `seat_is_owned` at the start of `acknowledge_codex_assignment` to reject
duplicate acknowledgement explicitly.

Remove `#[allow(dead_code)]` and its outdated future-UI comment.

Tests continue to use the helper as their ownership assertion boundary.

## Test strategy

Strengthen the real scheduler acknowledgement test rather than only testing
the isolated detector.

The acceptance test will:

1. dispatch a reused Codex ticket and capture attempt 1;
2. assert pending generation equals attempt 1;
3. release/remove the first run while preserving high-water;
4. make the same ticket schedulable and dispatch a replacement;
5. capture current attempt 2;
6. assert the replacement is pending and unowned;
7. submit an acknowledgement tagged with attempt 1;
8. assert it returns false and leaves the replacement pending;
9. submit an acknowledgement tagged with attempt 2;
10. assert it returns true and promotes the replacement to `Owned`.

Add focused fail-closed assertions for absent or mismatched current authority
where existing test structure makes them inexpensive.

Update recovery tests to assert that recovery generation equals the successor
lease and that the prior lease is no longer current.

Run focused acknowledgement and recovery tests, the plugin suite, workspace
suite, WASM check, Clippy, formatting, and diff checks.

## Final rationale

The selected design makes the existing `LISA_ASSIGNMENT` marker the actual
attempt-authority transport. Exact lease equality is checked at the scheduler
boundary before ownership changes, and E-033 recovery remains safe by becoming
a truthful successor execution attempt rather than inventing a second kind of
generation.
