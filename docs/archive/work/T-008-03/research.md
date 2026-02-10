# T-008-03 Research: Dogfood Idle Signal Phase Transitions

## What This Ticket Is

This is a dogfood/integration test ticket. The goal is to run `lisa loop` on a
real project and verify that idle signal phase transitions (implemented in
T-008-01 and T-008-02) work end-to-end without manual intervention.

## Current State of the Implementation

### Hook Infrastructure (T-008-01 — Done)

1. **`lisa init` generates hook infrastructure:**
   - `.lisa/hooks/on-idle.sh` — shell script that reads `LISA_TICKET_ID` env
     var and writes `.lisa/signals/{ticket_id}.idle` with a UTC timestamp
   - `.lisa/signals/` directory — gitignored via `.lisa/.gitignore`
   - `.claude/settings.local.json` — configures Claude Code's `Notification`
     hook with `idle_prompt` matcher pointing to the on-idle script
   - 14 total init actions (8 dirs + 6 files)

2. **Env var injection in plugin:**
   - `build_claude_command()` (lib.rs:39-45) wraps the Claude launch with
     `LISA_TICKET_ID={id} claude --dangerously-skip-permissions "..."`
   - On session reuse, the plugin sends `/exit` then re-launches with the
     new ticket's env var so `LISA_TICKET_ID` is correct

3. **Validation:**
   - `lisa validate` checks for `.claude/settings.local.json` and
     `.lisa/hooks/on-idle.sh` (warns if missing, doesn't error)

### Idle-Aware Phase Advancement (T-008-02 — Done)

1. **`check_idle_signals()` in lib.rs:508-645:**
   - Scans `.lisa/signals/` directory for `*.idle` files every poll cycle
   - Parses ticket ID from filename (e.g., `T-001.idle` -> `T-001`)
   - Deletes signal file immediately after reading (prevents re-trigger)
   - Only processes signals for running threads

2. **Phase advancement rules:**
   - **Implement**: idle signal alone advances to Review, parks thread
   - **Research/Design/Structure/Plan**: idle signal + artifact advances to
     next phase
   - **Idle without artifact**: generates alert in `idle_alerts` vec, surfaces
     via attention banner
   - **Ready/Review/Done**: signal cleaned up, no action

3. **Integration into poll_tick():**
   - `check_idle_signals()` runs after `check_artifact_advances()` and before
     `evaluate_health()` / `rebuild_dag()`
   - Coexists with existing artifact-based detection (both are triggers)

4. **Test coverage:** 102 tests all passing, includes specific tests for:
   - Implement -> Review via idle signal
   - Research -> Design via idle signal + artifact
   - Idle-without-artifact alert generation
   - Signal cleanup after processing
   - Stale signal for non-running thread (ignored)
   - UI state mapping of idle alerts

## Current Lisa Repo Setup

- `.claude/settings.local.json` exists but contains only permissions config,
  NOT the idle_prompt notification hook
- `.lisa/hooks/` directory does NOT exist
- `.lisa/signals/` directory does NOT exist
- `.lisa/.gitignore` does NOT exist
- `docs/active/tickets/` has active S-008 tickets (T-008-01 done, T-008-02 done, T-008-03 ready)

## What Needs to Happen for Dogfooding

1. Set up hook infrastructure on this repo (either via `lisa init` or manually)
2. Create a small test project OR use lisa itself with a controlled ticket set
3. Run `lisa loop` and observe phase transitions
4. Document findings

## Key Risk Areas

- **settings.local.json merge conflict**: Lisa's repo already has a
  `settings.local.json` with permissions. `lisa init` skips existing files.
  Need to manually merge the idle_prompt hook config into the existing file.

- **WASM plugin must be current**: The embedded WASM in the CLI binary must
  include T-008-02's `check_idle_signals()` code. Need `just build-cli` or
  `just release` before running `lisa loop`.

- **Signal directory creation**: The on-idle.sh script does `mkdir -p` so
  signals dir is created on first idle event. But the plugin's
  `check_idle_signals()` gracefully handles missing directory (returns early).

- **Artifact path resolution**: Inside WASM, work_dir is `/host/docs/active/work`.
  Artifacts must exist at that path. The signal dir is also under `/host/`.

- **Session reuse LISA_TICKET_ID**: When a slot is reused for a new ticket,
  the plugin sends `/exit` then relaunches. The new session must pick up the
  new LISA_TICKET_ID. Timing between `/exit` and the new command matters.

## Constraints

- This is a **dogfood** ticket — the deliverable is a test run + documented
  results, not new code.
- The acceptance criteria require at least one ticket completing Research
  through Review without manual phase intervention.
- Review -> Done still requires manual 'd' press (by design).
