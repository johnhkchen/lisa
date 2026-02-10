# Research: T-004-01 session-status-model

## Objective

Enrich thread/session tracking with time-in-phase monitoring and health status classification so the dashboard and scheduler can surface stuck or failed sessions.

## Current State

### Thread type (`crates/lisa-core/src/types.rs:271-345`)

```rust
pub struct Thread {
    pub ticket_id: String,
    pub pane_id: u32,
    pub current_phase: Phase,
    pub started_at: SystemTime,   // when the thread was created
    pub status: ThreadStatus,     // Running | Parked | Completed | Failed
}
```

- `started_at` captures thread creation time, not phase-entry time.
- No field tracks when the thread entered its current phase.
- No concept of health or "stuck" detection.

### ThreadStatus enum (`types.rs:253-265`)

```rust
pub enum ThreadStatus {
    Running,
    Parked,
    Completed,
    Failed,
}
```

Four states. No "Stuck" variant — stuck is a health signal, not a lifecycle state. A thread can be Running but Stuck.

### Phase transitions (`lib.rs:258-323`, `check_artifact_advances`)

- On each timer tick (5s), the plugin checks running threads for new phase artifacts.
- When an artifact is found, `thread.current_phase` is updated to the next phase.
- The phase update happens in-place: `thread.current_phase = next_phase`.
- **No timestamp is recorded** when the phase changes.

### Timer/polling (`lib.rs:328-375`, `poll_tick`)

- `set_timeout(POLL_INTERVAL_SECS)` fires every 5 seconds.
- `poll_tick` calls `check_artifact_advances()`, then `rebuild_dag()`, then `schedule_ready_tickets()`.
- This is the natural place to evaluate health on each tick.

### Scheduler thread tracking (`scheduler.rs:224-608`)

- `Scheduler` maintains its own `HashMap<TicketId, Thread>`.
- `update_thread_phase()` sets `thread.current_phase` but does not record when.
- `handle_thread_exit()` and `handle_pane_exit()` mark threads as Completed/Failed based on exit code.

### Plugin State thread tracking (`lib.rs:62-91`)

- `State` has its own `HashMap<TicketId, Thread>` separate from Scheduler.
- Thread phase updates happen in `check_artifact_advances()` and `poll_tick()`.
- Both places set `thread.current_phase = phase` without timestamp.

### UI rendering (`ui.rs`)

- `ActiveThread` and `ParkedThread` structs are constructed from Thread data for display.
- `ActiveThread` shows `started_at` (thread start), not phase-entry time.
- No health indicator currently rendered.

### PluginConfig (`types.rs:347-420`)

- No field for health/stuck timeout threshold.
- `auto_advance: bool` is the only behavior-modifying config field.

### ActivityEvent (`types.rs:422-474`)

- `PhaseCompleted` and `TicketPhaseChanged` events exist but carry no timestamp of their own.
- Events are pushed to `activity_log` in State, which could be used to reconstruct phase-entry times, but that's fragile.

## Key Observations

1. **`last_phase_change` belongs on Thread.** It should be updated whenever `current_phase` is set. This is the simplest, most direct approach — no need to reconstruct from activity logs.

2. **Health is orthogonal to ThreadStatus.** A thread can be `Running` and `Healthy`, or `Running` and `Stuck`. Health is a computed property, not a persisted state. ThreadStatus should not gain a `Stuck` variant.

3. **Two places update `current_phase`:**
   - `check_artifact_advances()` in `lib.rs` (when artifact detected)
   - `poll_tick()` in `lib.rs` (when DAG rebuild detects phase change)
   - `update_thread_phase()` in `scheduler.rs` (called by neither currently)
   - `Thread::new()` in `types.rs` (construction)

   All four sites need to record the phase-change timestamp.

4. **Health evaluation should happen in `poll_tick()`.** The timer tick is the natural evaluation point. After updating phases, check each running thread's time-in-phase against the threshold.

5. **Threshold should be configurable via PluginConfig.** Default: something like 300 seconds (5 minutes) per phase. Research/Design might take longer than Structure/Plan.

6. **`is_attention_needed()` is a method on Thread.** It returns true if the thread is stuck (computed from time-in-phase), failed, or parked for review. This is purely derived state.

7. **Artifact presence is the "progress signal."** If the phase has an artifact filename and it doesn't exist, no progress. If it appeared since last check, progress happened. The existing `check_artifact_advances()` already detects this.

8. **SystemTime is already imported and used.** `types.rs` already uses `SystemTime` for `started_at` with a custom serde helper module. The same serde helper can be reused for `last_phase_change`.

9. **Thread is in lisa-core, health evaluation is in lisa-plugin.** The `Thread` struct and `HealthStatus` enum belong in `lisa-core/src/types.rs`. The health evaluation logic (checking time-in-phase against threshold) goes in the plugin's poll logic.

## Files Involved

| File | Role |
|------|------|
| `crates/lisa-core/src/types.rs` | Add `last_phase_change` to Thread, add `HealthStatus` enum, add `is_attention_needed()` |
| `crates/lisa-core/src/types.rs` | Add `stuck_threshold_secs` to PluginConfig |
| `crates/lisa-plugin/src/lib.rs` | Update all phase-mutation sites to set `last_phase_change`, add health eval in poll_tick |
| `crates/lisa-plugin/src/scheduler.rs` | Update `register_pane()`, `update_thread_phase()` to set `last_phase_change` |
| `crates/lisa-plugin/src/ui.rs` | Add health indicator to ActiveThread display (optional, follow-up) |

## Constraints

- Thread is `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`. Adding `last_phase_change: SystemTime` needs the same serde helper as `started_at`.
- `SystemTime::now()` works in both native tests and WASI sandbox (WASI provides `clock_time_get`).
- HealthStatus should be `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` — it's computed, doesn't need Serialize.
- The `is_attention_needed()` method needs access to "now" and "threshold" to compute stuck status. Two options: pass them in, or store threshold on Thread. Passing them in is cleaner.

## Open Questions

None — the acceptance criteria are clear and the implementation path is straightforward.
