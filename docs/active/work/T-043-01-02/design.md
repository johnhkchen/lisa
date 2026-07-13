# Design: pane-time ownership lookup

## Goal

Provide a reusable plugin lookup that answers:

> Which ticket owned physical pane `pane_id` when a capture was observed at
> epoch-second `captured_at`?

The answer must come from scheduler-produced terminal attempt intervals, not
from inherited environment, the pane's current reservation, or ledger row
position.

This ticket settles the lookup contract and its regression test. It does not
load capture records, replace `read_usage`, sum tokens, or quarantine unmatched
captures.

## Design forces

The existing durable fact type is `lisa_core::provenance::ProvenanceRecord`.

The downstream consumer will need two sources of ownership intervals:

- execution records already appended to the provenance ledger;
- the current terminal execution record being assembled by
  `State::emit_provenance`, before that record itself is appended.

The lookup therefore benefits from accepting records supplied by its caller
rather than owning filesystem loading.

The same physical pane can appear in many records. Ticket identity can repeat
across attempts, and a pane can later be assigned to a different ticket.

Timestamps and capture times use the same epoch-second representation.

## Option 1: look up the current slot or thread only

This approach would add `State::owner_at` and scan `State::agent_slots` or
`State::threads`.

### Benefits

- Minimal code.
- No ledger parsing.
- Answers for a currently active attempt without constructing a terminal row.

### Costs

- A recycled pane's earlier owner disappears when its thread is removed.
- Slot `ticket_id` is a reservation/routing field during handoff, not durable
  historical ownership.
- Retained failed threads and pre-ack assignment states make thread presence an
  unsafe substitute for acknowledged ownership.
- It cannot satisfy the required A-then-B historical test once A is torn down.

### Decision

Rejected. It answers present scheduler topology rather than pane-time history.

## Option 2: add a second ownership timeline to `State`

This approach would introduce a vector or map of custom ownership intervals,
populate it at every lifecycle transition, and query it through a state method.

### Benefits

- Lookup could have the exact two-argument method shape on `State`.
- Active and completed intervals could be represented uniformly.
- The data structure could be indexed by pane.

### Costs

- It duplicates facts already written to provenance.
- Every teardown path would need correct new mutation ordering.
- Restart durability would require another persistence mechanism or replay.
- It broadens a contract ticket into a scheduler lifecycle rewrite.
- Two sources could disagree about start, end, pane, attempt, or ticket.

### Decision

Rejected. The story explicitly says to reuse the existing durable intervals, not
build another ownership store.

## Option 3: put the lookup in `lisa-core::provenance`

This approach would add a method or free function beside `ProvenanceRecord`.

### Benefits

- The operation is close to its data type.
- Other crates could reuse it.
- Core tests are fast and require no plugin setup.

### Costs

- The story assigns the lookup seam specifically to `lisa-plugin`.
- `T-043-01-01` concurrently owns the core capture schema, and the story promises
  disjoint crate/file boundaries between the foundation tickets.
- No current non-plugin consumer requires pane-time attribution.
- It would expand the public core API before another crate needs it.

### Decision

Rejected for this ticket. The operation can be promoted later if a genuine
cross-crate consumer appears.

## Option 4: plugin-local pure lookup over execution records

This approach creates a focused `ownership` module in `lisa-plugin`. It exposes
a crate-visible `owner_at` function accepting an iterable collection of borrowed
`ProvenanceRecord` values, a pane ID, and a capture timestamp.

### Benefits

- Uses the existing durable source of truth directly.
- Is independent of Zellij, mutable scheduler state, and filesystem behavior.
- Allows downstream code to chain persisted rows with the current provisional
  terminal row.
- Keeps `T-043-01-01` and `T-043-01-02` source ownership disjoint.
- Permits a deterministic unit test with explicit windows.
- Does not couple the contract to legacy usage artifact layout.

### Costs

- The caller remains responsible for parsing execution records from the mixed
  ledger.
- The free function technically takes the record collection in addition to the
  semantic `pane` and `captured_at` keys.
- A linear scan is O(number of execution records).

### Decision

Selected. The ledger is expected to be small enough that correctness and a
narrow seam matter more than adding an index. Loading and indexing can evolve
without changing the lookup semantics.

## Proposed interface

The plugin-local interface is conceptually:

```rust
pub(crate) fn owner_at<'a>(
    records: impl IntoIterator<Item = &'a ProvenanceRecord>,
    pane_id: u32,
    captured_at: u64,
) -> Option<&'a str>
```

An iterator input is preferable to a fixed slice because downstream attribution
can use `persisted.iter().chain(std::iter::once(&current_record))` without
cloning or temporarily appending the record.

The result borrows the ticket string from the matching record. The lookup does
not need to allocate a new `String`; callers can clone only if they need owned
storage.

The module is crate-visible rather than public outside `lisa-plugin`. There is
no external API requirement in the ticket.

## Match semantics

A record matches when all of the following hold:

- `record.pane_id == pane_id`;
- `record.started_at <= captured_at`;
- `captured_at <= record.ended_at`.

Both endpoints are inclusive.

Inclusive endpoints match the factual timestamp representation: attempt start,
attempt end, and capture observation all lose subsecond precision when converted
to epoch seconds. Excluding an endpoint would create a false gap for a capture
recorded during the same second.

The normal recycled-pane case has non-overlapping windows. A capture between or
outside them matches neither and returns `None`.

## Ambiguity semantics

The lookup scans all supplied records rather than returning the first physical
match.

If every match names the same ticket, it returns that ticket. This treats an
append retry or duplicate interval for one ticket as the same ownership answer.

If matching records name different tickets, it returns `None`.

There is no honest priority rule for conflicting ownership facts. Selecting the
first or last row would make attribution depend on input ordering and could blend
usage into the wrong ticket. Returning `None` lets the later quarantine ticket
surface the inconsistency.

An invalid interval with `ended_at < started_at` cannot satisfy both inclusive
comparisons and is ignored naturally.

## Record-kind boundary

The lookup accepts `ProvenanceRecord`, not `ProvenanceLedgerRecord`.

This type boundary prevents assignment-transition rows from becoming owners.
Callers parsing the mixed ledger must extract only `Execution` variants before
invoking the lookup.

That separation follows the schema documentation: assignment transition rows
ended before provider ownership.

## Ordering and performance

The algorithm performs one linear scan and holds at most one candidate ticket
reference.

It does not sort, allocate, build a pane map, or assume append order. Complexity
is O(n) time and O(1) additional space.

This is appropriate for the contract layer. If capture attribution later scans a
large ledger repeatedly, its loader may group records per pane or process
captures in batches while preserving the same `owner_at` behavior.

## Test design

The primary plugin unit test constructs two terminal execution records:

- ticket A on pane 7 from `t0=100` through `t1=199`;
- ticket B on pane 7 from `t2=300` through `t3=399`.

It asserts:

- a timestamp within A's window returns A;
- a timestamp within B's window returns B;
- a timestamp before both returns `None`;
- a timestamp between the windows returns `None`;
- a timestamp after both returns `None`.

The test also checks the inclusive endpoints because timestamp precision makes
that boundary part of the contract.

A small ambiguity test can assert that different-ticket overlap returns `None`
and same-ticket duplicate evidence still returns the ticket. This is beyond the
minimum acceptance criterion but protects the fail-closed policy.

## Rejected scope

The implementation will not:

- edit `ProvenanceRecord` or bump its schema version;
- parse `.lisa/provenance.jsonl`;
- add capture record types;
- change `State::read_usage`;
- change provenance emission ordering;
- sum token counts;
- write quarantine files;
- add operator activity events;
- index the ledger;
- infer ownership from leases, environment, session ID, or current slot state.

## Decision summary

Add a small `lisa-plugin` ownership module containing an iterator-based,
inclusive, ordering-independent `owner_at` lookup over terminal execution
records. Return the unique matching ticket, preserve `None` for gaps, and fail
closed to `None` when different tickets overlap. Prove pane recycling and the
boundary semantics with native unit tests colocated with the module.
