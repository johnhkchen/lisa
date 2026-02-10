# T-003-01 Plan: validate-claude-spawn

## Step 1: Fix RDSPI Path and Visibility in scheduler.rs

1. Change `fn build_claude_command` to `pub(crate) fn build_claude_command`
2. Change `docs/rdspi-workflow.md` to `docs/knowledge/rdspi-workflow.md` in the prompt string
3. Add `test_build_claude_command` test

**Verify**: `cargo test -p lisa-plugin -- test_build_claude_command` passes

## Step 2: Extract `build_spawn_args` in lib.rs

1. Add a `build_spawn_args(ticket_dir: &Path, ticket_id: &str) -> Vec<String>` function
2. Replace the inline args construction in `schedule_ready_tickets()` with a call to `build_spawn_args`
3. Add `test_build_spawn_args` test

**Verify**: `cargo test -p lisa-plugin -- test_build_spawn_args` passes

## Step 3: Fix pane exit handling in lib.rs

1. Rename `handle_pane_exited` to `handle_pane_exited_with_context`, adding `context: BTreeMap<String, String>` parameter
2. Update the method body to extract `ticket_id` from context first, falling back to `pane_to_ticket`
3. Update `Event::CommandPaneExited` match arm to pass `context` instead of `_context`
4. Add `test_handle_pane_exited_with_context` test

**Verify**: `cargo test -p lisa-plugin -- test_handle_pane_exited` passes

## Step 4: Full Test Suite

**Verify**: `cargo test --workspace` passes, `cargo check -p lisa-plugin --target wasm32-wasip1` passes

## Testing Strategy

- **Unit tests for command construction**: Test that `build_claude_command` and `build_spawn_args` produce args with correct paths, flags, and prompt content. These are pure functions (or close to it) so no mocking needed.
- **Unit test for pane exit handling**: Construct a `State` with a known thread, call `handle_pane_exited_with_context` with a context containing the ticket_id, assert thread status changes.
- **WASM check**: Ensure changes compile for wasm32-wasip1 target.
