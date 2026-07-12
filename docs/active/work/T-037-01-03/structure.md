# Structure — T-037-01-03 delayed-send-and-prompt-miss-regression

## Files touched

| File | Change | Ownership |
|---|---|---|
| `crates/lisa-plugin/src/lib.rs` | **Modified** — add two `#[test]` fns in the existing `#[cfg(test)] mod tests` block | ticket-owned |

No other files. No new modules, no production symbols, no adapter/UI changes.

## Insertion point

After `session_start_seat_never_paces_on_grace_and_still_requires_the_signal`
(ends ~lib.rs:9839), before `dispatch_mints_and_stamps_strictly_new_attempt_lease`.
Keeps all grace-bootstrap tests contiguous.

## Symbols reused (no new public/internal API)

- `pane_name_schedule_state("codex", AgentClient::Codex, None)` → one-slot
  grace-mode State on pane 10, ticket `T-NAME`.
- `State::schedule_ready_tickets()` → mints lease, launches, stages
  `assignment.md`, leaves `Starting`.
- `State::seat_assignment(pane) -> Option<SeatAssignmentState>`.
- `State::seat_readiness_mode(pane) -> Option<ReadinessMode>`.
- `State::seat_is_owned(pane) -> bool`.
- `State::check_assignment_ack_timeouts_at(now)` — injected clock.
- `acknowledge_assignment(state, pane, ticket_id, generation) -> bool` test
  helper (exact + stale + wrong-ticket drives).
- `State::to_ui_state().seat_assignment_statuses.get(&1)` — named UI status.
- `state.current_leases["T-NAME"].clone()` — the current `AttemptLease`.
- `state.threads["T-NAME"].status` vs `lisa_core::types::ThreadStatus::Failed`.
- `state.activity_log` filtered for `ActivityEvent::Info { message }` containing
  `"delivering assignment for T-NAME"` — delivery-count assertions.

## Test 1 — `codex_delayed_send_reaches_owned_only_on_current_attempt_ack`

Shape (blueprint, not code):

```
let (mut codex, _dir) = pane_name_schedule_state("codex", Codex, None)
codex.schedule_ready_tickets()
assert seat_readiness_mode(10) == Some(Grace)
let lease = current_leases["T-NAME"].clone()
let grace_deadline = match seat_assignment(10) {
    Starting { generation == lease, start_deadline: Some(d), relaunches: 0 } => d
    other => panic
}

// delayed: a poll strictly before the deadline delivers nothing
let before = grace_deadline - Duration::from_secs(1)
codex.check_assignment_ack_timeouts_at(before)
assert seat_assignment(10) is Starting            // still paced, not sent
assert ui status == Starting                       // never ReadyForAssignment
assert delivery_log_count == 0
assert !seat_is_owned(10)

// grace elapses → Delivering directly (no ReadyForAssignment node)
codex.check_assignment_ack_timeouts_at(grace_deadline)
assert seat_assignment(10) == Delivering { generation == lease, retries: 0 }
assert ui status == Delivering
assert !seat_is_owned(10)                           // elapsed time never owns

// Owned only on the exact current-attempt UserPromptSubmit
assert !acknowledge_assignment(codex, 10, "T-NAME", lease.attempt_id + 1)  // stale gen
assert !acknowledge_assignment(codex, 10, "T-OTHER", lease.attempt_id)     // wrong ticket
assert !seat_is_owned(10)
assert  acknowledge_assignment(codex, 10, "T-NAME", lease.attempt_id)      // exact
assert seat_assignment(10) == Owned
```

Distinct from the existing happy-path test by: the pre-deadline quiescence
assertions (delayed send) and the wrong-ticket rejection.

## Test 2 — `codex_prompt_miss_retries_then_recycles_to_delivery_failed_never_owned`

Shape:

```
let (mut codex, _dir) = pane_name_schedule_state("codex", Codex, None)
codex.config.assignment_ack_timeout_secs = 1     // tighten the Delivering clock
codex.schedule_ready_tickets()
assert seat_readiness_mode(10) == Some(Grace)
let lease = current_leases["T-NAME"].clone()
let grace_deadline = <Starting.start_deadline>

// grace elapses → Delivering{0}, no matching ack ever sent
codex.check_assignment_ack_timeouts_at(grace_deadline)
let d0 = match seat_assignment(10) { Delivering { generation==lease, ack_deadline, retries:0 } => ack_deadline }
assert !seat_is_owned(10)

// bounded retry
codex.check_assignment_ack_timeouts_at(d0)
let d1 = match seat_assignment(10) { Delivering { retries:1, ack_deadline, .. } => ack_deadline }
assert delivery_log_count == 2                    // initial + exactly one retry
assert !seat_is_owned(10)

// stale-attempt signal rejected mid-miss
assert !acknowledge_assignment(codex, 10, "T-NAME", lease.attempt_id + 1)
assert !seat_is_owned(10)

// named recycle → DeliveryFailed
codex.check_assignment_ack_timeouts_at(d1)
assert seat_assignment(10) == DeliveryFailed
assert ui status == DeliveryFailed
assert threads["T-NAME"].status == Failed
assert agent_slots[0].attempt_lease == Some(lease)   // reservation retained
assert current_leases.get("T-NAME") == Some(lease)   // for operator reset
assert !seat_is_owned(10)

// terminal: even an exact-generation ack after failure cannot own
assert !acknowledge_assignment(codex, 10, "T-NAME", lease.attempt_id)
assert seat_assignment(10) == DeliveryFailed
```

Distinct from `test_missing_fresh_chat_ack_...` by entering through the **grace
pace** (Codex, elapsed `Starting`→`Delivering`) rather than the Claude
`.started`-signal + `deliver_ready_assignments` seam.

## Ordering of changes

Single atomic edit to lib.rs (both tests together), one `lisa commit-ticket`.
No dependency ordering among the two tests; they share no state.

## Verification surface

`cargo test -p lisa-plugin` runs both new tests plus the full existing suite
(grace happy path, Claude SessionStart, E-035 no-inline-prompt / dquote
recovery, E-034 fencing). Green across all = acceptance met. A WASM
`cargo check --target wasm32-wasip1` guards the plugin still compiles for its
real target (tests are native-only but the module lives in the plugin crate).

## Helper-extraction decision

Both tests open with "schedule → read grace_deadline → elapse to Delivering{0}".
Per Design option D: inline first. If the duplicated block harms readability at
Implement time, extract a private `fn grace_elapse_to_delivering(state, pane,
lease) -> ack_deadline` local to the test module. Default: inline, ~6 lines
each, locally readable.
