# Plan: Review disposition emission contract

## Objective

Establish and test one exact agent-emitted Review disposition contract without
changing parser or scheduler runtime behavior.

## Step 1: update canonical project documentation

Edit only the Review section of `docs/knowledge/rdspi-workflow.md`.

Add the fixed companion filename, exact pass and block JSON payloads, and the
reason validity rules. State that both Review artifacts must exist before the
agent waits. List both output paths.

Verification:

- inspect the section in context;
- confirm JSON examples are syntactically valid;
- confirm no other workflow phase changed.

## Step 2: update the embedded outgoing workflow

Apply the identical Review-section edit to
`crates/lisa-cli/data/rdspi-workflow.md`.

Verification:

- byte-compare the entire current workflow files;
- ensure legacy workflow files have no diff;
- confirm `RDSPI_WORKFLOW` still uses `include_str!` for this data file.

## Step 3: pin the contract in template tests

Modify only the test module in `crates/lisa-cli/src/templates.rs`.

Extend the phase embedding test to include Review. Add a focused test that
constructs a detected Rust project, generates `CLAUDE.md`, and verifies the
workflow pointer/injection statement. In the same test, assert the embedded
workflow includes:

- `review-disposition.json`;
- the exact pass object;
- the exact block object/example;
- explicit invalidity of pass with reason and block without reason.

Keep the test independent of a parser. It protects text contract emission, not
runtime semantics.

Verification:

- run `cargo fmt --all` if formatting changes are required;
- run the named template tests;
- confirm a deliberately changed expected filename would make the test fail by
  inspection of exact assertions.

## Step 4: focused static verification

Run:

- byte comparison of the two current workflow files;
- targeted `rg` across both workflow files and `templates.rs`;
- `git diff --check`;
- exact-path diff review.

Acceptance checks:

- both current workflows specify the same fixed name;
- both specify `{disposition, reason}` through canonical JSON examples;
- both define reason nullability/content semantics;
- the test covers generated pointer plus injected body;
- no parser/plugin behavior enters the diff.

## Step 5: Rust verification

Run the focused `lisa-cli` template test first. Then run the complete
`cargo test -p lisa-cli` suite. Run `cargo fmt --all -- --check` after any format
operation. If failures are unrelated or environmental, record exact evidence in
`progress.md`; otherwise fix ticket-owned failures before committing.

If full CLI tests are successful, run `cargo test --workspace` as broader
regression coverage when feasible. This ticket is text/test-local, so a WASM
release build is not a necessary acceptance gate.

## Step 6: record implementation progress

Create `progress.md` in the attempt-private work directory before the source
transaction. Record:

- completed edits;
- filename/schema decision;
- test commands and results;
- any deviations;
- exact paths intended for the transaction.

Do not include `progress.md` in the source transaction.

## Step 7: commit the meaningful source unit

Use Lisa's isolated transaction exactly once because the documentation,
embedded data, and contract assertion form one indivisible behavior unit:

```text
lisa commit-ticket \
  --ticket-id T-040-01-01 \
  --message "Document Review disposition emission contract" \
  --include docs/knowledge/rdspi-workflow.md \
  --include crates/lisa-cli/data/rdspi-workflow.md \
  --include crates/lisa-cli/src/templates.rs
```

If the installed binary lacks the command, use the already-built repository CLI
or build it, then invoke its `commit-ticket` subcommand with the same exact
includes. Do not use ordinary `git add`, `git commit`, or the ordinary index.

Verification:

- capture the resulting commit identifier;
- inspect `git status --short` for each exact owned path;
- ensure no owned path remains staged, modified, or untracked;
- preserve scheduler-owned ticket phase changes and unrelated worktree changes.

## Step 8: Review artifact and disposition

Write `review.md` to the attempt-private directory. Summarize changed files,
the chosen schema, tests, coverage limits, transaction evidence, and open
concerns.

Then write this ticket's own `review-disposition.json` using the newly defined
contract. Use pass only if acceptance is met, tests are green, and all owned
source is committed. Otherwise write block with a non-empty actionable reason.

After both artifacts exist, stop on this ticket. Do not change ticket
frontmatter, publish artifacts, run completion manually, or start another
ticket.

## Test matrix

| Concern | Evidence |
|---|---|
| Fixed filename | exact assertions and text search |
| Pass shape | exact `RDSPI_WORKFLOW` assertion |
| Block shape | exact `RDSPI_WORKFLOW` assertion |
| Contradiction semantics | wording assertion |
| Docs/embedding parity | byte comparison |
| Generated agent link | `generate_claude_md` assertion |
| Init/template compatibility | `cargo test -p lisa-cli` |
| Repository regression | `cargo test --workspace` if feasible |
| Transaction isolation | exact-path status after `commit-ticket` |

## Completion criteria

Implementation is ready for Review when all three ticket-owned files contain the
settled contract, tests prove the injection chain, workflow copies match, the
source unit is committed through Lisa's isolated transaction, and the only
remaining ticket outputs are attempt-private Review artifacts awaiting Lisa's
publication.
