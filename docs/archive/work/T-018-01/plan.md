# Plan: T-018-01 timeout-config-parsing

## Step 1: Add field and constant to PluginConfig (types.rs)

- Add `DEFAULT_SESSION_TIMEOUT_SECS: u64 = 1800` constant
- Add `session_timeout_secs: u64` field to `PluginConfig`
- Initialize in `new()`
- Parse in `from_config_map()` with same pattern as `stuck_threshold_secs`
- Add two unit tests: default value, parsing from map

**Verify**: `cargo test -p lisa-core`

## Step 2: Add to CLI config layer (config.rs)

- Add `session_timeout_secs: Option<u64>` to `SchedulingConfig`
- Add `session_timeout_secs: u64` to `ResolvedConfig`
- Update `ResolvedConfig::default()` to use the core constant
- Add resolution in `resolve_config()`
- Add `"session_timeout_secs"` to `known_scheduling` in `validate_config()`
- Add commented-out line to `default_config_toml()`
- Add four unit tests: TOML parsing, default resolution, config override, known-key validation

**Verify**: `cargo test -p lisa-cli`

## Step 3: Pass through KDL layout (loop_cmd.rs)

- Add `session_timeout_secs` to the KDL plugin config block in `generate_layout()`
- Update `test_generate_layout` to assert the new field appears

**Verify**: `cargo test -p lisa-cli`

## Step 4: Display in `lisa status` (status.rs)

- Load and resolve config in `run_status()`
- Print config summary line with max_threads and session_timeout

**Verify**: `cargo test -p lisa-cli` (existing tests still pass)

## Step 5: Display in `lisa validate` (init.rs)

- After successful validation, print config summary including timeout
- Modify `run_validate()` to load config and display after diagnostics

**Verify**: `cargo test -p lisa-cli`

## Step 6: Full workspace verification

- `cargo test --workspace` — all tests pass
- `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM compiles

## Testing Strategy

- **Unit tests in types.rs**: Default value, parsing from BTreeMap
- **Unit tests in config.rs**: TOML parsing, resolution with/without config, known-key validation
- **Unit test in loop_cmd.rs**: Layout generation includes the new field
- **Integration**: Existing status/validate tests continue to pass (they don't assert on output text)
- No new integration tests needed — the feature is pure config plumbing
