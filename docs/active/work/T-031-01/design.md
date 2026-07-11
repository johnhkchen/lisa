# Design: T-031-01 isolated commit transaction

## Decision summary

Add a native `lisa commit-ticket` command backed by a focused
`lisa-cli::commit_transaction` module. The module will acquire the repository's
`.lisa-commit.lock`, construct the ticket tree in a temporary alternate Git
index from explicit pathspecs, create a commit with Git plumbing commands,
atomically advance `HEAD`, reconcile only committed ticket paths in the ordinary
index, verify foreign staged entries, remove temporary state, and then release
the lock.

The scheduler will invoke this provider-neutral command and consume its exit
status in T-031-02. This ticket does not alter phase transition or seat release
logic.

## Option 1: shell wrapper injected into agent/provider instructions

The workflow could teach Claude and Codex to run a `flock` plus Git command
sequence at the end of each ticket.

Advantages:

- Minimal Rust code.
- Commands are easy to experiment with manually.
- Agents already have shell access to the repository.

Disadvantages:

- Provider instructions become part of the correctness boundary.
- Different agent clients can interpret or omit the sequence differently.
- The scheduler receives no structured result and cannot reliably gate release.
- Quoting explicit path lists and preserving cleanup errors is fragile.
- It duplicates logic that T-031-03 explicitly intends to keep provider-neutral.

Decision: rejected. The ticket requires one Lisa-owned provider-neutral
transaction with a clear success/failure result.

## Option 2: perform Git operations directly in the WASM plugin

The plugin could manipulate index/tree/ref files itself or attempt to expose
host Git through Zellij calls for every transaction step.

Advantages:

- Scheduler state and commit state would live in one component.
- No additional CLI command would be visible.

Disadvantages:

- The WASI plugin cannot use `std::process::Command` for Git.
- Multiple asynchronous `run_command` calls would require a large state machine
  and would not hold one native process lock across the sequence.
- Implementing Git object/ref/index formats in the plugin would be unsafe scope.
- OS file locking support in the WASM target is not the established boundary.

Decision: rejected. Existing architecture uses the native Lisa binary for host
operations and gives the plugin a path to that binary.

## Option 3: implement the transaction in `lisa-core`

The shared crate could expose a transaction API used by the CLI and potentially
other native callers.

Advantages:

- A reusable library API.
- Separation from Clap command parsing.

Disadvantages:

- `lisa-core` is compiled into the WASM plugin.
- Native process and locking dependencies would require target-gated exports and
  dependencies in an otherwise platform-neutral crate.
- The only current consumer is the native CLI boundary.
- A core placement suggests availability to WASM callers that cannot execute it.

Decision: rejected for this ticket. The transaction belongs in a dedicated CLI
module until a second native library consumer exists.

## Option 4: native CLI transaction using ordinary porcelain index

The command could acquire a lock, run explicit `git add`, commit, and restore a
saved `.git/index` file.

Advantages:

- Familiar Git commands.
- Straightforward happy path.

Disadvantages:

- Ticket paths become observable in the ordinary index during staging.
- `git commit` can consume foreign staged entries.
- Copying/restoring the index around a moving `HEAD` leaves ticket paths staged
  in reverse relative to the new commit.
- Crash windows expose the shared index as a mailbox.

Decision: rejected. It fails the primary isolation criteria.

## Option 5: native CLI transaction using an alternate index

The command can set `GIT_INDEX_FILE` for all preparation commands, seed it from
`HEAD`, add only explicit paths, and create the commit from that tree.

Advantages:

- Ticket content is never used as a staged change in the ordinary index.
- Foreign staged entries cannot enter the alternate tree.
- Explicit pathspecs include tracked modifications and untracked files.
- Git itself continues to handle ignores, file modes, object creation, and refs.
- A process exit code is directly consumable by the scheduler.

Disadvantages:

- Moving `HEAD` changes how the ordinary index compares to the branch.
- Ticket paths require targeted post-ref reconciliation.
- Overlap with a pre-existing foreign staged path is inherently ambiguous.
- Cleanup and verification need deliberate error handling.

Decision: chosen. It matches the native host boundary and gives the strongest
isolation available in a shared worktree.

## Commit construction decision

Use Git plumbing:

1. Resolve and capture the current `HEAD` object ID.
2. Create a unique alternate index path under the repository Git directory.
3. Run `git read-tree HEAD` with `GIT_INDEX_FILE` set.
4. Run `git add -- <explicit pathspecs>` with the alternate index.
5. Obtain concrete changed paths from the alternate index.
6. Reject any overlap with ordinary staged paths.
7. Run `git write-tree` with the alternate index.
8. Run `git commit-tree <tree> -p <old-head> -m <message>`.
9. Run `git update-ref HEAD <new-commit> <old-head>`.
10. Reconcile committed paths in the ordinary index with
    `git reset --quiet HEAD -- <concrete paths>`.

`commit-tree` is chosen over `git commit` because it never needs the ordinary
index, exposes the exact tree input, and separates object creation from guarded
ref movement. `update-ref`'s old-value argument prevents silently committing on
top of an unexpected concurrent `HEAD`.

## Lock decision

Use an OS advisory exclusive lock on `<repo>/.lisa-commit.lock` for the entire
transaction. Open/create the file once and use a nonblocking exclusive lock.

Nonblocking acquisition is chosen so a scheduler attempt receives an immediate,
actionable contention result rather than hanging an event indefinitely. The
caller may retry according to scheduler policy in T-031-02.

The critical section begins before reading `HEAD` or the ordinary staged set and
ends only after ref update, index reconciliation, verification, alternate-index
cleanup, and explicit unlock. The lock inode remains on disk for reuse.

## Pathspec decision

The command accepts repeated `--include <repo-relative-path>` arguments.

- Paths must be nonempty and repository-relative.
- Absolute paths, `..` components, and the repository-wide `.` path are rejected.
- Duplicate normalized paths are removed.
- No implicit `git add -A` or `git add .` occurs.
- Directories are allowed; Git expands them into concrete paths in the alternate
  index.
- Missing/deleted tracked paths remain valid Git pathspecs.
- A transaction with no resulting changes fails rather than creating an empty
  completion commit.

T-031-02 can pass the ticket's exact frontmatter path, work directory, and the
ticket-owned code paths established by its integration mechanism. Ownership of
arbitrary code changes cannot be inferred safely by Git in a concurrent tree.

## Foreign staged entry preservation

Before preparation, capture:

- The exact list of ordinary staged paths from `git diff --cached`.
- Each staged path's `git ls-files --stage` representation.

After staging in the alternate index, compare its concrete changed paths with
the ordinary staged set. Any intersection fails before commit creation.

After ref movement and targeted reset, recapture the ordinary staged set and
stage representations. Success requires equality with the original snapshot.
This proves:

- No foreign staged path entered the ticket commit.
- No foreign staged entry changed blob ID, mode, or stage.
- No ticket path remains as an ordinary staged change.

The process-level regression will additionally inspect the new commit tree and
working ordinary index independently.

## Cleanup and failure semantics

All Git subprocess failures return an error naming the operation, exit status,
and trimmed stderr. No failed operation is mapped to success.

The module will structure the operation so temporary-index cleanup and unlock
are attempted after both success and failure. If primary work succeeds but
cleanup or unlock fails, the public result is failure. If both primary work and
cleanup fail, the error reports both facts.

The temporary index and its possible `.lock` sibling are explicitly removed.
An already-absent temporary file is successful cleanup. The persistent
`.lisa-commit.lock` is not deleted because deleting a locked inode permits a
second process to lock a replacement inode.

After `update-ref`, reconciliation failure is serious: the commit may exist and
`HEAD` may have advanced, but the command returns failure with the new commit ID
in context where possible. T-031-02 must treat any nonzero result as not safely
completed and surface it for operator recovery.

## Public command contract

Proposed command:

```text
lisa commit-ticket \
  --path <repository-root> \
  --ticket-id <ticket-id> \
  --message <commit-message> \
  --include <path> [--include <path> ...]
```

`ticket-id` supplies correlation and diagnostics; it is not used to discover
code ownership. On success stdout prints the new commit object ID. On failure
the existing CLI convention prints `Error: ...` to stderr and exits nonzero.

## Test strategy decision

Place unit and process-level tests beside the CLI transaction module.

- Pure tests validate path normalization and rejection.
- A real temporary Git repository validates modified and untracked ticket paths.
- The central regression stages a foreign entry, records its stage tuple, runs a
  ticket transaction, and inspects both the resulting commit and ordinary index.
- A lock-contention test holds `.lisa-commit.lock` and checks actionable failure.
- An overlap test stages a requested path ordinarily and verifies refusal before
  `HEAD` moves.
- An empty-change test verifies commit preparation failure.
- CLI parsing coverage ensures repeated includes reach the transaction request.

Focused `cargo test -p lisa-cli commit_transaction` runs during implementation,
followed by `cargo test --workspace` and the repository's `just check` if the
environment has the WASM target and required tools.
