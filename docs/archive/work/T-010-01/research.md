# T-010-01 Research: Hook Scaffolding

## Current State

### templates.rs — Hook Constants and Templates

**ON_IDLE_HOOK** (line 11): Shell script constant writing `.lisa/signals/pane-$LISA_PANE_ID.idle` on idle_prompt notification. Pattern: shebang, mkdir signals dir, write timestamped signal file keyed by `LISA_PANE_ID`.

**LISA_GITIGNORE** (line 24): Contains `signals/` — keeps signal files out of git.

**settings_local_json()** (line 27): Returns a fresh JSON string with a single `Notification` hook entry containing `idle_prompt` matcher pointing to `.lisa/hooks/on-idle.sh`. Only handles one hook event type.

**merge_idle_prompt_hook()** (line 49): Merges the idle_prompt hook into existing `settings.local.json`. Logic:
1. Parse JSON root as object
2. Get or create `hooks` object
3. Get or create `Notification` array
4. Check if any entry has `matcher: "idle_prompt"` — skip if present
5. Otherwise push the hook entry
6. Re-serialize with `serde_json::to_string_pretty`

This function is tightly coupled to `Notification[idle_prompt]`. Not generic.

### init.rs — Scaffolding and Validation

**plan_init_actions()** (line 32):
- Creates `.lisa/hooks` and `.lisa/signals` directories
- Creates `.lisa/hooks/on-idle.sh` (if missing)
- Creates `.lisa/.gitignore`
- Creates `.claude/settings.local.json` (fresh) or merges idle_prompt into existing one
- Never overwrites existing files — skip or update semantics

**run_init()** (line 186):
- After writing files, sets executable permission on `on-idle.sh` (unix only, line 247)
- Hardcoded to only chmod `on-idle.sh` — needs extension for new hooks

**validate()** (line 325), hook checks:
- Checks `.claude/settings.local.json` exists and contains `"idle_prompt"` string (line 410)
- Checks `.lisa/hooks/on-idle.sh` exists (line 431) and is executable (line 439, unix only)
- No checks for Stop or SessionStart hooks yet

### Test Helper: write_hook_infrastructure()

Line 818: Creates `.claude/settings.local.json` (via `settings_local_json()`) and `.lisa/hooks/on-idle.sh` (via `ON_IDLE_HOOK`), then chmods the hook. All tests that pass validation use this helper.

## What Needs to Change

### 1. New Constants in templates.rs
Two new shell script constants following ON_IDLE_HOOK pattern:
- `ON_STOP_HOOK` — writes `.stopped` signal file
- `ON_CLEAR_HOOK` — writes `.cleared` signal file

### 2. settings_local_json() Expansion
Must produce JSON with three hook event types:
- `Stop`: array with one entry (no matcher), command → `on-stop.sh`
- `SessionStart`: array with one entry, matcher `"clear"`, command → `on-clear.sh`
- `Notification`: array with one entry, matcher `"idle_prompt"`, command → `on-idle.sh`

### 3. Hook Merge Logic
`merge_idle_prompt_hook()` only handles Notification. Options:
- **Option A:** Rename to `merge_hooks()`, handle all three event types in one function
- **Option B:** Three separate functions (merge_stop_hook, merge_clear_hook, merge_idle_prompt_hook)
- **Option C:** Generic `ensure_hook()` function parameterized by event type, matcher, and command

Ticket says "rename to `merge_hooks()` or keep separate functions." The generic approach (C) is cleanest.

### 4. init.rs Scaffolding
Add `on-stop.sh` and `on-clear.sh` to `plan_init_actions()` following the on-idle.sh pattern. Add chmod for all three hooks in `run_init()`.

### 5. init.rs Validation
`validate()` must check:
- `.lisa/hooks/on-stop.sh` exists + executable
- `.lisa/hooks/on-clear.sh` exists + executable
- `settings.local.json` contains `"Stop"` and `"SessionStart"` hooks (in addition to idle_prompt)

### 6. init.rs settings.local.json Merge
The settings merge in `plan_init_actions()` (line 141-180) currently only checks for `idle_prompt`. It needs to check all three hook types and merge whichever are missing.

### 7. Test Updates
- `write_hook_infrastructure()` helper must scaffold all three hooks
- `test_plan_init_actions_empty_dir`: file count goes from 14 to 16 (2 more hook files)
- `settings_local_json()` tests need expanded assertions
- New tests for merge logic covering all three hook types
- Validation tests for missing stop/clear hooks

## Key Constraints

- `Stop` hook has NO `matcher` field (fires on every turn completion)
- `SessionStart` hook uses `matcher: "clear"` (only fires on /clear, not startup/resume/compact)
- Signal file extensions: `.stopped`, `.cleared`, `.idle` — consumed by plugin (T-010-02)
- `LISA_PANE_ID` env var persists across /clear (same process)
- Never overwrite existing hook scripts — only create if missing
- Merge logic must preserve user's existing hooks in settings.local.json

## Files to Modify

- `crates/lisa-cli/src/templates.rs` — constants, settings_local_json(), merge logic
- `crates/lisa-cli/src/init.rs` — plan_init_actions(), run_init(), validate(), tests
