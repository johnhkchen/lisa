# T-008-01 Structure: Hook Infrastructure for Idle Signal

## Files Modified

### 1. `crates/lisa-cli/src/init.rs`

**`plan_init_actions()`** — Add new init actions after existing ones:

```
CreateDir:  .lisa/hooks
CreateDir:  .lisa/signals
CreateFile: .lisa/hooks/on-idle.sh       (from templates::ON_IDLE_HOOK)
CreateFile: .lisa/.gitignore             (contains "signals/")
CreateFile: .claude/settings.local.json  (from templates::settings_local_json())
```

All five follow the existing skip-if-exists pattern. No new `InitAction` variants needed.

**`run_init()` execution** — After `fs::write` for `.lisa/hooks/on-idle.sh`, add a `#[cfg(unix)]` call to `std::fs::set_permissions()` with mode `0o755` to make it executable. Add a comment explaining why.

**`run_validate()`** — Add two new warning checks:
- Warn if `.claude/settings.local.json` doesn't exist
- Warn if `.lisa/hooks/on-idle.sh` doesn't exist

These are warnings, not errors — the hook infrastructure is optional for basic operation.

### 2. `crates/lisa-cli/src/templates.rs`

Add two new constants and one function:

```rust
/// The on-idle hook script content
pub const ON_IDLE_HOOK: &str = "#!/bin/sh\n...";

/// The .lisa/.gitignore content
pub const LISA_GITIGNORE: &str = "signals/\n";

/// Generate .claude/settings.local.json content
pub fn settings_local_json() -> String { ... }
```

`settings_local_json()` returns the JSON string with the `Notification` hook configuration for `idle_prompt`. It's a function rather than a constant because it may need to be parameterized in the future (e.g., custom hook path).

### 3. `crates/lisa-plugin/src/lib.rs`

**`build_claude_command()`** (line 38) — Prepend `LISA_TICKET_ID={ticket_id}` to the shell command:

```
Before: "claude --dangerously-skip-permissions \"...\""
After:  "LISA_TICKET_ID={id} claude --dangerously-skip-permissions \"...\""
```

**`schedule_ready_tickets()`** (line 326-338) — Change the session reuse path:

```
Before: /clear → queue prompt
After:  /exit  → queue full launch command (with env var)
```

The fresh-pane path stays the same (send full command with env var). The reuse path changes from `/clear` + bare prompt to `/exit` + fresh launch command. The `has_session` flag is kept because the pane shell is still alive after `/exit`.

### 4. `crates/lisa-plugin/src/scheduler.rs`

**`build_claude_command()`** (line 409) — Update to include `LISA_TICKET_ID` in the args. Note: this is the scheduler's version, currently unused by the active plugin flow but should stay consistent.

Since `CommandToRun` has no `env` field, change the command to use `sh -c`:
```rust
ClaudeCommand {
    binary: "sh",
    args: ["-c", "LISA_TICKET_ID={id} claude --dangerously-skip-permissions ..."]
}
```

Actually — this method returns `ClaudeCommand { args }` which is used with `open_command_pane_floating()`. Since this code path is unused, and the ticket specifically says to handle env var injection via `sh -c`, update it for consistency but mark it as secondary priority.

## Files NOT Modified

- `crates/lisa-core/` — No core type changes needed. Signal file detection is T-008-02's scope.
- `crates/lisa-cli/src/main.rs` — No CLI argument changes.
- `crates/lisa-cli/src/config.rs` — No config schema changes.
- `.gitignore` (project root) — Using `.lisa/.gitignore` instead.

## Module Boundaries

- **templates.rs** owns all generated file content (hook script, gitignore, settings JSON)
- **init.rs** owns the plan-execute lifecycle and file creation
- **lib.rs** owns the Claude spawn command construction and session lifecycle
- **scheduler.rs** owns the `CommandToRun` construction (secondary, currently unused)

## Interface Changes

No public API changes. All modifications are internal to existing functions.

## Ordering

1. templates.rs first — define the content constants/functions
2. init.rs second — wire up new init actions and validation
3. lib.rs third — modify spawn command and session reuse
4. scheduler.rs fourth — consistency update (low priority)

## Test Surface

- `templates.rs`: test hook script content, test settings JSON structure
- `init.rs`: test new init actions are planned, test executable permission, test validate warnings
- `lib.rs`: test env var in command string, test /exit in reuse path
- `scheduler.rs`: test updated command construction
