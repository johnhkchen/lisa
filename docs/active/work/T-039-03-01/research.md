# Research: failure and reclaim state machine

## Scope

This spike characterizes the current failure and reclaim behavior in
`crates/lisa-plugin/src/lib.rs` without changing it. The seven acceptance paths
are assignment delivery failure, assignment recovery failure, startup failure,
startup recovery failure, adapter error signal, session timeout, and stale-thread
reclamation.

The paths share scheduler state but do not share one teardown policy. Current
behavior separates failures which retain a physical seat for operator inspection
from reclaims which release a ticket for automatic scheduling.

## State authorities

- `current_leases: HashMap<TicketId, AttemptLease>` is current attempt authority.
- `lease_high_water` survives revocation and makes later attempt IDs monotonic.
- `seat_assignments` is scheduler-owned assignment truth per physical pane.
- `agent_slots` owns ticket-to-pane reservation, resident-session state, and pane
  transition eligibility.
- `threads` is authoritative for active execution and thread status.
- `emit_provenance` records a terminal run outcome while the thread still exists.
- `error_alerts` and `timeout_alerts` are dashboard-facing failure evidence.
- Retry authority is represented by whether the thread and slot reservation are
  retained or removed, not by a single retry counter.

## Assignment delivery failure

`check_assignment_ack_timeouts_at` evaluates a `Delivering` deadline. The
constant `MAX_ASSIGNMENT_DELIVERY_RETRIES` is one. An expired initial delivery is
resubmitted in the same pane, with the same generation and `retries: 1`. Expiry
after that invokes `fail_assignment_delivery`.

`fail_assignment_delivery` accepts `Starting`, `ReadyForAssignment`, or
`Delivering` and writes `SeatAssignmentState::DeliveryFailed`. When the slot has
a ticket, the thread becomes `Failed`, an error alert is added, and the activity
log requests operator reset. It does not revoke the current lease, release the
slot, remove the thread, close the pane, or emit provenance. Repeated timeout
polls do not match the terminal state.

## Assignment recovery failure

The older reused-Codex path begins in `AssignedPendingAck`. Its deadline invokes
`begin_assignment_recovery`, which validates the current predecessor lease,
mints exactly one successor, updates lease/slot/thread authority, moves the seat
to `Recovering`, and exits the old Codex TUI. The fresh recovery launch later
arms an acknowledgement deadline.

An expired recovery acknowledgement or a recovery-process `.error` invokes
`fail_assignment_recovery`. It writes `RecoveryFailed`, fails the retained
thread, adds an error alert, and requests operator reset. It retains lease,
reservation, thread, and pane, emits no provenance, and has no transition out on
another deadline poll. Invalid/missing recovery authority reaches the same
terminal state rather than starting another fallback.

## Startup failure

`fail_startup` handles a `Starting` seat when a first recovery cannot begin, for
example because the slot or predecessor lease is missing or stale. It writes
`StartupFailed`, fails the thread when a ticket reservation can be resolved, and
adds an error alert. It retains the current lease, reservation, thread, and pane.
It emits no provenance. This is an operator-reset terminal state.

## Startup recovery failure

For SessionStart readiness, the first `Starting` deadline invokes
`begin_startup_recovery`. That function validates the predecessor, revokes it,
mints one successor attempt, updates the slot and thread lease, writes
`ResettingStartup`, clears stale pane signals/attention, interrupts incomplete
shell input, and sends an exact shell-readiness probe in the same pane.

Missing shell readiness or missing replacement process start invokes
`fail_startup_recovery`. It writes `StartupFailed`, fails the thread, records an
error alert, revokes the successor lease, clears lifecycle/attention/input state,
sets the slot to `Fenced`, clears its resident-provider state, and closes the
terminal pane. The ticket reservation and failed thread remain for operator
reset; provenance is not emitted. `MAX_SAME_PANE_STARTUP_RELAUNCHES` is one, so
no second replacement launch is possible.

## Error signal

`check_error_signals` consumes typed `SignalRecord::Error` records. A recovering
seat is routed to assignment recovery failure. Otherwise the consumer resolves
the running thread by pane, fails it, emits `RunOutcome::Failed` provenance with
`fenced: false`, calls `release_slot_for_ticket`, removes the thread, records an
error alert, and logs automatic retry.

`release_slot_for_ticket` revokes the lease, clears ticket and attempt from the
slot, removes the seat assignment, and preserves a live resident session with a
cooldown unless the slot was already fenced. Thus ordinary error signals do not
close the pane. Unknown or idle-pane error records are consumed and logged with
no scheduler mutation.

## Session timeout

`check_session_timeouts` applies the configured global or phase budget only to
running, non-completing threads. Exceeding a budget is advisory while recent
activity exists. Reclaim requires silence for twice `stuck_threshold_secs`, and
`awaiting_human` suppresses the kill.

For a reclaimable thread, the function fails the thread, calls
`revoke_and_fence_attempt`, emits `RunOutcome::TimedOut` provenance with the
fence result, releases the slot, removes the thread, appends `timeout_alerts`,
and logs `SessionTimedOut`. The tested lifecycle order is lease revocation, pane
fence, then slot release. The high-water lease remains for monotonic redispatch.

## Stale-thread reclamation

`detect_stale_threads` selects running, non-completing, non-awaiting threads
whose health is `Stuck` at the same two-times-threshold silence boundary. It
fails the thread, revokes and fences the attempt, emits `RunOutcome::Failed`
provenance carrying the fence result, releases the slot, removes the thread, and
logs automatic retry. Unlike session timeout, it does not add a timeout alert.

## Boundaries and constraints

- Failure state, thread status, slot reservation, and lease authority are
  independent; no one field summarizes all teardown effects.
- Operator-retained terminal states intentionally prevent automatic retries.
- Automatic reclaim requires thread removal plus slot release.
- Only hard-silence paths fence panes; ordinary error signals preserve them.
- Provenance is emitted only at paths that remove the thread in this scope.
- Provenance must run before thread removal and is a no-op if no ledger is set.
- A failed provenance write does not block scheduler teardown.
- `lease_high_water` is never discarded by these paths.
- Pending completion and awaiting-human guards prevent inappropriate reclaim.
- Current tests are native fixtures; live-seat behavior is explicitly deferred
  by story S-039-03.

## Existing regression surfaces

The strongest delivery fixtures are
`test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership` and
`test_bounded_fresh_delivery_retries_once_then_fails_actionably`. Startup reset
and fencing are covered by `test_missing_shell_readiness_fences_without_relaunch`
and `missing_replacement_start_fences_without_second_relaunch`. Timeout ordering
is covered by `test_check_session_timeouts_expired`. Error consumption and
release are covered by `test_check_error_signals_fails_running_thread`. Stale
reclamation is covered by `test_detect_stale_threads` and the Codex composition
fixture `test_codex_heartbeat_honest_then_genuine_hang_reclaimed`.

Assignment recovery terminal behavior exists in code, but the current suite has
less direct field-by-field coverage than the other paths. That is an observed
test-boundary fact for the invariant matrix, not a proposal in this phase.
