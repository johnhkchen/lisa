# Design: T-006-04 Runtime State Snapshot

## Decision: Key Binding

**Chosen: 'D' (Shift+D)**

The ticket AC specifies 'd' for dump, but lowercase 'd' is already bound to the mark-done modal. 'D' (capital D, i.e. Shift+D) is the natural alternative — same letter, different modifier. No existing binding conflicts.

Rejected alternatives:
- 's' for snapshot — less discoverable, 'd' for dump is more intuitive
- Adding a sub-menu — overengineered for a single action

## Decision: Snapshot as a Method on State

**Chosen: `State::format_snapshot() -> String` method**

The snapshot formatting is a method on `State` that returns a `String`. This keeps the logic testable (construct State, call method, assert on output) without needing zellij APIs. The key handler calls this method and writes the result to disk.

Rejected alternatives:
- Separate snapshot module — unnecessary indirection for a single function
- Putting format logic in lisa-core — needs access to private State fields (agent_slots, modal, pending_pane_writes) that only exist in lisa-plugin

## Decision: Output Format

**Chosen: Sections with headers, aligned columns, one-line-per-item**

The output is human-readable plain text optimized for scanning in a terminal or editor. Each section has a `=== HEADER ===` delimiter. Data uses fixed-width columns where practical. No JSON/YAML per the AC.

Sections in order:
1. **Header** — timestamp, "Lisa State Snapshot"
2. **Config** — ticket_dir, work_dir, max_threads, stuck_threshold, auto_advance
3. **Plugin Status** — initialized, permissions_granted, slots_discovered, terminated, pending_timer_count
4. **Tickets** — table: ID, phase, status, depends_on
5. **DAG Edges** — list of "A -> B" dependency edges
6. **DAG Stats** — total, done, ready, in_progress, blocked, critical_path
7. **Threads** — table: ticket_id, pane_id, phase, status, started_at, last_phase_change, health
8. **Agent Slots** — table: pane_id, ticket_id, has_session
9. **Health Status** — table: ticket_id, last_known_health
10. **Activity Log** — last 50 events, newest first, one line each

## Decision: File Path

**Chosen: `/host/.lisa-state-dump.txt`** per AC.

Single file, overwritten each time. The `/host/` prefix maps to the project root in the WASI sandbox. The `.` prefix keeps it hidden in normal `ls` output.

## Decision: ActivityEvent Formatting

Each ActivityEvent variant gets a one-line text representation:
- `PluginStarted` -> "PluginStarted"
- `ThreadSpawned { ticket_id, pane_id }` -> "ThreadSpawned: T-001 pane=#42"
- `Error { message }` -> "Error: ..."
- etc.

This is a simple match on the enum — no Display trait impl needed (would pollute the core types for a debug feature).

## Decision: Timestamp Format

Use Unix epoch seconds for the snapshot timestamp header. Thread times (started_at, last_phase_change) shown as "Xs ago" relative to snapshot time for quick scanning.

## Testing Strategy

- Construct a `State` with known DAG (2-3 tickets with dependencies), threads (running + parked), agent slots (occupied + idle), activity events, and config.
- Call `format_snapshot()` and assert sections exist.
- Assert specific ticket IDs, phases, edges, and slot data appear in the output.
- Assert activity log is limited to last 50 entries.
- No file I/O in tests — only string content verification.
