# T-010-01 Plan: Hook Scaffolding

## Step 1: Add ON_STOP_HOOK and ON_CLEAR_HOOK constants to templates.rs

Add the two new shell script constants after `ON_IDLE_HOOK`. Follow the exact same pattern. Add corresponding tests (`test_on_stop_hook_content`, `test_on_clear_hook_content`).

**Verify:** `cargo test -p lisa-cli -- test_on_stop_hook_content test_on_clear_hook_content`

## Step 2: Update settings_local_json() in templates.rs

Replace the function body to return JSON with all three hook event types (Stop, SessionStart, Notification). Update `test_settings_local_json` to assert all three hooks present.

**Verify:** `cargo test -p lisa-cli -- test_settings_local_json`

## Step 3: Replace merge_idle_prompt_hook() with merge_hooks() in templates.rs

1. Add private `ensure_hook()` helper
2. Add public `merge_hooks()` calling `ensure_hook()` three times
3. Remove `merge_idle_prompt_hook()`
4. Replace the 4 old merge tests with new tests covering merge_hooks:
   - empty object → adds all three
   - existing with idle only → adds Stop + SessionStart
   - already complete → no duplicates
   - preserves unrelated keys
   - invalid JSON → error

**Verify:** `cargo test -p lisa-cli -- test_merge_hooks`

## Step 4: Update plan_init_actions() in init.rs

1. Add scaffolding blocks for `on-stop.sh` and `on-clear.sh`
2. Update settings.local.json skip condition to check all three hooks
3. Switch from `merge_idle_prompt_hook()` to `merge_hooks()`
4. Update `test_plan_init_actions_empty_dir` count (14 → 16)

**Verify:** `cargo test -p lisa-cli -- test_plan_init_actions`

## Step 5: Update run_init() in init.rs

Replace single hardcoded chmod with loop over all three hook scripts.

**Verify:** `cargo test -p lisa-cli -- test_run_init_creates_files`

## Step 6: Update validate() in init.rs

1. Add existence + executable checks for on-stop.sh and on-clear.sh
2. Expand settings.local.json content check to verify Stop and SessionStart
3. Update `write_hook_infrastructure()` test helper to scaffold all three hooks

**Verify:** `cargo test -p lisa-cli -- test_validate`

## Step 7: Add validation tests for new hooks in init.rs

- `test_validate_missing_stop_hook`
- `test_validate_missing_clear_hook`
- Update `test_diagnostics_hook_structure_errors` (expect 4 errors, was 2)

**Verify:** `cargo test -p lisa-cli -- test_validate_missing_stop test_validate_missing_clear test_diagnostics_hook`

## Step 8: Full test suite

Run `cargo test --workspace` and fix any breakage.

**Verify:** `cargo test --workspace` — all tests pass, zero failures
