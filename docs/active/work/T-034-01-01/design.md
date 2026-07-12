# T-034-01-01 Design — attempt lease core type

## Decision summary

Add `AttemptLease` and `AttemptLeaseMintError` to
`crates/lisa-core/src/types.rs`.

`AttemptLease` will contain an owned `TicketId` and a public `u64` attempt ID.
Its associated `mint` helper will produce attempt 1 without a predecessor and
will checked-increment a same-ticket predecessor. Its `is_current` helper will
compare the complete lease against an optional authoritative current value.

No scheduler, thread, seat, acknowledgement, or persistence integration is
part of this ticket.

## Goals

- represent execution-attempt authority as one comparable value;
- make ticket identity part of every validity comparison;
- provide deterministic per-ticket successor minting;
- preserve strict monotonicity at the numeric boundary;
- fail closed for caller keying mistakes;
- support later serialization on threads, seats, events, and provenance;
- keep the API independent of plugin and Zellij types.

## Option 1 — tuple alias

Define `type AttemptLease = (TicketId, u64)` and free helper functions.

Advantages:

- minimal code;
- standard tuple equality and Serde support.

Drawbacks:

- positions do not communicate ticket versus attempt semantics;
- consumers can freely swap or reconstruct components;
- documentation and field access are weak at every authority site;
- associated helpers cannot make the lease contract discoverable;
- adding metadata later would be disruptive.

Decision: rejected. Authority-bearing identity deserves a named struct.

## Option 2 — opaque random identity

Mint a UUID or random token for each attempt.

Advantages:

- uniqueness does not require predecessor state;
- tokens work across multiple issuers if randomness is sound.

Drawbacks:

- the ticket explicitly requires strict monotonicity;
- `lisa-core` has no randomness dependency;
- WASM randomness adds platform concerns;
- ordering and predecessor relationships would be unavailable;
- this duplicates rather than builds on the existing `u64` generation.

Decision: rejected.

## Option 3 — timestamp-derived attempt IDs

Use wall-clock time as the generation.

Advantages:

- values appear naturally ordered;
- no explicit prior lease parameter.

Drawbacks:

- clocks can repeat or move backward;
- multiple dispatches within a clock tick can collide;
- deterministic unit tests become more complicated;
- system time is unrelated to scheduler authority;
- a timestamp cannot guarantee strict monotonicity.

Decision: rejected.

## Option 4 — core-owned mutable lease registry

Introduce a registry containing per-ticket high-water marks and current leases,
with mint, revoke, and validation methods.

Advantages:

- one object could own allocation and validity;
- revocation and high-water retention could be centralized;
- misuse through mismatched predecessor lookup would be harder.

Drawbacks:

- revocation is owned by `T-034-01-03`, not this ticket;
- scheduler storage shape is owned by `T-034-01-02`;
- persistence and restart semantics are not settled yet;
- a registry would prematurely choose HashMap ownership and mutation APIs;
- the acceptance criterion only requires the lease and shared helpers.

Decision: rejected for this ticket. The value API leaves registry policy to the
integration tickets.

## Option 5 — named value with predecessor-based minting

Define:

```rust
pub struct AttemptLease {
    pub ticket_id: TicketId,
    pub attempt_id: u64,
}
```

and associated helpers:

```rust
pub fn mint(
    ticket_id: impl Into<TicketId>,
    previous: Option<&AttemptLease>,
) -> Result<AttemptLease, AttemptLeaseMintError>;

pub fn is_current(&self, current: Option<&AttemptLease>) -> bool;
```

Advantages:

- equality naturally covers ticket and attempt identity;
- callers can retain per-ticket high-water state in the scheduler;
- no global mutable core state is introduced;
- exact validation works for all later rejection sites;
- `None` naturally represents no predecessor or no current lease;
- checked arithmetic makes the strict-order promise honest.

Drawbacks:

- callers must retain the last lease after revocation if they need to mint a
  successor;
- the type alone cannot prevent a scheduler from passing the wrong predecessor;
- every integration caller must handle a theoretically unreachable error.

Decision: selected. The mismatch check and typed error address caller misuse,
while state ownership remains in the ticket designed to integrate it.

## Attempt numbering

The first attempt for each ticket is 1. Zero remains an unmistakable
uninitialized/default sentinel outside valid minted leases and matches the
existing plugin convention in which allocated assignment generations are
nonzero.

Successors use `checked_add(1)`. `u64::MAX` returns
`AttemptLeaseMintError::AttemptIdExhausted`; it never saturates and never wraps.
This is important even if exhaustion is operationally unrealistic because the
type promises strict monotonicity for all representable inputs.

Different tickets may each have attempt 1. Per-ticket monotonicity does not
require a process-global sequence. Complete lease equality prevents those
equal numeric values from crossing authority boundaries.

## Ticket mismatch handling

If `mint("T-B", Some(lease_for_t_a))` is requested, minting returns a
`TicketMismatch` error. It does not increment T-A's number and relabel it as
T-B. This protects the future scheduler from a wrong HashMap lookup or stale
reference.

The mismatch error will retain expected and predecessor ticket IDs for useful
diagnostics. It will implement `Display` and `std::error::Error` without adding
a dependency.

## Current validation

`candidate.is_current(Some(&authoritative))` is true only when the full values
are equal. It returns false for:

- `None`, representing an unleased or revoked ticket;
- a lower or higher attempt ID for the same ticket;
- the same numeric attempt ID for a different ticket.

The helper deliberately does not use `>=`. Only the exact minted owner is
current; a future-looking or fabricated generation has no authority.

## Trait surface

`AttemptLease` will derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`,
`Serialize`, and `Deserialize`.

`Clone` supports stamping one lease across scheduler-owned records. `Hash`
supports keyed diagnostic or rejection sets. Serde supports later addition to
serialized `Thread` and event/provenance values without revisiting the type.
The struct is not `Copy` because `TicketId` owns a `String`.

The error will derive `Debug`, `Clone`, `PartialEq`, and `Eq` so edge behavior
can be asserted exactly.

## API visibility and placement

Both public types and methods live in `lisa_core::types`, beside `TicketId` and
before unrelated workflow types. `lib.rs` already exposes `pub mod types`, so
no crate-root re-export is needed.

Public fields make integration straightforward and match existing core values
such as `Thread`. Mutation after minting remains possible, but consumers already
construct and serialize public domain structs throughout this crate. Making
fields private would add boilerplate without protecting values after cloning or
deserialization.

## Test design

Unit tests will prove:

1. two interleaved ticket sequences independently produce 1, 2, and 3;
2. a first lease validates as current before replacement;
3. after minting a successor, the predecessor never validates against it;
4. the successor validates against itself;
5. no authoritative current lease rejects every candidate;
6. equal attempt IDs for different tickets do not validate;
7. a cross-ticket predecessor returns `TicketMismatch`; and
8. minting after `u64::MAX` returns `AttemptIdExhausted`.

Focused and package tests plus formatting, Clippy, and diff checks will verify
the implementation.

## Rejected scope

- changing `next_assignment_generation`;
- adding a lease field to `Thread` or `AgentSlot`;
- adding current-lease maps to plugin `State`;
- revoking or fencing attempts;
- changing Codex marker JSON;
- gating signals, artifacts, completion, or provenance;
- defining restart durability for attempt counters.

These are explicit downstream story concerns.
