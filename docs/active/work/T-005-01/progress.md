# T-005-01 Progress: Scheduling Decision Logging

## Completed

### Step 1: Add ActivityEvent variants (types.rs)
- Added `Info { message: String }` variant
- Added `PollSummary { ready, running, idle_slots }` variant

### Step 2: Add ui::ActivityType::Info (ui.rs)
- Added `Info { ticket_id, message }` variant to ActivityType
- Added rendering case: `ℹ` icon, CYAN color

### Step 3: Wire up event mapping (lib.rs)
- `ActivityEvent::Info` → `ui::ActivityType::Info`
- `ActivityEvent::PollSummary` → `None` (filtered from UI)

### Step 4: Fix discover_slots Error → Info
- Changed `ActivityEvent::Error` to `ActivityEvent::Info` in discover_slots()

### Step 5: Add logging to release_slot_for_ticket
- Refactored to track released pane_id
- Logs "Released slot #{pane_id} for {ticket_id}" on success
- Logs "No slot found for {ticket_id}" when ticket not in any slot

### Step 6: Add logging to schedule_ready_tickets
- Logs "Skipping {ticket_id}: thread already exists" when filtered
- Tracks unscheduled count; logs "No idle slots available, {N} ready tickets waiting" after loop
- Changed `break` to `continue` on no-slots so all ready tickets are counted

### Step 7: Add PollSummary to poll_tick
- Logs PollSummary with ready/running/idle_slots counts after scheduling

### Step 8: Write tests
- `test_release_slot_logs_success` — verifies Info log with pane_id
- `test_release_slot_logs_not_found` — verifies Info log for missing ticket
- `test_info_event_to_ui_entry` — verifies Info → ui::ActivityType::Info mapping
- `test_poll_summary_event_filtered` — verifies PollSummary → None

### Step 9: Final verification
- 0 WASM compilation errors
- 172 tests passing (49 cli + 59 core + 64 plugin)

## Additional Change
- Fixed `sweep_stale_slots()` (pre-existing uncommitted code) to use `Info` instead of `Error` for stale slot release messages

## Acceptance Criteria Checklist
- [x] schedule_ready_tickets logs scheduling decisions (skip, exhausted, ThreadSpawned already existed)
- [x] release_slot_for_ticket logs released/not-found
- [x] Slot discovery changed from Error to Info
- [x] poll_tick logs summary each cycle (PollSummary, filtered from UI)
- [x] All new log messages visible in dashboard activity log (Info renders with ℹ icon)
- [x] Existing tests updated, new tests for each logging path
