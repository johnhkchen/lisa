# Research: recycled-seat assignment state model

## Scope and workflow constraints

- Ticket: `T-033-01-01`, currently in Research.
- The ticket introduces scheduler state for recycled Codex assignments.
- The named states are `assigned-pending-ack`, `owned`, and `recovering`.
- The immediate acceptance criterion is state-model coverage, not acknowledgment detection.
- A recycled Codex seat must be assigned without being reported as owned.
- Fresh and reused slot bookkeeping must carry the state.
- Transition timeout handling must not accidentally erase or promote the state.
- Claude behavior must remain unchanged.
- Later tickets own the acknowledgment detector, ack-gated promotion, and bounded recovery.
- `T-033-01-02` detects ticket-scoped Codex acknowledgment.
- `T-033-01-03` promotes pending assignments to owned on a matching acknowledgment.
- `T-033-01-04` moves timed-out pending assignments to recovering and launches fallback.
- Dashboard rendering is explicitly outside this ticket and belongs to `S-033-02`.
- Ticket frontmatter phase and status are Lisa-owned and must not be edited here.
- Source work must be committed with `lisa commit-ticket` and exact include paths.

## Relevant source layout

- The scheduler and its native unit tests live in `crates/lisa-plugin/src/lib.rs`.
- Provider-specific commands and reset strategies live in `crates/lisa-plugin/src/adapter.rs`.
- Shared thread lifecycle types live in `crates/lisa-core/src/types.rs`.
- Pane labels are formatted in `crates/lisa-plugin/src/pane_name.rs`.
- Dashboard projection and rendering live in `crates/lisa-plugin/src/ui.rs`.
- This ticket can remain within `lib.rs`; no adapter protocol changes are required yet.
- The scheduler `State` owns the DAG, active threads, physical slots, and timers.
- `State` derives `Default`, which is used heavily by scheduler unit tests.

## Existing physical slot model

`AgentSlot` represents one pre-created terminal pane. Its relevant fields are:

- `pane_id`: the stable Zellij terminal pane identity.
- `ticket_id`: `Some` when the scheduler has associated a ticket with the slot.
- `has_session`: whether a resident agent TUI is believed to occupy the pane.
- `transition_state`: reset/recycle transport state.
- `transition_started_at`: timeout origin for the current transport transition.
- `cooldown_until`: earliest reuse time after release.
- `last_activity_at`: activity/wind-down clock.
- `last_client`: resident or incoming provider identity.

The slot has no independent assignment or ownership field today.

## Current assignment semantics

- `schedule_ready_tickets` selects a slot and immediately binds a ticket to it.
- The binding is `agent_slots[slot_idx].ticket_id = Some(ticket_id.clone())`.
- The binding happens after initial reset/launch input is sent.
- A `Thread` is then inserted with `ThreadStatus::Running`.
- No state distinguishes “reserved and prompt delivery in progress” from “agent accepted.”
- Consequently, every scheduler consumer sees the ticket as occupying the pane immediately.
- That is true for fresh launches, same-provider reuse, and cross-provider recycling.
- `ThreadStatus` cannot express assignment acknowledgment.
- Its variants are `Running`, `Parked`, `Completed`, and `Failed`.
- `Running` describes ticket execution lifecycle, not seat ownership evidence.

## Slot-selection paths

`find_slot_for_client` returns one of two selection categories:

- `Compatible(index)` for an empty shell or a resident session of the requested provider.
- `Recycle(index)` for a quiet resident session of the opposite provider.

The compatible category therefore contains two materially different cases:

- a fresh pane with `has_session == false`;
- a same-provider reused pane with `has_session == true`.

The recycle category always starts with a resident opposite-provider session.
The scheduler snapshots `has_session` before mutating the chosen path.
That existing fact can classify whether an assignment reuses a physical seat/session.

## Existing provider behavior

- Both `ClaudeCodeAdapter` and `CodexAdapter` use `ResetStrategy::ClearHandshake`.
- A same-provider live session receives `/clear`.
- The scheduler then waits in `TransitionState::WaitingForClear`.
- The `.cleared` hook causes the new ticket prompt to be typed.
- Text submission is deferred through the existing pending-Enter queue.
- A missing clear signal has a bounded clear timeout fallback.
- The fallback sends the prompt anyway and returns transport state to `Idle`.
- Codex lacks Claude’s `.idle` and `.awaiting` optional signals.
- Both providers emit normalized `.cleared`, `.stopped`, and heartbeat signals.
- No existing signal proves Codex accepted the newly injected ticket prompt.

## Existing transport transition state

`TransitionState` has four variants:

- `Idle`: no reset/recycle transport operation is pending.
- `WaitingForStop`: wait for stop before sending `/clear`.
- `WaitingForClear`: wait for clear before sending the ticket prompt.
- `WaitingForExit`: wait for an old provider to exit before a fresh launch.

This state machine answers “which pane command may be sent next.”
It does not answer “does the scheduler have positive ownership evidence.”
Overloading it with assignment states would couple two different clocks:

- reset/exit delivery deadlines;
- future ticket acknowledgment deadlines.

The later recovery ticket needs both dimensions at once, so they cannot be aliases.

## Timeout behavior

`check_transition_timeouts` scans each slot with `transition_started_at`.
It collects actions before mutating state to avoid borrow conflicts.

- `WaitingForExit` expires after `AGENT_EXIT_GRACE_SECS`.
- It launches the incoming provider at the shell.
- It then changes transport state to `Idle` and records a resident session.
- `WaitingForStop` falls back to `/clear` after silence and its timeout.
- `WaitingForClear` falls back to sending the reuse prompt after silence and timeout.
- The clear fallback changes transport state to `Idle`.

Because ownership is currently implicit in `ticket_id`, timeout completion also
implicitly looks owned even though neither prompt delivery nor acceptance was proven.
An independent assignment state must survive these transport mutations.

## Release and cleanup behavior

`release_slot_for_ticket` is the central normal release operation.

- It finds the slot by `ticket_id`.
- It clears `ticket_id`.
- It deliberately retains `has_session` for provider reuse.
- It establishes cooldown and renames the pane to an idle label.

The method is called from completion, failure, timeout, stale-thread, and audit paths.
Any separate assignment state must be cleared here to avoid stale ownership.

There are also cleanup paths where a pending recycle loses its ticket:

- `check_transition_timeouts` handles `WaitingForExit` with no ticket.
- It restores an empty shell, clears `last_client`, and renames the pane idle.

That path likewise must not leave assignment metadata behind.

## Discovery and default state

`discover_slots` creates `AgentSlot` values after a Zellij `PaneUpdate`.
Newly discovered panes have no ticket, no session, and idle transition state.
They therefore have no assignment.

Tests construct many `AgentSlot` literals directly.
Adding a mandatory field to `AgentSlot` would require broad mechanical churn.
`State` already owns several pane-keyed maps for derived scheduler facts:

- `last_pane_names`;
- human-attention debounce state;
- awaiting-human state.

A pane-keyed assignment map can preserve default construction and keep the change local.

## Existing test seams

- Scheduler tests are colocated in `lib.rs` under `#[cfg(test)]`.
- `fresh_slot` builds common slot fixtures.
- `pane_name_schedule_state` builds a one-ticket DAG and schedulable pane.
- Existing tests execute `schedule_ready_tickets` through the native host stubs.
- `test_pane_title_same_provider_reuse_replaces_stale_name` covers Codex reuse.
- `test_pane_title_cross_provider_switch_uses_incoming_provider` covers provider recycle.
- `test_recycle_exit_grace_launches_fresh_incoming_client` covers exit timeout launch.
- `test_check_transition_timeouts_clear_timeout` covers clear fallback.
- Release tests verify ticket removal, cooldown, and resident-session retention.

These seams can verify assignment state without adding integration infrastructure.

## Boundaries with later tickets

- This ticket should define the vocabulary and initial classification only.
- It should not parse lifecycle fixtures or hook payloads.
- It should not promote pending Codex state to owned.
- It should not add an acknowledgment deadline.
- It should not perform fresh-session fallback on missing acknowledgment.
- It should make `Recovering` representable for the bounded-recovery ticket.
- It should expose a scheduler-owned ownership query for tests and later UI projection.
- It should preserve pending state through current clear/exit timeout handling.

## Constraints and assumptions surfaced by the code

- “Recycled Codex” includes a released pane with an existing session being reassigned.
- Fresh Codex launch behavior is not identified as defective by this ticket.
- Claude’s existing assignment timing is the compatibility baseline.
- `ticket_id` remains necessary as a reservation/routing key during transitions.
- Therefore `ticket_id` cannot simply be withheld until acknowledgment.
- Thread creation also remains necessary for capacity accounting and lifecycle tracking.
- Assignment truth needs a separate scheduler-owned dimension.
- Absence of assignment metadata naturally represents an unassigned slot.
- An explicit `Owned` value permits unchanged Claude behavior to be asserted.
- `AssignedPendingAck` must not satisfy an ownership predicate.
- `Recovering` must not satisfy an ownership predicate.
- State transitions should be centralized enough for later tickets to mutate safely.

## Research conclusion

The defect is a missing state dimension, not a missing timer tweak. Physical slot
reservation (`ticket_id`), transport progress (`TransitionState`), session residency
(`has_session`/`last_client`), and acknowledged assignment ownership are separate facts.
The smallest code boundary is a scheduler-owned per-pane assignment state in `State`,
written during scheduling, preserved through transport timeout paths, and cleared by
normal release/abandonment. The existing adapter interface and Claude handshake can
remain unchanged while later tickets add the Codex acknowledgment transitions.
