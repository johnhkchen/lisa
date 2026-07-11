# Review: T-031-01 isolated commit transaction

## Outcome

T-031-01 now provides a native, provider-neutral ticket commit transaction via
`lisa commit-ticket`. The command stages only caller-supplied repository-relative
paths in an alternate Git index, creates a commit from that isolated tree,
guardedly advances `HEAD`, reconciles the committed paths in the ordinary index,
verifies unrelated staged entries are unchanged, cleans temporary state, and
returns the new commit ID only after successful cleanup/unlock.

The transaction was also used to create its own implementation commit:
`4d5b0d890012262e57d69e75b331c9780962235e`.

Scheduler state integration was deliberately not added. Per the ticket notes,
T-031-02 will invoke this command, await its `RunCommandResult`, and gate Done,
seat release, dependent scheduling, and loop completion on success.

## Files created

### `crates/lisa-cli/src/commit_transaction.rs`

New native transaction module containing:

- request, result, and actionable error types;
- repository and absolute Git-directory discovery;
- argv-based Git subprocess execution with captured stderr/status;
- explicit include-path validation and deduplication;
- nonblocking advisory `.lisa-commit.lock` acquisition;
- unique alternate-index reservation and cleanup;
- ordinary staged path and stage-entry snapshots;
- overlap rejection for ticket paths already staged ordinarily;
- alternate-index `read-tree`, scoped `add -A`, and `write-tree`;
- `commit-tree` commit creation;
- guarded `update-ref HEAD <new> <old>` compare-and-swap;
- targeted ordinary-index reset for concrete committed paths;
- before/after foreign staged snapshot verification;
- combined primary-operation and cleanup error reporting;
- eight unit/process-level tests.

### RDSPI artifacts

- `docs/active/work/T-031-01/research.md`
- `docs/active/work/T-031-01/design.md`
- `docs/active/work/T-031-01/structure.md`
- `docs/active/work/T-031-01/plan.md`
- `docs/active/work/T-031-01/progress.md`
- `docs/active/work/T-031-01/review.md`

## Files modified

### `crates/lisa-cli/src/main.rs`

- Registers the transaction module.
- Adds the `commit-ticket` Clap subcommand.
- Accepts `--path`, `--ticket-id`, `--message`, and repeated `--include`.
- Prints the new commit ID on success.
- Uses the existing `Error: ...` plus nonzero-exit CLI convention on failure.

### `crates/lisa-cli/Cargo.toml`

- Adds `fs2 = "0.4"` for portable advisory file locking.

### `Cargo.lock`

- Locks `fs2 0.4.3` and records it as a `lisa-cli` dependency.

## Files not changed

- No `lisa-plugin` scheduler source was changed.
- No Claude or Codex adapter/hook source was changed.
- No `lisa-core` public API was changed.
- The ticket frontmatter was not changed; it remains `phase: research` and
  `status: open` for Lisa to transition from artifact detection.
- Unrelated modified/untracked working-tree files were not included in the
  implementation commit.

## Transaction behavior

The transaction sequence while holding `.lisa-commit.lock` is:

1. Resolve current `HEAD`.
2. Snapshot ordinary staged paths and their `ls-files --stage` bytes.
3. Initialize a unique alternate index from the captured `HEAD`.
4. Stage only explicit includes using pathspec-scoped `git add -A -- ...`.
5. Resolve the concrete changed path set.
6. Reject empty changes or overlap with ordinary staged paths.
7. Write the alternate index to a tree.
8. Create a one-parent commit object with `git commit-tree`.
9. Advance `HEAD` only if it still equals the captured old value.
10. Reset only concrete committed paths in the ordinary index to new `HEAD`.
11. Verify the ordinary staged snapshot exactly matches the original.
12. Remove alternate index/companion lock and release the transaction lock.

The caller must explicitly identify ticket-owned code paths. Git cannot safely
infer per-process ownership from a shared working tree. The future scheduler
integration should also include the exact ticket markdown path and
`docs/active/work/<ticket-id>/` path.

## Acceptance criteria assessment

### Full serialized critical section

Met for all mutating and index-sensitive work: ordinary snapshot, alternate
index creation/staging, tree/commit creation, ref update, reconciliation,
verification, temp cleanup, and unlock occur under the advisory lock.
Repository discovery and pure request validation occur before lock acquisition.

### Ticket-owned staging isolated from ordinary index

The commit tree is constructed exclusively through `GIT_INDEX_FILE`; ticket
content is never added to the ordinary index as a staged source. After `HEAD`
movement, exact ticket paths are reset in the ordinary index before success.
The process regression proves no ticket path is staged on return.

See the atomic visibility concern below for the literal during-transaction
interpretation of this criterion.

### Foreign staged entries preserved and excluded

Met and directly tested. The transaction snapshots staged names plus raw
mode/blob/stage entries, rejects ownership overlap, excludes the foreign content
from the commit tree, and requires exact snapshot equality after reconciliation.

### Modified/untracked code, artifacts, and ticket frontmatter

Met and directly tested. The central process regression commits:

- modified tracked ticket code;
- untracked ticket code;
- an untracked Review artifact under the ticket work directory;
- a modified ticket frontmatter file.

An unrelated untracked file remains outside the resulting tree. There is no
repository-wide `git add -A`; `-A` is applied only after explicit pathspecs.

### Process-level foreign-index regression

Met by `foreign_staged_entry_is_preserved_and_excluded`. It initializes a real
Git process repository, records the foreign stage entry, runs the transaction,
and separately inspects commit content, staged names, staged entry bytes, and
temporary-index residue.

### Actionable lock, commit, and cleanup failures

Lock contention and commit identity failure are directly tested. Git errors name
the failed operation, status, and stderr. Cleanup/unlock errors are propagated
and convert an otherwise successful body into failure; combined failures report
both primary and cleanup problems.

Actual filesystem cleanup failure injection is not tested because it is
platform/permission-sensitive. Cleanup control flow is covered indirectly by
success, no-change, overlap, and commit-failure tests, all of which assert or
depend on a usable ordinary index and no alternate-index residue.

### Provider neutrality

Met. The mechanism is a native Lisa CLI command and contains no provider hook,
prompt, client, or adapter dependency.

### Focused and workspace suites

Met. See verification evidence below.

## Test coverage

Eight focused tests cover:

1. include normalization/deduplication;
2. unsafe include rejection;
3. modified/untracked ticket commit with foreign staged preservation;
4. ordinary staged overlap rejection without mutation;
5. lock contention with actionable error;
6. unchanged include rejection without `HEAD` movement;
7. commit failure with actionable identity error, unchanged `HEAD`, usable index,
   and no temporary residue;
8. invalid repository discovery error.

The main regression asserts both sides of the isolation boundary: the ticket
commit retains the base foreign content, while the ordinary index retains the
new foreign staged blob/mode entry byte-for-byte.

## Verification evidence

- `cargo test -p lisa-cli commit_transaction`: 8 passed.
- `cargo clippy -p lisa-cli --bin lisa -- -D warnings`: passed.
- `cargo test --workspace`: passed.
- Final `just check`: passed:
  - `cargo check -p lisa-plugin --target wasm32-wasip1` passed;
  - 263 CLI tests passed;
  - 145 core tests passed;
  - 234 plugin tests passed;
  - doc tests passed.
- `cargo run -q -p lisa-cli -- commit-ticket --help`: passed and displays the
  expected command contract.
- `git diff --check` on ticket-owned changes: passed.
- The implementation transaction committed only its nine explicit source,
  dependency, lockfile, and work-artifact paths.

## Open concerns and limitations

### Critical: `HEAD` and ordinary index are separate filesystem updates

Git provides atomic ref updates and atomic index-file replacement separately,
but no atomic operation spanning both. This implementation updates `HEAD` and
then performs a targeted ordinary-index reset. Between those two operations, an
uncooperative process that ignores `.lisa-commit.lock` could compare the old
ordinary index to new `HEAD` and momentarily interpret ticket paths as reverse
staged changes.

No ticket content is ever written to the ordinary index as a staged source, and
the state is reconciled/verified before the command returns. Cooperating Lisa
transactions are fully serialized. A human reviewer should decide whether the
ticket's phrase "including while the transaction is in progress" requires a
stronger cross-file atomicity guarantee than Git exposes for a shared worktree.
If it does, the architecture likely needs per-ticket worktrees/indexes or a
repository-wide requirement that every index observer honor the same lock.

### Scheduler integration remains required

The command is not yet called by the plugin. Until T-031-02 lands, current
artifact detection can still mark tickets Done and release seats without a
commit. This is expected ticket decomposition, not a hidden completion claim.

### Ownership inputs remain a caller responsibility

The CLI refuses broad implicit ownership and accepts explicit `--include`
pathspecs. T-031-02 must define how ticket code paths are obtained. Passing a
directory intentionally includes all changes below that directory.

### UTF-8 path assumption

NUL-delimited Git path output is decoded as UTF-8. Repositories containing
non-UTF-8 path bytes will receive an actionable error rather than silent
misinterpretation. Supporting arbitrary Unix path bytes would require
platform-specific `OsStringExt` parsing.

### Existing commit required

The transaction seeds from and parents current `HEAD`; an unborn repository is
rejected by `rev-parse --verify HEAD`. Lisa-managed projects are expected to
already have history, but this is not separately documented in CLI help.

### Git hooks/signing

`git commit-tree` does not run normal `git commit` hooks and this command does
not request GPG/SSH signing. The ticket did not require either behavior. Projects
that mandate hook/signature policy need a follow-up design before adopting the
transaction as their completion authority.

### Cleanup fault injection gap

The error path exists and never reports cleanup failure as success, but there is
no deterministic test that forces unlink or unlock failure on every supported
platform. A future injectable filesystem/lock abstraction could close this gap
if the added complexity is warranted.

## Reviewer focus

Human review should concentrate on:

- whether the explicit path ownership contract is sufficient for T-031-02;
- whether the tiny ref/index reconciliation window satisfies the literal
  visibility criterion;
- whether bypassing commit hooks/signing is acceptable for Lisa completion
  commits;
- how T-031-02 retries lock contention and surfaces post-ref cleanup failure;
- ensuring every current Review-to-Done path is gated through one result state.

No other known correctness issues or TODO markers remain in the implementation.
