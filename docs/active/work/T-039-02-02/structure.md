# Structure: T-039-02-02

## Change set

The implementation is confined to the Lisa plugin crate.

Files created:

- `crates/lisa-plugin/src/signal.rs`

Files modified:

- `crates/lisa-plugin/src/lib.rs`

Files intentionally unchanged:

- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`
- `crates/lisa-plugin/src/codex_ack.rs`
- `crates/lisa-plugin/src/adapter.rs`
- all `lisa-core` files
- all `lisa-cli` files
- ticket frontmatter

## New `signal` module

`crates/lisa-plugin/src/signal.rs` owns the filesystem-facing ingestion edge.
It does not own scheduler state, state transitions, current-attempt admission,
provider acknowledgement semantics, logging, or notifications.

### Imports

- `std::path::Path` for the signal directory.
- `lisa_core::types::{AttemptLease, TicketId}` for typed domain payloads.
- Standard filesystem and filename facilities through qualified paths or focused
  imports.

### `SignalRequest`

Define a crate-private enum with one variant per existing check loop:

- `Heartbeats`
- `ProcessStarts`
- `ShellReady`
- `CodexAcknowledgements`
- `Awaiting`
- `Idle`
- `Transitions`
- `Errors`

The request is a closed declaration of which records one directory scan may
consume. It prevents callers from supplying arbitrary suffix/payload pairings.

### `IdleTarget`

Define a crate-private enum:

- `Pane(u32)`
- `LegacyTicket(TicketId)`

This type retains the only legacy filename route without representing legacy
support as a generic property of every signal.

### `SignalRecord`

Define a crate-private enum with explicit payload families:

- `Heartbeat { pane_id: u32, lease: AttemptLease }`
- `ProcessStarted { pane_id: u32, lease: AttemptLease }`
- `ShellReady { pane_id: u32, lease: AttemptLease }`
- `CodexAcknowledgement { pane_id: u32, payload: String }`
- `Awaiting { pane_id: u32 }`
- `Idle { target: IdleTarget }`
- `Stopped { pane_id: u32 }`
- `Cleared { pane_id: u32 }`
- `Error { pane_id: u32 }`

Lease-bearing variants store parsed leases rather than raw strings. The Codex
variant stores raw provider text rather than claiming it is a lease. Presence
variants have no payload field.

### `ingest`

Define the single crate-private boundary:

```rust
pub(crate) fn ingest(dir: &Path, request: SignalRequest) -> Vec<SignalRecord>
```

Responsibilities:

1. Call `read_dir` for the requested scan.
2. Return an empty vector if the directory is unavailable.
3. Flatten per-entry filesystem errors as existing loops do.
4. Ask request-specific recognition code to consume each path.
5. Collect zero or one typed record per directory entry.
6. Preserve underlying `read_dir` order within the returned vector.

The function does not sort, retry, log, or expose I/O errors.

### Strict pane helper

Provide a private filename parser equivalent to the existing
`pane_id_from_signal_filename` behavior:

- accept an `OsStr` and exact suffix;
- require UTF-8;
- require `pane-` prefix;
- require the exact suffix;
- parse the middle as `u32`.

Strict request branches call this before deleting. This helper moves from
`lib.rs` because filename grammar is part of ingestion.

### Lease ingestion helper

Provide a private helper used only by the three lease request branches.

Inputs:

- path;
- pane ID already recognized;
- constructor identifying the specific record variant.

Ordering:

1. Read the body as UTF-8.
2. Deserialize `AttemptLease` JSON.
3. Delete the recognized path regardless of read/parse outcome.
4. Return the constructed typed record only when parsing succeeded.

The helper must not validate current scheduler ownership.

### Raw provider ingestion helper

The Codex acknowledgement branch:

1. recognizes a strict `.ack` pane filename;
2. reads the file to `String`;
3. deletes the recognized path;
4. returns a `CodexAcknowledgement` only if reading succeeded.

It does not invoke `codex_ack` parsing.

### Strict presence ingestion

Awaiting and error branches:

1. recognize the complete strict pane filename;
2. delete the recognized path;
3. return the corresponding presence record.

They never read the file body.

### Idle ingestion

The idle request branch:

1. requires a UTF-8 filename ending in `.idle`;
2. deletes immediately after that broad recognition;
3. examines the stem before `.idle`;
4. if the stem starts with `pane-`, parses the remainder as `u32`;
5. returns `IdleTarget::Pane` for a valid pane;
6. returns no record for a malformed pane stem;
7. otherwise returns `IdleTarget::LegacyTicket` using the complete stem.

This ordering preserves deletion of malformed pane idle names.

### Transition ingestion

The transitions request branch:

1. requires a UTF-8 filename with the `pane-` prefix;
2. checks `.stopped` before `.cleared`, matching the current branch order;
3. deletes immediately once either suffix is broadly recognized;
4. parses the pane number after deletion;
5. returns `Stopped` or `Cleared` only for a valid pane number;
6. leaves `.idle` and every other suffix untouched.

Stopped and cleared remain interleaved according to directory entry order.

### Module tests

Add a `#[cfg(test)]` child module in `signal.rs`.

Test groups:

- lease record parses to `AttemptLease` and deletes;
- malformed lease is deleted and yields no record;
- raw provider payload is preserved exactly;
- presence record ignores body and deletes;
- invalid strict pane filename is retained;
- idle emits pane and legacy target variants;
- malformed pane idle is deleted without a record;
- transitions emit both variants in one scan;
- malformed transition pane is deleted without a record;
- unrelated suffixes remain untouched.

Tests use `tempfile`, already available to the plugin test target.

## `lib.rs` module declaration

Add:

```rust
mod signal;
```

near the other focused private modules.

Import the boundary types:

```rust
use signal::{IdleTarget, SignalRecord, SignalRequest};
```

Calls may use `signal::ingest` so the boundary remains obvious at each loop.

Remove the top-level `pane_id_from_signal_filename` helper after consumers and
its focused tests have migrated to the module. Existing behavioral coverage is
recreated in `signal.rs`; characterization tests exercise the public behavior.

## Consumer rewrites

### Heartbeat

Replace directory scan, filename parse, read, deserialize, and delete with:

```text
for record in ingest(Heartbeats)
  destructure Heartbeat
  apply existing exact-attempt admission
  apply existing state effects
```

No downstream admission expression changes.

### Process start

Iterate `ProcessStarted` records and call the existing acknowledgement method.

### Shell ready

Iterate `ShellReady` records and call the existing acknowledgement method with
the current time at dispatch.

### Codex acknowledgement

Iterate raw `CodexAcknowledgement` records. Keep downstream acknowledgement,
activity refresh, and logging unchanged.

### Awaiting

Iterate `Awaiting` records. Keep set insertion and conditional logging unchanged.

### Idle

Keep `idle_alerts.clear()` as the first operation.
Replace only scan, filename, deletion, and target resolution.

- `IdleTarget::Pane` performs the existing transition-state slot check, activity
  bump, and assigned-ticket resolution.
- `IdleTarget::LegacyTicket` supplies the ticket ID directly.
- Preserve `idle_pane_id` for notification environment construction.
- Leave the entire phase match and artifact logic unchanged.

### Transition

Iterate one `Transitions` ingestion result.

- `Stopped` refreshes activity and calls `handle_stopped_signal`.
- `Cleared` refreshes activity and calls `handle_cleared_signal`.
- No other request or scan is introduced.

### Error

Iterate `Error` records. Leave recovery and running-thread authority logic
unchanged.

## Poll tick

Do not restructure `poll_tick`.

The following order remains source-visible:

1. heartbeat;
2. awaiting;
3. process start;
4. shell ready;
5. Codex acknowledgement;
6. idle;
7. transition;
8. error.

Existing interleaved operations remain in place.

## Commit boundary

The source unit is one coherent refactor because the new module and consumer
migration must compile together. Commit both exact paths in one Lisa isolated
transaction:

- `crates/lisa-plugin/src/signal.rs`
- `crates/lisa-plugin/src/lib.rs`

The attempt-private artifacts are not included in the source commit; Lisa owns
their later admission and publication.

## Verification boundary

- Format the workspace.
- Run the unchanged characterization filter first.
- Run focused new `signal` module tests.
- Run `cargo test --workspace`.
- Run Clippy for the workspace with warnings denied, following repository tasks.
- Run formatting and diff whitespace checks.
- Confirm the characterization file has no diff.
- Confirm both ticket-owned source paths are clean after `lisa commit-ticket`.
- Confirm the ordinary Git index has no ticket-owned entries.
