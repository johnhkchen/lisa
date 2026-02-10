# T-008-01 Progress: Hook Infrastructure for Idle Signal

## Completed

### Step 1: Template constants and functions (templates.rs)
- Added `ON_IDLE_HOOK` constant — `/bin/sh` script that reads `$LISA_TICKET_ID` and writes `.lisa/signals/{id}.idle`
- Added `LISA_GITIGNORE` constant — `"signals/\n"`
- Added `settings_local_json()` function — returns Claude Code `idle_prompt` notification hook config
- Added 3 tests: hook content, settings JSON structure, gitignore content

### Step 2: Init actions (init.rs)
- Added `.lisa/hooks` and `.lisa/signals` directories to init plan
- Added `.lisa/hooks/on-idle.sh`, `.lisa/.gitignore`, `.claude/settings.local.json` as CreateFile actions
- All follow existing skip-if-exists pattern
- Added `#[cfg(unix)]` chmod 0o755 for hook script after creation
- Updated test count from 9 to 14 expected init actions
- Added tests: creates hook files, executable permission, never-overwrites hooks, existing hooks skipped

### Step 3: Validation warnings (init.rs)
- Added warnings for missing `.claude/settings.local.json` and `.lisa/hooks/on-idle.sh`
- These are warnings (not errors) — hook infrastructure is optional for basic operation

### Step 4: Env var injection in plugin spawn (lib.rs)
- Modified `build_claude_command()` to prepend `LISA_TICKET_ID={ticket_id}` to the shell command
- Changed session reuse from `/clear` + bare prompt to `/exit` + full launch command
  - Ensures env var is always correct for the current ticket
- Removed `build_claude_prompt()` function (no longer needed)
- Updated tests: env var presence, command format, removed stale prompt tests

### Step 5: Scheduler consistency (scheduler.rs)
- Updated `build_claude_command()` to use `sh -c` with `LISA_TICKET_ID` env var
- Updated `spawn_claude_session()` to use `sh` as the binary
- Updated test for new command format

### Step 6: Verification
- `cargo test --workspace` — 249 tests pass (77 cli + 77 core + 95 plugin)
- `cargo check -p lisa-plugin --target wasm32-wasip1` — compiles successfully
- All warnings are pre-existing dead_code warnings, none from new code

## Test Summary
- New tests added: 7
  - `test_on_idle_hook_content`
  - `test_settings_local_json`
  - `test_lisa_gitignore_content`
  - `test_build_claude_command_includes_env_var` (lib.rs)
  - `test_run_init_creates_files` (extended)
  - `test_run_init_never_overwrites_hooks`
  - `test_plan_init_actions_existing_hooks`
- Updated tests: 3
  - `test_plan_init_actions_empty_dir` (count 9→14)
  - `test_build_claude_command` (lib.rs, updated for env var prefix)
  - `test_build_claude_command` (scheduler.rs, updated for sh -c format)

## Acceptance Criteria Status
- [x] `lisa init` creates `.claude/settings.local.json` with idle_prompt hook
- [x] `lisa init` creates `.lisa/hooks/on-idle.sh` (executable)
- [x] `.lisa/signals/` is gitignored (via `.lisa/.gitignore`)
- [x] Plugin's spawn command sets `LISA_TICKET_ID` env var per session
- [x] Signal script writes correct file when invoked with env var set
