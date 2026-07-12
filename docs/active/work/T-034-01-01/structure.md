# T-034-01-01 Structure — attempt lease core type

## Change inventory

One source file is modified:

- `crates/lisa-core/src/types.rs`

No source files are created or deleted. `crates/lisa-core/src/lib.rs` remains
unchanged because its public `types` module already exposes the new API.

The six phase artifacts are created under:

- `docs/active/work/T-034-01-01/`

Lisa, rather than this ticket implementation, owns the final artifact and
ticket completion commit.

## `types.rs` public additions

Place the attempt lease immediately after `TicketId`. This keeps execution
identity primitives together and makes them available before `Thread` and
event definitions that later tickets may extend.

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttemptLease {
    pub ticket_id: TicketId,
    pub attempt_id: u64,
}
```

The field documentation defines `attempt_id` as a positive, strictly increasing
per-ticket generation created by `mint`.

## Mint error

Add a public error enum adjacent to the lease:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptLeaseMintError {
    TicketMismatch {
        ticket_id: TicketId,
        previous_ticket_id: TicketId,
    },
    AttemptIdExhausted {
        ticket_id: TicketId,
    },
}
```

Implement `fmt::Display` for concise scheduler diagnostics and
`std::error::Error` for ordinary Rust error composition. `std::fmt` is already
imported by `types.rs`, so no new crate dependency is needed.

The mismatch fields distinguish the requested ticket from the predecessor's
ticket. The exhaustion field identifies the affected ticket without retaining
an already-known `u64::MAX` constant.

## `AttemptLease` implementation

Add an inherent `impl AttemptLease` containing exactly the shared helpers
required by the ticket.

### `mint`

Signature:

```rust
pub fn mint(
    ticket_id: impl Into<TicketId>,
    previous: Option<&Self>,
) -> Result<Self, AttemptLeaseMintError>
```

Organization:

1. Convert ticket identity once into an owned `TicketId`.
2. With no predecessor, select attempt ID 1.
3. With a predecessor, compare its ticket identity to the requested ticket.
4. Return `TicketMismatch` on inequality.
5. Checked-increment the predecessor attempt ID.
6. Return `AttemptIdExhausted` if increment fails.
7. Construct and return the lease.

This method does not inspect clocks, panes, providers, phases, or global state.

### `is_current`

Signature:

```rust
pub fn is_current(&self, current: Option<&Self>) -> bool
```

Implementation shape:

```rust
current == Some(self)
```

Using derived whole-value equality makes ticket and attempt ID jointly
authoritative. Optional current state directly models absence/revocation.

## Unit-test additions

Add focused tests inside the existing `types.rs` `#[cfg(test)] mod tests`.
They do not need new imports beyond the module's existing `use super::*`.

### Per-ticket monotonic sequence

Create initial leases for `T-A` and `T-B`, interleave successor minting, and
assert each ticket independently yields `[1, 2, 3]`. Assert cross-ticket numeric
reuse is allowed while complete leases remain unequal.

This test is the primary acceptance proof for strict per-ticket monotonicity.

### Prior-generation invalidation

Mint attempt 1, treat it as authoritative, then mint attempt 2 and make it the
authoritative reference. Assert:

- attempt 1 is initially current;
- attempt 1 is not current against attempt 2;
- attempt 2 is current against itself;
- attempt 1 is not current when authoritative state is absent.

This test is the primary stale-lease acceptance proof.

### Full-identity comparison

Construct or mint attempt 1 for two different tickets. Assert neither validates
against the other despite matching numeric attempt IDs.

### Invalid predecessor

Pass a T-A predecessor while requesting T-B and compare the returned error to
`TicketMismatch` with both identities. This test fixes fail-closed caller
behavior as part of the public contract.

### Exhaustion

Construct a valid-shaped predecessor at `u64::MAX`, request its successor, and
assert `AttemptIdExhausted`. This proves the helper does not saturate or wrap.

## Dependency boundary

No Cargo dependency changes are required. Serde is already a normal
`lisa-core` dependency with derive support. Tests need no new dev dependency.

## Compatibility boundary

The addition is source-compatible for all current consumers:

- no existing type changes;
- no enum variants added to exhaustive consumer matches;
- no serialized struct layouts change;
- no constructors change;
- no plugin behavior changes;
- no CLI behavior changes.

Later tickets can import `AttemptLease` from `lisa_core::types` and decide how
to store the predecessor/high-water mark and authoritative current lease.

## Implementation order

1. Add the error enum and its standard trait implementations.
2. Add the lease struct and associated helpers.
3. Add happy-path monotonicity and stale-current tests.
4. Add mismatch, absent-current, cross-ticket, and exhaustion tests.
5. Format and run focused `lisa-core` tests.
6. Run workspace-quality checks proportional to the shared-core change.
7. Commit only `crates/lisa-core/src/types.rs` through Lisa's isolated command.
8. Audit the ordinary index and ticket-owned source paths.

## Ownership boundary

The ticket owns only `crates/lisa-core/src/types.rs` as source. If an unrelated
change appears in that path before commit, the diff must be inspected and exact
hunks separated rather than sweeping it into the isolated transaction.

RDSPI artifacts are intentionally excluded from the source implementation
commit because Lisa handles them at final completion.
