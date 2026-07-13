# Progress — T-043-01-01 append-only capture record schema

## Status

Implementation is complete. The planned source unit is committed through Lisa's isolated transaction, focused and workspace verification are green, and no ticket-owned source path remains modified, staged, or untracked.

Remaining work at the time of this artifact: Review artifacts only.

## Completed Step 1 — Capture module and schema

Created:

- `crates/lisa-core/src/capture.rs`

Implemented public `CaptureRecord` with:

- `pane_id: u32`
- `session_id: String`
- `captured_at: u64`
- `input_tokens: u64`
- `output_tokens: u64`

Derives:

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`
- `Serialize`
- `Deserialize`

Boundary retained:

- No ticket ID is present.
- No guessed artifact key is present.
- No provider or route is present.
- No cost is present.
- No cache-split dimensions are present.
- No no-capture marker is present.
- The timestamp follows the core epoch-seconds convention.

## Completed Step 2 — Append-only JSONL API

Added:

```rust
pub fn append_capture_record(
    path: &Path,
    record: &CaptureRecord,
) -> std::io::Result<()>;
```

Behavior implemented:

- Serializes a compact JSON object.
- Maps serde failures into invalid-data I/O errors.
- Adds exactly one row-ending newline.
- Creates missing parent directories.
- Opens the target with `create(true)` and `append(true)`.
- Writes through the append handle.
- Never opens with truncation.
- Never reads and rewrites existing rows.
- Leaves destination selection to application code.

## Completed Step 3 — Acceptance unit test

Added test:

`capture::tests::capture_record_round_trips_and_same_pane_appends_without_rewriting_first_row`

Evidence covered:

- Direct compact JSON serialization of a populated record.
- Deserialization back to an equal `CaptureRecord`.
- A first append to a nested temporary JSONL path.
- A raw-byte snapshot immediately after the first append.
- A second capture carrying the same pane ID.
- Different session, timestamp, and token facts in the second capture.
- A raw prefix comparison after the second append.
- Exact first-row comparison with the original serialized bytes.
- Exactly two non-empty newline-delimited rows.
- Ordered deserialization to the original two records.

The raw prefix assertion is stronger than semantic JSON equality: a read/rewrite operation that changed whitespace or field bytes would fail even if it produced equivalent JSON.

## Completed Step 4 — Public module exposure

Modified:

- `crates/lisa-core/src/lib.rs`

Added:

```rust
pub mod capture;
```

The public downstream paths are now:

- `lisa_core::capture::CaptureRecord`
- `lisa_core::capture::append_capture_record`

No root-level re-export or existing module change was needed.

## Completed Step 5 — Focused verification

Commands run:

```bash
cargo fmt --all
cargo test -p lisa-core capture
cargo test -p lisa-core
```

Results:

- Formatting completed successfully.
- Focused capture run: 1 passed, 0 failed.
- Full `lisa-core` unit run: 197 passed, 0 failed.
- `lisa-core` integration tests: 2 passed, 0 failed.
- `lisa-core` doc tests: 0 failures.

## Completed Step 6 — Isolated source commit

Command run:

```bash
lisa commit-ticket \
  --ticket-id T-043-01-01 \
  --message "feat(core): add append-only capture records" \
  --include crates/lisa-core/src/capture.rs \
  --include crates/lisa-core/src/lib.rs
```

Result:

- Commit: `2c2e314224a38444479322bcd7b92ea7f389bdf9`
- Subject: `feat(core): add append-only capture records`
- Source diff: 2 files, 110 insertions.
- Only the two exact planned repository-relative paths were included.
- Ordinary `git add` and ordinary `git commit` were not used.

Post-commit inspection:

- `crates/lisa-core/src/capture.rs` is clean.
- `crates/lisa-core/src/lib.rs` is clean.
- `git show --check` reports no whitespace error for the commit.

## Completed Step 7 — Broad verification

Commands run:

```bash
cargo fmt --all -- --check
cargo test --workspace
```

Results:

- Formatting check passed.
- Workspace tests exited successfully.
- `lisa-cli` library: 14 passed.
- `lisa-cli` binary: 267 passed.
- CLI integration suites shown in the run passed.
- `lisa-core`: 197 unit tests and 2 integration tests passed.
- `lisa-plugin`: 375 passed.
- Doc tests passed.
- One pre-existing real-Zellij integration test remained ignored by its environment gate.
- No live provider, model token, or Zellij session was used.

## Deviations from plan

No implementation or API deviation was required.

The only runtime wrinkle was that the workspace test exceeded the first command-yield window; it was polled to completion and exited with code 0. This did not alter scope or verification.

## Concurrent workspace observations

During post-commit status inspection, unrelated changes for sibling ticket T-043-01-02 were visible:

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/ownership.rs`
- sibling ticket/work artifacts

Those paths belong to the parallel pane-time ownership ticket. They were not edited, staged, included, or reverted by this implementation. The exact-path isolated commit kept this ticket's source boundary intact.

## Deferred work, as planned

- Rewriting `run_capture_usage` to construct and append this type belongs to T-043-02-01.
- Removing `resolve_key` and ticket-guessed overwrite files belongs to T-043-02-01.
- Visible no-capture markers belong to T-043-02-02.
- Reading and attributing capture rows belongs to T-043-03-01.
- Quarantining unattributable sessions belongs to T-043-03-02.
- Provider cache-split parity remains explicitly outside this epic slice.

## Implementation conclusion

The shared schema and append behavior required by this ticket are implemented, committed, and verified. The acceptance criterion is directly covered by a deterministic `lisa-core` test, and Review can proceed without further source changes.
