# T-005-02 Structure: Slot Status Dashboard

## Files Modified

### 1. `crates/lisa-plugin/src/ui.rs`

**New type** (after `HealthAlert`, around line 173):
```rust
/// Information about an agent pane slot for dashboard display.
#[derive(Debug, Clone)]
pub struct SlotInfo {
    pub pane_id: u32,
    pub ticket_id: Option<String>,
    pub has_session: bool,
}
```

**Modified struct** — `PluginState` (line 207):
Add field:
```rust
pub slots: Vec<SlotInfo>,
```

**Modified impl** — `Default for PluginState` (line 218):
Add `slots: Vec::new()` to the default.

**New function** — `render_slots()`:
- Signature: `fn render_slots(state: &PluginState, output: &mut Vec<String>)`
- Computes: total, idle, occupied from `state.slots`
- Renders header: "=== Slots: N total, M idle, K occupied ==="
- For each occupied slot: "  #PANE  TICKET_ID  [PHASE]"
  (phase looked up from `state.active_threads` by ticket_id)
- Warning line when idle==0 and ready tickets waiting:
  "⚠ N tickets waiting for slots" in yellow
- Empty case: "(no agent slots)"

**Modified function** — `render_status_line()` (line 847):
- Prepend "Slots: K/N | " before "Active: ..."
  where K = occupied count, N = total slots

**Modified function** — `render_dashboard_lines()` (line 875):
- Insert `render_slots(state, &mut output)` + blank line + separator
  after the initial separator, before `render_attention_banner()`.

**New tests** (in `mod tests`):
- `test_render_slots_all_idle`
- `test_render_slots_all_occupied`
- `test_render_slots_mixed`
- `test_render_slots_no_slots`
- `test_render_slots_warning_tickets_waiting`
- `test_status_line_with_slots`
- `test_slots_in_full_dashboard`

### 2. `crates/lisa-plugin/src/lib.rs`

**Modified function** — `to_ui_state()` (line 924):
Add slot mapping after alerts:
```rust
let slots: Vec<ui::SlotInfo> = self.agent_slots.iter().map(|s| ui::SlotInfo {
    pane_id: s.pane_id,
    ticket_id: s.ticket_id.clone(),
    has_session: s.has_session,
}).collect();
```
Add `slots` to the returned `ui::PluginState`.

No other changes to lib.rs.

## Module Boundaries

- `ui.rs` owns all rendering logic and UI types — `SlotInfo` lives here
- `lib.rs` owns the conversion from internal `AgentSlot` → `ui::SlotInfo`
- No changes to `lisa-core` — slot info is a plugin-level concept

## Ordering

1. Add `SlotInfo` struct to ui.rs
2. Add `slots` field to `PluginState` + Default
3. Implement `render_slots()`
4. Update `render_status_line()`
5. Update `render_dashboard_lines()` ordering
6. Update `to_ui_state()` in lib.rs
7. Add tests
8. Fix any existing tests broken by new field (add `slots: vec![]`)

## Interface Contract

`render_slots()` is self-contained — it reads from `PluginState.slots` and
`PluginState.tickets`/`active_threads` for cross-referencing. No new parameters
beyond what's already in `PluginState`.

The warning computation (ready tickets waiting) is done inside `render_slots()`
using data already present in `PluginState`:
- Ready tickets: `state.tickets` where `status == Ready`
- Active threads: `state.active_threads` ticket_ids
- Waiting = ready tickets not in active threads
