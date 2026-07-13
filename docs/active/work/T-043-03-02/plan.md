# Plan: quarantine unattributable usage

## Objective

Persist every eligible valid capture that `owner_at` cannot uniquely assign in
a provider-local file keyed by its session ID, and raise a dashboard-visible
warning without adding that usage to any ticket or shared fallback.

## Preconditions

- `T-043-03-01` has landed the capture-ledger consumer.
- `CaptureRecord` includes pane, session, capture time, and token totals.
- Provider capture ledgers are append-only JSONL files.
- `owner_at` returns `None` for missing or conflicting pane-time evidence.
- `emit_provenance` constructs the current interval before `read_usage`.
- `read_usage` scans prior execution rows plus the current interval.
- Generic warning activity already renders in the dashboard.
- Provider directories are ignored machine state.
- Ticket phase/status metadata is Lisa-owned during this attempt.

## Step 1: record baseline repository state

Inspect `git status --short` before source edits.

Treat changes to ticket metadata, provenance, completion journal, and admitted
phase artifacts as Lisa-owned concurrent state.

Do not stage, revert, or include those paths.

Run the focused existing attribution tests:

```text
cargo test -p lisa-plugin ownership::tests
cargo test -p lisa-plugin provenance_recycled_pane_attributes_capture_sums_to_each_ticket
```

Expected verification:

- ownership confidence semantics pass before modification;
- recycled-pane attribution and summation pass before modification;
- any unrelated baseline failure is recorded before proceeding.

## Step 2: create the quarantine storage module

Create `crates/lisa-plugin/src/quarantine.rs` with module documentation.

Import `CaptureRecord`, filesystem append support, paths, and Serde derives.

Define `QuarantinedCaptureRecord` with:

- 1-based `source_line: u64`;
- original `capture: CaptureRecord`.

Define typed `AppendOutcome` cases for new and prior persistence.

Implement a private byte-to-hex helper or equivalent constant table.

Implement reversible session ID encoding:

- retain ASCII letters and digits;
- retain hyphen and underscore;
- percent-encode all other bytes;
- map empty input to an encoded sentinel;
- never admit slash, backslash, dot, or percent literally.

Implement `session_path(provider_dir, session_id)`.

Expected verification:

- safe session IDs have recognizable filenames;
- unsafe IDs cannot escape the quarantine directory;
- distinct session byte strings map to distinct filenames.

## Step 3: implement idempotent append

Implement `append(provider_dir, source_line, capture)`.

Derive the per-session destination.

Read an existing file if present.

Parse existing lines as quarantine envelopes.

Return `AlreadyPresent` if the same source line already exists.

Propagate non-not-found read failures.

Serialize a compact envelope and newline.

Create the parent directory.

Open with create plus append.

Write the full line and return `Appended(path)`.

Do not truncate, rewrite, rename, or delete any existing row.

Expected verification:

- first append creates one parseable row;
- repeat of the same source line changes no bytes;
- a different source line appends even when capture values are identical.

## Step 4: add storage module tests

Add a filename safety/uniqueness unit test.

Use traversal-shaped, Unicode, percent, dot, and empty session IDs.

Check `path.parent()` equals `<provider>/quarantine` in every case.

Add an append/idempotence unit test.

Read and parse the file after each operation.

Compare the full original capture.

Compare bytes before and after an idempotent repeat.

Append source line 2 with an identical capture and assert two rows.

Run:

```text
cargo test -p lisa-plugin quarantine::tests
```

Expected verification: all new storage tests pass.

## Step 5: format and commit the storage unit

Run `cargo fmt --all -- --check`.

If formatting reports changes needed, run `cargo fmt --all`, inspect the exact
diff, and ensure no unrelated source file was modified.

Inspect:

```text
git diff -- crates/lisa-plugin/src/quarantine.rs
git status --short
```

Commit only the new module through Lisa:

```text
lisa commit-ticket \
  --ticket-id T-043-03-02 \
  --message "feat(plugin): add session quarantine store" \
  --include crates/lisa-plugin/src/quarantine.rs
```

Expected verification:

- Lisa reports a successful isolated commit;
- the new module is no longer untracked;
- no ordinary index command is used;
- Lisa-owned concurrent paths remain untouched.

## Step 6: register and orchestrate quarantine

Modify `crates/lisa-plugin/src/lib.rs`.

Register `mod quarantine;` near the other private modules.

Import `CaptureRecord` for the new state helper and parser.

Add `State::quarantine_capture` beside provenance usage methods.

Clone/select the provider directory based on `AgentClient`.

Call the storage append function.

On `Appended`, log one warning containing:

- client;
- raw session ID;
- pane ID;
- capture timestamp;
- destination path.

On `AlreadyPresent`, log nothing.

On error, log a visible error with capture identity and I/O context.

Expected verification:

- the storage module stays independent of activity/UI types;
- activity behavior stays in mutable scheduler state.

## Step 7: branch explicitly in `read_usage`

Change the method receiver to `&mut self`.

Keep provider selection, ledger reads, and return tuple unchanged.

Enumerate source lines before parsing.

Skip malformed rows.

Skip other-pane rows.

Skip same-pane rows after `current.ended_at` as pending ownership evidence.

Call `owner_at` once for each remaining capture.

For the current ticket, execute existing checked token addition.

For another ticket, continue without persistence or totals.

For `None`, call `quarantine_capture` and continue.

Expected verification:

- no unmatched capture changes token totals;
- no attributable other-ticket capture enters quarantine;
- future preloaded rows do not enter quarantine prematurely;
- checked overflow behavior remains unchanged.

## Step 8: add the acceptance regression

Add a `T-043-03-02`-named test beside provenance usage tests.

Use one Codex capture on the current pane.

Place its timestamp before the current record's start and before its end.

Verify `read_usage` returns null tokens and cost.

Verify the exact per-session quarantine file exists.

Parse one `QuarantinedCaptureRecord` and assert:

- `source_line == 1`;
- the embedded capture equals the source capture.

Verify no provider-root `quarantine.jsonl` exists.

Verify no `last` or `last.usage.json` file exists.

Find the one warning whose text identifies quarantine and the session.

Convert it through `activity_event_to_ui_entry`.

Match `ui::ActivityType::Warning` to prove dashboard visibility.

Invoke `read_usage` a second time.

Verify one quarantine row and one matching warning still exist.

Expected verification: the acceptance test fails against the predecessor
consumer and passes with this ticket's integration.

## Step 9: run focused regression gates

Run:

```text
cargo test -p lisa-plugin quarantine
cargo test -p lisa-plugin owner_at
cargo test -p lisa-plugin provenance_recycled_pane_attributes_capture_sums_to_each_ticket
cargo test -p lisa-plugin provenance_codex_usage_flows_into_record
cargo test -p lisa-plugin provenance_claude_usage_flows_into_record
```

Expected verification:

- storage safety and idempotence pass;
- ownership confidence semantics pass;
- A/B future-preload behavior remains correct;
- provider routing remains correct;
- owned usage totals remain unchanged.

## Step 10: run broader verification

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin
cargo test --workspace
```

Run the repository quick gate if time/environment permit:

```text
just check
```

Record exact commands, pass/fail counts, and any environmental limitation in
`progress.md` and `review.md`.

Do not hide unrelated failures.

## Step 11: inspect and commit integration

Inspect only the ticket source diff:

```text
git diff -- crates/lisa-plugin/src/lib.rs
git status --short
```

Confirm `quarantine.rs` is already committed and `lib.rs` is the only remaining
ticket-owned source change.

Commit through Lisa:

```text
lisa commit-ticket \
  --ticket-id T-043-03-02 \
  --message "feat(plugin): quarantine unattributable captures" \
  --include crates/lisa-plugin/src/lib.rs
```

Expected verification:

- no ticket-owned source remains staged, modified, or untracked;
- ordinary-index state is not consumed;
- phase artifacts remain Lisa-owned and outside source commits.

## Step 12: complete progress and review

Write `progress.md` in the attempt-private work directory.

Record completed steps, commits, tests, and deviations before Review.

Inspect final source commits and status.

Write `review.md` summarizing:

- files created and modified;
- storage and ownership behavior;
- activity visibility;
- test coverage;
- known limitations and deferred 03-03 work;
- final worktree ownership state.

Write `review-disposition.json` with exactly:

```json
{"disposition":"pass","reason":null}
```

Use a block disposition instead if a ticket-owned defect or required gate
remains unresolved.

After both Review artifacts exist, remain on this ticket and stop.
