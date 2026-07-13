# Structure: pane-time ownership lookup

## Change summary

The implementation adds one focused plugin module and registers it from the
plugin crate root.

No core schema, CLI writer, hook, provenance writer, usage reader, ticket, or
shared work artifact changes belong to this ticket.

## Files created

### `crates/lisa-plugin/src/ownership.rs`

This new module owns pane-time selection semantics over durable execution
provenance intervals.

It imports `lisa_core::provenance::ProvenanceRecord` and defines the crate-visible
lookup:

```rust
pub(crate) fn owner_at<'a>(
    records: impl IntoIterator<Item = &'a ProvenanceRecord>,
    pane_id: u32,
    captured_at: u64,
) -> Option<&'a str>
```

The module contains no filesystem access and no scheduler mutation.

Its private test module constructs representative `ProvenanceRecord` fixtures
and verifies lookup behavior.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

Add a private module declaration:

```rust
mod ownership;
```

No existing `State` fields or methods change in this ticket.

The declaration makes `ownership::owner_at` available to future plugin
attribution work while leaving it outside the crate's external API.

## Files not modified

### `crates/lisa-core/src/provenance.rs`

`ProvenanceRecord` already contains the complete interval facts. The schema and
append behavior remain unchanged.

### `crates/lisa-core/src/types.rs`

`Thread` remains the scheduler's active-attempt source. No second interval type
or timeline is added.

### `crates/lisa-plugin/src/lib.rs` usage/provenance methods

`State::read_usage` continues reading the legacy per-ticket JSON artifact.
`State::emit_provenance` continues constructing the record, reading usage, and
appending the row in its current order.

`T-043-03-01` will integrate capture loading and call the lookup.

### CLI and hook files

`crates/lisa-cli/src/capture_usage.rs`, templates, and live hook files are owned
by `S-043-02`; this ticket does not touch them.

### Ticket frontmatter

Lisa owns phase/status transitions. Existing working-tree changes in
`docs/active/tickets/T-043-01-01.md` and `T-043-01-02.md` remain outside the
source transaction.

## Module boundary

The new module has one responsibility: map a pane-time key to a unique ticket in
a caller-supplied collection of terminal execution records.

Inputs are already-typed records. Parsing a mixed ledger belongs to a caller or
future loader.

Output is an optional borrowed string. Allocation and downstream aggregation
belong to the consumer.

This boundary deliberately excludes:

- capture record shape;
- capture session identity;
- token aggregation;
- quarantine persistence;
- activity logging;
- ledger I/O;
- current slot or lease lookup;
- provenance mutation.

## Internal organization of `ownership.rs`

The source file is ordered as follows:

1. module-level documentation describing pane-time attribution and the durable
   record source;
2. the `ProvenanceRecord` import;
3. documentation for `owner_at`;
4. the lookup implementation;
5. a `#[cfg(test)]` module;
6. a compact fixture constructor;
7. the recycled-pane acceptance test;
8. an ambiguity/duplicate-evidence policy test.

## Lookup algorithm

Initialize an empty optional candidate ticket reference.

For each supplied execution record:

1. skip it if the pane differs;
2. skip it if capture time precedes `started_at`;
3. skip it if capture time follows `ended_at`;
4. if no candidate exists, retain this ticket;
5. if the existing candidate equals this ticket, continue;
6. if it differs, return `None` immediately.

After the scan, return the optional candidate.

This is a unique-owner reduction, not a first-match search.

## Interval contract

The owned window is closed and inclusive:

```text
started_at <= captured_at <= ended_at
```

This matches the schema's epoch-second resolution. The new module documents the
choice so the downstream capture consumer does not invent separate boundary
semantics.

## Ambiguity contract

Zero matching intervals means no owner.

One matching ticket identity means that ticket is the owner.

Multiple matching rows naming the same ticket still mean that ticket. This
supports append retries and duplicate evidence without false quarantine.

Multiple matching rows naming different tickets mean no confident owner. The
lookup returns `None`, independent of iterator or ledger order.

The module does not return a richer ambiguity enum because the ticket and
downstream contract use `Option` and only need confident attribution versus no
confident attribution. Future operator surfacing can treat `None` as
unattributable.

## Fixture shape

The test fixture helper creates a complete `ProvenanceRecord` because the schema
does not define a smaller interval projection.

Fields unrelated to ownership receive inert deterministic values:

- current provenance schema version;
- `RunOutcome::Done`;
- authoritative true;
- fenced false;
- a static Claude route for requested and actual;
- no tokens or cost;
- zero concurrency.

The attempt lease ticket must agree with the record ticket. Attempt IDs differ
per fixture so the rows remain realistic.

## Acceptance test layout

The primary test uses a single physical pane and two distinct ticket intervals.

```text
time:  50    100-----------199    250    300-----------399    450
owner: None  A               A    None   B               B    None
pane:         7              7            7              7
```

Assertions cover an interior timestamp for A, an interior timestamp for B,
both endpoints, and before/between/after gaps.

A different pane at a time inside a window also returns `None`, demonstrating
that time alone cannot attribute ownership.

## Policy test layout

The secondary test uses overlapping records on one pane.

Two records for the same ticket and timestamp return that ticket.

Adding an overlapping record for a different ticket makes both iterator orders
return `None`.

This protects the design's ordering-independent, fail-closed behavior and avoids
later callers assuming the last ledger row wins.

## Dependency direction

```text
lisa-core::provenance::ProvenanceRecord
                    |
                    v
lisa-plugin::ownership::owner_at
                    |
                    v
future T-043-03 capture attribution in State
```

The new module depends inward on the stable schema. It does not depend outward
on current or future consumer code.

## Build and visibility consequences

Registering the module compiles it for native and WASM plugin builds.

Tests remain native-only through Rust's standard `#[cfg(test)]` mechanism.

`pub(crate)` allows later code in `lib.rs` to call the function. Nothing is
re-exported from `lisa-plugin`, which is a Zellij plugin rather than a library API
surface for external consumers.

## Source transaction boundary

The meaningful source unit consists of exactly:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ownership.rs`.

They must be committed together because the new file is unreachable without the
module declaration, while declaring the module without the file does not build.

The isolated commit command must pass both exact paths with separate `--include`
arguments. It must not include ticket frontmatter, private phase artifacts,
other source files, or broad directory paths.

## Verification boundary

Focused verification runs the new module's tests through the `lisa-plugin`
package.

Package verification runs all `lisa-plugin` tests.

Workspace verification runs `cargo test --workspace` if time and repository
state permit.

Formatting is checked with `cargo fmt --all -- --check` after applying normal
formatting. The final diff and Git status are inspected before and after the
isolated source commit.

## Resulting architecture

After this ticket, the plugin has a small stable ownership seam grounded in the
same terminal execution rows it already persists. Later capture attribution can
compose ledger history and a current record without changing lookup semantics,
while current writer and consumer behavior remains exactly as before.
