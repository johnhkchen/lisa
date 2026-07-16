# Structure — T-048-01-01 structured-block-schema

## Change inventory

Ticket-owned source changes are limited to:

- `crates/lisa-core/src/disposition.rs`;
- `crates/lisa-core/src/provenance.rs`;
- `crates/lisa-core/src/completion.rs`;
- `crates/lisa-core/tests/completion_state_machine.rs`;
- `crates/lisa-plugin/src/lib.rs`.

No source files are created or deleted.

The private attempt artifacts are:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`;
- `review-disposition.json`.

Lisa publishes admitted artifacts later. This attempt does not write them to
`docs/active/work/T-048-01-01/` and does not edit ticket phase or status.

## Module boundary: disposition

File: `crates/lisa-core/src/disposition.rs`.

This remains the sole owner of Review disposition JSON parsing and semantic
validation.

### New public enum

```rust
pub enum RemedyOwner {
    Agent,
    Operator,
    World,
}
```

Traits:

- `Debug`;
- `Clone`;
- `Copy`;
- `PartialEq`;
- `Eq`;
- `Serialize`;
- `Deserialize`.

Serde representation uses lowercase variant names. This is both the wire
format accepted in disposition JSON and the value serialized in provenance.

The enum stays in `disposition` because ownership is authored and validated as
part of the block contract. Provenance imports the enum rather than defining a
parallel vocabulary.

### Extended public enum variant

`ReviewDisposition::Block` changes from:

```rust
Block { reason: String }
```

to:

```rust
Block {
    reason: String,
    remedy_owner: RemedyOwner,
    ask: String,
    steps: Option<Vec<String>>,
    check: Option<String>,
    unstructured: bool,
}
```

`ReviewDisposition` retains its current derives. All added field types support
clone and full equality.

`reason` remains first and continues to mean the agent's original engineering
reason. `ask` is separately preserved for future human-facing surfaces.

### Parser flow

`parse_review_disposition` keeps its filesystem and JSON error flow unchanged.

`validate_document` still establishes object shape, extracts disposition and
reason, and matches pass/block/unknown relationships.

The block arm delegates structural parsing to a new private helper after
confirming the raw reason is a non-empty string.

Conceptual helper boundary:

```rust
fn block_disposition(
    reason: String,
    object: &mut Map<String, Value>,
) -> ReviewDisposition
```

The helper removes or reads:

- `remedy_owner`;
- `ask`;
- `steps`;
- `check`.

It attempts to construct a complete structured tuple. Any failure returns the
fallback through a second small helper.

Conceptual fallback boundary:

```rust
fn unstructured_block(reason: String) -> ReviewDisposition
```

This centralizes the exact defaults:

- `RemedyOwner::Operator`;
- `ask = reason.clone()`;
- `steps = None`;
- `check = None`;
- `unstructured = true`.

The structured constructor stores the original reason, parsed fields, and
`unstructured = false`.

### Validation helpers

A private string helper may validate non-whitespace JSON strings without
trimming returned contents.

Steps validation remains local to structural parsing:

- require a JSON array when present;
- require every element to be a non-whitespace string;
- collect without modifying element bytes.

There is no shell helper, process helper, or check evaluator in this file.

### Disposition test organization

Keep unit tests in the existing `#[cfg(test)]` module.

Retain `parse_document` for normal cases.

Add a helper that destructures a block or compare full variants explicitly.
Full equality comparisons document every semantic default.

Group cases by behavior:

1. pass and outer fail-closed cases;
2. fully structured owners and optional fields;
3. legacy fallback;
4. malformed-structure fallback matrix;
5. inert-check regression.

The inert-check regression creates a temporary directory and a sentinel path,
embeds a shell-looking write command as a JSON string using `serde_json::json!`,
writes only the JSON document, parses it, and checks that the sentinel does not
exist.

## Module boundary: provenance

File: `crates/lisa-core/src/provenance.rs`.

This remains the sole owner of ledger schema, serde representation, and append
I/O.

### Schema version

Change:

```rust
pub const SCHEMA_VERSION: u32 = 4;
```

Historical fixtures keep their explicit old versions. Sample records created
through the constant become schema 4.

### Import relationship

Add:

```rust
use crate::disposition::RemedyOwner;
```

This dependency is one-way. `disposition` does not import provenance.

`lib.rs` already publicly exposes both modules, so no re-export change is
required.

### New public transition discriminator

```rust
pub enum ParkingTransitionType {
    Park,
    Unpark,
}
```

Derives:

- `Debug`;
- `Clone`;
- `Copy`;
- `PartialEq`;
- `Eq`;
- `Serialize`;
- `Deserialize`.

Serde names are kebab-case, yielding `park` and `unpark`.

### New public record

```rust
pub struct ParkingTransitionRecord {
    pub schema_version: u32,
    pub record_type: ParkingTransitionType,
    pub ticket_id: String,
    pub attempt_lease: AttemptLease,
    pub remedy_owner: RemedyOwner,
    pub started_at: u64,
    pub ended_at: u64,
    pub wall_clock_secs: u64,
}
```

The record derives debug, clone, full equality, serialize, and deserialize.

It intentionally has no pane, provider, reason, ask, steps, or check. The
ticket's provenance acceptance requires identity, owner, and time. Structured
block content remains board state for the scheduler ticket; duplicating it into
every ledger row would enlarge the durable schema without a current query need.

The top-level ticket ID and attempt lease follow existing provenance
attribution conventions. Producers are responsible for keeping their ticket
IDs consistent, as with execution and assignment records.

### Ledger enum

Extend:

```rust
pub enum ProvenanceLedgerRecord {
    AssignmentTransition(AssignmentTransitionRecord),
    ParkingTransition(ParkingTransitionRecord),
    Execution(ProvenanceRecord),
}
```

Keep `#[serde(untagged)]`.

The discriminated transition variants precede the undiscriminated historical
execution shape. Their different `record_type` enum domains prevent one
transition shape from parsing as the other.

### Append API

Add:

```rust
pub fn append_parking_transition_record(
    path: &Path,
    record: &ParkingTransitionRecord,
) -> std::io::Result<()>
```

It delegates directly to `append_serialized`.

No changes are made to append flags, directory creation, newline framing, or
error conversion.

### Provenance tests

Add a `sample_parking_transition(record_type)` fixture with stable values:

- ticket and matching lease;
- operator owner by default;
- start/end and derived duration.

Add compact serialization and round-trip assertions for both transition types.

Add append coverage that writes park then unpark to one temporary ledger,
checks two newline-terminated rows, and parses both through
`ProvenanceLedgerRecord`.

Expand mixed replay to include:

- the existing schema-v2 execution constant;
- a schema-v3 assignment transition whose fixture version is explicitly set;
- a schema-v4 park row;
- a schema-v4 unpark row.

Assertions cover variant selection, attempt ID, remedy owner, timestamps, and
wall-clock duration.

Update current-version string assertions from 3 to 4 for records constructed
with `SCHEMA_VERSION`.

## Consumer compatibility updates

### Core completion tests

Files:

- `crates/lisa-core/src/completion.rs`;
- `crates/lisa-core/tests/completion_state_machine.rs`.

Replace direct one-field Block fixtures with the complete legacy-fallback
semantic shape. Use operator owner, raw reason as ask, no steps/check, and
`unstructured: true`.

Production completion logic remains unchanged because it checks only for exact
`Pass`.

### Plugin reason matches

File: `crates/lisa-plugin/src/lib.rs`.

Change direct matches from:

```rust
ReviewDisposition::Block { reason }
```

to:

```rust
ReviewDisposition::Block { reason, .. }
```

This preserves existing completion rejection and attention text. Do not render
ask, branch on owner, park, retry, or execute a check in this ticket.

### Plugin ledger matches

Add `ProvenanceLedgerRecord::ParkingTransition(_)` arms to exhaustive matches.

Execution-only readers return `None` for parking rows. Tests that intentionally
expect an execution row keep explicit destructuring with a failure branch.
Assignment-only readers ignore parking rows unless the compiler already accepts
an `if let` without exhaustiveness concerns.

No plugin append call is added. T-048-01-02 will import and emit the new record.

## Implementation ordering

1. Extend disposition types and parser.
2. Add disposition regression tests.
3. Update direct Block constructors and matches so the workspace compiles.
4. Extend provenance types, ledger enum, and append API.
5. Add provenance serialization, append, and replay tests.
6. Update exhaustive plugin ledger matches.
7. Format and run focused tests.
8. Commit the disposition unit through Lisa with exact paths.
9. Commit the provenance unit through Lisa with exact paths.
10. Run workspace verification and produce Review artifacts.

The commit split follows semantic ownership: the disposition contract and its
necessary consumers form one buildable unit; the new provenance schema and its
reader compatibility changes form the second.

## Explicit non-changes

- no scheduler retry bound;
- no ticket parking or status mutation;
- no DAG changes;
- no provenance emission from the plugin;
- no check execution;
- no check read-only validation;
- no operator unblock command;
- no status or dashboard wording;
- no Review template guidance;
- no ticket phase/status edits;
- no publication to shared work artifacts.
