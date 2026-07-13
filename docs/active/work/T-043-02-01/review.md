# Review — T-043-02-01 append capture, not overwrite

## Disposition

Pass. The requested writer change is complete, committed through Lisa's isolated transaction, directly covered at the compiled CLI boundary, and green across the workspace. No critical issue blocks completion.

## Commit reviewed

- Commit: `ea9b29507bb6dd583e3d5e856ed53b4b89e8e991`.
- Subject: `fix(cli): append honest capture records`.
- Files changed: 3.
- Insertions: 137.
- Deletions: 61.
- Dependency changes: none.
- Lockfile changes: none.

`git show --check` reports no whitespace errors.

## What changed

### `crates/lisa-cli/src/capture_usage.rs`

The native capture writer now emits the shared append-only record contract.

For every successful transcript observation it records:

- the numeric pane ID from `LISA_PANE_ID`;
- the opaque provider session ID from Stop-hook JSON;
- the current UTC epoch-second capture time;
- aggregate input tokens;
- aggregate output tokens.

It appends the record to:

```text
.lisa/<client>/captures.jsonl
```

Claude and Codex remain separated through their existing provider directories. The record itself remains provider-neutral and contains only the facts selected by the prerequisite shared contract.

The writer uses:

- `lisa_core::capture::CaptureRecord` for schema;
- `lisa_core::capture::append_capture_record` for JSONL emission;
- `lisa_core::provenance::system_time_to_epoch` for scheduler-compatible time.

It no longer uses:

- `LISA_TICKET_ID`;
- a guessed ticket key;
- a pane-derived fallback key;
- the shared fallback string `last`;
- a nested `{ key, usage }` artifact;
- `std::fs::write` overwrite semantics;
- `<key>.usage.json` filenames.

The legacy `resolve_key` and `usage_artifact` helpers were deleted, rather than retained unused. This makes the false-attribution path unavailable inside the module and satisfies the explicit criterion that the key-guess path be gone.

Existing Claude and Codex transcript parsers were preserved. The change does not alter provider token calculation, cache folding, malformed-line handling, or Codex latest-cumulative selection.

### `crates/lisa-cli/src/main.rs`

The `capture-usage` help text now describes native-session capture JSONL instead of a Claude ticket-keyed usage file.

The command's flags and dispatch remain unchanged:

- command name remains `capture-usage`;
- `--cwd` remains available and defaults to `.`;
- main still resolves the path;
- current hook-safe error swallowing remains pending T-043-02-02.

No unrelated command surface was modified.

### `crates/lisa-cli/tests/capture_usage_cli.rs`

A new integration test exercises the actual compiled `lisa` binary, not a private helper.

It creates:

- one temporary project root;
- two distinct Claude transcript files;
- two distinct provider session IDs;
- one shared pane ID;
- one shared deliberately stale ticket environment value.

It then invokes `lisa capture-usage` twice, piping a separate Stop payload into each child process.

The resulting capture ledger is parsed through the public `CaptureRecord` type, making the test both a CLI behavior test and a real cross-crate serialization-contract test.

## Acceptance-criterion assessment

Criterion:

> A CLI test: two `capture-usage` invocations for one pane with different transcripts append two CaptureRecords carrying pane/session/captured_at, and no `<ticket>.usage.json` is written; resolve_key's ticket-guess path is gone.

Assessment: satisfied in full.

### Two CLI invocations

The integration test starts `env!("CARGO_BIN_EXE_lisa")` twice. Each process receives `capture-usage --cwd <temp-root>` and actual JSON over stdin.

### One pane

Both invocations set `LISA_PANE_ID=42`. Both decoded rows assert `pane_id == 42`.

### Different transcripts

The first transcript calculates:

- input: `11 + 2 + 3 = 16`;
- output: `5`.

The second transcript calculates:

- input: `101 + 20 + 30 = 151`;
- output: `50`.

Both distinct totals survive in invocation order, proving the second call did not replace the first.

### Two CaptureRecords

The test reads `.lisa/claude/captures.jsonl`, parses its non-empty lines as `CaptureRecord`, and requires exactly two values.

### Pane, session, and captured-at facts

The records assert:

- pane `42` on both rows;
- `session-first` on row one;
- `session-second` on row two;
- both `captured_at` values lie within epoch bounds surrounding the invocations.

The test intentionally does not require different timestamps because the shared contract uses epoch seconds and two valid captures can occur in one second. JSONL row order preserves invocation order.

### No ticket usage artifact

Both processes receive:

```text
LISA_TICKET_ID=T-STALE-FIRST-TICKET
```

The test asserts that `.lisa/claude/T-STALE-FIRST-TICKET.usage.json` does not exist after both calls. This is direct regression evidence for the recycled-pane defect.

### Ticket guess path removed

`resolve_key` was deleted. `capture_usage.rs` contains no `LISA_TICKET_ID` reference. There is no replacement ticket, pane, or `last` filename resolver.

## Test-first evidence

Before implementation, the new integration test failed because `.lisa/claude/captures.jsonl` did not exist. That is the expected old-writer behavior.

After implementation, the same command passed:

```text
1 passed; 0 failed
```

This before/after result demonstrates that the test is not merely restating behavior already provided by the core append helper.

## Focused test coverage

### CLI regression

```bash
cargo test -p lisa-cli --test capture_usage_cli
```

- 1 passed.
- 0 failed.
- Covers both child-process invocations and filesystem output.

### Capture parsing

```bash
cargo test -p lisa-cli capture_usage
```

- 5 capture module tests passed.
- 0 failed.
- Covers Claude input-class summation.
- Covers output summation.
- Covers malformed/non-assistant lines.
- Covers missing token fields.
- Covers empty/no-assistant input.
- Covers Codex latest cumulative token totals.

### Help surface

```bash
cargo test -p lisa-cli --test help_surface
```

- 3 passed.
- 0 failed.
- Confirms the command set remains intact.

## Broad test coverage

```bash
cargo fmt --all -- --check
cargo test --workspace
```

Result: passed with exit code 0.

Observed broad coverage includes:

- all lisa-cli library tests;
- all lisa-cli binary tests;
- all CLI integration tests;
- all lisa-core unit and integration tests;
- all 377 lisa-plugin tests;
- all workspace doc tests.

No external provider, model call, Zellij instance, network access, or billable token use was needed.

## Correctness assessment

### Attribution honesty

The writer persists only facts available at capture time. It does not attach a ticket ID. The scheduler-owned pane-time lookup can later perform attribution from durable ownership intervals.

Pane identity remains valid when a native client process is reused; inherited ticket identity does not. Removing the ticket read therefore fixes the false claim at its source.

### Data retention

The production writer delegates to the shared helper that opens with create-plus-append. The prerequisite core test already proves the first row remains byte-intact after a same-pane second append. The new CLI test additionally proves both product-level invocations produce two ordered rows.

### Schema alignment

The writer constructs the exact public core type, and the integration test deserializes actual file rows through that same type. This prevents local drift in field naming or nesting.

### Time alignment

`captured_at` uses the same epoch-second conversion as provenance and pane-time ownership lookup. No datetime dependency or local conversion convention was introduced.

### Provider behavior

Provider parser selection remains unchanged. Claude and Codex write distinct ledgers below their existing directories, avoiding transcript-format blending while keeping the shared row shape.

## Scope assessment

The change stays within the ticket boundary:

- no plugin attribution code changed;
- no scheduler ownership code changed;
- no core schema code changed;
- no hook template changed;
- no live hook changed;
- no hooks guide changed;
- no no-capture marker was invented;
- no cache-split expansion was introduced.

The old artifact compatibility unit test was appropriately removed because it tested a shape the command intentionally no longer emits. Parser coverage and the new shared-record integration coverage replace its useful portions.

## Gaps and limitations

### No-capture outcomes remain silent

Missing pane/session/transcript, unreadable transcript, malformed payload, or zero observed totals still return `Ok(())` without a row. This is explicitly the next dependent ticket T-043-02-02, which will introduce an operator-visible marker and remove hook stderr suppression.

### Plugin does not consume the new ledger yet

The plugin still reads old ticket-keyed usage files. S-043-03 owns replacement consumption, pane-time attribution, summation, and quarantine. This staged ticket makes the source data honest and durable but does not yet restore end-to-end provenance token population.

### Main still swallows append errors

`main` retains the existing best-effort hook behavior. The source function returns append errors, but command dispatch ignores them. The dependent loud-no-write ticket owns surfacing this class of failure; expanding here would overlap its acceptance boundary.

### Repeated cumulative Stops are not deduplicated

Each successful Stop becomes one observation. The ticket requires append behavior, and the later consumer owns interpretation and aggregation. No session/time deduplication policy was specified or introduced here.

### Concurrent append guarantees

The shared helper uses append handles and one framed row per call, matching the existing project ledger pattern. It does not add explicit inter-process locking. The acceptance case is sequential, and no stronger concurrency guarantee was requested by this ticket.

### Cache-class detail remains collapsed

Claude cache creation/read input is included in aggregate input tokens, as before. Provider cache-split parity is a separately named future concern and not part of this correctness slice.

## Workspace hygiene

All ticket-owned source paths are clean after the isolated commit:

- `crates/lisa-cli/src/capture_usage.rs` has no staged or unstaged change.
- `crates/lisa-cli/src/main.rs` has no staged or unstaged change.
- `crates/lisa-cli/tests/capture_usage_cli.rs` is tracked and clean.

No ticket-owned source file remains untracked.

Existing visible changes are Lisa-managed workflow state and published artifacts, not source owned by this ticket. They were not included in the source commit.

No ordinary `git add`, broad staging, or ordinary `git commit` was used.

## Open concerns requiring human attention

None critical for this ticket.

Reviewers should note the intentional staged interval: after this commit, new captures are retained honestly in `captures.jsonl`, while the current plugin reader still expects old ticket-keyed files. The dependency chain explicitly assigns consumption to S-043-03. Reintroducing a parallel legacy write for temporary compatibility would preserve the known false attribution and violate this ticket's acceptance criterion.

## Final assessment

The implementation removes both mechanisms behind the incident: stale ticket guessing and destructive overwrite. It replaces them with the prerequisite shared append-only contract, records every required honest fact, and proves behavior through the real CLI with the stale environment present. Verification is comprehensive and the committed scope is clean. The ticket is ready for Lisa's completion gate.
