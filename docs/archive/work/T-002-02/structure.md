# T-002-02 Structure: Config Validation

## Files Modified

### `crates/lisa-cli/src/config.rs`

**New type:**
```rust
pub struct ConfigValidation {
    pub config: LisaConfig,
    pub warnings: Vec<String>,
}
```

**New function:**
```rust
pub fn validate_config(content: &str) -> Result<ConfigValidation, String>
```
- Parses TOML as `toml::Value`
- Checks top-level, `[dirs]`, and `[scheduling]` keys against known sets
- Collects unknown keys as warnings
- Deserializes into `LisaConfig`
- Validates `max_threads` semantic constraint (must be >= 1 if present)
- Returns `ConfigValidation`

**Modified function:**
```rust
pub fn load_config(root: &Path) -> Result<ConfigValidation, String>
```
- When file exists: read content, call `validate_config`
- When file doesn't exist: return `ConfigValidation { config: default, warnings: [] }`

Known key sets (constants or inline):
- Top-level: `dirs`, `scheduling`
- `[dirs]`: `tickets`, `stories`, `work`
- `[scheduling]`: `max_threads`, `auto_advance`

### `crates/lisa-cli/src/main.rs`

**Modified:** `Commands::Loop` handler (lines 69-87)
- `load_config` now returns `ConfigValidation`
- Extract `.config` for `resolve_config`
- Print any warnings to stderr before proceeding

### `crates/lisa-cli/src/init.rs`

**Modified:** `run_validate` function (line 180-188)
- `load_config` now returns `ConfigValidation`
- Print config warnings in the warnings section of validation output

## Files NOT Modified

- `crates/lisa-core/src/types.rs` — `PluginConfig` untouched, separate code path
- `crates/lisa-plugin/` — plugin receives pre-validated config via KDL layout
- `crates/lisa-cli/src/loop_cmd.rs` — receives `ResolvedConfig`, no changes needed
- `crates/lisa-cli/src/templates.rs` — generates default config, no validation needed

## Public Interface Changes

- `load_config` return type: `Result<LisaConfig, String>` → `Result<ConfigValidation, String>`
- New public type `ConfigValidation`
- New public function `validate_config`

## Test Plan Location

Tests go in `config.rs`'s existing `#[cfg(test)] mod tests` block. New tests for:
- Unknown top-level key → warning
- Unknown key in `[dirs]` → warning
- Unknown key in `[scheduling]` → warning
- `max_threads = 0` → error
- Negative integer for max_threads → error (TOML parse)
- Valid config with no warnings
- Multiple warnings accumulated
- `load_config` returns warnings from file
