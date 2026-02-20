# Plan: T-013-01 — `lisa doctor` subcommand

## Step 1: Create `doctor.rs` with types and core logic

Create `crates/lisa-cli/src/doctor.rs` with:
- `CheckResult` enum (`Found`, `NotFound`, `Skipped`)
- `DependencyCheck` struct (name, required, check closure)
- `CheckReport` struct (name, required, result)
- `run_checks()` function
- `format_report()` function
- `has_failures()` function

**Verify**: Compiles. Unit tests for `run_checks`, `format_report`, `has_failures` using mock closures pass.

## Step 2: Add version-extraction and real check functions

Add to `doctor.rs`:
- `get_command_version()` helper using `std::process::Command`
- `check_zellij()`, `check_claude()`, `check_wasm_target()` functions
- `build_checks()` that assembles the production check list

**Verify**: Compiles. Manual `cargo run -p lisa-cli -- doctor` works on this machine.

## Step 3: Wire up `run_doctor()` and integrate into main.rs

- Add `pub fn run_doctor() -> Result<(), String>` to `doctor.rs`
- Add `mod doctor` and `Doctor` variant to `main.rs`
- Add dispatch in main match

**Verify**: `cargo run -p lisa-cli -- doctor` prints check results. `lisa --help` shows `doctor` subcommand.

## Step 4: Add comprehensive tests

Add to `doctor.rs` `#[cfg(test)]`:
- `test_check_result_found` — mock check returns Found, verify report
- `test_check_result_not_found` — mock check returns NotFound, verify report
- `test_check_result_skipped` — mock check returns Skipped, verify report
- `test_format_report_all_ok` — all checks pass, verify "All dependencies satisfied" message
- `test_format_report_with_failure` — one required check fails, verify install hint in output
- `test_format_report_skipped_not_failure` — optional skipped check doesn't trigger failure
- `test_has_failures_all_found` — returns false
- `test_has_failures_required_missing` — returns true
- `test_has_failures_optional_missing_not_failure` — Skipped optional doesn't fail

**Verify**: `cargo test --workspace` passes.

## Step 5: Run full test suite and verify

- `cargo test --workspace` — all tests pass
- `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM check passes (doctor.rs is CLI only)

## Testing strategy

Unit tests only — no integration tests requiring real binaries. All check logic is tested through mock closures. Format tests assert exact output strings. This keeps tests deterministic across CI environments.
