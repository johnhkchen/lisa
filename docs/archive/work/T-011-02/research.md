# T-011-02 Research: Run lisa loop end-to-end on a real project

## Environment Snapshot

- **lisa**: v0.1.6, installed at `/Volumes/ext1/cargo/bin/lisa`
- **zellij**: v0.43.1, installed at `/opt/homebrew/bin/zellij`
- **claude**: v2.1.49 (Claude Code), installed at `/Users/johnchen/.local/bin/claude`
- **Platform**: macOS (Darwin 25.3.0), ARM64
- **Target project**: the lisa repo itself (`/Users/johnchen/swe/repos/lisa`)

All three binaries are on PATH. Prerequisites for `lisa loop` are met.

## Current Infrastructure State (lisa repo)

### What exists

| Path | Status | Notes |
|------|--------|-------|
| `CLAUDE.md` | Present | Project-specific, hand-maintained |
| `docs/rdspi-workflow.md` | Present | Required by validate |
| `docs/active/tickets/` | Present | ~20+ tickets, many untracked |
| `docs/active/stories/` | Present | Story files present |
| `docs/active/work/` | Present | Work artifacts for several tickets |
| `.lisa/hooks/on-idle.sh` | Present | **Old version**: uses `LISA_TICKET_ID` (legacy), not `LISA_PANE_ID` |
| `.lisa/hooks/on-stop.sh` | **Missing** | Required by validate |
| `.lisa/hooks/on-clear.sh` | **Missing** | Required by validate |
| `.lisa/signals/` | Present, empty | Signal directory exists |
| `.lisa/.gitignore` | Present | Ignores `signals/` |
| `.claude/settings.local.json` | Present | Has `Notification[idle_prompt]` only; **missing `Stop` and `SessionStart` hooks** |
| `.lisa.toml` | **Missing** | Not required (defaults apply), but validate will use defaults |

### Validation gaps

Running `lisa validate` will fail because:

1. **Missing hook scripts**: `on-stop.sh` and `on-clear.sh` do not exist
2. **Missing hook config**: `settings.local.json` lacks `Stop` and `SessionStart` entries
3. **Old idle hook**: uses `LISA_TICKET_ID` env var instead of `LISA_PANE_ID` (won't match current signal-processing code in `lib.rs:622-642`)

Running `lisa init` should fix most of these:
- Will skip existing files (CLAUDE.md, docs/rdspi-workflow.md, on-idle.sh)
- Will create on-stop.sh and on-clear.sh
- Will **update** settings.local.json via `merge_hooks()` to add Stop + SessionStart entries and upgrade the bare-path idle command to guarded form
- Will **not** create .lisa.toml (not critical — defaults apply)
- Will **not** fix the old on-idle.sh since it skips existing files

**Critical issue**: The existing `on-idle.sh` uses `LISA_TICKET_ID` to name signal files (`$TICKET_ID.idle`), but `lib.rs:622-642` expects `pane-$PANE_ID.idle`. Since `lisa init` never overwrites existing hook scripts, this must be fixed manually.

## CLI Flow: `lisa init`

1. `detect_project()` — checks for Cargo.toml/package.json/go.mod/pyproject.toml
2. `plan_init_actions()` — builds a list of CreateDir/CreateFile/UpdateFile/Skip actions
3. Prints the plan, executes if not `--dry-run`
4. Makes hook scripts executable (`chmod 755`)
5. Never overwrites existing files (except settings.local.json which gets merged)

For the lisa repo: will detect Rust project, skip most existing files, create on-stop.sh and on-clear.sh, update settings.local.json.

## CLI Flow: `lisa validate`

Checks in order:
1. Optional tool check (zellij, claude on PATH)
2. CLAUDE.md exists
3. docs/rdspi-workflow.md exists
4. .lisa.toml parse (if present)
5. Hook infrastructure: settings.local.json content, hook scripts exist + executable
6. Directory structure (stories, work — warnings only)
7. Ticket directory exists
8. Ticket scanning with diagnostics (parse errors surfaced)
9. At least one ticket exists
10. Acceptance criteria (warning only)
11. DAG build: cycle detection, missing dependencies, at least one ready ticket

## CLI Flow: `lisa loop`

1. Check zellij + claude on PATH (skipped in dry-run)
2. Check CLAUDE.md + ticket directory exist
3. Verify WASM plugin is embedded (not empty)
4. Write WASM to content-hashed temp path (`/tmp/lisa-plugin-{hash}.wasm`)
5. Generate KDL layout with `2 * max_threads` agent panes
6. Write layout to `.lisa-layout.kdl` in project root
7. `exec()` zellij with `--layout` (replaces process)

### Layout structure

```
tab "lisa" {
  stacked pane 70%:  N agent panes (N = 2 * max_threads)
  plugin pane 30%:   lisa-plugin.wasm with config params
}
```

Plugin receives config via KDL: `ticket_dir`, `story_dir`, `work_dir`, `max_threads`, `auto_advance`, `review_timeout_secs`.

## Plugin Lifecycle (lib.rs)

### Initialization

1. `load()` — parse config from KDL, set up paths with `/host/` prefix, subscribe to events
2. Request permissions (FullHdAccess, RunCommands, ChangeApplicationState, ReadApplicationState)
3. On `PermissionRequestResult(Granted)` — set `permissions_granted`, rebuild DAG, schedule
4. On `PaneUpdate` — discover agent slots (non-plugin panes), set `slots_discovered`

### Scheduling loop

1. **Timer fires** (every `POLL_INTERVAL_SECS = 5s`)
2. `check_transition_signals()` — process `.stopped` / `.cleared` files
3. `rebuild_dag()` — rescan tickets, detect phase changes
4. `check_artifact_advances()` — detect new phase artifacts in work dir
5. `check_idle_signals()` — process `.idle` files
6. Release slots for done tickets, sweep stale slots
7. `schedule_ready_tickets()` — fill idle slots with ready tickets
8. Handle review timeouts (auto_advance or finish-up prompt)
9. Termination check: if all tickets done, log and set `terminated`
10. Re-arm timer

### Session spawning

- **Fresh slot**: `LISA_PANE_ID=N LISA_TICKET_ID=ID claude --dangerously-skip-permissions "prompt"` via `send_line_to_pane()`
- **Reused slot**: Send `/clear`, wait for `.cleared` signal, then send prompt text

### Signal flow

```
Agent finishes → Stop hook → .stopped signal → Plugin sends /clear
/clear processed → SessionStart[clear] hook → .cleared signal → Plugin sends prompt
Agent goes idle → Notification[idle_prompt] hook → .idle signal → Plugin processes phase transition
```

### Key behaviors

- Concurrency capped at `max_threads` running threads (not pane count)
- 2x pane slots allow overlap during transitions
- Slot cooldown: 60s after ticket completion before reuse
- Transition timeouts: 60s for stop signal, 30s for clear signal (fallback to proceed anyway)
- Enter delay: 2s between writing text and pressing Enter (TUI timing)
- Review timeout: configurable (default 240s), sends finish-up prompt or auto-advances

## Ticket ID Resolution in Signals

The plugin identifies tickets from signal files via pane ID:

- Signal filename: `pane-{pane_id}.idle` / `pane-{pane_id}.stopped` / `pane-{pane_id}.cleared`
- Plugin looks up pane_id → agent_slot → ticket_id
- Legacy fallback: `{ticket_id}.idle` (for older hook versions) — but this only exists for `.idle`, not `.stopped`/`.cleared`

The `build_claude_command()` function sets both `LISA_PANE_ID` and `LISA_TICKET_ID` as env vars.

## Existing Tickets Relevant to Testing

There are many tickets in various phases. For a clean test, we need:
- At least 2-3 tickets with `phase: ready` and `status: open`
- At least one dependency chain to test ordering
- Simple enough work that a ticket can complete a full phase cycle

Options:
1. **Use existing tickets**: Many S-012 through S-016 tickets are ready/open. But they're real work — running `lisa loop` on them would create real changes.
2. **Create test tickets**: Safer. Create 2-3 small test tickets with a dependency chain, exercise the full loop, then clean up.

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Old on-idle.sh won't produce correct signal filenames | High | Manually update to use `LISA_PANE_ID` |
| Missing on-stop.sh / on-clear.sh | High | Run `lisa init` to create them |
| `--dangerously-skip-permissions` runs without guardrails | Medium | Use test tickets with limited scope |
| Real tickets get modified during test | Medium | Create isolated test tickets, don't run on real tickets |
| Session reuse deadlock (`.stopped` consumed before WaitingForStop) | Low | Current code sends `/clear` directly for reused slots |
| WASM plugin mismatch (installed v0.1.6 vs local changes) | Medium | Rebuild before testing: `just release` or `just build-cli` |

## Summary of Findings

1. **Infrastructure is partially set up** — needs `lisa init` to create missing hooks, plus manual fix for old on-idle.sh
2. **All tooling is installed** — lisa, zellij, claude all on PATH
3. **The lisa repo is a viable test target** — it has tickets, CLAUDE.md, and rdspi-workflow.md already
4. **Safest approach**: run `lisa init` to fill gaps, manually fix on-idle.sh, create isolated test tickets, then run `lisa loop`
5. **Must rebuild** before testing to ensure embedded WASM matches current code
