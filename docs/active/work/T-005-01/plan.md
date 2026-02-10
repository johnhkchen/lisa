# T-005-01 Plan: Scheduling Decision Logging

## Step 1: Add ActivityEvent variants to types.rs

Add `Info { message: String }` and `PollSummary { ready: usize, running: usize, idle_slots: usize }` to the `ActivityEvent` enum in `crates/lisa-core/src/types.rs`.

**Verify:** `cargo check -p lisa-core`

## Step 2: Add ui::ActivityType::Info variant to ui.rs

Add `Info { ticket_id: String, message: String }` to the `ActivityType` enum in `crates/lisa-plugin/src/ui.rs`.

Add rendering case in `render_activity_log()` for the new Info variant: icon `ℹ`, color CYAN.

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1`

## Step 3: Wire up activity_event_to_ui_entry mapping in lib.rs

Add match arms in `activity_event_to_ui_entry()`:
- `ActivityEvent::Info` → `Some(ui::ActivityType::Info { .. })`
- `ActivityEvent::PollSummary` → `None`

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1`

## Step 4: Fix discover_slots Error → Info

In `discover_slots()`, change `ActivityEvent::Error` to `ActivityEvent::Info`.

**Verify:** `cargo test --workspace` (existing tests still pass)

## Step 5: Add logging to release_slot_for_ticket

Refactor `release_slot_for_ticket()` to track whether a slot was found. Log:
- Found: `Info { message: "Released slot #{pane_id} for {ticket_id}" }`
- Not found: `Info { message: "No slot found for {ticket_id}" }`

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1`

## Step 6: Add logging to schedule_ready_tickets

1. After `self.threads.contains_key()` check: log `Info { message: "Skipping {ticket_id}: thread already exists" }`
2. After the loop: track unscheduled count. If > 0, log `Info { message: "No idle slots available, {N} ready tickets waiting" }`

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1`

## Step 7: Add PollSummary to poll_tick

After `schedule_ready_tickets()`, compute counts and log `PollSummary { ready, running, idle_slots }`.

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1`

## Step 8: Write tests

New tests in `crates/lisa-plugin/src/lib.rs` tests module:
1. `test_release_slot_logs_success` — slot found, Info logged with pane_id
2. `test_release_slot_logs_not_found` — ticket not in slots, Info logged
3. `test_info_event_to_ui_entry` — Info → ui::ActivityType::Info
4. `test_poll_summary_event_filtered` — PollSummary → None

**Verify:** `cargo test --workspace` — all tests pass

## Step 9: Final verification

Run full check: `cargo check -p lisa-plugin --target wasm32-wasip1 && cargo test --workspace`

Confirm all acceptance criteria are met by reviewing the code changes against the ticket.

## Testing Strategy

- Unit tests for each new log path (steps 5-7)
- Mapping tests for new event types (step 8)
- `schedule_ready_tickets` and `poll_tick` can't be fully tested in native because they call zellij host functions — test the logging paths by constructing state and calling the sub-functions that can be tested
- Existing tests should continue to pass unchanged (the only behavioral change is Error → Info in discover_slots, which existing tests don't assert on)
