# Design — T-049-04-02

## Decision summary

Keep the isolated transaction's advisory serialization model, but split its two
responsibilities across two files.

Use a stable, persistent advisory guard inside the Git directory for process
serialization.

Use the existing root `.lisa-commit.lock` as an ephemeral, owner-described
operator marker.

Acquire the stable guard first and retain the visible marker's advisory lock for
compatibility with older Lisa processes.

Write PID and acquisition time into the visible marker.

Remove the visible marker on every owned completion or failure path while the
stable guard is still held.

Treat an existing unheld marker whose recorded process is absent as a recovered
stale lock and return one actionable failure naming its age and absent holder.

Never remove or rewrite a marker held by a live process.

Preflight completion-key discovery for an unborn current branch and return
`Ok(None)` before invoking `git log`.

Connect an actual CLI failure from an unborn, identity-less repository to the
existing plugin bounded-retry fixture.

## Lock option 1: delete the existing inode directly

The smallest change would make `TransactionLock::finish` remove
`.lisa-commit.lock` before or after fs2 unlock.

Removing before unlock creates a replacement-inode window.

Another process can create and lock the same pathname while the first process
still holds the now-unlinked inode.

Removing after unlock creates a waiter-removal window.

Another process can acquire the old inode after unlock and lose its pathname
when the previous owner removes it.

Either ordering can admit two transactions.

This option is rejected.

## Lock option 2: owner file with create-new only

The visible path could become a conventional PID file created with
`create_new(true)` and deleted on release.

This naturally leaves evidence after a crash.

It does not by itself make stale takeover race-free.

Two recovery processes can both inspect the same stale owner; one can remove
and recreate the path before the other removes the replacement.

Nonce and inode comparisons reduce but do not eliminate the check/remove race.

It also stops respecting older Lisa processes that hold an fs2 lock on the
visible file.

This option is rejected.

## Lock option 3: stable guard plus ephemeral marker

Create or reuse a guard file under the repository's Git directory.

The guard remains a stable inode and is never used as an operator signal.

Every new transaction takes a nonblocking exclusive lock on the guard first.

While holding the guard, it opens the root marker and attempts the existing
nonblocking exclusive marker lock.

Failure to take the guard means another updated Lisa transaction is live.

Failure to take the marker means a live older or external compatible holder is
present.

In either live case the transaction returns contention without changing the
marker.

Once both locks are held, no updated peer can race marker inspection,
replacement, or removal.

At finish, remove the visible marker, unlock its open inode, then unlock the
stable guard.

The transaction's Git/ref/index work is already complete before that removal.

This option is selected.

## Stable guard location

Place the guard at `<absolute-git-dir>/lisa-commit.guard`.

The Git directory is already discovered before transaction lock acquisition.

Keeping the guard in `.git` prevents it from appearing as worktree residue.

The guard is implementation state, not an operator-facing stall signal.

It may remain as an empty reusable inode just as the former root lock did.

No Git command treats that filename as a native Git lock.

## Visible owner record

Serialize one JSON object into `.lisa-commit.lock`.

Fields are a schema version, process ID, and acquisition Unix milliseconds.

The schema is private to the CLI transaction and intentionally small.

After both advisory locks are held, truncate and rewrite the marker.

Flush the bytes before entering completion discovery or transaction work.

An operator can read the file without special tooling.

The stale error computes age from the saved timestamp.

On Unix, PID liveness uses `kill(pid, 0)`.

Success or `EPERM` means the process exists.

`ESRCH` means the process is absent.

Other results are treated conservatively as not proven absent.

## Stale-marker policy

An existing marker is considered automatically recoverable only when:

- its owner record parses;
- the recorded PID is proven absent; and
- the current transaction has acquired both advisory locks.

The age is evidence, not a takeover timeout.

An absent holder cannot still own an OS advisory lock, so no minimum age is
needed for safety.

The recovery path removes the stale marker while holding the stable guard.

It then releases both locks and returns a failure saying that the stale lock was
recovered.

The message names `.lisa-commit.lock`, its age, recorded PID, and `no such
process`/absent-holder fact.

Returning once instead of silently continuing gives the plugin's existing
stale-lock classifier a real operator-visible reason and consumes at most one
bounded retry.

The next transaction begins from a clean marker path.

Malformed legacy markers have no trustworthy PID.

They can be described as having no recorded holder and recovered because the
transaction owns both advisory locks; the field regression will focus on a
well-formed dead-owner record.

If a parsed PID is still live even though the marker advisory lock is
available, do not claim the owner is absent.

Return a conservative live-owner error and leave the marker unchanged.

## Live-holder behavior

No acquisition failure owns the contested marker.

The RAII value therefore records whether marker cleanup is authorized.

A failed marker lock releases only the guard it acquired.

It does not truncate, unlink, or unlock the other process's marker.

The error includes live-holder metadata when readable and otherwise retains the
existing actionable path plus OS contention detail.

Tests hold the marker file open and locked, snapshot its bytes, and verify both
bytes and pathname remain unchanged after a competing transaction.

## Cleanup model

`TransactionLock::finish` owns ordered cleanup for marker removal, marker
unlock, and guard unlock.

It attempts all applicable operations even if an earlier cleanup operation
fails.

It combines cleanup errors rather than abandoning later releases.

`Drop` calls the same operations best-effort for any early return or panic.

The transaction's explicit cleanup remains responsible for returning failures
to callers.

The existing alternate-index cleanup/rollback composition remains intact.

Representative transaction errors assert visible marker absence.

Direct lock-scope tests exercise drop without explicit finish.

## Discovery option 1: ignore all `git log` failures

Mapping any log failure to `Ok(None)` would fix the field symptom.

It would also hide corrupt repositories, unreadable objects, invalid Git
configuration, and genuine command failures.

Idempotency could then be bypassed and a duplicate completion committed.

This option is rejected.

## Discovery option 2: search all refs

Adding `--all` makes `git log` exit successfully in an empty repository.

It changes idempotency scope from current-HEAD ancestry to every repository
ref.

A matching trailer on an unrelated branch could incorrectly short-circuit the
current completion.

This option is rejected.

## Discovery option 3: explicitly detect unborn HEAD

Ask Git for the symbolic HEAD ref.

If HEAD is symbolic and that exact ref does not exist, the current branch is
unborn.

Return `Ok(None)` immediately in that case.

Otherwise retain the existing `git log` command and strict error propagation.

Detached or malformed HEAD cases do not match the unborn predicate and remain
errors.

This preserves current ancestry semantics and is selected.

## Transaction behavior after empty discovery

No parentless commit support is added by this ticket.

After `Ok(None)`, `run_transaction_body` reaches its existing `resolve HEAD`
operation.

That operation fails in an unborn repository.

The failure is now correctly classified as the actual commit-seal precondition,
not an idempotency lookup failure.

The ticket's later seal-ladder work can change that precondition independently.

The discovery regression asserts both facts.

## Field replay design

Retain the existing plugin scheduler state fixture so the production retry and
park policy remains under test.

Create the fixture root as a real Git repository.

Ensure it has no commits.

Remove local identity and isolate global/system Git configuration for the CLI
calls so the fixture is identity-less regardless of developer machine setup.

Use the actual fixture ticket and private review artifact as the
`complete_ticket` inputs.

Call the CLI transaction, capture its error, and pass that real stderr-equivalent
detail into `handle_completion_result`.

Repeat only for the scheduler-launched retry.

After the second failure, assert the scheduler parked and did not retain or
launch a third completion.

Parse the preserved source journal through `include_str!` and assert it contains
80 rejected rows with the old discovery failure.

This citation makes the test fail if its claimed field evidence disappears or
changes shape.

## Field replay assertions

The new journal contains exactly two `failure-observed` records.

The first has retry-scheduled consequence.

The second has park consequence and count 2/2.

The launch-effect count is exactly two: initial plus one retry.

The canonical disposition is an operator-owned structured block.

Its ask equals `HISTORY_IDENTITY_ASK` exactly.

Provenance contains exactly one Park row.

The ticket is Review/Blocked and its seat is released.

Additional reconciliation cannot launch another completion.

The repository root has no `.lisa-commit.lock` after either failed transaction.

## Compatibility and scope

The public CLI command syntax and success output remain unchanged.

Completion-key trailers and verification remain unchanged.

The ordinary Git index remains isolated exactly as before.

No Cargo dependency is added.

Production code changes stay in `commit_transaction.rs`.

The only plugin change is native regression code connecting its existing policy
to the real CLI transaction boundary.
