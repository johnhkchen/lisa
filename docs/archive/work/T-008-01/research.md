# T-008-01 Research: Hook Infrastructure for Idle Signal

## Problem Context

The plugin detects phase completion via artifact files (e.g., `research.md` appearing in the work directory). This breaks down for the Implement phase — `progress.md` is a living document, not a completion marker. There is no signal for when an agent finishes working and returns to idle. Phase transitions currently rely on manual frontmatter edits or the 'd' key in the dashboard.

Claude Code's `idle_prompt` notification hook provides the missing signal. When a session goes idle ("Awaiting User Input"), a `Notification` event fires. A hook script can write a signal file that the plugin detects.

## Codebase Mapping

### 1. `lisa init` — init.rs (crates/lisa-cli/src/init.rs)

The init command uses a **plan-then-execute** pattern:

- `plan_init_actions(root, project) -> Vec<InitAction>` builds a list of actions (CreateDir, CreateFile, Skip)
- `run_init(root, dry_run)` prints the plan, then executes if not dry-run
- **Never overwrites** existing files — uses `Skip` with reason "already exists"
- Currently creates: 6 directories, CLAUDE.md, docs/rdspi-workflow.md, .lisa.toml

New actions needed:
- CreateDir: `.lisa/hooks/`, `.lisa/signals/`
- CreateFile: `.lisa/hooks/on-idle.sh` (must be marked executable)
- CreateFile or merge: `.claude/settings.local.json`

**Key constraint:** `InitAction::CreateFile` uses `fs::write()` — no chmod. Making the script executable requires a new action variant or post-write chmod. On Unix, `std::fs::set_permissions` with mode 0o755 would work.

**Key constraint:** `.claude/settings.local.json` may already exist with user content. The ticket says "generates (or updates)" — this means read-parse-merge, not overwrite. Need to handle: file doesn't exist (create), file exists without hooks (add hooks key), file exists with hooks (merge the Notification array entry).

### 2. Claude spawn command — lib.rs (crates/lisa-plugin/src/lib.rs:38-43)

Two paths for launching Claude Code sessions:

**Fresh pane (no existing session):**
```rust
fn build_claude_command(ticket_dir: &Path, ticket_id: &str) -> String {
    format!(
        "claude --dangerously-skip-permissions \"{}\"",
        ticket_prompt(ticket_dir, ticket_id)
    )
}
```
This generates a shell command string sent to the pane via `send_line_to_pane()`. Since it's a shell string, env var injection is straightforward:
```
LISA_TICKET_ID={id} claude --dangerously-skip-permissions "..."
```

**Existing session (reuse):**
```rust
fn build_claude_prompt(ticket_dir: &Path, ticket_id: &str) -> String {
    ticket_prompt(ticket_dir, ticket_id)
}
```
When reusing a session, `/clear` is sent first, then just the prompt text. The env var was set when the session was first launched, but after `/clear` and a new prompt, the session is working on a new ticket. The `LISA_TICKET_ID` env var needs to be updated.

**Problem:** There is no way to change an environment variable in an already-running process. When a slot is reused for a different ticket, the original `LISA_TICKET_ID` value persists. Options:
- Accept this limitation — the hook script can read ticket ID from another source
- Kill and re-launch Claude Code for each ticket (defeats the purpose of session reuse)
- Write a secondary file (`.lisa/signals/current-ticket-{pane_id}`) that the hook reads instead of the env var
- Use the pane_id to look up the current ticket assignment in a shared mapping file

### 3. Scheduler spawn — scheduler.rs (crates/lisa-plugin/src/scheduler.rs:359-397)

The `spawn_claude_session()` method uses zellij's `open_command_pane_floating()` with `CommandToRun`. This struct has `path`, `args`, `cwd` but **no `env` field**. The ticket already identifies this — the workaround is to spawn via `sh -c "LISA_TICKET_ID=... claude ..."`.

However, **this code path is not currently used** in the active flow. The plugin uses `send_line_to_pane()` in `schedule_ready_tickets()` (lib.rs:284-374) instead of `open_command_pane_floating()`. The scheduler's `spawn_claude_session()` is dead code relative to the current plugin flow.

### 4. Session reuse flow — lib.rs:326-338

```rust
if self.agent_slots[slot_idx].has_session {
    send_line_to_pane("/clear", PaneId::Terminal(pane_id));
    let cmd = build_claude_prompt(&host_ticket_dir, &ticket_id);
    self.pending_pane_writes.push((pane_id, cmd));
} else {
    let cmd = build_claude_command(&host_ticket_dir, &ticket_id);
    send_line_to_pane(&cmd, PaneId::Terminal(pane_id));
    self.agent_slots[slot_idx].has_session = true;
}
```

For session reuse: the env var injection only works on the initial `build_claude_command` (fresh launch). On reuse, only a prompt string is sent into an existing Claude session. The env var is baked into the process environment from the original launch.

### 5. .gitignore — project root

Current `.gitignore`:
```
/target
.DS_Store
.lisa-layout.kdl
.ralph-commit.lock
.obsidian/
```

Need to add `.lisa/signals/`. The `.lisa/hooks/` directory should likely be committed (it's generated infrastructure, like `.github/workflows/`), so only signals should be ignored.

### 6. Claude Code hooks format

The ticket specifies the hook format:
```json
{
  "hooks": {
    "Notification": [{
      "matcher": "idle_prompt",
      "hooks": [{
        "type": "command",
        "command": ".lisa/hooks/on-idle.sh"
      }]
    }]
  }
}
```

This goes in `.claude/settings.local.json`. The `.claude/` directory is Claude Code's per-project settings directory. `settings.local.json` is the local (non-committed) settings file.

### 7. Validation — init.rs:169-309

`run_validate()` checks for required files and structure. New checks needed:
- Warn if `.claude/settings.local.json` doesn't exist or lacks the idle_prompt hook
- Warn if `.lisa/hooks/on-idle.sh` doesn't exist or isn't executable

## Key Constraints and Observations

1. **init never overwrites** — this is an established invariant with tests (`test_run_init_never_overwrites`). The settings.local.json merge must respect this principle while still being able to add hooks to an existing file.

2. **Session reuse vs env var** — the biggest design challenge. When a pane is reused for a different ticket, the env var doesn't change. The hook script needs a reliable way to identify which ticket is currently running in the session.

3. **Cross-platform** — init.rs runs on the host (not WASM), so Unix chmod is available. The hook script is a shell script, so it's Unix-only, which is fine since Zellij is Unix-only.

4. **WASI sandbox** — the plugin reads from `/host/` mount. Signal files written to `.lisa/signals/` on the host appear at `/host/.lisa/signals/` in the plugin's filesystem. The plugin already knows about `/host/` prefix stripping.

5. **Test count** — currently 88 tests. New tests needed for: hook config generation, signal script generation, settings.json merge logic, env var injection in spawn command, gitignore entry.

## Files That Will Be Modified

- `crates/lisa-cli/src/init.rs` — new init actions for hooks, signals, settings.json
- `crates/lisa-cli/src/templates.rs` — hook script content, settings.json template
- `crates/lisa-plugin/src/lib.rs` — env var injection in `build_claude_command()`
- `.gitignore` — add `.lisa/signals/`

## Open Questions for Design Phase

1. How to handle the session reuse env var problem — dedicated mapping file vs accepting stale env vars?
2. Should `.claude/settings.local.json` merge be a new `InitAction` variant (MergeJson) or handled as a special case in CreateFile?
3. Should the hook script be a static string or take any runtime configuration?
4. Should `on-idle.sh` be `.lisa/hooks/on-idle.sh` (committed) or somewhere ephemeral?
