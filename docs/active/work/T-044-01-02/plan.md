# Plan: verb-forward command help and examples

## Objective

Add one concrete `Example:` line to each of the five operator command help
screens, preserve their existing plain imperative purposes, and extend the
black-box help regression suite to pin both pieces of copy.

## Step 1: establish the baseline

1. Run the focused `help_surface` integration test.
2. Confirm it passes before ticket edits.
3. Capture each current `lisa <cmd> --help` output.
4. Confirm each current first line matches the source summary.
5. Confirm no current command contains `Example:`.

Verification:

- `cargo test -p lisa-cli --test help_surface` succeeds.
- Five direct help invocations succeed.
- The observed gap matches Research.

## Step 2: add help metadata

1. Edit only `crates/lisa-cli/src/main.rs`.
2. Expand `Init`'s command attribute with its concrete example.
3. Expand `Validate`'s command attribute with its concrete example.
4. Expand `Status`'s command attribute with its concrete example.
5. Expand `Doctor`'s command attribute with its concrete example.
6. Expand `Loop`'s command attribute with its concrete example.
7. Do not edit variant payloads or match dispatch.
8. Run Rust formatting.

Verification:

- `cargo fmt --all -- --check` succeeds after formatting.
- Each direct command help ends with exactly one `Example:` line.
- The five purpose lines remain the first rendered content.
- `lisa --help` still matches the predecessor snapshot.

## Step 3: commit help metadata

1. Inspect the diff for `crates/lisa-cli/src/main.rs`.
2. Confirm only five command attributes changed.
3. Run the focused test; it should still pass because existing tests do not yet
   require examples and variant after-help does not alter top-level help.
4. Commit through Lisa's isolated transaction.

Command shape:

```text
lisa commit-ticket \
  --ticket-id T-044-01-02 \
  --message "T-044-01-02: add operator command examples" \
  --include crates/lisa-cli/src/main.rs
```

Verification:

- The isolated commit succeeds.
- `main.rs` is no longer modified.
- Unrelated ordinary worktree entries remain untouched.

## Step 4: build command-specific snapshots

1. Edit only `crates/lisa-cli/tests/help_surface.rs`.
2. Update the module contract comment to name the new property.
3. Add a small snapshot record type.
4. Add five inline expected stdout strings.
5. Preserve the canonical operator order.
6. Include exact generated option spacing and trailing newlines.
7. Add a consistency assertion between the operator list and snapshot list.
8. Add a looped exact-output test with command-specific diagnostics.
9. Retain the existing jargon scan over full command output.

Verification:

- The new test observes all five commands.
- Removing any one `Example:` footer would fail its exact comparison.
- Removing or changing any purpose line would fail its exact comparison.
- Introducing a banned term would fail the separate jargon assertion even if
  expected copy were updated.

## Step 5: run focused verification

1. Run formatting.
2. Run the help-surface integration test.
3. If expected spacing differs from actual Clap output, inspect actual stdout.
4. Adjust snapshots only to match intended rendered structure.
5. Re-run until clean.
6. Run direct help invocations for visual inspection.

Commands:

```text
cargo fmt --all -- --check
cargo test -p lisa-cli --test help_surface
cargo run -q -p lisa-cli -- init --help
cargo run -q -p lisa-cli -- validate --help
cargo run -q -p lisa-cli -- status --help
cargo run -q -p lisa-cli -- doctor --help
cargo run -q -p lisa-cli -- loop --help
```

Verification criteria for every command:

- exit status is zero;
- purpose is the first nonempty line;
- purpose begins with Set, Check, Show, Check, or Start as designed;
- no banned jargon appears;
- usage names the correct subcommand;
- example begins with `Example: lisa`;
- example names the correct subcommand;
- example contains actual values rather than Clap metavariables.

## Step 6: commit regression coverage

1. Inspect the test-file diff.
2. Confirm no production behavior was added to the test.
3. Confirm the predecessor top-level snapshot remains unchanged.
4. Commit only the integration test through Lisa.

Command shape:

```text
lisa commit-ticket \
  --ticket-id T-044-01-02 \
  --message "T-044-01-02: snapshot operator command help" \
  --include crates/lisa-cli/tests/help_surface.rs
```

Verification:

- The isolated commit succeeds.
- The test file is clean afterward.
- No ticket-owned source file is staged, modified, or untracked.

## Step 7: crate-level verification

1. Run every `lisa-cli` test.
2. Investigate any failure for connection to help metadata.
3. Do not alter unrelated source to mask a pre-existing failure.

Command:

```text
cargo test -p lisa-cli
```

Verification:

- All CLI unit and integration tests pass.
- The new help snapshots run within that suite.

## Step 8: workspace verification

1. Run the repository's quick check if its required target/tooling is present.
2. Run the full workspace test suite.
3. Record command results and any environment limitation in `progress.md`.

Commands:

```text
just check
cargo test --workspace
```

Verification:

- The WASM check and native workspace tests pass, or an environmental failure
  is documented precisely.
- No test failure is ignored.

## Step 9: source cleanliness audit

1. Inspect `git status --short`.
2. Distinguish pre-existing unrelated state from ticket-owned paths.
3. Inspect recent commits for the two ticket source units.
4. Confirm no ordinary index operation was used.
5. Confirm the two exact source paths are clean.

Verification:

- `crates/lisa-cli/src/main.rs` is clean.
- `crates/lisa-cli/tests/help_surface.rs` is clean.
- Unrelated Lisa state, epic, story, and ticket entries remain as found.

## Step 10: progress artifact

Maintain `.lisa/attempts/T-044-01-02/1/work/progress.md` during implementation.
Record:

- completed metadata changes;
- completed test changes;
- each isolated source commit and hash;
- focused, crate, workspace, and formatting results;
- deviations from this plan;
- remaining work until Review.

## Step 11: Review

1. Inspect the final committed diff for both source files.
2. Summarize the five help examples.
3. Explain positive snapshot and negative jargon coverage.
4. List all verification commands and outcomes.
5. State open concerns or limitations.
6. Write `review.md` in the attempt work directory.
7. Write `review-disposition.json` with the exact valid shape.
8. Use pass only if source is committed, clean, and verification succeeds.
9. Remain on the current ticket after both artifacts exist.

Expected disposition when all criteria pass:

```json
{"disposition":"pass","reason":null}
```

## Atomicity summary

- Source unit 1: five production help metadata additions in `main.rs`.
- Source unit 2: five command snapshots and the new assertion in
  `help_surface.rs`.
- Artifacts are not mixed into source commits.
- Each source commit has one exact include path.
- The plan creates no cross-ticket ownership ambiguity.
