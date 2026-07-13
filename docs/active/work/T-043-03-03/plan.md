# Plan: field repro regression guard

## Implementation objective

Build one deterministic regression that connects the repaired native Stop
writer to the repaired plugin consumer and encodes six successive stale-key
overwrite opportunities. The test passes only when captures append as honest
pane/session/time facts, seven ticket intervals receive distinct usage, an
unowned capture is quarantined by session with visible activity, and an empty
Stop leaves a visible no-capture marker and diagnostic.

## Preconditions

1. Confirm the active ticket remains `T-043-03-03`.
2. Confirm Lisa-managed ticket/provenance changes are not treated as source
   ownership.
3. Confirm no ticket-owned source path is already modified or staged.
4. Preserve ordinary-index entries throughout all commits.
5. Use only `.lisa/attempts/T-043-03-03/1/work/` for phase artifacts.

## Step 1: expose feature-gated CLI test support

### Files

- `crates/lisa-cli/Cargo.toml`
- `crates/lisa-cli/src/lib.rs`
- `crates/lisa-plugin/Cargo.toml`

### Actions

1. Add a non-default empty `test-support` feature to `lisa-cli`.
2. Under that feature, export the existing `capture_usage` source module from
   the CLI library.
3. Enable `test-support` on the plugin's existing `lisa-cli` dev-dependency.
4. Do not change normal runtime dependencies.
5. Do not move or copy the capture module.

### Verification

Run:

```text
cargo check -p lisa-cli
cargo check -p lisa-cli --features test-support
cargo test -p lisa-plugin --no-run
```

Expected result: the normal CLI and feature-enabled library both compile, and
the plugin test target can resolve the module.

## Step 2: separate process acquisition from capture processing

### File

- `crates/lisa-cli/src/capture_usage.rs`

### Actions

1. Preserve `run_capture_usage(cwd)` as the command-facing adapter.
2. Move Stop payload reading and processing into a generic internal function.
3. Supply provider choice, validated pane ID, and epoch timestamp explicitly.
4. Supply the Stop payload through an `io::Read` parameter.
5. Supply visible diagnostics through an `io::Write` parameter.
6. Update `append_no_capture_marker` to accept the timestamp and diagnostic
   writer.
7. Replace direct `SystemTime::now()` in record construction with the supplied
   timestamp.
8. Replace `eprintln!` in marker persistence with `writeln!` to the supplied
   writer.
9. Keep diagnostic emission after successful marker persistence.
10. Keep every existing error kind and reason string.
11. Add a feature-gated doc-hidden public test wrapper delegating to the same
    internal function.
12. Keep parser unit tests and command behavior intact.

### Focused verification

Run:

```text
cargo test -p lisa-cli capture_usage
cargo test -p lisa-cli --test capture_usage_cli
```

Expected result:

- parser tests pass;
- compiled-command append test passes;
- compiled-command no-capture stderr test passes;
- no existing snapshot or path changes.

### Commit unit 1

Before committing:

1. Inspect `git diff` for the four support paths.
2. Confirm `crates/lisa-cli/src/main.rs` is unchanged unless compilation made a
   dispatch adjustment necessary.
3. Run `git status --short` and ensure no source path outside this unit is
   included.

Commit with:

```text
lisa commit-ticket \
  --ticket-id T-043-03-03 \
  --message "test(cli): expose deterministic capture support" \
  --include crates/lisa-cli/Cargo.toml \
  --include crates/lisa-cli/src/lib.rs \
  --include crates/lisa-cli/src/capture_usage.rs \
  --include crates/lisa-plugin/Cargo.toml
```

If one listed path has no diff, omit it from `--include`. Never include
`Cargo.lock` unless dependency resolution actually changes it; no new dependency
is planned.

## Step 3: add the combined field replay

### File

- `crates/lisa-plugin/src/lib.rs`

### Fixture construction

1. Add the test beside existing provenance usage regressions.
2. Create a temporary State with a real provenance ledger.
3. Point the scenario at the Claude provider directory because the incident was
   pane reuse through native Claude `/clear`.
4. Choose one pane ID not semantically significant to existing fixtures.
5. Define seven ticket IDs in chronological order.
6. Define deterministic interval starts and ends with gaps.
7. Define one capture timestamp inside each interval.
8. Give every session ID a ticket-specific suffix.
9. Give every transcript distinct, easily checked totals.
10. Use the test-support writer for each successful Stop.

### Six-overwrite representation

1. Treat ticket 1 as the process-birth ticket.
2. Treat tickets 2 through 7 as six pane recycles.
3. Do not pass a ticket identity to the capture processor.
4. Assert no `T-FIELD-01.usage.json` exists.
5. Assert the capture ledger retains all seven owned facts.
6. Document in the test comment that the old env-keyed writer would direct all
   six later observations to ticket 1 and overwrite the same file.

### Unattributable observation

1. Write a valid transcript with conspicuous large totals.
2. Use the same physical pane.
3. Use a timestamp earlier than all seven intervals.
4. Give it session `session-unattributable`.
5. Invoke the same capture processor so it becomes a real capture-ledger row.
6. Ensure it is early enough to be eligible for quarantine during the first
   closed interval scan.

### No-capture observation

1. Write an empty transcript file.
2. Build a payload with session `session-no-capture`.
3. Call the same capture processor with an explicit timestamp.
4. Capture diagnostics in a byte vector.
5. Assert the call succeeds only after marker persistence.
6. Assert diagnostics name the no-capture prefix, session, and
   `empty-transcript`.
7. Deserialize `no-captures.jsonl` through a test-local marker shape.
8. Assert exactly one row and exact pane/session/time/reason fields.
9. Assert no successful `CaptureRecord` uses the no-capture session.

### Capture-ledger assertions

1. Deserialize every physical row as `CaptureRecord`.
2. Assert exactly eight rows.
3. Assert the unowned row and seven owned rows remain in call order.
4. Assert every expected session is present once.
5. Assert every expected timestamp is exact.
6. Assert every expected token pair is exact.
7. Assert no legacy `.usage.json` fallback exists.

## Step 4: replay pane-time ownership

### Actions

1. Build the Claude `Route` once.
2. For ticket 1 through ticket 7, build a current null-usage execution record.
3. Use one attempt lease per ticket.
4. Use the common recycled pane ID.
5. Use the deterministic ownership interval.
6. Call `State::read_usage` before appending the current record.
7. Assert only the interval's expected capture contributes.
8. Fill tokens into the current record.
9. Append it through `provenance::append_record`.
10. Re-read the final ledger through the existing helper.
11. Compare ordered ticket IDs and token pairs for all seven rows.
12. Assert costs remain null.

### Overwrite guard

The final assertion must inspect all seven rows, not merely the last row or row
count. This proves every earlier ticket remains distinct after all six later
recycles.

## Step 5: assert quarantine and activity

### Actions

1. Resolve the unowned session path through `quarantine::session_path`.
2. Deserialize its rows as `QuarantinedCaptureRecord`.
3. Assert one row.
4. Assert its source line matches the unowned capture ledger row.
5. Assert its capture exactly matches the original unowned observation.
6. Assert no provider-wide `quarantine.jsonl` exists.
7. Count matching quarantine warnings in `state.activity_log`.
8. Assert exactly one warning names the unowned session.
9. Project that event through `activity_event_to_ui_entry`.
10. Assert the projected activity type is Warning and retains the session.
11. Assert all later attribution scans left quarantine row/warning counts at
    one.
12. Assert the conspicuous unowned totals appear in no provenance row.

## Step 6: run the new test red/green where practical

The prerequisites are already implemented, so a literal current-branch red
state is obtained by first adding the regression before the test-support seam is
complete or by observing compilation/assertion failure during construction.
The durable proof of old failure is structural: the predecessor implementation
at `ea9b295^` creates only `<key>.usage.json`, overwrites it, emits neither
capture nor marker ledgers, and returns no diagnostic for empty transcripts.

Run the completed focused test:

```text
cargo test -p lisa-plugin provenance_field_repro -- --nocapture
```

Expected result: exactly one matching test passes.

Run neighboring regressions:

```text
cargo test -p lisa-plugin provenance_recycled_pane
cargo test -p lisa-plugin provenance_unattributable
cargo test -p lisa-plugin provenance_claude_usage
```

Expected result: all prerequisite behavior remains green.

## Step 7: format and commit field test

1. Run `cargo fmt --all`.
2. Inspect formatting diffs and ensure no unrelated file changed.
3. Re-run the focused CLI and plugin tests.
4. Inspect `git diff -- crates/lisa-plugin/src/lib.rs`.
5. Commit only the test file:

```text
lisa commit-ticket \
  --ticket-id T-043-03-03 \
  --message "test(plugin): replay six usage overwrites" \
  --include crates/lisa-plugin/src/lib.rs
```

6. Confirm the ticket-owned file is clean after the isolated commit.

## Step 8: full verification

Run in order:

```text
cargo fmt --all -- --check
cargo test -p lisa-cli --test capture_usage_cli
cargo test -p lisa-plugin provenance_field_repro
cargo test --workspace
just check
```

Record command results, test counts, and any deviations in `progress.md`.

If `just check` repeats the workspace suite, retain both results because it also
checks the WASM target expected by project policy.

## Step 9: ownership and cleanliness audit

1. Run `git status --short`.
2. Run `git diff --name-only` for unstaged changes.
3. Run `git diff --cached --name-only` for ordinary-index entries.
4. Confirm no ticket-owned source path is modified, staged, or untracked.
5. Confirm Lisa-managed ticket/provenance/journal paths remain untouched by the
   ticket commits.
6. Inspect the two new commit IDs and their exact path lists.
7. Do not amend, squash, or ordinary-commit them.

## Step 10: progress and review artifacts

### `progress.md`

Record:

- the selected feature-gated seam;
- the seven-ticket/six-recycle topology;
- successful/no-capture writer outcomes;
- attribution, quarantine, and visibility assertions;
- every verification result;
- each Lisa commit ID and path set;
- deviations from this plan;
- final ticket-owned cleanliness.

### `review.md`

Summarize:

- all source paths changed;
- why the CLI refactor is production-neutral;
- how the test fails against old behavior;
- exact coverage of the acceptance criterion;
- test/gate results;
- remaining limitations, especially no paid live provider rerun;
- any human-review concerns.

### `review-disposition.json`

Write exactly:

```json
{"disposition":"pass","reason":null}
```

only if all source changes are committed and verification is green. Otherwise
write a blocking disposition with a non-empty actionable reason.

After both Review artifacts exist, remain on this ticket and stop. Lisa owns
publication, Done transition, completion commit, and seat release.
