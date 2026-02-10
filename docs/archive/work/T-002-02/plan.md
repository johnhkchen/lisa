# T-002-02 Plan: Config Validation

## Step 1: Add ConfigValidation type and validate_config function

In `crates/lisa-cli/src/config.rs`:

1. Add `ConfigValidation` struct with `config: LisaConfig` and `warnings: Vec<String>` fields.
2. Add `validate_config(content: &str) -> Result<ConfigValidation, String>`:
   - Parse content as `toml::Value` (table). Error → return Err with message.
   - Define known keys for each section.
   - Walk top-level table keys, collect warnings for unknowns.
   - If `[dirs]` exists as a table, walk its keys, collect warnings for unknowns.
   - If `[scheduling]` exists as a table, walk its keys, collect warnings for unknowns.
   - Deserialize content into `LisaConfig` via `toml::from_str`. Error → return Err.
   - If `max_threads == Some(0)` → return Err("max_threads must be at least 1").
   - Return Ok(ConfigValidation { config, warnings }).

**Verify:** Unit tests for validate_config (Step 3).

## Step 2: Update load_config signature and callers

1. Change `load_config` to return `Result<ConfigValidation, String>`.
   - Missing file: return `Ok(ConfigValidation { config: LisaConfig::default(), warnings: vec![] })`.
   - File exists: read content, call `validate_config(content)`.

2. Update `main.rs` Loop handler:
   - `load_config` returns `ConfigValidation`.
   - Extract `validation.config` for `resolve_config`.
   - Print `validation.warnings` to stderr with `eprintln!`.

3. Update `init.rs` `run_validate`:
   - `load_config` returns `ConfigValidation`.
   - Add config warnings to the warnings vec.

**Verify:** Existing tests still compile and pass. `cargo test -p lisa-cli`.

## Step 3: Add tests

Add to `config.rs` tests module:

1. `test_validate_unknown_top_level_key` — TOML with `[unknown_section]`, expect warning.
2. `test_validate_unknown_dirs_key` — TOML with `[dirs]\nfoo = "bar"`, expect warning.
3. `test_validate_unknown_scheduling_key` — TOML with `[scheduling]\nmax_thread = 4`, expect warning mentioning the typo.
4. `test_validate_max_threads_zero` — `max_threads = 0`, expect Err.
5. `test_validate_negative_max_threads` — `max_threads = -1`, expect Err (TOML i64 can't deserialize to usize).
6. `test_validate_valid_config_no_warnings` — full valid config, expect empty warnings.
7. `test_validate_multiple_warnings` — multiple unknown keys, expect multiple warnings.
8. `test_load_config_with_warnings` — file with unknown key, `load_config` returns warnings.

**Verify:** `cargo test -p lisa-cli` — all tests pass. `cargo check -p lisa-plugin --target wasm32-wasip1` still passes.

## Step 4: Full workspace check

Run `cargo test --workspace` and `cargo check -p lisa-plugin --target wasm32-wasip1` to confirm nothing is broken.
