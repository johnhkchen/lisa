# Review: surface handoff state in the dashboard

## Outcome

The Operations dashboard now exposes the scheduler's explicit physical-seat
assignment truth in the Threads STATUS column.

A recycled Codex pane displays:

- `assigned-pending-ack` after handoff and before positive acceptance;
- `owned` after an exact current ticket/generation acknowledgment;
- `recovering` after the bounded acknowledgment deadline expires.

These labels are driven solely by `State::seat_assignments`, joined by physical
pane ID during `State::to_ui_state`. The renderer does not inspect terminal
contents and does not infer ownership from routes, pane titles, ticket
reservations, threads, activity events, or transport transitions.

The source implementation is committed as:

```text
a7f016f4eeafffa50a148882c30295d86f6a1586
feat: surface Codex handoff state in dashboard
```

Commit scope:

```text
2 files changed, 158 insertions(+), 12 deletions(-)
```

The source commit was created through Lisa's isolated transaction with two
exact include paths. The installed Homebrew Lisa predates `commit-ticket`, so
the repository-built `target/debug/lisa` was used.

## Files modified

### `crates/lisa-plugin/src/ui.rs`

Added `SeatAssignmentStatus`, a UI-specific semantic reduction of the scheduler
state:

```text
AssignedPendingAck -> assigned-pending-ack
Owned              -> owned
Recovering         -> recovering
RecoveryFailed     -> recovery-failed
```

The type deliberately omits assignment generation and acknowledgment deadline.
Those are scheduler mechanics and are not needed for at-a-glance presentation.

Added `PluginState.seat_assignment_statuses`, keyed by dashboard slot number.
An absent entry preserves the pre-existing generic `Running` behavior for
legacy or unassigned slots.

The Threads renderer now selects active status in this order:

1. `Awaiting`, when the existing awaiting-human scheduler state is set;
2. explicit seat assignment status;
3. legacy `Running` fallback.

This preserves the existing immediate-human-attention signal. Recycled Codex
handoffs do not synthesize awaiting state, so all three required assignment
indicators remain visible.

The STATUS column expanded from 14 to 20 characters to fit
`assigned-pending-ack`. Header, separator, active, parked, winding-down, and idle
row formats were adjusted together.

`render_threads` is now `pub(crate)` solely so the scheduler tests in the parent
module can render the production dashboard section. It is not a public crate
API.

### `crates/lisa-plugin/src/lib.rs`

`State::to_ui_state` now enumerates physical agent slots and looks up each
slot's `SeatAssignmentState` by pane ID. The mapping is exhaustive:

- pending payload fields are discarded at the UI boundary;
- owned maps directly;
- recovery payload fields are discarded at the UI boundary;
- terminal recovery failure maps to an explicit UI semantic state.

The mapped value is keyed by the existing one-based UI slot number. This keeps
the renderer independent of physical Zellij pane IDs while ensuring the source
lookup uses the scheduler's canonical pane identity.

Added an integrated test helper that renders the production Threads section,
strips ANSI SGR sequences, and normalizes dynamic elapsed time. It does not
alter production rendering.

Added
`test_dashboard_snapshot_shows_recycled_codex_handoff_states`, which exercises
real scheduler state transitions and asserts one exact multiline dashboard-row
snapshot.

## Acceptance criterion evaluation

### Distinct pending indicator on a recycled Codex pane

Met.

The test schedules `T-NAME` onto a pane with a resident Codex session. The real
scheduler inserts `AssignedPendingAck`, `to_ui_state` projects it, and the
dashboard row snapshots:

```text
[1]    T-NAME       RES        codex          assigned-pending-ack <elapsed>
```

The test does not manually construct UI assignment state.

### Flip to owned on acknowledgment

Met.

The same scheduler instance receives a native Codex acknowledgment containing
the exact current ticket and generation. `acknowledge_codex_assignment` performs
the production transition to `Owned`. A second render snapshots:

```text
[1]    T-NAME       RES        codex          owned                <elapsed>
```

### Show recovering after timeout

Met.

A second deterministic fixture schedules the same recycled handoff, delivers
the prompt through `handle_cleared_signal`, reads the armed scheduler deadline,
and evaluates timeout at that deadline. The production recovery edge changes
the state to `Recovering`; the rendered snapshot is:

```text
[1]    T-NAME       RES        codex          recovering           <elapsed>
```

No sleep or wall-clock race is involved.

### Scheduler state rather than terminal inference

Met structurally and by test construction.

The only production population site is:

```text
seat_assignment(slot.pane_id)
    -> SeatAssignmentStatus
    -> seat_assignment_statuses[slot_number]
    -> STATUS cell
```

No terminal-read API or pane-content field is referenced in either change.

## Test coverage

### Acceptance-level test

Passed:

```text
cargo test -p lisa-plugin dashboard_snapshot_shows_recycled_codex_handoff_states
1 passed, 0 failed
```

This covers the scheduler-to-projection-to-renderer path for all three required
states.

### Focused UI regressions

Passed:

```text
cargo test -p lisa-plugin render_threads
9 passed, 0 failed
```

Coverage retains active Running fallback, Awaiting precedence, route display,
parked rows, mixed active/idle/winding-down rows, all-running, all-idle, and
empty dashboard behavior.

### Focused scheduler regressions

Passed:

```text
cargo test -p lisa-plugin recycled_codex_ownership
1 passed, 0 failed

cargo test -p lisa-plugin bounded_ack_wait
1 passed, 0 failed
```

These retain exact-once acknowledgment and bounded one-shot recovery behavior.

### Broad verification

Passed:

```text
cargo test --workspace
270 CLI + 1 integration + 150 core + 266 plugin = 687 passed

cargo clippy -p lisa-plugin --all-targets -- -D warnings

cargo fmt --all -- --check

cargo build -p lisa-plugin --target wasm32-wasip1 --release

git diff --check -- crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/ui.rs
```

## Design deviation

The initial Structure placed optional assignment status directly on
`SlotInfo`. Implementation instead stores a typed slot-number-keyed map on
`PluginState`.

This retains the same scheduler authority and typed UI boundary while avoiding
mechanical changes to nineteen unrelated slot fixtures. It also represents
absence naturally and keeps `SlotInfo` focused on stable slot structure. The
deviation and rationale were recorded in `progress.md` before broad
verification.

## Open concerns and limitations

### Owned is now the normal explicit assignment label

All scheduler assignments recorded as `Owned`, including fresh Codex and Claude
seats, display `owned` rather than the older generic `Running`. This is
intentional: once the dashboard has explicit scheduler ownership data, hiding
it for non-recycled assignments would make identical scheduler state render
differently based on history that `Owned` does not retain.

Legacy active slots without a `seat_assignments` entry still display `Running`.

### Recovery-failed row behavior

`RecoveryFailed` is included in the typed UI projection and label vocabulary.
The scheduler also marks its thread failed, so the existing failed alert is the
primary visible operator surface and the active-thread branch no longer renders
that row as an active assignment status. This ticket's criterion ends at
`recovering`; a later operations ticket may choose to render retained failed
seat reservations directly in the slot row as well.

### Narrow terminals

The table's logical width increased by six columns to preserve the full pending
label. Existing dashboard code already permits logical lines wider than narrow
panes; this change does not add truncation or horizontal scrolling. Operators
on very narrow panes may see terminal wrapping, as they could with long ticket
or route cells before this change.

### No live Zellij screenshot

Verification uses the production renderer and real scheduler transitions in a
native deterministic test plus a release WASM build. No separate live Zellij
session was launched because the acceptance criterion requests snapshot test
evidence and the current worktree/session contains concurrent unrelated work.

## Critical issues

None found.

The acceptance criterion is fully covered, all focused and broad verification
passes, and the source commit is isolated to the intended two files.

## Workflow integrity

- All six RDSPI phases were completed continuously.
- `research.md`, `design.md`, `structure.md`, `plan.md`, `progress.md`, and
  `review.md` exist under the ticket work directory.
- Ticket phase and status frontmatter were not manually edited.
- The ordinary Git index is empty.
- Ticket-owned source paths are clean after the isolated commit.
- Unrelated modified and untracked files remain untouched.
- No next ticket was started.
