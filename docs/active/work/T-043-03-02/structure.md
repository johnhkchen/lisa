# Structure: quarantine unattributable usage

## Change inventory

Create one plugin-internal source module:

- `crates/lisa-plugin/src/quarantine.rs`

Modify one existing plugin source file:

- `crates/lisa-plugin/src/lib.rs`

Create no core, CLI, schema migration, or configuration files.

Delete no files.

Keep all phase artifacts under the attempt-private work directory.

Lisa may admit and publish those artifacts independently of source commits.

## Module registration

Add `mod quarantine;` with the other private modules at the top of `lib.rs`.

The module remains private to `lisa-plugin`.

It does not become a workspace public API.

It depends on `lisa_core::capture::CaptureRecord` as the observed fact schema.

It depends only on standard filesystem/I/O types and Serde JSON.

It does not depend on scheduler state, DAG state, provenance ownership, or UI.

This boundary keeps durable quarantine mechanics separate from attribution
policy and activity presentation.

## New module: `quarantine.rs`

### Module responsibility

Own the provider-local, per-session quarantine file format and append behavior.

The module answers two questions:

1. Which safe file path corresponds to an opaque session ID?
2. Has this append-only capture-ledger source row already been quarantined?

It does not decide whether a capture is attributable.

It does not select a provider directory.

It does not log activity.

It does not add usage totals.

### Durable record

Define a crate-visible envelope for tests and internal inspection:

```rust
pub(crate) struct QuarantinedCaptureRecord {
    pub(crate) source_line: u64,
    pub(crate) capture: CaptureRecord,
}
```

Derive debug, clone, equality, serialize, and deserialize traits.

`source_line` is 1-based and refers to the physical line in the provider's
append-only `captures.jsonl`.

`capture` preserves every observed field without translation.

The session ID remains both in the record and in the selected path.

### Append result

Define a small typed result:

```rust
pub(crate) enum AppendOutcome {
    Appended(PathBuf),
    AlreadyPresent(PathBuf),
}
```

Both cases return the resolved destination for diagnostics and tests.

`Appended` means a durable row was newly written and deserves a warning.

`AlreadyPresent` means the source row was handled by an earlier scan and must
not produce another row or event.

I/O and serialization failures use `std::io::Result`.

### Filename helper

Define:

```rust
pub(crate) fn session_path(provider_dir: &Path, session_id: &str) -> PathBuf
```

The function joins `provider_dir`, `quarantine`, and an encoded JSONL filename.

Its encoding preserves ASCII alphanumeric bytes, hyphen, and underscore.

All other UTF-8 bytes become uppercase `%HH` sequences.

An empty ID maps to a reserved percent-encoded marker.

The function never treats provider input as a directory component.

### Append function

Define:

```rust
pub(crate) fn append(
    provider_dir: &Path,
    source_line: u64,
    capture: &CaptureRecord,
) -> io::Result<AppendOutcome>
```

It derives the destination through `session_path`.

If the destination exists, it reads all parseable quarantine envelopes.

If one has the requested source line, return `AlreadyPresent`.

If reading fails for a reason other than not-found, return that failure.

Serialize the new envelope as compact JSON with a trailing newline.

Create the quarantine directory when absent.

Open with create plus append flags.

Write the complete line without rewriting prior bytes.

Return `Appended` only after `write_all` succeeds.

### Module tests

Keep focused unit tests in `quarantine.rs`.

Test filename encoding with:

- a normal UUID-like/safe session;
- slash and traversal-shaped data;
- percent and dot data;
- non-ASCII data;
- an empty ID;
- two IDs whose naive sanitization might collide.

Assert every derived file remains directly under `quarantine/`.

Test first append creates the expected nested file and parseable envelope.

Save the first file bytes.

Repeat the same source line and assert `AlreadyPresent` plus byte equality.

Append a second source line carrying an identical capture and assert a second
row exists, proving source identity does not collapse distinct ledger rows.

Optionally cover an unreadable/directory destination as an I/O failure.

## Existing file: `lib.rs`

### Consumer mutability

Change `State::read_usage` receiver from `&self` to `&mut self`.

No argument or return type changes.

All current calls already originate from mutable state or mutable tests.

`emit_provenance` remains the production entry point.

### Capture iteration

Replace `filter_map` iteration with explicit enumerated line iteration.

Enumeration must happen before JSON parsing so source line identity remains
stable in the presence of malformed rows.

Convert the zero-based index to a 1-based `u64`.

Skip malformed capture JSON exactly as today.

Retain the current physical-pane filter.

Add a pending-time filter:

```rust
capture.captured_at > current.ended_at
```

Such rows receive neither usage nor quarantine in this call.

### Ownership branch

Call `owner_at` once per eligible valid capture and branch on its result.

For `Some(current.ticket_id)`, run the existing checked summation.

For `Some(other_ticket)`, continue without side effects.

For `None`, call a new `State` helper that persists and reports quarantine.

Continue scanning after a successful or failed quarantine operation.

The totals accumulator remains absent until an owned capture occurs.

The final token/cost tuple retains its existing semantics.

### State orchestration helper

Add a private method near `read_usage`:

```rust
fn quarantine_capture(
    &mut self,
    client: AgentClient,
    source_line: u64,
    capture: &CaptureRecord,
)
```

Select and clone the provider directory from `client` before mutation.

Call `quarantine::append`.

For `AppendOutcome::Appended(path)`, log one `ActivityEvent::Warning`.

The warning names client, raw session ID, pane, timestamp, and path.

For `AlreadyPresent`, do nothing.

For `Err`, log one `ActivityEvent::Error` containing capture identity and error.

The helper returns no usage value and cannot assign ownership.

### Acceptance regression

Add one ticket-named test beside existing provenance usage regressions.

Use a temporary provider directory and provenance ledger.

Construct a current ticket interval on pane 7.

Append one capture on pane 7 whose timestamp is before the current start.

This makes `owner_at` return `None` while keeping the capture behind the
current closed-time horizon.

Call `read_usage` for the current execution record.

Assert all returned usage remains null.

Resolve the expected session file with `quarantine::session_path`.

Parse its only row as `QuarantinedCaptureRecord`.

Assert source line 1 and complete capture equality.

Assert no provider-level `quarantine.jsonl` exists.

Assert no `last`-named file exists in the provider directory.

Find the quarantine `ActivityEvent::Warning` in `state.activity_log`.

Pass it through `activity_event_to_ui_entry` and assert a warning activity.

Call `read_usage` again.

Assert the quarantine file still has one row and the log still has one matching
quarantine warning.

This proves both visibility and rescan idempotence.

## Existing behavior preserved

`CaptureRecord` remains unchanged.

`capture-usage` continues appending observed facts only.

`owner_at` remains the sole ownership confidence rule.

Prior plus current execution records remain the ownership input.

Current-ticket captures retain checked summation.

Other-ticket captures remain excluded from the current record.

Malformed capture and provenance rows remain non-authoritative.

Dollar cost remains null.

Provenance append order remains usage fill followed by durable terminal row.

Provider directories remain machine-owned and ignored by Git.

## Implementation ordering

1. Add and test the standalone quarantine module.
2. Register the module and integrate it into `read_usage`.
3. Add the end-to-end plugin acceptance regression.
4. Run formatting and focused module tests.
5. Run existing ownership and provenance regression groups.
6. Run workspace verification.

The standalone storage unit can be committed first with only
`crates/lisa-plugin/src/quarantine.rs`.

The integration unit then commits `crates/lisa-plugin/src/lib.rs`.

No phase artifact path is included in either source commit; Lisa owns their
admission and final publication.
