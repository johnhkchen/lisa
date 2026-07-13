# Plan: CLI pre-ownership reconstruction

## Goal

Deliver a `lisa status` ticket mode that reconstructs retained pre-ownership
failure evidence from JSONL alone.
Preserve existing DAG status behavior and all unrelated worktree changes.

## Step 1: introduce the typed ledger reader

Modify `crates/lisa-cli/src/status.rs`.

Add imports for file reading, buffered JSONL iteration, typed core provenance
records, and writer-based rendering.

Implement a writer-based evidence function accepting a ledger path, ticket ID,
and output writer.

Open the file with an error that names the ledger.
Read every physical line with a one-based index.
Ignore blank lines.
Deserialize through `ProvenanceLedgerRecord`.
Return an actionable error on any malformed or unreadable row.

Verification:

- schema-v2 execution rows parse and are ignored;
- schema-v3 assignment rows parse;
- rows for other tickets are ignored;
- all matching rows are retained in append order.

## Step 2: implement deterministic rendering

In `status.rs`, add an exhaustive `AssignmentState` name mapping.

Render an explicit no-match line for a valid query with no matching rows.
Render a count heading and one evidence block per matching attempt otherwise.

Include:

- attempt ID;
- pane ID;
- named state;
- exact reason;
- provider;
- start epoch;
- end epoch;
- wall-clock seconds.

Validate the full ledger before writing so parse failures cannot leave partial
stdout that looks like a complete report.

Verification:

- expected output is exact and ends with a newline;
- multiple rows have a stable blank-line separator;
- output values come directly from retained records.

## Step 3: add status-module unit tests

Extend the existing `status.rs` test module.

Add a mixed-ledger filter test with execution, unrelated assignment, and
matching assignment content.
Assert exact report text.

Add a no-match test.
Assert exact no-evidence text.

Add a malformed-line test.
Assert the ledger path and one-based line number are in the error.
Assert the output buffer remains empty.

Run:

```text
cargo test -p lisa-cli status::tests
```

## Step 4: wire the Clap surface

Modify `crates/lisa-cli/src/main.rs`.

Extend `Commands::Status` with optional `--ticket` and `--ledger` values.
Declare `--ledger` dependent on `--ticket`.
Keep `--path` and display order unchanged.

In dispatch, resolve the project path first.
When ticket mode is selected, resolve an absolute or project-relative ledger
override and otherwise default to `.lisa/provenance.jsonl`.
Call the evidence entry point.

When ticket mode is absent, call the existing `run_status` path without output
or behavior changes.

Verification:

- `lisa status --help` documents both flags;
- `lisa status --ledger x` is rejected by Clap;
- ordinary `lisa status --path ...` still uses the DAG path.

## Step 5: commit the persisted fixture

Create
`crates/lisa-cli/tests/fixtures/preownership-ledger.jsonl`.

Write the literal compact T-040-02-01 schema-v3 assignment-transition row.
Retain deterministic attempt, pane, provider, named state, reason, and
timestamp values.
End the fixture with exactly one newline.

Verify it independently parses through the CLI test.
Do not generate it from current Rust structs during the test.

## Step 6: add the CLI-level regression

Create `crates/lisa-cli/tests/preownership_status.rs`.

Invoke the actual built Lisa binary.
Pass only `status`, the ticket ID, and the fixture path.
Do not construct a project, ticket directory, pane, signal, or scheduler.

Assert process success.
Assert stderr is empty.
Assert stdout exactly matches the retained evidence report.

Run:

```text
cargo test -p lisa-cli --test preownership_status
```

This is the primary acceptance-criterion test.

## Step 7: format and inspect the scoped diff

Run:

```text
cargo fmt --all
git diff --check -- crates/lisa-cli/src/main.rs crates/lisa-cli/src/status.rs crates/lisa-cli/tests/preownership_status.rs crates/lisa-cli/tests/fixtures/preownership-ledger.jsonl
git diff -- crates/lisa-cli/src/main.rs crates/lisa-cli/src/status.rs crates/lisa-cli/tests/preownership_status.rs crates/lisa-cli/tests/fixtures/preownership-ledger.jsonl
```

Confirm no formatter change lands in unrelated ticket-owned Rust files.
Because the worktree already contains foreign plugin changes, inspect status
before and after formatting and preserve them.

If workspace formatting changes the foreign plugin file, do not include it and
do not attempt destructive cleanup of another ticket's work.

## Step 8: run focused and crate verification

Run the status unit tests and new black-box test.
Then run the complete CLI crate suite:

```text
cargo test -p lisa-cli
```

This covers existing status behavior, command parsing, help command counts,
and the new fixture report.

Resolve any failure within the four ticket-owned paths.
Document any necessary plan deviation in `progress.md` before applying it.

## Step 9: run workspace verification

Run:

```text
cargo test --workspace
cargo fmt --all -- --check
```

The full workspace run verifies that new CLI use of core provenance remains
compatible with all core and plugin tests.
The real-Zellij test may remain ignored under its established environment gate.

If failures arise solely from concurrent unrelated dirty work, identify them
precisely and do not claim they are ticket regressions.

## Step 10: record implementation progress

Create and maintain
`.lisa/attempts/T-040-02-03/1/work/progress.md`.

Record each completed plan step, commands and results, deviations, exact owned
paths, and commit outcome.
Keep the artifact in the private attempt directory.
Do not publish it to `docs/active/work`.

## Step 11: isolated ticket commit

Commit the complete meaningful CLI unit with exactly:

```text
lisa commit-ticket \
  --ticket-id T-040-02-03 \
  --message "feat(cli): report pre-ownership failures" \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/src/status.rs \
  --include crates/lisa-cli/tests/preownership_status.rs \
  --include crates/lisa-cli/tests/fixtures/preownership-ledger.jsonl
```

Use the available Lisa binary path if `lisa` is not installed on `PATH`.
Do not use ordinary `git add`, `git commit`, or the ordinary index.

After the command, inspect the returned commit and verify it contains exactly
the intended files.
Run `git show --check` on the commit.
Verify all four ticket-owned paths are clean.

## Step 12: review

Create `review.md` summarizing:

- command behavior;
- file changes;
- typed parsing and error semantics;
- fixture-only CLI proof;
- test results;
- commit identity;
- remaining limitations or concerns;
- final ownership state.

Create `review-disposition.json` with exactly the valid pass or block shape.
Use pass only if implementation, tests, commit scope, and owned-path
cleanliness are all satisfactory.

Do not update ticket phase or status.
Do not start another ticket.

## Acceptance verification checklist

- `lisa status --ticket` is a real CLI command path.
- It reads pre-ownership rows through the shared mixed ledger type.
- It filters by the requested ticket.
- It prints stable named state.
- It prints the exact stored reason.
- It prints the stored provider.
- It prints start, end, and duration timestamps.
- It works against the committed fixture alone.
- The CLI-level test needs no live pane.
- Default DAG status remains unchanged.
- All ticket-owned files are committed through Lisa's isolated transaction.
- Unrelated worktree changes remain excluded and preserved.
