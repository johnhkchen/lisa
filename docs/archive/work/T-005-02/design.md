# T-005-02 Design: Slot Status Dashboard

## Decision: Minimal Extension of Existing Patterns

### Approach

Follow the established ui.rs pattern: add a `SlotInfo` struct and a `slots`
field to `PluginState`, a `render_slots()` function, update the status line,
and wire it through `to_ui_state()`.

### Alternatives Considered

**A. Derive slot info from active/parked threads (no new struct)**
- Pro: No new types needed — count occupied from active_threads + parked_threads
- Con: Doesn't capture idle slots or total slot count. Can't distinguish "2
  slots, 1 idle" from "5 slots, 4 idle". Loses the `has_session` info. Doesn't
  match the AC which asks for `pane_id + ticket_id` per occupied slot.
- Rejected: Insufficient information.

**B. Full SlotInfo struct with all fields (chosen)**
- New `ui::SlotInfo` with `pane_id`, `ticket_id: Option<String>`, `has_session: bool`
- Add `Vec<SlotInfo>` to `PluginState`
- Pro: Full visibility. Matches AC exactly. All derived counts computable.
- Con: Adds one struct and one field.
- Chosen: Straightforward, complete, matches AC.

**C. Summary-only (total, idle, occupied counts)**
- Just add `slot_total: usize, slot_idle: usize` to PluginState
- Pro: Simpler
- Con: Can't show "which ticket in which slot" as AC requires. Would need
  another struct anyway for the occupied list.
- Rejected: Doesn't satisfy AC.

### Detailed Design

#### New Type: `ui::SlotInfo`

```rust
pub struct SlotInfo {
    pub pane_id: u32,
    pub ticket_id: Option<String>,
    pub has_session: bool,
}
```

#### PluginState Changes

Add one field:
```rust
pub struct PluginState {
    // ... existing fields ...
    pub slots: Vec<SlotInfo>,
}
```

Update `Default` impl to include `slots: Vec::new()`.

#### `to_ui_state()` Changes

Map `self.agent_slots` → `Vec<ui::SlotInfo>`:
```rust
let slots: Vec<ui::SlotInfo> = self.agent_slots.iter().map(|s| ui::SlotInfo {
    pane_id: s.pane_id,
    ticket_id: s.ticket_id.clone(),
    has_session: s.has_session,
}).collect();
```

#### Status Line Update

Change from:
```
Active: 1 | Parked: 0 | Done: 1/3 | [d] mark done
```
To:
```
Slots: 1/2 | Active: 1 | Parked: 0 | Done: 1/3 | [d] mark done
```

Where "1/2" = occupied/total. Computed from `state.slots`.

#### Slots Section

A compact one-line-per-occupied-slot section, rendered between the status line
separator and the attention banner:

```
=== Slots: 2 total, 1 idle, 1 occupied ===

  #5  T-003-01  [IMP]

⚠ 2 tickets waiting for slots
```

When all slots are idle:
```
=== Slots: 2 total, 2 idle ===
```

When no slots discovered yet:
```
(no agent slots)
```

#### Warning Logic

Show yellow warning when:
1. `idle_count == 0` (all slots occupied)
2. There exist ready tickets not already in `active_threads`

The ready-but-waiting count: iterate `state.tickets` where status is Ready,
and not present in `active_threads`. This is a UI-side computation from
existing data — no new data transfer needed.

#### Dashboard Section Ordering

Updated `render_dashboard_lines()`:
1. Title bar + status line (with slot prefix)
2. Separator
3. **Slots section (new)**
4. Attention banner
5. DAG
6. Separator
7. Active threads
8. Parked threads
9. Separator
10. Activity log
11. Separator
12. Quick jump

#### Tests

For slot rendering, test:
1. All idle (2 slots, no tickets) — shows "2 total, 2 idle"
2. All occupied (2 slots, 2 tickets) — shows occupied list + warning
3. Mixed (3 slots, 1 occupied, 2 idle) — shows one occupied entry
4. No slots — shows "(no agent slots)"
5. Warning: all occupied + ready tickets waiting
6. Status line includes "Slots: N/M"
7. Full dashboard includes slots section before DAG

### Risks

- **Terminal width**: Slot section lines could get long with many slots. The
  ticket mentions 2-slot default (`max_threads = 2`), so this is unlikely to
  be an issue in practice. Truncation at 80 chars is already in place.
- **Test fixtures**: Need to add `slots` to all existing test fixtures. Using
  `..Default::default()` or `PluginState { slots: vec![], .. }` keeps this
  non-invasive.
