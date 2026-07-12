# Structure: T-034-02-04 one authoritative provenance record

## Change inventory

### Modify `crates/lisa-core/src/provenance.rs`

- import `AttemptLease` from `lisa_core::types`;
- increment `SCHEMA_VERSION` from 1 to 2;
- extend `ProvenanceRecord` with required attempt and publication fields;
- document the exact semantics of each field;
- update the core sample fixture;
- extend serialization assertions;
- retain route, usage extraction, timestamps, and append helper unchanged.

No new core module is required.

### Modify `crates/lisa-plugin/src/lib.rs`

- extend `State::emit_provenance` with a fence-state argument;
- require the active thread to carry an attempt lease;
- reject Done when the thread lease is not current;
- populate schema-v2 fields;
- return publication success for focused assertions;
- classify fence outcomes through one helper or local match;
- reorder timeout/stale teardown to record actual fence state;
- pass `false` for non-fencing error failures and normal Done;
- exclude pending completion tickets from timeout/stale candidates;
- revalidate pending authority in `handle_completion_result`;
- update provenance fixtures to use real current leases;
- add the ticket acceptance regression.

No scheduler state field is added.

### Modify `docs/knowledge/provenance-ledger.md`

- change the documented schema version to 2;
- update the JSON example;
- add `attempt_lease`, `fenced`, and `authoritative` to the field table;
- document attempt-history versus ticket-authoritative semantics;
- mention schema-v1 historical rows;
- add or update a query for authoritative Done.

### Create work artifacts

- `docs/active/work/T-034-02-04/research.md`;
- `docs/active/work/T-034-02-04/design.md`;
- `docs/active/work/T-034-02-04/structure.md`;
- `docs/active/work/T-034-02-04/plan.md`;
- `docs/active/work/T-034-02-04/progress.md`;
- `docs/active/work/T-034-02-04/review.md`.

Lisa owns the final artifact commit; source changes use `commit-ticket`.

## Core public shape

The record remains public and serde-enabled:

```rust
pub struct ProvenanceRecord {
    pub schema_version: u32,
    pub ticket_id: String,
    pub attempt_lease: AttemptLease,
    pub outcome: RunOutcome,
    pub authoritative: bool,
    pub fenced: bool,
    // existing route, timing, usage, concurrency, and pane fields
}
```

Field order groups identity and terminal semantics before metrics.

`AttemptLease` already derives clone, equality, serialize, and deserialize.

No optional/default serde annotation is added to the new fields.

This makes the latest Rust type an exact schema-v2 representation.

## Scheduler internal interface

The publisher becomes conceptually:

```rust
fn emit_provenance(
    &mut self,
    ticket_id: &str,
    outcome: RunOutcome,
    fenced: bool,
) -> bool
```

`bool` means a record was appended successfully.

Unset ledger path remains a successful no-op only for unrelated native tests,
or returns false consistently; callers do not branch on it in production.

Missing thread, missing lease, stale Done, and append failure return false.

The publisher derives `authoritative` rather than accepting it from callers:

```text
authoritative = outcome == Done && attempt lease is current
```

This prevents callers from marking failed history authoritative.

## Completion result boundary

Add an internal predicate over `PendingCompletion::authority`:

```text
Attempt(lease) => lease.is_current(current_leases[ticket])
Operator       => source is Manual and no execution thread is required
```

The predicate runs immediately after cloning pending state and before processing
command success as a scheduler publication.

On rejection:

- remove the pending entry;
- rebuild the DAG without its mask;
- log a warning naming ticket and stale authority;
- return without other lifecycle changes.

Normal failure and durable-Done verification retain their current structure.

## Pending completion boundary

`check_session_timeouts` candidate filters gain:

```text
ticket not present in pending_completions
```

`detect_stale_threads` gains the same filter.

The filters operate during candidate collection, before mutation.

Review timeout prompting and ordinary health display are unchanged.

The scheduler can continue to report a pending thread's health while refusing
to create a competing lease until the transaction resolves.

## Fence classification

`FenceOutcome::Fenced` and `FenceOutcome::AlreadyFenced` map to `true`.

`FenceOutcome::NoAssignedPane` maps to `false` because no physical fence was
confirmed, even though lease revocation still occurred.

Session timeout and hard-silence paths retain the returned value long enough to
pass that classification into provenance.

Error-signal failure does not invoke the fence method and passes `false`.

Completion passes `false`.

## Test organization

Core schema assertions remain in `provenance.rs::tests` beside the record owner.

Plugin ledger fixtures remain in the existing provenance test section near the
bottom of `lib.rs`.

Existing helper `install_current_attempt` supplies monotonically increasing
leases and stamps thread/slot where present.

The combined regression should use `with_ledger` and `read_ledger`.

The regression asserts values, not raw JSON formatting.

The timeout critical-section test belongs near completion/provenance tests even
if it calls `check_session_timeouts`, because its invariant is completion authority.

## Change ordering

1. Change the core schema and its unit fixtures.
2. Change the plugin publisher so compilation identifies all call sites.
3. Reorder fenced teardown and supply fence state.
4. Add pending timeout exclusion and result revalidation.
5. Repair direct-construction provenance tests with leases.
6. Add combined stale/predecessor/replacement coverage.
7. Update ledger documentation.
8. Format, run focused tests, then workspace verification.
9. Commit only the three source/documentation paths through Lisa.

## Ownership boundary

Ticket-owned committed paths are:

- `crates/lisa-core/src/provenance.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `docs/knowledge/provenance-ledger.md`.

The repository already contains unrelated modified and untracked files.

No unrelated path will be included in `lisa commit-ticket`.

The work directory artifacts remain for Lisa's phase/completion transaction.

## Deletions and compatibility

No files or public enum variants are deleted.

Schema-v1 JSON remains valid historical JSON but is not silently upgraded.

Existing jq queries that ignore new fields continue to work.

Rust consumers deserializing mixed versions must branch on `schema_version`.

The plugin itself only writes and never deserializes runtime ledger rows.

## Resulting lifecycle

```text
attempt N times out
  -> revoke and fence N
  -> append {lease: N, timed-out, fenced: true, authoritative: false}
  -> release and redispatch
attempt N+1 completes isolated commit
  -> revalidate N+1 is current
  -> verify durable Done
  -> append {lease: N+1, done, fenced: false, authoritative: true}
  -> release and schedule dependents
late N request/result
  -> rejected without provenance publication
```
