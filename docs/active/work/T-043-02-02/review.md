# Review — T-043-02-02 loud no-write signal

## Disposition

Ready to complete.

The acceptance criterion is implemented, tested at the compiled CLI boundary, committed through Lisa's isolated transaction, and verified across the workspace and WASM target.

No critical issue requires human intervention.

## What changed

Six ticket-owned tracked files changed in two meaningful commits.

No production file was created or deleted.

No core or plugin schema was changed.

## Commit summary

### `a6d9ac2f2c03d6e1c8b94a2daabfbcc790384851`

Subject:

```text
fix(cli): record visible no-capture outcomes
```

Paths:

- `crates/lisa-cli/src/capture_usage.rs`
- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/tests/capture_usage_cli.rs`

### `737c10a0ee3b9ce97ca1b420d4cda6f21f28d32d`

Subject:

```text
fix(cli): surface Stop hook capture failures
```

Paths:

- `crates/lisa-cli/src/templates.rs`
- `.lisa/hooks/on-stop.sh`
- `crates/lisa-cli/data/hooks-guide.md`

## Capture writer behavior

`run_capture_usage` now persists one honest outcome for every Stop whose pane and provider session are known.

Successful observations continue to append the existing shared `CaptureRecord` to:

```text
.lisa/<client>/captures.jsonl
```

Unsuccessful identified observations append a separate marker to:

```text
.lisa/<client>/no-captures.jsonl
```

The separate stream is important because every successful ledger row remains directly deserializable as `CaptureRecord`.

Downstream attribution work does not need to understand a mixed success/failure enum.

## No-capture marker

Each marker contains:

```text
pane_id
session_id
captured_at
reason
```

The timestamp uses the same epoch-second convention as successful captures and pane-time ownership history.

The reason values are stable and machine-readable:

- `missing-transcript-path`
- `unreadable-transcript`
- `empty-transcript`

Rows are compact, newline-terminated JSON and append rather than overwrite.

The marker intentionally excludes ticket identity.

The hook process still cannot truthfully know which ticket owned a recycled pane at capture time.

## Empty transcript semantics

`empty-transcript` means the provider parser observed no supported nonzero totals.

It covers a literal empty file and other no-observation inputs such as malformed-only or non-usage-only transcript content.

This preserves the prior “never fabricate measured zero” rule.

The writer does not create a successful record with zero token totals when observation failed.

## Missing and unreadable semantics

An absent or empty `transcript_path` produces `missing-transcript-path` after pane and session identity are established.

A supplied path that cannot be read produces `unreadable-transcript`.

The stable durable reason does not include raw OS error text or the transcript path.

This keeps rows deterministic and avoids retaining potentially sensitive machine-local paths.

## Identity failures

Malformed Stop JSON, missing session ID, and invalid `LISA_PANE_ID` cannot truthfully produce the required fully identified marker.

Those conditions now return an error rather than silently succeeding.

`CaptureUsage` dispatch prints the error and exits nonzero.

This is a strict improvement over the prior ignored result while preserving factual marker content.

## Persistence failures

Serialization, directory creation, file open, and row write errors propagate.

The stderr no-capture notice is emitted only after the marker row has been written successfully.

The command therefore never claims a durable marker exists when persistence failed.

## Immediate visibility

A successfully marked no-capture Stop emits a concise stderr line naming:

- pane;
- session;
- reason.

The JSONL row supplies durable visibility.

The stderr line supplies immediate attached-session visibility.

Expected no-capture outcomes still exit successfully after persistence, so the hook distinguishes a recorded absence from a broken capture command.

## Embedded hook

`templates::ON_STOP_HOOK` still writes the `.stopped` signal before attempting token capture.

It still reads stdin once and forwards the payload through `${LISA_BIN:-lisa}`.

It no longer contains:

```text
2>/dev/null
|| true
```

Real capture failures therefore remain visible and retain nonzero status.

## Live hook

The tracked `.lisa/hooks/on-stop.sh` was updated in the same commit.

Its executable mode is preserved.

A unit test reads the live file and requires exact equality with `ON_STOP_HOOK`.

This directly covers both surfaces named in acceptance instead of inferring live behavior from the generator.

## Safe hook upgrades

The formerly current silent capture hook is now an exact entry in `LEGACY_ON_STOP_HOOKS`.

The older pre-capture hook remains registered too.

As a result, `lisa init` can safely upgrade an unmodified deployed silent hook.

Existing unknown/customized hooks continue to be preserved by init's safety boundary.

Focused and package tests cover known-prior upgrade behavior.

## Operator documentation

The hooks guide now names both capture ledgers.

It explains the no-capture marker fields and reason role.

It explicitly says stderr and exit status are visible for malformed identity or persistence failure.

The guide does not overstate current behavior: plugin-side ticket attribution and quarantine remain deferred.

## Acceptance test coverage

The new integration test launches the compiled `lisa` binary twice against an isolated temporary root.

Invocation one supplies:

- pane 73;
- session `session-empty`;
- a zero-byte transcript.

Invocation two supplies:

- pane 73;
- session `session-unreadable`;
- a nonexistent transcript path.

The test requires:

- successful status after each marker append;
- visible stderr for each outcome;
- `empty-transcript` and `unreadable-transcript` reasons;
- absence of `captures.jsonl`;
- exactly two `no-captures.jsonl` rows;
- correct pane and distinct sessions;
- correct append order;
- capture timestamps within process invocation bounds.

This test failed against the old behavior before implementation because stderr was empty.

## Regression coverage

The preceding successful-capture integration test remains green.

It still proves two distinct successful Stops for one pane append two honest `CaptureRecord` rows without trusting inherited ticket identity.

All five existing provider parser tests remain green.

No successful schema field or token calculation changed.

## Template coverage

The Stop hook test proves:

- `.stopped` signal emission remains;
- `capture-usage` invocation remains;
- `LISA_BIN` fallback remains;
- stdin is read once;
- current stderr redirection is absent;
- current forced-success masking is absent;
- live and embedded hook bytes are identical.

The legacy hook intentionally still contains the old suffix as upgrade fixture data.

## Verification performed

### Formatting

```sh
cargo fmt --all -- --check
```

Passed.

### Focused compiled CLI

```sh
cargo test -p lisa-cli --test capture_usage_cli
```

Passed: 2 tests, 0 failures.

### Focused capture parser

```sh
cargo test -p lisa-cli capture_usage
```

Passed: 5 relevant unit tests, 0 failures.

### Focused template

```sh
cargo test -p lisa-cli \
  templates::tests::stop_hook_writes_stopped_and_keeps_capture_outcomes_visible
```

Passed.

### Focused upgrade

```sh
cargo test -p lisa-cli init::tests::test_run_init_upgrades_known_prior_hook
```

Passed.

### CLI package

```sh
cargo test -p lisa-cli
```

All active tests passed; the existing real-Zellij test remained environment-gated and ignored.

### Workspace

```sh
cargo test --workspace
```

All active tests passed with no failures.

### Project quick check

```sh
just check
```

Passed both:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo test --workspace`.

## Repository hygiene

All six ticket-owned source paths are clean after their isolated commits.

The ordinary Git index is empty.

No ticket-owned path is staged, modified, or untracked.

Unrelated runtime and concurrent T-043-03-01 changes remain untouched and excluded.

The ticket's phase/status frontmatter was not edited manually.

Artifacts were authored under the private attempt directory, not directly in the shared publication directory.

## Open concerns

### Cross-process append locking

No-capture marker writes use the same create/append model as successful capture rows.

There is no explicit cross-process file lock.

This ticket does not worsen the existing contract, but extremely concurrent writes rely on host append semantics.

If production evidence shows interleaving, both ledgers should receive one shared locked append primitive in a separate ticket.

### Timestamp granularity

Markers use epoch seconds.

Multiple Stops in one second can share a timestamp; JSONL row order still preserves append order in the tested single-process sequence.

This matches the successful capture and ownership lookup time unit.

### Missing-path integration branch

The production path for `missing-transcript-path` is implemented but the compiled regression specifically exercises empty and unreadable transcripts, matching the ticket's explicit test criterion.

A future contract table test could add malformed payload, missing session, invalid pane, and missing path cases.

Those gaps do not block the requested acceptance behavior.

### Downstream surfacing

The writer and hook make the event durable and immediately visible.

The plugin does not yet ingest no-capture rows into its activity log.

That end-to-end surfacing belongs to the explicitly dependent S-043-03 work and was not absorbed here.

## Out-of-scope behavior preserved

- No ticket attribution at capture time.
- No session-keyed quarantine.
- No plugin activity event.
- No cache-dimension parity change.
- No successful capture schema change.
- No transcript parser semantic change.
- No live metered provider run.

## Final assessment

The silent identified no-write path is removed.

Empty and unreadable transcripts now leave durable pane/session/reason evidence and an immediate stderr notice.

Unrepresentable or unwritable outcomes now fail visibly.

Both shipped and live hooks expose Lisa stderr and status.

Upgrade compatibility, successful capture behavior, and workspace health are preserved.

