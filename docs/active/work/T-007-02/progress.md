# Progress: T-007-02 validate-pre-loop-readiness

## Completed

### Step 1: Make `which` pub(crate) in loop_cmd.rs
- Changed `fn which` to `pub(crate) fn which` at loop_cmd.rs:137

### Step 2: Add `--check-tools` flag to CLI
- Added `check_tools: bool` field to `Commands::Validate` in main.rs
- Updated match arm to destructure and pass to `run_validate`

### Step 3: Rewrite `run_validate` in init.rs
- Changed signature to `pub fn run_validate(root: &Path, check_tools: bool)`
- Added tool checks (zellij, claude) when `check_tools` is true
- Upgraded rdspi-workflow.md check from warning to error
- Added config-aware ticket directory resolution via `config::load_config`
- Used `scan_tickets_with_diagnostics` for per-file error surfacing
- Added "no tickets" error check
- Added "no ready tickets" error check
- Grouped output: errors first, then warnings, then summary
- Summary: "Ready for `lisa loop`." or "N error(s) must be fixed."
- Removed `run_validate` call from `run_init` (replaced with next-steps message)

### Step 4: Updated existing tests
- All 7 existing tests updated with `false` second argument
- `test_validate_valid_setup` and `test_validate_valid_lisa_toml` now include a ready ticket

### Step 5: Added 7 new tests
- `test_validate_missing_rdspi_workflow` — error when missing
- `test_validate_empty_ticket_dir` — error when no .md files
- `test_validate_no_ready_tickets` — all tickets done, error
- `test_validate_ticket_parse_error` — malformed ticket surfaces as error
- `test_validate_acceptance_criteria_warning` — returns Ok (warning only)
- `test_validate_check_tools_false` — tools not checked when flag is false
- `test_validate_no_ticket_dir` — error when directory missing

### Step 6: Full test suite
- 245 tests pass (72 cli + 77 core + 96 plugin)
- WASM target compiles clean

## Files Changed
- `crates/lisa-cli/src/main.rs` — added `check_tools` field, updated match
- `crates/lisa-cli/src/loop_cmd.rs` — `which` visibility to `pub(crate)`
- `crates/lisa-cli/src/init.rs` — rewrote `run_validate`, added tests, updated `run_init`

## Deviations from Plan
- Removed `run_validate` call from `run_init` (post-init always fails since no tickets exist yet). Replaced with next-steps instructions. This is cleaner than trying to make validate lenient when called from init.
