# Research: External Project Dogfood (T-009-03)

## Goal

Map all code paths in the `lisa init` → `lisa validate` → `lisa loop` pipeline to
identify issues that would surface when running on an external project.

## CLI Pipeline Overview

### `lisa init` (init.rs)

Creates 14 items in a plan-then-execute pattern:
- **8 directories**: `docs/active/{tickets,stories,work}`, `docs/archive/{tickets,stories,work}`, `.lisa/hooks`, `.lisa/signals`
- **6 files**: `CLAUDE.md`, `docs/rdspi-workflow.md`, `.lisa.toml`, `.lisa/hooks/on-idle.sh`, `.lisa/.gitignore`, `.claude/settings.local.json`

Project detection (`detect.rs`) identifies Rust, Node, Go, Python, or Unknown from marker files.
Generated CLAUDE.md includes project name, type, build/test/lint commands, source layout, and references `docs/rdspi-workflow.md`.

Never overwrites existing files — uses `InitAction::Skip` for anything already present.
Makes `on-idle.sh` executable on Unix.

### `lisa validate` (init.rs)

Structured diagnostic output from T-009-04. Checks:
1. Optional tool checks (zellij, claude on PATH)
2. CLAUDE.md exists
3. `docs/rdspi-workflow.md` exists
4. `.lisa.toml` parses correctly
5. `.claude/settings.local.json` exists and contains `idle_prompt`
6. `.lisa/hooks/on-idle.sh` exists and is executable
7. Optional directory warnings (`docs/active/stories`, `docs/active/work`)
8. Ticket directory exists
9. Tickets parse correctly (frontmatter validation)
10. Acceptance criteria presence (warning)
11. DAG builds (no cycles, no missing deps)
12. At least one ready ticket

Exit 0 on pass, exit 1 on errors.

### `lisa loop` (loop_cmd.rs)

1. Checks for `zellij` and `claude` on PATH (skipped in dry-run)
2. Verifies CLAUDE.md and ticket directory exist
3. Checks embedded WASM plugin is non-empty
4. Writes WASM to `/tmp/lisa-plugin.wasm`
5. Generates KDL layout with agent pane slots
6. Execs `zellij --layout .lisa-layout.kdl`

Dry-run mode scans tickets, builds DAG, prints summary and generated layout.

### Plugin (lib.rs)

- Discovers agent pane slots from PaneManifest
- Rebuilds DAG every 5 seconds from ticket files
- Schedules ready tickets into idle slots
- Detects phase artifacts (research.md, design.md, etc.) to advance phases
- Detects idle signals (`.lisa/signals/{ticket_id}.idle`) to advance implement→review
- Session reuse: sends `/exit` then re-launches Claude Code with new env var

## Critical Issues Found

### Issue 1: rdspi-workflow.md Path Mismatch (BUG)

**Plugin prompt** (lib.rs:30) hardcodes:
```
docs/knowledge/rdspi-workflow.md
```

**`lisa init`** (init.rs:70) creates the file at:
```
docs/rdspi-workflow.md
```

**`lisa validate`** (init.rs:328) checks for:
```
docs/rdspi-workflow.md
```

**Generated CLAUDE.md** (templates.rs:113) references:
```
docs/rdspi-workflow.md
```

The plugin prompt path `docs/knowledge/rdspi-workflow.md` is the lisa-internal path.
In the lisa repo itself, `docs/rdspi-workflow.md` is a symlink to `docs/knowledge/rdspi-workflow.md`.
On external projects, `docs/knowledge/` doesn't exist. Agents will be told to read a
file that doesn't exist.

**Fix**: Change `ticket_prompt()` in lib.rs to reference `docs/rdspi-workflow.md`.
This is the canonical path that `lisa init` creates and `lisa validate` checks.

### Issue 2: Idle Hook Preconditions

The on-idle hook (`on-idle.sh`) requires:
- `LISA_TICKET_ID` env var set — handled by `build_claude_command()` ✓
- `.lisa/signals/` directory exists — created by `lisa init` ✓
- Claude Code settings have `idle_prompt` notification — created by `lisa init` ✓

No issues found here. Pipeline is consistent.

### Issue 3: Session Reuse Path

When a slot is reused, the plugin sends `/exit` to the existing Claude Code session,
then queues the new `claude` command via `pending_pane_writes` with a short timer delay.
This depends on:
- `/exit` completing fast enough for the shell prompt to return
- The flush delay (0.5s) being sufficient

This is a runtime concern, not a code bug, but should be monitored during dogfood.

## File Inventory for External Project

After `lisa init`, an external project should have:

| Path | Purpose |
|------|---------|
| `CLAUDE.md` | Project context for agents |
| `docs/rdspi-workflow.md` | RDSPI workflow definition |
| `.lisa.toml` | Lisa configuration |
| `docs/active/tickets/` | Ticket files |
| `docs/active/stories/` | Story files |
| `docs/active/work/` | Work artifacts |
| `docs/archive/tickets/` | Archived tickets |
| `docs/archive/stories/` | Archived stories |
| `docs/archive/work/` | Archived work |
| `.lisa/hooks/on-idle.sh` | Idle signal hook |
| `.lisa/signals/` | Signal file directory |
| `.lisa/.gitignore` | Ignores signals/ |
| `.claude/settings.local.json` | Claude Code hook config |

## Additional CLI Commands

- `lisa status` — Prints DAG summary, execution waves, ready tickets
- `lisa setup-guide` — LLM-friendly setup instructions

## Test Suite

94 tests pass across all 3 crates:
- lisa-core: types, ticket parsing, DAG computation
- lisa-plugin: scheduling, UI, slot management, artifact detection, idle signals
- lisa-cli: init, validate, detect, templates, config, loop, status, setup-guide

## Summary

One blocking bug (Issue 1: rdspi-workflow.md path mismatch) must be fixed before
external project dogfood will work correctly. All other pipeline components appear
consistent and well-tested. The fix is a one-line change in `lib.rs:ticket_prompt()`.
