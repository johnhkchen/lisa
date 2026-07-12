# Structure: surface handoff state in the dashboard

## Change set

The source change is confined to the plugin crate:

- modify `crates/lisa-plugin/src/ui.rs`;
- modify `crates/lisa-plugin/src/lib.rs`;
- create no new Rust module;
- add no dependency;
- delete no file.

Workflow artifacts for this ticket live under
`docs/active/work/T-033-02-01/` and are left for Lisa's completion transaction.

## `crates/lisa-plugin/src/ui.rs`

### New UI semantic type

Add `SeatAssignmentStatus` near `SlotInfo` because it is slot-level dashboard
data.

Shape:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatAssignmentStatus {
    AssignedPendingAck,
    Owned,
    Recovering,
    RecoveryFailed,
}
```

This type intentionally omits scheduler generations and deadlines. It models
only presentation-relevant assignment semantics.

Add private presentation methods:

```rust
fn label(self) -> &'static str
fn color(self) -> &'static str
```

Labels are stable acceptance-test vocabulary. Colors remain UI implementation
detail.

### Extend `SlotInfo`

Add:

```rust
pub assignment_status: Option<SeatAssignmentStatus>,
```

Meaning:

- `Some` is an explicit projection from scheduler seat truth;
- `None` means no current scheduler assignment or legacy/unavailable data;
- this field must never be populated by renderer inference.

All direct `SlotInfo` fixtures in `ui.rs` receive `assignment_status: None`
unless a focused renderer test intentionally exercises a state.

### Thread renderer

Change `render_threads` visibility from private to `pub(crate)` so the parent
module's scheduler test can render the real dashboard Threads section.

For active rows:

- retain the existing ticket cell and awaiting marker behavior;
- choose `Awaiting` when `active.awaiting` is true;
- otherwise choose the slot's explicit assignment status label/color;
- otherwise fall back to existing `Running`/green behavior.

The assignment is read from the same `SlotInfo` currently being iterated. No
secondary lookup, route parsing, or terminal inspection is introduced.

Increase STATUS formatting width to 20 characters in:

- the header;
- active rows;
- parked rows;
- winding-down rows;
- idle rows.

Increase the separator length to match the expanded table.

### UI unit tests

Keep existing tests compiling with `assignment_status: None`.

Add a small renderer-focused test if useful to pin:

- exact labels;
- assignment status overriding generic Running;
- legacy `None` retaining Running.

The acceptance-level state transition snapshot belongs in `lib.rs`, because
only scheduler state can prove the source of the labels.

## `crates/lisa-plugin/src/lib.rs`

### Scheduler-to-UI mapping helper

Add a private pure conversion near existing phase/status UI conversions or
inside `to_ui_state`:

```rust
fn seat_assignment_status_to_ui(
    state: SeatAssignmentState,
) -> ui::SeatAssignmentStatus
```

Mapping is exhaustive:

- `AssignedPendingAck { .. }` -> `AssignedPendingAck`;
- `Owned` -> `Owned`;
- `Recovering { .. }` -> `Recovering`;
- `RecoveryFailed` -> `RecoveryFailed`.

The wildcard payload patterns deliberately discard generation and deadline at
the UI boundary.

### `State::to_ui_state`

While enumerating each `AgentSlot`, populate `SlotInfo.assignment_status` with:

```rust
self.seat_assignment(s.pane_id)
    .map(seat_assignment_status_to_ui)
```

This is the only production source for the UI field.

The slot's `ticket_id`, `slot_number`, and `transitioning` calculation remain
unchanged.

### Ownership helper cleanup

Remove or update the obsolete `#[allow(dead_code)]` comment on
`seat_is_owned` only if production projection actually calls that helper.

The chosen mapping needs the full enum, so `seat_is_owned` remains primarily a
test helper. Its existing annotation can be reworded rather than falsely
claiming the UI consumes the boolean.

### Acceptance snapshot test

Add one test adjacent to the existing recycled Codex assignment/recovery tests.

The test uses the established `pane_name_schedule_state` fixture. It creates
three rendered checkpoints through real scheduler transitions:

1. pending checkpoint after `schedule_ready_tickets`;
2. owned checkpoint after exact `acknowledge_codex_assignment`;
3. recovering checkpoint on a second state after prompt delivery and injected
   deadline expiry.

At each checkpoint:

- call `State::to_ui_state`;
- call `ui::render_threads`;
- isolate the row containing `T-NAME`;
- remove ANSI control sequences with a test-local helper;
- normalize trailing whitespace;
- append a named checkpoint to a snapshot string.

Use one exact `assert_eq!` multiline literal. The expected snapshot includes
slot number, ticket, phase, route, and state label, demonstrating that the same
kind of recycled Codex dashboard row changes assignment indicator.

The recovery branch will use:

- `assignment_ack_timeout_secs = 1`;
- `handle_cleared_signal(10)` to deliver and arm the prompt;
- the stored deadline from `AssignedPendingAck`;
- `check_assignment_ack_timeouts_at(deadline)`.

No sleep, filesystem signal race, terminal buffer, or wall-clock inference is
needed.

## Test-only ANSI normalization

Implement a compact helper inside the `#[cfg(test)]` module rather than adding
a crate dependency.

It should skip bytes from ESC `[` through the terminating `m`, preserving all
ordinary UTF-8 row text. The dashboard uses color/style SGR sequences in this
path, so a narrow helper is sufficient.

Alternatively, the test may strip the known constants if that is clearer. The
helper is not production API.

## Dependency direction

The resulting flow is:

```text
State.seat_assignments[pane_id]
    -> State::to_ui_state
    -> ui::SlotInfo.assignment_status
    -> ui::render_threads STATUS cell
```

No arrow points back from `ui.rs` into scheduler behavior.

## Ordering

1. Define the UI enum and extend `SlotInfo`.
2. Update all UI fixtures to compile.
3. Update `render_threads` and its layout.
4. Add the scheduler-to-UI mapping.
5. Populate the new field in `to_ui_state`.
6. Add the scheduler-driven snapshot test.
7. Format and run focused tests.
8. Run workspace and WASM verification.
9. Commit exactly the two source paths with Lisa's isolated transaction.

## Non-changes

- no changes to `SeatAssignmentState` transitions;
- no changes to Codex acknowledgment parsing;
- no changes to recovery deadlines or launches;
- no changes to ticket/thread persistence;
- no changes to pane naming;
- no changes to dashboard view navigation;
- no changes to ticket frontmatter;
- no terminal-content inspection.
