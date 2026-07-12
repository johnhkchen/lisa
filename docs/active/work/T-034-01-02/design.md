# T-034-01-02 Design — mint lease on dispatch

## Decision summary

Add a scheduler-owned `current_leases: HashMap<TicketId, AttemptLease>` to
`State`. On every admitted dispatch, mint a successor from the map's existing
value, store it as current, and clone the same value onto the selected
`AgentSlot` and newly created `Thread`.

The map retains the last minted lease across release so redispatch can mint a
strict successor. Release clears the seat stamp because an idle physical pane
is not assigned. Thread removal naturally removes the run stamp.

Minting occurs after all admission gates and before pane rename/input or other
assignment side effects. A mint failure logs an error and skips the ticket.

## Design goals

- Mint exactly once for every scheduler dispatch.
- Use one lease value across authoritative state, seat, and thread.
- Make redispatch strictly monotonic for the same ticket.
- Cover Claude and Codex, fresh and reused panes, identically.
- Preserve the separate meaning of Codex acknowledgment generations.
- Fail closed before launching when a lease cannot be minted.
- Preserve backward compatibility for serialized `Thread` records.
- Keep the change small enough for the later revocation/fencing ticket to
  extend without undoing this work.

## Option 1 — derive the attempt from Codex assignment generation

Use `next_assignment_generation` as the lease attempt ID and place that number
on scheduler records.

Advantages:

- reuses an existing monotonic counter;
- avoids a new state map;
- aligns with generation-tagged Codex prompts on some reuse paths.

Disadvantages:

- the counter is global rather than per ticket;
- fresh launches and Claude assignments do not allocate generations;
- Codex recovery allocates another generation without a new scheduler
  dispatch;
- generation identity describes delivery acknowledgment, not authority to act
  for the ticket;
- it cannot pass the provider-parity boundary in the story.

Rejected because it conflates two lifecycles and leaves valid dispatch paths
without leases.

## Option 2 — infer the next attempt from active or historical threads

Add a lease to `Thread` and search `threads` for the predecessor when
dispatching.

Advantages:

- no additional registry;
- the thread already represents a ticket run;
- the current lease is visible beside run metadata.

Disadvantages:

- completed/reclaimed threads are removed before redispatch;
- retaining old threads only for attempt history conflicts with concurrency and
  active-thread guards;
- a removed thread loses the monotonic high-water mark;
- reconstructing history from logs or artifacts adds unrelated persistence and
  I/O behavior.

Rejected because the existing lifecycle intentionally deletes threads and the
acceptance criterion explicitly exercises redispatch after release.

## Option 3 — store only a per-ticket numeric counter

Add `attempt_ids: HashMap<TicketId, u64>`, increment the number on dispatch,
then construct an `AttemptLease` directly.

Advantages:

- minimal stored data;
- naturally survives seat/thread release;
- straightforward per-ticket monotonicity.

Disadvantages:

- bypasses the shared `AttemptLease::mint` contract from the prerequisite;
- duplicates checked-increment behavior and error handling;
- permits ticket identity and counter lookup to drift apart;
- does not itself represent the authoritative current lease required by the
  acceptance criterion.

Rejected because the prerequisite exists specifically to make mint sites share
one validated helper and complete lease identity.

## Option 4 — current lease map plus seat and thread stamps

Add `current_leases: HashMap<TicketId, AttemptLease>` to `State`, an optional
lease to `AgentSlot`, and an optional lease to `Thread`.

Advantages:

- the map is explicit scheduler authority keyed by ticket;
- the complete predecessor is available for `AttemptLease::mint`;
- retaining the entry across release preserves monotonicity;
- one minted value can be cloned to every representation;
- optional stamps preserve default construction and old thread JSON;
- the private slot stamp directly identifies the physical assigned seat;
- later fence/rejection work can query one canonical registry.

Disadvantages:

- the map currently combines current authority with the high-water predecessor;
- cloning creates multiple representations that code must update atomically;
- plugin restart resets the map unless persistence is added later;
- every `AgentSlot` fixture needs an added field.

Selected because it directly models the ticket, seat, and thread requirements
with the existing core lease contract and scheduler lifetime.

## Option 5 — embed the lease in `SeatAssignmentState`

Place an `AttemptLease` in every pending/owned/recovering variant and use that
as the seat stamp.

Advantages:

- assignment acceptance state and attempt authority are adjacent;
- no separate lease field on `AgentSlot`;
- acknowledgment and recovery code can inspect the attempt.

Disadvantages:

- `SeatAssignmentState` currently derives `Copy`; `AttemptLease` owns a string
  and would make it non-`Copy`;
- many transition matches and tests would need mechanical lease propagation;
- a seat can be assigned with immediate ownership while the physical slot is
  the existing routing reservation;
- provider acceptance is a narrower concern than provider-neutral lease
  identity;
- it increases coupling before stale acknowledgment gating is in scope.

Rejected for this slice. The private `AgentSlot` is the stable physical-seat
record, while `SeatAssignmentState` remains the provider acknowledgment state.

## Authoritative state semantics

`current_leases[ticket_id]` is the latest successfully minted lease for the
ticket. During an active dispatch it is the current authority. After release,
this ticket retains it as the predecessor/high-water record needed by the next
dispatch.

The next ticket owns revocation semantics. It may split current authority from
high-water history or introduce an explicit revoked state. Retention here is
intentional and necessary; deleting the record would violate monotonicity.

Only `schedule_ready_tickets` writes a successor lease. Neither acknowledgment
recovery nor prompt resend mints a scheduler attempt.

## Dispatch ordering

The chosen order inside `schedule_ready_tickets` is:

1. pass active-thread, cap, provider, slot, and human-attention gates;
2. mint from `current_leases.get(ticket_id)`;
3. insert the clone into `current_leases`;
4. begin pane lifecycle side effects;
5. reserve the slot and stamp its lease;
6. create the thread and stamp the same lease;
7. insert the thread and log dispatch.

Minting before external effects prevents an unleased launch. Storing the lease
immediately after minting ensures any subsequent scheduler observation sees the
authority selected for that dispatch.

The remaining operations are synchronous and do not return fallible results
that can roll back scheduler state. This matches the existing scheduling
transaction boundary.

## Mint failure behavior

`AttemptLease::mint` can fail on a cross-ticket predecessor or exhausted ID.
Both indicate corrupted/unusable scheduler state rather than temporary slot
pressure.

The scheduler will:

- log `ActivityEvent::Error` with ticket and cause;
- increment the unscheduled count;
- continue considering other ready tickets;
- perform no pane rename, input, slot reservation, or thread insertion for the
  failed ticket.

No fallback counter or wraparound is allowed because either could grant stale
authority.

## Thread compatibility

Add `pub attempt_lease: Option<AttemptLease>` to `Thread` with serde default and
skip serialization when absent. `Thread::new` initializes it to `None` because
construction alone is not dispatch authority; the scheduler must explicitly
stamp a successfully minted lease.

This keeps existing unit fixtures concise and permits older serialized threads
without the field to deserialize. A focused core test will extend the existing
backward-compatibility assertion.

## Seat semantics

Add `attempt_lease: Option<AttemptLease>` to private `AgentSlot`. Discovery and
test helpers initialize it to `None`. Dispatch sets it to the minted lease when
it sets `ticket_id`. Release clears both together.

An idle slot therefore cannot appear to carry authority for its prior ticket.
The scheduler registry remains the only retained predecessor after release.

## Test design

Add one scheduler test using the real `schedule_ready_tickets` path.

The test will:

1. create one ready ticket and fresh slot;
2. dispatch it and capture the current, thread, and slot leases;
3. assert attempt 1 and exact equality across all three records;
4. release the slot and remove the active thread;
5. assert the idle slot stamp is cleared while the registry retains attempt 1;
6. redispatch the same ticket;
7. assert the current attempt is strictly higher;
8. assert the new thread and assigned slot equal the new current lease.

Also extend core thread compatibility coverage to prove old JSON yields no
lease and `Thread::new` starts unstamped.

## Non-goals

- Revoking current authority during timeout or release.
- Terminating or disqualifying stale panes.
- Validating hook, liveness, artifact, or completion events against a lease.
- Persisting lease high-water marks across plugin restarts.
- Replacing Codex assignment-generation tags.
- Exposing attempt IDs in pane names, dashboard rows, or provenance.

## Final rationale

The selected design gives the scheduler a single provider-neutral authority
record and makes assignment identity inspectable on both physical and logical
run records. It consumes the core helper exactly as intended, preserves
redispatch history without retaining dead threads, and leaves acknowledgment
and later fencing responsibilities at their existing boundaries.
