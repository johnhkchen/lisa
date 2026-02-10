# T-008-01 Design: Hook Infrastructure for Idle Signal

## Decision 1: Session Reuse and Ticket ID Tracking

### Options

**A. Env var only, no session reuse fix**
Set `LISA_TICKET_ID` on initial launch. Accept that on session reuse the env var is stale. The hook script writes a signal with the original ticket ID, which is wrong.

**B. Kill and re-launch Claude Code for each ticket**
Never reuse sessions — always launch fresh. Env var is always correct. Downside: slower, loses Claude Code's session warmup.

**C. Pane-to-ticket mapping file**
Write `.lisa/signals/slot-{pane_id}.ticket` with the current ticket ID whenever a slot is assigned. The hook script reads the mapping file instead of the env var. The plugin writes this file on every schedule, the hook reads it on idle.

**D. Env var on fresh launch + mapping file for reuse**
Use `LISA_TICKET_ID` env var for fresh launches. Additionally, write a mapping file that the hook script checks first. If the mapping file exists and is newer, use it; otherwise fall back to env var.

### Decision: Option C — Pane-to-ticket mapping file

Rationale:
- Simple, reliable, works for both fresh and reused sessions
- The plugin already knows the pane_id ↔ ticket_id mapping (in `agent_slots`)
- Writing a small file is cheap and atomic (`fs::write` is atomic on ext4/APFS for small writes)
- The hook script just reads one file — no env var dependency at all
- Avoids the complexity of option D's dual-source fallback

However, there's a complication: the hook script runs inside the Claude Code process, which doesn't know its own Zellij pane ID. The hook can't look up `slot-{pane_id}.ticket` without knowing which pane it's in.

**Revised approach:** Instead of pane-based mapping, use a simpler scheme:
- The plugin writes `.lisa/signals/active-tickets.json` (or one file per slot) whenever it assigns a ticket to a slot
- But the hook still needs to know *which* session it belongs to

**Final approach: Env var is sufficient for the current architecture.**

Re-examining the session reuse flow: when a session is reused via `/clear` + new prompt, Claude Code is still running in the same shell process. The `LISA_TICKET_ID` env var from the original launch persists. But we can update it by sending an `export LISA_TICKET_ID=...` command to the pane before sending the prompt.

Since `send_line_to_pane()` sends text to a terminal, we can send shell commands. Before sending the `/clear` command, we can send `export LISA_TICKET_ID={new_ticket_id}` to the shell. But wait — Claude Code is running interactively, so the shell is not available. The `export` would be typed into Claude Code's input, not a shell.

**Actually, env var works perfectly for fresh launches, and session reuse can kill + relaunch.** But the ticket says the plugin already reuses sessions to avoid overhead. Let me reconsider.

**Simplest correct solution:** For fresh pane launches, set the env var via the shell command. For session reuse, exit Claude Code first (`/exit`), then launch fresh with the new env var. This is slightly slower than `/clear` but guarantees correct env vars.

**Even simpler:** Just always kill and re-launch. The `/clear` reuse path saves ~2 seconds of Claude Code startup. Not worth the complexity of mapping files. But the reuse path already exists and works — we shouldn't break it.

### Revised Decision: Env var + re-launch on reuse

- Fresh pane: `LISA_TICKET_ID={id} claude --dangerously-skip-permissions "..."`
- Reused pane: send `/exit` first, wait briefly, then launch fresh with new env var
- The hook script uses `$LISA_TICKET_ID` which is always correct

This replaces the current `/clear` reuse path with `/exit` + fresh launch. The env var is always correct because it's set in the shell command that launches Claude Code.

## Decision 2: settings.local.json Handling

### Options

**A. Create-only (skip if exists)**
Match init's existing pattern. If `.claude/settings.local.json` exists, skip it with a warning.

**B. New `InitAction::MergeJson` variant**
Add a merge action that reads existing JSON, adds the hooks entry, and writes back.

**C. Separate merge function, still `CreateFile` for new**
If file doesn't exist, create it. If it exists, run a merge function that adds the hook if missing. This is a special case in `run_init` execution, not a new `InitAction`.

### Decision: Option A — Create-only, with validation warning

Rationale:
- Maintains init's "never overwrite" invariant — no risk of corrupting user settings
- JSON merging is error-prone (preserving comments, formatting, unknown keys)
- `settings.local.json` is not a file users typically hand-edit — it's usually auto-generated
- If the file already exists, `lisa validate` can warn that the idle_prompt hook may be missing
- Users can copy the hook config from `lisa init --dry-run` output or docs

If the file exists, `plan_init_actions` returns `Skip` with reason "already exists — verify hooks config". Validate warns if the hook is missing.

## Decision 3: Hook Script Content

The script should be minimal:
```bash
#!/bin/sh
# Lisa idle signal hook — called by Claude Code on idle_prompt notification
# Writes a signal file so the plugin knows this session finished its work.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_TICKET_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/$LISA_TICKET_ID.idle"
fi
```

Key choices:
- `/bin/sh` not `/bin/bash` — more portable
- `mkdir -p` is idempotent — no error if dir exists
- Write ISO timestamp, not just `touch` — useful for debugging
- Guard on `$LISA_TICKET_ID` being set — no-op if env var missing
- Exit 0 implicitly — hook is observational, never blocks

## Decision 4: .gitignore Entries

Add to the project's `.gitignore` during `lisa init`:
- `.lisa/signals/` — ephemeral signal files, never committed

The `.lisa/hooks/` directory should be committed — it's generated infrastructure. The hook script is deterministic and project-agnostic.

However, `.gitignore` is not currently managed by `lisa init`. Adding a new `InitAction` for `.gitignore` is complex (append vs create). Simplest: add `.lisa/signals/` to the project's `.gitignore` as a new `CreateFile` or append action.

**Decision:** Add a `.lisa/.gitignore` file inside the `.lisa/` directory that ignores `signals/`. This avoids modifying the project's root `.gitignore` (which init never touches). The `.lisa/.gitignore` is a new file that init can create with `CreateFile`.

Wait — that only works if `.lisa/` itself is committed. If `.lisa/` is gitignored at the project root, the nested `.gitignore` is moot. Better to just document that `.lisa/signals/` should be gitignored, and have `run_init` create a `.lisa/.gitignore` containing `signals/`.

## Decision 5: Env Var Injection in Plugin Spawn

Modify `build_claude_command()` in lib.rs to include the env var:

```rust
fn build_claude_command(ticket_dir: &Path, ticket_id: &str) -> String {
    format!(
        "LISA_TICKET_ID={} claude --dangerously-skip-permissions \"{}\"",
        ticket_id,
        ticket_prompt(ticket_dir, ticket_id)
    )
}
```

For session reuse, change the flow from `/clear` + prompt to `/exit` + new launch:
```rust
if self.agent_slots[slot_idx].has_session {
    send_line_to_pane("/exit", PaneId::Terminal(pane_id));
    // Queue the fresh launch after a delay
    let cmd = build_claude_command(&host_ticket_dir, &ticket_id);
    self.pending_pane_writes.push((pane_id, cmd));
} else {
    let cmd = build_claude_command(&host_ticket_dir, &ticket_id);
    send_line_to_pane(&cmd, PaneId::Terminal(pane_id));
    self.agent_slots[slot_idx].has_session = true;
}
```

The `has_session` flag remains true after `/exit` + re-launch because the pane still has a running shell. The flag tracks "has this pane ever had Claude Code started in it" which is always true after first use. The re-launch command goes through `pending_pane_writes` with the existing flush delay.

## Summary of Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Env var + /exit on reuse | Always-correct ticket ID, minimal complexity |
| 2 | Create-only settings.json | Maintains never-overwrite invariant |
| 3 | /bin/sh script with timestamp | Portable, debuggable, idempotent |
| 4 | .lisa/.gitignore with `signals/` | Avoids modifying project root .gitignore |
| 5 | Inline env var in shell command | Works with terminal pane launch model |
