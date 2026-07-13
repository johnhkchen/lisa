# Structure: attempt transition/failure provenance schema

## Change inventory

One ticket-owned source file changes:

`crates/lisa-core/src/provenance.rs`

No source files are created or deleted.
No plugin or CLI files change.
No scheduler behavior changes.
No public exports in `lisa-core/src/lib.rs` change because the provenance module
is already public and its new types are declared public within it.

Private RDSPI artifacts are created under:

`.lisa/attempts/T-040-02-01/1/work/`

Those artifacts are not included in the source commit; Lisa publishes them
through its completion transaction.

## Module-level organization

The module remains organized in this order:

1. module documentation;
2. imports;
3. schema version;
4. shared route and execution-outcome vocabulary;
5. new assignment-transition vocabulary;
6. existing terminal execution row;
7. new assignment-transition row;
8. mixed ledger row enum;
9. time and usage helpers;
10. append I/O;
11. unit tests.

Keeping both row structs adjacent makes the serialized alternatives visible
without mixing scheduler code into the schema module.

## Public constants

```rust
pub const SCHEMA_VERSION: u32 = 3;
```

This continues to mean the version stamped by current writers.
Deserialization does not require equality with the constant.

## New discriminator vocabulary

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProvenanceRecordType {
    AssignmentTransition,
}
```

The field is mandatory on the new row.
It provides an explicit JSON-level record kind while leaving legacy execution
objects untouched.

## New named-state vocabulary

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssignmentState {
    DeliveryFailed,
    RecoveryFailed,
    StartupFailed,
}
```

This is durable evidence vocabulary, not scheduler authority.
It intentionally contains no deadlines, generations, or retry counters.
Attempt generation is already represented by `AttemptLease`.

## New row type

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentTransitionRecord {
    pub schema_version: u32,
    pub record_type: ProvenanceRecordType,
    pub ticket_id: String,
    pub attempt_lease: AttemptLease,
    pub pane_id: u32,
    pub provider: AgentClient,
    pub state: AssignmentState,
    pub reason: String,
    pub started_at: u64,
    pub ended_at: u64,
    pub wall_clock_secs: u64,
}
```

Field ordering establishes readable compact JSON and groups identity before
state and timing.
All fields are required.
No serde defaults are added.
The new row can therefore only deserialize when the complete evidence shape is
present.

## Mixed-ledger reader type

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProvenanceLedgerRecord {
    AssignmentTransition(AssignmentTransitionRecord),
    Execution(ProvenanceRecord),
}
```

The assignment variant is attempted first and requires its explicit
`record_type`, `state`, and `reason` fields.
The execution variant retains the exact existing struct.
Serde's default tolerance for unknown fields does not create ambiguity because
each variant also has unique missing required fields when given the other
shape.

The wrapper provides the shared read contract needed by T-040-02-03.
It also provides a serialization path for either row without altering the
existing execution append API.

## Existing row boundary

`ProvenanceRecord` remains structurally unchanged.
No new required, defaulted, optional, or flattened fields are added.
Its field order and serde representation remain unchanged.
Existing construction in `lisa-plugin/src/lib.rs` remains source-compatible.

New execution records constructed with `SCHEMA_VERSION` will be stamped v3.
That is a version change, not a structural mutation of the Rust type.
Literal old rows retain their embedded v2 value when parsed.

## Append API boundary

`append_record(path, &ProvenanceRecord)` remains unchanged.
This prevents source churn in the current plugin writer.
T-040-02-02 may add a sibling append function, generalize serialization behind
an internal helper, or serialize the wrapper depending on its exact call-site
needs.
The new types do not preempt that implementation choice.

## Module documentation

The opening description changes from a ledger containing only one terminal
execution row per attempt to an append-only ledger containing lifecycle rows.
It distinguishes:

- `ProvenanceRecord` for terminal execution;
- `AssignmentTransitionRecord` for pre-ownership transitions/failures;
- `ProvenanceLedgerRecord` for mixed reads.

The scheduler-independent ownership statement remains.
References to plugin timing remain factual for execution rows and do not claim
the downstream writer is already wired.

## Unit-test fixtures

The existing `sample()` helper remains the execution fixture.
Its version follows `SCHEMA_VERSION`, so current-writer tests change their
literal assertion from 2 to 3.

A new `sample_assignment_transition()` helper returns a complete v3 row with:

- ticket `T-040-02-01`;
- a deterministic attempt lease;
- pane 12;
- Codex provider;
- `delivery-failed` state;
- a concrete acknowledgment-timeout reason;
- deterministic start/end/duration values.

A literal schema-v2 fixture is embedded as JSON text rather than produced by
the current `sample()` helper.
This prevents the compatibility test from accidentally tracking the new
constant and ceasing to test old bytes.

## New tests

`assignment_transition_serializes_to_one_compact_line`

Asserts no newline, exact schema/discriminator/provider/state values, all
identity and timing fields, and equality after direct round trip.

`mixed_ledger_reads_schema_v2_execution_and_schema_v3_assignment_transition`

Builds two JSONL lines: the literal v2 execution fixture and the serialized new
assignment fixture.
Parses each line as `ProvenanceLedgerRecord`.
Matches the first as `Execution` and compares it to a separately parsed
`ProvenanceRecord`.
Matches the second as `AssignmentTransition` and compares it to the sample.
This test simultaneously locks backward compatibility and mixed-ledger forward
recognition.

## Existing test adjustments

`record_serializes_to_one_compact_line` changes only its literal schema-version
expectation from 2 to 3.
All append, route, outcome, usage, time, and I/O failure tests remain unchanged.

## Verification boundary

The narrow verification command is:

`cargo test -p lisa-core provenance`

The repository verification command is:

`cargo test --workspace`

Formatting is checked with:

`cargo fmt --all -- --check`

Only `crates/lisa-core/src/provenance.rs` is passed to `lisa commit-ticket`.
