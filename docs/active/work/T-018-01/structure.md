# Structure: T-018-01 timeout-config-parsing

## Files Modified

### 1. `crates/lisa-core/src/types.rs`

**PluginConfig struct** (line ~434):
- Add constant: `DEFAULT_SESSION_TIMEOUT_SECS: u64 = 1800`
- Add field: `session_timeout_secs: u64`
- Update `new()`: initialize to `Self::DEFAULT_SESSION_TIMEOUT_SECS`
- Update `from_config_map()`: parse `"session_timeout_secs"` key, same pattern as `stuck_threshold_secs`

**Tests** (bottom of file):
- Add `test_config_session_timeout_default` — verify default is 1800
- Add `test_config_session_timeout_from_map` — verify parsing from BTreeMap

### 2. `crates/lisa-cli/src/config.rs`

**SchedulingConfig** (line ~26):
- Add field: `session_timeout_secs: Option<u64>`

**ResolvedConfig** (line ~34):
- Add field: `session_timeout_secs: u64`
- Update `Default` impl: use `PluginConfig::DEFAULT_SESSION_TIMEOUT_SECS`

**resolve_config()** (line ~79):
- Add resolution: `config.scheduling.session_timeout_secs.unwrap_or(defaults.session_timeout_secs)`

**validate_config()** (line ~111):
- Add `"session_timeout_secs"` to `known_scheduling` array

**default_config_toml()** (line ~184):
- Add commented-out line: `# session_timeout_secs = 1800`

**Tests**:
- Add `test_parse_session_timeout_secs` — parse from TOML
- Add `test_resolve_session_timeout_default` — verify default 1800
- Add `test_resolve_session_timeout_from_config` — verify TOML override
- Add `test_validate_session_timeout_known_key` — no warnings

### 3. `crates/lisa-cli/src/loop_cmd.rs`

**generate_layout()** (line ~193):
- Add `session_timeout_secs "{session_timeout_secs}"` to KDL plugin config block
- Add format arg

**Tests**:
- Update `test_generate_layout` — assert `session_timeout_secs` appears in layout

### 4. `crates/lisa-cli/src/status.rs`

**run_status()** (line ~8):
- After loading config (already done at line 10-17), also resolve scheduling config
- Add a config summary line before the wave output:
  ```
  Config: max_threads=2, session_timeout=1800s
  ```

**Tests**:
- Existing tests cover status output; no new test needed (output is printed, not returned).

### 5. `crates/lisa-cli/src/init.rs`

**print_diagnostics()** (line ~742):
- Load config and display timeout in success message. Or, simpler: the success message already says "All checks passed" — append config info.
- Actually, `print_diagnostics` doesn't have access to root path. Better approach: have `run_validate()` print config summary after `print_diagnostics()` succeeds.

## Files NOT Modified

- `crates/lisa-plugin/src/scheduler.rs` — Enforcement is T-018-02's scope.
- `crates/lisa-plugin/src/ui.rs` — Display in dashboard is not in acceptance criteria.
- `crates/lisa-core/src/ticket.rs` — No ticket-level changes.
- `crates/lisa-core/src/dag.rs` — No DAG changes.

## Module Boundaries

- `lisa-core::types::PluginConfig` owns the default constant. Both CLI and plugin reference it.
- `lisa-cli::config` owns the TOML parsing and resolution. CLI commands consume `ResolvedConfig`.
- The KDL layout is the bridge between CLI config and WASM plugin config.

## Ordering

1. Core types first (types.rs) — establishes the constant and field
2. CLI config (config.rs) — TOML parsing depends on the core constant
3. KDL layout (loop_cmd.rs) — depends on ResolvedConfig having the field
4. Status/validate display (status.rs, init.rs) — depends on config resolution
