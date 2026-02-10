# Research: T-003-03 completion-reschedule

## What This Ticket Requires

When a Claude session exits or a ticket reaches `done`, unblock dependents and schedule the next wave. Four acceptance criteria:

1. On session exit with exit code 0 + ticket in implement phase: mark ticket as done
2. On non-zero exit: mark thread as failed, log error, leave phase unchanged for retry
3. After any ticket state change: rebuild DAG, call `schedule()` for newly-ready tickets
4. Clean loop termination when all tickets are done

## Current Architecture

### Spawning Model (lib.rs:52-249)

The plugin uses a **terminal pane + text injection** model:

- The Zellij layout pre-creates terminal panes (agent slots) — `AgentSlot { pane_id, ticket_id, has_session }`
- On scheduling, `write_chars_to_pane_id()` sends a claude command string to an idle pane
- Session reuse: first use sends `claude --dangerously-skip-permissions "prompt"\r`, subsequent uses send `/clear\rprompt\r`
- This is fundamentally different from `open_command_pane()` which would spawn a command pane that fires exit events

Because claude runs **inside** a persistent terminal pane, there is no `CommandPaneExited` event. When claude exits, the pane returns to its shell prompt — it doesn't close.

### Completion Detection (lib.rs:328-375)

Currently done via timer-based polling in `poll_tick()` (every 5 seconds):

1. `check_artifact_advances()` — scans work dirs for new phase artifacts, advances ticket phases via frontmatter update
2. `rebuild_dag()` — re-reads all ticket files, detects phase changes by comparing to `last_phases` snapshot
3. If changes detected: marks threads with Done-phase tickets as complete, releases their slots
4. `schedule_ready_tickets()` — finds newly-ready tickets and assigns to freed slots

### Thread Lifecycle (types.rs:252-345)

`Thread` has: `ticket_id`, `pane_id`, `current_phase`, `started_at`, `status`

`ThreadStatus`: Running → Parked | Completed | Failed

Key methods:
- `mark_exited(exit_code)`: Some(0) → Completed, anything else → Failed
- `complete()`, `fail()`, `park()`, `resume()`

### Scheduler Module (scheduler.rs)

Contains a standalone `Scheduler` struct with `handle_pane_exit()`, `complete_thread()`, etc. — but this struct is **not used** by the plugin's `State`. The plugin manages threads directly in its own `HashMap<TicketId, Thread>`. The Scheduler was likely an earlier design that was superseded by the direct management in lib.rs.

### DAG Module (dag.rs:222-238)

`get_ready_tickets()`: Returns tickets where `can_start()` is true — meaning all deps are done AND ticket phase is startable (anything except Done).

## Gaps Between Current Code and Acceptance Criteria

### Gap 1: No Exit Code Detection

The terminal pane model provides no exit code signal. When claude finishes:
- The terminal returns to a shell prompt
- No event fires to the plugin
- The only detection is via `poll_tick()` checking if phase artifacts appeared or ticket frontmatter changed

**Resolution approach**: Since agents already update ticket frontmatter as they complete phases (per RDSPI workflow), and `check_artifact_advances()` detects implement→review transitions via `progress.md`, the path to done must be: agent writes progress.md → plugin detects and advances to review → review completes (manually or auto-advance) → plugin detects done.

For **failure detection**: there's no signal at all when claude exits non-zero inside a terminal pane. Possible approaches:
- A heartbeat/liveness check (detect stale threads that haven't advanced)
- A sentinel file written by the agent on success/failure
- Using Zellij's `RunCommandResult` with a wrapper script instead of text injection

### Gap 2: Implement Phase → Done Transition

The current `check_artifact_advances()` transitions implement → review (when progress.md appears). The ticket wants: "if ticket was in implement phase, mark as done." This contradicts the RDSPI workflow which has implement → review → done.

Two interpretations:
- **Literal**: Session exit with code 0 during implement = done (skipping review). This makes sense if the agent's session covering the implement phase completes successfully — the work is done.
- **Workflow-aligned**: The agent finishes implement, produces progress.md, plugin advances to review, then review is handled separately. Session exit during implement means the agent finished its run.

The practical path: when a thread is in implement phase and the DAG poll detects the ticket reached review (via artifact), **and** the agent is no longer actively working (terminal pane is idle), the plugin can auto-advance review → done or leave it for human review.

However, re-reading the AC: "On `CommandPaneExited` with exit code 0: if ticket was in implement phase, mark as done" — this specifically says the **session exit** is the trigger, not the artifact. This suggests the intent is: when the Claude session itself finishes during the implement phase, that means the implementation is complete and we should mark the ticket done.

### Gap 3: No Failure Handling

No mechanism exists to detect or handle failed sessions. The `Thread::fail()` method exists but is never called. No retry logic exists.

### Gap 4: No Clean Termination

`poll_tick()` always re-arms the timer (`set_timeout(POLL_INTERVAL_SECS)`). There's no check for "all tickets done, stop the loop."

## Relevant Zellij Plugin API

Events currently subscribed: `PaneUpdate`, `PermissionRequestResult`, `Timer`

Potentially relevant additions:
- `EventType::RunCommandResult` — fires after `run_command()` completes, includes exit code and context map. Could be used to run a wrapper that checks agent status.
- No `CommandPaneExited` in the API for terminal panes (only for command panes opened via `open_command_pane`).
- `PaneUpdate` fires on pane changes but doesn't carry exit codes.

## Key Files

| File | Lines | Relevance |
|------|-------|-----------|
| `crates/lisa-plugin/src/lib.rs` | 328-375 | `poll_tick()` — main detection/scheduling loop |
| `crates/lisa-plugin/src/lib.rs` | 196-249 | `schedule_ready_tickets()` — slot assignment |
| `crates/lisa-plugin/src/lib.rs` | 186-193 | `release_slot_for_ticket()` — slot freeing |
| `crates/lisa-plugin/src/lib.rs` | 258-323 | `check_artifact_advances()` — artifact detection |
| `crates/lisa-plugin/src/lib.rs` | 103-149 | `rebuild_dag()` — DAG rescan + change detection |
| `crates/lisa-plugin/src/lib.rs` | 420-457 | `update()` — event handler |
| `crates/lisa-core/src/types.rs` | 252-345 | Thread struct and lifecycle |
| `crates/lisa-core/src/types.rs` | 44-114 | Phase enum (next, artifact_filename, is_startable) |
| `crates/lisa-core/src/dag.rs` | 222-238 | `get_ready_tickets()` |
| `crates/lisa-core/src/ticket.rs` | 398-406 | `update_ticket_phase()` — writes frontmatter |

## Constraints and Assumptions

1. **Terminal pane model is fixed** — the session reuse feature (T-003-01 reuse) was just added and the slot-based architecture is established. Switching to command panes would break session reuse.

2. **Polling is the primary detection mechanism** — without exit events from terminal panes, the 5-second poll interval is the detection latency floor.

3. **RDSPI workflow has 5 phases + review** — implement is followed by review. The "mark as done on implement exit" AC may mean: detect that the agent has finished its implement work (via some signal), then mark done directly (auto-advancing through review).

4. **The Scheduler struct in scheduler.rs is dead code** for the plugin's actual operation. All thread management happens directly in State.

5. **Test coverage**: 88 tests exist across the workspace. New logic should follow the existing test pattern (tempdir-based, no zellij API calls).
