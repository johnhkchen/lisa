# Structure — T-043-02-02 loud no-write signal

## Change set overview

Five tracked files are modified:

1. `crates/lisa-cli/src/capture_usage.rs`
2. `crates/lisa-cli/src/main.rs`
3. `crates/lisa-cli/src/templates.rs`
4. `crates/lisa-cli/tests/capture_usage_cli.rs`
5. `crates/lisa-cli/data/hooks-guide.md`
6. `.lisa/hooks/on-stop.sh`

No source file is created or deleted.

Phase artifacts are created only in `.lisa/attempts/T-043-02-02/1/work/` and are not part of source commits.

## `crates/lisa-cli/src/capture_usage.rs`

### Responsibility

Continue to own native Stop-payload parsing and transcript usage extraction, while adding the unsuccessful-observation persistence boundary.

### Imports

Add:

- `std::fs` or the specific filesystem types needed for append mode;
- `std::io::Write` for JSONL marker writes;
- `serde::Serialize` alongside `Deserialize`.

Keep:

- `std::io::Read` for stdin;
- `SystemTime` for capture timestamps;
- successful core capture imports;
- provider parsers and `serde_json::Value`.

### `StopPayload`

Remain private and deserializable.

Fields remain:

```rust
transcript_path: Option<String>
session_id: Option<String>
```

No ticket field is introduced.

### `NoCaptureMarker`

Add a private serializable struct adjacent to `StopPayload`:

```rust
struct NoCaptureMarker<'a> {
    pane_id: u32,
    session_id: &'a str,
    captured_at: u64,
    reason: &'static str,
}
```

Borrowing session identity avoids an unnecessary clone on failure paths.

The serialized field order is stable by declaration order, though tests should parse JSON rather than compare whole bytes.

### Reason constants

Add module-private string constants:

```rust
const MISSING_TRANSCRIPT_PATH: &str = "missing-transcript-path";
const UNREADABLE_TRANSCRIPT: &str = "unreadable-transcript";
const EMPTY_TRANSCRIPT: &str = "empty-transcript";
```

Constants prevent drift between persistence, stderr, and tests inside the module.

### Error constructor boundary

Use `std::io::Error::new` with:

- `InvalidData` for unreadable stdin or malformed JSON payload;
- `InvalidInput` for missing session or pane identity.

Messages name the missing or invalid fact.

The exact error text is operator-facing but not used as the durable marker reason.

### `append_no_capture_marker`

Add a private helper with a narrow interface:

```rust
fn append_no_capture_marker(
    client_dir: &Path,
    pane_id: u32,
    session_id: &str,
    reason: &'static str,
) -> std::io::Result<()>;
```

Internal ordering:

1. Construct timestamped marker.
2. Serialize marker.
3. Add newline.
4. Create `client_dir`.
5. Open `client_dir.join("no-captures.jsonl")` in create/append mode.
6. Write bytes.
7. Emit concise stderr notice.
8. Return success.

The notice occurs after a completed append.

### `run_capture_usage`

Public signature remains unchanged:

```rust
pub fn run_capture_usage(cwd: &Path) -> std::io::Result<()>;
```

The control-flow shape becomes:

```text
read stdin
  -> parse payload
  -> establish client
  -> require session
  -> require pane
  -> derive client_dir
  -> classify transcript path/read/parse
       failure observation -> append no-capture marker
       successful totals   -> append CaptureRecord
```

Missing transcript path moves after identity validation.

An empty path string is filtered into the missing-path outcome.

Unreadable transcript error details are not stored in the marker.

Successful record construction remains unchanged.

### Module documentation

Replace the statement that every missing input writes nothing.

Describe:

- successful capture ledger;
- no-capture ledger;
- required identity facts;
- real errors when an identified marker cannot be written.

### Unit tests

Existing parser tests remain unchanged unless formatting moves code.

No stdin-global unit test is added because compiled CLI tests isolate process stdin and environment safely.

## `crates/lisa-cli/src/main.rs`

### Responsibility

Make `CaptureUsage` honor `run_capture_usage` failure status.

### Dispatch change

Replace ignored result:

```rust
let _ = capture_usage::run_capture_usage(&cwd);
```

with normal error handling:

```rust
if let Err(error) = capture_usage::run_capture_usage(&cwd) {
    eprintln!("Error: {error}");
    std::process::exit(1);
}
```

Update the adjacent comment or remove it so it no longer claims errors are swallowed.

No clap flags or command names change.

## `crates/lisa-cli/src/templates.rs`

### `ON_STOP_HOOK`

Keep signal directory creation and `.stopped` write unchanged.

Keep reading stdin once into `in`.

Change only the capture command suffix so stderr and status propagate.

Update comments from “best effort” to the durable marker/error behavior.

Update the Rust doc comment to remove old overwrite and null-token claims.

### `LEGACY_ON_STOP_HOOKS`

Expand the slice to two exact generations:

1. The immediately preceding capture hook with `2>/dev/null || true`.
2. The older v0.3 hook with no capture invocation.

Neither entry may equal the new current template.

The previous current hook bytes must remain exact so init recognizes deployed copies.

### Template tests

Extend `stop_hook_still_writes_stopped_and_captures_usage` or rename it to cover visibility.

Assertions retain:

- stopped signal path;
- `capture-usage`;
- `LISA_BIN` fallback;
- one stdin read.

Assertions add:

- no `2>/dev/null`;
- no `|| true`.

Optional legacy assertions can ensure both prior hook generations differ from current.

## `.lisa/hooks/on-stop.sh`

### Responsibility

Represent the live repository hook used by the current Lisa session.

### Change

Match the new `ON_STOP_HOOK` bytes exactly:

- preserve shebang and executable mode;
- preserve `.stopped` signal behavior;
- preserve stdin forwarding;
- remove stderr redirection;
- remove forced-success masking;
- update explanatory comments.

This file is explicitly ticket-owned by story scope.

## `crates/lisa-cli/tests/capture_usage_cli.rs`

### Existing helper

Refactor `capture_usage` to return `std::process::Output` instead of internally asserting success.

The successful-capture test will assert success through a small assertion helper or directly.

Keep environment setup:

- `LISA_PANE_ID`;
- stale `LISA_TICKET_ID` regression guard;
- absent `LISA_AGENT_CLIENT` for Claude;
- piped stdin/stdout/stderr.

### Marker representation

Integration tests cannot use the private CLI marker type.

Define a test-local `Deserialize` struct with the same observable fields:

```rust
struct NoCaptureMarker {
    pane_id: u32,
    session_id: String,
    captured_at: u64,
    reason: String,
}
```

This verifies the on-disk public shape without making production internals public.

### New test

Add:

```rust
#[test]
fn empty_and_unreadable_transcripts_append_visible_no_capture_markers()
```

Fixture setup:

- temporary root;
- one empty transcript file;
- one path that does not exist;
- one pane ID;
- two distinct session IDs;
- before/after epoch bounds.

Invocation assertions:

- both process statuses are success;
- each stderr contains `lisa capture-usage: no capture`;
- each stderr contains its stable reason.

Persistence assertions:

- `.lisa/claude/captures.jsonl` does not exist;
- `.lisa/claude/no-captures.jsonl` exists;
- exactly two rows deserialize;
- row order matches invocation order;
- pane IDs match;
- session IDs match;
- reasons match;
- timestamps fall in bounds.

### Optional missing-path coverage

The same helper can support a payload without a transcript field, but the acceptance test only needs empty and unreadable cases.

Missing-path behavior may be covered through an additional invocation if it keeps the test focused.

## `crates/lisa-cli/data/hooks-guide.md`

### Operator-facing addition

Add a short subsection near “How hooks work” or the lifecycle table.

Name both provider-specific ledgers and their roles.

Explain that no-capture rows include:

- pane;
- session;
- time;
- reason.

Explain that hook stderr is intentionally visible when marker persistence or payload identity fails.

Avoid promising plugin attribution or dashboard surfacing not yet implemented.

## Commit units

### Unit 1 — Capture outcome contract and CLI regression

Paths:

- `crates/lisa-cli/src/capture_usage.rs`
- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/tests/capture_usage_cli.rs`

This is one meaningful unit because the writer, exit boundary, and compiled regression define one behavior.

### Unit 2 — Hook visibility, upgrade compatibility, and operator guide

Paths:

- `crates/lisa-cli/src/templates.rs`
- `.lisa/hooks/on-stop.sh`
- `crates/lisa-cli/data/hooks-guide.md`

This is one meaningful unit because generated/live behavior and its documentation must move together.

## Explicit non-changes

- No change to `lisa_core::capture::CaptureRecord`.
- No change to `append_capture_record`.
- No change to plugin code.
- No change to pane ownership intervals.
- No change to ticket phase or status fields.
- No publication to `docs/active/work/T-043-02-02`.
- No ordinary git index operations.
- No direct completion command.
