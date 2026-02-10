# T-008-01 Plan: Hook Infrastructure for Idle Signal

## Step 1: Add template constants and functions in templates.rs

Add to `crates/lisa-cli/src/templates.rs`:
- `ON_IDLE_HOOK` constant — the `/bin/sh` hook script
- `LISA_GITIGNORE` constant — `"signals/\n"`
- `settings_local_json()` function — returns the `.claude/settings.local.json` content

Tests:
- `test_on_idle_hook_content` — script starts with shebang, references `LISA_TICKET_ID`, writes to `.lisa/signals/`
- `test_settings_local_json` — contains `idle_prompt`, references `on-idle.sh`
- `test_lisa_gitignore_content` — contains `signals/`

Commit: "Add hook infrastructure templates for idle signal"

## Step 2: Wire init actions in init.rs

Modify `plan_init_actions()` in `crates/lisa-cli/src/init.rs`:
- Add `.lisa/hooks` and `.lisa/signals` to the directories list
- Add `.lisa/hooks/on-idle.sh` as a CreateFile action
- Add `.lisa/.gitignore` as a CreateFile action
- Add `.claude/settings.local.json` as a CreateFile action

Modify `run_init()` execution:
- After writing `.lisa/hooks/on-idle.sh`, set executable permissions on Unix

Tests:
- `test_plan_init_actions_empty_dir` — update expected count from 9 to 14 (3 new dirs, 3 new files, minus 1 dir overlap: .lisa/hooks dir + .lisa/signals dir = 2 new dirs + .lisa/hooks/on-idle.sh + .lisa/.gitignore + .claude/settings.local.json = 3 new files = 5 new actions total, so 9+5=14)
  - Wait, let me recount: current = 6 dirs + 3 files = 9. New = 2 dirs (.lisa/hooks, .lisa/signals) + 3 files (on-idle.sh, .lisa/.gitignore, settings.local.json) = 5. Total = 14.
- `test_run_init_creates_hook_files` — verify hook script exists and is executable, verify .gitignore, verify settings.json
- `test_run_init_never_overwrites_hooks` — pre-create hook files, verify they aren't overwritten

Commit: "Wire hook infrastructure into lisa init"

## Step 3: Add validation warnings in init.rs

Modify `run_validate()` in `crates/lisa-cli/src/init.rs`:
- Add warning if `.claude/settings.local.json` doesn't exist
- Add warning if `.lisa/hooks/on-idle.sh` doesn't exist

Tests:
- `test_validate_missing_hooks_warns` — setup without hook files, verify warnings but success

Commit: "Add validation warnings for missing hook infrastructure"

## Step 4: Inject LISA_TICKET_ID in plugin spawn command

Modify `build_claude_command()` in `crates/lisa-plugin/src/lib.rs`:
- Prepend `LISA_TICKET_ID={ticket_id}` to the command string

Modify `schedule_ready_tickets()` session reuse path:
- Change `/clear` → `/exit`
- Queue full `build_claude_command()` instead of `build_claude_prompt()`

Tests:
- `test_build_claude_command_includes_env_var` — verify command string contains `LISA_TICKET_ID=`

Commit: "Inject LISA_TICKET_ID env var in Claude spawn command"

## Step 5: Update scheduler's build_claude_command for consistency

Modify `build_claude_command()` in `crates/lisa-plugin/src/scheduler.rs`:
- Include `LISA_TICKET_ID` in the prompt or args for consistency

Tests:
- Update `test_build_claude_command` — verify env var presence

Commit: "Update scheduler command builder for env var consistency"

## Step 6: Verify with cargo test and cargo check

- `cargo test --workspace` — all tests pass
- `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM compiles

No separate commit — verification step.

## Testing Strategy

- **Unit tests** for all template content (step 1)
- **Unit tests** for init action planning with new files (step 2)
- **Integration-style tests** for full init flow creating hook files (step 2)
- **Unit tests** for validate warnings (step 3)
- **Unit tests** for command string format with env var (steps 4-5)
- **WASM compilation check** to ensure plugin changes compile for target (step 6)

## Verification Criteria

- `lisa init` creates `.lisa/hooks/on-idle.sh` (executable), `.lisa/.gitignore`, `.claude/settings.local.json`
- `lisa init` skips these files if they already exist
- `lisa validate` warns about missing hook files
- Plugin spawn command includes `LISA_TICKET_ID={id}` prefix
- Session reuse uses `/exit` + fresh launch instead of `/clear` + prompt
- All existing tests still pass
- WASM target compiles
