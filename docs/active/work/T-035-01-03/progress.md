# T-035-01-03 Progress — gate Owned on observed start

## Status

Implementation is complete and committed. All planned source behavior and acceptance
coverage landed as one coherent scheduler contract unit.

## Completed implementation

- [x] Added `SeatAssignmentState::Starting { generation }`.
- [x] Kept `seat_is_owned` strict: only `Owned` is owned.
- [x] Added exact current-lease process-start admission.
- [x] Added one-shot `.started` signal scanning.
- [x] Wired start-signal consumption into the early poll sequence.
- [x] Changed fresh process dispatch from immediate `Owned` to `Starting`.
- [x] Covered unused-seat and cross-provider fresh-process classification.
- [x] Preserved same-process Claude reuse as immediately owned.
- [x] Preserved recycled Codex `AssignedPendingAck` and exact ack promotion.
- [x] Added the visible `starting` seat-status label in yellow.
- [x] Added native fresh-dispatch start-gating coverage.
- [x] Added malformed, stale-generation, exact, and duplicate signal assertions.
- [x] Updated existing fresh and cross-provider scheduler expectations.

## Behavioral result

Immediately after a fresh scheduler dispatch:

- the slot is reserved for the ticket;
- the minted `AttemptLease` is installed on the slot and thread;
- the seat assignment is `Starting` with that lease's attempt generation;
- `seat_is_owned` is false;
- the dashboard active row shows `starting`.

After an exact `.lisa/signals/pane-<pane>.started` lease is consumed:

- the signal file is removed;
- the candidate is checked against the starting generation;
- the candidate must equal the pane slot's ticket and attempt lease;
- the candidate must remain current in `current_leases`;
- the seat transitions once to `Owned`;
- the dashboard active row shows `owned`.

Malformed, stale, and duplicate files are consumed without an invalid transition.

## Fresh-route classification

The implementation gates routes that launch a provider process:

- an unused physical pane;
- a cross-provider recycle that launches after the old TUI exits;
- an adapter using `ResetStrategy::FreshExec`.

It does not impose process-start gating on same-process clear-handshake reuse. Native
Claude reuse therefore remains owned, while native Codex reuse retains E-033's separate
prompt-acknowledgment gate.

## Source files

- `crates/lisa-plugin/src/lib.rs`
  - internal starting state;
  - admission and signal scanner;
  - dispatch and poll integration;
  - UI mapping;
  - native scheduler tests and updated route expectations.
- `crates/lisa-plugin/src/ui.rs`
  - dashboard starting status, label, and color.

## Verification completed

Formatting:

```text
cargo fmt --all -- --check
```

Result: passed.

Focused acceptance:

```text
cargo test -p lisa-plugin test_fresh_dispatch_becomes_owned_only_after_exact_process_start -- --nocapture
```

Result: 1 passed, 0 failed.

Plugin regression suite:

```text
cargo test -p lisa-plugin
```

Result: 277 passed, 0 failed.

Workspace suite:

```text
cargo test --workspace
```

Result: passed across Lisa CLI, core, plugin, and doc tests. The plugin repeated with
277 passing tests; the CLI and core suites also completed without failures.

Notable E-033/E-034 regressions observed green include:

- `test_recycled_codex_ownership_requires_matching_ack_exactly_once`;
- `test_dropped_post_prompt_ack_reproduces_open_loop_stall_and_recovers_boundedly`;
- `test_bounded_ack_wait_recovers_once_then_fails_actionably`;
- `test_recovery_ack_promotes_only_the_fresh_generation`;
- `test_reused_claude_assignment_remains_owned`;
- `test_consecutive_reused_panes_resolve_codex_ack_or_fallback_and_preserve_claude`;
- `split_brain_timeline_fences_old_attempt_and_admits_one_winner`;
- stale-attempt heartbeat/artifact publication coverage.

## Commit

Committed through Lisa's isolated ticket transaction:

```text
5cd47a9343270c5a529a84d990b98d4ae12d4e0c
feat(plugin): gate fresh ownership on process start
```

Exact included paths:

```text
crates/lisa-plugin/src/lib.rs
crates/lisa-plugin/src/ui.rs
```

The installed `/opt/homebrew/bin/lisa` predates `commit-ticket`. The current repository
CLI had already been built and the commit used `target/debug/lisa commit-ticket`, which
executes the same current `lisa` subcommand implementation. No ordinary `git add` or
ordinary `git commit` was used.

Post-commit verification:

- both ticket-owned source paths are clean;
- the ordinary Git index has no staged paths;
- no ticket-owned source file is modified or untracked;
- unrelated concurrent worktree changes were not included or altered.

## Plan deviations

- The plan anticipated that cross-provider recycling should use `Starting`; one existing
  test still expected E-033 `AssignedPendingAck` and failed on the first full plugin run.
  Its expectation was updated because the route launches a genuinely fresh Codex process.
- No production behavior deviated from the selected design.
- No startup timeout was added; that remains deliberately assigned to T-035-01-04.

## Remaining ticket work

- [x] Implement source changes.
- [x] Commit exact ticket-owned source paths.
- [x] Run focused and full verification.
- [x] Write private progress artifact.
- [ ] Write private review artifact.
