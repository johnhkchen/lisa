# Research: surface handoff state in the dashboard

## Ticket boundary

- Ticket `T-033-02-01` begins in `research`.
- Its single acceptance criterion concerns the dashboard representation of a
  recycled Codex pane during three scheduler-owned assignment states.
- The required visible labels are `assigned-pending-ack`, `owned`, and
  `recovering`.
- The test must demonstrate the state changes at dashboard level.
- The indicator must originate in scheduler state rather than pane text or
  terminal-content inspection.
- The ticket does not request a change to acknowledgment detection, timeout
  policy, recovery launch behavior, pane naming, or ticket frontmatter.

## Relevant crate and module boundary

- Dashboard scheduling and rendering live in `crates/lisa-plugin`.
- `crates/lisa-plugin/src/lib.rs` owns the runtime `State`, Zellij event
  processing, scheduler transitions, and projection into UI data.
- `crates/lisa-plugin/src/ui.rs` owns presentation-only structs and terminal
  dashboard rendering.
- `State::render` calls `State::to_ui_state`, then
  `ui::print_dashboard`.
- The UI module does not read signal files, terminal panes, or scheduler maps.
- This establishes a deliberate boundary: `lib.rs` supplies facts and `ui.rs`
  formats those facts.

## Scheduler-owned assignment model

- `SeatAssignmentState` is private to `lib.rs` and keyed by physical terminal
  pane ID in `State::seat_assignments`.
- Absence from the map means the seat is unassigned.
- `AssignedPendingAck { generation, ack_deadline }` means a recycled Codex seat
  is reserved but has not positively acknowledged the assignment.
- `Owned` means the provider has accepted the assigned ticket.
- `Recovering { generation, ack_deadline }` means the original delivery timed
  out and the one permitted fresh-session fallback is in progress.
- `RecoveryFailed` means that fallback failed and the reservation is retained
  for explicit operator recovery.
- `AgentSlot.ticket_id` remains the reservation/routing key; it does not encode
  provider acceptance.
- `TransitionState` separately describes transport lifecycle such as waiting
  for `/clear` or `/exit`.
- Consequently, `transition_state` cannot substitute for assignment state: a
  pending or recovering seat may independently be in several transport states.

## Existing assignment transitions

- `schedule_ready_tickets` records reused Codex seats as
  `AssignedPendingAck` with a new generation.
- Fresh Codex and Claude assignments retain the established immediate `Owned`
  behavior.
- `acknowledge_codex_assignment` compares a native Codex payload with the
  currently reserved ticket and active generation.
- Only an exact match replaces pending or recovering state with `Owned`.
- Stale and duplicate acknowledgments do not create another ownership edge.
- `start_assignment_ack_wait` arms the deadline only after prompt delivery.
- `check_assignment_ack_timeouts_at` is the deterministic timeout seam used by
  scheduler tests.
- Expired pending state invokes `begin_assignment_recovery`.
- Recovery installs a new generation before sending `/exit`, fencing late
  acknowledgments for the abandoned generation.
- Existing tests cover pending-to-owned and pending-to-recovering scheduler
  transitions without sleeping.

## Existing ownership helper

- `State::seat_assignment(pane_id)` returns the explicit state.
- `State::seat_is_owned(pane_id)` is true only for `Owned`.
- The helper currently carries an `#[allow(dead_code)]` comment stating that
  S-033-02 will project it to the UI.
- A boolean alone cannot meet the dashboard requirement because pending and
  recovering are both not-owned yet must remain visibly distinct.
- The full enum is therefore the authoritative source needed by the UI
  projection.

## Current UI data model

- `ui::PluginState` contains tickets, active threads, parked threads, activity,
  alerts, slots, timing, modal state, pause state, and view preset.
- `ui::SlotInfo` currently contains `ticket_id`, `slot_number`, and a boolean
  `transitioning`.
- `ui::ActiveThread` contains ticket, phase, start time, slot number, awaiting
  state, and a preformatted provider/model route.
- No UI-facing type currently represents seat assignment or handoff state.
- `State::to_ui_state` builds slots by enumerating `agent_slots`.
- It derives `transitioning` from `TransitionState` and cooldown time.
- It currently does not consult `seat_assignments` while building `SlotInfo`.

## Current thread table behavior

- `ui::render_threads` is slot-centric and renders one row per slot.
- Active rows currently show `Running`, or `Awaiting` when the scheduler's
  awaiting-human set says so.
- Parked rows show `Parked`.
- Unoccupied transitioning rows show `Winding Down`.
- Other unoccupied rows show `Idle`.
- The table includes SLOT, TICKET, PHASE, AGENT, STATUS, and TIME columns.
- The STATUS column is the existing at-a-glance lifecycle surface.
- A recycled Codex assignment has an active `Thread`, so it currently renders
  as ordinary `Running` throughout pending, owned, and recovery states.
- The existing `transitioning` field only affects rows without active or parked
  threads, so recovery transport is hidden behind the active-row branch.

## Existing dashboard test patterns

- UI tests live inline in `ui.rs` and construct `PluginState` values directly.
- Most tests call `render_threads`, join returned lines, and assert selected
  tokens.
- Full-dashboard tests call the private `render_dashboard_lines` helper.
- The crate does not depend on `insta` or store `.snap` files.
- Existing “snapshot” terminology elsewhere in `lib.rs` refers to the state
  dump generated by `format_snapshot`, not golden-file dashboard testing.
- Scheduler tests live inline in `lib.rs` and can directly mutate private state.
- Those tests already have `pane_name_schedule_state`, a deterministic fixture
  for a resident pane and ready ticket.
- A scheduler-to-dashboard test can therefore exercise real assignment
  transitions and inspect a rendered UI representation without adding external
  dependencies.

## Constraints and invariants

- The UI should remain presentation-only; it should not infer state from route,
  thread status, pane title, terminal text, transition state, or activity logs.
- Pane ID is the stable join key between `AgentSlot` and `seat_assignments`.
- Slot number is the UI join key between `SlotInfo` and `ActiveThread`.
- Existing awaiting-human behavior must remain visible and should retain its
  current precedence when applicable.
- Fresh sessions and Claude sessions should not acquire misleading recycled
  handoff labels unless their scheduler assignment state explicitly requests
  one.
- `RecoveryFailed` exists even though the acceptance criterion names only three
  states; silently rendering it as normal running would hide an actionable
  scheduler condition.
- Column widths must accommodate the longest required label,
  `assigned-pending-ack` (20 characters).
- ANSI color codes are already embedded in rendered strings; assertions can
  focus on stable text tokens or normalize the output.
- Tests should use deterministic timestamps or compare only stable row content.

## Worktree and workflow observations

- The repository worktree contains many unrelated modified and untracked files.
- Ticket-owned source changes must be limited to exact paths and committed with
  `lisa commit-ticket` through an isolated index.
- Ordinary `git add` and `git commit` are prohibited by the project workflow.
- Work artifacts remain outside the source commit for Lisa's final completion
  transaction.
- The ticket frontmatter must not be manually edited.

## Research conclusion

- The assignment truth already exists and has deterministic transition seams.
- The missing link is an explicit UI-facing projection from pane-keyed
  `SeatAssignmentState` to the slot row rendered by `ui.rs`.
- The thread table STATUS cell is the established at-a-glance location.
- The acceptance test needs to cross the scheduler/UI boundary so a test that
  only constructs UI data would be insufficient evidence for scheduler
  sourcing.
