# T-034-01-01 Research — attempt lease core type

## Ticket boundary

This ticket introduces the shared value contract for an execution attempt.
It does not attach leases to scheduler state, dispatches, panes, threads,
signals, artifacts, completion, or provenance. Those integration seams are
owned by the later tickets in `S-034-01` and `S-034-02`.

The acceptance criterion requires `lisa-core` to expose:

- an `AttemptLease` value containing ticket identity and attempt identity;
- a minting helper;
- a current-lease validation helper;
- unit proof that attempts increase strictly for each ticket; and
- unit proof that a prior attempt cannot validate after a successor is current.

## Existing core identity model

`crates/lisa-core/src/types.rs` is the shared domain-type module. It defines
`TicketId` as a `String` alias near the top of the file and defines `Thread`
later in the same module. Core types commonly derive `Debug`, `Clone`,
`PartialEq`, `Eq`, and Serde traits where they cross persisted or plugin
boundaries.

`crates/lisa-core/src/lib.rs` exposes modules rather than selectively
re-exporting individual types. Consumers currently import shared values with
paths such as `lisa_core::types::Thread` and `lisa_core::types::TicketId`.
Placing the new type in `types.rs` therefore makes it public without adding a
new module or changing the crate's export convention.

`TicketId` is an owned string rather than a validated newtype. An attempt lease
must follow that existing identity boundary; ticket syntax validation remains
the responsibility of ticket parsing and DAG construction.

## Existing thread model

`Thread` represents one active agent session and includes a ticket ID, pane ID,
phase timestamps, status, agent client, concurrency, and route. It is serialized
and has a backward-compatible constructor.

The `S-034-01` story anticipates that a later ticket will stamp a lease on a
thread and seat. This ticket must therefore produce a lease that can be cloned,
compared, and serialized without changing `Thread` yet. Adding a field to
`Thread` now would exceed the ticket boundary and create persistence/defaulting
questions that belong to dispatch integration.

## Existing assignment generation

`crates/lisa-plugin/src/lib.rs` owns `next_assignment_generation: u64` and
`allocate_assignment_generation`. The allocator uses `saturating_add(1)` and
is process-global rather than per ticket. Its generation is stored only in
Codex pending/recovering seat states and is transported through
`codex_ack::CodexAssignmentRef`.

That generation solves positive acknowledgement for recycled Codex prompt
delivery. It is not yet a provider-neutral execution lease:

- fresh assignments do not all carry it;
- Claude does not use it;
- `Thread` does not carry it;
- it is keyed operationally by pane state rather than ticket attempt;
- it is not checked by timeout, artifact, completion, or provenance paths; and
- saturation permits repeated `u64::MAX`, which is not strictly monotonic.

The new core value should be compatible with this transport without moving or
rewriting the plugin state machine in this ticket. A `u64` attempt identifier
matches the existing generation representation and Serde support.

## Story and downstream constraints

`S-034-01` defines a lease as ticket ID plus monotonic attempt/generation. Its
next ticket, `T-034-01-02`, will mint a fresh lease on every dispatch and record
it as current while stamping the assigned pane/thread. `T-034-01-03` will revoke
and fence before rescheduling.

`S-034-02` later binds acknowledgement, liveness, artifact admission,
completion, and provenance to the current lease. Those sites need one exact
equality rule; provider signals must not invent their own partial comparisons.

The current ticket consequently needs a value whose equality includes both
ticket and attempt ID. Comparing only the numeric attempt would permit an event
from one ticket to validate against another ticket that happens to have the
same per-ticket generation.

## Monotonicity semantics

“Strictly monotonic per ticket” means a ticket's initial attempt receives a
defined first positive ID and each successor is greater than its predecessor.
Different tickets may independently use the same numeric attempt IDs.

Minting needs predecessor context because `AttemptLease` is a value, not a
global allocator. The scheduler will own durable/process-lifetime high-water
state in the integration ticket. Keeping allocation state out of the value
avoids hidden global mutation in `lisa-core` and keeps tests deterministic.

A predecessor from another ticket is invalid input. Silently using its numeric
ID would hide a caller keying defect. Exhaustion at `u64::MAX` is also invalid:
saturation or wrapping would violate the advertised strict-order invariant.

Both cases require a fail-closed result from minting. Neither should panic in a
runtime scheduler path.

## Current validation semantics

Validity is relational rather than intrinsic. A lease is current only when it
exactly equals the authoritative current lease supplied by the scheduler.

The absence of a current lease means every candidate is stale or revoked.
An equal attempt number belonging to another ticket is not current. A prior
lease for the same ticket stops validating immediately when a successor is
installed as current.

The helper should accept an optional authoritative lease so future revocation
can be represented by `None` without manufacturing a tombstone lease.

## Error conventions

`lisa-core` does not depend on `thiserror`. Existing modules generally use
small local error enums with `Display`/`Error` implementations when callers
need diagnostic distinctions.

Minting has two meaningful failures: predecessor ticket mismatch and exhausted
attempt space. A public error enum keeps both explicit, testable, and usable by
the later scheduler integration without adding dependencies.

## Test location and style

`types.rs` already contains a large `#[cfg(test)]` module covering constructors,
serialization, phases, health, and activity events. Lease tests belong there
beside the type and require no fixture files, temporary directories, or plugin
state.

Focused unit tests can interleave two ticket sequences to prove per-ticket
behavior, then compare old and new leases through the validation helper. Edge
tests should cover missing current state, cross-ticket equality protection,
ticket mismatch during minting, and numeric exhaustion.

## Worktree constraints

The repository begins with many unrelated modified and untracked files. Neither
`crates/lisa-core/src/types.rs` nor `crates/lisa-core/src/lib.rs` is modified at
ticket start. Ticket-owned source work should be limited to `types.rs` unless
implementation reveals a genuine export need.

The RDSPI artifacts belong under
`docs/active/work/T-034-01-01/`. Source changes must be committed with
`lisa commit-ticket` using exact repository-relative paths. The ordinary index
and unrelated worktree entries must remain untouched.

## Research conclusion

The missing primitive is a small serializable value plus fail-closed mint and
exact-current helpers in `lisa_core::types`. The core contract can establish
identity, ordering, mismatch, exhaustion, and stale-generation semantics
without changing any scheduler behavior in this ticket.
