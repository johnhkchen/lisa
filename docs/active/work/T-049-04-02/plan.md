# Plan — T-049-04-02

## Step 1: separate raw and strict Git execution

Modify `crates/lisa-cli/src/commit_transaction.rs`.

Extract command spawning into a helper that returns raw `Output` regardless of
exit status.

Keep operation-named spawn errors unchanged.

Have existing `run_git_at` validate status and retain its current error text.

Run the focused CLI transaction tests to establish no behavior regression.

Verification:

`cargo test -p lisa-cli commit_transaction`

Expected result: all existing transaction tests pass before new behavior is
asserted.

## Step 2: implement exact unborn-HEAD detection

Add a private helper using raw `symbolic-ref` and `show-ref` results.

Return true only for a symbolic current branch whose ref does not exist.

Preserve strict errors for unexpected Git statuses and malformed output.

Call the helper before `git log` in completion-key discovery.

Return `Ok(None)` for the exact unborn case.

Leave candidate verification unchanged.

Add an unborn discovery unit test.

Assert direct discovery returns `None`.

Assert a full completion attempt advances to the existing `resolve HEAD`
precondition error.

Assert the error no longer names prior-completion discovery.

Assert ticket rollback remains byte-exact.

Verification:

`cargo test -p lisa-cli unborn_completion_history`

`cargo test -p lisa-cli repeated_completion_key`

Expected result: new repositories do not fail discovery; nonempty key replay
still short-circuits idempotently.

## Step 3: introduce owner-described visible markers

Add the private owner JSON type and current-time/PID constructor.

Add marker read and write helpers.

Add age formatting.

Add conservative Unix PID liveness detection with a non-Unix fallback.

Unit-test parsing and liveness indirectly through stale/live fixtures rather
than exposing new public APIs.

Verification:

Run the focused stale/live test filters after Step 4 completes.

## Step 4: split stable serialization from visible lock state

Refactor `TransactionLock` to own a stable Git-dir guard and the visible root
marker.

Acquire the guard first.

Acquire the marker's advisory lock second for backward compatibility.

Do not touch marker bytes until both locks belong to this transaction.

Write the current owner record for a new marker.

Detect an existing parsed absent owner as stale.

Recover that marker under the guard and return one actionable error naming age,
PID, absence, and recovery.

Keep a parsed live-owner marker untouched.

Keep any marker whose advisory lock cannot be acquired untouched.

Change `finish` to remove only an owned marker, then unlock marker and guard,
attempting every step.

Make `Drop` reuse the cleanup path best-effort.

Change transaction acquisition to pass the Git directory.

Verification:

`cargo test -p lisa-cli stale_commit_lock`

`cargo test -p lisa-cli live_commit_lock`

Expected result: dead-owner residue produces one named recovery and disappears;
live ownership remains byte-for-byte intact.

## Step 5: prove failure-path cleanup

Add a shared assertion for visible root-marker absence.

Apply it after successful commits and idempotent short-circuit.

Apply it after unchanged-path failure.

Apply it after staged-overlap failure.

Apply it after identity/commit-tree failure.

Apply it after completion rollback.

Apply it after unborn HEAD failure.

Add a direct lock-drop test if any pre-body post-acquisition path lacks natural
coverage.

Retain all existing HEAD, ordinary-index, alternate-index, and ticket-byte
assertions.

Verification:

`cargo test -p lisa-cli commit_transaction`

Expected result: all transaction tests pass and no owned path leaves the root
marker behind.

## Step 6: construct the field environment in the plugin fixture

Modify only test code in `crates/lisa-plugin/src/lib.rs`.

Initialize the existing completion failure fixture root with `git init`.

Isolate global and system Git config for fixture commands.

Leave HEAD unborn.

Ensure no repository identity exists.

Build a real `CompleteTicketRequest` from the fixture ticket, canonical work
directory, attempt ID, and generation.

Call `lisa_cli::commit_transaction::complete_ticket` for each host command
attempt and capture its real error.

Assert each failure cleans `.lisa-commit.lock` before handing the result to the
scheduler.

Verification:

Run the new replay test filter after Step 7.

## Step 7: connect preserved evidence to bounded scheduler policy

Add a native plugin test named for the 2026-07-16 replay.

Embed the preserved field journal with `include_str!`.

Parse JSONL and count exactly 80 rejected old discovery failures.

Dispatch the fixture completion once.

Run the actual CLI transaction and feed its first failure to scheduler handling.

Assert exactly one retry launch.

Run the actual CLI transaction for that retry and feed its second failure.

Assert exactly one Park transition and exact history/identity ask.

Assert the journal has only two new failure observations with 1/2 then 2/2.

Assert the ticket is blocked, the thread/seat are released, and no pending
completion remains.

Trigger reconciliation again and assert zero further launches.

Assert no root marker remains.

Verification:

`cargo test -p lisa-plugin field_journal_replay`

Expected result: the 80-rejection source reduces to one initial attempt, one
retry, one park, and no residue.

## Step 8: format and focused regression checks

Run Rust formatting.

Run diff whitespace validation.

Run all CLI transaction tests.

Run plugin completion-failure and deadline tests because the replay shares
their scheduler machinery.

Commands:

`cargo fmt --all`

`git diff --check`

`cargo test -p lisa-cli commit_transaction`

`cargo test -p lisa-plugin completion_failure`

`cargo test -p lisa-plugin field_journal_replay`

Expected result: zero failures and no changes to unrelated behavior.

## Step 9: crate and workspace verification

Run the complete native test suites for both changed crates.

Run the workspace suite.

Run the repository's combined check, including WASM checking, if the configured
target is available.

Commands:

`cargo test -p lisa-cli`

`cargo test -p lisa-plugin --lib`

`cargo test --workspace`

`just check`

Expected result: all executable tests pass; any environment-only failure is
recorded precisely in progress and review.

## Step 10: inspect ownership and repository hygiene

Inspect diffs only for the two ticket-owned source paths.

Inspect the ordinary index without changing it.

Confirm no ticket-owned source path is staged.

Confirm existing unrelated modifications are unchanged.

Confirm no root `.lisa-commit.lock` exists after tests.

Commands:

`git diff -- crates/lisa-cli/src/commit_transaction.rs crates/lisa-plugin/src/lib.rs`

`git diff --cached --name-only`

`git status --short`

Expected result: only the two intended source files are ticket-owned
modifications; unrelated Lisa-managed state remains outside the commit.

## Step 11: commit the meaningful source unit

Commit the coupled production change and regression with Lisa's isolated
transaction.

Command:

`lisa commit-ticket --ticket-id T-049-04-02 --message "fix(cli): clean completion locks and handle unborn history" --include crates/lisa-cli/src/commit_transaction.rs --include crates/lisa-plugin/src/lib.rs`

If the PATH binary is unavailable, use the already-built repository CLI with
the same subcommand and exact include paths.

Do not use ordinary `git add`, `git commit`, or a broad include.

After commit, verify both source paths are clean relative to HEAD and the
ordinary index remains unchanged.

Record the returned commit ID in `progress.md`.

## Step 12: implementation record

Write `progress.md` in the current attempt directory.

Record completed steps, test commands and outcomes, the isolated commit ID,
and any deviations from this plan.

Do not include attempt artifacts in the source transaction.

## Step 13: review

Inspect the committed diff from the source commit.

Re-evaluate each acceptance criterion against code and test evidence.

Confirm live-lock behavior cannot delete another holder's marker.

Confirm stale errors contain age and absent-holder facts.

Confirm empty discovery still preserves nonempty-history idempotency.

Confirm the field replay cites the preserved journal and observes actual CLI
errors.

Write `review.md` in the attempt directory.

Write exactly `{"disposition":"pass","reason":null}` when ready.

Otherwise write the required actionable block disposition.

Remain on T-049-04-02 after Review and do not publish or advance frontmatter.
