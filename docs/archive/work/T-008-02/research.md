# T-008-02 Research: Idle-Aware Phase Advancement

## Problem Context

The plugin detects phase completion via artifact files (`research.md`, `design.md`, etc.) appearing in the work directory. Two gaps exist:

1. **Implement -> Review**: The `check_artifact_advances()` method explicitly skips Implement phase (lib.rs:428) because `progress.md` is a living document, not a completion marker. There is no current mechanism to advance Implement -> Review automatically.

2. **Earlier phases**: Artifact detection works but requires the artifact to exist at poll time. An agent that goes idle without producing the expected artifact gets no feedback — it just appears "stuck" after the threshold.

T-008-01 built the hook infrastructure: the `on-idle.sh` hook writes `.lisa/signals/{ticket_id}.idle` when Claude Code goes idle. This ticket adds the plugin-side logic to read those signals and act on them.

## Codebase Mapping

### 1. Signal file format (from T-008-01)

The hook script writes:
```
.lisa/signals/{ticket_id}.idle
```
Content is an ISO timestamp. The file's presence is the signal; content is for debugging.

Inside the WASI sandbox, this path resolves to `/host/.lisa/signals/{ticket_id}.idle`.

### 2. poll_tick() — lib.rs:617-706

The main poll loop runs every 5 seconds:
1. `check_artifact_advances()` — scan for phase artifacts
2. `evaluate_health()` — log Healthy->Stuck transitions
3. `detect_stale_threads()` — hard-timeout at 2x threshold
4. `rebuild_dag()` — rescan ticket files
5. Detect done tickets — mark threads complete, release slots
6. Sync thread phases with DAG
7. `sweep_stale_slots()` — safety sweep
8. `audit_threads()` — remove orphaned threads
9. `schedule_ready_tickets()` — fill idle slots
10. Log poll summary, check for termination

Idle signal checking should slot in early, alongside or after `check_artifact_advances()` (step 1). This ensures signals are processed before DAG rebuild and scheduling.

### 3. check_artifact_advances() — lib.rs:415-488

This method:
- Iterates running threads
- Skips Implement phase explicitly (line 428)
- Checks if the artifact file exists for the current phase
- If found, computes next phase, updates frontmatter via `ticket::update_ticket_phase()`
- Logs events, updates thread phase, parks if advancing to Review

Key observations:
- Uses `self.config.work_dir` to find artifacts
- Updates the ticket file on disk via `ticket::update_ticket_phase()`
- Parks thread (sets `ThreadStatus::Parked`) when advancing to Review
- The existing pattern handles artifact existence check, phase advancement, and parking — new idle signal logic should follow the same pattern

### 4. Phase advancement rules

From `types.rs` Phase::next():
```
Research -> Design
Design -> Structure
Structure -> Plan
Plan -> Implement
Implement -> Review
```

Artifact filenames (Phase::artifact_filename()):
- Research: "research.md"
- Design: "design.md"
- Structure: "structure.md"
- Plan: "plan.md"
- Implement: "progress.md" (but skipped in check_artifact_advances)
- Review, Ready, Done: None

### 5. Alert mechanism — lib.rs:1295-1320, ui.rs:157-173

Health alerts are built in `to_ui_state()` from thread health status. They go into `ui::PluginState.alerts` which renders in the attention banner.

Current alert types: `AlertType::Stuck` and `AlertType::Failed`.

For idle-without-artifact alerts, two approaches:
- **Option A**: Add a new `AlertType::IdleWithoutArtifact` variant in ui.rs
- **Option B**: Reuse `ActivityEvent::Warning` in the activity log

The ticket says "add an alert to `state.alerts`" — but `State` doesn't have an `alerts` field. Alerts are computed on-the-fly in `to_ui_state()` from thread health. The ticket's intent is to surface idle-without-artifact in the attention banner.

To do this we could either:
1. Store idle-without-artifact alerts in State and include them in to_ui_state()
2. Add a new health-like status to threads that to_ui_state() can detect

Option 1 is simpler — add a `Vec<ui::HealthAlert>` field to State, populate it in the idle signal handler, clear them after processing. Then merge them into alerts in to_ui_state().

### 6. Thread lookup

The idle signal provides a ticket_id (from the filename). To find the corresponding thread:
- `self.threads.get(&ticket_id)` — returns the Thread if one exists
- `self.threads.get(&ticket_id).map(|t| t.current_phase)` — gets current phase

The thread must exist and be Running for the signal to be meaningful. If the thread doesn't exist or isn't Running, the signal should be ignored (cleaned up silently).

### 7. Signal file I/O in WASI

The plugin runs inside the WASI sandbox. The host filesystem is at `/host/`. The `.lisa/signals/` directory on the host appears at `/host/.lisa/signals/` in the plugin.

For scanning signals:
- `std::fs::read_dir("/host/.lisa/signals/")` — works but slow in WASI
- `scan_host_folder` — zellij API for fast directory listing, but returns a complex structure
- Since signals are few files (max = max_threads), `std::fs::read_dir` is fine

For deleting processed signals:
- `std::fs::remove_file()` — works in WASI for files under /host/ with FullHdAccess

The plugin doesn't currently request `PermissionType::FullHdAccess` (lib.rs:1110-1114). It requests WriteToStdin, ChangeApplicationState, ReadApplicationState. For `std::fs::remove_file()` to work on `/host/` paths, we may need FullHdAccess. However, the plugin already reads ticket files via `std::fs::read_to_string()` and `std::fs::read_dir()` in `scan_tickets()`, so read access to `/host/` is already available. Write access for signal deletion should also work since the plugin writes to `/host/.lisa-state-dump.txt` (lib.rs:982).

### 8. Existing parking logic

When a thread reaches Review phase:
- `check_artifact_advances()` sets `thread.park()` (lib.rs:484)
- The slot is NOT released when parking — the thread stays assigned to the slot
- The parked thread appears in the attention banner for human review

For Implement -> Review via idle signal:
- Should also park the thread (consistent with existing behavior)
- Should also release the slot so another ticket can be scheduled (the ticket says "park thread" but the parked thread currently holds its slot)

Wait — re-reading the ticket: "Advance to Review, park thread." Looking at the current code: when `check_artifact_advances()` parks a thread at Review, it does NOT release the slot. The thread stays in the slot. This is intentional — the session might continue if the human approves.

But for Implement -> Review, the agent has genuinely finished. The idle signal means "I'm done coding." Parking without releasing the slot would waste a slot. The ticket says "park thread" which is the existing behavior, but we should also release the slot.

Actually, looking more carefully: the "mark done" modal (lib.rs:1030-1080) is the mechanism for handling Review. The human presses 'd', selects the ticket, and it gets marked Done. The slot is released when the ticket moves to Done. So parking at Review is a holding state until human review.

For T-008-02, the behavior should be: advance to Review, park thread, release slot. This frees the slot for another ticket while the ticket waits for human review.

### 9. Config — signal directory path

The plugin config (`PluginConfig`) has `ticket_dir`, `story_dir`, `work_dir` but no signal directory. The signal path `.lisa/signals/` is hardcoded relative to the project root.

Inside WASI, the project root is at `/host/`. So the signal directory is `/host/.lisa/signals/`.

We could either:
- Hardcode `/host/.lisa/signals/` in the signal scanning code
- Derive it from the config (but .lisa/ is not configurable currently)
- Add a `signal_dir` field to PluginConfig

Since `.lisa/` is a conventional fixed path (like `.git/`), hardcoding is appropriate. The path should be computed once and stored, similar to how `work_dir` is prefixed with `/host/` in `load()`.

## Key Constraints

1. **WASI sandbox** — signals at `/host/.lisa/signals/`, deletion via `std::fs::remove_file()`
2. **Poll-based** — signals checked every 5 seconds, no filesystem events needed
3. **Coexist with artifact detection** — idle signals are additional, not replacement
4. **Thread must be Running** — ignore signals for non-existent or non-running threads
5. **Implement phase is special** — idle signal alone triggers advance (no artifact check)
6. **Earlier phases need both** — idle signal + artifact must both be present
7. **Alert on idle-without-artifact** — surface in attention banner, not just activity log
8. **Clean up signals** — delete after processing to prevent re-triggering

## Files That Will Be Modified

- `crates/lisa-plugin/src/lib.rs` — new `check_idle_signals()` method, call it in `poll_tick()`, update `to_ui_state()` for idle alerts
- `crates/lisa-plugin/src/ui.rs` — possibly new `AlertType::IdleWithoutArtifact` variant

## Files NOT Modified

- `crates/lisa-core/` — no core type changes (signal detection is plugin-only)
- `crates/lisa-cli/` — no CLI changes (hook infrastructure already created by T-008-01)
- `crates/lisa-plugin/src/scheduler.rs` — scheduler doesn't handle signals
