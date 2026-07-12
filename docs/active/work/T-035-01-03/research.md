# T-035-01-03 Research — gate Owned on observed start

## Ticket boundary

- The ticket changes the native scheduler's fresh-launch ownership contract.
- Immediately after a fresh dispatch, the physical seat must be reserved but not owned.
- Ownership becomes true only after Lisa observes the provider-neutral process-start signal.
- The pending state must remain visible through the existing seat-status dashboard column.
- Bounded recovery for a missing start signal belongs to the following ticket, T-035-01-04.
- Recycled Codex prompt acknowledgement from E-033 remains a separate acceptance path.
- Attempt lease fencing from E-034 remains the authority boundary for every signal.

## Relevant source layout

- `crates/lisa-plugin/src/lib.rs` owns scheduling, attempt leases, signal consumption,
  physical seat assignment state, native tests, and conversion into UI state.
- `crates/lisa-plugin/src/ui.rs` owns dashboard-facing seat status variants, labels,
  colors, and thread-row rendering.
- `crates/lisa-cli/src/templates.rs` owns the installed provider hook scripts.
- T-035-01-01 already changed the templates and native launch environment.
- T-035-01-02 already changed fresh launch delivery to an atomic script artifact.

## Process-start producer already present

- `templates::ON_START_HOOK` is provider-neutral.
- It runs from the native provider SessionStart/startup hook.
- Fresh launch commands export `LISA_PANE_ID`, `LISA_TICKET_ID`, and
  `LISA_ATTEMPT_ID` through `SpawnContext`.
- The hook constructs the exact compact JSON form of the expected `AttemptLease`.
- It reads `.lisa/signals/pane-<pane>.lease`.
- It emits no signal when the pane marker does not exactly match launch identity.
- On a match it atomically publishes `.lisa/signals/pane-<pane>.started`.
- The `.started` content is the exact scheduler-owned attempt lease bytes.
- Existing template fixture tests cover matching, missing, and mismatched identities.

## Current scheduling path

- `State::schedule_ready_tickets` finds a provider-compatible or recyclable slot.
- It records `reused_seat` before lifecycle mutations.
- It mints an `AttemptLease`, records it in `lease_high_water` and `current_leases`,
  and publishes the pane lease marker when the pane is initially fresh.
- It builds a `SpawnContext` from that exact lease.
- Fresh native launches go through `State::prepare_fresh_launch` before PTY delivery.
- The selected `AgentSlot` receives the ticket, lease, and provider.
- The scheduler then writes `seat_assignments[pane]`.
- Only reused Codex assignments currently receive `AssignedPendingAck`.
- Every other route, including every fresh launch, receives `Owned` immediately.
- The unconditional fresh `Owned` fallback is the precise ticket seam.

## Existing seat state model

- `SeatAssignmentState` is scheduler-owned truth, separate from terminal transition state.
- Absence from the map means that the physical seat is unassigned.
- `AssignedPendingAck` stores the lease generation and optional Codex ack deadline.
- `Owned` means the provider has accepted the ticket.
- `Recovering` and `RecoveryFailed` implement E-033's bounded reused-Codex fallback.
- `seat_is_owned` is true only for the exact `Owned` variant.
- Slot reservation and thread creation do not therefore require ownership to be true.
- Release paths remove the seat assignment independently of its variant.

## Existing signal admission pattern

- `check_heartbeat_signals` scans `.lisa/signals` for pane-scoped suffixes.
- It parses the file as `AttemptLease` and removes the file before admission.
- Admission requires all of the following:
  - the pane resolves to an assigned slot;
  - the slot ticket matches the candidate ticket;
  - the slot lease equals the candidate lease;
  - the candidate is current in `current_leases`.
- Invalid, malformed, stale, and unassigned signals are consumed without effects.
- This is the established E-034 fencing boundary for lease-bearing hook signals.
- `check_codex_ack_signals` is a distinct consumer for recycled native Codex prompts.
- `poll_tick` consumes heartbeat and Codex acknowledgement signals before timeouts.
- There is currently no `.started` consumer anywhere in the plugin.

## Fresh versus reused semantics

- A slot with no resident session receives a fresh provider process launch.
- Cross-provider recycle exits the resident TUI and later launches a fresh process.
- Some adapters use `FreshExec` after a resident process; that also launches a process.
- Native Claude same-provider reuse uses `/clear` and does not start a new process.
- Native Codex same-provider reuse uses `/clear` plus E-033's prompt acknowledgment.
- A SessionStart signal cannot gate in-process reuse because no process starts there.
- The ticket specifically replaces immediate ownership for fresh launches.
- Existing reused Claude ownership and reused Codex pending-ack behavior are outside it.

## Dashboard path

- `State::to_ui_state` maps each internal seat state to `ui::SeatAssignmentStatus`.
- `ui::render_threads` uses that status for the active row's status cell.
- `AssignedPendingAck` currently renders `assigned-pending-ack` in yellow.
- `Owned` renders `owned` in green.
- Recovery variants have distinct visible labels.
- A fresh starting state therefore needs an internal variant, a UI variant, and mapping.
- The existing seat-status channel needs no new dashboard column or rendering mechanism.

## Test infrastructure

- Native tests in `lib.rs` construct scheduler states and call `schedule_ready_tickets`.
- `pane_name_schedule_state` provides a compact one-ticket/one-pane scheduling fixture.
- Existing tests assert recycled Codex pending/ack/timeout/recovery behavior.
- `test_reused_claude_assignment_remains_owned` protects unchanged Claude reuse.
- Split-brain tests inject stale heartbeat and ack files and verify lease fencing.
- Dashboard snapshot helpers strip ANSI and normalize elapsed time.
- The acceptance test can drive a truly fresh slot, inspect pending state and label,
  write the exact current lease to `.started`, consume it, and inspect `Owned`.

## Constraints and assumptions

- The signal file is untrusted until exact lease admission succeeds.
- A signal must be removed even if malformed or stale so it cannot replay forever.
- A duplicate valid start signal must not create a second transition.
- A valid start for an already owned reused assignment must not alter its semantics.
- A stale predecessor start cannot promote a successor attempt or another pane.
- No new timeout belongs in this ticket; pending-start may persist until T-035-01-04.
- Ticket-owned source changes are expected in `lib.rs` and `ui.rs` only.
- Phase artifacts remain in the attempt-private directory for Lisa to admit.
