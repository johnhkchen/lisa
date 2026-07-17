# Plan: benefit-first context and run summary

## Preconditions

Preserve all unrelated modified and untracked files.

Do not update ticket frontmatter phase or status.

Write phase artifacts only in the private attempt directory.

Use `lisa commit-ticket` for every ticket-owned source unit.

## Step 1: add canonical purpose copy

Create `crates/lisa-core/src/context.rs`.

Define the exact T-046-07-02 paragraph once.

Export the module from `lisa-core/src/lib.rs`.

Verification: targeted core compilation and string equality in consumers.

## Step 2: render generated project context from the shared copy

Update `templates.rs` to interpolate `PURPOSE_PARAGRAPH`.

Change Agents generation to an owned-string function.

Update `init.rs`, setup tests, and template tests for the function.

Remove every Rust test-local retyping of the paragraph.

Verification: template unit tests pass.

## Step 3: put benefit first in workflow and assignment context

Prefix the embedded workflow Markdown with the canonical paragraph text.

Apply the same bytes to the repository workflow document.

Prefix plugin `ticket_prompt` with the shared constant.

Add ordering tests for `DAG`, `phase`, `scheduling`, and `Zellij`.

Require every present term to follow the purpose paragraph.

Verify the embedded and repository workflow files remain byte-identical.

## Step 4: commit the context unit

Run formatting for changed Rust files.

Run targeted template and adapter tests.

Inspect the diff for accidental workflow-rule changes.

Commit exact paths with:

```text
lisa commit-ticket --ticket-id T-047-01-01 \
  --message "T-047-01-01: lead session context with Lisa's purpose" \
  --include crates/lisa-core/src/context.rs \
  --include crates/lisa-core/src/lib.rs \
  --include crates/lisa-cli/src/templates.rs \
  --include crates/lisa-cli/src/init.rs \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-cli/data/rdspi-workflow.md \
  --include docs/knowledge/rdspi-workflow.md
```

Do not include active ticket or attempt artifacts.

## Step 5: add durable latest-run boundaries

Create `run_summary.rs`.

Define versioned baseline serialization.

Create/retain the append-only event ledger.

Measure both byte offsets immediately before launch.

Write baseline via same-directory temporary and rename.

Test first run, existing ledgers, and invalid baseline behavior.

## Step 6: retain interactive-gate facts

Update the generated question hook command to append a static question event.

Update the permission/attention hook command to append a static permission
event even when the optional notification script is absent.

Do not retain payload data.

Keep idle-prompt filtering before permission event append.

Keep the `.awaiting` signal behavior unchanged.

Update hook generation and merge tests.

## Step 7: collect summary facts

Accept current board tickets and configured work path.

Count total, done, and remaining tickets from ticket phases.

Read only provenance bytes after the baseline offset.

Filter rows to current ticket IDs.

Count explicit failed and timed-out outcomes.

Read only event bytes after the baseline offset.

Count manual-intervention rows by kind.

Treat missing, malformed, or offset-invalid evidence as unknown.

Check each displayable evidence path independently.

## Step 8: render the narrative

Add a writer-based rendering function.

Print the unattended-win sentence only for complete, no-failure, zero-gate
facts that are all positively known.

Print exact completed/total counts for every nonempty board.

Print exact remaining count for partial boards.

Print failed/timed-out counts only from explicit rows.

Print `Manual approvals requested: 0` only from an empty tracked event segment.

Print an intervention count when any gate fired.

Print only existing evidence paths.

## Step 9: add fixture-run tests

Build a clean two-ticket fixture with valid latest-run evidence.

Assert the benefit sentence, 2-of-2 count, zero approvals, and three evidence
paths.

Build a failed partial fixture.

Assert 1-of-2, one remaining, explicit failed count, and no clean sentence.

Build a partially completed fixture without explicit failure evidence.

Assert correct counts and no invented failure.

Build a completed fixture with one question event.

Assert no zero-approval line and an explicit gate count.

Build a fixture missing every optional evidence path.

Assert none of their path strings are rendered.

Build malformed and stale-offset fixtures.

Assert they never render clean or zero claims.

## Step 10: wire `lisa status`

Retain a clone of scanned tickets for the narrative.

Call the summary after existing scheduling output.

Use resolved `work_dir` rather than assuming the default.

Add an integration-style status fixture if output can be writer-captured without
large refactoring; otherwise rely on the shared renderer fixture plus existing
status command tests.

## Step 11: wire post-loop output

Replace Unix process replacement with foreground child waiting.

Record the baseline directly before `Command::status`.

After Zellij exits, rescan tickets and render the summary.

Render before returning a nonzero Zellij status error.

Keep dry-run output and side effects unchanged.

Add helper tests for command construction and baseline behavior.

## Step 12: run focused verification

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-core
cargo test -p lisa-cli templates::tests
cargo test -p lisa-cli run_summary::tests
cargo test -p lisa-cli status::tests
cargo test -p lisa-cli loop_cmd::tests
cargo test -p lisa-plugin ticket_prompt
```

Adjust exact plugin test filtering to the test names actually added.

## Step 13: commit the summary unit

Commit the exact runtime-summary paths:

```text
lisa commit-ticket --ticket-id T-047-01-01 \
  --message "T-047-01-01: report factual run outcomes" \
  --include crates/lisa-cli/src/run_summary.rs \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/src/status.rs \
  --include crates/lisa-cli/src/loop_cmd.rs \
  --include crates/lisa-cli/src/templates.rs
```

If implementation requires another source path, document the deviation in
`progress.md` before including it.

## Step 14: broader verification

Run `cargo test --workspace`.

Run `cargo clippy --workspace --all-targets -- -D warnings` if the workspace's
current unrelated state permits it.

Run a source search for the canonical sentence and inspect intentional
checked-in Markdown copies separately from Rust source.

Run `git status --short` and compare against the preexisting dirty worktree.

Require no ticket-owned source file to remain modified, staged, or untracked.

## Step 15: complete Implement artifact

Write `progress.md` in the private attempt directory.

Record completed steps, commands, commit IDs, tests, and deviations.

Note any broader-suite failures with exact causes.

## Step 16: Review

Inspect the committed diff from both ticket commits.

Recheck acceptance criteria against test names and output assertions.

Confirm no scheduling/state-machine code changed.

Write `review.md` with files, behavior, coverage, and concerns.

Write exactly one valid `review-disposition.json` object.

Stop on this ticket after both Review artifacts exist.
