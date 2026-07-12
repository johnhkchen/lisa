# T-035-04-02 Structure — same-pane startup reset

## Source ownership

The implementation is contained in one meaningful source unit:

- modify `crates/lisa-plugin/src/lib.rs`;
- do not modify `adapter.rs`, because bare provider launch construction already exists;
- do not modify `ui.rs`, because terminal `StartupFailed` is already visible and the
  transient reset can map to the existing yellow `Starting` presentation if desired;
- do not modify CLI templates or generated hooks;
- do not modify ticket frontmatter or shared work artifacts.

The inline plugin test module remains the correct home for state-machine regressions.

## Constants

Add beside assignment delivery constants:

```rust
const MAX_SAME_PANE_STARTUP_RELAUNCHES: u8 = 1;
```

The existing assignment acknowledgment timeout supplies both the process-start and shell
reset finite deadlines. No new configuration field is needed; the boundary is operational
rather than a separate user-tuning concern.

Add a signal suffix constant only if it reduces repeated literals:

```rust
const SHELL_READY_SUFFIX: &str = ".shell-ready";
```

## Assignment state shape

Change the initial state to:

```rust
Starting {
    generation: u64,
    start_deadline: Option<SystemTime>,
    relaunches: u8,
}
```

Add:

```rust
ResettingStartup {
    generation: u64,
    reset_deadline: SystemTime,
}
```

Keep:

- `ReadyForAssignment` unchanged;
- `Delivering` unchanged;
- inherited E-033 pending/recovering states unchanged;
- `Owned` unchanged;
- `StartupFailed` as the named terminal startup/recovery failure.

All fresh dispatch constructors use `relaunches: 0`. The sole successful reset relaunch
uses `relaunches: 1`.

## Generation lookup

Extend internal generation lookup only where needed.

`active_assignment_generation` must remain restricted to states that can accept a chat
acknowledgment. Do not include `Starting` or `ResettingStartup`.

Add a dedicated helper if reset code needs the current generation:

```rust
fn startup_generation(&self, pane_id: u32) -> Option<u64>
```

It may match `Starting` and `ResettingStartup`, but must not be reused by chat admission.

## Raw pane interrupt helper

Add a method near `send_line_to_pane`:

```rust
fn interrupt_shell_input(&mut self, pane_id: u32)
```

Responsibilities:

1. remove `PendingEnter` entries for `PaneId::Terminal(pane_id)`;
2. write byte `3` through `write_to_pane_id`;
3. update the pane activity clock only through the caller's coherent state transition.

The helper must not send `/exit`, Enter, or a provider command.

## Shell probe builder

Add a pure helper near launch preparation:

```rust
fn shell_readiness_probe(
    signal_dir: &Path,
    pane_id: u32,
    lease: &AttemptLease,
) -> Result<String, String>
```

It serializes the lease, creates unique temporary and final paths, converts `/host/...`
signal paths for host pane execution, and returns one bounded atomic shell command.

Use `shell_quote` for JSON and both paths. The helper does not write a signal itself and
does not mutate scheduler state. Unit tests inspect its shape and execute it under `sh`
or `zsh` where available to prove exact bytes and atomic destination behavior.

## Pane signal cleanup

Add a narrow helper:

```rust
fn clear_pane_lifecycle_signals(&self, pane_id: u32)
```

Remove known attempt-derived files for that pane before the reset probe:

- `.started`;
- `.ack`;
- `.heartbeat`;
- `.idle`;
- `.stopped`;
- `.cleared`;
- `.error`;
- `.awaiting`;
- `.shell-ready`;
- any stale pane lease marker.

Removal is best-effort. Correctness still depends on exact lease admission, not cleanup.
Removing the old `.lease` is important so a predecessor hook cannot copy it after
revocation, while delaying the successor marker prevents successor impersonation.

## Begin-reset method

Add near existing assignment recovery:

```rust
fn begin_startup_recovery(&mut self, pane_id: u32, now: SystemTime)
```

Input state:

```text
Starting { relaunches: 0, expired deadline }
```

Validation:

- slot exists and retains a ticket;
- slot lease generation equals state generation;
- slot lease is exact current authority;
- high water equals the predecessor.

Mutation order:

1. retain predecessor value;
2. revoke current authority;
3. mint strict successor;
4. install successor in high water/current;
5. stamp slot and thread with successor;
6. enter `ResettingStartup` with absolute deadline;
7. clear attention and lifecycle residue;
8. interrupt shell input;
9. send the shell probe with deferred Enter;
10. record activity and a warning naming same-pane recovery.

Any validation/mint/probe-construction error reaches terminal recovery failure without a
relaunch.

## Shell-ready admission

Add:

```rust
fn acknowledge_shell_ready(
    &mut self,
    pane_id: u32,
    candidate: &AttemptLease,
    now: SystemTime,
) -> bool
```

It requires:

- `ResettingStartup` with matching generation;
- exact slot ticket and attempt lease;
- exact current authority;
- the same physical pane reservation.

Before sending pane input it reconstructs all successor runtime artifacts:

1. resolve host ticket directory;
2. resolve adapter and route;
3. resolve exact private successor work directory;
4. create successor assignment text and atomically publish `assignment.md`;
5. create the bare launch payload and atomically publish its script;
6. publish the successor pane lease marker;
7. change state to replacement `Starting` before or coherently with input;
8. send only the `sh <script>` command;
9. arm the replacement start deadline;
10. refresh slot session/client/activity fields and log relaunch.

Return `true` only after launch submission is scheduled. Errors call the terminal helper
and return `false`.

## Signal scanner

Add:

```rust
fn check_shell_ready_signals(&mut self)
```

Mirror `check_process_start_signals`:

- enumerate `signal_dir`;
- select `pane-<id>.shell-ready`;
- parse `AttemptLease`;
- remove the file before admission;
- call `acknowledge_shell_ready(..., SystemTime::now())`.

Place it in `poll_tick` after process-start scanning and before chat acknowledgment and
deadline evaluation. This preserves evidence-before-timeout behavior.

## Deadline integration

Extend `check_assignment_ack_timeouts_at` collection to include:

```rust
ResettingStartup { reset_deadline, .. }
```

Change the expired `Starting` branch:

- `relaunches == 0` calls `begin_startup_recovery`;
- `relaunches == 1` calls terminal startup recovery failure and fences;
- values greater than one are treated as exhausted and fence defensively.

Expired `ResettingStartup` calls terminal startup recovery failure with a message naming
missing positive shell readiness.

Keep existing Delivering, AssignedPendingAck, and Recovering branches unchanged.

## Failure and fencing split

Refactor fencing mechanics into a pane-scoped helper if necessary:

```rust
fn fence_assigned_pane(&mut self, ticket_id: &TicketId, preserve_assignment: bool)
```

The existing E-034 `revoke_and_fence_attempt` continues to remove assignment state and
returns its current `FenceOutcome` contract.

The startup recovery failure path should:

- set `StartupFailed`;
- fail thread and add alert;
- revoke successor;
- mark slot `Fenced`, clear pending input and attention, close the pane;
- retain ticket/attempt stamps only as diagnostic reservation state until operator reset;
- avoid release or automatic reschedule.

If retaining stamps conflicts with reset handling, clear the slot lease after revocation
but keep ticket ID and `StartupFailed`; the authoritative registry remains empty either
way. The tests should define the selected invariant precisely.

## Dashboard mapping

Map `ResettingStartup` to the existing yellow `SeatAssignmentStatus::Starting`. This
avoids a `ui.rs` source change while remaining truthful: the provider is not started and
the seat is in a startup lifecycle. The activity log supplies the more specific recovery
description.

Map terminal failure through existing red `StartupFailed`.

## Existing constructor updates

Update every `Starting` construction and exact comparison in `lib.rs`:

- fresh schedule path: zero relaunches;
- E-033 post-exit fresh fallback: preserve its intended count as zero because it is a
  different recovery axis, or explicitly use one if startup reset must be disallowed;
- tests installing manual states;
- `start_assignment_ack_wait` shape preservation;
- UI projection matches;
- helper matches and assertions.

For the inherited E-033 fallback, use `relaunches: 0`: it has not yet exercised this
ticket's incomplete-shell recovery and may safely receive one shell reset if its bare
launcher is malformed. Its own provider fallback count remains independently bounded.

## Test additions

Add focused unit tests in the existing plugin test module:

```text
shell_readiness_probe_publishes_exact_attempt_atomically
starting_timeout_rotates_lease_before_same_pane_shell_probe
stale_predecessor_signals_cannot_advance_startup_replacement
same_pane_replacement_requires_start_and_chat_ack_before_owned
missing_shell_readiness_fences_without_relaunch
missing_replacement_start_fences_without_second_relaunch
started_and_owned_states_never_enter_shell_recovery
claude_and_codex_share_same_pane_startup_recovery_contract
```

Strengthen existing `test_missing_fresh_start_signal...` expectations to the new bounded
single-recovery contract rather than the prior zero-relaunch contract.

## Commit unit

After focused and full verification, commit exactly:

```text
crates/lisa-plugin/src/lib.rs
```

using:

```text
lisa commit-ticket --ticket-id T-035-04-02 \
  --message "fix(plugin): recover incomplete shell startup in place" \
  --include crates/lisa-plugin/src/lib.rs
```

Phase artifacts remain in the private attempt directory for Lisa publication and final
completion handling.
