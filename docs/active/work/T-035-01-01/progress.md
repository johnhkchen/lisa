# T-035-01-01 Progress — process-start signal producer

## Completed

- Read project, ticket, story, epic, and RDSPI workflow context.
- Mapped native Claude/Codex startup configuration and lease marker production.
- Wrote Research, Design, Structure, and Plan artifacts to the attempt-private directory.
- Added a shared `ON_START_HOOK` POSIX template.
- Bound both native configurations to `SessionStart[startup]`.
- Preserved the existing separate `SessionStart[clear]` behavior.
- Extended both hook merge paths idempotently.
- Added init materialization and validation for `.lisa/hooks/on-start.sh`.
- Added executable fixtures for matching, stale, mismatched, invalid, and absent starts.
- Verified the matching signal contains the exact attempt lease bytes.
- Committed the CLI hook/configuration unit with Lisa's isolated transaction.
- Added immutable `attempt_id` to the native spawn context.
- Exported `LISA_ATTEMPT_ID` in fresh Claude and Codex launch commands.
- Populated fresh launches directly from the scheduler-minted `AttemptLease`.
- Updated provider command and dispatch fixtures.
- Committed the launch-identity unit with Lisa's isolated transaction.

Commit:

- `e4f812d feat: scaffold native process-start signal`
  - `crates/lisa-cli/src/templates.rs`
  - `crates/lisa-cli/src/init.rs`
- `7379efd feat: bind native starts to attempt identity`
  - `crates/lisa-plugin/src/adapter.rs`
  - `crates/lisa-plugin/src/lib.rs`

## Verification completed

- `cargo test -p lisa-cli templates`: 32 relevant tests passed.
- Targeted init count regression: passed after updating the managed-file count.
- Targeted executable start fixture: passed.
- `git diff --check` for the CLI source unit: passed.
- `cargo test -p lisa-plugin`: 276 tests passed.
- `cargo fmt --all -- --check`: passed after both parallel source units landed.
- `cargo test --workspace`: passed across CLI, core, plugin, integration, and doc tests.
- Existing Codex acknowledgment, stale lease, split-brain, and recovery tests passed.

## Remaining

- Write Review artifact and remain on this ticket for Lisa's completion commit.

## Deviations and concurrency notes

- The shared worktree acquired concurrent T-035-01-02 changes in
  `crates/lisa-plugin/src/adapter.rs` and `crates/lisa-plugin/src/lib.rs` during this
  implementation. Those files are also required for this ticket's launch identity.
- No concurrent source was included in the first commit.
- T-035-01-02 committed its transport implementation and fixture correction independently
  before this ticket committed the attempt-identity delta on top.
- No source from the parallel ticket was included in either T-035-01-01 commit.
