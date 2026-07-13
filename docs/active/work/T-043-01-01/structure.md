# Structure — T-043-01-01 append-only capture record schema

## Change summary

The implementation adds one focused `lisa-core` module and exposes it from the crate root. No existing capture writer or plugin consumer changes in this ticket.

## Files created

### `crates/lisa-core/src/capture.rs`

Purpose:

- Own the shared serialized shape for one successful usage capture.
- Own the filesystem operation that appends one shape as one JSONL row.
- Hold focused unit tests for serialization and append-only preservation.

Module-level documentation will state the boundary:

- A capture row is an observation made before ticket attribution.
- It intentionally lacks ticket identity.
- Later code attributes rows using pane and capture time.
- The helper appends and does not select an application storage location.

Public type:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRecord {
    pub pane_id: u32,
    pub session_id: String,
    pub captured_at: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}
```

Field boundaries:

- `pane_id` is the terminal pane observed by the capturing process.
- `session_id` is the opaque provider/client session identifier.
- `captured_at` is UTC epoch seconds.
- `input_tokens` is the captured aggregate input-side token count.
- `output_tokens` is the captured output-side token count.
- All fields are public because downstream writers construct records and downstream consumers inspect them.
- There are no attribution, route, cost, or lifecycle fields.

Public function:

```rust
pub fn append_capture_record(
    path: &Path,
    record: &CaptureRecord,
) -> std::io::Result<()>;
```

Function responsibilities:

1. Serialize the record to compact JSON.
2. Convert a serde serialization failure to `io::ErrorKind::InvalidData`.
3. Add one newline to frame the JSONL row.
4. Create the parent directory when the path has a non-empty parent.
5. Open the target with `create(true)` and `append(true)`.
6. Write the framed row bytes.
7. Return any directory, open, or write error to the caller.

Function non-responsibilities:

- It does not choose `.lisa` or a provider-specific directory.
- It does not derive a filename from ticket, pane, or session values.
- It does not read, deduplicate, aggregate, or rewrite existing records.
- It does not attach a ticket ID.
- It does not convert clock values.
- It does not quarantine failed attribution.
- It does not emit no-capture markers.

Private implementation organization:

- Standard-library imports: `fs`, `io::Write`, and `path::Path`.
- Serialization imports: `serde::{Deserialize, Serialize}`.
- The type precedes the append function.
- Tests live in a trailing `#[cfg(test)] mod tests` block.
- The implementation is intentionally direct; no generic JSONL abstraction is introduced.

Unit-test organization:

- A `sample_capture()` helper constructs the baseline record.
- The primary acceptance test may combine round-trip and two-append evidence so the exact ticket condition is readable in one place.
- A temporary directory supplies an isolated path.
- The path includes a missing parent to cover directory creation incidentally.
- The first append is read as raw bytes and saved.
- The second record retains the same pane ID.
- The second record changes session, timestamp, and token totals.
- After the second append, raw bytes must start with the saved first-write bytes.
- Newline splitting must yield exactly two non-empty rows.
- Each row must deserialize into the expected `CaptureRecord`.
- The first raw row must equal the serialized first record exactly.

## Files modified

### `crates/lisa-core/src/lib.rs`

Change:

```rust
pub mod capture;
```

Placement:

- Add the module declaration with the existing alphabetically arranged public modules.
- `capture` belongs before `client`.

Effect:

- Downstream code can import `lisa_core::capture::CaptureRecord`.
- Downstream code can import `lisa_core::capture::append_capture_record`.
- No root-level re-export is introduced because other core domains use module-qualified imports.

## Files unchanged by design

### `crates/lisa-cli/src/capture_usage.rs`

- Continues writing the old ticket-guessed overwrite artifact until T-043-02-01.
- Does not import or construct `CaptureRecord` in this ticket.

### `crates/lisa-plugin/src/lib.rs`

- Continues reading the old provider usage artifacts until T-043-03-01.
- Does not deserialize capture JSONL in this ticket.

### `crates/lisa-core/src/provenance.rs`

- Its schema, schema version, append API, and tests remain unchanged.
- It provides precedent but is not refactored into a shared utility.

### Cargo manifests and lockfile

- No dependencies are added.
- Existing serde, serde_json, and tempfile entries are sufficient.
- No lockfile change is expected.

### Ticket frontmatter

- No manual phase or status updates are made.
- Lisa transitions phases in response to attempt artifacts.

## Component boundary after the change

```text
lisa-core::capture
  CaptureRecord                   shared raw-capture schema
  append_capture_record(path, r)  shared append-only JSONL operation

lisa-cli::capture_usage           unchanged legacy writer (later consumer)
lisa-plugin                       unchanged legacy reader (later consumer)
lisa-core::provenance             unchanged attributed terminal ledger
```

## Implementation ordering

1. Create `capture.rs` with imports, documentation, public record, and append function.
2. Add the acceptance-focused unit test in the same module.
3. Declare `pub mod capture` in `lib.rs`.
4. Format the workspace.
5. Run the focused core test suite.
6. Run the workspace suite to validate downstream compilation.
7. Commit both source paths as one meaningful schema unit through Lisa's isolated transaction.

## Commit ownership

The single source unit consists of exactly:

- `crates/lisa-core/src/capture.rs`
- `crates/lisa-core/src/lib.rs`

Both paths are required for a usable public API and should be committed together with one `lisa commit-ticket` invocation. Attempt artifacts are not included; Lisa owns their publication and final completion commit.

## Expected stable interface

The later CLI writer can construct a `CaptureRecord` from pane/session environment or hook payload data, the capture clock, and parser totals, then call `append_capture_record` on its chosen ledger path.

The later plugin reader can parse each non-empty JSONL row directly as `CaptureRecord` and pass `pane_id` plus `captured_at` to its ownership lookup. This ticket does not prescribe those call sites beyond providing the type-safe boundary they share.
