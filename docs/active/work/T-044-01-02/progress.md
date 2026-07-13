# Progress: verb-forward command help and examples

## Status

- Research complete.
- Design complete.
- Structure complete.
- Plan complete.
- Implementation complete.
- Source commits complete.
- Focused verification complete.
- CLI crate verification complete.
- Workspace/WASM verification complete.
- Review remains.

## Baseline

- Ran `cargo test -p lisa-cli --test help_surface` before source edits.
- Result: pass.
- Baseline count: 4 passed, 0 failed.
- Directly rendered all five operator help screens.
- Each existing purpose was plain and verb-forward.
- None contained an `Example:` line.
- This matched the Research gap.

## Implementation unit 1: operator examples

Modified `crates/lisa-cli/src/main.rs` only.

Added variant-level `after_help` metadata for:

- init;
- validate;
- status;
- doctor;
- loop.

Rendered examples are:

- `Example: lisa init --path ./my-project`
- `Example: lisa validate --path ./my-project --check-tools`
- `Example: lisa status --path ./my-project`
- `Example: lisa doctor --path ./my-project`
- `Example: lisa loop --path ./my-project --max-threads 3`

The existing purpose doc comments were retained because they already begin
with direct verbs and satisfy the existing jargon guard.

No changes were made to:

- top-level help metadata;
- command names;
- option definitions or defaults;
- parser payloads;
- match dispatch;
- command implementation modules;
- plumbing or hidden command metadata.

### Unit 1 verification

- `cargo fmt --all -- --check`: pass.
- `cargo test -p lisa-cli --test help_surface`: pass, 4 tests.
- Direct help rendering for all five commands: pass.
- Top-level snapshot remained unchanged and passed.
- Diff inspection showed only five command-attribute expansions.

### Unit 1 commit

- Used `lisa commit-ticket`.
- Ticket ID: `T-044-01-02`.
- Exact include: `crates/lisa-cli/src/main.rs`.
- Commit: `d12cf4d101d588bda189bf70d2b6e671e8816ddb`.
- Message: `T-044-01-02: add operator command examples`.

## Implementation unit 2: command-specific snapshots

Modified `crates/lisa-cli/tests/help_surface.rs` only.

Added:

- an `OperatorHelpSnapshot` test-only record;
- a five-record inline snapshot array;
- full expected stdout for each operator command;
- `operator_help_matches_snapshots`;
- a length invariant between snapshots and the canonical operator list;
- an order/name invariant between the two collections;
- updated test-contract comments.

The exact snapshots positively preserve:

- each purpose line;
- each generated usage line;
- current option help;
- exactly located concrete examples;
- example-to-command correspondence;
- spacing and trailing newlines.

Existing negative coverage remains:

- the banned-jargon test scans full operator output, including examples;
- the top-level snapshot protects the everyday-path orientation;
- the grouping test protects the operator/plumbing split;
- the resolution test protects all twelve owned command names.

### Unit 2 verification

- `cargo fmt --all -- --check`: pass.
- `cargo test -p lisa-cli --test help_surface`: pass.
- Updated count: 5 passed, 0 failed.
- `git diff --check`: pass.
- Diff inspection showed only snapshot/test contract changes.

### Unit 2 commit

- Used `lisa commit-ticket`.
- Ticket ID: `T-044-01-02`.
- Exact include: `crates/lisa-cli/tests/help_surface.rs`.
- Commit: `4d4c75beaafde5a9ff2dd0be41cae4fa4bcf8c2f`.
- Message: `T-044-01-02: snapshot operator command help`.

## Crate verification

Ran:

```text
cargo test -p lisa-cli
```

Result: pass.

Observed suite results included:

- 14 `lisa_cli` library tests passed in that feature selection;
- 269 binary unit tests passed;
- 1 atomic provider contract test passed;
- 2 capture-usage CLI tests passed;
- 5 help-surface tests passed;
- 1 preownership status test passed;
- 1 real-Zellij test remained intentionally ignored by its existing
  environment gate;
- doc tests passed.

No failures occurred.

## Repository quick check

Ran:

```text
just check
```

Result: pass.

This command completed:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo test --workspace`.

The WASM target check passed. The workspace test suite passed, including the
new five-test help surface. No ticket-related warning or failure was observed.

## Source cleanliness

- `crates/lisa-cli/src/main.rs` is clean.
- `crates/lisa-cli/tests/help_surface.rs` is clean.
- `git diff --cached --name-only` returned no staged paths.
- Both ticket-owned source paths are committed through Lisa.
- No ordinary `git add` or `git commit` was used.

The worktree still contains unrelated/managed Lisa state and active planning
files that were present before the implementation or published by Lisa while
artifacts advanced. They were not included in either ticket source commit.

## Plan deviations

- No implementation design deviation occurred.
- The explicit standalone `cargo test --workspace` planned after `just check`
  was not repeated because `just check` itself ran that exact command and
  passed. Repeating it would add no distinct coverage.
- Progress was written after both atomic source commits rather than updated
  after each command; all executed steps and commit identities are recorded
  here.

## Remaining

- Perform final self-review of the two committed source units.
- Write `review.md`.
- Write the exact `review-disposition.json` contract.
- Remain on this ticket for Lisa completion handling.
