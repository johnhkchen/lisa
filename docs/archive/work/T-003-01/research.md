# T-003-01 Research: validate-claude-spawn

## Overview

This ticket validates that the Claude command construction and pane-to-ticket correlation work correctly when Lisa spawns Claude Code sessions.

## Two Diverged Spawn Implementations

There are **two independent implementations** of Claude session spawning:

### 1. `lib.rs::schedule_ready_tickets()` (lines 91-158) — ACTIVE

This is the code path actually used by the running plugin. It:

- Computes `host_ticket_path` by stripping `/host/` prefix from `config.ticket_dir` (correct: commands run on host, not in WASI sandbox)
- Uses `open_command_pane` (tiled, not floating) for stacked layout
- Passes the prompt as a positional arg (interactive mode, no `--print`)
- Sets `context = BTreeMap::from([("ticket_id", ticket_id)])` — only ticket_id
- Creates `Thread::new(ticket_id, 0)` — pane_id hardcoded to 0

Prompt template:
```
Read the ticket at {host_ticket_path}, the project context in CLAUDE.md, and the RDSPI workflow in docs/knowledge/rdspi-workflow.md. Start from the current phase indicated in the ticket frontmatter.
```

Path to RDSPI: `docs/knowledge/rdspi-workflow.md` — **CORRECT** (matches actual file location)

### 2. `scheduler.rs::build_claude_command()` (lines 409-428) — DEAD CODE

This is inside the `Scheduler` struct which is never instantiated from `lib.rs`. It:

- Uses `self.config.tickets_dir.join(format!("{}.md", ticket_id))` for ticket path
- Includes `--print` flag (non-interactive, print-and-exit)
- Called from `spawn_claude_session()` which uses `open_command_pane_floating`
- Sets context with both `ticket_id` and `command_id`

Prompt template:
```
Read the ticket at {ticket_path}, the project context in CLAUDE.md, and the RDSPI workflow in docs/rdspi-workflow.md. Start from the current phase indicated in the ticket frontmatter.
```

Path to RDSPI: `docs/rdspi-workflow.md` — **BUG** (file is at `docs/knowledge/rdspi-workflow.md`)

## pane_to_ticket Mapping Is Broken

The `State.pane_to_ticket: HashMap<u32, TicketId>` (lib.rs:45) is **never populated**.

### Write path (missing)

- `schedule_ready_tickets()` creates threads with `pane_id: 0`
- `PaneUpdate` handler (lib.rs:333) is a no-op: `let _ = pane_manifest;`
- No code ever calls `self.pane_to_ticket.insert(...)`

### Read path (always fails)

- `handle_pane_exited()` (lib.rs:162) does `self.pane_to_ticket.remove(&pane_id)` — always returns `None`
- Thread status never transitions to Completed/Failed on pane exit
- All pane exits are silently lost

### Available but unused: CommandPaneExited context

- Event handler (lib.rs:314): `Event::CommandPaneExited(pane_id, exit_code, _context)`
- The `_context` BTreeMap echoes back the context passed to `open_command_pane`
- It contains `ticket_id` (set at lib.rs:128) but is discarded with `_context`

## Relevant Types

- `Thread` (types.rs:268): `ticket_id`, `pane_id`, `current_phase`, `started_at`, `status`
- `ThreadStatus` (types.rs:250): `Running`, `Parked`, `Completed`, `Failed`
- `PluginConfig` (types.rs:348): `ticket_dir`, `story_dir`, `work_dir`, `max_threads`, `auto_advance`
- `SchedulerConfig` (scheduler.rs:189): `tickets_dir`, `stories_dir`, `work_dir`, `repo_root`, `max_concurrent_threads`, `claude_binary`

## open_command_pane API

From zellij_tile prelude:

- `open_command_pane(CommandToRun, BTreeMap<String, String>)` — tiled pane
- `open_command_pane_floating(CommandToRun, Option<FloatingPaneCoordinates>, BTreeMap<String, String>)` — floating pane
- `CommandToRun { path: PathBuf, args: Vec<String>, cwd: Option<PathBuf> }`
- Context BTreeMap is echoed back in `Event::CommandPaneExited(pane_id, exit_code, context)`

## Existing Test Coverage

### scheduler.rs tests (7 tests)

- `test_scheduler_creation` — basic creation
- `test_thread_lifecycle` — park/resume/complete
- `test_commit_lock_path` — lock file path
- `test_ticket_work_dir` — work dir path
- `test_spawn_thread_capacity` — capacity limits
- `test_handle_pane_exit` — exit handling
- `test_phase_artifact_check` — artifact existence

**Missing**: No test for `build_claude_command()` (it's private)

### lib.rs tests (3 tests)

- `test_phase_to_ui_phase` — phase mapping
- `test_ticket_status_to_ui_status` — status mapping
- `test_activity_event_to_ui_entry` — activity log conversion

**Missing**: No test for `schedule_ready_tickets()` command construction

## Key Findings Summary

| Issue | Location | Severity |
|-------|----------|----------|
| Wrong RDSPI path in scheduler.rs | scheduler.rs:422 | Bug (dead code, low impact) |
| pane_to_ticket never populated | lib.rs | Critical — pane exits silently lost |
| CommandPaneExited context ignored | lib.rs:314 | Critical — the fix data is available |
| Thread pane_id always 0 | lib.rs:149 | Linked to pane_to_ticket issue |
| Scheduler struct is dead code | scheduler.rs | Design — two parallel implementations |
| No test for command construction | scheduler.rs / lib.rs | Gap — ticket's AC #3 |

## Files Involved

- `crates/lisa-plugin/src/scheduler.rs` — Scheduler struct, build_claude_command, tests
- `crates/lisa-plugin/src/lib.rs` — State struct, schedule_ready_tickets, PaneUpdate/CommandPaneExited handlers
- `crates/lisa-core/src/types.rs` — Thread, PluginConfig, ActivityEvent
- `docs/knowledge/rdspi-workflow.md` — actual RDSPI file location
