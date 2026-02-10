# T-003-01 Progress: validate-claude-spawn

## Completed

### Step 1: Fix RDSPI path and visibility in scheduler.rs
- Changed `build_claude_command` from `fn` to `pub(crate) fn`
- Changed `ClaudeCommand` struct to `pub(crate)`
- Fixed RDSPI path: `docs/rdspi-workflow.md` -> `docs/knowledge/rdspi-workflow.md`
- Added `test_build_claude_command` test

### Step 2: Extract `build_spawn_args` in lib.rs
- Added `build_spawn_args(ticket_dir: &Path, ticket_id: &str) -> Vec<String>` free function
- Updated `schedule_ready_tickets()` to call it
- Added `test_build_spawn_args` test

### Step 3: Fix pane exit handling in lib.rs
- Updated `handle_pane_exited` signature to accept `context: BTreeMap<String, String>`
- Context-based ticket lookup with fallback to `pane_to_ticket`
- Updated `CommandPaneExited` event handler to pass context instead of `_context`
- Added 3 tests: `test_handle_pane_exited_with_context`, `test_handle_pane_exited_failure`, `test_handle_pane_exited_no_context_fallback`

### Step 4: Verification
- `cargo test --workspace`: 116 tests pass (44 + 31 + 41)
- `cargo check -p lisa-plugin --target wasm32-wasip1`: passes

## Acceptance Criteria Status
- [x] `build_claude_command()` references correct paths (fixed RDSPI path, test validates)
- [x] `open_command_pane` passes correct context BTreeMap, and `handle_pane_exited` uses it
- [x] Tests validate command construction for sample tickets (5 new tests)

## Deviations from Plan
- None
