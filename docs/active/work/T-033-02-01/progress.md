# Progress: surface handoff state in the dashboard

## Current state

Implementation, verification, the isolated source commit, and Review are
complete.

## Completed phases

- [x] Research mapped scheduler assignment truth and the UI projection boundary.
- [x] Design evaluated inference, strings, shared scheduler types, and a narrow
  typed UI representation.
- [x] Structure defined the two-file plugin change and integrated test seam.
- [x] Plan sequenced implementation, focused tests, broad verification, and
  isolated commit ownership.

## Implementation completed

### Typed UI state

- [x] Added `ui::SeatAssignmentStatus`.
- [x] Added `AssignedPendingAck`, `Owned`, `Recovering`, and `RecoveryFailed`.
- [x] Added exact labels:
  - `assigned-pending-ack`;
  - `owned`;
  - `recovering`;
  - `recovery-failed`.
- [x] Added presentation-owned colors for every variant.

### Scheduler projection

- [x] `State::to_ui_state` enumerates physical agent slots.
- [x] Each slot is joined to `State::seat_assignments` by physical pane ID.
- [x] The scheduler enum is exhaustively reduced to the UI enum.
- [x] Projected states are keyed by dashboard slot number.
- [x] Missing scheduler entries remain absent, preserving legacy fallback.
- [x] No route, pane title, terminal content, thread status, transition state, or
  activity log is used to infer assignment ownership.

### Dashboard rendering

- [x] Threads STATUS displays explicit assignment state for active seats.
- [x] Awaiting-human retains priority for immediate operator action.
- [x] Missing assignment projection retains `Running`.
- [x] STATUS width increased from 14 to 20 for the longest required label.
- [x] Header, separator, active, parked, winding-down, and idle row widths were
  adjusted consistently.
- [x] Threads-section renderer is crate-visible for integrated testing only.

### Acceptance snapshot

- [x] Added `test_dashboard_snapshot_shows_recycled_codex_handoff_states`.
- [x] The test schedules a real ready ticket onto a resident Codex pane.
- [x] First dashboard checkpoint shows `assigned-pending-ack`.
- [x] An exact current ticket/generation ack drives the second checkpoint to
  `owned`.
- [x] A second state delivers the real prompt, arms its deadline, and expires it
  through `check_assignment_ack_timeouts_at`.
- [x] The third checkpoint shows `recovering`.
- [x] ANSI SGR sequences and dynamic elapsed time are normalized test-locally.
- [x] One exact multiline assertion snapshots slot, ticket, phase, agent, and
  all three required status labels.

## Plan deviation

The Design/Structure draft placed the optional UI assignment value directly on
every `SlotInfo`. During implementation, it became clear that a
`PluginState.seat_assignment_statuses` map keyed by slot number preserves the
same typed scheduler-to-UI boundary with less coupling and avoids modifying
nineteen unrelated `SlotInfo` test fixtures.

The deviation does not change behavior or source authority:

```text
State.seat_assignments[pane_id]
    -> State::to_ui_state
    -> PluginState.seat_assignment_statuses[slot_number]
    -> render_threads
```

Rationale:

- assignment is an optional overlay on existing slot/thread presentation;
- absence has a natural map representation;
- `SlotInfo` remains the stable structural slot model;
- direct UI fixtures keep their prior empty-projection default;
- production still has exactly one explicit mapping site;
- the renderer still performs no inference.

No acceptance or testing step was dropped.

## Focused verification completed

Passed:

```text
cargo test -p lisa-plugin dashboard_snapshot_shows_recycled_codex_handoff_states
1 passed

cargo test -p lisa-plugin render_threads
9 passed

cargo test -p lisa-plugin recycled_codex_ownership
1 passed

cargo test -p lisa-plugin bounded_ack_wait
1 passed
```

The first attempted snapshot expectation used the ready phase and an idle dash.
The real scheduler correctly advances a scheduled ticket to Research and the
active row correctly displays elapsed time. The literal was corrected to the
actual production row, and elapsed time was normalized to prevent boundary
flakiness. No production behavior was changed in response.

## Source files changed

- `crates/lisa-plugin/src/lib.rs`
  - scheduler-to-UI mapping;
  - integrated snapshot helper and test.
- `crates/lisa-plugin/src/ui.rs`
  - UI assignment enum;
  - projected-state storage;
  - status rendering and table width.

## Worktree safety

- The repository had unrelated modified and untracked files before this ticket.
- Ticket source edits are confined to the two plugin files above.
- Ordinary `git add` and `git commit` have not been used.
- The ticket frontmatter has not been edited.
- Work artifacts are untracked as expected for Lisa's final transaction.

## Broad verification completed

Passed:

```text
cargo fmt --all -- --check

git diff --check -- crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/ui.rs

cargo clippy -p lisa-plugin --all-targets -- -D warnings

cargo test --workspace
270 CLI + 1 integration + 150 core + 266 plugin = 687 passed

cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

## Isolated source commit

The installed `/opt/homebrew/bin/lisa` predates `commit-ticket`. The
repository-built `target/debug/lisa` exposes the required command and was used
for the isolated transaction.

Commit:

```text
a7f016f4eeafffa50a148882c30295d86f6a1586
feat: surface Codex handoff state in dashboard
```

Exact includes:

```text
crates/lisa-plugin/src/lib.rs
crates/lisa-plugin/src/ui.rs
```

Post-commit checks:

- [x] commit contains exactly the two ticket-owned source paths;
- [x] both source paths are clean;
- [x] ordinary index is empty;
- [x] unrelated worktree changes remain untouched;
- [x] ticket frontmatter remains unedited;
- [x] artifacts remain for Lisa's completion transaction.

## Remaining work

- [x] Run Rust format check.
- [x] Run plugin Clippy with warnings denied.
- [x] Run ticket source `git diff --check`.
- [x] Run `cargo test --workspace`.
- [x] Build release WASM target.
- [x] Inspect final ticket source diff.
- [x] Commit exact source paths with `lisa commit-ticket`.
- [x] Verify ticket source paths are clean and ordinary index untouched.
- [x] Write `review.md` with outcome, test coverage, and open concerns.
