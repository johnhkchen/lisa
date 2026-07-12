# T-035-01-03 Design — gate Owned on observed start

## Decision

Add a lease-scoped `Starting` seat assignment state for every fresh provider process.
Consume `.started` files through a provider-neutral scanner that admits the exact current
pane/ticket/attempt lease and performs the sole `Starting -> Owned` transition. Surface
the state as `starting` through the existing dashboard seat-status channel.

## Goals

- Make `Owned` mean that provider process start was positively observed.
- Preserve the scheduler's reservation before the provider starts.
- Apply the same contract to fresh Claude and fresh Codex launches.
- Reject stale, malformed, cross-pane, and replayed process-start signals.
- Preserve E-033's reused-Codex prompt-acknowledgement state machine.
- Preserve same-process Claude reuse semantics.
- Keep missing-start recovery out of this ticket.

## Option 1 — reuse `AssignedPendingAck`

The fresh launch could enter the existing Codex pending state and `.started` could
promote it.

Advantages:

- No new internal or UI variants.
- Existing yellow pending label is already visible.

Disadvantages:

- The name and fields describe a prompt acknowledgment, not process startup.
- Its optional deadline is coupled to E-033 timeout/recovery behavior.
- `active_assignment_generation` and `acknowledge_codex_assignment` intentionally use it.
- A start signal could accidentally become eligible for Codex recovery machinery.
- Operators could not distinguish process start from recycled prompt acceptance.

Rejected because state names and transition eligibility would no longer mean what they say.

## Option 2 — infer start from heartbeat or another lifecycle signal

Fresh ownership could wait for an existing `.heartbeat`, `.idle`, or `.stopped` signal.

Advantages:

- Avoids a new consumer.
- Heartbeats already carry attempt leases.

Disadvantages:

- A provider can start successfully before making a tool call.
- Idle and stopped signals describe later lifecycle moments and do not carry the same
  positive start contract.
- T-035-01-01 deliberately added a provider-neutral process-start producer.
- Waiting for unrelated activity would misclassify successfully started quiet sessions.

Rejected because it ignores the purpose-built producer and changes the meaning of start.

## Option 3 — add `Starting` and a dedicated start consumer

Fresh dispatch records `Starting { generation }`. A `.started` scanner parses the
candidate lease and promotes only the exact current starting assignment.

Advantages:

- State semantics are explicit and operable.
- The generation binds pending state to one attempt.
- Admission can exactly reuse the established heartbeat lease checks.
- The consumer is provider-neutral and symmetric with the producer.
- Reused Codex acknowledgment remains isolated.
- T-035-01-04 can later add a bounded deadline without reinterpreting E-033 fields.

Disadvantages:

- Adds one internal state, one UI state, and exhaustive match updates.
- Fresh-launch classification must be explicit in scheduling.

Chosen because it matches the contract and leaves recovery extensible without coupling.

## Fresh-launch classification

Use a boolean derived from the dispatch route before seat lifecycle mutation:

- no resident process: fresh launch;
- cross-provider recycle: fresh launch after exit;
- adapter `FreshExec`: fresh process launch;
- same-provider `ClearHandshake`: in-process reuse, not a fresh launch.

The existing `reused_seat` fact and selected adapter reset strategy provide these facts.
The scheduling branch already knows whether it prepared a launch script. The design will
record `fresh_launch` alongside `launch_cmd` construction and choose seat state afterward.

For the immediate acceptance path, a truly unused slot is sufficient. The implementation
must nevertheless avoid leaving other fresh-process routes on immediate `Owned`.

## Starting state

Add:

```text
Starting { generation: u64 }
```

The generation is the minted `AttemptLease::attempt_id`. It is diagnostic and provides
a local equality check before consulting the slot and current lease registries.

`seat_is_owned` remains unchanged: only `Owned` returns true.
`active_assignment_generation` remains scoped to E-033 ack/recovery states.

## Start-signal admission

`acknowledge_process_start(pane, candidate)` returns true only when:

1. the current seat state is `Starting` with the candidate generation;
2. the pane has an assigned slot with a ticket and attempt lease;
3. candidate ticket and attempt match the slot;
4. candidate is exactly current in `current_leases`.

It then replaces `Starting` with `Owned`. Calling it in any other state is inert.

`check_process_start_signals` scans `pane-<id>.started`, reads and parses an
`AttemptLease`, removes the file, and attempts admission. Consume-before-admit matches
the heartbeat pattern and makes malformed/stale signals one-shot.

## Poll ordering

Consume process-start signals early in `poll_tick`, beside heartbeat signals and before
health, artifact, and timeout decisions. This makes observed provider start visible in
the same scheduler tick and gives future start-timeout logic a clean ordering rule:
matching start wins before timeout evaluation.

## UI

Add `ui::SeatAssignmentStatus::Starting`:

- label: `starting`;
- color: yellow;
- mapping: internal `Starting` to UI `Starting`.

The active thread row already reserves enough width. No new layout is needed.

## Test design

Primary native scheduler test:

1. construct a state with a truly fresh pane and one ready ticket;
2. call `schedule_ready_tickets`;
3. assert the pane holds `Starting` with the current attempt generation;
4. assert `seat_is_owned` is false;
5. assert the dashboard row contains `starting`;
6. write the current attempt lease to `pane-<id>.started`;
7. call the real scanner;
8. assert the signal was consumed and the state is `Owned`;
9. assert the dashboard row contains `owned`.

Fencing cases should cover a stale or malformed signal remaining non-owned. Existing
E-033 tests protect recycled ack/recovery; the split-brain suite protects E-034 generally.

## Non-goals

- No startup deadline or recovery/failure state.
- No producer or hook configuration changes.
- No change to atomic fresh launch delivery.
- No change to prompt acknowledgments.
- No ticket frontmatter mutation or shared artifact publication.
