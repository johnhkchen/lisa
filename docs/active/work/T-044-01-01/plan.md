# Plan: orient and separate help

## Objective

Change only the top-level help presentation so it opens with the everyday path
and visually separates machine-facing plumbing from operator-facing commands,
then lock that screen with black-box regression coverage.

## Preconditions

- Work from repository root `/Users/johnchen/swe/repos/lisa`.
- Preserve unrelated modified and untracked worktree files.
- Do not change ticket phase or status frontmatter.
- Write workflow artifacts only to the attempt work directory.
- Use `apply_patch` for source edits.
- Use `lisa commit-ticket` for each meaningful source unit.
- Never use ordinary `git add` or `git commit`.

## Step 1: establish the baseline

Run the focused existing help-surface integration test:

`cargo test -p lisa-cli --test help_surface`

Verification criteria:

- the current test binary builds;
- all existing help-surface tests pass before changes;
- any pre-existing failure is documented before implementation proceeds.

Capture the current non-interactive help output with:

`cargo run -q -p lisa-cli -- --help`

Verification criteria:

- no everyday path line is present;
- all four plumbing commands share the single generated command list;
- output matches the Research description.

## Step 2: add the orientation metadata

Edit `crates/lisa-cli/src/main.rs`.

Add this top-level `before_help` content to `Cli`:

`Everyday path: init → validate → status → loop`

Keep the current name, about line, and version metadata unchanged.

Verification criteria:

- `lisa --help` begins with the everyday-path line;
- the sequence names all four required commands in the required order;
- the current product about line remains present;
- command parsing is unchanged.

## Step 3: separate plumbing from the generated list

In the same source file, add `hide = true` to:

- `AgentExec`
- `CaptureUsage`
- `CommitTicket`
- `CompleteTicket`

Retain their existing `display_order` values.

Add a top-level `after_help` block containing:

- a plumbing heading explaining that Lisa and agent hooks call these commands;
- one row for each of the four plumbing commands;
- the existing concise description of each command.

Verification criteria:

- the generated command block still includes the five operator commands;
- it includes no plumbing command;
- a distinct plumbing footer includes all four plumbing names;
- the footer occurs below the generated options;
- already-hidden commands remain absent from top-level help;
- `lisa <plumbing-command> --help` still succeeds for all four commands.

## Step 4: format and inspect production output

Run:

`cargo fmt --all`

Then run:

`cargo run -q -p lisa-cli -- --help`

Inspect exact spacing, line wrapping, headings, and final newline.

If Clap renders an unexpected extra blank line or wraps a footer row, adjust
only the metadata string formatting needed for a legible one-screen result.
Record any material departure from the Design in `progress.md` before
continuing.

Verification criteria:

- output is readable in captured non-TTY mode;
- orientation is first;
- category boundary is explicit;
- no implementation or argument code changed.

## Step 5: run focused production checks

Run the existing help test before changing its expectations:

`cargo test -p lisa-cli --test help_surface`

Expected result:

- the about-line test may fail because orientation is now first;
- the old grouping test may still pass because it only checks relative order;
- this expected red state demonstrates why the test must change.

Also run direct help probes as needed:

- `lisa agent-exec --help`
- `lisa capture-usage --help`
- `lisa commit-ticket --help`
- `lisa complete-ticket --help`

Verification criteria:

- all probes exit successfully;
- hiding affects only the top-level listing.

## Step 6: commit the production help unit

Inspect the source diff for `main.rs` only.

Run:

`lisa commit-ticket --ticket-id T-044-01-01 --message "T-044-01-01: orient and separate top-level help" --include crates/lisa-cli/src/main.rs`

Verification criteria:

- Lisa reports a commit identifier;
- only `crates/lisa-cli/src/main.rs` is included;
- unrelated ordinary-index and worktree state remains untouched;
- `main.rs` is no longer modified after the transaction.

If the installed `lisa` binary does not support the required transaction or
the command reports a lease/ownership failure, document the exact failure and
resolve it without falling back to ordinary Git staging or commits.

## Step 7: add the full top-level snapshot

Edit `crates/lisa-cli/tests/help_surface.rs`.

Add an inline raw-string constant with the exact captured stdout.

Add `top_level_help_matches_snapshot` that compares `help_stdout(&["--help"])
with that constant using `assert_eq!`.

Verification criteria:

- the snapshot contains the orientation line;
- it contains the operator command block;
- it contains the separate plumbing heading and all four rows;
- no plumbing row appears inside the operator block;
- the expected string includes the exact final newline;
- the snapshot test passes against the real binary.

## Step 8: strengthen semantic grouping assertions

Update the old hook grouping test.

Split the complete help text at the exact plumbing heading.

Treat the prefix as primary/operator help and the suffix as plumbing help.

Assert:

- all five operator names have listing anchors in the prefix;
- all four plumbing names lack listing anchors in the prefix;
- all four plumbing names have listing anchors in the suffix;
- the three internal names lack listing anchors in full top-level help.

Verification criteria:

- deleting `hide = true` from any plumbing variant would fail this assertion;
- deleting a footer row would fail this assertion;
- deleting or renaming an operator row would fail this assertion;
- failure messages identify the misplaced or missing command.

## Step 9: adapt the about-line assertion

Update the existing jargon test so it locates the about sentence by its
`coding agents` content instead of selecting the first nonempty line.

Keep all existing jargon rules and operator direct-help checks.

Verification criteria:

- the new orientation is not mistaken for the about line;
- the actual about line remains positively anchored;
- no weakening of the jargon checks occurs.

## Step 10: run the focused test suite

Run:

`cargo fmt --all`

Run:

`cargo test -p lisa-cli --test help_surface`

Verification criteria:

- snapshot passes;
- structural grouping passes;
- all twelve direct command help probes pass;
- jargon checks pass;
- no warnings or unexpected stderr require action.

## Step 11: commit the regression-test unit

Inspect the diff for `help_surface.rs` only.

Run:

`lisa commit-ticket --ticket-id T-044-01-01 --message "T-044-01-01: snapshot the separated help surface" --include crates/lisa-cli/tests/help_surface.rs`

Verification criteria:

- Lisa reports a commit identifier;
- only the integration test path is included;
- the test file is clean after the transaction;
- unrelated worktree and index entries remain untouched.

## Step 12: run crate-wide verification

Run:

`cargo test -p lisa-cli`

This is the acceptance command and must include the new snapshot.

Then run:

`cargo fmt --all -- --check`

Verification criteria:

- every `lisa-cli` unit and integration test passes;
- format check passes;
- no snapshot is ignored or gated by a feature;
- runtime transaction, status, capture, and template tests remain green.

If a failure is unrelated and pre-existing, isolate it and document evidence.
If it is caused by the ticket, fix it in the appropriate ticket-owned source
unit and commit that exact path through another Lisa transaction.

## Step 13: inspect final source state

Run read-only checks:

- `git status --short`
- `git diff -- crates/lisa-cli/src/main.rs`
- `git diff -- crates/lisa-cli/tests/help_surface.rs`
- `git diff --cached -- crates/lisa-cli/src/main.rs`
- `git diff --cached -- crates/lisa-cli/tests/help_surface.rs`
- `git log --oneline` for the ticket commits.

Verification criteria:

- both ticket-owned source files are committed;
- neither source file is modified, staged, or untracked;
- only pre-existing unrelated state remains in the worktree;
- commit includes were exact.

## Step 14: update progress

Maintain `progress.md` throughout implementation.

It records:

- baseline results;
- production edit completion;
- exact help shape;
- production commit identifier;
- test edit completion;
- test commit identifier;
- focused and crate-wide test results;
- formatting result;
- deviations and their rationale;
- final cleanliness check.

## Step 15: Review

Write `review.md` in the attempt work directory.

Cover:

- files changed;
- user-visible help behavior;
- preserved parser/runtime behavior;
- snapshot and semantic test coverage;
- executed verification;
- open concerns, particularly static footer maintenance;
- whether the dependent ticket remains cleanly scoped.

Write `review-disposition.json` with exactly:

`{"disposition":"pass","reason":null}`

only if all acceptance and cleanliness checks pass.

If a blocking issue remains, write the exact valid block shape with a nonempty,
actionable reason instead.

After both Review artifacts exist, remain on this ticket and stop. Do not edit
ticket phase/status, publish to shared work, call completion, or begin the
dependent ticket.
