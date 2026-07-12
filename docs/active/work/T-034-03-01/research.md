# Research: T-034-03-01 deterministic split-brain regression

## Ticket boundary

The ticket begins in `research` and asks for a committed deterministic test.

The required timeline is the field sequence recorded for T-031-02:

1. a slow Codex attempt owns a ticket;
2. it becomes over-budget and hard-silent;
3. Lisa times it out;
4. the ticket is dispatched to another physical pane;
5. the replacement prompt acknowledgement is missed;
6. the old process later emits activity and artifacts;
7. only the valid replacement may finish the ticket.

The acceptance boundary is regression evidence, not new scheduler behavior.

The parent story explicitly says this slice consumes S-034-01 and S-034-02 and
changes no scheduler logic.

## Existing fixture and test locations

Most scheduler tests live in the private `tests` module at the bottom of
`crates/lisa-plugin/src/lib.rs`.

Those tests can construct `State` directly and call its private methods.

This makes them able to drive scheduler time without sleeping and without a
Zellij host.

`codex_state_with_dag` creates a temporary two-ticket Codex DAG, ticket files,
work directory, and signal directory.

`codex_slot` adds a physical slot assigned to a selected fixture ticket.

`install_current_attempt` mirrors production lease installation across:

- `lease_high_water`;
- `current_leases`;
- the logical `Thread`;
- the matching physical `AgentSlot`.

`with_ledger` redirects provenance into the temporary fixture.

`read_ledger` parses the JSONL ledger back into typed provenance records.

The external provider-contract harness lives at
`docs/active/work/T-031-03/harness/run.sh`.

It drives real Git and Lisa CLI processes in a temporary repository.

It does not expose the plugin's in-memory scheduler or virtualized clock.

The split-brain regression therefore fits the plugin test scaffolding better
than the external Git harness; the later live-proof ticket owns fresh-loop
execution.

## Attempt identity

`AttemptLease` contains a ticket ID and monotonically increasing attempt ID.

`AttemptLease::mint` uses the retained high-water lease as predecessor.

`State::current_leases` is the authority registry.

`State::lease_high_water` survives revocation and release so redispatch cannot
reuse an old generation.

Production dispatch stamps the new lease into both the thread and slot.

The current attempt is therefore an equality relation among scheduler
authority, logical thread identity, and physical seat identity.

## Timeout and fencing order

`check_session_timeouts` uses injected state timestamps and the current wall
clock; no sleep is necessary in a test.

A session must exceed its configured global or phase budget and also exceed
twice `stuck_threshold_secs` in silence.

Pending completions and awaiting-human panes are excluded from reclamation.

For each timed-out ticket, the method performs these relevant operations:

1. mark the thread failed;
2. call `revoke_and_fence_attempt`;
3. emit TimedOut provenance with the fence result;
4. release the slot;
5. remove the thread;
6. record timeout activity.

`revoke_and_fence_attempt` removes the current lease before looking up or
mutating the physical slot.

It then makes the old slot terminal by setting `TransitionState::Fenced`,
clearing provider-session facts, removing assignment state, and closing the
pane at the host boundary.

In native tests, `attempt_lifecycle` records `LeaseRevoked`, `PaneFenced`, and
later `SlotReleased` events.

That vector is a deterministic observation seam for fence-before-reschedule.

`release_slot_for_ticket` revokes defensively again, clears the ticket and lease
from the slot, and preserves `Fenced` rather than returning that pane to Idle.

`find_idle_slot` and `find_slot_for_client` require Idle slots.

Consequently a fenced pane cannot be selected for redispatch.

## Replacement dispatch and missed injection

`schedule_ready_tickets` obtains still-open tickets from the DAG after their
failed thread has been removed.

It skips any existing live thread and enforces global/provider capacity.

It selects an eligible compatible or recyclable physical pane.

Only after admission gates pass does it mint the successor lease.

A resident Codex seat is treated as reused.

For reused Codex seats, scheduling installs
`SeatAssignmentState::AssignedPendingAck` with the new lease generation.

The acknowledgement deadline is not armed until the tagged prompt is actually
delivered after the clear/exit handshake.

The absence of an ack models the missed-injection/open-loop condition.

`seat_is_owned` returns false for pending and recovering assignments.

Thus a ticket reservation on the replacement pane is deliberately distinct
from acknowledged provider ownership.

## Stale acknowledgement rejection

`acknowledge_codex_assignment` requires all of the following:

- the addressed pane is pending or recovering;
- its generation equals its stamped slot lease;
- the slot lease is current for the ticket;
- the payload contains the expected ticket and generation tag.

An acknowledgement from the predecessor generation cannot promote the
replacement.

The method performs only one pending-to-Owned edge.

## Stale heartbeat rejection

Heartbeat files are named `pane-<id>.heartbeat` and contain serialized lease
JSON.

`check_heartbeat_signals` deletes each candidate regardless of validity.

It updates clocks only when pane, slot ticket, slot lease, and current scheduler
lease all agree with the payload.

After timeout the old pane has no ticket or lease and the predecessor is no
longer current.

Its resumed heartbeat therefore cannot update either the replacement thread or
the replacement pane.

## Other old-pane signals

Idle, stopped, cleared, and error files remain physically addressed by pane ID.

The fenced old slot is retained with no ticket and `TransitionState::Fenced`.

Idle handling requires an Idle slot with an assigned ticket.

Stopped/cleared handling cannot move a Fenced slot through a handoff state.

Error handling resolves a running thread by the signal's physical pane ID.

Once the replacement thread belongs to a different pane, an old-pane error
cannot fail or release it.

These consumers delete their signal files, preventing replay.

The Codex ack path is also physical-pane addressed and lease-generation bound.

Together these paths form the resumed old-pane signal vocabulary relevant to
the field reproduction.

## Artifact attribution

Every leased attempt stages workflow output under:

`.lisa/attempts/<ticket-id>/<attempt-id>/work/`

`admit_artifact` publishes only the candidate lease's private file.

The candidate must match the requested ticket and current lease exactly.

The scheduler then atomically renames admitted bytes into the canonical logical
work directory.

A predecessor artifact remains in its private directory and cannot advance the
replacement phase.

The regression can write distinct sentinel bytes into predecessor and
replacement staging directories to make cross-attribution visible.

## Completion and provenance

`request_completion` accepts attempt-originated completion only for the current
lease.

Rejected stale completion creates no pending transaction.

The admitted lease is retained in `PendingCompletion` and checked again when
the command result is handled.

`emit_provenance` stamps the attempt lease.

TimedOut history is non-authoritative even when the old pane was fenced.

Done publication requires the exact current lease and is authoritative.

The existing schema intentionally preserves the timeout row while allowing
exactly one authoritative Done row.

Accordingly, “one provenance record” in this ticket is consistent with the
prerequisite contract only when read as one authoritative completion record;
the append-only ledger also retains the predecessor timeout record.

## Existing partial regressions

`stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact` covers
stale versus current heartbeat and artifact admission after two leases exist.

`fenced_attempt_and_replacement_publish_one_authoritative_done_record` covers
manual fence/replacement setup, stale completion rejection, and provenance.

Timeout tests cover fence and monotonic redispatch separately.

Reused Codex tests cover missing acknowledgements and bounded recovery.

No single committed test currently composes the complete field timeline through
the real timeout function, scheduler redispatch, missed acknowledgement state,
old-pane signal replay, artifact isolation, and final completion.

## Constraints

The repository has unrelated modified and untracked paths.

`crates/lisa-plugin/src/lib.rs` is clean at the start of this ticket.

Ticket-owned source changes must be committed only through
`lisa commit-ticket` with exact include paths.

The ticket's phase and status frontmatter must not be edited.

The deterministic ticket should not invoke a live Codex client, depend on
credentials, sleep, or hot-reload the parent loop.

The later T-034-03-02 ticket owns the isolated fresh-loop proof and Claude
parity.

## Research conclusion

All mechanisms needed for the reproduction already exist behind deterministic
plugin test seams.

The remaining gap is compositional coverage: one test must arrange the exact
timeline and assert the safety properties at every boundary without adding a
parallel model or changing scheduler production logic.
