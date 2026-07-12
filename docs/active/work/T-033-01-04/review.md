# T-033-01-04 Review — bounded acknowledgment recovery

## Outcome

Implemented a finite, configurable acknowledgment contract for recycled Codex
assignments.

Once Lisa actually submits a generation-tagged prompt, the seat has a positive
acceptance deadline. With acknowledgment withheld, the original generation is
fenced and the seat changes from `AssignedPendingAck` to `Recovering`. Lisa
exits the unacknowledged TUI and launches exactly one fresh Codex session for
the same ticket with a new generation.

The fresh fallback remains not-owned until its exact acknowledgment arrives. A
matching recovery acknowledgment establishes `Owned`; a missing acknowledgment
or fresh-process error ends in `RecoveryFailed`, retains the ticket reservation,
and tells the operator to reset the ticket. No automatic recovery loop is
possible.

Claude scheduling and ordinary fresh Codex ownership are unchanged.

## Source commit

```text
f907a762dcf0b1dddfd02bc2a7fe6e79c54e8514
feat: bound Codex assignment recovery
```

Commit scope:

```text
6 files changed, 583 insertions(+), 34 deletions(-)
```

The source commit was created through Lisa's isolated transaction with six
exact include paths. The installed Lisa binary lacked `commit-ticket`, so the
repository-built `target/debug/lisa` was used, matching the established project
fallback.

The ticket and RDSPI work artifacts were not included. Lisa owns their final
completion transaction.

## Files modified

### `crates/lisa-core/src/types.rs`

Added `PluginConfig::assignment_ack_timeout_secs` and the 30-second default
constant.

Raw KDL-map parsing accepts only a positive `u64`. Missing, malformed, and zero
values retain the finite default. This prevents direct plugin configuration
from restoring an infinite wait even if CLI validation is bypassed.

Added three tests covering default, positive override, and fail-safe values.

### `crates/lisa-cli/src/config.rs`

Added the optional `[scheduling].assignment_ack_timeout_secs` TOML field and its
resolved `u64` representation.

Registered the setting as a known scheduling key, applied the core default,
and rejected zero with:

```text
assignment_ack_timeout_secs must be at least 1
```

Added the commented 30-second example to generated `.lisa.toml` and a focused
contract test for parse, resolution, default, validation, zero rejection, and
template presence.

### `crates/lisa-cli/src/loop_cmd.rs`

Added `assignment_ack_timeout_secs` to the generated Zellij KDL plugin block.
The loop now transports the resolved TOML value into WASM scheduler
configuration. The layout test verifies the 30-second default.

### `crates/lisa-cli/src/init.rs`

Added the setting to the ownership-aware scheduling-key upsert. Existing
projects discover the setting after `lisa init` without overwriting or
duplicating active/commented values.

Extended init merge tests to require the new key.

### `crates/lisa-cli/src/setup_guide.rs`

Documented the positive timeout, 30-second default, single fresh-session
fallback, and terminal error if that fallback is not acknowledged.

### `crates/lisa-plugin/src/lib.rs`

Extended seat truth to:

```rust
AssignedPendingAck { generation, ack_deadline }
Owned
Recovering { generation, ack_deadline }
RecoveryFailed
```

An absent deadline means the tagged prompt has not yet been submitted. A
present deadline means provider acceptance is being bounded. Only `Owned`
reports ownership.

Generalized assignment identity lookup and exact acknowledgment promotion to
support the active pending and recovery generations. The detector remains
unchanged and still requires exact ticket/generation evidence.

Added delivery-aware deadline arming. `/clear` and `/exit` transport do not
start the clock. Post-clear prompt delivery, clear-timeout delivery, exit-grace
launch, and immediate tagged delivery do. The absolute deadline includes the
existing deferred Enter delay before the configured wait, so a one-second
setting cannot expire before the prompt is submitted.

Added deterministic timeout evaluation with an injected `SystemTime` for unit
tests and a real-time wrapper in `poll_tick`. Acknowledgment consumption occurs
first, so a matching boundary payload wins before timeout recovery.

On the original deadline:

- a new recovery generation is allocated;
- seat state changes before pane input, fencing the old generation;
- abandoned-TUI flags are cleared;
- `/exit` is submitted;
- the existing one-shot exit grace is reused;
- the same ticket reservation stays attached and not-owned.

After exit grace, Lisa launches one new Codex command carrying the recovery
generation, clears transport state, arms the fallback deadline, and records the
actual launch. Repeated polls have no transition that can launch it again.

On matching fallback acknowledgment, recovery becomes owned exactly once. On
fallback timeout or `.error`, the shared failure helper enters
`RecoveryFailed`, marks the retained thread failed, adds an alert, and logs the
manual reset instruction.

The failure path deliberately does not release the seat or remove the thread.
That prevents the same ready ticket from cycling through unbounded automatic
fallbacks and prevents another assignment from claiming the physical seat.

## Acceptance criterion evaluation

### Finite configurable deadline

Met.

`assignment_ack_timeout_secs` is configurable through `.lisa.toml`, defaults to
30, is emitted through KDL, and cannot be zero. Scheduler deadlines are absolute
and evaluated by the five-second poll loop.

The wait begins only after prompt submission transport, not when the scheduler
first reserves a seat.

### Pending to recovering with acknowledgment withheld

Met.

`test_bounded_ack_wait_recovers_once_then_fails_actionably` schedules a resident
Codex seat, delivers the generation-1 prompt, withholds acknowledgment, advances
to its deadline, and observes:

```text
AssignedPendingAck { generation: 1, ... }
-> Recovering { generation: 2, ack_deadline: None }
```

The same pane retains the same ticket and reports not-owned.

### At most one fresh Codex launch for the same ticket

Met.

The test advances the existing exit grace, observes one generation-2
`SessionLaunch` for the same ticket, calls transition evaluation again, and
still counts exactly one recovery launch. After terminal failure and another
timeout evaluation, the count remains one.

### Original assignment abandoned cleanly

Met.

Recovery state and its new generation are installed before `/exit`. The test
injects a late generation-1 payload after recovery starts and confirms it cannot
claim the seat. No point in the fallback path reports owned before exact
generation-2 acknowledgment.

### Actionable fallback failure

Met.

With fallback acknowledgment also withheld, the test advances the second
deadline and observes:

- `RecoveryFailed`;
- not-owned;
- retained slot/ticket association;
- retained failed thread;
- one error alert;
- an error message containing `reset the ticket`;
- no further fresh launch.

Fresh Codex `.error` signals enter the same helper rather than the generic
automatic retry path.

### No double ownership, lost ticket, infinite retry, or silent stall

Met by state construction and tests.

- Only exact current-generation acknowledgment creates `Owned`.
- The original generation disappears at the recovery edge.
- The ticket remains bound through recovery and failure.
- `WaitingForExit` is cleared after one launch.
- `RecoveryFailed` has no automatic transition.
- failed state is surfaced through the existing error alert and activity log.

## Test coverage

### Focused configuration coverage

Passed:

```text
cargo test -p lisa-core assignment_ack_timeout
  3 passed

cargo test -p lisa-cli assignment_ack_timeout
  1 passed
```

Coverage includes finite default, positive override, direct-map zero/malformed
fallback, CLI zero rejection, known-key behavior, resolution, and template
presence.

### Focused scheduler coverage

Passed:

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

Coverage includes unarmed transport, armed delivery, first timeout, generation
fencing, same-ticket reservation, one fresh launch, repeated-poll idempotence,
exact recovery acknowledgment, terminal fallback timeout, failed retained
thread, alert/log output, and no retry.

Existing tests additionally cover stale ticket/generation rejection, duplicate
ack idempotence, raw ack signal consumption, clear timeout, cross-provider exit
grace, and Claude immediate ownership.

### Package coverage

Passed:

```text
cargo test -p lisa-core
  150 passed, 0 failed

cargo test -p lisa-plugin
  264 passed, 0 failed

cargo test -p lisa-cli
  270 unit tests passed, 0 failed
  atomic_provider_contract: 1 passed, 0 failed
```

### Workspace and WASM coverage

Passed:

```text
cargo test --workspace
just check
```

`just check` completed the `wasm32-wasip1` plugin check and workspace tests.

### Formatting and lint coverage

Passed:

```text
cargo fmt --all -- --check
cargo clippy -p lisa-core --lib -- -D warnings
cargo clippy -p lisa-plugin --lib -- -D warnings
cargo clippy -p lisa-cli --bin lisa -- -D warnings
```

## Diff and transaction audit

- `git diff --check` passed for all ticket-owned production paths.
- Commit `f907a762` contains exactly the six reviewed source paths.
- Every ticket-owned production path is clean after commit.
- The ordinary Git index is empty.
- The ticket file and work artifacts remain untracked for Lisa's final
  completion transaction.
- Pre-existing unrelated modified and untracked worktree content was preserved
  and excluded.
- Ticket phase and status frontmatter were not edited.

## Open concerns and limitations

### No live Codex proof in this ticket

The scheduler contract is deterministic and token-free. A real consecutive
reuse/fallback proof belongs to `S-033-03`, as the story's honest boundary
states. This ticket does not claim live provider validation.

### Existing projects need configuration regeneration

Projects must run the updated `lisa init` and start a new loop for the new KDL
setting to reach the WASM plugin. Older layouts omit the key and safely receive
the 30-second plugin default.

### Recovery state is private scheduler truth

`RecoveryFailed` is surfaced through the existing failed thread, error alert,
and activity message. The dashboard does not yet render the assignment-state
name itself; assignment-state UI belongs to `S-033-02`.

### Manual reset is required after terminal failure

This is intentional to prevent infinite retry. An operator must diagnose the
hook/process problem and reset the ticket. The error message names that action.

### In-memory timing and generation

Deadlines and generations remain process-local, matching existing seat state.
A plugin restart reconstructs scheduler state rather than persisting an in-flight
recovery attempt. Durable attempt leases are outside this ticket.

### Old-TUI exit failure

If the abandoned Codex process reports `.error` during recovery, the scheduler
terminates recovery immediately rather than assuming the pane returned safely
to a shell. This is conservative: it avoids launching a second client into an
uncertain pane and asks for operator reset.

### Extremely large direct timeout

CLI validation accepts any positive `u64`. If adding it to `SystemTime` would
overflow, the scheduler falls back to the finite 30-second default plus deferred
Enter delay rather than panicking.

## Critical issues

None identified.

The acceptance criterion is fully covered, all production targets pass strict
linting, the WASM and workspace gates pass, and the isolated source transaction
is clean.

## Handoff summary

Recycled Codex ownership can no longer remain silently pending. Each submitted
tagged delivery has a finite clock. The first miss fences the old attempt and
permits one fresh generation; the fresh attempt either acknowledges into one
owner or terminates in an explicit reset-required failure. The ticket and seat
remain attributable throughout, and no automatic path can launch a second
fallback.
