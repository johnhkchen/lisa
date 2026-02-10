# T-004-03 Structure: Error & Health Alerts

## File Changes

### 1. `crates/lisa-core/src/types.rs` (modify)

**Add `Serialize, Deserialize, Clone, Copy` derives to `HealthStatus`:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthStatus { Healthy, Stuck, Failed }
```

**Add `HealthStateChanged` variant to `ActivityEvent`:**
```rust
ActivityEvent::HealthStateChanged {
    ticket_id: TicketId,
    old_health: HealthStatus,
    new_health: HealthStatus,
}
```

No new types needed — `HealthStatus` and `Thread::health()` already exist.

### 2. `crates/lisa-plugin/src/lib.rs` (modify)

**Add to `State` struct:**
```rust
/// Active health alerts for the attention banner.
alerts: Vec<HealthAlert>,
/// Last known health status per ticket, for transition detection.
last_health: HashMap<TicketId, HealthStatus>,
```

**Add `HealthAlert` struct (private to lib.rs):**
```rust
struct HealthAlert {
    ticket_id: TicketId,
    health: HealthStatus,
    /// Exit code if the thread failed with a known code.
    exit_code: Option<i32>,
    /// Duration since last phase change (for stuck alerts).
    time_in_phase: std::time::Duration,
}
```

**Add `evaluate_health()` method to State:**
- Runs each poll tick, before `detect_stale_threads()`
- For each Running thread, compute `health(now, config.stuck_threshold_secs)`
- Compare with `last_health` map — if changed, log `HealthStateChanged` event and create/update alert
- For Failed threads (detected via ThreadStatus), create alert with exit code
- Remove alerts for threads that are no longer problematic (health improved or thread removed)

**Modify `detect_stale_threads()`:**
- Change hardcoded `30 * 60` to `self.config.stuck_threshold_secs * 2` (hard timeout)
- The warning threshold is `stuck_threshold_secs` (used by `evaluate_health()`)
- Keep existing behavior: fail + release slot + remove thread at hard timeout

**Modify `poll_tick()`:**
- Add `self.evaluate_health()` call before `self.detect_stale_threads()`

**Modify `to_ui_state()`:**
- Convert `self.alerts` into `Vec<ui::HealthAlert>` on the `PluginState`

**Store exit code on failed threads:**
- In `poll_tick()` where `ThreadExited` is logged, also record exit code before thread cleanup
- Alternatively, create alert at the point where thread is failed

### 3. `crates/lisa-plugin/src/ui.rs` (modify)

**Add `HealthAlert` struct to UI types:**
```rust
pub struct HealthAlert {
    pub ticket_id: String,
    pub alert_type: AlertType,
    pub detail: String,
    pub suggested_actions: Vec<String>,
}

pub enum AlertType {
    Failed,
    Stuck,
}
```

**Add `alerts` field to `PluginState`:**
```rust
pub alerts: Vec<HealthAlert>,
```

**Add `render_attention_banner()` function:**
- Renders a colored banner between the title bar and DAG
- Failed alerts: red background, shows ticket ID and exit code
- Stuck alerts: yellow background, shows ticket ID and time since last activity
- Each alert shows suggested actions as text
- Compact format: one line per alert, max ~5 visible

**Add `ActivityType::Warning` variant:**
```rust
ActivityType::Warning { ticket_id: String, message: String },
```

**Modify `render_dashboard_lines()`:**
- Insert `render_attention_banner()` call after title bar, before DAG
- Only render if `state.alerts` is non-empty

**Modify `render_status_line()`:**
- Add alert count: `Alerts: N` in red when N > 0

## Module Boundaries

```
types.rs (core)
  └── HealthStatus: +Serialize +Deserialize
  └── ActivityEvent: +HealthStateChanged variant

lib.rs (plugin)
  └── HealthAlert (private struct)
  └── State: +alerts, +last_health fields
  └── evaluate_health() (new method)
  └── detect_stale_threads() (fix threshold)
  └── poll_tick() (add evaluate_health call)
  └── to_ui_state() (pass alerts to UI)

ui.rs (plugin)
  └── HealthAlert (UI struct)
  └── AlertType enum
  └── PluginState: +alerts field
  └── ActivityType: +Warning variant
  └── render_attention_banner() (new function)
  └── render_dashboard_lines() (add banner call)
  └── render_status_line() (add alert count)
```

## Interface Contract

`lib.rs` converts internal `HealthAlert` → `ui::HealthAlert` in `to_ui_state()`. The UI module only knows about UI types. The mapping:
- `HealthStatus::Failed` + exit_code → `AlertType::Failed`, detail = "Exit code: {code}"
- `HealthStatus::Stuck` + time_in_phase → `AlertType::Stuck`, detail = "No progress for {duration}"
- Suggested actions: hardcoded strings per alert type:
  - Failed: ["restart session (re-queued automatically)", "check agent pane for errors"]
  - Stuck: ["check agent pane for progress", "mark as done if complete"]

## Ordering of Changes

1. types.rs — add derives and event variant (no breaking changes)
2. ui.rs — add UI types and rendering (no breaking changes, new code only)
3. lib.rs — wire health evaluation, fix threshold, pass to UI (depends on 1 and 2)
4. Tests throughout
