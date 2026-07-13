# Research — T-045-03-03 delivered awaiting claim

## Ticket boundary

T-045-03-03 is the final scheduler ticket in story S-045-03.

The acceptance criterion requires one scheduler-level sequence:

- a ticket assignment has been delivered to a live Codex TUI;
- no `UserPromptSubmit` hook evidence arrives before the old deadline;
- the scheduler enters a real delivered-awaiting-claim state;
- the scheduler sends no duplicate assignment prompt;
- the passive wait remains finite;
- expiry ends in a named operator-actionable state;
- the result is not reported as delivery failure.

The story explicitly excludes live Codex/Zellij proof.
That proof belongs to S-045-05.
This ticket is fixture-driven and remains inside the scheduler ownership boundary.

## Predecessor state

T-045-03-01 connected pane-scoped assignment claims to ownership.

The scheduler consumes `pane-<id>.claim`, validates the claim against the slot,
current attempt lease, retained assignment reference, and assignment nonce, and then
promotes the seat to `Owned`.

The claim can establish ownership without any hook record.

T-045-03-02 ranked the remaining evidence paths.

A matching `UserPromptSubmit` remains a supplemental fast path.
A valid artifact admitted from the exact current attempt work directory is a bounded
fallback.
Stale hook and artifact evidence is consumed or rejected without granting ownership.

All three evidence paths use `active_assignment_generation` to determine whether a
seat is in an unowned state that can still become owned.

## Scheduler location

The project documentation refers to `scheduler.rs`, but the active scheduler remains
implemented in `crates/lisa-plugin/src/lib.rs`.

The assignment machine is `SeatAssignmentState`, stored by physical pane ID in
`State::seat_assignments`.

The relevant current states are:

- `Starting`, while a fresh provider launch is in its readiness window;
- `ReadyForAssignment`, after positive process-start readiness;
- `Delivering`, after a bounded assignment reference was typed;
- `AssignedPendingAck`, for older/reused delivery paths;
- `Owned`, after accepted ownership evidence;
- `Recovering`, during the existing one-fresh-session fallback;
- `DeliveryFailed`, `RecoveryFailed`, and `StartupFailed`, retained terminal states.

Only `Owned` satisfies `seat_is_owned`.
A slot reservation and a live process do not imply ownership.

## Fresh Codex delivery path

S-045-02 changed fresh Codex launch transport.

Before dispatch, the scheduler writes an immutable, nonce-bearing assignment file.
The Codex adapter builds a Lisa launcher command containing that exact file path.
Zellij receives a bounded launch-script invocation rather than the assignment body.

Codex uses `ResetStrategy::ExitThenFresh` and `ReadinessMode::Grace`.
An empty seat launches immediately.
A resident seat receives `/exit`, waits for the shell grace, and then launches a
fresh Codex process for the new ticket.

After fresh launch, the slot records:

- `has_session = true`;
- `last_client = Codex`;
- the exact ticket and attempt lease;
- `SeatAssignmentState::Starting`;
- an absolute startup grace deadline.

When that grace expires, `check_assignment_ack_timeouts_at` calls
`deliver_assignment_to_pane`.
That helper types an attempt-tagged assignment-file reference into the TUI and changes
the state to `Delivering` with an acknowledgement deadline and retry count zero.

The existing tests call this deterministic sequence through
`exit_then_deliver_fresh_codex`.

## Existing retry behavior

`MAX_ASSIGNMENT_DELIVERY_RETRIES` is one.

When a `Delivering` deadline expires with retries below that maximum,
`check_assignment_ack_timeouts_at` calls `deliver_assignment_to_pane` again with an
incremented retry count.

That helper performs another `send_line_to_pane` call.
It queues another deferred Enter and logs another `delivering assignment` event.

The second deadline therefore represents an active duplicate chat injection, not a
passive evidence wait.

If the retry deadline expires, `fail_assignment_delivery` changes the seat to
`DeliveryFailed`, marks the logical thread failed, appends pre-ownership provenance,
adds an error alert, and tells the operator to reset the ticket.

This path cannot distinguish an unavailable delivery boundary from a live Codex TUI
that accepted the assignment but emitted no hook while its first turn remained slow.

## Live-TUI evidence available to the scheduler

`AgentSlot` owns physical-seat lifecycle data.

For this transition the scheduler can observe:

- the addressed pane exists in `agent_slots`;
- the slot still retains the ticket reservation and current attempt lease;
- `has_session` is true after the fresh launch is submitted;
- `last_client` is `AgentClient::Codex`;
- the transition state is normally `Idle` after launch;
- the assignment machine still carries the exact generation in `Delivering`.

There is no trustworthy pre-prompt Codex process-start hook.
That absence is why Codex uses grace-mode readiness.

The ticket's fixture-level “alive TUI” premise therefore maps to scheduler-owned slot
lifecycle state, not an invented hook signal.

The existing `.error` consumer independently handles an actual launcher or child
failure.

## Timeout machinery

`assignment_ack_deadline` derives a finite absolute deadline from
`PluginConfig::assignment_ack_timeout_secs` plus the deferred-Enter transport delay.

The default configured acknowledgement timeout is 30 seconds.
Configuration validation requires a positive value.

`DeadlineEvaluator::acknowledgements` is generic over the copied state value.
It returns candidates whose absolute deadline is at or before injected `now`.

`check_assignment_ack_timeouts_at` snapshots state and deadline together, evaluates
expiry, and rechecks exact state equality before mutating.

This makes a new deadline-bearing assignment state compatible with the existing
deterministic timeout evaluator without changing `deadline.rs`.

Signal consumers run before timeout evaluation in `poll_tick`.
A claim, matching hook, or admitted artifact at the boundary can therefore publish
`Owned` before an expired passive wait is processed.

## Ownership admission boundary

`active_assignment_generation` currently admits:

- `Delivering`;
- `AssignedPendingAck`;
- `Recovering`.

`admit_assignment_claim`, `acknowledge_codex_assignment`, and
`admit_artifact_ownership` all depend on it.

A new delivered-awaiting-claim state must expose its generation through this helper
or it would become a dead state that cannot accept the evidence it is waiting for.

Once any evidence path inserts `Owned`, later timeout evaluation sees a state mismatch
and performs no terminal transition.

## Failure and provenance vocabulary

`FailureTransitionOutcome` gives tests and callers typed results for completed failure
mutations.

Current assignment outcomes distinguish delivery, recovery, and startup failures.
There is no claim-wait timeout outcome.

`lisa_core::provenance::AssignmentState` is stable serialized evidence vocabulary.
It currently contains:

- `delivery-failed`;
- `recovery-failed`;
- `startup-failed`.

`emit_assignment_transition` writes the exact attempt lease, pane, provider, named
state, reason, and bounded timestamps to the mixed provenance ledger.

A new terminal state needs its own provenance name if the result is to remain distinct
from false delivery failure across dashboard and CLI retrieval boundaries.

## Dashboard projection

`State::to_ui_state` maps every private assignment state to
`ui::SeatAssignmentStatus`.

The UI currently renders pending states yellow, ownership green, recovery bright
yellow, and terminal states red.

The story says delivered-awaiting-claim must be a scheduler transition rather than a
dashboard-only label.
The UI should therefore project new private states after the scheduler states exist;
it must not infer them from elapsed time or missing signals.

## Existing regression surface

The embedded `lib.rs` tests cover:

- Codex startup grace into `Delivering`;
- exact hook ownership;
- exact claim-only ownership;
- hook acceleration while a claim is pending;
- current-attempt artifact fallback;
- stale evidence rejection;
- one chat retry followed by `DeliveryFailed`;
- terminal assignment provenance and dashboard projection;
- Claude's SessionStart and delivery paths.

Several historical Codex tests encode the behavior this ticket intentionally changes:
they expect retry count one and a later `DeliveryFailed`.

Claude tests also use `Delivering`, so changing every delivery timeout globally would
violate the explicit story boundary that Claude keeps its existing mechanism.

The discriminating inputs are the live slot's `last_client` and `has_session` values.

## Workflow and worktree constraints

Phase artifacts belong only under:

`.lisa/attempts/T-045-03-03/1/work`

Lisa-managed provenance, completion journal, and materialized planning files are
already dirty or untracked and are not owned by this ticket.

Ticket source must be committed with `lisa commit-ticket` and exact include paths.
The ordinary Git index must remain unused.

## Constraints surfaced

- Claim, hook, and artifact evidence must remain valid during the new passive state.
- The live-Codex path must perform no assignment send when its first deadline expires.
- The passive wait must use an absolute finite deadline.
- The terminal result must not reuse `DeliveryFailed`.
- The terminal reservation should remain retained for operator reset, matching other
  pre-ownership terminal states.
- A late claim after terminal expiry must not restore ownership.
- Claude delivery retry semantics must remain unchanged.
- `.error`, lease fencing, attempt generation matching, and poll ordering remain
  authoritative and unchanged.
- Fixture tests should count actual send-side observations, not only inspect a label.
