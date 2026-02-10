# T-005-02 Progress: Slot Status Dashboard

## Completed

### Step 1: Add SlotInfo struct and PluginState field
- Added `ui::SlotInfo` struct with `pane_id`, `ticket_id`, `has_session`
- Added `slots: Vec<SlotInfo>` field to `ui::PluginState`
- Updated `Default` impl

### Step 2: Implement render_slots()
- Compact section showing total/idle/occupied counts
- Per-occupied-slot lines with pane_id, ticket_id, phase shortname
- Warning when all slots occupied and ready tickets waiting

### Step 3: Update render_status_line()
- Prepends "Slots: K/N | " when slots are present

### Step 4: Update render_dashboard_lines()
- Slots section inserted after title/separator, before attention banner

### Step 5: Wire to_ui_state() in lib.rs
- Maps `agent_slots` → `Vec<ui::SlotInfo>` in `to_ui_state()`

### Step 6: Fix existing tests
- Added `slots: Vec::new()` to `sample_state()` and diamond DAG test
- Other tests use `..PluginState::default()` which picks it up automatically

### Step 7: Add slot-specific tests (7 new tests)
- `test_render_slots_all_idle` — 2 idle → "2 total, 2 idle"
- `test_render_slots_all_occupied` — 2 occupied → shows ticket list + phases
- `test_render_slots_mixed` — 1 occupied, 2 idle → both shown
- `test_render_slots_no_slots` — empty → "(no agent slots)"
- `test_render_slots_warning_tickets_waiting` — all occupied + ready → warning
- `test_status_line_with_slots` — "Slots: 1/2" in status line
- `test_slots_in_full_dashboard` — slots before DAG in full render

## Verification
- `cargo check -p lisa-plugin --target wasm32-wasip1` — passes
- `cargo test --workspace` — 189 tests pass (was 182, +7 new)
- No new warnings introduced (existing warnings are pre-existing)
