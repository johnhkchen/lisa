# T-005-01 Structure: Scheduling Decision Logging

## File-Level Changes

### 1. `crates/lisa-core/src/types.rs` — Add two ActivityEvent variants

Add after `HealthStateChanged` (line ~562):

```rust
/// Informational log message (not an error or warning)
Info { message: String },

/// Poll cycle summary (filtered from UI to avoid noise)
PollSummary { ready: usize, running: usize, idle_slots: usize },
```

No other changes to this file. The variants follow the existing pattern (Error carries `message: String`).

### 2. `crates/lisa-plugin/src/lib.rs` — Scheduling logic changes

#### 2a. `discover_slots()` (line 215)
Replace `ActivityEvent::Error` with `ActivityEvent::Info`.

#### 2b. `release_slot_for_ticket()` (lines 231-239)
Add a `found` tracking variable. After the loop, log:
- Found: `ActivityEvent::Info { message: "Released slot #{pane_id} for {ticket_id}" }`
- Not found: `ActivityEvent::Info { message: "No slot found for {ticket_id}" }`

Must capture `pane_id` before clearing the slot.

#### 2c. `schedule_ready_tickets()` (lines 242-298)
1. Inside the `for ticket_id in ready` loop, after `self.threads.contains_key()` check: log skip Info.
2. After loop completes: if any ready tickets went unscheduled due to no slots, log exhausted Info. Track via a counter.

#### 2d. `poll_tick()` (lines 481-545)
After `schedule_ready_tickets()` call, before termination check, log PollSummary with counts:
- `ready`: `self.dag.get_ready_tickets().len()`
- `running`: `self.threads.values().filter(|t| t.status == Running).count()`
- `idle_slots`: `self.agent_slots.iter().filter(|s| s.ticket_id.is_none()).count()`

#### 2e. `activity_event_to_ui_entry()` (lines 932-1006)
Add match arms:
- `ActivityEvent::Info { message }` → `Some(ui::ActivityEntry { activity: ui::ActivityType::Info { ticket_id: String::new(), message } })`
- `ActivityEvent::PollSummary { .. }` → `None` (filtered from UI)

### 3. `crates/lisa-plugin/src/ui.rs` — UI rendering

#### 3a. `ActivityType` enum (line 176)
Add variant:
```rust
Info { ticket_id: String, message: String },
```

#### 3b. `render_activity_log()` (line 725)
Add rendering case for `ActivityType::Info`:
```rust
ActivityType::Info { ticket_id, message } => {
    let prefix = if ticket_id.is_empty() { String::new() } else { format!("{} ", ticket_id) };
    ("ℹ", CYAN, format!("{}{}", prefix, msg))
}
```

### 4. Tests — `crates/lisa-plugin/src/lib.rs` (tests module)

New tests:
1. `test_release_slot_logs_success` — Create State with agent_slot holding a ticket, call release_slot_for_ticket, assert Info log contains "Released slot".
2. `test_release_slot_logs_not_found` — Call release_slot_for_ticket for unknown ticket, assert Info log contains "No slot found".
3. `test_discover_slots_logs_info_not_error` — Cannot test directly (needs PaneManifest from zellij). Instead verify in existing test updates.
4. `test_poll_summary_event_filtered` — Assert activity_event_to_ui_entry returns None for PollSummary.
5. `test_info_event_to_ui_entry` — Assert Info maps to ui::ActivityType::Info.

Updated tests:
- `test_detect_stale_threads` — May need to account for new Info logs from release_slot_for_ticket (the existing assertion uses `any()` so should still pass).

## Module Boundaries

- `lisa-core/types.rs`: Only adds enum variants. No new imports or dependencies.
- `lisa-plugin/lib.rs`: Uses existing `ActivityEvent`, `log_activity()`, and state fields. No new dependencies.
- `lisa-plugin/ui.rs`: Uses existing color constants and rendering patterns. No new dependencies.

## Ordering

1. types.rs first (adds the variants other files depend on)
2. ui.rs second (adds the ActivityType::Info variant that lib.rs mapping references)
3. lib.rs last (uses both new types)

## Public Interface Changes

None. ActivityEvent is a pub enum, so adding variants is a breaking change in semver, but within this workspace it's fine — all consumers are in the same crate family.
