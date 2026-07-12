# T-034-01-01 Review — attempt lease core type

## Review outcome

The ticket is complete and its acceptance criterion is met.

`lisa-core` exposes a provider-neutral attempt lease with shared mint and
current-validation semantics. Successful minting is strictly monotonic for a
ticket, and a prior-generation lease cannot validate when its successor is
authoritative.

No critical issue requires human intervention before the dependent dispatch
integration ticket proceeds.

## Source commit

```text
1094e7b91f8b31ec729bf78721d85e34cdde6185
feat: add attempt lease core contract
```

The source commit was created through Lisa's isolated transaction using one
exact include path. It contains:

```text
crates/lisa-core/src/types.rs | 157 insertions
```

The ordinary Git index is empty, and the committed source path is clean.

## Files modified

### `crates/lisa-core/src/types.rs`

Added `AttemptLease` beside `TicketId` with two public fields:

```rust
pub ticket_id: TicketId
pub attempt_id: u64
```

The type derives clone, complete equality, hashing, and Serde traits. Complete
equality is important: per-ticket sequences may share numeric attempt IDs, so a
generation alone is never sufficient authority.

Added `AttemptLease::mint`. It creates attempt 1 when no predecessor exists and
checked-increments a same-ticket predecessor. The helper returns a typed error
instead of panicking, wrapping, or saturating when its invariant cannot be
preserved.

Added `AttemptLease::is_current`. It returns true only when a candidate exactly
equals an authoritative current lease. An absent authority rejects all leases,
which is compatible with the revocation behavior planned by `T-034-01-03`.

Added `AttemptLeaseMintError` with `TicketMismatch` and
`AttemptIdExhausted` variants plus standard error formatting.

Added five unit tests covering the required behavior and defensive boundaries.

## Files created

Created the six RDSPI handoff artifacts under
`docs/active/work/T-034-01-01/`:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

These artifacts are intentionally not in the ticket-owned source commit. Lisa
owns their isolated completion transaction and the ticket transition.

## Files deleted

None.

## Acceptance criterion evaluation

### `lisa-core` exposes `AttemptLease`

Met.

The public type is available as `lisa_core::types::AttemptLease`. No new module
or crate-root re-export is needed because `types` is already public.

### `mint` helper is exposed

Met.

`AttemptLease::mint(ticket_id, previous)` is public. The first minted attempt is
1; every successful successor is the checked increment of a predecessor for the
same ticket.

The helper refuses cross-ticket predecessors. This prevents a future scheduler
keying defect from silently copying one ticket's high-water mark into another
ticket's lease.

The helper also refuses a successor to `u64::MAX`. This is stronger than the
existing plugin assignment allocator, whose saturating increment can repeat
the maximum value. The new contract never reports success unless the returned
attempt is strictly greater.

### `is_current` helper is exposed

Met.

`AttemptLease::is_current(current)` is public and validates the whole lease. It
fails closed for missing current state, a stale generation, a future/different
generation, or another ticket with the same generation.

### Unit tests prove strict monotonicity per ticket

Met.

`attempt_lease_ids_are_strictly_monotonic_per_ticket` interleaves two ticket
sequences and observes `[1, 2, 3]` independently for both. This proves the
contract does not depend on a global cross-ticket sequence.

### Unit tests prove a prior generation is never current

Met.

`prior_attempt_lease_never_validates_as_current` first validates attempt 1,
mints attempt 2, then proves attempt 1 fails against the successor while
attempt 2 succeeds. It also proves absence/revocation rejects attempt 1.

## Test coverage

Passed focused lease coverage:

```text
cargo test -p lisa-core attempt_lease
5 passed; 0 failed
```

The five tests cover:

- interleaved strict monotonicity for two tickets;
- predecessor rejection after successor installation;
- exact current acceptance;
- missing-current rejection;
- cross-ticket rejection with equal numeric generations;
- cross-ticket predecessor mint failure;
- `u64::MAX` exhaustion failure.

Passed full core coverage:

```text
cargo test -p lisa-core
155 passed; 0 failed
```

Passed complete repository coverage:

```text
cargo test --workspace
693 passed; 0 failed
```

Passed deployed-target compatibility:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Passed formatting, library Clippy, and whitespace checks:

```text
cargo fmt --all -- --check
cargo clippy -p lisa-core --lib -- -D warnings
git diff --check
```

## Test coverage gap

The all-target Clippy invocation is not green because 12 existing `dag.rs` unit
test expressions trigger `clippy::unnecessary_to_owned` with warnings denied.
The warnings are outside the ticket-owned file and predate this change, so they
were not swept into this critical-path ticket. Library-target Clippy is green,
and all core test targets compile and pass normally.

There is no dedicated Serde round-trip test for `AttemptLease`. The derives are
compiled on both native and WASM targets, and no custom representation is
present. A representation-specific test can be added when a later ticket embeds
the lease in `Thread`, signals, or provenance and establishes a compatibility
schema.

## Compatibility assessment

The change is additive:

- no existing public type changed;
- no existing constructor changed;
- no enum gained a variant that consumers must match;
- no serialized record layout changed;
- no plugin scheduler behavior changed;
- no CLI behavior changed;
- no dependency changed.

The type uses the existing `TicketId = String` boundary and the existing `u64`
generation representation, so downstream migration can map Codex assignment
generation directly to `attempt_id`.

## Open concerns and deferred work

### Scheduler must retain a high-water predecessor

`AttemptLease` is intentionally a value rather than a mutable registry. After a
lease is revoked, the scheduler must retain the last minted lease or its
equivalent high-water state so redispatch calls `mint` with the predecessor and
does not restart at 1. This storage policy is the central responsibility of
`T-034-01-02` and `T-034-01-03`.

### Existing assignment generation is not migrated here

`lisa-plugin` still owns a global saturating Codex assignment generation. This
ticket does not change it. The dependent dispatch ticket must decide how the
attempt lease becomes the canonical generation without creating a second
competing identity.

### Deserialization can construct arbitrary values

Serde and public fields allow callers to construct attempt 0 or arbitrary
generations. Minting is the only API that promises ordering. Authority remains
safe only when validation compares a candidate to scheduler-owned current
state. This mirrors the project's public domain-struct style and should be
reviewed again when persistence is introduced.

### Restart durability remains unspecified

The core helper is process-agnostic. It does not define how attempt high-water
marks survive plugin restart. The current story describes scheduler-lifetime
state and later integration tickets must make any persistence assumption
explicit. Reusing a generation while stale events or processes can survive a
restart would violate the epic's intent.

### Fencing and rejection are not implemented yet

This ticket only supplies the shared identity contract. It does not by itself
prevent split-brain execution. The guarantee becomes operational only after:

- `T-034-01-02` mints and stamps every dispatch;
- `T-034-01-03` revokes and fences before reschedule;
- `S-034-02` gates acknowledgements, liveness, artifacts, completion, and
  provenance on exact current leases.

These are expected, explicit dependencies rather than defects in this ticket.

## Human review focus

A reviewer should confirm two API decisions before downstream code compounds
them:

1. `attempt_id` is the desired field name for the provider-neutral identity,
   with E-033's `generation` treated as its transport representation.
2. Scheduler-owned predecessor/high-water storage, rather than a core-owned
   lease registry, is the intended state boundary.

Both decisions are grounded in the current story decomposition and preserve the
narrow scope of this ticket. Neither blocks the acceptance criterion.

## Final assessment

The implementation is small, additive, fully tested at the core contract, and
safe for the dependent scheduler ticket to consume. Strict monotonicity and
stale-current rejection are encoded once rather than left to individual mint,
fence, or rejection sites.

No ticket frontmatter phase or status field was modified. Lisa can now perform
the artifact-driven phase transitions and completion transaction.
