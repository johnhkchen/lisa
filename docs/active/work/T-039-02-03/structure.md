# Structure: T-039-02-03

## Change summary

This ticket adds one test source file and one nested-module declaration. It does
not alter production behavior, public APIs, core types, provider adapters, hooks,
or the existing characterization source.

## Files created

### `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`

Responsibility: post-refactor regression coverage for the typed signal-ingestion
contract and its scheduler seam.

The module is compiled only through the parent `#[cfg(test)] mod tests` in
`lib.rs`. It uses the parent test namespace to access private plugin types,
private ingestion enums, and established scheduler fixture helpers.

Planned internal sections:

1. imports and constants;
2. small temporary-state fixture;
3. lease construction or current-attempt fixture;
4. record sorting helper where multi-record requests are asserted;
5. exact typed-record contract test;
6. recognition/deletion distinction test;
7. ingestion-versus-attempt-admission test;
8. full poll-sequence test.

No symbols in this file are exposed outside tests.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

Add one declaration inside the existing `#[cfg(test)] mod tests`:

```rust
mod signal_ingestion_regression;
```

It will sit next to:

```rust
mod signal_consumer_characterization;
```

No production imports or methods change. The declaration is the only planned
diff in `lib.rs`.

## Files retained unchanged

### `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`

The 399-line BEFORE suite remains byte-for-byte unchanged. This is a hard
structural requirement because the story uses it as the pre-refactor behavioral
baseline.

### `crates/lisa-plugin/src/signal.rs`

The typed boundary and its focused unit tests remain unchanged unless a regression
test exposes an actual mismatch. No such mismatch is currently observed.

### Other source areas

- `crates/lisa-core` remains unchanged.
- `crates/lisa-cli` remains unchanged.
- Provider adapters remain unchanged.
- Codex acknowledgement parsing remains unchanged.
- Hook templates remain unchanged.
- Cargo manifests remain unchanged.
- External fixture directories remain unchanged.

## Test module boundary

The parent module already imports all production items with `use super::*`.
Nested test modules can use their own `use super::*` and receive:

- `State`;
- `AttemptLease`;
- `AgentClient`;
- `SeatAssignmentState`;
- `SignalRequest`;
- `SignalRecord`;
- `IdleTarget`;
- test-only helpers such as `fresh_slot`;
- test-only helpers such as `install_current_attempt`.

Additional standard-library imports stay local to the regression file:

- `std::fs` for temporary signal files;
- `std::time::SystemTime` where activity timestamps are compared.

The existing `tempfile` and `serde_json` dependencies are sufficient. No new
manifest dependency is needed.

## Fixture boundary

The regression file defines a local state fixture rather than importing the
private helper from the sibling characterization module. Sibling nested modules
do not expose their helpers, and coupling the AFTER suite to the BEFORE suite
would weaken their conceptual separation.

The local fixture will:

- allocate a `TempDir`;
- create its `signals` child directory;
- return a default `State` with `signal_dir` replaced;
- retain the `TempDir` for the test lifetime.

A running-attempt helper will:

- create a Codex slot for one pane;
- bind a ticket ID;
- create a running `Thread`;
- call the parent's `install_current_attempt` helper;
- stamp the slot with the returned lease if the parent helper does so through
  the bound ticket fixture.

The fixture constants will use an isolated ticket ID and pane ID.

## Typed-record test organization

The exact mapping test will use independent write/ingest/assert blocks. Reusing
one directory is safe because each request owns a distinct suffix and consumes
its own fixture before the next block.

Lease-bearing blocks use one common `AttemptLease` value serialized with
`serde_json`. The expected record reproduces that value by clone.

The acknowledgement block uses raw provider-shaped text that is deliberately
not an `AttemptLease`. Exact string equality proves raw preservation.

Presence blocks write nonempty arbitrary bodies. Expected variants contain no
payload, proving body absence from the typed contract.

Idle and transition blocks return two records. Their actual and expected vectors
will be sorted using a stable debug-derived key before equality. This avoids
asserting platform-dependent directory iteration.

After every block, consumed path existence will be checked where it strengthens
the contract.

## Recognition/deletion test organization

This test will create each edge case immediately before invoking the relevant
request. That avoids an earlier request unintentionally observing an unrelated
fixture and keeps failure attribution local.

Strict cases:

- malformed pane heartbeat remains;
- valid pane malformed heartbeat disappears with no record;
- valid pane raw acknowledgement disappears and returns the raw record;
- legacy acknowledgement remains.

Broad cases:

- malformed pane idle disappears with no record;
- malformed pane stopped disappears with no record;
- unrelated idle remains during transition ingestion.

Assertions use both returned vectors and filesystem existence. A change in
recognition or deletion timing therefore cannot hide behind the same final
scheduler state.

## Attempt-admission test organization

The test uses heartbeat because it gives direct observability of admission:

- slot activity timestamp;
- thread activity timestamp;
- `awaiting_human` membership;
- `notified_attention` membership.

The stale lease differs only in `attempt_id`. It retains the same ticket ID and
valid serialized shape. A direct `signal::ingest` call must return it exactly.
That direct record is consumed and the file is recreated before scheduler
testing.

The scheduler stale branch must leave all four observable facts unchanged. The
current branch must update activity and clear both gates. This locates currency
admission downstream without exposing internal helper methods.

## Poll-order test organization

The test reads `lib.rs` at compile time with `include_str!`. It slices from the
`poll_tick` declaration through the next method declaration. It walks an array
of expected call strings using sequential `split_once` operations.

The expected array includes signal and key non-signal calls. It does not assert
comments or whitespace. It therefore tolerates documentation edits while
rejecting missing or reordered scheduler actions.

## Ownership and commit shape

One meaningful test unit owns exactly:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`.

Both paths will be passed explicitly to one `lisa commit-ticket` invocation.
The characterization file will not be included because it will not change.
Attempt-private phase artifacts are not source commit inputs; Lisa publishes
them during completion.

## Verification boundaries

Fast verification targets the new module name. Preservation verification targets
the existing characterization module name. Broader checks cover the plugin and
workspace. Static checks cover formatting and Clippy. Git checks confirm the
ordinary index is empty and all ticket-owned source files are clean after the
isolated commit.

## Expected final tree

```text
crates/lisa-plugin/src/
├── lib.rs
├── signal.rs
└── tests/
    ├── signal_consumer_characterization.rs
    └── signal_ingestion_regression.rs
```

This makes the temporal testing strategy visible in the repository: retained
behavioral characterization beside explicit post-boundary regression locks.
