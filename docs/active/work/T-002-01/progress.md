# Progress: T-002-01 — Add .lisa.toml Config File

## Completed

- [x] Step 1: Added `toml` and `serde` dependencies to lisa-cli Cargo.toml
- [x] Step 2: Created `config.rs` module with LisaConfig, ResolvedConfig, load_config(), resolve_config(), default_config_toml() — 11 unit tests
- [x] Step 3: Wired config into main.rs — `mod config`, optional `--max-threads`, config loading in Loop handler
- [x] Step 4: Updated loop_cmd.rs to use ResolvedConfig — run_loop(), generate_layout(), run_dry() all accept ResolvedConfig, KDL layout now includes all config fields, added test for custom dirs
- [x] Step 5: Updated init.rs — .lisa.toml in plan_init_actions(), validation of .lisa.toml in run_validate(), 3 new tests (existing_lisa_toml, valid_lisa_toml, invalid_lisa_toml)
- [x] Step 6: Full test suite passes (111 tests: 41 CLI + 44 core + 26 plugin), WASM check passes, toml crate not in plugin dependency tree

## Test Results

- `cargo test --workspace`: 111 tests, 0 failures
- `cargo check -p lisa-plugin --target wasm32-wasip1`: passes (no new dependencies leaked)

## Files Changed

- `crates/lisa-cli/Cargo.toml` — added toml + serde deps
- `crates/lisa-cli/src/config.rs` — NEW: .lisa.toml parsing and config resolution
- `crates/lisa-cli/src/main.rs` — mod config, optional --max-threads, config loading
- `crates/lisa-cli/src/loop_cmd.rs` — ResolvedConfig instead of bare max_threads
- `crates/lisa-cli/src/init.rs` — .lisa.toml in init plan and validate

## No Deviations from Plan
