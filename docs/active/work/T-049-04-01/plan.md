# Plan: bounded park on completion failure

## Step 1: establish the durable retry vocabulary

Modify `crates/lisa-plugin/src/completion_journal.rs`.

Add completion failure class and consequence enums with stable serialized
names.

Add the FailureObserved transition and journal row.

Advance the writer schema while accepting legacy schemas.

Add retry count, limit, and exhaustion to the aggregate.

Reset these fields on a new Requested generation.

Preserve them through Rejected and Confirmed transitions.

Validate monotonic counts, stable bounds, matching correlations, and legal
consequences while folding.

Verification:

`cargo test -p lisa-plugin completion_journal`

Expected result: legacy and new journal fixtures pass; malformed retry streams
fail closed.

## Step 2: implement pure classification and action policy

Modify `crates/lisa-plugin/src/lib.rs` near completion constants and policy
types.

Add the fixed failure limit.

Add the exact shared history/identity ask.

Add private failure remedy and action types.

Implement narrow case-insensitive matching for:

- unborn branch;
- missing identity;
- repository permissions/read-only state;
- stale Lisa/Git lock;
- transient index contention;
- unrecognized errors.

Order stale-lock recognition before transient lock contention.

Implement action selection:

- known operator failure below limit: retry;
- known operator failure at limit: park;
- transient below limit: retry;
- transient at limit: wait for deadline;
- unrecognized: park immediately.

Add table tests for classification, exact ask selection, unstructured fallback,
and bound edges.

Verification:

`cargo test -p lisa-plugin completion_failure`

Expected result: every ticket-named class maps to one explicit policy.

## Step 3: add the common completion parking transaction

Generalize the existing parking provenance helper name if needed.

Implement atomic canonical block disposition publication.

Structured remedies write operator owner and ask.

Unknown remedies write only raw reason and therefore parse unstructured.

Implement `park_failed_completion`.

Append the ActionRequired rejection before releasing scheduler ownership.

Restore the journal aggregate's prior phase.

Set ticket status blocked.

Append the E-048 Park provenance row with the current attempt lease and retry
progress when present.

Remove pending completion state, release the slot, remove the thread, and
rebuild the DAG.

Keep a plain ask at the start of activity detail.

Add a helper fixture that installs a Review attempt, passing disposition,
journal, ledger, ticket file, and slot.

Verification:

Focused park tests inspect all four durable surfaces: journal, canonical
disposition, ticket frontmatter, and provenance.

## Step 4: route completion command failures through the policy

Refactor `handle_completion_result` failure handling.

Retain the full current technical envelope for journal audit.

Read prior failure count from the matching journal aggregate.

Append one FailureObserved row for every failed command result.

For retry, relaunch the exact generation and absolute deadline.

Generalize replay validation for both attempt and operator authority while
preserving source and authority.

Remove the current replay branch that drops failure evidence.

For transient exhaustion, remove only the finished host invocation and leave
the journal aggregate in flight.

Guard reconciliation replay when the durable aggregate says retries are
exhausted.

For parking actions, call the common park helper.

Do not alter success handling.

Add fixture coverage:

- unborn history failure 1/2 retries;
- unborn history failure 2/2 parks;
- identity failure follows the same exact ask;
- permission failure uses its structured ask;
- stale-lock failure uses its structured ask;
- transient failure 1/2 retries;
- transient failure 2/2 launches no third command and does not immediately
  park;
- unrecognized failure parks immediately with raw unstructured ask.

Assert the journal exposes each count and the configured limit.

Assert no fixture can launch beyond the bound.

Verification:

`cargo test -p lisa-plugin completion_commit_failure`

or the exact available test-name filters after implementation.

## Step 5: replace deadline dead-end with park and recovery

Refactor `expire_in_flight_completion` to use the common park helper.

Keep the complete correlation/deadline reason in the completion journal.

Persist a plain structured uncertainty ask for the operator surfaces.

Update the existing deadline regression.

Assert after expiry:

- pending command is absent;
- ticket is Review/blocked;
- canonical disposition is a structured operator block;
- journal is ActionRequired for audit/masking;
- Park provenance exists;
- seat and thread are released;
- no further completion launch occurs while blocked.

Simulate the ordinary unpark path by restoring ticket status open through the
same ticket API used by `lisa unblock`.

Rebuild and run unpark reconciliation without touching the completion journal.

Assert:

- Unpark provenance follows Park;
- ordinary DAG scheduling can mint a new attempt;
- the old ActionRequired generation no longer makes the ticket permanently
  unschedulable.

Verification:

`cargo test -p lisa-plugin reconciliation_deadline`

## Step 6: focused verification and formatting

Run formatter checks on the changed Rust sources.

Use `cargo fmt --all -- --check` after applying formatting.

Run completion-journal tests.

Run completion failure and deadline tests.

Run the operator recovery matrix tests because they exercise adjacent modal
and authority behavior.

Run all plugin native tests.

Commands:

`cargo fmt --all`

`cargo test -p lisa-plugin completion_journal`

`cargo test -p lisa-plugin completion_failure`

`cargo test -p lisa-plugin reconciliation_deadline`

`cargo test -p lisa-plugin --test operator_recovery_matrix`

`cargo test -p lisa-plugin`

Expected result: zero failures and no changed successful-completion assertions.

## Step 7: workspace verification

Run `cargo test --workspace`.

Run `just check` if the configured WASM target and toolchain are available.

If a repository-wide check fails outside the owned paths, record the exact
failure and distinguish it from ticket regressions.

Inspect exact diffs for only the two owned source paths.

Inspect ordinary index state without modifying it.

Commands:

`git diff -- crates/lisa-plugin/src/completion_journal.rs crates/lisa-plugin/src/lib.rs`

`git diff --cached --name-only`

`git status --short`

Expected result: ticket-owned source is unstaged before the Lisa transaction;
unrelated existing changes remain untouched.

## Step 8: isolated source commit

Commit the coupled source unit with exact paths only:

`lisa commit-ticket --ticket-id T-049-04-01 --message "fix(plugin): bound and park completion failures" --include crates/lisa-plugin/src/completion_journal.rs --include crates/lisa-plugin/src/lib.rs`

If `lisa` on PATH is unavailable, use the already-built project CLI with the
same subcommand and exact include arguments.

Do not use ordinary `git add` or `git commit`.

After the transaction, verify both owned paths are clean relative to HEAD.

Record the returned commit identity in `progress.md`.

## Step 9: final review

Write `progress.md` with completed steps, tests, deviations, and commit result.

Write `review.md` with file summary, behavioral summary, test coverage, open
concerns, and repository hygiene.

Write exactly the pass disposition JSON when all criteria and checks pass.

If a ticket-owned defect remains, write a structured block disposition with an
actionable agent remedy instead.

Do not modify the ticket phase or status.

Do not publish attempt artifacts to the shared work directory.

Remain on T-049-04-01 after Review for Lisa's completion gate.
