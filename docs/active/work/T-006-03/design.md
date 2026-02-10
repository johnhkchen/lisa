# T-006-03 Design: Session Launch Command Audit

## Decision: Add `SessionLaunch` event + extract command builder for testability

### Approach

Add a `SessionLaunch` variant to `ActivityEvent` that captures the ticket ID, pane ID, and full command string. Log it immediately after sending the command in `schedule_ready_tickets()`. Extract the `/host/` stripping and command construction into testable helper functions.

### Options Considered

**Option A: Log with existing `Info` variant**
- Just log `ActivityEvent::Info { message: format!("Launch: {}", cmd) }`.
- Pros: Zero type changes, minimal diff.
- Cons: Not structured. Can't pattern-match in tests or filter in UI. Doesn't meet the AC ("Add a `SessionLaunch { ticket_id, pane_id, command }` variant").
- **Rejected**: AC explicitly requires a new variant.

**Option B: Add `SessionLaunch` variant, log inline (minimal change)**
- Add the variant to `ActivityEvent`. Log it in `schedule_ready_tickets()` right after sending the command.
- Keep command construction where it is.
- Pros: Small diff, meets AC.
- Cons: Command construction still untestable (embedded in a function that calls zellij APIs).
- **Rejected**: We can't test command content without extraction.

**Option C: Add `SessionLaunch` variant + extract command helpers (chosen)**
- Add `SessionLaunch { ticket_id, pane_id, command }` to `ActivityEvent`.
- Extract `strip_host_prefix()` helper for `/host/` path normalization.
- Make `build_claude_command()` and `build_claude_prompt()` take a `&Path` ticket_dir (already do) and return the command string (already do). Add tests for them.
- Log `SessionLaunch` in `schedule_ready_tickets()` after sending.
- Add UI rendering for the new event in the `activity_event_to_ui_entry` mapping.
- Pros: Testable, structured, meets all ACs.
- Cons: Slightly larger diff than option B, but the extraction is trivial.

### What NOT to Do

- **Don't remove scheduler.rs command builder**: It's used by the `Scheduler` tests and the `open_command_pane_floating` path. Reconciling the two builders is out of scope.
- **Don't add work directory to the prompt**: Research found that work directory is not currently in the prompt. The AC says "command includes... work directory path" but the current prompt doesn't mention it. We'll add it to the prompt since the AC requires it — the agent needs to know where to write its artifacts.
- **Don't change `--print` behavior**: The lib.rs interactive sessions intentionally don't use `--print`. The AC mentions it as an example ("--print or appropriate flags"). The appropriate flag for interactive sessions is `--dangerously-skip-permissions` without `--print`.

### SessionLaunch Variant Design

```rust
SessionLaunch {
    ticket_id: TicketId,
    pane_id: u32,
    command: String,
}
```

The `command` field stores the full string that was sent to the pane. For fresh launches this is `claude --dangerously-skip-permissions "..."`. For reused sessions this is the prompt text (since /clear is sent separately).

### `/host/` Stripping

Extract a simple helper:

```rust
fn strip_host_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    PathBuf::from(s.strip_prefix("/host/").unwrap_or(&s).to_string())
}
```

This is currently inline in `schedule_ready_tickets()`. Extracting it makes it testable and reusable.

### UI Mapping

Map `SessionLaunch` to a new or existing `ActivityType`. The simplest approach: map to `Info` with the command as the message, since the UI already renders info entries. This avoids adding a new `ActivityType` variant to ui.rs.

### Test Plan (preview)

1. Test `strip_host_prefix()` with various inputs
2. Test `build_claude_command()` includes required elements
3. Test `build_claude_prompt()` includes required elements
4. Test `SessionLaunch` event construction and pattern matching
5. Test `activity_event_to_ui_entry` handles `SessionLaunch`
6. Test that the command includes ticket path, CLAUDE.md reference, RDSPI reference
7. Test `/host/` prefix handling edge cases (no prefix, double prefix, etc.)
