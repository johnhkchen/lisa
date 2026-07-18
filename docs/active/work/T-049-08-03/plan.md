# Plan: T-049-08-03 notes acknowledgment

## Preconditions

1. Preserve all pre-existing unrelated modifications and untracked files.
2. Do not update ticket phase or status fields manually.
3. Write implementation progress only to the attempt-private `progress.md`.
4. Use no ordinary `git add`, broad add, or ordinary `git commit` command.
5. Commit ticket-owned source through exact `lisa commit-ticket` includes.

## Step 1: Establish the core outcome contract

Modify `crates/lisa-core/src/notes.rs`.

Add the public acknowledgment outcome enum near `QueuedNote`.

Give the enum equality traits used by tests.

Represent empty bare acknowledgment as `NothingToRead`.

Represent successful durable acknowledgment with:

- exact returned `QueuedNote`;
- same-ticket remaining count;
- whether bare selection started from multiple matches.

Verification:

- the enum has no formatting or CLI vocabulary;
- every successful outcome corresponds to one appended exact record;
- the empty outcome does not require a fabricated key.

## Step 2: Implement bare and exact selection

Extend `acknowledge_note` with `generation: Option<u64>`.

Collect same-ticket active notes from one projection snapshot.

Sort same-ticket matches by numeric generation, then exact note key.

For `None`, select the first sorted match or return `NothingToRead`.

For `Some(n)`, select the equal generation.

If exact selection misses, return a plain error naming:

- requested generation;
- requested ticket;
- sorted deduplicated active generation values.

Use a nonblank no-listed-generations sentence when the value list is empty.

Keep the existing append record schema and error context unchanged.

Calculate remaining count from the same pre-append snapshot.

Verification:

- no branch returns the old multiple-active-notes error;
- append runs once for a successful selection and zero times otherwise;
- oldest is numeric generation order, not attempt lexical order.

## Step 3: Expand core unit coverage

Update existing calls with the new optional-generation argument.

Assert the new outcome variants rather than only success/error.

Build a two-active-note fixture whose attempt names sort opposite to generation.

Assert bare selection chooses generation 1.

Assert the provenance record retains generation 1 and its correct attempt ID.

Reconstruct and assert generation 2 remains.

Assert explicit generation 2 can be chosen from a fresh fixture.

Assert explicit unknown generation returns the plain listed-values error and
writes no row.

Assert bare acknowledgment after the last note returns `NothingToRead` without
growing the ledger.

Run:

`cargo test -p lisa-core notes`

Pass gate:

- all notes module tests pass;
- warnings do not identify dead or unreachable new API code.

## Step 4: Record and commit the core unit

Update `progress.md` with completed core behavior and focused test result.

Inspect the exact diff for `crates/lisa-core/src/notes.rs`.

Commit only that file with:

`lisa commit-ticket --ticket-id T-049-08-03 --message "Make note acknowledgment selectable" --include crates/lisa-core/src/notes.rs`

Verification:

- the command succeeds;
- the commit contains the one exact include path;
- the source path is no longer modified afterward.

## Step 5: Add conditional generation labels

Modify `crates/lisa-cli/src/notes.rs`.

Count active notes per ticket before rendering.

Retain the current line for tickets with one active note.

Add `Generation N` between ticket and summary for tickets with multiple active
notes.

Retain criterion, evidence, heading, and blank-line behavior.

Extend formatter tests to pin:

- unchanged single-note output;
- labels for two notes on one ticket;
- no labels for one note each on two tickets.

Verification:

- conditionality is per ticket;
- list order is not changed by counting;
- generation number comes from the exact entry key.

## Step 6: Map core outcomes to plain CLI output

Change `run_ack` to accept `Option<u64>`.

Pass the option to the core function.

Print the ticket-prescribed oldest line for a bare multi-note acknowledgment.

Retain `{ticket_id} acknowledged.` for ordinary bare single-note success.

Print explicit generation identity for exact success.

Print `Nothing to read for {ticket_id}.` for a bare empty queue and return success.

Handle singular and plural remaining counts.

Verification:

- prescribed one-remaining sentence matches punctuation exactly;
- every successful branch prints a nonblank line;
- core errors continue through main's existing exit-1 handling.

## Step 7: Expose `--generation`

Modify `crates/lisa-cli/src/main.rs`.

Add `generation: Option<u64>` to the ack clap variant.

Describe it as selecting a listed generation.

Revise ack summary to cover oldest/default semantics.

Add an exact-generation example to notes help.

Pass the parsed option through dispatch to `run_ack`.

Modify `crates/lisa-cli/tests/help_surface.rs` to match intended help.

Verification:

- bare invocation parses unchanged;
- flag parses before or after the path placement supported by global `--path`;
- an invalid numeric generation is rejected by clap.

## Step 8: Build acceptance fixtures

Modify `crates/lisa-cli/tests/notes_ux.rs`.

Generalize journal construction for more than one completion generation.

Retain the existing built-binary, path-with-spaces, and fresh-process pattern.

Update single-note duplicate acknowledgment to expect successful nothing-to-read
output and unchanged provenance length.

Add a bare two-note drain test:

1. Write generations 1 and 2.
2. List and pin both generation labels.
3. Bare ack and pin the exact oldest/one-remaining line.
4. List from a fresh process and see only generation 2.
5. Bare ack again and retain single-note success text.
6. List from a fresh process and see global emptiness.
7. Deserialize both provenance rows and assert generations `[1, 2]`.

Add an explicit-selection test:

1. Write generations 1 and 2.
2. Request unknown generation 9.
3. Assert exit 1 and a plain error naming listed generations 1 and 2.
4. Assert no provenance acknowledgment was appended.
5. Request generation 2.
6. Assert exact success and provenance generation 2.
7. List from a fresh process and assert generation 1 remains.

Verification commands:

- `cargo test -p lisa-cli --test notes_ux`
- `cargo test -p lisa-cli --test help_surface`

Pass gate:

- all process exits, stdout, stderr, and durable rows match the acceptance text;
- single-note behavior remains pinned;
- durability is observed only through fresh executable invocations.

## Step 9: Format and perform focused regression checks

Run `cargo fmt --all -- --check`.

If formatting fails, run the repository formatter and inspect only ticket paths.

Rerun core notes tests, CLI notes UX, and help snapshots.

Search the codebase for the exact old jargon string and its distinctive fragments.

Pass gate:

- formatter passes;
- focused tests pass;
- `multiple active notes` and `acknowledgment requires an exact generation` do
  not occur in Rust source.

## Step 10: Record and commit the CLI unit

Update `progress.md` with fixture evidence and any deviations.

Inspect exact diffs for the four CLI-owned paths.

Commit them through:

`lisa commit-ticket --ticket-id T-049-08-03 --message "Let operators drain note generations" --include crates/lisa-cli/src/notes.rs --include crates/lisa-cli/src/main.rs --include crates/lisa-cli/tests/notes_ux.rs --include crates/lisa-cli/tests/help_surface.rs`

Verification:

- the command succeeds;
- no unrelated path appears in the commit;
- no ticket-owned source path remains modified or untracked.

## Step 11: Run full verification

Run `cargo test --workspace`.

If a failure is unrelated, establish that with the specific failing path and
existing worktree state; otherwise fix it within ticket scope and commit the
exact affected path through another isolated transaction.

Run final focused tests after any correction.

Inspect `git status --short` without altering unrelated changes.

Record command results and any remaining external concern in `progress.md`.

## Step 12: Review and disposition

Write `review.md` in the attempt-private work directory.

Summarize every changed source file and externally visible behavior.

Map each acceptance criterion to concrete test evidence.

Report full-suite and focused-suite results.

Report the old-string search result.

Report any gaps, limitations, or unrelated dirty paths.

Write `review-disposition.json` as exact compact pass JSON only when all required
work is ready:

`{"disposition":"pass","reason":null}`

Run:

`lisa check-disposition T-049-08-03`

Correct all reported issues before stopping.

Remain on this ticket after Review; do not publish Done or begin other work.
