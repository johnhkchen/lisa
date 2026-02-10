# T-005-02 Research: Slot Status Dashboard

## Overview

This ticket adds slot-level visibility to the dashboard. Currently the UI shows
active threads and parked threads, but operators cannot see the state of the
underlying agent pane slots — whether they're idle, occupied, or how many exist.

## Key Components

### 1. AgentSlot (lib.rs:49-56)

The internal slot representation:
```rust
struct AgentSlot {
    pane_id: u32,
    ticket_id: Option<TicketId>,  // None = idle
    has_session: bool,            // true after first Claude session launched
}
```

Slots are stored as `agent_slots: Vec<AgentSlot>` on `State` (lib.rs:86).
They are populated in `discover_slots()` (lib.rs:208-231) from PaneUpdate
events — every non-plugin pane becomes a slot.

### 2. Slot Lifecycle

- **Discovery**: `discover_slots()` populates `agent_slots` from PaneManifest.
  Runs once (`slots_discovered` flag). Logs `Info` event with count.
- **Assignment**: `schedule_ready_tickets()` (lib.rs:264-352) finds idle slots
  via `find_idle_slot()`, assigns ticket_id, sets has_session=true.
- **Release**: `release_slot_for_ticket()` (lib.rs:243-261) clears ticket_id
  but keeps has_session=true for session reuse.
- **Sweep**: `sweep_stale_slots()` (lib.rs:359-387) releases slots still
  assigned to done tickets.

### 3. Current `to_ui_state()` (lib.rs:924-1033)

Converts `State` → `ui::PluginState`. Currently maps:
- `dag.tickets()` → `Vec<ui::TicketNode>`
- Running threads → `Vec<ui::ActiveThread>`
- Parked threads → `Vec<ui::ParkedThread>`
- Activity log → `Vec<ui::ActivityEntry>` (filtering internal events)
- Health alerts from stuck/failed threads

**No slot information is currently passed to the UI.**

### 4. Current `ui::PluginState` (ui.rs:206-216)

```rust
pub struct PluginState {
    pub tickets: Vec<TicketNode>,
    pub active_threads: Vec<ActiveThread>,
    pub parked_threads: Vec<ParkedThread>,
    pub activity_log: Vec<ActivityEntry>,
    pub alerts: Vec<HealthAlert>,
    pub current_time: Duration,
    pub selected_ticket: Option<String>,
    pub modal: ModalState,
}
```

### 5. Current Dashboard Layout (ui.rs:875-919)

`render_dashboard_lines()` renders sections in order:
1. Title bar with `render_status_line()` — "Active: N | Parked: N | Done: N/M"
2. Separator
3. Attention banner (reviews + health alerts)
4. DAG section
5. Separator
6. Active threads table
7. Parked threads table
8. Separator
9. Activity log
10. Separator
11. Quick jump section

### 6. Status Line (ui.rs:847-868)

```rust
fn render_status_line(state: &PluginState) -> String {
    // "Active: {} | Parked: {} | Done: {}/{}  [d] mark done"
    // Plus alert count if any
}
```

### 7. Existing UI Types Pattern

ui.rs defines its own types separate from types.rs (architectural note from
memory). Types like `TicketNode`, `ActiveThread`, `ParkedThread`, `HealthAlert`
are all standalone structs in ui.rs. This is intentional — UI types are
rendering-focused, not data-model types.

### 8. T-005-01 Dependency (Done)

T-005-01 added scheduling decision logging — `Info` events for scheduling,
skipping, slot exhaustion, release, and `PollSummary`. These are already
implemented and working. The `PollSummary` event tracks ready/running/idle_slots
counts per poll cycle but is filtered from UI display (`return None`).

## Data Available for Slots

From `State.agent_slots`, per slot:
- `pane_id: u32`
- `ticket_id: Option<TicketId>` — None means idle
- `has_session: bool` — whether Claude Code has been launched

Derived:
- Total slots: `agent_slots.len()`
- Idle count: slots where `ticket_id.is_none()`
- Occupied count: slots where `ticket_id.is_some()`
- Occupied list: `(ticket_id, pane_id)` pairs

From DAG context:
- Ready tickets waiting: `dag.get_ready_tickets()` count minus running threads

## Dashboard Placement Considerations

The ticket specifies slots section "between status line and DAG". Currently:
- Line 1: Title bar + status line (same line, `render_status_line` is inlined)
- Line 2: Separator
- Line 3: Blank
- Then: Attention banner (if any) → DAG

The status line is part of the title bar, not a separate section. The "Slots"
section would go after the separator, before the attention banner.

Alternatively, the ticket says to update the status line itself to include
slot count: "Slots: 1/2 | Active: 1 | ...". This means both:
1. Status line gets slot count prefix
2. A separate compact "Slots" section shows details when occupied

## Warning Condition

"When all slots are occupied and ready tickets are waiting, show a yellow
warning: 'N tickets waiting for slots'"

This needs both:
- `idle_count == 0` (all occupied)
- Ready tickets exist that aren't already assigned threads

The ready-but-waiting count is already computed in `schedule_ready_tickets()`
as `unscheduled`. For the UI, we'd compute it from: ready tickets in DAG minus
tickets that have active threads.

## Test Patterns

Existing ui.rs tests (1016-1592) use `PluginState` directly — constructing
states with specific configurations and calling render functions. Test assertions
check for string content in output lines. This pattern extends naturally to
slot testing.

## Summary of Changes Needed

1. **ui.rs**: New `SlotInfo` struct, add `slots` field to `PluginState`, new
   `render_slots()` function, update `render_status_line()`, add warning logic,
   update `render_dashboard_lines()` section ordering, add tests.

2. **lib.rs**: Update `to_ui_state()` to populate slot info from `agent_slots`,
   compute waiting-for-slots count.
