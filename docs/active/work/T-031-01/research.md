# Research: T-031-01 isolated commit transaction

## Ticket scope

- T-031-01 is a critical bug in story S-031, atomic ticket completion.
- The current ticket supplies a commit transaction boundary.
- T-031-02 is explicitly responsible for integrating that boundary with scheduler
  completion and dependent release.
- T-031-03 covers provider instructions and a live mixed-provider regression.
- This ticket therefore needs an executable, provider-neutral Git primitive and
  process-level regression coverage, but should not change scheduler state flow.
- The ticket begins in `phase: research` and has no existing work artifacts.
- The requested path `docs/active/tickets/T-031-01.md` is not present; the ticket
  is stored as `docs/active/tickets/T-031-01-isolated-commit-transaction.md`.

## Repository organization

- The workspace contains `lisa-core`, `lisa-cli`, and `lisa-plugin` crates.
- `lisa-core` owns shared data types, ticket parsing, DAG computation, routing,
  diagnostics, and provenance serialization.
- `lisa-cli` is a native executable and already owns host process execution for
  doctor, loop startup, agent wrappers, and project initialization.
- `lisa-plugin` is a Zellij WASM plugin and owns polling, scheduling, pane reuse,
  phase transitions, and the dashboard.
- The root `CLAUDE.md` is the project context source of truth.
- The source layout documented there still names a removed `scheduler.rs`; the
  scheduler currently lives in the large `crates/lisa-plugin/src/lib.rs`.

## Current completion flow

- `State::check_artifact_advances` polls each running thread's artifact path.
- Research, Design, Structure, and Plan use their phase artifact filenames.
- Implement completion is signaled by `review.md`, because `progress.md` is a
  living document rather than a terminal artifact.
- The method loops, allowing a full artifact set to advance through multiple
  phases in one poll.
- It writes frontmatter directly through `ticket::update_ticket_phase`.
- `State::auto_complete_review` changes phase to Done and status to Done.
- It then completes the thread, emits provenance, releases the pane slot, and
  allows the next polling pass to rebuild the DAG and schedule dependents.
- Manual completion has a parallel path in `mark_ticket_done`.
- Idle-signal handling also contains phase/update paths, including direct
  Review-to-Done completion.
- None of those paths currently creates a Git commit.
- None waits for a host command result before releasing scheduler state.
- T-031-02 will need to consolidate or gate those paths; that is outside this
  ticket's stated boundary.

## Existing host-command boundary

- The plugin requests Zellij `PermissionType::RunCommands`.
- It subscribes to `EventType::RunCommandResult`.
- `fire_notify` demonstrates a host command invocation with explicit argv,
  environment, cwd, and a context map for result correlation.
- `Event::RunCommandResult` currently recognizes only `lisa_notify` context.
- The plugin records the absolute host project root from
  `get_plugin_ids().initial_cwd`.
- Loop layout generation passes the native Lisa executable path to the plugin as
  `lisa_bin`.
- `PluginConfig` already parses and retains that executable path.
- These pieces allow T-031-02 to invoke a native Lisa transaction without adding
  provider-specific hooks or attempting subprocess work inside WASM.

## Native CLI boundary

- `crates/lisa-cli/src/main.rs` uses Clap and a `Commands` enum.
- Each command dispatches to a module-level function and turns an error into a
  nonzero process exit with an `Error:` prefix.
- Native commands already receive explicit project roots and normalize relative
  paths with `resolve_path`.
- The CLI can use `std::process::Command`, OS file locking, and ordinary Git
  executable behavior unavailable to the WASM plugin.
- There is no current commit-related CLI command or module.
- A native subcommand is therefore the existing architectural seam for a host
  transaction callable by the scheduler.

## Existing lock references

- `.lisa-commit.lock` is ignored at repository root.
- The plugin passes `/host/.lisa-commit.lock` only to startup diagnostics.
- Current source contains no acquisition of that lock and no commit wrapper.
- Archived research and reviews also note that the lock path was diagnostic-only.
- The ticket context's statement that Lisa serializes commit calls describes the
  desired/field workflow, not an effective lock implementation in current code.

## Git index behavior

- Git normally stages through `.git/index`, shared by every process in one worktree.
- A commit made against that index consumes every staged entry, regardless of
  which process produced it.
- `GIT_INDEX_FILE` directs index-aware Git commands to a different index file.
- An alternate index can be initialized from the current `HEAD` with
  `git read-tree HEAD`.
- `git add -- <explicit paths>` against that index includes modified tracked files
  and untracked files without inspecting unrelated paths.
- `git write-tree` materializes exactly the alternate index as a tree object.
- `git commit-tree <tree> -p <old-head>` creates a commit object without using or
  rewriting the ordinary index.
- `git update-ref HEAD <new> <old>` advances the checked-out branch with a
  compare-and-swap guard against an unexpected concurrent HEAD change.
- These plumbing commands separate staging/tree construction from ref movement.

## Ordinary-index reconciliation constraint

- Advancing `HEAD` while leaving the ordinary index untouched changes the meaning
  of ordinary index entries relative to `HEAD`.
- For ticket paths, old index entries would appear as reverse staged changes after
  the new commit even though the transaction never staged ticket content there.
- Targeted `git reset --quiet HEAD -- <ticket paths>` reconciles only ticket paths
  after the ref update.
- Unrelated staged entries remain in the ordinary index.
- Their staged blob IDs and modes can be compared before and after with
  `git ls-files --stage`.
- A pre-existing staged entry that overlaps a requested ticket path cannot both
  be preserved and replaced by the ticket commit.
- Such overlap must be rejected before preparation.
- The full lock must cover overlap detection, alternate-index creation, staging,
  commit creation, ref update, index reconciliation, verification, and cleanup.

## Path ownership boundary

- Git cannot infer which concurrent process owns an arbitrary working-tree change.
- The transaction therefore needs explicit ticket-owned code paths as input.
- The ticket file and `docs/active/work/<ticket-id>/` are deterministically owned
  paths and can be included by the caller or convenience layer.
- Explicit pathspecs avoid `git add -A`, `git add .`, and status-wide capture.
- Requested paths must be repository-relative and must not escape with `..`.
- Empty paths and repository-wide `.` should be rejected.
- Git expands directory pathspecs into concrete staged entries in the alternate
  index; overlap checks should use the resulting staged path set as well as the
  initial explicit paths.

## Failure boundaries

- Lock creation/open and lock contention are distinct actionable failures.
- Git discovery or missing `HEAD` must fail before any mutation.
- Preparation failures occur before ref movement and need only temp cleanup.
- `update-ref` failure leaves `HEAD` unchanged and must not report success.
- Once `HEAD` advances, ordinary-index reconciliation is mandatory.
- Cleanup failure after ref movement cannot safely be described as success even
  though a commit object/ref may already exist.
- Errors should identify the failed Git operation and include stderr.
- The lock file itself can remain as an empty reusable inode; the OS lock, not
  lock-file deletion, supplies serialization.
- Alternate index files and their `.lock` companions must be removed on every exit.

## Test patterns and constraints

- Workspace tests use `tempfile` heavily for isolated filesystem fixtures.
- Process tests can initialize a real temporary Git repository and configure a
  local author identity.
- A core regression should create a base commit, stage a foreign file, modify
  ticket code, create untracked work artifacts, and execute one transaction.
- The resulting commit tree can be inspected with `git show`/`git ls-tree`.
- The ordinary staged entry can be inspected with `git diff --cached` and
  `git ls-files --stage` before and after.
- The test should assert the ticket paths are not left staged after reconciliation.
- Focused tests should also cover overlap rejection, invalid paths, lock
  contention, commit failure, and cleanup-related reporting where injectable.
- The workspace currently has many unrelated modified and untracked user files.
- Implementation must avoid broad formatting/staging and preserve those changes.

## Constraints carried into Design

- Provider neutrality requires no Claude or Codex hooks.
- The implementation needs to be callable by the native Lisa executable.
- Scheduler state integration is deferred to T-031-02.
- Ticket path ownership must be explicit rather than inferred from the shared tree.
- The ordinary index must never be used to construct the ticket tree.
- Foreign staged entries must survive with the same staged object/mode.
- No successful return is allowed for lock, Git, verification, or cleanup failure.
- Ticket frontmatter phase/status must not be manually changed during this RDSPI run.
