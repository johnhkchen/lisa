# T-033-01-04 Progress — bounded acknowledgment recovery

## Status

Implementation and verification are complete. The ticket-owned production
change is ready for Lisa's isolated source transaction.

| Unit | Status | Evidence |
|---|---|---|
| finite config contract | complete | core + CLI focused and package tests |
| delivery-timed deadline | complete | clear/exit transition assertions |
| pending to recovering | complete | deterministic withheld-ack test |
| fenced fresh fallback | complete | recovery generation 2; generation 1 rejected |
| at-most-one launch | complete | repeated transition polls retain one launch |
| recovery success | complete | exact recovery ack promotes to owned |
| recovery failure | complete | terminal state, failed retained thread, reset instruction |
| Claude regression | complete | full plugin suite |
| workspace/WASM/lint | complete | all required gates pass |
| isolated source commit | complete | `f907a762dcf0b1dddfd02bc2a7fe6e79c54e8514` |

## Configuration implementation

Added `assignment_ack_timeout_secs` as a positive scheduling setting with a
30-second default.

### Core plugin configuration

`crates/lisa-core/src/types.rs` now provides:

- `PluginConfig::DEFAULT_ASSIGNMENT_ACK_TIMEOUT_SECS = 30`;
- `PluginConfig::assignment_ack_timeout_secs`;
- positive KDL-map parsing;
- fail-safe fallback to 30 for missing, zero, or malformed direct map values.

The plugin therefore cannot be configured into an infinite wait through raw
layout input.

### CLI TOML and resolution

`crates/lisa-cli/src/config.rs` now carries the setting through:

- `SchedulingConfig: Option<u64>`;
- `ResolvedConfig: u64`;
- default and file-value resolution;
- known-key validation;
- a semantic error for zero;
- the commented default `.lisa.toml` template.

Zero is rejected because disabling this deadline would recreate the silent
pending state the ticket removes.

### Layout and upgrade transport

`crates/lisa-cli/src/loop_cmd.rs` emits the resolved value into the generated
Zellij plugin configuration.

`crates/lisa-cli/src/init.rs` adds the commented setting to existing
`.lisa.toml` files through the ownership-aware textual merge. Active and
commented user values remain non-duplicated under the existing key detector.

`crates/lisa-cli/src/setup_guide.rs` documents the default, positive contract,
one fresh fallback, and terminal error.

## Assignment state implementation

`SeatAssignmentState` now represents both identity and wait state:

```rust
AssignedPendingAck { generation, ack_deadline }
Owned
Recovering { generation, ack_deadline }
RecoveryFailed
```

`ack_deadline: None` means transport has not yet submitted the tagged prompt.
`Some` means the finite provider-acceptance wait is active.

`active_assignment_generation` returns the detector identity for pending and
recovering states. The acknowledgment detector itself was not changed. It
still requires an exact ticket/generation marker and event type.

An exact payload now supports either:

- original pending -> owned;
- fresh recovery -> owned.

Once owned, the generation is gone and duplicate payloads remain inert.

## Deadline start boundary

`start_assignment_ack_wait` is called only after a tagged prompt or launch line
is written. It is not called when scheduling merely sends `/clear` or `/exit`.

The computed absolute deadline includes `ENTER_DELAY_SECS` before the configured
wait. `send_line_to_pane` types characters immediately but submits Enter later;
including that transport delay means even a one-second configuration cannot
expire while the prompt is still sitting unsubmitted in the composer.

Delivery sites covered:

- post-`.cleared` reuse prompt;
- clear-signal timeout prompt fallback;
- normal cross-provider exit-grace launch;
- recovery exit-grace fresh launch;
- immediate tagged scheduling if a future adapter uses `FreshExec`.

Existing tests were updated to distinguish unarmed handoff state from armed
post-delivery state.

## Initial timeout and recovery

`check_assignment_ack_timeouts_at(now)` evaluates absolute deadlines and gives
tests deterministic control without sleeping. `poll_tick` invokes its real-time
wrapper after acknowledgment consumption, error consumption, and transition
delivery.

When an original deadline expires:

1. the scheduler confirms the pane still has a ticket reservation;
2. it allocates a distinct recovery generation;
3. it changes the seat to recovering before pane input;
4. it clears abandoned-TUI question/attention flags;
5. it submits Codex `/exit`;
6. it enters the existing `WaitingForExit` transport with an eight-second grace;
7. it retains the same ticket and remains not-owned.

Changing generation before `/exit` fences a late payload from the abandoned
prompt.

After exit grace, the existing transition machinery launches a fresh Codex TUI
for the same ticket with the recovery generation marker. It clears the
transition to `Idle`, arms the recovery deadline, records one actual recovery
`SessionLaunch`, and retains `Recovering` rather than declaring ownership.

Because the transport leaves `WaitingForExit` after that action, repeated polls
cannot issue another fresh launch.

## Terminal recovery behavior

A matching recovery-generation payload promotes the seat to `Owned` exactly
once.

If the recovery deadline expires or the fresh Codex process emits `.error`,
`fail_assignment_recovery`:

- changes the assignment to `RecoveryFailed`;
- marks the thread failed;
- adds one error alert;
- retains the slot/ticket/thread association;
- logs an actionable instruction to reset the ticket.

Retention is intentional. Releasing and removing the thread would let the same
ready ticket enter another automatic attempt and could retry forever. The
existing manual reset action is the explicit retry authority.

Generic non-recovery `.error` behavior is unchanged.

## Acceptance test

`test_bounded_ack_wait_recovers_once_then_fails_actionably` exercises the full
withheld-ack contract:

- schedules a resident Codex seat;
- proves no deadline runs during `/clear`;
- delivers the generation-1 prompt and observes an armed pending state;
- expires the first deadline;
- observes `Recovering` generation 2, same ticket, and not-owned;
- rejects a late generation-1 acknowledgment;
- advances exit grace and observes one fresh generation-2 launch;
- repeats transition polling and still observes one launch;
- expires the recovery deadline;
- observes `RecoveryFailed`, failed retained thread, retained reservation,
  error alert, reset instruction, and no ownership;
- repeats timeout evaluation and observes no new launch.

`test_recovery_ack_promotes_only_the_fresh_generation` covers the success fork:
after the same fenced fallback, exact generation 2 promotes to `Owned`, and the
old deadline cannot later fail it.

Existing stale-ticket, stale-generation, duplicate-ack, signal scanner, Claude
reuse, clear-timeout, and cross-provider exit tests remain green.

## Test results

Focused configuration tests passed:

```text
cargo test -p lisa-core assignment_ack_timeout
  3 passed

cargo test -p lisa-cli assignment_ack_timeout
  1 passed
```

Focused scheduler tests passed:

```text
cargo test -p lisa-plugin bounded_ack_wait
  1 passed

cargo test -p lisa-plugin recovery_ack
  1 passed

cargo test -p lisa-plugin recycled_codex_ownership
  1 passed

cargo test -p lisa-plugin transition_timeouts
  5 passed
```

Package suites passed:

```text
cargo test -p lisa-core
  150 passed

cargo test -p lisa-plugin
  264 passed

cargo test -p lisa-cli
  270 unit tests passed
  1 integration test passed
```

Workspace and WASM verification passed:

```text
cargo test --workspace
just check
```

Formatting and strict production-target Clippy passed:

```text
cargo fmt --all -- --check
cargo clippy -p lisa-core --lib -- -D warnings
cargo clippy -p lisa-plugin --lib -- -D warnings
cargo clippy -p lisa-cli --bin lisa -- -D warnings
```

`git diff --check` passed for all six ticket-owned production paths.

## Deviations from plan

The implementation added the configured wait after the existing deferred Enter
delay rather than starting it at the character-write call. This is a precision
improvement discovered during review of `send_line_to_pane`: Enter is the actual
prompt submission boundary.

No adapter change was required. The existing `SpawnContext` generation field
already tags full fresh launch commands, as anticipated by Design.

No separate `.error` scheduler test was added because the required fallback
failure and no-retry behavior are fully exercised by the bounded recovery
deadline test, while the existing `.error` suite continues to cover generic
process failures. The recovery `.error` branch uses the same terminal helper.

## Ownership audit

Ticket-owned production paths:

- `crates/lisa-core/src/types.rs`;
- `crates/lisa-cli/src/config.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-cli/src/init.rs`;
- `crates/lisa-cli/src/setup_guide.rs`;
- `crates/lisa-plugin/src/lib.rs`.

The worktree contains pre-existing unrelated modified and untracked files.
They were not edited intentionally and will not be included in the isolated
transaction. The ticket file and this work directory remain Lisa-owned final
completion inputs.

## Source transaction

The globally installed `/opt/homebrew/bin/lisa` did not recognize
`commit-ticket`. The repository CLI was built and used instead:

```text
target/debug/lisa commit-ticket \
  --ticket-id T-033-01-04 \
  --message "feat: bound Codex assignment recovery" \
  --include crates/lisa-core/src/types.rs \
  --include crates/lisa-cli/src/config.rs \
  --include crates/lisa-cli/src/loop_cmd.rs \
  --include crates/lisa-cli/src/init.rs \
  --include crates/lisa-cli/src/setup_guide.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Result:

```text
f907a762dcf0b1dddfd02bc2a7fe6e79c54e8514
feat: bound Codex assignment recovery
6 files changed, 583 insertions(+), 34 deletions(-)
```

Post-commit audit:

- the commit contains exactly the six ticket-owned production paths;
- all six paths are clean;
- the ordinary Git index is empty;
- the ticket file and work artifacts remain untracked for Lisa's completion
  transaction;
- unrelated modified and untracked paths remain present and excluded.
