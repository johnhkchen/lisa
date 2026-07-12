# Structure: T-034-02-02 gate completion on current lease

## Change inventory

Modify one source file:

- `crates/lisa-plugin/src/lib.rs`

Create workflow artifacts under:

- `docs/active/work/T-034-02-02/`

No source module, CLI, configuration, hook, or signal file is created or
deleted. No ticket frontmatter is edited by this work.

## `PendingCompletion`

Add a private authority enum and change the pending record from a Copy value to
a Clone value that retains validated authority:

```rust
enum CompletionAuthority {
    Attempt(AttemptLease),
    Operator,
}

#[derive(Debug, Clone)]
struct PendingCompletion {
    prior_phase: Phase,
    prior_status: TicketStatus,
    source: CompletionSource,
    authority: CompletionAuthority,
}
```

The field is non-optional because the record is created only after successful
attempt validation or an explicit manual operator request.

`handle_completion_result` changes its lookup from `.copied()` to `.cloned()`.
Its success and failure state transitions otherwise remain unchanged.

## Completion admission interface

Change the private method signature to:

```rust
fn request_completion(
    &mut self,
    ticket_id: TicketId,
    source: CompletionSource,
    authority: Option<CompletionAuthority>,
) -> bool
```

The `Option` centralizes rejection and logging for callers that cannot resolve
attempt identity. Operator authority is accepted only with Manual source.

At method entry, after the existing pending deduplication guard:

1. match `authority`;
2. compare Attempt with `current_leases.get(&ticket_id)` via `is_current`;
3. accept Operator only when the diagnostic source is Manual;
4. log a warning and return false on absence, mismatch, or invalid pairing;
5. retain accepted authority for pending-record construction.

No current-lease map mutation occurs in this method.

## Artifact completion caller

In `check_artifact_advances`, change the running snapshot from:

```rust
Vec<(TicketId, Phase)>
```

to:

```rust
Vec<(TicketId, Phase, Option<AttemptLease>)>
```

Clone `thread.attempt_lease` while building the snapshot. When Review advances
to Done, pass that snapshot value to `request_completion`.

Intermediate phase advancement does not consume the lease and remains
unchanged.

## Idle completion caller

In both idle paths that request Done, resolve the active thread's lease before
the mutable request call:

```rust
let source_lease = self
    .threads
    .get(&ticket_id)
    .and_then(|thread| thread.attempt_lease.clone());
```

Pass that value with `CompletionSource::Idle`.

Do not change idle filename parsing, signal deletion, intermediate phase
updates, or attention behavior.

## Stopped completion caller

Keep `CompletionSource::Stopped(pane_id)` for diagnostics.

In `auto_complete_review`, resolve a lease only from the slot whose:

- `pane_id` equals the stopped pane;
- `ticket_id` equals the requested ticket.

Clone that slot lease and pass it to `request_completion`.

This component boundary makes the physical event source explicit and avoids
borrowing conflicts in `handle_stopped_signal`.

## Manual completion caller

In `mark_ticket_done`, clone an existing thread's lease into Attempt authority.
If no thread exists, pass Operator authority so the existing manual recovery
control remains available.

The modal population logic is unchanged. An existing thread without an
authoritative lease reaches the central rejection log.

## Observed-Done caller

In `poll_tick`, change the Done snapshot from ticket IDs to pairs:

```rust
Vec<(TicketId, Option<AttemptLease>)>
```

Clone the iterated thread lease into each pair and pass it to
`request_completion`.

The pending mask, DAG rebuild, reconciliation, stale-slot sweep, and audit
ordering remain unchanged.

## Test support

Add a private test helper near the existing slot/state helpers:

```rust
fn install_current_attempt(state: &mut State, ticket_id: &str) -> AttemptLease
```

The helper:

1. mints a successor from `lease_high_water` when present;
2. inserts the same lease in `lease_high_water` and `current_leases`;
3. stamps a matching thread when present;
4. stamps a matching assigned slot when present;
5. returns the lease.

It mirrors production dispatch and avoids inconsistent test setup.

Use it only in fixtures that reach completion admission. Tests that exercise
non-completion phase changes or legacy state can remain unleased.

## New acceptance regression

Add a test near existing completion transaction tests:

```rust
request_completion_rejects_stale_attempt_and_accepts_current_lease
```

Fixture shape:

- one Review ticket in the DAG;
- one active Review thread and matching slot;
- prior lease minted for the ticket;
- successor installed as high-water/current and stamped on active records.

Assertions for the prior lease:

- request returns false;
- `pending_completions` has no ticket entry;
- current authority remains the successor;
- thread and slot remain assigned;
- warning identifies stale lease rejection.

Assertions for the successor:

- request returns true;
- pending entry exists;
- pending entry retains Attempt authority containing the successor lease;
- source remains the supplied diagnostic origin;
- ticket file remains Review until transaction preparation.

## Existing completion tests

Update fixtures that expect pending completion after:

- Review artifact detection;
- all-artifact catch-up;
- Implement-to-Review catch-up with pre-existing review artifact;
- idle Review completion;
- stopped Review completion;
- manual completion;
- observed external Done;
- verified success and failed/retry transaction cases.

The helper should be called after both thread and slot have been inserted so
one invocation stamps all representations.

Where a test creates no slot, the helper stamps the thread and both maps only.
That remains a valid logical completion fixture because source selection for
artifact/manual/observed paths uses the thread.

## Public interfaces

There are no public API changes.

- `CompletionSource`, `CompletionAuthority`, `PendingCompletion`, and `State`
  are plugin-private.
- No serialized type changes.
- No CLI arguments change.
- No hook payload changes.
- No command context keys change.
- No native transaction interface changes.

## State relationship

Before admission:

```text
event source lease ─────┐
                       ├─ exact equality ─> request accepted
current_leases[ticket] ─┘
```

After admission:

```text
PendingCompletion.authority = Attempt(current lease)
              │
              └─ validated source identity for the unchanged transaction
```

A stale or missing source terminates at the equality boundary and creates no
pending state.

## Ordering constraints

- Keep pending deduplication first so duplicate triggers remain silent and
  idempotent while a transaction is active.
- Perform lease validation before dependency and file checks.
- Install pending state only after all admission checks succeed.
- Keep command launch after pending insertion so synchronous observations see
  the ticket masked.
- Keep result verification and successful publication ordering exactly as it
  exists under T-031.

## Ownership and commits

The source implementation unit owns only:

- `crates/lisa-plugin/src/lib.rs`

Commit it through:

```text
lisa commit-ticket --ticket-id T-034-02-02 ... \
  --include crates/lisa-plugin/src/lib.rs
```

Workflow artifacts remain for Lisa's final ticket completion transaction. No
unrelated worktree path may be included.
