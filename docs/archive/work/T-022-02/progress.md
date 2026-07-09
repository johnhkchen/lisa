# T-022-02 Progress — Error Signal Consumer

## Status: Implementation complete, all tests green

## Steps completed

- [x] **Step 1 — `error_alerts` field** (`lib.rs`): added `error_alerts: Vec<(TicketId, u32)>`
  after `timeout_alerts`, with a doc comment mirroring it.
- [x] **Step 2 — `check_error_signals` method** (`lib.rs`): read-and-delete loop over
  `pane-<id>.error`; resolves the running thread via `threads` (not `agent_slots`);
  fail + `release_slot_for_ticket` + remove + `error_alerts.push` + `Error` log; else
  `Info` log no-op. Placed next to `check_transition_signals`.
- [x] **Step 3 — poll_tick wiring** (`lib.rs`): `self.check_error_signals()` inserted
  between `check_transition_signals()` and `check_transition_timeouts()`, so an errored
  pane is failed before the force-advance fallback runs.
- [x] **Step 4 — clear on reschedule** (`lib.rs`): `error_alerts.retain(...)` beside the
  `timeout_alerts` retain in the spawn path.
- [x] **Step 5 — UI surfacing** (`lib.rs` `to_ui_state`): drain `error_alerts` into
  `HealthAlert { alert_type: Failed, detail: "Session reported an error (pane N)",
  actions: [Check pane output, Retry] }`.
- [x] **Step 6 — hooks-guide doc** (`data/hooks-guide.md`): `.error` row added to the
  signal table + a paragraph describing it as adapter-emitted core, the immediate
  fail+release behaviour, presence-is-signal, and idle-pane harmlessness.
- [x] **Step 7 — tests** (`lib.rs`): three tests added —
  `test_check_error_signals_fails_running_thread`,
  `test_check_error_signals_idle_pane_noop`,
  `test_to_ui_state_includes_error_alerts`. File-deletion asserted in the first two.
- [x] **Step 8 — verification**:
  - `cargo test --workspace` → **187 passed, 0 failed**.
  - `cargo build -p lisa-plugin --target wasm32-wasip1 --release` → **ok**.
  - `cargo clippy -p lisa-plugin` → no new warnings on touched code.

## Deviations from plan

None. The plan's step order and design decisions were followed exactly. No new
`ActivityEvent` variant and no `AlertType` variant were needed (reused
`ActivityEvent::Error` and `AlertType::Failed`, as designed).

## Files changed

- `crates/lisa-plugin/src/lib.rs` — field, method, poll wiring, reschedule-clear, UI
  alert, 3 tests.
- `crates/lisa-cli/data/hooks-guide.md` — signal-contract documentation.

## Notes

Landed as a single cohesive change (one feature). Not committed — leaving the commit to
the operator's convention; the tree is clean and green.
