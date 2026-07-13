# Structure: attribute captures by pane time

## Change summary

Modify one ticket-owned source file:

- `crates/lisa-plugin/src/lib.rs`.

Do not create or delete source modules.

Do not change the core capture or provenance schemas.

Do not change the CLI capture writer.

Do not change `ownership::owner_at`.

The private workflow artifacts remain in this attempt directory and are not
part of the source transaction.

## Existing module dependencies

`lib.rs` already declares:

```rust
mod ownership;
```

The module exposes crate-visible `ownership::owner_at`.

`lib.rs` already imports core provenance types and functions.

The import set will expand to include `ProvenanceLedgerRecord`.

The production consumer will refer to the capture type through
`lisa_core::capture::CaptureRecord`.

No re-export is needed.

## Import change

Extend the existing provenance import group from:

```text
AssignmentState
AssignmentTransitionRecord
ProvenanceRecord
ProvenanceRecordType
Route
RunOutcome
```

to also include:

```text
ProvenanceLedgerRecord
```

The test may import `append_capture_record` and `CaptureRecord` locally to avoid
adding test-only names to production scope.

## `State::emit_provenance` boundary

Keep all current validation and base-record construction in place.

The current record remains fully populated with:

- ticket identity;
- attempt lease;
- outcome and authority;
- requested and actual routes;
- inclusive ownership start and end;
- concurrency;
- pane ID;
- null usage fields.

Change only the consumer invocation.

The old call is structurally:

```rust
self.read_usage(client, ticket_id)
```

The new call is structurally:

```rust
self.read_usage(client, &record)
```

Usage fields are still applied with record update syntax before append.

`provenance::append_record` remains the only terminal ledger write.

Error logging and boolean return behavior remain unchanged.

## `State::read_usage` signature

Retain the private method name but replace its identity input.

New shape:

```rust
fn read_usage(
    &self,
    client: AgentClient,
    current: &ProvenanceRecord,
) -> (Option<u64>, Option<u64>, Option<f64>)
```

`current.ticket_id`, `current.pane_id`, and the interval fields are the
authoritative current-run facts.

The method does not mutate plugin state.

It does not append provenance or capture rows.

It does not log or quarantine in this ticket.

## Provider capture path selection

Keep the existing `AgentClient` match:

```text
Codex  -> self.codex_dir
Claude -> self.claude_dir
```

Replace the ticket-derived filename with the fixed append log name:

```text
captures.jsonl
```

The resulting paths mirror the CLI writer exactly.

No fallback to `<ticket>.usage.json` remains.

## Durable ownership history load

Read `self.ledger_path` as text.

An unreadable or absent ledger produces an empty prior-record vector.

Split the text into lines.

Deserialize each line as `ProvenanceLedgerRecord`.

Retain only `ProvenanceLedgerRecord::Execution(record)` values.

Discard assignment-transition rows from ownership evidence.

Skip malformed rows.

The vector owns its records for the duration of capture attribution.

The current record is not inserted or cloned into this vector.

It is chained as one additional borrowed item at lookup time.

## Capture ledger load

Read the selected `captures.jsonl` as text.

An unreadable or absent capture file returns `(None, None, None)`.

Split the text into lines.

Deserialize each independently as `CaptureRecord`.

Skip malformed rows.

Process valid rows as a stream; no persistent collection is required.

First filter to `capture.pane_id == current.pane_id`.

This realizes the ticket phrase "loads a pane's CaptureRecords" and avoids
calling ownership lookup for unrelated physical panes.

## Per-capture attribution

For every current-pane capture, build an ownership iterator from:

```text
prior_records.iter()
    chained with
std::iter::once(current)
```

Call `ownership::owner_at` with the capture pane and timestamp.

The capture contributes only if the returned ticket string equals
`current.ticket_id`.

Captures owned by prior or other tickets are not included.

Captures with no unique owner are not included.

The implementation creates no fallback owner.

## Aggregation representation

Use an optional accumulator:

```text
Option<(u64 input, u64 output)>
```

The initial value is `None`, representing no attributed measurement.

The first matching capture creates a concrete pair.

Later matches use `checked_add` on both values.

Any overflow terminates aggregation as failure and yields absent usage.

On success:

```text
Some((input, output)) -> (Some(input), Some(output), None)
None                  -> (None, None, None)
```

The third element is always `None` because the input schema has no cost.

## Test location

Add the regression in the existing `#[cfg(test)]` module of
`crates/lisa-plugin/src/lib.rs`.

Place it beside the provenance usage-flow tests.

Use existing helpers:

- `with_ledger`;
- `read_ledger`;
- `install_current_attempt`.

Use shared core capture construction and append rather than hand-formatting
capture JSON.

## Regression fixture

Create a temporary state using `codex_state_with_dag` and `with_ledger`.

Use `state.codex_dir.join("captures.jsonl")` as the source ledger.

Choose one fixed pane, such as pane 7.

Use a fixed epoch base safely before `SystemTime::now()`.

Create ticket A's thread with:

- ticket ID `T-CDX-01`;
- pane 7;
- client Codex;
- `started_at` equal to the A interval start.

Append two A capture rows within that interval.

Emit A.

The real `emit_provenance` end time closes A's ownership window.

Remove A's thread while keeping its ledger row.

Create ticket B's thread with:

- ticket ID `T-CDX-02`;
- the same pane 7;
- client Codex;
- a later `started_at`.

Append two B captures at or after B's start.

Emit B.

The prior A record and current B record form disjoint ownership evidence.

## Regression assertions

After A emission, assert one provenance row exists and carries A's expected
input/output sum.

After B emission, assert exactly two rows exist.

Assert row zero still has:

- A's ticket ID;
- A's original token sum.

Assert row one has:

- B's ticket ID;
- B's token sum.

Use intentionally different totals so cross-attribution is visible.

The row-count and row-zero assertions prove later recycling did not overwrite
the earlier record.

## Documentation updates in source

Replace the legacy `read_usage` doc comment.

Document:

- fixed provider capture-ledger paths;
- pane-time attribution;
- use of prior plus current ownership intervals;
- per-ticket summation;
- missing/unparseable facts producing null usage;
- lack of capture cost data.

Remove claims about the old nested `{ usage: ... }` artifact.

Update stale field comments on `codex_dir` and `claude_dir` so they describe
append-only capture ledgers rather than ticket-keyed artifacts.

## Commit unit

The meaningful source unit is the consumer, call-site wiring, documentation,
and its focused regression in one file.

Commit it through:

```text
lisa commit-ticket
  --ticket-id T-043-03-01
  --message "fix(plugin): attribute captures by pane time"
  --include crates/lisa-plugin/src/lib.rs
```

No ordinary Git index operation is part of the structure.

## Deliberately unchanged boundaries

The change does not alter:

- capture production;
- `CaptureRecord` fields;
- provenance schema version;
- provenance row shape;
- ownership interval semantics;
- scheduler thread lifecycle;
- attempt leases;
- terminal append ordering;
- assignment-transition evidence;
- activity event vocabulary;
- quarantine storage;
- dashboard rendering;
- ticket frontmatter.

This structure leaves a narrow extension point for `T-043-03-02`: the consumer
already observes syntactically valid captures for which `owner_at` returns no
unique ticket, and that later change can route those rows into session-keyed
quarantine without changing the capture writer or ownership lookup.
