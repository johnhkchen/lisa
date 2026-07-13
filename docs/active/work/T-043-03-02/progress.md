# Progress: quarantine unattributable usage

## Status

Implementation is complete and source changes are committed.

Research, Design, Structure, and Plan artifacts were written to the private
attempt work directory and admitted by Lisa.

No ticket phase or status metadata was edited manually.

## Baseline verification

The predecessor behavior was verified before source changes.

Command:

```text
cargo test -p lisa-plugin ownership::tests
```

Result: pass, 2 passed, 0 failed.

Command:

```text
cargo test -p lisa-plugin provenance_recycled_pane_attributes_capture_sums_to_each_ticket
```

Result: pass, 1 passed, 0 failed.

This established a green ownership and recycled-pane baseline.

## Completed implementation work

Created `crates/lisa-plugin/src/quarantine.rs`.

The module now provides:

- a `QuarantinedCaptureRecord` envelope;
- stable 1-based capture-ledger source identity;
- an `AppendOutcome` distinguishing new and prior persistence;
- reversible path-safe session ID encoding;
- provider-local `quarantine/<session>.jsonl` paths;
- append-only writes;
- durable rescan idempotence;
- mismatch detection when one source line contains different capture data.

Registered the private module in `crates/lisa-plugin/src/lib.rs`.

Added module tests for:

- ordinary safe session IDs;
- traversal-shaped session IDs;
- slash and backslash encoding;
- dot and percent encoding;
- UTF-8 byte encoding;
- empty session identity;
- collision resistance for the empty sentinel;
- first append;
- exact rescan idempotence;
- preservation of two identical capture values from distinct source lines.

Ran formatting.

Command:

```text
cargo test -p lisa-plugin quarantine::tests
```

Result: pass, 2 passed, 0 failed.

## Plan deviation

The Plan proposed committing only `quarantine.rs` as the first source unit.

Rust does not compile or discover a module file until its parent declares it.

Without `mod quarantine;`, the focused tests would not run and the first commit
would contain unverified dead source.

The first atomic unit therefore includes:

- `crates/lisa-plugin/src/quarantine.rs`;
- the one-line `mod quarantine;` registration in
  `crates/lisa-plugin/src/lib.rs`.

This does not integrate quarantine into capture attribution yet.

The second unit will modify `lib.rs` again for runtime behavior and acceptance
coverage.

The deviation preserves the intended storage-before-integration ordering while
making the storage unit independently compiled and tested.

## Completed integration work

Changed `State::read_usage` to borrow mutable state so attribution can raise
activity events.

Capture ledger rows are enumerated before parse.

Malformed rows retain their prior skip behavior while later valid rows retain
stable physical source line numbers.

The consumer now branches explicitly on `owner_at`:

- the current ticket's capture enters checked token summation;
- another ticket's capture is skipped as attributable elsewhere;
- `None` enters session quarantine and never token summation.

Same-pane captures later than the current record's closed end are deferred.

This prevents a preloaded future ticket capture from being quarantined before
that future ownership interval is available.

Added `State::quarantine_capture`.

It selects the provider namespace, invokes the idempotent store, and logs:

- one `ActivityEvent::Warning` for a newly persisted quarantine;
- no repeated event for an already persisted source row;
- an `ActivityEvent::Error` for inspection, encoding, directory, or append
  failures.

The success warning includes provider, raw escaped session ID, pane, capture
timestamp, and destination path.

Added the ticket acceptance regression:

```text
provenance_unattributable_capture_is_quarantined_by_session_and_visible
```

The regression proves:

- a valid capture outside every known ownership interval returns null usage;
- the capture is preserved under its session-specific quarantine file;
- the envelope retains source line and every capture field;
- no provider-wide `quarantine.jsonl` exists;
- no `last` or `last.usage.json` fallback exists;
- the activity event converts to a dashboard warning;
- rescanning does not duplicate the durable row or warning.

Extended the recycled-pane regression to assert B's future captures do not
create quarantine while A is being closed.

## Focused verification

Command:

```text
cargo test -p lisa-plugin quarantine
```

Result: pass, 3 passed, 0 failed.

This filter includes two storage tests and the acceptance integration test.

Command:

```text
cargo test -p lisa-plugin owner_at
```

Result: pass, 2 passed, 0 failed.

Command:

```text
cargo test -p lisa-plugin provenance_recycled_pane_attributes_capture_sums_to_each_ticket
```

Result: pass, 1 passed, 0 failed.

Command:

```text
cargo test -p lisa-plugin provenance_codex_usage_flows_into_record
```

Result: pass, 1 passed, 0 failed.

Command:

```text
cargo test -p lisa-plugin provenance_claude_usage_flows_into_record
```

Result: pass, 1 passed, 0 failed.

The acceptance test was rerun immediately before the integration commit and
passed again.

## Broad verification

Command:

```text
cargo fmt --all -- --check
```

Result: pass.

Command:

```text
cargo test -p lisa-plugin
```

Result: pass, 381 passed, 0 failed, 0 ignored.

Command:

```text
cargo test --workspace
```

Result: pass across the workspace; the environment-dependent real Zellij test
remained its declared ignored test.

Command:

```text
just check
```

Result: pass.

The quick gate completed:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo test --workspace`.

## Ticket source commits

Storage module plus compiled registration:

```text
c7a05511a35ec8a192bae9a4b3033858944989db
feat(plugin): add session quarantine store
```

Exact included paths:

- `crates/lisa-plugin/src/quarantine.rs`;
- `crates/lisa-plugin/src/lib.rs`.

Consumer integration and acceptance coverage:

```text
309b282d1d3a3ac9a9e313871382663ce6bbb179
feat(plugin): quarantine unattributable captures
```

Exact included path:

- `crates/lisa-plugin/src/lib.rs`.

Both commits were created through `lisa commit-ticket`.

No ordinary index command was used.

## Remaining workflow work

Implementation has no remaining source step.

Review artifacts and disposition remain to be written.

## Worktree ownership note

Lisa currently owns concurrent changes under ticket metadata, provenance,
completion journal, and admitted work artifacts.

Those paths are not part of either ticket source commit.

No ordinary `git add`, `git add -A`, or `git commit` command has been used.

The ordinary index has no staged paths.

No ticket-owned source file remains modified or untracked.
