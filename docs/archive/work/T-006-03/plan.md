# T-006-03 Plan: Session Launch Command Audit

## Step 1: Add `SessionLaunch` variant to `ActivityEvent`

**File**: `crates/lisa-core/src/types.rs`

Add after the `Info` variant:
```rust
SessionLaunch {
    ticket_id: TicketId,
    pane_id: u32,
    command: String,
},
```

**Verify**: `cargo check -p lisa-core` compiles (the variant is unused yet, but that's fine).

## Step 2: Extract `strip_host_prefix()` in `lib.rs`

**File**: `crates/lisa-plugin/src/lib.rs`

Add a new free function near `build_claude_command`:
```rust
fn strip_host_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    PathBuf::from(s.strip_prefix("/host/").unwrap_or(&s).to_string())
}
```

Replace the inline logic in `schedule_ready_tickets()` (lines 301-308):
```rust
let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);
```

**Verify**: `cargo check -p lisa-plugin --target wasm32-wasip1` compiles. Existing tests pass.

## Step 3: Add `SessionLaunch` logging in `schedule_ready_tickets()`

**File**: `crates/lisa-plugin/src/lib.rs`

In both branches of the has_session check, after building the command string but before the `ThreadSpawned` log, add:
```rust
self.log_activity(ActivityEvent::SessionLaunch {
    ticket_id: ticket_id.clone(),
    pane_id,
    command: cmd.clone(),
});
```

For the reused-session branch, log the prompt (not `/clear`), since that's the meaningful part.

**Verify**: `cargo check -p lisa-plugin --target wasm32-wasip1` compiles.

## Step 4: Update `activity_event_to_ui_entry()` for `SessionLaunch`

**File**: `crates/lisa-plugin/src/lib.rs`

Add match arm in `activity_event_to_ui_entry`:
```rust
ActivityEvent::SessionLaunch { ticket_id, command, .. } => ui::ActivityType::Info {
    ticket_id: ticket_id.clone(),
    message: if command.len() > 120 {
        format!("{}...", &command[..120])
    } else {
        command.clone()
    },
},
```

**Verify**: `cargo check -p lisa-plugin --target wasm32-wasip1` compiles. Existing tests pass.

## Step 5: Add tests

**File**: `crates/lisa-plugin/src/lib.rs` (in `mod tests`)

### Test 5a: `test_strip_host_prefix`
- Input `/host/docs/active/tickets` -> `docs/active/tickets`
- Input `docs/active/tickets` (no prefix) -> `docs/active/tickets`
- Input `/host/` -> empty path
- Input `/host/host/nested` -> `host/nested`

### Test 5b: `test_build_claude_command_includes_required_elements`
- Call `build_claude_command` with a ticket dir and ID
- Assert command contains `claude`
- Assert command contains `--dangerously-skip-permissions`
- Assert command contains the ticket file path
- Assert command contains `CLAUDE.md`
- Assert command contains `docs/knowledge/rdspi-workflow.md`

### Test 5c: `test_build_claude_prompt_includes_context`
- Call `build_claude_prompt` with a ticket dir and ID
- Assert contains ticket path
- Assert contains `CLAUDE.md`
- Assert contains `rdspi-workflow.md`
- Assert does NOT contain `claude` invocation (it's just the prompt)

### Test 5d: `test_ticket_prompt_content`
- Call `ticket_prompt` with various ticket dirs and IDs
- Assert path construction is correct for each

### Test 5e: `test_session_launch_event_to_ui`
- Create a `SessionLaunch` event
- Pass to `activity_event_to_ui_entry`
- Assert it produces a non-None `ActivityEntry`
- Assert the message contains the command

### Test 5f: `test_strip_host_prefix_for_wasi_paths`
- Test with typical WASI paths: `/host/docs/...`, `/host/src/...`
- Confirm they produce correct relative paths

**Verify**: `cargo test --workspace` — all new and existing tests pass.

## Step 6: Final verification

- `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM compiles
- `cargo test --workspace` — all tests pass
- Review: the `SessionLaunch` event is logged for both fresh and reused sessions
