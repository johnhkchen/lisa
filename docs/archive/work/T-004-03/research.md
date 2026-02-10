# T-004-03 Research: Error & Health Alerts

## Scope

Surface failed and stuck sessions in the dashboard so the user knows something is wrong, with suggested actions.

## Existing Health Infrastructure

### HealthStatus Model (types.rs:270-278)
T-004-01 introduced `HealthStatus` enum: `Healthy | Stuck | Failed`. Computed by `Thread::health()` (types.rs:370-389) which checks:
- Failed → if `thread.status == ThreadStatus::Failed`
- Stuck → if Running and `time_since(last_phase_change) >= stuck_threshold`
- Healthy → otherwise (including Parked and Completed)

`Thread::is_attention_needed()` (types.rs:394-403) returns true for Stuck, Failed, or Parked threads. This method exists but is **never called** anywhere in the plugin.

### Thread Lifecycle (types.rs:280-404)
- `Thread::mark_exited(exit_code)`: Some(0) → Completed, anything else → Failed
- `Thread::last_phase_change`: SystemTime, set on creation and phase transitions
- `ThreadStatus`: Running | Parked | Completed | Failed

### Stuck Detection (lib.rs:345-372)
`detect_stale_threads()` runs on each poll tick. It:
1. Finds Running threads where `health(now, 30min) == Stuck`
2. Immediately marks them Failed, releases their slot, **removes from threads map**
3. Logs a generic `ActivityEvent::Error` message

**Problems with current approach:**
- Uses hardcoded 30-minute threshold instead of `config.stuck_threshold_secs` (bug)
- No intermediate "stuck warning" state — thread goes straight from Running to removed
- Thread is deleted from the map, so the UI never sees it as stuck
- No way for user to see which sessions are struggling before auto-recovery kicks in

### Configuration (types.rs:406-498)
`PluginConfig::stuck_threshold_secs` defaults to 600 (10 minutes). Configurable via plugin config map. This value is currently **unused** — the hardcoded 30min in `detect_stale_threads()` takes precedence.

### Activity Events (types.rs:500-555)
`ActivityEvent` variants include `Error { message }` and `ThreadExited { ticket_id, exit_code }`. No variant exists for health state changes specifically.

### Current Event→UI Mapping (lib.rs:785-841)
`activity_event_to_ui_entry()` maps ActivityEvents to UI entries. `ActivityEvent::Error` maps to `ActivityType::Error`. No special handling for health-related events.

## Current UI Architecture

### Dashboard Layout (ui.rs:655-697)
`render_dashboard_lines()` renders sections in order:
1. Title bar with status line (Active/Parked/Done counts)
2. Separator
3. DAG
4. Separator
5. Active Threads table
6. Parked Threads table
7. Separator
8. Activity Log
9. Separator
10. Quick Jump

No attention/alert banner exists.

### UI Types (ui.rs:40-193)
- `PluginState`: tickets, active_threads, parked_threads, activity_log, current_time, selected_ticket, modal
- `ActiveThread`: ticket_id, phase, started_at, pane_id
- `ActivityType`: PhaseCompleted | Commit | Error | ThreadStarted | ThreadParked
- No health/alert types exist

### State→UI Conversion (lib.rs:664-747)
`to_ui_state()` builds `PluginState` from internal State. Only populates `active_threads` (Running) and `parked_threads` (Parked). Failed threads are not represented in the UI state.

## Key Data Flow

```
poll_tick() → check_artifact_advances()
            → detect_stale_threads()    # fails+removes stuck threads
            → rebuild_dag()
            → schedule_ready_tickets()

render()    → to_ui_state()             # converts State → PluginState
            → ui::print_dashboard()     # renders to terminal
```

Health is evaluated in `detect_stale_threads()` but result is consumed silently. The UI rendering path has no access to health information.

## Gaps vs Acceptance Criteria

| Criterion | Current State |
|---|---|
| Failed sessions with exit code in banner | Failed threads are removed from threads map; no banner exists |
| Stuck sessions with time-since-last-activity | Stuck threads immediately failed+removed; no warning state |
| Suggested actions (restart/logs/blocked) | No suggested action UI exists |
| Activity log entries for health changes | Only generic Error events logged; no health-specific events |

## Files That Need Changes

- **types.rs**: Add `HealthStateChanged` activity event variant
- **lib.rs**: Fix stuck threshold to use config; add health evaluation pass that surfaces alerts before cleanup; pass health/alert info to UI state; store failed thread info before removal
- **ui.rs**: Add attention banner section; add health alert types; render suggested actions

## Constraints and Risks

1. The `detect_stale_threads()` removal behavior serves a purpose — it frees slots for retry. Any "stuck warning" phase must eventually still release the slot.
2. Dashboard real estate is limited (terminal pane). The attention banner must be compact.
3. WASM target — no filesystem beyond /host mount, no network. All state is in-memory.
4. Suggested actions (restart, check logs, mark blocked) need keyboard shortcuts since this is a terminal plugin. The 'd' key is already taken for mark-done modal.
