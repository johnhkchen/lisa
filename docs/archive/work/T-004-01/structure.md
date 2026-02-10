# Structure: T-004-01 session-status-model

## Files Modified

### 1. `crates/lisa-core/src/types.rs`

**Add `HealthStatus` enum** (after `ThreadStatus`, ~line 265):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HealthStatus {
    Healthy,
    Stuck,
    Failed,
}
```

**Add `last_phase_change` field to `Thread` struct** (after `started_at`):

```rust
pub struct Thread {
    pub ticket_id: String,
    pub pane_id: u32,
    pub current_phase: Phase,
    #[serde(with = "system_time_serde")]
    pub started_at: SystemTime,
    #[serde(with = "system_time_serde")]
    pub last_phase_change: SystemTime,   // NEW
    #[serde(default)]
    pub status: ThreadStatus,
}
```

**Update `Thread::new()`** to initialize `last_phase_change`:

```rust
pub fn new(ticket_id: impl Into<String>, pane_id: u32) -> Self {
    let now = SystemTime::now();
    Self {
        ticket_id: ticket_id.into(),
        pane_id,
        current_phase: Phase::Ready,
        started_at: now,
        last_phase_change: now,
        status: ThreadStatus::Running,
    }
}
```

**Add `health()` method to Thread impl** (after `mark_exited`):

```rust
pub fn health(&self, now: SystemTime, stuck_threshold: std::time::Duration) -> HealthStatus {
    if self.status == ThreadStatus::Failed {
        return HealthStatus::Failed;
    }
    if self.status != ThreadStatus::Running {
        return HealthStatus::Healthy;
    }
    let elapsed = now.duration_since(self.last_phase_change).unwrap_or_default();
    if elapsed >= stuck_threshold {
        HealthStatus::Stuck
    } else {
        HealthStatus::Healthy
    }
}
```

**Add `is_attention_needed()` method to Thread impl**:

```rust
pub fn is_attention_needed(&self, now: SystemTime, stuck_threshold: std::time::Duration) -> bool {
    matches!(self.health(now, stuck_threshold), HealthStatus::Stuck | HealthStatus::Failed)
        || self.status == ThreadStatus::Parked
}
```

**Add `stuck_threshold_secs` to `PluginConfig`**:

```rust
pub struct PluginConfig {
    pub ticket_dir: PathBuf,
    pub story_dir: PathBuf,
    pub work_dir: PathBuf,
    pub max_threads: usize,
    pub auto_advance: bool,
    pub stuck_threshold_secs: u64,  // NEW, default 600
}
```

**Update `PluginConfig::new()`**: Set `stuck_threshold_secs: 600`.

**Update `PluginConfig::from_config_map()`**: Parse `"stuck_threshold_secs"` key.

**Add tests** to the existing `#[cfg(test)] mod tests` block:
- `test_health_healthy`
- `test_health_stuck`
- `test_health_failed`
- `test_health_parked_not_stuck`
- `test_is_attention_needed`
- `test_last_phase_change_initialized`
- `test_config_stuck_threshold`

### 2. `crates/lisa-plugin/src/lib.rs`

**In `check_artifact_advances()`** (~line 314): After `thread.current_phase = next_phase`, add:
```rust
thread.last_phase_change = std::time::SystemTime::now();
```

**In `poll_tick()`** (~line 364): After `thread.current_phase = ticket.phase`, add:
```rust
thread.last_phase_change = std::time::SystemTime::now();
```

### 3. `crates/lisa-plugin/src/scheduler.rs`

**In `update_thread_phase()`** (~line 498-502): After setting phase, add:
```rust
thread.last_phase_change = std::time::SystemTime::now();
```

## Files NOT Modified

- `crates/lisa-plugin/src/ui.rs` — No UI changes in this ticket. Health indicators are a follow-up.
- `crates/lisa-core/src/dag.rs` — DAG computation unaffected.
- `crates/lisa-core/src/ticket.rs` — Ticket parsing unaffected.
- `crates/lisa-cli/` — CLI unaffected.

## Module Boundaries

- `HealthStatus` and `is_attention_needed()` live in `lisa-core::types` — they're part of the data model.
- Phase-mutation sites that set `last_phase_change` are in `lisa-plugin` — they're plugin lifecycle logic.
- The `health()` method takes parameters (`now`, `threshold`) rather than reading global state, keeping `lisa-core` free of zellij dependencies.

## Public Interface Changes

Added to `lisa_core::types`:
- `pub enum HealthStatus { Healthy, Stuck, Failed }`
- `pub fn Thread::health(&self, now: SystemTime, stuck_threshold: Duration) -> HealthStatus`
- `pub fn Thread::is_attention_needed(&self, now: SystemTime, stuck_threshold: Duration) -> bool`
- `pub field Thread::last_phase_change: SystemTime`
- `pub field PluginConfig::stuck_threshold_secs: u64`

No removals. No breaking changes to existing API — `Thread::new()` still takes `(ticket_id, pane_id)`.
