# Structure: T-031-01 isolated commit transaction

## Change inventory

### Create `crates/lisa-cli/src/commit_transaction.rs`

This module owns the complete native Git transaction and its tests.

Public-to-crate data structures:

```rust
pub(crate) struct CommitTransactionRequest {
    pub repo_root: PathBuf,
    pub ticket_id: String,
    pub message: String,
    pub includes: Vec<PathBuf>,
}

pub(crate) struct CommitTransactionResult {
    pub commit_id: String,
    pub committed_paths: Vec<PathBuf>,
}

pub(crate) fn commit_ticket(
    request: CommitTransactionRequest,
) -> Result<CommitTransactionResult, CommitTransactionError>;
```

The result retains committed paths for native callers/tests even though the CLI
initially prints only the commit ID.

### Modify `crates/lisa-cli/src/main.rs`

- Declare `mod commit_transaction`.
- Add a `CommitTicket` Clap subcommand.
- Accept `--path`, `--ticket-id`, `--message`, and repeated required `--include`.
- Normalize the repository root using existing `resolve_path`.
- Build `CommitTransactionRequest` and invoke `commit_ticket`.
- Print the commit ID on success.
- Preserve the CLI's existing `Error: ...` plus exit-code-1 convention.

### Modify `crates/lisa-cli/Cargo.toml`

- Add `fs2` as a native dependency for advisory file locks.
- Reuse the existing `tempfile` dev dependency for process test repositories.
- Add no Git library dependency; the installed Git executable remains the source
  of truth for index, object, and ref behavior.

### Create RDSPI artifacts

- `docs/active/work/T-031-01/research.md`
- `docs/active/work/T-031-01/design.md`
- `docs/active/work/T-031-01/structure.md`
- `docs/active/work/T-031-01/plan.md`
- `docs/active/work/T-031-01/progress.md`
- `docs/active/work/T-031-01/review.md`

The ticket file itself is not modified by this work session.

## Internal module organization

### Error type

`CommitTransactionError` is a displayable structured error with variants or
constructors for:

- invalid request/path;
- repository discovery;
- lock open/acquisition;
- Git command failure with operation, status, and stderr;
- foreign staged overlap;
- no ticket changes;
- verification mismatch;
- cleanup/unlock failure;
- combined primary and cleanup failures.

No error requires serialization. `std::error::Error` is implemented so the CLI
can treat it like existing command errors.

### `Repository`

An internal context holds canonical paths and command configuration:

```rust
struct Repository {
    root: PathBuf,
    git_dir: PathBuf,
}
```

Construction runs Git discovery against the requested root and verifies the
resolved top-level directory matches the intended worktree. It resolves the Git
directory using `git rev-parse --absolute-git-dir`, supporting ordinary and
linked worktrees better than assuming `.git` is a directory.

### `GitCommand`

One helper executes Git with:

- `-C <root>`;
- optional `GIT_INDEX_FILE=<alternate-index>`;
- explicit argument arrays, never shell interpolation;
- captured stdout and stderr;
- operation labels for actionable errors.

Separate helpers return trimmed UTF-8 stdout or raw bytes where NUL-delimited
path data must be preserved. Nonzero exit is always an error.

### Path validation

`normalize_includes` validates each input component and produces sorted,
deduplicated repository-relative paths.

Rejected inputs:

- empty paths;
- absolute paths;
- `.` as a whole-repository path;
- parent-directory components;
- platform root/prefix components.

Normal components and harmless current-directory components inside a path are
collapsed. Git path bytes are passed through `OsStr` arguments rather than shell
strings.

### Lock guard

`TransactionLock` opens/creates `<root>/.lisa-commit.lock` and calls
`try_lock_exclusive`.

- It is acquired before snapshots or `HEAD` resolution.
- Explicit `finish()` unlocks and returns unlock errors.
- A defensive `Drop` attempts unlock if explicit completion was skipped.
- The file is never removed.

### Alternate index guard

`AlternateIndex` reserves a unique path inside the resolved Git directory using
process ID plus a creation counter/timestamp and `create_new` semantics.

- The empty reservation file is removed before `read-tree`, because Git treats
  an existing empty file as an invalid index.
- The path is then supplied through `GIT_INDEX_FILE`.
- `cleanup()` removes both the index and `<index>.lock`.
- Missing files are accepted; other removal errors are surfaced.
- `Drop` provides best-effort cleanup after early returns.

### Ordinary staged snapshot

`StagedSnapshot` contains sorted staged paths and the corresponding stage-entry
bytes from `git ls-files --stage -z -- <paths>`.

Snapshot construction:

1. `git diff --cached --name-only -z` obtains staged paths.
2. Empty output produces an empty snapshot.
3. `git ls-files --stage -z -- <paths>` captures blob/mode/stage tuples.

Equality after reconciliation is the success invariant.

### Transaction body

The body is ordered as follows:

1. Validate request and repository.
2. Acquire `TransactionLock`.
3. Read `HEAD` and ordinary `StagedSnapshot`.
4. Create `AlternateIndex` path.
5. Seed alternate index with `read-tree HEAD`.
6. Stage explicit includes with `git add -A -- <includes>`.
   `-A` is scoped after `--` to only the explicit pathspecs, allowing deletions.
7. Read concrete alternate staged paths.
8. Fail if none changed.
9. Fail on intersection with ordinary staged paths.
10. Write the tree.
11. Create the commit with parent `HEAD` and the supplied message.
12. Guardedly advance `HEAD`.
13. Reset only concrete committed paths in the ordinary index to new `HEAD`.
14. Recapture ordinary staged state and compare it to the original snapshot.
15. Clean the alternate index and companion lock.
16. Unlock the transaction lock.
17. Return commit ID and concrete paths.

Cleanup is orchestrated outside the body so it runs after every body result. A
cleanup failure converts success to failure or is appended to a primary error.

## Test module structure

Tests remain in `commit_transaction.rs` so private helpers can be exercised.

### Pure request tests

- accepts and deduplicates ordinary relative paths;
- rejects absolute, parent, empty, and repository-wide paths;
- preserves directory pathspecs without expanding them outside Git.

### `GitRepo` test fixture

An internal fixture:

- owns a `tempfile::TempDir`;
- initializes Git;
- configures local user name/email;
- writes files with `std::fs`;
- runs Git commands with captured assertions;
- creates a base commit;
- reads `HEAD`, commit trees, staged names, and stage entries.

### Process regression test

`foreign_staged_entry_is_preserved_and_excluded`:

- Base-commit `foreign.txt`, `src/ticket.txt`, and a ticket markdown file.
- Modify and stage `foreign.txt`.
- Record its stage entry.
- Modify `src/ticket.txt`.
- Create untracked work artifacts and ticket-owned untracked code.
- Invoke the transaction with only ticket-owned pathspecs.
- Assert `HEAD` advanced once.
- Assert the commit tree contains ticket content and the old foreign content.
- Assert ordinary staged names contain only `foreign.txt`.
- Assert its stage entry is byte-identical.
- Assert committed paths are not ordinary staged entries.
- Assert no alternate index residue remains.

### Failure tests

- staged overlap refuses and leaves `HEAD`/index unchanged;
- held transaction lock returns a contention error;
- unchanged included paths return a no-changes error;
- invalid repository returns an actionable discovery error;
- invalid include returns before lock/Git mutation.

## Architectural boundaries

- `lisa-plugin` remains unchanged in this ticket.
- Provider adapters remain unchanged.
- Ticket parsing and DAG types remain unchanged.
- No phase/status frontmatter update is performed by the transaction module.
- The transaction commits whatever exact frontmatter path/content its future
  caller includes; T-031-02 owns when that content becomes Done.
- No global working-tree scan is used to infer ownership.
- No shell command string is built from user path or message input.

## Implementation ordering

1. Add dependency and module skeleton.
2. Implement validation/errors/Git runner.
3. Implement lock and alternate-index lifecycle.
4. Implement staged snapshots and transaction body.
5. Add process fixtures and regressions.
6. Wire the Clap command.
7. Run focused formatting/lints/tests, then workspace verification.
8. Record actual progress and review findings.
