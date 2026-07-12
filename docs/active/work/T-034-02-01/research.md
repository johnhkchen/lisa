# T-034-02-01 Research — bind Codex ack to lease

## Ticket boundary

The ticket starts in Research and requires the full RDSPI sequence.

Its acceptance criterion has two observable parts:

- an acknowledgement carrying the current lease generation promotes a seat to
  `Owned`;
- an acknowledgement carrying a prior generation leaves the replacement seat
  unpromoted.

It also requires removal of the dead-code allowance on the acknowledgement /
ownership path.

The ticket belongs to `S-034-02`, which gates every authority-bearing input
against the current attempt lease.

This ticket owns only Codex acknowledgement promotion.

Later tickets own completion, liveness, artifact admission, and provenance.

## Prerequisite lease model

`crates/lisa-core/src/types.rs` defines `AttemptLease`.

An `AttemptLease` contains:

- an owned `ticket_id`;
- a positive per-ticket `attempt_id`.

`AttemptLease::mint` creates attempt 1 without a predecessor and checked-
increments a same-ticket predecessor.

`AttemptLease::is_current` returns true only when the candidate exactly equals
the optional authoritative lease.

It rejects absence, different ticket IDs, older IDs, and future IDs.

`Thread` carries `attempt_lease: Option<AttemptLease>`.

The optional form preserves compatibility with older serialized records and
test fixtures.

## Scheduler lease storage

`crates/lisa-plugin/src/lib.rs` owns scheduler integration.

`State::lease_high_water` retains the latest lease ever minted for each ticket
in the current scheduler process.

`State::current_leases` contains only leases that are presently authorized.

The maps were separated by `T-034-01-03` because revocation must not erase the
predecessor needed for monotonic redispatch.

Dispatch mints from `lease_high_water`.

Dispatch inserts the minted value into both maps before provider lifecycle
side effects.

The same lease is cloned onto:

- the selected `AgentSlot`;
- the newly created `Thread`.

`AgentSlot::attempt_lease` identifies the attempt assigned to a physical pane.

`Thread::attempt_lease` identifies the attempt represented by the logical run.

`release_slot_for_ticket` revokes `current_leases` and clears the slot stamp.

Hard-silence reclaim revokes the lease, fences the pane, and then releases the
slot.

A replacement dispatch therefore receives a strictly greater attempt ID and a
different physical slot when the old slot was fenced.

## Existing Codex acknowledgement marker

`crates/lisa-plugin/src/codex_ack.rs` owns marker serialization and detection.

`CodexAssignmentRef` contains borrowed `ticket_id` and numeric `generation`.

`tag_codex_assignment` appends one line beginning with `LISA_ASSIGNMENT` and a
JSON object containing those two fields.

`detect_codex_ack` parses a Codex lifecycle payload.

It accepts only `UserPromptSubmit`.

It requires the marker to start its own prompt line.

It returns false for malformed JSON, missing prompts, other lifecycle events,
wrong tickets, and wrong generations.

The marker schema is already the attempt transport named by the story.

No hook format change is required to carry a numeric attempt generation.

The detector has fixture coverage for matching, stale-ticket, stale-generation,
clear, malformed, and unrelated-field payloads.

## Existing assignment state

`SeatAssignmentState` is private to the plugin.

Its relevant variants are:

- `AssignedPendingAck { generation, ack_deadline }`;
- `Owned`;
- `Recovering { generation, ack_deadline }`;
- `RecoveryFailed`.

Fresh Codex launches and all Claude assignments are immediately `Owned` under
the established provider contract.

Only reused physical seats assigned to Codex begin pending acknowledgement.

The pending generation is currently allocated by
`State::allocate_assignment_generation`.

That helper increments `State::next_assignment_generation` with saturation.

The counter is process-global rather than per ticket.

It is independent from `AttemptLease::attempt_id`.

Recovery also allocates another delivery generation without a new scheduler
dispatch or a new attempt lease.

The acknowledgement timeout remains unarmed until the tagged prompt is sent.

`active_assignment_generation` reads the generation from pending or recovering
assignment state.

## Existing promotion path

`State::acknowledge_codex_assignment` receives a pane ID and raw payload.

It currently:

1. reads the active assignment generation from `seat_assignments`;
2. reads the ticket ID from the pane's `AgentSlot` reservation;
3. constructs a `CodexAssignmentRef`;
4. calls `codex_ack::detect_codex_ack`;
5. replaces the assignment state with `Owned` on a match.

The helper does not read `AgentSlot::attempt_lease`.

The helper does not read `State::current_leases`.

Consequently its current notion of a match is ticket plus delivery generation,
not exact current scheduler authority.

`check_codex_ack_signals` is the production file consumer.

It scans `pane-<id>.ack`, reads and removes each file, and calls the promotion
helper.

Successful promotion bumps pane activity and logs an acknowledgement event.

Rejected or unreadable evidence is consumed without promotion.

## Current stale behavior

The existing E-033 regression test proves that a prior delivery generation
does not promote a later pending delivery state.

That test constructs generations directly around the process-global counter.

It does not install a lease in `current_leases` or on the slot.

The scheduling fixtures now mint leases because of S-034-01.

The assignment generation can happen to equal the first lease attempt ID in a
fresh test process because both start at 1.

That equality is incidental.

Interleaved tickets or recovery generations can make the counters diverge.

A pane reservation can also retain a ticket while its lease has been revoked.

In that state, the existing promotion helper can still see a matching ticket
and assignment generation even though no current authority exists.

## Recovery relationship

E-033 provides a bounded fresh-session fallback after a reused Codex prompt
does not acknowledge.

The fallback preserves the same logical scheduler dispatch and ticket lease.

It changes the provider delivery generation so a delayed acknowledgement from
the abandoned reused-session prompt cannot claim the fresh fallback.

The E-034 epic explicitly preserves this handshake and fallback rather than
creating a second recovery mechanism.

The lease identifies scheduler attempt authority.

The recovery generation distinguishes multiple prompt deliveries inside that
one attempt.

Both constraints are present at acknowledgement time.

## Dead-code allowance

`State::seat_is_owned` has `#[allow(dead_code)]` with an obsolete comment that
the UI story will consume it later.

The dashboard now projects `SeatAssignmentState` directly rather than calling
this helper.

The helper is heavily used in scheduler tests.

It has no non-test production caller, so removing the allowance without adding
a production use would expose a dead-code warning.

The natural production authority question in this ticket is whether a matching
acknowledgement may promote a seat to owned.

## Existing tests near the boundary

`test_codex_ack_signal_promotes_matching_pending_seat` covers file scanning and
matching promotion but constructs no attempt lease.

`test_recycled_codex_ownership_requires_matching_ack_exactly_once` covers stale
ticket, stale generation, exact promotion, and duplicate inertness.

It uses the real scheduler but asserts the old delivery counter semantics.

`dispatch_mints_and_stamps_strictly_new_attempt_lease` proves dispatch,
release, and redispatch lease stamping.

`test_check_session_timeouts_expired` proves revocation, fencing, and a newer
replacement lease.

No current test crosses the lease redispatch boundary with an acknowledgement
payload.

## Constraints and assumptions

The ticket must not change ticket phase or status frontmatter.

Ticket-owned source changes must be committed through `lisa commit-ticket`.

The ordinary index and unrelated dirty files must remain untouched.

The hook payload and marker JSON are already sufficient for this slice.

Claude behavior is outside this acknowledgement-specific path.

Acknowledgement rejection must fail closed when the slot stamp, current lease,
ticket reservation, or pending state is absent or inconsistent.

The implementation must preserve E-033 recovery's distinct delivery fencing
inside one current attempt.

The acceptance test must make current-versus-prior lease identity explicit,
not rely on two counters coincidentally having the same value.
