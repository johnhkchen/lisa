# Structure: T-018-03 Per-Phase Timeout

## File Changes

### 1. `crates/lisa-core/src/types.rs` (modify)

**PluginConfig struct** — add field:
```rust
pub phase_timeouts: HashMap<Phase, u64>,
```

**PluginConfig::new()** — initialize to empty HashMap (no overrides by default).

**PluginConfig::from_config_map()** — parse `phase_timeout_{phase}` keys:
```rust
for phase_name in ["research", "design", "structure", "plan", "implement", "review"] {
    let key = format!("phase_timeout_{}", phase_name);
    if let Some(val) = config.get(&key) {
        if let (Ok(secs), Ok(phase)) = (val.parse::<u64>(), /* parse phase */) {
            result.phase_timeouts.insert(phase, secs);
        }
    }
}
```

**New method** `PluginConfig::timeout_for_phase(&self, phase: Phase) -> u64`:
- Returns `phase_timeouts.get(&phase)` if present
- Falls back to `session_timeout_secs`
- Returns 0 only if `session_timeout_secs == 0` and no per-phase override

Add `use std::collections::HashMap;` (already imported as BTreeMap, add HashMap).

### 2. `crates/lisa-cli/src/config.rs` (modify)

**SchedulingConfig** — add field:
```rust
pub phase_timeouts: Option<HashMap<String, u64>>,
```
Uses `HashMap<String, u64>` (not `Phase`) because TOML deserialization produces string keys. Convert to `Phase` during resolution.

**ResolvedConfig** — add field:
```rust
pub phase_timeouts: HashMap<String, u64>,
```

**resolve_config()** — carry phase_timeouts through:
```rust
phase_timeouts: config.scheduling.phase_timeouts.clone().unwrap_or_default(),
```

**validate_config()** — add `"phase_timeouts"` to `known_scheduling` array. Validate:
- All keys in `[scheduling.phase_timeouts]` are valid phase names
- All values are non-negative integers (TOML handles this)

**default_config_toml()** — add commented-out example:
```toml
# [scheduling.phase_timeouts]
# research = 300
# implement = 1800
```

### 3. `crates/lisa-plugin/src/lib.rs` (modify)

**check_session_timeouts()** — add per-phase timeout check:

After the existing global timeout check, add a second pass checking per-phase timeout:
```rust
// Per-phase timeout: check time-in-phase against phase-specific limit
let phase_timed_out: Vec<(TicketId, u64, Phase)> = self.threads.iter()
    .filter(|(_, t)| t.status == ThreadStatus::Running)
    .filter(|(tid, _)| !already_timed_out.contains(tid))
    .filter_map(|(tid, t)| {
        let limit = self.config.timeout_for_phase(t.current_phase);
        if limit == 0 { return None; }
        let elapsed = now.duration_since(t.last_phase_change).unwrap_or_default();
        if elapsed >= Duration::from_secs(limit) {
            Some((tid.clone(), elapsed.as_secs(), t.current_phase))
        } else { None }
    })
    .collect();
```

Actually, simpler: replace the single timeout logic. For each running thread, compute the effective timeout as `min(global_remaining, phase_timeout)`. But the acceptance criteria say per-phase overrides the global for that phase. So:

**Revised approach:** Single pass. For each running thread:
1. Check global timeout (`started_at` vs `session_timeout_secs`) — if exceeded, timeout
2. Check per-phase timeout (`last_phase_change` vs `timeout_for_phase(phase)`) — if exceeded, timeout

Either trigger marks the thread as timed out.

### 4. `crates/lisa-cli/src/init.rs` (modify)

**run_validate()** — after printing the Config line, if `phase_timeouts` is non-empty, print:
```
  phase_timeouts: research=300s implement=1800s
```

### 5. `crates/lisa-cli/src/status.rs` (modify)

**run_status()** — same addition as init.rs, print per-phase timeouts if configured.

## Module Boundaries

- `types.rs` owns the `PluginConfig` struct and `timeout_for_phase()` method
- `config.rs` owns TOML parsing and resolution (string keys → no Phase dependency)
- `lib.rs` owns enforcement logic
- `init.rs` and `status.rs` own display

## No New Files

All changes are modifications to existing files. No new modules or crates needed.

## Ordering

1. `types.rs` first — add field and helper method
2. `config.rs` second — add parsing and validation
3. `lib.rs` third — modify timeout enforcement
4. `init.rs` and `status.rs` last — display changes
