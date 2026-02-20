# T-013-02 Plan: Add dependency checks to `lisa loop`

## Step 1: Add `which` and `check_required_deps` to doctor.rs

1. Rename `is_on_path` to `which` and make it `pub(crate)`
2. Update `check_wasm_target` to call `which` instead of `is_on_path`
3. Add `check_required_deps_inner(checks: Vec<DependencyCheck>) -> Result<(), Vec<String>>`:
   - Run checks, filter for required + NotFound, collect names
   - Return `Ok(())` if empty, `Err(names)` otherwise
4. Add `pub(crate) fn check_required_deps() -> Result<(), Vec<String>>` that calls `check_required_deps_inner(build_checks())`
5. Add 3 tests for `check_required_deps_inner` using existing mock helpers

**Verify:** `cargo test -p lisa-cli` — all existing doctor tests pass + 3 new tests pass

## Step 2: Update loop_cmd.rs to use doctor::check_required_deps

1. Remove `check_binary` function
2. Remove `which` function
3. Replace the two `check_binary` calls in `run_loop` with:
   ```rust
   crate::doctor::check_required_deps().map_err(|missing| {
       format!(
           "Missing required dependencies: {}\n\nRun `lisa doctor` for details and install instructions.",
           missing.join(", ")
       )
   })?;
   ```

**Verify:** `cargo test -p lisa-cli` — existing loop_cmd tests still pass

## Step 3: Update init.rs import path

1. Change `crate::loop_cmd::which("zellij")` → `crate::doctor::which("zellij")`
2. Change `crate::loop_cmd::which("claude")` → `crate::doctor::which("claude")`

**Verify:** `cargo test -p lisa-cli` — all tests pass

## Step 4: Final verification

1. `cargo test --workspace` — all tests green
2. `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM still builds
3. Manual smoke: `cargo run -p lisa-cli -- doctor` works
4. Manual smoke: confirm `lisa loop` error message format if a dep is missing (optional, only if easy to test)

## Testing strategy

- **Unit tests in doctor.rs:** `check_required_deps_inner` with mock DependencyChecks (3 tests)
- **Existing tests:** All existing doctor.rs tests (11) and loop_cmd.rs tests (7) must continue to pass
- **No new integration tests needed:** The gating logic is a thin wrapper around `check_required_deps`, which is thoroughly tested with mocks
