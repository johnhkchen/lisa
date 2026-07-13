# Structure — T-043-02-01 append capture, not overwrite

## Change summary

The implementation rewires the existing CLI capture module from guessed-key overwrite artifacts to shared append-only `CaptureRecord` rows. It adds one binary integration test and updates command help that names the old path. No core or plugin behavior changes.

## Files modified

### `crates/lisa-cli/src/capture_usage.rs`

Purpose after the change:

- Parse Stop-hook input.
- Select the provider transcript parser.
- Calculate successful aggregate token totals.
- Collect pane, session, and capture-time facts.
- Append one pre-attribution success record.

#### Imports

Add:

- `std::time::SystemTime` for capture time.
- `lisa_core::capture::{append_capture_record, CaptureRecord}` for the shared contract.
- `lisa_core::provenance::system_time_to_epoch` for timestamp conversion.

Retain:

- `std::io::Read` for hook standard input.
- `std::path::Path` for project-root handling.
- serde payload deserialization.
- serde_json values for provider transcript parsing.

#### Module documentation

Replace the last-write-wins description with the new pre-attribution model:

- Successful Stops append one record.
- Provider-specific ledgers live below `.lisa/claude` or `.lisa/codex`.
- Records contain only pane, session, time, and token observations.
- Ticket ownership is deliberately deferred to the plugin.
- Current no-capture behavior remains best-effort until T-043-02-02.

#### `StopPayload`

Current shape:

```rust
struct StopPayload {
    transcript_path: Option<String>,
}
```

New shape:

```rust
struct StopPayload {
    transcript_path: Option<String>,
    session_id: Option<String>,
}
```

Both fields retain `#[serde(default)]`. The module accepts only a non-empty session string for a successful record.

#### `UsageTotals`

No shape or visibility change.

Responsibilities remain:

- Internal normalization of provider transcript totals.
- Equality with default totals for existing no-observation detection.

#### Provider parsers

No structural change to:

- `sum_claude_transcript_usage`.
- `codex_transcript_usage`.

Their unit tests remain colocated in the module.

#### Removed `usage_artifact`

Delete the private helper entirely.

Its former responsibilities are obsolete:

- No guessed key is serialized.
- No nested legacy `usage` object is emitted.
- Shared `CaptureRecord` serde owns the new row shape.

Delete the unit test named `artifact_shape_matches_extract_usage`, since the old artifact shape is no longer a product contract.

#### Removed `resolve_key`

Delete the private helper entirely.

The capture path must no longer:

- Read `LISA_TICKET_ID`.
- Prefer a ticket string.
- Format a pane fallback key.
- Use a shared `last` fallback.
- Select filenames through inferred attribution.

No replacement key resolver is introduced.

#### `run_capture_usage`

The public signature remains:

```rust
pub fn run_capture_usage(cwd: &Path) -> std::io::Result<()>;
```

Input sequence:

1. Read standard input.
2. Deserialize `StopPayload`.
3. Extract a transcript path.
4. Extract a non-empty session ID.
5. Read and parse `LISA_PANE_ID` as `u32`.
6. Read transcript contents.
7. Select Claude or Codex parser.
8. Reject the existing all-zero no-observation result.

Record sequence:

1. Convert `SystemTime::now()` to epoch seconds.
2. Construct `CaptureRecord` from required facts and totals.
3. Choose provider directory from `LISA_AGENT_CLIENT` exactly as today.
4. Append to `<provider-dir>/captures.jsonl`.

Filesystem boundary:

- `append_capture_record` creates the provider directory through parent creation.
- `run_capture_usage` no longer calls `create_dir_all` itself.
- `run_capture_usage` no longer calls `std::fs::write`.
- `run_capture_usage` no longer serializes its own persistence value.

Non-responsibilities:

- It does not attribute a capture to a ticket.
- It does not read scheduler history.
- It does not deduplicate repeated Stops.
- It does not aggregate capture rows.
- It does not quarantine unmatched records.
- It does not emit a no-capture marker yet.

### `crates/lisa-cli/src/main.rs`

Only the `CaptureUsage` help surface changes.

Old semantics named in help:

- Claude-specific output.
- `<ticket>.usage.json`.
- Direct provenance-ledger consumption.

New semantics named in help:

- Native session token usage.
- Provider capture JSONL.
- Project-root destination below `.lisa`.

The dispatch behavior remains unchanged:

- Resolve `--cwd`.
- Invoke `run_capture_usage`.
- Preserve current hook-safe error swallowing.

No command names, flags, ordering, or Clap structure change.

## File created

### `crates/lisa-cli/tests/capture_usage_cli.rs`

Purpose:

- Exercise the compiled `lisa capture-usage` command twice.
- Reproduce the stale inherited ticket environment.
- Verify append-only shared-record persistence at the CLI boundary.

#### Test helpers

A small invocation helper will:

- Build `std::process::Command` from `env!("CARGO_BIN_EXE_lisa")`.
- Supply `capture-usage --cwd <temp-root>`.
- Set stable `LISA_PANE_ID`.
- Set deliberately stale `LISA_TICKET_ID`.
- Remove `LISA_AGENT_CLIENT`.
- Pipe a serialized Stop payload into child standard input.
- Await and assert successful exit.

The helper will use `std::io::Write` to feed stdin and will retain the child output for a useful failure message.

#### Main acceptance test

The single test owns:

- A temporary directory.
- Two transcript files.
- Two unique session IDs.
- One shared numeric pane ID.
- One deliberately stale ticket ID.
- Epoch bounds around both invocations.

The first transcript produces one known input/output pair. The second transcript produces a different pair, including cache-input coverage if useful. Both are valid Claude assistant JSONL.

After invoking the command twice, the test:

- Reads `.lisa/claude/captures.jsonl`.
- Splits and parses every row as `CaptureRecord`.
- Requires exactly two records.
- Compares pane, session, tokens, ordering, and timestamp bounds.
- Requires the stale `<ticket>.usage.json` path to be absent.

The test does not rely on sleeping between invocations because equal epoch seconds are valid.

## Files unchanged

### `crates/lisa-core/src/capture.rs`

- The schema and append helper are consumed as-is.
- No contract extension is required.
- Its unit test remains the low-level byte-preservation proof.

### `crates/lisa-core/src/provenance.rs`

- Only its public timestamp conversion is reused.
- Provenance schema and extraction behavior remain unchanged.

### `crates/lisa-plugin/src/lib.rs`

- The old usage reader remains until T-043-03-01.
- No temporary dual-write compatibility is added.

### `crates/lisa-cli/src/templates.rs`

- Hook stderr suppression remains until T-043-02-02.
- Stop payload forwarding already includes the complete JSON input.

### `crates/lisa-cli/data/hooks-guide.md`

- Operator guidance changes belong to T-043-02-02.

### Cargo manifests and lockfile

- No dependency is added.
- The integration test uses the standard library and existing tempfile dependency.

### Ticket frontmatter

- No phase or status field is edited manually.

## Component boundary after the change

```text
native Stop hook
  -> lisa capture-usage
     -> StopPayload { transcript_path, session_id }
     -> provider transcript parser
     -> CaptureRecord { pane_id, session_id, captured_at, tokens }
     -> append_capture_record
     -> .lisa/<provider>/captures.jsonl

plugin attribution and provenance consumption
  -> unchanged in this ticket; replaced by S-043-03
```

## Implementation ordering

1. Add the binary integration test expressing the new observable contract.
2. Run it against the old writer to confirm regression sensitivity.
3. Update `capture_usage.rs` imports, payload, documentation, and persistence.
4. Delete legacy artifact and key helpers plus obsolete shape test.
5. Update the `main.rs` capture command help.
6. Format and run focused CLI tests.
7. Commit the three exact source/test paths as one meaningful writer unit.
8. Run workspace verification.
9. Inspect the committed diff and ticket-owned path cleanliness.

## Commit ownership

One cohesive source unit owns exactly:

- `crates/lisa-cli/src/capture_usage.rs`
- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/tests/capture_usage_cli.rs`

The implementation and its binary regression test should be committed together through `lisa commit-ticket`. Attempt-private RDSPI artifacts are not included; Lisa admits and publishes them separately.
