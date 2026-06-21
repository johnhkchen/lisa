# Research: T-018-02 session-timeout-enforcement

## Objective

Enforce the `session_timeout_secs` config (added by T-018-01) in the WASM plugin. When an agent session exceeds the wall-clock timeout, Lisa should log the event, mark the thread as timed out, free the scheduling slot, and show the timeout distinctly in the dashboard.

## Codebase Mapping

### Config (already in place from T-018-01)

**`crates/lisa-core/src/types.rs`** — `PluginConfig::session_timeout_secs: u64` (default 1800, 0 = disabled). Already parsed from the Zellij config map. No changes needed here for the config itself.

### Thread Model (`types.rs` lines 279–428)

- `Thread` struct has `started_at: SystemTime` — this is the wall-clock start time, set in `Thread::new()`.
- `ThreadStatus` enum: `Running`, `Parked`, `Completed`, `Failed`. There is **no `TimedOut` variant** currently.
- `Thread::health()` computes health from `last_phase_change` against `stuck_threshold_secs`. This is per-phase staleness, **not** total session timeout.
- `Thread::mark_exited()` sets Completed or Failed based on exit code.

Key observation: The ticket says "Mark the thread as timed out (new state or reuse existing error state)." We need to decide whether to add a new `ThreadStatus::TimedOut` variant or reuse `Failed`.

### Scheduling (`lib.rs` — State struct, lines 147–211)

- `State.threads: HashMap<TicketId, Thread>` — active threads by ticket ID.
- `State.agent_slots: Vec<AgentSlot>` — pane slots with `ticket_id`, `has_session`, `transition_state`, `cooldown_until`.
- `release_slot_for_ticket()` (line 353) — clears the slot's `ticket_id`, sets cooldown. Does NOT kill the process.
- `schedule_ready_tickets()` (line 378) — finds idle slots and launches new sessions.

### Poll Tick (`lib.rs` lines 1232–1340)

The `poll_tick()` method runs every 5 seconds and calls:
1. `check_artifact_advances()` — detect new artifacts, advance phases
2. `check_idle_signals()` — process `.idle` signal files
3. `check_transition_signals()` — process `.stopped`/`.cleared` signals
4. `check_transition_timeouts()` — force-advance stalled transitions
5. `check_review_timeouts()` — send finish-up prompts to review threads
6. `evaluate_health()` — log Healthy→Stuck transitions
7. `detect_stale_threads()` — hard timeout (2x stuck threshold), marks failed, releases slot, removes thread

The session timeout check should be a new method called from `poll_tick()`, similar to `detect_stale_threads()` but checking `started_at` against `session_timeout_secs` instead of `last_phase_change` against `stuck_threshold_secs`.

### Stale Thread Detection (`lib.rs` lines 1159–1195)

`detect_stale_threads()` is the closest analogue:
- Checks `last_phase_change` against `2 * stuck_threshold_secs`
- Marks failed, releases slot, removes from threads map
- Logs `ActivityEvent::Error`

Session timeout differs: it checks `started_at` against `session_timeout_secs`, and the ticket says "Do NOT kill the Claude Code process" and "just stop waiting for it."

### Dashboard UI (`ui.rs`)

- `ActiveThread` struct: `ticket_id`, `phase`, `started_at`, `slot_number`
- `ParkedThread` struct: similar
- `AlertType` enum: `Failed`, `Stuck`, `IdleWithoutArtifact`
- `HealthAlert` struct: `ticket_id`, `alert_type`, `detail`, `suggested_actions`
- The dashboard renders threads in the slot grid and alerts in the attention banner.

For timed-out sessions, the ticket wants: `"T-024-01 [TIMEOUT 32m]"`. This could be:
1. A new `AlertType::TimedOut` variant
2. Display in the slot grid showing TIMEOUT status
3. Both

### ActivityEvent (`types.rs` lines 552–637)

Current variants include `ThreadExited`, `Error`, `Warning`, `HealthStateChanged`. No timeout-specific variant. The ticket asks to "Log an `ActivityEvent` with timeout details: ticket ID, elapsed time, phase at timeout." This suggests a new variant like `SessionTimedOut { ticket_id, elapsed_secs, phase }`.

### Handling Late Artifacts from Timed-Out Sessions

The ticket says: "If a timed-out session later produces artifacts, lisa should recognize and handle it gracefully."

After a timeout:
- The thread is removed from `State.threads`
- The slot is released (ticket_id cleared)
- But `has_session` stays true — Claude Code is still running in the pane

If the session later produces an artifact (e.g., writes `research.md`), `check_artifact_advances()` won't pick it up because there's no thread for that ticket. However, the next time the ticket is scheduled (if it's retried), the artifacts will already be there.

The main risk: the session writes to the ticket frontmatter directly (e.g., updates phase). This is handled by `rebuild_dag()` which re-reads ticket files regardless of thread state.

## Constraints

1. WASM plugin has no process management — cannot kill Claude Code. Matches the ticket's requirement.
2. `session_timeout_secs = 0` means disabled (convention from T-018-01).
3. The check must be cheap — runs every 5 seconds in `poll_tick`.
4. Must not double-fire: once timed out, the thread is removed, so subsequent polls won't re-trigger.

## Files to Modify

1. `crates/lisa-core/src/types.rs` — Add `ThreadStatus::TimedOut` (or not), add `ActivityEvent::SessionTimedOut`
2. `crates/lisa-plugin/src/lib.rs` — Add `check_session_timeouts()`, call from `poll_tick()`
3. `crates/lisa-plugin/src/ui.rs` — Add `AlertType::TimedOut`, render timeout display

## Open Questions

1. New `ThreadStatus::TimedOut` vs reuse `Failed`? Adding a distinct variant gives better observability.
2. Should timed-out tickets be automatically retried? Ticket says no for v1.
3. Where in `poll_tick` should the check go? Before or after `detect_stale_threads`?
