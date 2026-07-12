# T-034-02-01 Structure — bind Codex ack to lease

## Change overview

The implementation is localized to the plugin scheduler.

One production source file is modified:

```text
crates/lisa-plugin/src/lib.rs
```

No source file is created or deleted.

The existing marker module remains structurally unchanged:

```text
crates/lisa-plugin/src/codex_ack.rs
```

Its `CodexAssignmentRef { ticket_id, generation }` contract already carries the
two fields required by an `AttemptLease`.

## `State` field changes

Remove:

```rust
next_assignment_generation: u64
```

The scheduler already owns the two authoritative generation stores:

```rust
current_leases: HashMap<TicketId, AttemptLease>
lease_high_water: HashMap<TicketId, AttemptLease>
```

No replacement counter field is added.

`State` continues to derive `Default`; removing the numeric field requires no
custom initialization changes.

## Removed helper

Delete:

```rust
fn allocate_assignment_generation(&mut self) -> u64
```

Every future acknowledgement generation is obtained from a successfully minted
lease.

`active_assignment_generation` remains.

It continues to project the cached attempt ID from pending and recovery state.

## `SeatAssignmentState`

The enum shape remains stable:

```rust
AssignedPendingAck {
    generation: u64,
    ack_deadline: Option<SystemTime>,
}

Recovering {
    generation: u64,
    ack_deadline: Option<SystemTime>,
}
```

Update comments so `generation` is documented as the attempt lease ID rather
than a process-local delivery counter.

The deadline fields and `Copy` derivation remain unchanged.

`Owned` and `RecoveryFailed` remain unchanged.

## Dispatch integration

In `schedule_ready_tickets`, remove generation allocation before lease minting.

Keep the existing admission and mint order:

```text
resolve route
select slot
check awaiting-human gate
mint AttemptLease
install high-water and current authority
derive acknowledgement generation
build SpawnContext
perform provider lifecycle
stamp slot and thread
```

Derive:

```rust
let assignment_generation =
    (route.agent == AgentClient::Codex && reused_seat)
        .then_some(attempt_lease.attempt_id);
```

The derivation occurs after `attempt_lease` exists and before `SpawnContext` is
constructed.

The existing shared assignment-state construction receives this value without
interface changes.

The existing adapter paths continue to serialize the value through
`SpawnContext::assignment_generation`.

## Promotion integration

Refactor `acknowledge_codex_assignment` to obtain all authority facts from the
addressed slot.

The helper remains private with the same signature:

```rust
fn acknowledge_codex_assignment(
    &mut self,
    pane_id: u32,
    payload_json: &str,
) -> bool
```

Its internal candidate tuple is conceptually:

```text
(pending_generation, reserved_ticket, stamped_lease)
```

The slot lookup must require both `ticket_id` and `attempt_lease` from the same
`AgentSlot`.

The helper compares:

```text
reserved_ticket == stamped_lease.ticket_id
pending_generation == stamped_lease.attempt_id
stamped_lease.is_current(current_leases[reserved_ticket])
```

Only then does it call `detect_codex_ack` with:

```rust
CodexAssignmentRef {
    ticket_id: reserved_ticket,
    generation: stamped_lease.attempt_id,
}
```

Successful detection inserts `SeatAssignmentState::Owned` exactly as today.

The boolean return remains the signal-file scanner's transition indicator.

## Ownership helper

Retain:

```rust
fn seat_is_owned(&self, pane_id: u32) -> bool
```

Remove its `#[allow(dead_code)]` attribute and obsolete comment.

Call it as the first duplicate/inapplicable guard in
`acknowledge_codex_assignment`.

Its semantics remain exact equality with `SeatAssignmentState::Owned`.

## Recovery lease helper boundary

Keep successor-mint logic inside `begin_assignment_recovery`; do not add a
public interface.

The method already owns the transition from expired original delivery to the
fresh fallback.

For a valid assigned slot, it will:

- clone the ticket reservation;
- mint with `AttemptLease::mint(ticket_id, lease_high_water.get(ticket_id))`;
- install the successor in `lease_high_water` and `current_leases`;
- clone the successor onto `AgentSlot::attempt_lease`;
- clone the successor onto the matching `Thread::attempt_lease`, when present;
- create `Recovering` with the successor `attempt_id`;
- continue the existing `/exit`, transition, and logging behavior.

The thread update is scoped by ticket ID, matching the existing thread map.

The slot update is scoped by the already resolved `slot_idx`.

No new pane lookup is introduced after authority replacement.

## Recovery failure shape

If no ticket-bearing slot exists, use the existing state transition to
`RecoveryFailed` without allocating a fabricated generation.

If successor minting fails, retain the old stamped lease for diagnosis but do
not send `/exit` or a recovery prompt.

Enter `Recovering` temporarily with the prior pending generation only as needed
to route through `fail_assignment_recovery`, then end in `RecoveryFailed`.

Do not update `current_leases` or `lease_high_water` on mint failure.

The existing error alert and actionable log remain the terminal behavior.

## Scanner behavior

`check_codex_ack_signals` keeps its file contract:

```text
pane-<id>.ack
```

It continues to read and remove evidence before classification.

It continues to bump activity and log only after true promotion.

Lease rejection therefore produces no ownership or liveness side effect.

Later tickets will add lease-aware handling for other signal types.

## Test changes in `lib.rs`

### Scanner fixture test

Update `test_codex_ack_signal_promotes_matching_pending_seat` to install:

- a current `AttemptLease`;
- the same stamp on the slot;
- a pending generation equal to its `attempt_id`.

This keeps the scanner test representative of the production authority
contract.

### Replacement acceptance test

Strengthen or replace
`test_recycled_codex_ownership_requires_matching_ack_exactly_once`.

Use the real scheduler for two dispatches of the same ticket.

The first dispatch yields lease N.

Release and redispatch yield lease N+1.

The replacement must remain pending after an N acknowledgement and become
`Owned` only after an N+1 acknowledgement.

Retain stale-ticket and duplicate-ack assertions where they remain useful.

### Recovery tests

Update bounded recovery tests to assert:

- original pending generation equals original current lease attempt ID;
- timeout installs a strictly newer current lease;
- recovering state generation equals the successor attempt ID;
- slot and thread stamps equal the successor;
- the old acknowledgement remains rejected;
- the successor acknowledgement promotes normally.

The existing launch-count and terminal-failure assertions remain.

### Consecutive reuse harness

The harness continues to discover generations from assignment state.

Per-ticket first attempts may share numeric ID 1; ticket identity remains part
of every marker and lease.

The recovery case continues to require a distinct generation and exactly one
fallback launch.

## Artifact files

The RDSPI directory contains:

```text
docs/active/work/T-034-02-01/research.md
docs/active/work/T-034-02-01/design.md
docs/active/work/T-034-02-01/structure.md
docs/active/work/T-034-02-01/plan.md
docs/active/work/T-034-02-01/progress.md
docs/active/work/T-034-02-01/review.md
```

Lisa owns final completion publication for these artifacts.

## Ownership and commit boundary

The ticket-owned source commit includes only:

```text
crates/lisa-plugin/src/lib.rs
```

The command must be:

```text
lisa commit-ticket --ticket-id T-034-02-01 ... --include crates/lisa-plugin/src/lib.rs
```

No ordinary index operation is permitted.

Unrelated existing modifications and untracked files remain excluded.
