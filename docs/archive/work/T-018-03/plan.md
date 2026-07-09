# Plan: T-018-03 Per-Phase Timeout

## Step 1: Add phase_timeouts to PluginConfig (types.rs)

- Add `pub phase_timeouts: HashMap<Phase, u64>` to `PluginConfig`
- Add `use std::collections::HashMap;` import
- Initialize to `HashMap::new()` in `PluginConfig::new()`
- Add `timeout_for_phase(&self, phase: Phase) -> u64` method
- Parse `phase_timeout_{name}` keys in `from_config_map()`
- Add `Phase::from_str()` helper (or use a match block)

**Tests:**
- `test_config_phase_timeouts_default` — empty by default
- `test_config_phase_timeouts_from_map` — parses phase_timeout_* keys
- `test_timeout_for_phase_with_override` — returns override value
- `test_timeout_for_phase_fallback` — falls back to session_timeout_secs
- `test_timeout_for_phase_disabled` — returns 0 when session_timeout is 0 and no override

**Verify:** `cargo test -p lisa-core`

## Step 2: Add phase_timeouts to CLI config (config.rs)

- Add `pub phase_timeouts: Option<HashMap<String, u64>>` to `SchedulingConfig`
- Add `pub phase_timeouts: HashMap<String, u64>` to `ResolvedConfig`
- Wire through in `resolve_config()`
- Add `"phase_timeouts"` to `known_scheduling` in `validate_config()`
- Validate phase names inside `[scheduling.phase_timeouts]` — warn on unknown phase names
- Add commented example to `default_config_toml()`

**Tests:**
- `test_parse_phase_timeouts` — TOML with [scheduling.phase_timeouts]
- `test_resolve_phase_timeouts` — resolved config carries values
- `test_validate_phase_timeouts_known_key` — no warnings for valid phase names
- `test_validate_phase_timeouts_unknown_phase` — warning for invalid phase name
- `test_parse_partial_phase_timeouts` — only some phases specified

**Verify:** `cargo test -p lisa-cli`

## Step 3: Modify timeout enforcement (lib.rs)

- In `check_session_timeouts()`, add per-phase check using `last_phase_change`
- For each running thread: check both global and per-phase timeout
- Use `self.config.timeout_for_phase(t.current_phase)` for per-phase limit
- Keep existing global check (`started_at` vs `session_timeout_secs`) unchanged
- Both checks can trigger timeout independently

**Tests:**
- `test_per_phase_timeout_triggers` — thread exceeds phase timeout, gets timed out
- `test_per_phase_timeout_resets_on_advance` — advancing phase resets the timer
- `test_per_phase_timeout_fallback_to_global` — no override uses session_timeout_secs
- `test_global_timeout_still_enforced` — global cap still works even with generous per-phase

**Verify:** `cargo test -p lisa-plugin`

## Step 4: Update display (init.rs + status.rs)

- In `run_validate()`, after Config line, print per-phase timeouts if non-empty
- In `run_status()`, same addition
- Format: `  phase_timeouts: research=300s implement=1800s`

**Tests:**
- Existing tests still pass (output format is additive)

**Verify:** `cargo test --workspace`

## Step 5: Full verification

- `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM builds
- `cargo test --workspace` — all tests pass
- `just check` — combined check

## Test Strategy Summary

| Area | New Tests | Existing |
|------|-----------|----------|
| types.rs | 5 | 22 |
| config.rs | 5 | 23 |
| lib.rs | 4 | ~22 |
| init.rs/status.rs | 0 (existing cover) | ~37 |
| **Total** | **14** | **~104** |
