# Research — T-049-04-02

## Ticket boundary

T-049-04-02 closes the transaction side of the S-049-04 incident.

Its implementation boundary is primarily
`crates/lisa-cli/src/commit_transaction.rs`.

Its field-replay assertion crosses into the scheduler fixture in
`crates/lisa-plugin/src/lib.rs` because retry and parking policy live there.

The ticket begins in Research and requires all RDSPI phases in one pass.

Attempt artifacts belong only in the current `.lisa/attempts/.../work`
directory.

Lisa owns ticket phase/status transitions and final artifact publication.

Ticket-owned source must be committed with `lisa commit-ticket` and exact
repository-relative paths.

## Incident evidence

The preserved source is
`docs/active/work/T-046-06-03/cbt-0716-211915-variant-xdg/demo-completion-journal.jsonl`.

It contains 246 JSONL records.

The state distribution is:

- 82 `requested` records;
- 82 `command-in-flight` records;
- 80 `rejected` records;
- 2 `confirmed` records.

All 80 rejected records have the same technical reason.

That reason names `discover prior completion commit` as the failing Git
operation.

Git exited 128 because the current `master` branch had no commits yet.

The completion belonged to T-001, attempt 1, generation 1.

The correlation remained stable while the old scheduler repeatedly created
new request/in-flight/rejected cycles.

The evidence establishes that prior-completion discovery failed before commit
identity could be consulted.

The preserved field folder also contained a root `.lisa-commit.lock` marker.

## Transaction entry points

`commit_ticket` accepts an explicit ticket ID, message, and include paths.

`complete_ticket` first verifies its completion-generation key.

It resolves the ticket and work paths relative to the enclosing Git root.

It saves the ticket's original bytes.

It writes Done frontmatter before entering the isolated transaction.

On transaction failure it restores the exact original ticket bytes.

`commit_ticket_with_key` is the common isolated transaction entry point.

It normalizes explicit include paths before repository discovery.

It discovers the Git worktree root and absolute Git directory.

It acquires `TransactionLock` before completion-key discovery.

It discovers an earlier commit carrying the same generation key.

It reserves an alternate index under the Git directory.

It runs the transaction body, cleans the index, and releases the lock.

## Existing lock behavior

`TransactionLock::acquire` opens or creates `<git-root>/.lisa-commit.lock`.

The file is opened read/write without truncation.

The transaction takes a nonblocking fs2 exclusive advisory lock.

The struct stores the open file, path, and a Boolean `locked` flag.

`finish` explicitly unlocks and clears the flag.

`Drop` makes a best-effort unlock if `finish` was not reached.

The root lock file is intentionally never removed in the current code.

This matches T-031-01's original persistent-inode design.

That design avoided the classic race where unlinking a locked inode lets a
second process lock a replacement inode.

It also means a normal transaction leaves `.lisa-commit.lock` on disk.

The current file contains no owner metadata.

It cannot name a PID, acquisition time, age, or whether the recorded process
still exists.

An unheld leftover inode is harmless to fs2 acquisition but looks like a
stalled transaction to operators and diagnostics.

The held-lock test proves only that a live advisory holder makes acquisition
fail.

It does not prove owner identity, age reporting, stale recovery, or marker
cleanup.

## Existing cleanup control flow

After lock acquisition, completion discovery has a dedicated error branch that
calls `lock.finish`.

Alternate-index reservation has another dedicated branch that calls
`lock.finish`.

The main transaction body always attempts index cleanup and lock release.

Cleanup failures are combined with the primary error.

When cleanup fails after HEAD advancement, the transaction attempts a
compensating ref rollback.

`Drop` is the last-resort unlock path for unwinding and early returns.

Because the marker is persistent, existing tests cannot use marker absence as
evidence that cleanup ran.

## Stable serialization constraints

Removing the visible marker while it is the sole serialization inode is unsafe.

Unlink-before-unlock permits another process to create and lock a replacement
while the first process still owns the old inode.

Unlock-before-unlink permits a waiter to lock the old inode immediately before
the previous owner removes its pathname.

The repository's absolute Git directory is available before lock acquisition.

A stable advisory guard inside that directory can serialize creation and
removal of the visible root marker.

New transaction processes can lock the guard first, then the visible marker.

The visible marker can be removed while the stable guard is still held.

The guard inode may remain inside `.git`; the operator-visible root marker can
then accurately represent an active or crashed transaction.

The visible marker's advisory lock is still needed to respect older Lisa
processes and existing tests that know only `.lisa-commit.lock`.

## Owner and liveness facilities

`lisa-cli` already depends on serde and serde_json.

On Unix it already depends on libc.

The current process PID is available from `std::process::id`.

Unix `kill(pid, 0)` can distinguish an absent PID (`ESRCH`) from a live or
permission-protected PID.

`SystemTime` can supply an acquisition timestamp and an age.

The lock file can therefore carry a small JSON owner record.

The test environment is Unix-like, matching the repository's existing Unix
process-control tests.

## Completion-key discovery

`discover_completion_commit` builds a fixed trailer marker.

It currently runs `git log --format=%H --fixed-strings --grep <marker>` without
an explicit revision.

Git therefore defaults the traversal to HEAD.

On an unborn branch, the default traversal exits 128.

`Repository::git` maps every nonzero exit to `CommitTransactionError`.

The `?` in discovery propagates that error before returning `Ok(None)`.

For nonempty history, candidate commits are individually verified with
`git show -s --format=%B`.

The exact marker must appear as its own message line.

The existing generation-idempotency test proves that a matching earlier commit
short-circuits without moving HEAD or creating another commit.

The missing case is an explicitly unborn current branch.

After discovery returns `Ok(None)`, the current transaction body still requires
HEAD as the parent of a ticket commit.

`run_transaction_body` calls `git rev-parse --verify HEAD` first.

An unborn repository should therefore advance past discovery and fail at that
real Tier-1 precondition until later seal-ladder work supplies another path.

## Existing bounded failure policy

T-049-04-01 landed in `crates/lisa-plugin/src/lib.rs` and
`completion_journal.rs`.

`MAX_COMPLETION_FAILURES` is two failed host-command observations per durable
completion generation.

Known history/identity failures retry below the limit and park at the limit.

The exact plain ask is stored in `HISTORY_IDENTITY_ASK`.

The completion journal records failure count, limit, class, reason, and
consequence.

The scheduler preserves the same completion key and absolute deadline during
retry.

Parking publishes a structured operator block, updates the ticket to Blocked,
writes provenance, and releases the seat.

The existing `history_and_identity_failures_retry_to_bound_then_park_and_unpark`
test injects two hand-written stderr strings.

That test proves scheduler behavior but does not invoke the CLI transaction.

`lisa-plugin` already has a dev-dependency on `lisa-cli` with test support.

The plugin test can therefore call `lisa_cli::commit_transaction::complete_ticket`
inside its native fixture without adding a dependency cycle.

## Test surface

CLI transaction unit tests are colocated in `commit_transaction.rs`.

They already cover unsafe paths, staged isolation, overlap, held lock,
unchanged paths, identity failure, invalid repositories, completion rollback,
nested projects, compensating rollback, and key idempotency.

Representative post-acquisition failures can be strengthened to assert that
the visible marker is absent afterward.

A stale-owner fixture can write explicit old PID/timestamp metadata, then
assert age plus absent-holder text and recovery.

A live-holder fixture can hold the visible advisory lock and assert its bytes
and inode are not stolen or removed.

An unborn fixture can call discovery directly and then call `complete_ticket`
to distinguish discovery success from the later HEAD precondition failure.

The plugin field replay can create a real Git repository with no commits and no
local or inherited identity, run the real completion transaction twice, and
feed those two actual errors into `handle_completion_result`.

The test can cite and parse the preserved journal to prove its source has 80
matching rejected rows.

It can then assert two new failure observations, one retry launch, one Park
provenance row, the exact plain ask, no third attempt, and no root lock marker.

## Repository state and ownership

The shared worktree already contains Lisa-owned modifications to completion and
provenance journals and ticket frontmatter.

It also contains unrelated T-049-03-01 work artifacts.

Those paths must remain untouched and uncommitted by this ticket.

Expected ticket-owned source paths are
`crates/lisa-cli/src/commit_transaction.rs` and, for the cross-boundary replay
test only, `crates/lisa-plugin/src/lib.rs`.

No Cargo dependency change is presently required.
