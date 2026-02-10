# T-005-02 Plan: Slot Status Dashboard

## Step 1: Add SlotInfo struct and PluginState field (ui.rs)

- Add `SlotInfo` struct after `HealthAlert`
- Add `pub slots: Vec<SlotInfo>` to `PluginState`
- Add `slots: Vec::new()` to `Default for PluginState`
- Verify: `cargo check -p lisa-plugin --target wasm32-wasip1`

## Step 2: Implement render_slots() (ui.rs)

- New function `render_slots(state: &PluginState, output: &mut Vec<String>)`
- Compute total/idle/occupied from `state.slots`
- Header: "=== Slots: N total, M idle[, K occupied] ==="
- Per-occupied-slot line with pane_id, ticket_id, phase shortname
- Warning when idle==0 and ready tickets waiting (yellow)
- Empty case: "(no agent slots)"
- Verify: cargo check

## Step 3: Update render_status_line() (ui.rs)

- Add "Slots: K/N | " prefix to status line
- K = occupied count, N = total from `state.slots`
- Verify: cargo check

## Step 4: Update render_dashboard_lines() ordering (ui.rs)

- Insert `render_slots(state, &mut output)` + blank line
  after initial separator, before attention banner
- Verify: cargo check

## Step 5: Wire to_ui_state() in lib.rs

- Map `self.agent_slots` → `Vec<ui::SlotInfo>`
- Add `slots` field to returned `ui::PluginState`
- Verify: cargo check

## Step 6: Fix existing tests

- Any existing test that constructs `PluginState` directly needs
  `slots: vec![]` added (or rely on `..PluginState::default()`)
- Run `cargo test --workspace` — fix all failures

## Step 7: Add slot-specific tests (ui.rs)

Tests:
1. `test_render_slots_all_idle` — 2 idle slots → "2 total, 2 idle"
2. `test_render_slots_all_occupied` — 2 occupied → shows ticket list
3. `test_render_slots_mixed` — 1 idle, 1 occupied → both shown
4. `test_render_slots_no_slots` — empty → "(no agent slots)"
5. `test_render_slots_warning_tickets_waiting` — all occupied + ready waiting → yellow warning
6. `test_status_line_with_slots` — status line contains "Slots: N/M"
7. `test_slots_in_full_dashboard` — full render shows slots section before DAG

Run: `cargo test --workspace`

## Verification

- `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM compiles
- `cargo test --workspace` — all tests pass
- Manual review of render output via test assertions
