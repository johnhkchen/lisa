# T-006-03 Structure: Session Launch Command Audit

## Files Modified

### 1. `crates/lisa-core/src/types.rs`

**Change**: Add `SessionLaunch` variant to `ActivityEvent` enum.

```rust
// After existing variants, before the closing brace:
/// A session was launched for a ticket
SessionLaunch {
    ticket_id: TicketId,
    pane_id: u32,
    command: String,
},
```

No other changes to types.rs.

### 2. `crates/lisa-plugin/src/lib.rs`

**Changes**:

a) **Extract `strip_host_prefix()`** — new free function near the top, alongside `ticket_prompt`, `build_claude_command`, `build_claude_prompt`:

```rust
fn strip_host_prefix(path: &Path) -> PathBuf
```

Replace the inline stripping logic in `schedule_ready_tickets()` with a call to this function.

b) **Log `SessionLaunch` event** — in `schedule_ready_tickets()`, after sending the command and before inserting the thread, log:

```rust
self.log_activity(ActivityEvent::SessionLaunch {
    ticket_id: ticket_id.clone(),
    pane_id,
    command: cmd.clone(),
});
```

This goes in both the fresh-pane and reused-pane branches (the `cmd` variable already exists in both).

c) **Add `SessionLaunch` to `activity_event_to_ui_entry()`** — map to `ActivityType::Info` with the command as the message (truncated if too long for display).

d) **Add tests** for:
- `strip_host_prefix()` — with /host/ prefix, without, empty path, nested /host/
- `build_claude_command()` — ticket path, CLAUDE.md, RDSPI, --dangerously-skip-permissions
- `build_claude_prompt()` — same content checks without claude invocation
- `ticket_prompt()` — contains all required context elements
- `SessionLaunch` event mapping to UI entry

### 3. No new files created

All changes are modifications to existing files.

## Module Boundaries

- `types.rs` (lisa-core): Only the enum variant addition. No new logic.
- `lib.rs` (lisa-plugin): All behavioral changes. Command construction, logging, UI mapping.
- `scheduler.rs` (lisa-plugin): **No changes**. The parallel command builder stays as-is.
- `ui.rs` (lisa-plugin): **No changes**. The UI module consumes `ActivityType` entries, which are produced by `activity_event_to_ui_entry()` in lib.rs.

## Public Interface Changes

- `ActivityEvent` gains one new variant. This is a non-breaking change for pattern matches with `_` arms. Exhaustive matches (in `activity_event_to_ui_entry`) must be updated.

## Ordering

1. Add variant to `ActivityEvent` (types.rs)
2. Extract `strip_host_prefix()` (lib.rs)
3. Add `SessionLaunch` logging to `schedule_ready_tickets()` (lib.rs)
4. Update `activity_event_to_ui_entry()` (lib.rs)
5. Add tests (lib.rs)

Steps 2-4 can be done in one pass through lib.rs. Step 1 must come first since lib.rs depends on the type.
