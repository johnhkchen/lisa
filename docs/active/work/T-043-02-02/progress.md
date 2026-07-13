# Progress — T-043-02-02 loud no-write signal

## Status

Implementation is complete.

All ticket-owned source changes are committed through Lisa's isolated transaction.

No ticket-owned source path remains staged, modified, or untracked.

## Acceptance outcome

Identified Stops that cannot produce token totals now leave an append-only, operator-visible marker instead of silently returning without a trace.

The marker destination is provider-specific:

```text
.lisa/claude/no-captures.jsonl
.lisa/codex/no-captures.jsonl
```

Each compact JSONL row carries:

- `pane_id`
- `session_id`
- `captured_at`
- `reason`

The embedded and live Stop hooks no longer redirect Lisa stderr to `/dev/null` or force a successful shell status.

## Red test evidence

The compiled CLI regression was added before production changes.

Command:

```sh
cargo test -p lisa-cli --test capture_usage_cli \
  empty_and_unreadable_transcripts_append_visible_no_capture_markers
```

Pre-implementation result:

- test failed as expected;
- failure was at the stderr visibility assertion;
- the existing command returned success silently;
- no `lisa capture-usage: no capture` message existed.

This directly characterized the old behavior rather than relying on a unit-only helper.

## Capture outcome implementation

Modified `crates/lisa-cli/src/capture_usage.rs`.

Added a private serialized `NoCaptureMarker` carrying the four required observation facts.

Added stable reason strings:

- `missing-transcript-path`
- `unreadable-transcript`
- `empty-transcript`

`empty-transcript` includes every case where provider parsing observes no nonzero recognized totals:

- zero-byte transcript;
- blank-only transcript;
- malformed-only transcript;
- non-usage events only;
- recognized usage records whose supported totals remain zero.

This preserves the established rule that the writer does not fabricate a measured zero.

## Append behavior

Added `append_no_capture_marker`.

The helper:

1. timestamps the observation in UTC epoch seconds;
2. serializes compact JSON;
3. creates the provider directory if needed;
4. opens `no-captures.jsonl` with create and append;
5. writes one newline-terminated row;
6. emits a concise stderr notice after persistence succeeds;
7. propagates serialization and filesystem failures.

Existing successful `CaptureRecord` persistence remains unchanged.

Successful rows still append only to `captures.jsonl`.

Failure rows never enter the successful homogeneous stream.

## Control-flow changes

`run_capture_usage` now establishes identity before transcript classification.

It requires:

- valid Stop JSON;
- nonempty provider session ID;
- valid numeric `LISA_PANE_ID`.

Those facts are necessary to form the ticket-required marker.

Malformed payload, missing session, and invalid pane now return actionable `std::io::Error` values instead of silent `Ok(())` results.

Once identity is known:

- absent or empty transcript path appends `missing-transcript-path`;
- transcript read failure appends `unreadable-transcript`;
- no observed totals appends `empty-transcript`;
- observed totals append the existing successful `CaptureRecord`.

Every identified Stop therefore persists exactly one outcome row.

## CLI error boundary

Modified `crates/lisa-cli/src/main.rs`.

`Commands::CaptureUsage` no longer assigns the writer result to `_`.

It now:

- prints `Error: <detail>` to stderr;
- exits with status 1 when payload identity or persistence fails.

Expected no-capture outcomes return success after their marker has been written.

## Compiled CLI regression

Modified `crates/lisa-cli/tests/capture_usage_cli.rs`.

The process helper now returns complete `Output`, allowing status and stderr assertions.

The new test invokes the compiled `lisa` binary twice:

1. one Stop points at an empty transcript;
2. one Stop points at a nonexistent transcript.

Both use pane 73 and distinct session IDs.

Assertions prove:

- both commands succeed after marker persistence;
- stderr contains the no-capture notice;
- stderr contains the correct stable reason;
- no successful capture ledger is created;
- exactly two failure rows append;
- both rows carry pane 73;
- rows carry their distinct sessions;
- reasons preserve invocation order;
- timestamps fall within the invocation bounds.

The preceding successful two-capture regression remains green.

## Embedded Stop hook

Modified `crates/lisa-cli/src/templates.rs`.

Current capture invocation is now:

```sh
printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage
```

Removed from the current hook:

- `2>/dev/null`
- `|| true`

The hook still:

- creates the signal directory;
- writes `.stopped` before capture;
- reads stdin once;
- uses `LISA_BIN` with PATH fallback;
- forwards the complete Stop payload.

Rust and shell comments now describe append-only successful/no-capture outcomes and visible errors.

## Live Stop hook

Modified `.lisa/hooks/on-stop.sh`.

The live checked-in script exactly equals `templates::ON_STOP_HOOK`.

Its executable mode remains `100755`.

The template unit test reads the live file and requires byte equality, proving both acceptance surfaces remain synchronized.

## Upgrade compatibility

Expanded `LEGACY_ON_STOP_HOOKS`.

It now retains:

1. the immediately preceding capture hook containing stderr/status suppression;
2. the older pre-capture v0.3 hook.

This means `lisa init` upgrades an uncustomized deployed silent hook to the visible generation.

Unknown project-owned hooks remain protected by existing safety behavior.

The focused known-prior upgrade test passed.

## Operator guide

Modified `crates/lisa-cli/data/hooks-guide.md`.

The guide now names:

- `.lisa/<client>/captures.jsonl` for successful observations;
- `.lisa/<client>/no-captures.jsonl` for identified failures;
- marker pane/session/time/reason facts;
- intentional stderr and exit-status visibility for genuine errors.

It does not claim plugin attribution or quarantine behavior that belongs to S-043-03.

## Commit 1

Command:

```sh
lisa commit-ticket \
  --ticket-id T-043-02-02 \
  --message "fix(cli): record visible no-capture outcomes" \
  --include crates/lisa-cli/src/capture_usage.rs \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/tests/capture_usage_cli.rs
```

Commit:

```text
a6d9ac2f2c03d6e1c8b94a2daabfbcc790384851
```

## Commit 2

Command:

```sh
lisa commit-ticket \
  --ticket-id T-043-02-02 \
  --message "fix(cli): surface Stop hook capture failures" \
  --include crates/lisa-cli/src/templates.rs \
  --include .lisa/hooks/on-stop.sh \
  --include crates/lisa-cli/data/hooks-guide.md
```

Commit:

```text
737c10a0ee3b9ce97ca1b420d4cda6f21f28d32d
```

## Targeted verification

```sh
cargo test -p lisa-cli --test capture_usage_cli
```

Result:

- 2 passed;
- 0 failed.

```sh
cargo test -p lisa-cli capture_usage
```

Result:

- 5 capture parser unit tests passed;
- filtered non-capture tests did not run in this focused command.

```sh
cargo test -p lisa-cli \
  templates::tests::stop_hook_writes_stopped_and_keeps_capture_outcomes_visible
```

Result:

- 1 passed;
- embedded/current hook has no suppression;
- live hook equals embedded hook.

```sh
cargo test -p lisa-cli init::tests::test_run_init_upgrades_known_prior_hook
```

Result:

- 1 passed;
- previous known hook upgraded to current.

## Package verification

```sh
cargo test -p lisa-cli
```

Result:

- all active package tests passed;
- real-Zellij delivery test remained ignored by its existing environment gate;
- no failures.

## Workspace verification

```sh
cargo test --workspace
```

Result:

- all active workspace tests passed;
- CLI, core, and plugin suites passed;
- plugin suite reported 378 passed;
- no failures.

```sh
just check
```

Result:

- `cargo check -p lisa-plugin --target wasm32-wasip1` passed;
- repeated `cargo test --workspace` passed;
- no failures.

## Suppression search

The focused search found one remaining `capture-usage 2>/dev/null || true` occurrence.

It is inside `LEGACY_ON_STOP_HOOKS`, where the exact previous bytes are intentionally required for upgrade recognition.

Neither current embedded hook nor live hook contains stderr redirection or status masking.

## Deviations from plan

No behavioral deviation was required.

One verification strengthening was added during implementation: the template test reads `.lisa/hooks/on-stop.sh` and asserts exact equality with `ON_STOP_HOOK` rather than checking only the embedded constant.

The plan described six modified tracked paths but numbered the overview list as five; implementation correctly changed all six named paths.

## Repository hygiene

The following ticket-owned paths are clean:

- `crates/lisa-cli/src/capture_usage.rs`
- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/tests/capture_usage_cli.rs`
- `crates/lisa-cli/src/templates.rs`
- `.lisa/hooks/on-stop.sh`
- `crates/lisa-cli/data/hooks-guide.md`

The ordinary index contains no staged paths.

Unrelated runtime and concurrent-ticket changes remain visible in repository status and were not included:

- `.lisa/provenance.jsonl`
- `.lisa/completion-journal.jsonl`
- Lisa-managed ticket frontmatter
- concurrent T-043-03-01 plugin/work changes

## Remaining work

Only Review artifacts remain for this ticket.
