# Design — T-048-01-02 park-instead-of-churn

## Goals

Turn an admitted blocking Review disposition into bounded scheduler behavior.

The design must make operator/world blocks release their seats immediately,
bound agent-owned retries within one loop, retain durable visible board state,
and append replayable block retry/park/unpark provenance.

It must preserve the completion, lease, and Done transaction contracts.

## Option 1 — retain `ThreadStatus::Parked`

The plugin already has a parked thread variant and parked-thread UI rows.

The scheduler could call `thread.park()` after a block and leave the thread in
the `threads` map.

Advantages:

- reuses an existing type and existing UI projection;
- keeps pane and attempt data directly available in memory.

Disadvantages:

- a retained thread is still special scheduler state;
- `schedule_ready_tickets` skips every ticket present in `threads`, so changing
  frontmatter back to `open` would not by itself make the ticket schedulable;
- restart loses the parked thread even though the ticket must stay parked;
- the pane association makes it easy to imply the ticket still owns a seat;
- durable visibility would still need frontmatter or another persisted store.

Rejected. It conflicts with status-driven unpark semantics and makes the
in-memory representation stronger than durable repository truth.

## Option 2 — keep tickets open and maintain an in-memory parked set

The scheduler could release threads and add ticket IDs to a `HashSet` excluded
by `schedule_ready_tickets`.

Advantages:

- small local implementation;
- no ticket frontmatter writes.

Disadvantages:

- restart forgets every park and recreates the churn;
- the DAG and board continue reporting the ticket as open;
- unparking requires a new special command/state mutation;
- it ignores the ticket's explicit direction to formalize `status: blocked`.

Rejected because it is not durable or operator-legible.

## Option 3 — durable status is authority; canonical disposition is payload

On a park decision, update the ticket to `status: blocked`, append provenance,
release the slot, and remove the thread.

The already-admitted canonical `review-disposition.json` carries the complete
reason, owner, ask, steps, check, and unstructured marker semantics.

Advantages:

- directly uses `Dag::can_start`'s existing exclusion;
- survives restart;
- remains visible as a normal blocked ticket;
- owns no pane or running-thread capacity;
- changing status back to `open` naturally restores DAG eligibility;
- does not duplicate the structured block in a second persisted file.

Disadvantages:

- requires a scheduler-owned frontmatter write;
- later UI features must read the canonical disposition to render the ask;
- unpark duration provenance must reconstruct the prior park record.

Chosen. These tradeoffs match the story's intended durable state model.

## Policy decision

Introduce a small pure decision function with a fixed limit of two agent-owned
retries per loop.

Inputs:

- `RemedyOwner`;
- number of agent-block retries already consumed for the ticket.

Outputs:

- retry with `retry_count` and `retry_limit`;
- park with owner, count/limit metadata, and `recheck_eligible`.

Operator and world owners always produce `Park` on first observation.

World produces `recheck_eligible = true`; agent and operator produce false.

Agent produces Retry while consumed count is below two, then Park. Thus the
initial blocked attempt may launch two fresh Review attempts; a block from the
third attempt parks.

The fixed constant is local scheduler configuration, not user-facing TOML. The
ticket explicitly asks for a small fixed count per loop, so widening the public
configuration surface would exceed scope.

## Retry state

Keep only consumed agent-block retry counts in a
`HashMap<TicketId, u8>` on `State`.

This map is not scheduling authority. It decides whether another open attempt
is allowed before durable parking.

It intentionally resets when the loop process restarts, matching “per loop.”

Operator/world decisions do not need entries.

An unpark clears any stale count so a future independent block episode starts
with a fresh bound.

## Policy observation seam

Add a scheduler method that scans current Review threads after artifact
admission and before later timeout/reseat behavior.

For each current lease it calls the existing Review input admission path. This
ensures both Review artifacts are canonical and validated through the same
authority boundary used by completion.

Only `ReviewDisposition::Block` triggers policy. Pass and Invalid remain owned
by existing completion/protocol behavior.

The method takes a snapshot before mutating threads to avoid borrow conflicts.

After each decision it immediately tears down that exact attempt, so the same
block cannot be processed again on a later pass.

## Retry transition ordering

For an agent retry:

1. append a retry provenance row while the thread and lease still exist;
2. increment the in-memory consumed count;
3. release the ticket's slot, revoking the lease;
4. remove the thread;
5. leave ticket status and phase unchanged.

The end-of-poll scheduler then sees the open Review ticket and may mint the next
attempt. Ready-ticket ordering remains the existing DAG policy.

No failure outcome is fabricated: an agent-authored block is represented as a
block retry, not an assignment failure or timeout.

## Park transition ordering

For a park:

1. resolve the ticket file from the DAG;
2. update its status to `blocked`;
3. if the write fails, log and leave the live attempt intact;
4. append the park provenance row;
5. release the slot and revoke the lease;
6. remove the thread;
7. rebuild the DAG before scheduling further work.

Status-first ordering prevents a released ticket from becoming briefly ready
when the durable exclusion failed.

The canonical Review disposition is not deleted or rewritten.

## Provenance options

### Reuse execution failure rows

Rejected. A deliberate block is not a runtime failure, and execution rows have
no retry ordinal or limit.

### Reuse assignment transition rows

Rejected. The agent has already owned the attempt and produced Review output;
classifying this as pre-ownership assignment failure would corrupt meaning.

### Append “park” for every retry

Rejected. A retry leaves the ticket open, so calling it parked would make the
ledger disagree with durable state.

### Additive extension to parking/block transition provenance

Chosen. Extend `ParkingTransitionType` with `Retry` and extend its record with:

- optional retry count;
- optional retry limit;
- a boolean `recheck_eligible` marker.

Retry rows use the exact attempt lease and `n/limit` values. Final agent park
rows retain the consumed count and limit. Operator/world park rows omit retry
values. World park rows explicitly set recheck eligibility.

Fields receive serde defaults/skip rules so schema-4 park/unpark rows remain
readable. The writer schema advances to version 5 because new rows/fields are
durable ledger vocabulary.

The existing `ParkingTransitionRecord` name is retained to minimize consumer
breakage; its documentation broadens to blocked-work transitions.

## Unpark detection options

### Persist an in-memory parked map

Rejected as the sole source because it fails across restart.

### Add fields to ticket frontmatter

Rejected. Owner, check, and timestamps already exist in canonical artifacts and
provenance. More frontmatter would create duplicated state.

### Replay latest parking transition

Chosen. Read the mixed ledger, keep the latest retry/park/unpark record per
ticket, and find tickets whose durable status is now `open` while their latest
block transition is `Park`.

Append one `Unpark` row using the prior park's lease, owner, and start time;
compute end/duration at observation time. Since that new row becomes latest,
subsequent polls are idempotent.

Scheduling uses only open status. Ledger replay is observational and cannot
prevent a ticket from becoming ready.

## Structured block and recheck marker

The canonical disposition is the durable structured payload. No parallel
`ParkedBlock` state is introduced.

The world-owned provenance row explicitly records `recheck_eligible: true`.

This ticket never executes the check and never claims a remedy is satisfied.
T-048-02-02 can locate open/blocked world cases through the canonical
disposition and provenance marker.

## Error behavior

Artifact admission and parsing remain fail-closed.

Invalid dispositions are not silently parked as structured blocks; the existing
Review protocol warning continues to require correction.

Status mutation failure blocks teardown and emits an activity error.

Provenance append failure follows existing best-effort semantics: log the error
but preserve the durable scheduling transition.

Malformed historical ledger lines are skipped during unpark reconstruction,
matching other provenance readers' tolerant replay behavior.

## Verification decision

Add focused core provenance serialization/replay tests for the additive shape.

Add pure scheduler policy tests for exact count and owner classification.

Add a production-shaped two-seat replay using real temporary tickets, attempt
artifacts, slots, leases, scheduler methods, and the mixed ledger.

Add an agent sequence test that observes retry 1/2, retry 2/2, final park, no
further reseat, then `status: open`, one unpark row, and renewed schedulability.

Run focused tests, `cargo check --workspace`, and the full workspace test suite.
