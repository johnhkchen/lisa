# Design: T-004-01 session-status-model

## Decision

Health status is a **computed property**, not a persisted state. Thread gains a `last_phase_change: SystemTime` field. A standalone `HealthStatus` enum and `health()` method on Thread compute the health from time-in-phase vs threshold. `is_attention_needed()` combines health, status, and phase into a single boolean.

## Approach: Computed Health with Phase Timestamp

### Why computed, not stored

Storing a `health: HealthStatus` field on Thread means keeping it in sync across every timer tick. It becomes stale between ticks and adds mutation surface. Instead, `health()` is a pure function: `(now, threshold, last_phase_change) -> HealthStatus`. It's always current when called, never stale, and trivially testable.

### Alternatives considered

**A. Stored health field on Thread**
- Pro: No computation on access
- Con: Stale between ticks, must be updated everywhere, adds mutation surface
- Rejected: Health changes every tick — storing it means updating it every tick anyway

**B. Reconstruct phase timing from ActivityEvent log**
- Pro: No new field on Thread
- Con: Fragile, O(n) scan through activity log, log may be truncated
- Rejected: Activity log is capped at 100 entries and has no structural guarantee

**C. Per-phase timeout thresholds**
- Pro: Research/Design might naturally take longer than Structure/Plan
- Con: More configuration surface, harder to explain, diminishing returns
- Rejected for now: Single threshold is simpler. Can add per-phase later if needed.

## Design

### 1. `last_phase_change` field on Thread

```rust
pub struct Thread {
    pub ticket_id: String,
    pub pane_id: u32,
    pub current_phase: Phase,
    pub started_at: SystemTime,
    pub last_phase_change: SystemTime,  // NEW
    pub status: ThreadStatus,
}
```

- Initialized to `SystemTime::now()` in `Thread::new()` (same as `started_at`).
- Updated whenever `current_phase` is set.
- Uses the same `system_time_serde` helper as `started_at`.

### 2. HealthStatus enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HealthStatus {
    Healthy,
    Stuck,
    Failed,
}
```

Placed in `types.rs` alongside ThreadStatus. Not serialized — it's computed.

- `Healthy`: time-in-phase < threshold, thread running
- `Stuck`: time-in-phase >= threshold, thread running, no new artifact detected
- `Failed`: thread status is `ThreadStatus::Failed`

### 3. `health()` method on Thread

```rust
impl Thread {
    pub fn health(&self, now: SystemTime, stuck_threshold: Duration) -> HealthStatus {
        if self.status == ThreadStatus::Failed {
            return HealthStatus::Failed;
        }
        if self.status != ThreadStatus::Running {
            return HealthStatus::Healthy;  // Parked/Completed are not "stuck"
        }
        let elapsed = now.duration_since(self.last_phase_change).unwrap_or_default();
        if elapsed >= stuck_threshold {
            HealthStatus::Stuck
        } else {
            HealthStatus::Healthy
        }
    }
}
```

Takes `now` and `stuck_threshold` as parameters rather than reading global state. This makes it deterministic and testable.

### 4. `is_attention_needed()` method on Thread

```rust
impl Thread {
    pub fn is_attention_needed(&self, now: SystemTime, stuck_threshold: Duration) -> bool {
        matches!(self.health(now, stuck_threshold), HealthStatus::Stuck | HealthStatus::Failed)
            || self.status == ThreadStatus::Parked
    }
}
```

Returns true if:
- Health is Stuck (running too long without phase change)
- Health is Failed (thread exited with error)
- Status is Parked (awaiting human review)

### 5. `stuck_threshold_secs` in PluginConfig

```rust
pub struct PluginConfig {
    // ... existing fields ...
    pub stuck_threshold_secs: u64,  // NEW, default 600 (10 minutes)
}
```

- Parsed from config map key `"stuck_threshold_secs"`.
- Default: 600 seconds (10 minutes). This is generous — most phases complete in 2-5 minutes, but Research/Design can be slower.

### 6. Phase mutation consolidation

Every site that sets `current_phase` must also set `last_phase_change`. The sites:

| Location | Code change |
|----------|-------------|
| `Thread::new()` | Set `last_phase_change: SystemTime::now()` |
| `lib.rs:check_artifact_advances()` | After `thread.current_phase = next_phase`, add `thread.last_phase_change = SystemTime::now()` |
| `lib.rs:poll_tick()` | After `thread.current_phase = ticket.phase`, add `thread.last_phase_change = SystemTime::now()` |
| `scheduler.rs:register_pane()` | Already calls `Thread::new()`, so covered |
| `scheduler.rs:update_thread_phase()` | After setting phase, add `thread.last_phase_change = SystemTime::now()` |

### 7. No UI changes in this ticket

The UI can use `is_attention_needed()` to highlight threads, but that's a separate concern. This ticket focuses on the data model. The UI already shows active/parked threads; adding health indicators is a natural follow-up.

## Testing Strategy

All tests run on native (not WASM), using `SystemTime::now()` and `Duration::from_secs()`:

1. **`health()` returns Healthy for fresh thread** — create thread, call health with 0 elapsed
2. **`health()` returns Stuck after threshold** — create thread with `last_phase_change` in the past
3. **`health()` returns Failed for failed thread** — set status to Failed
4. **`health()` returns Healthy for parked thread** — parked is not stuck
5. **`is_attention_needed()` for stuck** — true
6. **`is_attention_needed()` for failed** — true
7. **`is_attention_needed()` for parked** — true
8. **`is_attention_needed()` for healthy running** — false
9. **`last_phase_change` updated on phase transition** — verify timestamp changes
10. **PluginConfig parses stuck_threshold_secs** — from config map

## Risk

- `SystemTime::now()` in WASI sandbox: WASI provides `clock_time_get` which powers `SystemTime::now()`. Already used for `started_at` without issues.
- Serde for `last_phase_change`: reuses existing `system_time_serde` module with `#[serde(with = "system_time_serde")]`. No new code needed.
