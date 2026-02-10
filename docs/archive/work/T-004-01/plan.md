# Plan: T-004-01 session-status-model

## Steps

### Step 1: Add HealthStatus enum to types.rs

Add `HealthStatus` enum after `ThreadStatus` in `crates/lisa-core/src/types.rs`.

**Verification:** `cargo check -p lisa-core`

### Step 2: Add `last_phase_change` field to Thread

- Add `last_phase_change: SystemTime` field with `#[serde(with = "system_time_serde")]` attribute.
- Update `Thread::new()` to initialize both `started_at` and `last_phase_change` from a single `SystemTime::now()` call.

**Verification:** `cargo check -p lisa-core`

### Step 3: Add `health()` and `is_attention_needed()` methods to Thread

- `health(now, stuck_threshold) -> HealthStatus`
- `is_attention_needed(now, stuck_threshold) -> bool`

**Verification:** `cargo check -p lisa-core`

### Step 4: Add `stuck_threshold_secs` to PluginConfig

- Add field with default 600.
- Add `DEFAULT_STUCK_THRESHOLD_SECS` constant.
- Parse from config map in `from_config_map()`.

**Verification:** `cargo check -p lisa-core`

### Step 5: Write unit tests for Thread health methods

Tests in `types.rs` mod tests:
- `test_health_healthy_fresh_thread`
- `test_health_stuck_after_threshold`
- `test_health_failed_thread`
- `test_health_parked_not_stuck`
- `test_health_completed_not_stuck`
- `test_is_attention_needed_stuck`
- `test_is_attention_needed_failed`
- `test_is_attention_needed_parked`
- `test_is_attention_needed_healthy`
- `test_last_phase_change_initialized`
- `test_config_stuck_threshold_default`
- `test_config_stuck_threshold_from_map`

**Verification:** `cargo test -p lisa-core`

### Step 6: Update phase mutation sites in lib.rs

In `check_artifact_advances()`: after `thread.current_phase = next_phase`, set `thread.last_phase_change = SystemTime::now()`.

In `poll_tick()`: after `thread.current_phase = ticket.phase`, set `thread.last_phase_change = SystemTime::now()`.

**Verification:** `cargo check -p lisa-plugin --target wasm32-wasip1`

### Step 7: Update phase mutation site in scheduler.rs

In `update_thread_phase()`: after setting `thread.current_phase = phase`, set `thread.last_phase_change = SystemTime::now()`.

**Verification:** `cargo check -p lisa-plugin --target wasm32-wasip1`

### Step 8: Update existing tests that construct Thread

Any test that constructs `Thread` directly (not via `Thread::new()`) will need the `last_phase_change` field. Search for struct literal construction and update.

**Verification:** `cargo test --workspace`

### Step 9: Final verification

Run full test suite and WASM check.

**Verification:** `cargo test --workspace && cargo check -p lisa-plugin --target wasm32-wasip1`

## Testing Strategy

- **Unit tests (Step 5):** Cover all health evaluation logic — healthy, stuck, failed, parked, completed. Use `SystemTime::now() - Duration::from_secs(...)` to simulate time passage.
- **Existing tests (Step 8):** Update Thread struct literals in plugin tests to include `last_phase_change`.
- **No integration tests needed:** This is a data model change. The health evaluation is a pure function on Thread fields.

## Commit Strategy

- Steps 1-5: Single commit "Add HealthStatus enum and time-in-phase tracking to Thread"
- Steps 6-8: Single commit "Update phase mutation sites to record last_phase_change"
- Or all in one commit if the diff is small enough.
