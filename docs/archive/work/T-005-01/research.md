# T-005-01 Research: Scheduling Decision Logging

## Scope

Add ActivityEvent logging to scheduling functions that currently operate silently, fix a misleading Error-level log, and add a poll_tick summary.

## Current State of Scheduling Functions

### `schedule_ready_tickets()` — lib.rs:242-298

Iterates over DAG-ready tickets and assigns them to idle agent slots. Currently logs:
- **ThreadSpawned** on successful assignment (line 288) — this is the only event.

**Missing logs (per acceptance criteria):**
1. Skip reason when `self.threads.contains_key(&ticket_id)` is true (line 251) — ticket already has a thread.
2. Slots exhausted: when `find_idle_slot()` returns None, the loop breaks silently (line 258). Should log how many ready tickets are still waiting.

### `release_slot_for_ticket()` — lib.rs:231-239

Iterates `agent_slots`, finds the one matching `ticket_id`, clears it. Completely silent — no ActivityEvent logged on success or failure (ticket not found in any slot).

Called from:
- `poll_tick()` when a ticket moves to Done (line 512)
- `detect_stale_threads()` when a thread is stale (line 457)
- `mark_ticket_done()` manual override (line 661)

### `find_idle_slot()` — lib.rs:222-226

Pure lookup (position of first `ticket_id.is_none()` slot). Not a logging site itself — logging belongs in callers.

### `discover_slots()` — lib.rs:196-219

Discovers agent pane slots from PaneManifest. Currently logs:
```rust
self.log_activity(ActivityEvent::Error {
    message: format!("Discovered {} agent pane slots", self.agent_slots.len()),
});
```
This is the **misleading Error** mentioned in the ticket — slot discovery is a normal informational event, not an error.

### `poll_tick()` — lib.rs:481-545

The main timer callback. Calls `check_artifact_advances()`, `evaluate_health()`, `detect_stale_threads()`, `rebuild_dag()`, then `schedule_ready_tickets()`. **No summary log** of cycle state (ready count, running count, idle slots).

## ActivityEvent Enum — types.rs:505-563

Current variants:
| Variant | Usage |
|---------|-------|
| PluginStarted | Plugin load |
| ThreadSpawned | Thread assigned to pane |
| PhaseCompleted | Phase artifact detected |
| ThreadExited | Thread done/failed |
| TicketStatusChanged | Status field changed |
| TicketPhaseChanged | Phase field changed |
| ArtifactCreated | Artifact file created |
| CommitMade | Git commit |
| Error | Catch-all error messages |
| DagRecomputed | DAG rebuild (filtered from UI) |
| AllTicketsDone | Terminal state |
| HealthStateChanged | Health transitions |

**No Info variant exists.** The ticket requests either adding `ActivityEvent::Info` or reusing a non-error variant.

## UI Mapping — lib.rs:932-1006 + ui.rs:176-184

`activity_event_to_ui_entry()` maps ActivityEvent → ui::ActivityEntry. Key behaviors:
- `DagRecomputed` → returns `None` (filtered from dashboard)
- `Error` → `ui::ActivityType::Error`
- `HealthStateChanged` → `Warning` (stuck) or `Error` (failed)

ui::ActivityType variants: PhaseCompleted, Commit, Error, Warning, ThreadStarted, ThreadParked.

For new Info events, we need either:
- A new `ui::ActivityType::Info` variant (for distinct rendering), or
- Reuse `Warning` (yellow, "⚠" icon — not ideal for informational messages)

## Key Files and Lines to Modify

| File | Lines | Change |
|------|-------|--------|
| `crates/lisa-core/src/types.rs` | ~549 | Add `Info { message: String }` variant to ActivityEvent |
| `crates/lisa-plugin/src/lib.rs` | 215 | Change `Error` → `Info` in discover_slots |
| `crates/lisa-plugin/src/lib.rs` | 242-298 | Add skip/exhausted logs in schedule_ready_tickets |
| `crates/lisa-plugin/src/lib.rs` | 231-239 | Add release/not-found logs in release_slot_for_ticket |
| `crates/lisa-plugin/src/lib.rs` | 481-545 | Add poll summary log in poll_tick |
| `crates/lisa-plugin/src/lib.rs` | 932-1006 | Map Info variant in activity_event_to_ui_entry |
| `crates/lisa-plugin/src/ui.rs` | 176-184 | Add Info variant to ui::ActivityType |
| `crates/lisa-plugin/src/ui.rs` | 725-781 | Render Info entries in activity log |

## Existing Test Coverage

- `test_activity_event_to_ui_entry` — tests mapping for several variants including DagRecomputed (None), ThreadSpawned, PhaseCompleted, Error
- `test_build_claude_command` — command construction
- `test_check_artifact_advances_*` — artifact detection and phase advancement
- `test_detect_stale_threads` — stale thread detection + Error log assertion
- `test_evaluate_health_*` — health transitions
- `test_rescheduling_conditions_after_completion` — slot release + ready tickets (doesn't call schedule_ready_tickets due to zellij host function)

Tests that assert on `activity_log` contents will need updating if we change Error → Info for slot discovery.

## Constraints

1. `schedule_ready_tickets()` calls `write_chars_to_pane_id()` (zellij host function) — cannot be called in native tests. Logging tests must build State manually and call the method indirectly, or test logging paths in isolation.
2. `DagRecomputed` is currently filtered from the UI (returns None). The poll summary could reuse this variant with extra fields, but that changes existing semantics. A separate `Info` variant is cleaner.
3. The `Error` variant carries only `message: String`. The new `Info` variant should match this shape for consistency.

## Observations

- The "line 219" reference in the ticket corresponds to `self.threads.contains_key(&ticket_id)` check. In the current code this is at line 251 (file has grown since the ticket was written). The logic is the same.
- `schedule_ready_tickets()` currently only logs on success (ThreadSpawned). The three new logs (skip, exhausted, scheduling) will make the scheduler decisions fully observable.
- For poll_tick summary, calculating ready/running/idle counts is straightforward: `dag.get_ready_tickets().len()`, `self.threads.values().filter(running).count()`, `self.agent_slots.iter().filter(idle).count()`.
