# Design: attribute captures by pane time

## Decision summary

Replace the ticket-keyed JSON reader with a provider capture-ledger consumer.

At terminal emission, pass the fully formed in-memory `ProvenanceRecord` into
the consumer.

Load prior execution provenance from the durable mixed ledger.

Treat the current in-memory record as the final ownership interval in that
lookup set.

Load `.lisa/<client>/captures.jsonl`, select captures for the current pane,
attribute each through `ownership::owner_at`, retain only captures owned by the
current ticket, and sum their input/output counts.

Return optional token totals and no cost.

## Goals

The design must:

- stop depending on `<ticket>.usage.json`;
- use capture pane and time rather than inherited ticket identity;
- recognize both already-finished and currently-finishing ownership windows;
- assign only uniquely attributable captures;
- sum all captures owned by the current ticket;
- preserve null usage when no capture is attributable;
- leave prior provenance rows byte-intact;
- support both provider directories with one consumer;
- keep quarantine behavior out of this ticket;
- prove sequential A-then-B reuse of one physical pane.

## Option 1: attribute during `emit_provenance` with prior plus current intervals

This option changes `read_usage` to receive the current execution record rather
than only a client and ticket ID.

The consumer reads capture and provenance JSONL files on demand.

It deserializes terminal execution records from the mixed ledger and appends a
borrow of the not-yet-durable current record to the ownership iterator.

For each current-pane capture it calls:

```text
owner_at(prior execution records + current record, pane_id, captured_at)
```

Only an answer equal to the current record's ticket ID contributes tokens.

### Advantages

- Uses the existing `owner_at` contract directly.
- Fits the existing write-after teardown ordering.
- Requires no new scheduler state.
- Uses durable history for prior pane occupants.
- Includes the current occupant without appending a partial row.
- Keeps terminal provenance append-only.
- Gives the next quarantine ticket a single consumer seam to extend.

### Costs

- Reads the provenance ledger for each terminal emission.
- Reads the provider capture ledger for each terminal emission.
- Leaves unmatched rows unfiled until the next ticket adds quarantine.
- Requires clear malformed-row behavior inside the plugin.

## Option 2: append the current provenance row, then update its usage

This option would append a null-usage current record first so all ownership
windows are durable, run attribution, and then fill its usage.

### Advantages

- Ownership lookup would read only durable rows.
- No special current-record iterator element would be needed.

### Rejection

The provenance ledger is append-only.

Updating the just-written record would rewrite history or require a second row
shape that patches the first.

Neither behavior exists in the schema.

Appending a second full execution row would duplicate terminal evidence and
complicate authoritative outcome queries.

This conflicts directly with the acceptance requirement that later recycling
must not overwrite prior records.

## Option 3: use active `State::threads` as the ownership history

This option would inspect active threads rather than reading provenance.

### Advantages

- Avoids reading the provenance ledger.
- Active thread data already contains pane and start time.

### Rejection

Ticket A is removed from `State::threads` before pane B completes.

The acceptance case specifically requires attribution across pane reuse.

Active memory cannot reconstruct prior ownership windows after teardown or
restart.

The existing ownership foundation deliberately treats terminal provenance as
the durable interval history.

## Option 4: consume captures by session ID

This option would associate provider sessions with tickets and group capture
rows by `session_id`.

### Advantages

- Session IDs are already present on `CaptureRecord`.
- It could naturally group multiple rows from the same provider session.

### Rejection

No scheduler-owned durable session-to-ticket mapping exists.

The ticket explicitly requires `owner_at` over pane-time ownership.

Session ID is included for later session-keyed quarantine, not as current
ownership authority.

Adding a new mapping would duplicate the existing provenance timeline.

## Option 5: precompute a global ticket-to-usage map

This option would scan all captures once, attribute them all, then look up the
current ticket's total.

### Advantages

- Makes the global join explicit.
- Could amortize work if cached across many terminal emissions.

### Rejection

A cache would need invalidation whenever capture or provenance rows append.

The plugin has no file-change version for either ledger.

The current ticket only needs one current record's totals at teardown.

Global mutable aggregation also risks introducing a shared or last-owner bucket
that the parent story explicitly rejects.

## Ownership evidence set

The consumer will use only `ProvenanceRecord` execution intervals.

`AssignmentTransitionRecord` rows are pre-ownership evidence and must not own a
capture.

The durable portion is parsed through `ProvenanceLedgerRecord` so mixed ledgers
remain supported.

Execution variants are collected in file order.

The current in-memory record is chained after them for lookup.

`owner_at` itself does not depend on ordering for unique or conflicting owners.

## Capture selection

The provider directory comes from `record.actual.method` indirectly through the
existing `AgentClient` argument.

The path is always `captures.jsonl` under `codex_dir` or `claude_dir`.

The consumer first filters by `capture.pane_id == current.pane_id`.

It then resolves ownership at `capture.captured_at`.

It includes the capture only when the unique owner equals
`current.ticket_id`.

A unique owner for another ticket is ignored for the current record.

No owner or conflicting ownership is also ignored in this ticket.

The next ticket will replace that final ignore boundary with session-keyed
quarantine and a visible event.

## Summation semantics

Every included `CaptureRecord` contributes its complete `input_tokens` and
`output_tokens` values.

The acceptance language calls for summed tokens across the ticket's capture
records.

Aggregation will use `checked_add` through `try_fold`.

If either sum overflows `u64`, the read fails closed to absent usage rather than
wrapping or fabricating a smaller total.

If at least one capture is included and both sums fit, both token options are
`Some`, including a measured zero on one side.

If no capture is included, both are `None`.

`cost_usd` remains `None` because `CaptureRecord` contains no observed cost.

## Parse and I/O behavior

A missing or unreadable capture ledger yields no usage.

A missing provenance ledger is valid for the first terminal record; the current
in-memory interval can still attribute its captures.

JSONL rows will be parsed independently.

Malformed capture rows cannot supply reliable facts and are skipped.

Malformed provenance rows cannot supply ownership intervals and are skipped.

This matches the producer's defensive external-transcript behavior and keeps a
single corrupt line from hiding later valid append-only rows.

Quarantine of syntactically valid but unattributable capture rows remains the
next ticket's responsibility.

## API shape

Keep the consumer as a private `State` method:

```rust
fn read_usage(
    &self,
    client: AgentClient,
    current: &ProvenanceRecord,
) -> (Option<u64>, Option<u64>, Option<f64>)
```

The method name remains local and minimizes call-site churn.

The `ticket_id` parameter disappears because identity is carried by the current
record being populated.

No new public API or core schema is required.

## Test design

Add one plugin regression test beside existing provenance tests.

Use a temporary ledger and provider directory.

Write multiple `CaptureRecord` JSONL rows through the shared core append helper.

Use one physical pane with deterministic ownership windows:

- ticket A owns pane 7 for an earlier interval;
- ticket B owns pane 7 for a later interval.

Give A at least two captures so summation is observable.

Give B at least two captures with distinct totals.

Emit A first and assert its row contains only A's sum.

Replace the active thread with B on the same pane, emit B, and assert:

- two provenance rows exist;
- the first row still contains A's identity and sum;
- the second row contains B's identity and sum;
- neither row contains the other ticket's captures.

The test will use explicit past `SystemTime` starts and capture epoch seconds so
the intervals are stable without sleeps.

## Chosen design

Adopt Option 1.

It is the only option that combines durable prior ownership with the exact
currently-closing interval while preserving the existing append-only terminal
record shape.

It directly composes the three predecessor contracts and confines the change to
the plugin consumer and its regression evidence.
