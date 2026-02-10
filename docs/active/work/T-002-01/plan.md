# Plan: T-002-01 — Add .lisa.toml Config File

## Step 1: Add `toml` dependency to lisa-cli

- Edit `crates/lisa-cli/Cargo.toml` to add `toml = "0.8"` and `serde = { version = "1.0", features = ["derive"] }`
- Verify: `cargo check -p lisa-cli`

## Step 2: Create `config.rs` module

- Create `crates/lisa-cli/src/config.rs` with:
  - `LisaConfig`, `DirsConfig`, `SchedulingConfig` structs (serde Deserialize)
  - `ResolvedConfig` struct with resolved values
  - `load_config(root: &Path) -> Result<LisaConfig, String>` — reads .lisa.toml or returns defaults
  - `resolve_config(config: &LisaConfig, cli_max_threads: Option<usize>) -> ResolvedConfig` — merges defaults + file + CLI overrides
  - `default_config_toml() -> &'static str` — returns the default .lisa.toml content
- Add `mod config;` to `main.rs`
- Unit tests: parse valid TOML, parse empty TOML, missing file returns defaults, partial config merges correctly, CLI overrides win
- Verify: `cargo test -p lisa-cli`

## Step 3: Wire config into `main.rs` CLI

- Change `Commands::Loop.max_threads` from `usize` to `Option<usize>` (remove `default_value`)
- In the `Loop` handler: call `config::load_config()`, then `config::resolve_config()`, pass to `run_loop()`
- Verify: `cargo check -p lisa-cli`

## Step 4: Update `loop_cmd.rs` to use `ResolvedConfig`

- Change `run_loop()` signature to accept `&ResolvedConfig` instead of `max_threads: usize`
- Change `generate_layout()` to accept `&ResolvedConfig`, emit all config fields into KDL
- Change `run_dry()` to use `config.ticket_dir` instead of hardcoded path, use `config.max_threads`
- Update all existing tests in loop_cmd.rs to construct `ResolvedConfig`
- Verify: `cargo test -p lisa-cli`

## Step 5: Update `init.rs` for .lisa.toml creation and validation

- In `plan_init_actions()`: add `.lisa.toml` to the plan (CreateFile with `default_config_toml()` content, or Skip if exists)
- In `run_validate()`: if `.lisa.toml` exists, attempt to parse it with `config::load_config()` and report errors
- Update existing tests, add test for .lisa.toml creation and validation
- Verify: `cargo test -p lisa-cli`

## Step 6: Run full test suite and WASM check

- `cargo test --workspace`
- `cargo check -p lisa-plugin --target wasm32-wasip1`
- Verify no new warnings in lisa-plugin (toml crate should not be in its dependency tree)

## Testing Strategy

- **Unit tests in config.rs**: Parse valid TOML, handle missing file, handle partial config, verify precedence (defaults < file < CLI flags), reject invalid TOML with clear error
- **Updated tests in loop_cmd.rs**: All tests pass with `ResolvedConfig` instead of bare `max_threads`
- **Updated tests in init.rs**: Verify .lisa.toml appears in init plan, verify validate catches bad TOML
- **Integration**: `cargo test --workspace` passes, WASM check passes

## Commit Strategy

- Step 1-2: Single commit "Add config.rs with .lisa.toml parsing"
- Step 3-4: Single commit "Wire .lisa.toml config into CLI loop command"
- Step 5: Single commit "Add .lisa.toml to init and validate commands"
- Step 6: Verification only, no commit needed
