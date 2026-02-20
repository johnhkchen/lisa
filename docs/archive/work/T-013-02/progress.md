# T-013-02 Progress: Add dependency checks to `lisa loop`

## Completed

### Step 1: Add `which` and `check_required_deps` to doctor.rs
- Renamed `is_on_path` → `which` with `pub(crate)` visibility
- Updated `check_wasm_target` to call `which` instead of `is_on_path`
- Added `check_required_deps()` (public wrapper) and `check_required_deps_inner()` (testable with mocks)
- Added 4 tests: all_found, one_missing, all_missing, optional_skipped_is_ok

### Step 2: Updated loop_cmd.rs
- Removed `check_binary()` and `which()` functions
- Replaced two `check_binary` calls with single `check_required_deps()` call
- Error message now includes missing dep names and points to `lisa doctor`

### Step 3: Updated init.rs
- Changed `crate::loop_cmd::which(...)` → `crate::doctor::which(...)` (2 occurrences)

### Step 4: Verification
- `cargo test -p lisa-cli`: 127 tests pass (was 123, +4 new doctor tests)
- `cargo check -p lisa-plugin --target wasm32-wasip1`: compiles clean

## Acceptance criteria status

- [x] `lisa loop` checks for `zellij` and `claude` before launching
- [x] Clear error message pointing to `lisa doctor` on failure
- [x] Normal operation when deps are present (no extra output)
- [x] Shared check logic between `doctor` and `loop` (no duplication)
- [x] Tests cover the gating behavior (4 new mock-based tests)

## Deviations from plan

None.
