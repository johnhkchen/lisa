# Progress: T-018-03 Per-Phase Timeout

## Completed

- [x] Step 1: Added `phase_timeouts: HashMap<Phase, u64>` to `PluginConfig` in types.rs
  - Added `Phase::from_name()` helper
  - Added `timeout_for_phase()` method
  - Added `from_config_map()` parsing for `phase_timeout_{name}` keys
  - 6 new tests, all passing

- [x] Step 2: Added phase_timeouts to CLI config in config.rs
  - `SchedulingConfig::phase_timeouts: Option<HashMap<String, u64>>`
  - `ResolvedConfig::phase_timeouts: HashMap<String, u64>`
  - Validation of phase names in `[scheduling.phase_timeouts]`
  - Added to `known_scheduling` keys
  - Added commented example to `default_config_toml()`
  - 6 new tests, all passing

- [x] Step 3: Modified timeout enforcement in lib.rs
  - `check_session_timeouts()` now checks both global and per-phase timeouts
  - Per-phase uses `last_phase_change` (resets on phase transition)
  - Global still uses `started_at` (total wall-clock)
  - 4 new tests, all passing

- [x] Step 4: Updated display in init.rs and status.rs
  - Both now print `phase_timeouts: research=300s implement=1800s` when configured

- [x] Step 5: Full verification
  - `cargo test --workspace` — 152 tests pass (was ~133, +19 new)
  - `cargo check -p lisa-plugin --target wasm32-wasip1` — clean

## Deviations

None. Implementation followed the plan exactly.
