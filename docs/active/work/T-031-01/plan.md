# Plan: T-031-01 isolated commit transaction

## Goal

Deliver a native, provider-neutral ticket commit transaction that serializes its
entire operation, never constructs the ticket commit from the ordinary index,
preserves unrelated staged entries, accepts explicit modified/untracked ticket
paths, and reports every lock/Git/verification/cleanup failure.

Scheduler completion gating remains for T-031-02.

## Step 1: establish the native module boundary

- Add `fs2` to `lisa-cli` dependencies.
- Add `mod commit_transaction` in the CLI entry point.
- Create `commit_transaction.rs` with request/result/error types.
- Implement `Display` and `Error` for the transaction error.

Verification:

- `cargo check -p lisa-cli` resolves the new dependency and module.
- No plugin/WASM code depends on the native lock/process module.

Atomic unit: dependency plus compiling module skeleton.

## Step 2: validate explicit ownership inputs

- Implement include path normalization.
- Reject empty paths, absolute paths, repository-wide `.`, parent traversal, and
  platform prefixes/roots.
- Sort and deduplicate accepted includes.
- Reject empty ticket IDs and commit messages.
- Add pure tests for accepted and rejected cases.

Verification:

- Focused path-validation tests pass.
- Inputs are passed to Git as argv/path values, never shell interpolated.

Atomic unit: request validation with unit coverage.

## Step 3: implement repository and Git command helpers

- Resolve the repository top level and absolute Git directory through
  `git rev-parse`.
- Require the requested root to address the repository worktree.
- Centralize `git -C` process invocation.
- Support optional `GIT_INDEX_FILE` per command.
- Return stdout bytes for NUL path parsing and strings for object IDs.
- Include operation name, exit status, and stderr on subprocess failure.

Verification:

- Invalid repository test is actionable.
- Helper tests/fixture can initialize and query a temporary repository.

Atomic unit: native Git execution boundary.

## Step 4: serialize and isolate temporary state

- Implement `.lisa-commit.lock` open/create plus nonblocking exclusive lock.
- Keep the lock handle alive across the full transaction.
- Implement explicit unlock and best-effort Drop unlock.
- Reserve a unique alternate-index path in the Git directory.
- Remove the empty reservation before Git initialization.
- Implement explicit cleanup of index and companion `.lock`.
- Add held-lock regression coverage.

Verification:

- A second transaction attempt fails with a lock-contention message.
- No temporary index remains after ordinary pre-commit failures.

Atomic unit: lock and temp lifecycle.

## Step 5: snapshot and compare the ordinary staged index

- Collect staged paths with a NUL-delimited cached diff.
- Capture corresponding `ls-files --stage` bytes.
- Sort/normalize representations where necessary for deterministic equality.
- Implement overlap detection between ordinary staged paths and concrete ticket
  changes from the alternate index.
- Add overlap refusal coverage.

Verification:

- An ordinarily staged requested file causes failure before `HEAD` changes.
- The staged snapshot remains identical after refusal.

Atomic unit: foreign-index invariant and ambiguity guard.

## Step 6: implement the Git transaction

- Capture old `HEAD` while holding the lock.
- Initialize alternate index from old `HEAD`.
- Run pathspec-scoped `git add -A -- <includes>`.
- Obtain concrete changed ticket paths.
- Reject an empty transaction and staged overlap.
- Write the tree and create a commit object with one parent.
- Guardedly advance `HEAD` from the captured old ID to the new commit ID.
- Reset only concrete committed paths in the ordinary index to new `HEAD`.
- Verify the ordinary staged snapshot exactly matches its pre-transaction state.
- Return commit ID and committed paths only after cleanup/unlock succeed.

Verification:

- Modified files, untracked files, and deletions under explicit pathspecs can be
  represented in the commit tree.
- No unrelated working-tree path enters the commit.
- Empty changes fail without moving `HEAD`.

Atomic unit: end-to-end transaction body.

## Step 7: add the required process-level regression

- Initialize a real temporary Git repository with a base commit.
- Stage a foreign modification in the ordinary index.
- Record its `ls-files --stage` bytes.
- Create modified and untracked ticket-owned code.
- Create the ticket work directory/artifacts and a modified ticket file.
- Invoke `commit_ticket` with explicit ticket-owned includes.
- Inspect the resulting commit tree/content.
- Inspect the ordinary index staged names and stage bytes.
- Assert foreign content is excluded from the commit.
- Assert ticket content is not left staged ordinarily.
- Assert unrelated unstaged content remains uncommitted.
- Assert temporary index cleanup.

Verification:

- This single test directly covers the field regression and principal acceptance
  criteria using separate Git processes.

Atomic unit: central process regression.

## Step 8: expose the CLI contract

- Add `CommitTicket` to the Clap command enum.
- Require ticket ID, message, and one or more includes.
- Resolve `--path` consistently with other commands.
- Dispatch into `commit_ticket`.
- Print the new commit ID to stdout.
- Preserve nonzero actionable error behavior.
- Add a Clap parsing test if command construction cannot be sufficiently covered
  through existing derive behavior.

Verification:

- `cargo run -p lisa-cli -- commit-ticket --help` documents the contract.
- A manual/temp invocation exits zero with a commit ID on success.
- Lock/commit errors exit nonzero.

Atomic unit: callable provider-neutral host command.

## Step 9: focused quality checks

- Run `cargo fmt --all -- --check` after formatting changed Rust files.
- Run `cargo test -p lisa-cli commit_transaction`.
- Run `cargo clippy -p lisa-cli --all-targets -- -D warnings` if compatible with
  the workspace's current warning baseline.
- Inspect `git diff` to ensure only ticket files and artifacts changed.

Verification:

- All focused tests pass.
- No provider-specific files or scheduler completion paths changed.

Atomic unit: focused validation and cleanup.

## Step 10: workspace verification

- Run `cargo test --workspace`.
- Run `just check`, which includes the WASM check and native tests.
- If an environment/toolchain prerequisite prevents a command, record the exact
  failure and retain the narrower passing evidence.
- Re-run the focused transaction regression after any fix.

Verification:

- Relevant workspace suites pass.
- WASM build remains unaffected by the native-only module.

Atomic unit: repository-wide regression confidence.

## Step 11: implementation tracking and handoff

- Create `progress.md` before source edits and update it after each meaningful
  unit, noting deviations before applying them.
- Commit incrementally only if exact-path commits can be made without consuming
  the user's unrelated worktree/index state.
- Do not modify ticket phase/status frontmatter.
- Create `review.md` summarizing actual files, behavior, test evidence, coverage
  gaps, open concerns, and critical issues.

Verification:

- All six RDSPI artifacts exist under `docs/active/work/T-031-01/`.
- Review explicitly maps outcomes to acceptance criteria.
- The ticket frontmatter remains `phase: research`, `status: open` in this
  session, leaving phase transitions to Lisa.
