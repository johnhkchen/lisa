# Research: T-006-04 Runtime State Snapshot

## Objective

Add a key-triggered state snapshot ('d' is taken by mark-done modal) that writes the plugin's full internal state to `/host/.lisa-state-dump.txt` for offline inspection and debugging.

## Relevant Files and Structures

### Plugin State (`crates/lisa-plugin/src/lib.rs`)

The `State` struct (line 72) holds all runtime state. Fields to dump:

| Field | Type | Purpose |
|-------|------|---------|
| `dag` | `Dag` | Computed dependency graph |
| `threads` | `HashMap<TicketId, Thread>` | Active thread records |
| `config` | `PluginConfig` | Plugin configuration |
| `activity_log` | `Vec<ActivityEvent>` | Recent events (capped at 100) |
| `agent_slots` | `Vec<AgentSlot>` | Pre-created terminal pane slots |
| `last_phases` | `HashMap<TicketId, Phase>` | Phase snapshot for change detection |
| `last_health` | `HashMap<TicketId, HealthStatus>` | Last known health per ticket |
| `initialized` | `bool` | Load completed flag |
| `permissions_granted` | `bool` | Whether permissions were granted |
| `slots_discovered` | `bool` | Whether PaneUpdate has run |
| `terminated` | `bool` | Whether loop is complete |
| `pending_pane_writes` | `Vec<(u32, String)>` | Deferred commands |
| `pending_timer_count` | `u32` | Outstanding timer count |

### Key Handling (`lib.rs:690-724`)

`handle_key(&mut self, key: KeyWithModifier) -> bool` handles keyboard input:
- Modal mode: Esc, up/down, Enter for the mark-done modal
- Normal mode: 'd' opens the mark-done modal

The ticket AC says 'd' for dump, but 'd' is already taken by the mark-done modal. A different key is needed — the ticket may need updating, or we use a key like 'D' (shift-d) or 's' (snapshot).

### Dag (`crates/lisa-core/src/dag.rs`)

Key methods for snapshot content:
- `tickets()` -> iterator over all `Ticket`s (id, title, phase, depends_on, blocks)
- `get_dependencies(id)` -> `HashSet<TicketId>` (forward edges)
- `get_blocked_by(id)` -> `HashSet<TicketId>` (reverse edges)
- `edge_count()` -> total edges
- `get_ready_tickets()` -> ready ticket IDs
- `execution_waves()` -> wave groupings
- `stats()` -> `DagStats` (total, done, ready, in_progress, blocked, critical_path_length)

### Thread (`crates/lisa-core/src/types.rs:315-434`)

Thread fields: ticket_id, pane_id, current_phase, started_at, last_phase_change, status.
Methods: `health(now, threshold) -> HealthStatus`, `is_active()`, `is_parked()`.

### AgentSlot (`lib.rs:51-57`)

Private struct: pane_id (u32), ticket_id (Option<TicketId>), has_session (bool).

### ActivityEvent (`types.rs:534-605`)

Enum with variants: PluginStarted, ThreadSpawned, PhaseCompleted, ThreadExited, TicketStatusChanged, TicketPhaseChanged, ArtifactCreated, CommitMade, Error, DagRecomputed, AllTicketsDone, HealthStateChanged, Warning, Info, PollSummary.

Display impl exists for Phase and TicketStatus, but not for ActivityEvent. We need to format these for the dump.

### PluginConfig (`types.rs:440-528`)

Fields: ticket_dir, story_dir, work_dir, max_threads, auto_advance, stuck_threshold_secs.

### File Writing in WASI

The host filesystem is mounted at `/host`. The dump file path is `/host/.lisa-state-dump.txt` per the ticket AC. Standard `std::fs::write` works for writing files in the WASI sandbox when writing to `/host/`.

## Key Handling Conflict

The 'd' key is already bound to the mark-done modal in normal mode (lib.rs:718). The ticket AC says "Pressing 'd' (dump)", but this conflicts. Options:
1. Use 'D' (Shift+D) — `BareKey::Char('D')`
2. Use 's' (snapshot) — unambiguous
3. Use 'S' (shift+s)

Zellij's `KeyWithModifier` has `bare_key: BareKey` and modifier checking. `BareKey::Char('D')` is a capital D (with shift held). This is the most natural choice since lowercase 'd' is mark-done.

## Testing Approach

The snapshot formatting is pure string generation from known state. Tests can:
1. Construct a State (or equivalent test struct) with known threads, slots, DAG
2. Call the snapshot formatting function
3. Assert the output contains expected sections and data

Since tests run on native (not WASM), file I/O works normally. The actual key event handling uses zellij APIs and can't be tested natively, but the formatting function can be tested extensively.

The existing test pattern (see `test_check_artifact_advances_research_to_design` at lib.rs:1339) shows how to construct a State with a DAG, threads, and config for testing.

## Boundaries

- The snapshot function lives in `lisa-plugin` since it needs access to the private `State` struct, `AgentSlot`, and `MarkDoneModal`.
- It could be a method on `State` that returns a `String`, making it testable.
- File writing (the `std::fs::write` call) stays in the `handle_key` path.
- The `ActivityEvent::Info` log message is emitted after a successful write.

## Summary

The implementation adds: a snapshot formatting method on State, a key binding ('D') in handle_key, file writing to `/host/.lisa-state-dump.txt`, an Info activity event, and tests. The Dag, Thread, AgentSlot, PluginConfig, and ActivityEvent types all have the accessors needed to produce the human-readable dump.
