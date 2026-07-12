# T-035-01-04 Structure — bounded startup recovery

## Change set overview

Modify two production files:

1. `crates/lisa-plugin/src/lib.rs`
2. `crates/lisa-plugin/src/ui.rs`

Create no Rust modules and delete no files. The behavior is a small extension of the
existing private assignment state machine, timeout evaluator, and dashboard mapping.

Attempt-private phase artifacts remain under:

```text
.lisa/attempts/T-035-01-04/1/work/
```

They are not source commit inputs and are not written directly to the shared work path.

## `crates/lisa-plugin/src/lib.rs`

### Assignment enum

Change:

```rust
Starting {
    generation: u64,
}
```

to:

```rust
Starting {
    generation: u64,
    start_deadline: Option<std::time::SystemTime>,
}
```

Document the field meanings:

- `None`: the attempt is reserved but the fresh launcher has not been submitted;
- `Some`: launcher submission occurred and positive process-start observation is bound.

Add a fieldless terminal variant after `RecoveryFailed` or adjacent to `Starting`:

```rust
StartupFailed,
```

Place it after `RecoveryFailed` if lifecycle grouping favors all terminal failures at
the bottom, or immediately after `Starting` if fresh lifecycle locality is clearer.
All exhaustive mappings must distinguish it from E-033 `RecoveryFailed`.

### Process-start admission

Update `acknowledge_process_start` destructuring to ignore the deadline while preserving
all existing exact identity checks:

```rust
let Some(SeatAssignmentState::Starting { generation, .. }) = ...
```

No other admission logic changes. `StartupFailed` must not expose a generation and
therefore cannot later accept a delayed start signal.

### Deadline arming helper

Keep `start_assignment_ack_wait` as the central deadline constructor because it already:

- adds `ENTER_DELAY_SECS`;
- uses the positive configured timeout;
- handles checked-add overflow with a finite default;
- accepts injected current time;
- arms only an unarmed state.

Extend its state transform with:

```rust
SeatAssignmentState::Starting {
    generation,
    start_deadline: None,
} => SeatAssignmentState::Starting {
    generation,
    start_deadline: Some(deadline),
},
```

Update its documentation from tagged-prompt-only language to positive provider
acceptance after actual input submission. The same helper remains private.

No rename is required; changing a broad internal call graph would add churn without
improving the ticket boundary. If implementation readability strongly favors it,
`start_assignment_acceptance_wait` is an acceptable mechanical rename within this file,
but it is not necessary.

### Startup failure helper

Add near `fail_assignment_recovery`:

```rust
fn fail_startup(&mut self, pane_id: u32, reason: &str)
```

Responsibilities:

1. return unless current state matches `Starting`;
2. insert `StartupFailed` before resolving auxiliary data;
3. find the retained slot ticket for the pane;
4. if absent, log an actionable pane-scoped startup error and return;
5. mark the retained thread failed when present;
6. add an error alert only if the same ticket/pane alert is absent;
7. log ticket, pane, reason, and reset instruction.

The helper must not:

- call `revoke_current_lease`;
- clear `ticket_id` or `attempt_lease` from the slot;
- release the slot;
- remove the thread;
- call an adapter;
- send input to the pane;
- create a transition timer.

The retained state is the structural guarantee against unbounded scheduling retries.

### Timeout collection

Extend `check_assignment_ack_timeouts_at`'s deadline match:

```rust
SeatAssignmentState::Starting {
    start_deadline: Some(deadline),
    ..
} => *deadline,
```

Keep unarmed starting states excluded.

Extend the action match:

```rust
SeatAssignmentState::Starting { .. } => {
    self.fail_startup(
        pane_id,
        "provider process start was not observed before the deadline",
    );
}
```

Retain the existing current-state equality guard before dispatching actions. It prevents
an action collected for an old value from applying after another transition.

`Owned`, `RecoveryFailed`, and `StartupFailed` remain absent from timeout collection.

### Dispatch state construction

Update the fresh assignment construction in `schedule_ready_tickets`:

```rust
SeatAssignmentState::Starting {
    generation: attempt_lease.attempt_id,
    start_deadline: None,
}
```

Change the post-insert arming condition from:

```rust
assignment_generation.is_some() && transition_state == Idle
```

to a state/transport condition that arms any applicable acceptance wait after immediate
delivery while preserving delayed cross-provider behavior. The narrow shape is:

```rust
if transition_state == TransitionState::Idle {
    self.start_assignment_ack_wait(pane_id, SystemTime::now());
}
```

Why this is safe:

- immediate fresh launch: `Starting(None)` becomes armed;
- reused Codex clear handshake: not Idle, still arms after prompt delivery;
- same-process Claude reuse: state is `Owned`, helper returns false;
- cross-provider recycle: not Idle, arms after exit-grace launch;
- `FreshExec`: Idle after launch, so startup becomes armed.

The `check_transition_timeouts` exit-grace launch path already calls
`start_assignment_ack_wait(pane_id, now)` after sending the prepared command. No new
call site is required there.

### Poll loop

No call ordering change. Existing sequence already provides the safety order:

1. consume process-start signals;
2. process transition delivery;
3. evaluate assignment/start deadlines.

Only the timeout evaluator's recognized states change.

### Dashboard conversion

Extend `State::to_ui_state`:

```rust
SeatAssignmentState::StartupFailed => {
    ui::SeatAssignmentStatus::StartupFailed
}
```

Update all direct `Starting` equality expectations to include `start_deadline` or use
`matches!` where the deadline is not the focus. Tests that verify launch timing should
inspect the field exactly.

### Native regression test

Add a sibling to `test_fresh_dispatch_becomes_owned_only_after_exact_process_start`:

```rust
#[test]
fn test_missing_fresh_start_signal_fails_within_bound_without_relaunch()
```

Use `pane_name_schedule_state("claude", AgentClient::Claude, None)` and set the timeout
to one before scheduling.

Test observations:

- extract current lease and `Some(start_deadline)` from `Starting`;
- assert generation matches the lease;
- assert `seat_is_owned` is false;
- snapshot launch evidence before expiry;
- evaluate exactly at the stored deadline;
- assert `StartupFailed`, never owned, failed thread, retained slot/ticket/lease;
- assert a single matching alert and an actionable error activity;
- assert dashboard contains `startup-failed`;
- evaluate again after one and many configured intervals;
- assert state, reservation, and launch evidence are unchanged.

Prefer existing command/event storage over introducing a test-only production counter.

### Existing test updates

Compiler-guided updates are expected in:

- fresh fallback route naming test;
- positive exact process-start test;
- split-brain or stale-liveness fixture assertions;
- any manual `seat_assignments.insert(Starting { ... })` construction.

Do not loosen E-033 tests. Their pending, recovering, and recovery-failed values must
remain byte-for-byte semantically equivalent.

## `crates/lisa-plugin/src/ui.rs`

### Public UI enum

Add:

```rust
StartupFailed,
```

to `SeatAssignmentStatus`.

### Label

Add:

```rust
Self::StartupFailed => "startup-failed",
```

The label is distinct from `recovery-failed` so operators know whether the initial
provider-start boundary or E-033's recovery acknowledgment boundary failed.

### Color

Group with terminal red statuses:

```rust
Self::RecoveryFailed | Self::StartupFailed => RED,
```

No layout changes are required. The existing seat-status cell sizes itself from label
content and already renders the longer `assigned-pending-ack` string.

## Configuration and documentation files

No source changes to:

- `crates/lisa-core/src/types.rs`;
- `crates/lisa-cli/src/config.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-cli/src/init.rs`;
- `crates/lisa-cli/src/setup_guide.rs`.

E-033 already provides, validates, transports, and documents the positive 30-second
`assignment_ack_timeout_secs` value. This ticket consumes it for the related fresh-start
acceptance boundary.

## Implementation ordering

1. Extend internal and UI enums plus mappings.
2. Extend deadline arming and timeout collection.
3. Add the retained startup failure helper.
4. Arm immediate launch routes after assignment insertion.
5. Update existing `Starting` tests and fixtures.
6. Add the missing-signal bounded regression.
7. Format and run focused then full verification.
8. Commit both source paths together through Lisa's isolated transaction.

## Source ownership and commit boundary

The meaningful source unit is the assignment behavior plus its UI state. Rust exhaustive
matching makes the two files one compiling atomic unit.

Commit with exact includes only:

```text
crates/lisa-plugin/src/lib.rs
crates/lisa-plugin/src/ui.rs
```

Do not include attempt artifacts, ticket frontmatter, Lisa provenance, or concurrent
story/ticket changes. After commit, both owned source paths must be clean and unstaged.
