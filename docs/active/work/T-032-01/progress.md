# Progress: T-032-01 Zellij pane lifecycle names

## Status

Implementation, automated verification, and the isolated source commit are complete.
Live-environment inspection found no safe post-change validation session. Review remains.

## Completed

- Read `CLAUDE.md`, `AGENTS.md`, the ticket, and the RDSPI workflow.
- Mapped scheduler, route, adapter, completion, release, and Zellij API boundaries.
- Wrote `research.md`.
- Evaluated adapter-owned, poll-reconciled, and scheduler-mutation designs.
- Chose a single pure formatter plus cached scheduler rename gate.
- Wrote `design.md`, `structure.md`, and `plan.md`.
- Created `crates/lisa-plugin/src/pane_name.rs`.
- Added assigned and idle semantic inputs to one formatter.
- Documented an 80-Unicode-scalar normal display bound.
- Added control/whitespace sanitization and `untitled` fallback.
- Added title-only ellipsis truncation preserving agent and ticket ID.
- Added state-level last-applied names keyed by physical pane ID.
- Added a single deduplicating `rename_terminal_pane` call site.
- Added initial `lisa · idle` naming after application-state permission is available.
- Added assigned naming before `/exit`, `/clear`, or fresh launch input.
- Added resident-provider or empty-shell idle naming at actual slot release.
- Added clean-shell idle naming when an exited provider loses its pending ticket.
- Added lifecycle tests for fresh launch, same-provider reuse, cross-provider switch,
  release, empty shell, clean-shell recovery, and deduplication.
- Added actual-vs-requested fallback routing coverage at the scheduler naming boundary.
- Extended commit success/failure tests with pane-name assertions.

## Deviations from Plan

During integration, slot discovery was found able to precede the permission-result event.
Calling the rename API immediately from discovery could therefore run before
`ChangeApplicationState` permission was granted and never be retried because slot discovery
is one-shot. The implementation adds `name_unnamed_idle_slots` and invokes it from both safe
orderings: discovery when permission is already granted, and permission grant when slots are
already known. This is within the planned discovery scope and closes an ordering race.

The last-applied cache remains on `State` rather than `AgentSlot`, as decided before Structure.
It avoids mechanical changes to the large existing slot-fixture set and is still keyed to the
physical pane lifecycle.

## Focused verification completed

Command:

`cargo test -p lisa-plugin pane_name`

Result: 6 passed, 0 failed.

Coverage includes exact formats, both idle forms, control sanitization, whitespace collapse,
empty title fallback, exact limit, Unicode truncation, and scan-key preservation.

Command:

`cargo test -p lisa-plugin pane_title`

Result: 6 passed, 0 failed.

Coverage includes rename deduplication, fresh actual-route fallback, same-provider reuse,
cross-provider switch, resident/empty release, and missing-ticket clean-shell recovery.

Command:

`cargo test -p lisa-plugin artifact_completion_publishes_only_after_verified_commit_result`

Result: 1 passed, 0 failed. Verified successful durable completion changes the retained
Codex session name to `codex · idle` only after release.

Command:

`cargo test -p lisa-plugin failed_manual_completion_retries_without_early_release_or_duplicate_provenance`

Result: 1 passed, 0 failed. Verified failed completion retains the assigned name, while the
successful retry changes it to idle.

## Remaining

- Inspect exact diff and ordinary index.
- Commit the two source paths through `lisa commit-ticket`.
- Write `review.md`.

## Full verification completed

- `cargo fmt --all`: completed; ticket source is formatted.
- `cargo test --workspace`: passed. The plugin suite reports 250 passed, 0 failed; CLI and
  core suites also passed, including the provider-contract integration test.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`: passed.
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`: passed with no warnings.
- `git diff --check -- crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/pane_name.rs`:
  passed with no whitespace errors.

## Live validation status

Environment inspection found:

- Zellij 0.44.3 is installed.
- Both `/Users/johnchen/.local/bin/claude` and `/Users/johnchen/.local/bin/codex` exist.
- The current session is `nautical-piano` and contains this Codex pane plus another active
  ticket pane.
- `zellij action dump-layout` shows the session is running a cached temporary WASM path that
  predates this implementation.

The post-change mixed-provider sequence was not run. Reloading/replacing the active session's
plugin would disrupt this ticket and T-031-03, while starting a second fully authenticated
multi-ticket loop from inside the current Zellij session is not a safe isolated validation.
This acceptance item remains open and is explicitly carried into Review; automated lifecycle
tests are not presented as live evidence.

## Commit tooling

The installed `/opt/homebrew/bin/lisa` reports `commit-ticket` as an unknown subcommand, so it
predates the isolated transaction feature. The repository-built CLI exposes the required
command (`cargo run -p lisa-cli -- commit-ticket --help`). The source commit will therefore
invoke that repository-built command with the same exact-path transaction contract.

## Source commit completed

Command:

`cargo run -q -p lisa-cli -- commit-ticket --ticket-id T-032-01 --message "Name Zellij panes across scheduler lifecycle" --include crates/lisa-plugin/src/lib.rs --include crates/lisa-plugin/src/pane_name.rs`

Result: commit `cd9257d5a9499068d6ba61c16f24e502bc7e70a7`.

Post-commit verification:

- Commit contains exactly `crates/lisa-plugin/src/lib.rs` and
  `crates/lisa-plugin/src/pane_name.rs`.
- Both ticket-owned source paths are clean.
- `git diff --cached --name-only` is empty; the ordinary index was not used.
- Pre-existing unrelated modified and untracked paths remain outside the commit.
- Ticket and work artifacts remain untracked for Lisa's completion transaction, as required.

## Final remaining work

- Write `review.md` with the live-validation gap flagged for human attention.
