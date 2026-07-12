# T-035-01-03 Structure — gate Owned on observed start

## File inventory

### Modify `crates/lisa-plugin/src/lib.rs`

Responsibilities added or changed:

- represent the fresh process-start pending state;
- classify fresh process launches during dispatch;
- replace immediate fresh `Owned` publication with `Starting`;
- validate exact attempt-bound process-start signals;
- scan and consume `.started` signal files;
- invoke the scanner from the poll loop;
- map the new state to dashboard UI state;
- add native scheduler and fencing tests.

No new Rust module is warranted. The state and consumer are small peers of existing
assignment and signal methods and depend on private `State` fields.

### Modify `crates/lisa-plugin/src/ui.rs`

Responsibilities added:

- represent a dashboard-facing starting assignment;
- render its stable `starting` label;
- render it with pending/yellow color semantics.

No layout or public plugin API changes are needed.

### No change `crates/lisa-cli/src/templates.rs`

T-035-01-01 already owns and tests the producer. This ticket consumes its output.

## Internal state change

Extend `SeatAssignmentState`:

```rust
Starting {
    generation: u64,
}
```

Placement should be before the reused-Codex pending state so the lifecycle reads from
fresh start, through provider-specific reuse acceptance, to owned/recovery states.

The variant carries no timeout. T-035-01-04 owns bounded recovery.

## Admission method

Add near `seat_assignment` and the existing acknowledgment helpers:

```rust
fn acknowledge_process_start(
    &mut self,
    pane_id: u32,
    candidate: &AttemptLease,
) -> bool
```

Method boundary:

- inspect the exact starting generation;
- resolve the pane's slot ticket and lease;
- compare candidate with state, slot, and current authority;
- write `Owned` only after all checks pass;
- return whether this call performed the edge.

The method must not accept `AssignedPendingAck`, `Recovering`, `RecoveryFailed`, or
already `Owned`. Duplicate signals therefore cannot repeat the transition.

## Signal scanner

Add beside `check_heartbeat_signals`:

```rust
fn check_process_start_signals(&mut self)
```

Scanner organization:

1. read `signal_dir`, returning quietly if absent;
2. select filenames shaped as `pane-<u32>.started`;
3. read and deserialize `AttemptLease`;
4. remove the file regardless of validity;
5. skip malformed candidates;
6. pass valid candidates to `acknowledge_process_start`.

This deliberately mirrors the heartbeat scanner's one-shot lease-bearing transport.

## Dispatch integration

Introduce a local `fresh_launch` fact in `schedule_ready_tickets`.

The value is true for branches that submit `adapter.launch_command(&ctx)` as a fresh
process launch, whether immediately or after exit. It is false for in-process prompt
reuse after a clear handshake.

Assignment-state precedence after delivery setup:

1. if `fresh_launch`, publish `Starting` with the minted attempt generation;
2. else if reused Codex has `assignment_generation`, publish `AssignedPendingAck`;
3. else publish `Owned` for established same-process reuse.

This precedence matters for cross-provider recycling into Codex: it is a fresh process,
so startup observation, not recycled-prompt acknowledgment, is the relevant contract.

The existing acknowledgment clock is armed only when `assignment_generation` exists
and the resulting state is `AssignedPendingAck`/`Recovering`; `Starting` is unaffected.

## Poll integration

At the top of `poll_tick`:

```text
check_heartbeat_signals
check_process_start_signals
check_awaiting_signals
check_codex_ack_signals
...
```

Exact placement beside heartbeat consumption keeps positive process activity current
before later scheduler decisions. It also establishes start-before-timeout ordering for
the following recovery ticket.

## UI conversion

Extend `ui::SeatAssignmentStatus` with `Starting`.

Extend its methods:

- `label(Starting) -> "starting"`;
- `color(Starting) -> YELLOW`.

Extend `State::to_ui_state` mapping:

- `SeatAssignmentState::Starting { .. } -> ui::SeatAssignmentStatus::Starting`.

No caller needs to know the generation at the UI boundary.

## Native test structure

Add a test near the existing dashboard/recycled ownership tests. Reuse the scheduler
fixture when it can represent an empty slot; otherwise construct a focused state using
the existing ticket fixture helpers.

Assertions before start:

- slot ticket and attempt lease are installed;
- state is `Starting { generation }`;
- state is not `Owned`;
- dashboard row shows `starting`.

Assertions after start:

- exact current lease is written to `.started`;
- scanner removes the file;
- state becomes `Owned`;
- dashboard row shows `owned`.

Add rejection assertions for a stale generation or malformed payload if they fit the
same focused test without obscuring the primary acceptance sequence.

## Regression boundaries

- Existing recycled Codex tests must retain `AssignedPendingAck` and ack promotion.
- Existing reused Claude test must retain immediate `Owned` after in-process reuse.
- Existing attempt lease and split-brain tests must remain green.
- Fresh recovery tests may need expectations updated from ack-pending to starting only
  where they truly launch a new process; bounded behavior itself remains ticket-owned
  by the predecessor E-033 implementation until T-035-01-04 extends startup recovery.

## Change ordering

1. Add internal/UI variants and exhaustive mappings.
2. Add admission method and scanner.
3. Change dispatch classification and state publication.
4. Wire poll consumption.
5. Add acceptance and fencing tests.
6. Format, run focused tests, then workspace verification.
7. Commit the exact two source paths in Lisa's isolated transaction.
