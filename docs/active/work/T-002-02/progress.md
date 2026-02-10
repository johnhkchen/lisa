# T-002-02 Progress: Config Validation

## Completed

### Step 1: ConfigValidation type and validate_config function
- Added `ConfigValidation` struct (config + warnings) in `config.rs`
- Added `validate_config(content: &str)` that:
  - Parses TOML as `toml::Value`, checks keys against known sets
  - Warns on unknown top-level sections, unknown `[dirs]` keys, unknown `[scheduling]` keys
  - Rejects `max_threads = 0` with clear error message
  - Negative integers fail TOML→usize deserialization naturally

### Step 2: Updated load_config and callers
- `load_config` now returns `Result<ConfigValidation, String>`
- `main.rs` Loop handler: prints warnings to stderr, extracts `.config` for resolve
- `init.rs` `run_validate`: includes config warnings in validation output

### Step 3: Tests
Added 8 new tests:
- `test_validate_unknown_top_level_key`
- `test_validate_unknown_dirs_key`
- `test_validate_unknown_scheduling_key`
- `test_validate_max_threads_zero`
- `test_validate_negative_max_threads`
- `test_validate_valid_config_no_warnings`
- `test_validate_multiple_warnings`
- `test_load_config_with_warnings`

Updated 2 existing tests for new return type.

### Step 4: Full workspace check
- `cargo test --workspace`: 122 tests pass (49 CLI + 45 core + 28 plugin)
- `cargo check -p lisa-plugin --target wasm32-wasip1`: compiles OK

## Acceptance Criteria Status
- [x] Invalid max_threads (0, negative) rejected with message
- [x] Unknown keys warned about
- [x] Missing optional fields use defaults silently
