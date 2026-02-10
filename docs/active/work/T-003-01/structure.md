# T-003-01 Structure: validate-claude-spawn

## Files Modified

### 1. `crates/lisa-plugin/src/scheduler.rs`

**Changes**:
- Line 422: Fix RDSPI path from `docs/rdspi-workflow.md` to `docs/knowledge/rdspi-workflow.md`
- Line 409: Change `fn build_claude_command` from `fn` to `pub(crate) fn` for testability
- Add test `test_build_claude_command` in the `#[cfg(test)] mod tests` block

**Test**: Construct a `Scheduler` with known `SchedulerConfig`, call `build_claude_command("T-001")`, assert:
- Args contain `--dangerously-skip-permissions`
- Args contain `--print`
- Prompt string contains `docs/active/tickets/T-001.md`
- Prompt string contains `docs/knowledge/rdspi-workflow.md`
- Prompt string contains `CLAUDE.md`

### 2. `crates/lisa-plugin/src/lib.rs`

**Changes**:

A. Extract command-building logic from `schedule_ready_tickets()` into a standalone function:

```rust
fn build_spawn_args(ticket_dir: &Path, ticket_id: &str) -> Vec<String>
```

This takes the ticket_dir (already with /host/ prefix stripped) and ticket_id, returns the args vec. The existing inline code at lines 135-141 moves into this function.

B. Update `Event::CommandPaneExited` match arm (line 314):

From:
```rust
Event::CommandPaneExited(pane_id, exit_code, _context) => {
    self.handle_pane_exited(pane_id, exit_code);
```

To:
```rust
Event::CommandPaneExited(pane_id, exit_code, context) => {
    self.handle_pane_exited_with_context(pane_id, exit_code, context);
```

C. Add `handle_pane_exited_with_context()` method:

```rust
fn handle_pane_exited_with_context(
    &mut self,
    pane_id: u32,
    exit_code: Option<i32>,
    context: BTreeMap<String, String>,
)
```

Logic:
1. Try to get `ticket_id` from context first
2. Fall back to `pane_to_ticket.remove(&pane_id)` if context is empty
3. If ticket_id found, update thread status and log activity (same as current `handle_pane_exited`)

D. Remove old `handle_pane_exited()` (or keep as private helper called by the new method)

E. Add tests:
- `test_build_spawn_args` — validates the extracted function produces correct args
- `test_handle_pane_exited_with_context` — validates context-based ticket lookup

## No New Files

All changes are modifications to existing files.

## Module Boundaries

- `build_spawn_args` is a free function in lib.rs (not a method on State) since it only needs path + id
- `build_claude_command` stays as a method on Scheduler (just made pub(crate))
- `handle_pane_exited_with_context` replaces `handle_pane_exited` on State
