# T-010-01 Progress: Hook Scaffolding

## Completed

### Step 1: Hook constants (templates.rs)
- Added `ON_STOP_HOOK` — writes `.lisa/signals/pane-$LISA_PANE_ID.stopped`
- Added `ON_CLEAR_HOOK` — writes `.lisa/signals/pane-$LISA_PANE_ID.cleared`
- Added `test_on_stop_hook_content` and `test_on_clear_hook_content`

### Step 2: settings_local_json() (templates.rs)
- Expanded to produce JSON with Stop, SessionStart[clear], and Notification[idle_prompt] hooks
- Updated `test_settings_local_json` to assert all three hooks

### Step 3: merge_hooks() (templates.rs)
- Added generic `ensure_hook()` helper (private) — parameterized by event type, matcher, command
- Added `merge_hooks()` — calls ensure_hook three times for Stop, SessionStart, Notification
- Removed `merge_idle_prompt_hook()` — replaced at all call sites
- Replaced 4 old merge tests with 5 new tests covering merge_hooks

### Step 4: plan_init_actions() (init.rs)
- Refactored hook scaffolding to loop over all three scripts
- Updated settings.local.json skip condition to check all three hooks
- Switched from `merge_idle_prompt_hook()` to `merge_hooks()`
- Updated `test_plan_init_actions_empty_dir` count from 14 to 16

### Step 5: run_init() (init.rs)
- Replaced single hardcoded chmod with loop over all three hook scripts

### Step 6: validate() (init.rs)
- Added existence + executable checks for on-stop.sh and on-clear.sh
- Expanded settings.local.json content check to verify Stop and SessionStart
- Updated `write_hook_infrastructure()` helper to scaffold all three hooks

### Step 7: New tests (init.rs)
- Added `test_validate_missing_stop_hook`
- Added `test_validate_missing_clear_hook`
- Updated `test_diagnostics_hook_structure_errors` to expect 4 errors (was 2)
- Updated `test_run_init_creates_files` to verify all three hooks
- Updated `test_run_init_never_overwrites_hooks` to verify new hooks created

### Step 8: Full test suite
- `cargo test --workspace` — 290 tests pass (105 + 78 + 107)
- `cargo check -p lisa-plugin --target wasm32-wasip1` — compiles clean

## Deviations from Plan

None. Implementation followed the plan exactly.

## Files Modified

- `crates/lisa-cli/src/templates.rs` — +2 constants, updated settings_local_json, replaced merge function, updated tests
- `crates/lisa-cli/src/init.rs` — updated scaffolding, chmod, validation, test helper, added tests
