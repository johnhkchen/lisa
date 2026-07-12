# Research — T-037-01-03 delayed-send-and-prompt-miss-regression

## Ticket in one line

Add two deterministic, injected-time native tests that pin the Codex grace
bootstrap behaviour T-037-01-02 landed: (1) a *delayed-send* path that reaches
`Owned` only on the exact current-attempt `UserPromptSubmit`, and (2) a
*prompt-miss* path that resolves in a bounded named state without ever reaching
`Owned`. Existing Claude SessionStart, E-035, and E-034 tests must stay green.

This is a **test-only ticket**. No production code changes are expected: the
grace transition, the retry→`DeliveryFailed` resolution, and the
acknowledgement gate all already exist. The work is to prove them.

## The state machine (crates/lisa-plugin/src/lib.rs)

`SeatAssignmentState` (lib.rs:291-347) is the scheduler-owned truth for a
physical seat's assignment lifecycle, deliberately independent of
`TransitionState`. Relevant variants:

- `Starting { generation, start_deadline: Option<SystemTime>, relaunches }` —
  a fresh provider was launched; its exact process-start signal is not yet
  observed. `start_deadline` is `None` until the launcher is submitted.
- `ReadyForAssignment { generation }` — process-start proven (Claude path).
- `Delivering { generation, ack_deadline, retries }` — the tagged chat
  reference was submitted; awaiting the exact `UserPromptSubmit`.
- `Owned` — provider positively accepted the current attempt.
- `DeliveryFailed` — bounded chat delivery exhausted; terminal, retained for
  operator reset. Surfaces as `SeatAssignmentStatus::DeliveryFailed` (RED).

## Provider readiness mode (T-037-01-01)

`seat_readiness_mode(pane_id) -> Option<ReadinessMode>` (lib.rs:1406) records,
per pane, what was classified at launch dispatch: `ReadinessMode::Grace`
(Codex) or `ReadinessMode::SessionStart` (Claude). It is disjoint from the
`SeatAssignmentState` machine.

## Where grace behaviour lives

1. **Arming the grace deadline** — `start_assignment_ack_wait` (lib.rs:1574).
   On the first submission of a `Starting { start_deadline: None }` seat, if the
   readiness mode is `Grace` it sets `start_deadline = startup_grace_deadline(now)`
   (`now + STARTUP_GRACE_SECS`, lib.rs:1455-1458, `STARTUP_GRACE_SECS = 8`).
   Otherwise (SessionStart) it uses the acceptance clock deadline.

2. **Elapsing the grace** — `check_assignment_ack_timeouts_at(now)`
   (lib.rs:2176-2278). A `Starting { relaunches: 0, start_deadline: Some(d) }`
   whose `d <= now`:
   - **Grace mode** → `deliver_assignment_to_pane(pane, generation, 0, now)`;
     on error → `fail_assignment_delivery`. This is the direct
     `Starting → Delivering` pace. It never passes through `ReadyForAssignment`.
   - **SessionStart mode** → `begin_startup_recovery` (unchanged Claude path).

3. **Delivering retry / failure** — same loop:
   - `Delivering { retries }` with `retries < MAX_ASSIGNMENT_DELIVERY_RETRIES`
     (`=1`, lib.rs:175) → re-`deliver_assignment_to_pane(..., retries+1, ...)`.
   - `Delivering { .. }` otherwise → `fail_assignment_delivery(pane, "provider
     did not acknowledge the bounded chat assignment")` → `DeliveryFailed`.

4. **Ownership gate** — `acknowledge_codex_assignment(pane, payload_json)`
   (lib.rs:1627). Returns false if already owned, if
   `active_assignment_generation(pane)` is `None` (only `Delivering` /
   `AssignedPendingAck` / `Recovering` yield a generation — `DeliveryFailed`
   yields `None`, so a late ack cannot resurrect it), if the slot lease is not
   current, or if `codex_ack::detect_codex_ack` rejects the payload
   (wrong ticket/generation). On success → `Owned`.

## Delivery mechanics

`deliver_assignment_to_pane` (lib.rs:1462) requires: pane not awaiting human
input (`is_pane_awaiting`, false by default — `awaiting_human` empty), a slot
with a current lease matching `(ticket_id, generation)`, and the assignment
file `<attempt_work_dir>/assignment.md` present on disk. `schedule_ready_tickets`
stages that file, so it exists after scheduling. On success it sets
`Delivering { ack_deadline: assignment_ack_deadline(now), retries }` and logs an
`Info` "delivering assignment for <ticket>".

## Existing test harness (the module at lib.rs:9481+)

- `pane_name_schedule_state(requested_agent, default_agent, resident_agent)
  -> (State, TempDir)` (lib.rs:9495): builds a one-slot (pane 10) State with a
  `T-NAME` ticket. `schedule_ready_tickets()` then mints lease attempt 1,
  launches, stages `assignment.md`, and leaves the seat `Starting`.
- `acknowledge_assignment(state, pane_id, ticket_id, generation) -> bool`
  (lib.rs:9600): builds a tagged `UserPromptSubmit` payload and calls
  `acknowledge_codex_assignment`. Used to drive the ownership gate.
- Injected time: `check_assignment_ack_timeouts_at(now)` takes the clock as an
  argument. Deadlines are read out of the matched state and fed straight back
  in — no sleeps, no wall-clock dependence.
- `to_ui_state().seat_assignment_statuses.get(&1)` maps slot index 0 → key `1`
  (lib.rs:5737), giving the operator-visible named status.

## Directly adjacent existing tests

- `codex_startup_grace_paces_first_prompt_into_delivering` (lib.rs:9722): the
  T-037-01-02 happy path — grace elapse → `Delivering` → `Owned` on ack, with a
  stale-generation payload rejected. Does **not** assert the pre-deadline
  "delayed" quiescence, and stops at a single stale-ack rejection.
- `session_start_seat_never_paces_on_grace_and_still_requires_the_signal`
  (lib.rs:9786): Claude never paces; deadline → `ResettingStartup`.
- `test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership`
  (lib.rs:10337): retry→`DeliveryFailed`, but entered via the **Claude /
  SessionStart** path (`.started` signal → `deliver_ready_assignments`), not the
  grace pace. The grace-mode prompt-miss is not yet covered.

## Constraints / assumptions

- Native `cargo test --workspace` target (not wasm). Tests use `#[cfg(test)]`
  helpers; `close_fenced_pane` and host calls are no-ops under `cfg(test)`.
- Determinism: never call the argless `check_assignment_ack_timeouts()`; always
  the `_at` variant with a deadline captured from state.
- Scope (N4 / story boundary): touch only the lib.rs test module. Do not alter
  the Starting machine, adapter.rs, E-034 fencing, or E-035's split. No new
  production symbols.
- `MAX_ASSIGNMENT_DELIVERY_RETRIES = 1` ⇒ exactly one retry before
  `DeliveryFailed`: grace→`Delivering{0}`, deadline→`Delivering{1}`,
  deadline→`DeliveryFailed`.
