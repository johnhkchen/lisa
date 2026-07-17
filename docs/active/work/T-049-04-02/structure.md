# Structure — T-049-04-02

## Source ownership

Modify `crates/lisa-cli/src/commit_transaction.rs` for all production behavior.

Modify `crates/lisa-plugin/src/lib.rs` only for the cross-boundary field replay
test and its native-test helpers.

Do not modify Cargo manifests because the plugin already dev-depends on
`lisa-cli` with test support.

Do not modify ticket frontmatter.

Do not write phase artifacts to `docs/active/work/T-049-04-02`.

## `commit_transaction.rs` import changes

Extend serde imports for private lock-owner serialization.

Extend `std::io` imports with read/seek/write operations used on the marker.

Extend time imports with `SystemTime` and `UNIX_EPOCH`.

Keep fs2's `FileExt` for both stable guard and visible marker locks.

Use Unix libc only behind `cfg(unix)`.

## Raw Git execution boundary

Split current Git execution into two layers.

The lower layer launches Git and returns `Output` even when Git exits nonzero.

It still maps command-spawn failures to `CommitTransactionError` with the
operation name.

The existing `run_git_at` wrapper calls the lower layer and retains current
nonzero status formatting.

`Repository::git` remains the strict successful-command API used throughout
transaction work.

Unborn detection uses the lower layer because exit 1 from `show-ref --verify`
is data, not a transaction error.

No public API is added.

## Unborn-HEAD helper

Add a private `head_is_unborn(&Repository) -> Result<bool, ...>` helper near
completion discovery.

Run `git symbolic-ref --quiet HEAD` with raw status access.

If symbolic-ref succeeds, decode and trim the exact ref name.

An empty or non-UTF-8 ref remains an error.

Run `git show-ref --verify --quiet <ref>` with raw status access.

Exit 0 means the current branch has a commit and returns false.

Exit 1 means the symbolic branch ref is absent and returns true.

Any other exit is formatted and returned as an inspection error.

If symbolic-ref reports detached HEAD, return false and let ordinary discovery
handle it.

## Completion discovery change

At the start of `discover_completion_commit`, call `head_is_unborn`.

Return `Ok(None)` immediately when true.

Leave marker construction, `git log`, UTF-8 decoding, candidate verification,
and exact message-line matching unchanged.

This isolates empty-history policy from normal idempotency logic.

## Lock constants

Add a private guard filename constant, `lisa-commit.guard`.

Retain `.lisa-commit.lock` as the visible marker filename.

Add a private owner schema version constant.

No retry count belongs in the transaction module.

## Lock owner type

Add `TransactionLockOwner` beside `TransactionLock`.

Derive serialize, deserialize, debug, clone, partial equality, and equality as
useful for production and tests.

Fields:

- `schema_version: u32`;
- `pid: u32`;
- `acquired_unix_ms: u64`.

Provide a constructor for the current process and current time.

Provide age calculation with saturating subtraction.

## Process liveness helper

Add a small private liveness enum or predicate.

On Unix, call `libc::kill(pid as i32, 0)`.

Treat zero and `EPERM` as present.

Treat `ESRCH` as absent.

Treat invalid/out-of-range PID and other errno values as unknown/present for
conservative recovery.

On non-Unix targets, return unknown rather than auto-recover a parsed owner.

Expose no process-control API outside the module.

## `TransactionLock` fields

Replace the single file with:

- stable `guard_file`;
- visible `marker_file` when opened;
- `guard_path` for errors;
- visible `path` for errors/removal;
- `guard_locked` Boolean;
- `marker_locked` Boolean;
- `owns_marker` Boolean.

The marker file remains open until cleanup completes.

Ownership becomes true only after this process successfully locks the marker.

## `TransactionLock::acquire`

Change its signature to accept both Git root and Git directory.

Open/create the stable guard read/write without truncation.

Try its exclusive lock nonblocking.

On failure, report a live/temporary transaction and return without opening or
changing the marker.

Record whether the visible marker existed before opening it.

Open/create the marker read/write without truncation.

Try its exclusive lock nonblocking.

On failure, read owner metadata best-effort for the error, release the guard,
and leave marker bytes/path untouched.

After both locks are held, inspect existing marker bytes before truncation.

If they describe an absent PID, call stale recovery.

If they describe a live PID, return conservatively without stealing it.

For a malformed legacy marker, use file modification age and the acquired
advisory lock to describe/recover an ownerless stale marker.

For a new marker, write the current owner record and return the RAII value.

## Marker metadata helpers

Add a helper to read the complete marker from offset zero.

Empty content returns no parsed owner rather than a JSON error.

Add a helper to truncate, seek to zero, serialize the owner, append newline,
and flush/sync data.

Add formatting helpers for age in milliseconds/seconds and owner facts.

Keep operator text deterministic enough for substring assertions.

## Stale recovery helper

Operate only on a `TransactionLock` that owns both advisory locks.

Capture the stale age and holder description before cleanup.

Call `finish` so marker deletion and both unlocks occur.

Return a `CommitTransactionError` containing:

- the word `stale`;
- `.lisa-commit.lock` path;
- age;
- recorded PID where present;
- absent/no-such-process wording;
- explicit `recovered` wording;
- any cleanup error if recovery cleanup also failed.

## `TransactionLock::finish`

Make cleanup idempotent.

If this process owns the marker, remove its pathname while the guard is still
locked.

Ignore `NotFound` only for the marker removal.

Unlock the marker file if locked.

Unlock the stable guard if locked.

Attempt every step and collect all errors.

Clear state flags as each step succeeds or is no longer applicable.

Return one combined `CommitTransactionError` when any cleanup failed.

## `TransactionLock::drop`

Call `finish` best-effort when any owned resource remains.

This covers early `?` exits, panics, and future failure paths.

Do not remove the marker unless `owns_marker` is true.

Never remove the persistent guard path.

## Transaction integration

Change `commit_ticket_with_key` to call
`TransactionLock::acquire(&repo.root, &repo.git_dir)`.

Retain explicit finish calls in discovery and alternate-index reservation
error branches so cleanup errors remain observable.

Retain existing main-body cleanup aggregation.

The successful idempotent-discovery branch must finish and remove its marker
before returning the earlier commit.

No changes are needed in `complete_ticket` rollback logic.

## CLI unit-test helper extensions

Add `GitRepo::new_unborn_without_identity` or equivalent setup.

Ensure tests can execute Git without inherited global/system identity by
passing isolated config environment where necessary.

Add `assert_no_visible_lock` for concise cleanup assertions.

Add a helper to create an old owner record with a guaranteed absent PID.

Tests may call private discovery and lock helpers because they are module-local.

## CLI lock tests

Replace or extend `held_lock_returns_actionable_error`.

Write live owner bytes before taking the external advisory marker lock.

Run a competing transaction.

Assert the error names contention/live ownership.

Assert marker bytes are identical and the path still exists while held.

Unlock and remove the fixture marker during test teardown.

Add a stale dead-holder test.

Write an owner record with an old timestamp and absent PID without holding the
advisory lock.

Run a transaction and assert its first result is the recovered-stale error.

Assert the message includes age and absent-holder facts.

Assert the root marker is absent after that result.

Run again and assert the transaction proceeds, demonstrating recovery.

## CLI cleanup tests

Strengthen unchanged-path, staged-overlap, identity/commit failure, completion
rollback, and success/idempotency tests with root-marker absence assertions.

Add a direct scope/drop test if a deterministic post-acquisition early return
is otherwise uncovered.

Continue checking alternate-index cleanup and ordinary-index preservation.

Do not require the stable `.git/lisa-commit.guard` inode to be absent.

## CLI discovery tests

Add an unborn repository test that calls `discover_completion_commit` and
asserts `Ok(None)`.

Then create ticket/work inputs and call `complete_ticket`.

Assert its error names `resolve HEAD`, not `discover prior completion commit`.

Assert the original ticket bytes are restored.

Assert the visible marker is absent.

Retain the existing repeated-key test as coverage for history short-circuit.

Add explicit marker-absence assertions after replay.

## Plugin field-replay helpers

Add a test-only helper near `completion_failure_fixture` to initialize its root
as a Git repository without commits.

The helper invokes Git with isolated global/system configuration and removes
any local identity.

Add a helper that constructs `CompleteTicketRequest` from the fixture state,
lease, and ticket/work paths.

Call `lisa_cli::commit_transaction::complete_ticket` directly.

Return the error string that production would place on stderr.

## Plugin field-replay test

Name the test after the 2026-07-16 field journal replay.

Use `include_str!` with the repository-relative preserved journal path.

Parse its lines as JSON values.

Assert 80 rejected rows contain the old discovery failure text.

Build T-001 with the existing completion failure fixture and initialize its
root Git repository as unborn/identity-less.

Dispatch one production completion generation.

Execute the real CLI completion transaction for the first launched command.

Feed its error into `handle_completion_result` and assert exactly one retry was
launched.

Execute the same real CLI transaction for the retry.

Feed its error into `handle_completion_result` and assert one park.

Assert no third launch after reconciliation.

Assert exact plain ask, exact 2/2 journal count, one Park provenance row, seat
release, Blocked ticket state, and no root marker.

The older hand-written history/identity table test may remain for narrow
classification coverage.

## Commit unit

The production lock/discovery change and cross-boundary regression form one
meaningful incident-closure unit.

Commit both exact source paths in one `lisa commit-ticket` command.

Do not include attempt artifacts, Lisa journals, ticket files, or unrelated
worktree paths in that transaction.
