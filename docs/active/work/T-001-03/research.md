# Research: T-001-03 end-to-end-dashboard

## Ticket Goal

Verify the full pipeline works end-to-end: tickets are parsed, DAG is computed, threads are scheduled, and the dashboard renders accurate state.

## Pipeline Components

### 1. Ticket Parsing (`lisa-core/src/ticket.rs`)

- `parse_ticket(path)` reads a markdown file, extracts YAML frontmatter via `extract_frontmatter()`, and produces a `Ticket` struct.
- `scan_tickets(dir)` iterates `.md` files in a directory and calls `parse_ticket` on each.
- Frontmatter is parsed field-by-field (not via serde_yaml): `parse_yaml_line`, `parse_phase`, `parse_status`, etc.
- All fields except `story` and `blocks` are required; missing required fields return `TicketError::MissingField`.
- `depends_on` is parsed via `parse_string_vec` which handles `[T-001, T-002]` syntax.
- Existing tests: 14 tests covering parsing, frontmatter extraction, field validation, phase updates.

### 2. DAG Computation (`lisa-core/src/dag.rs`)

- `Dag::from_tickets(iter)` builds the graph in two passes: first inserts all nodes, then builds forward (`depends_on`) and reverse (`blocks`) edge maps.
- Validates all referenced dependencies exist; returns `DagError::MissingDependency` if not.
- `can_start(ticket_id)` checks: ticket is in `Ready` phase AND all dependencies are in `Done` phase.
- `get_ready_tickets()` returns all ticket IDs that satisfy `can_start`.
- `get_runnable_tickets()` returns `&Ticket` references for the same.
- `detect_cycles()` and `topological_sort()` use Kahn's algorithm.
- `critical_path()` computes longest dependency chain via dynamic programming on topological order.
- Existing tests: 18 tests covering empty DAG, single tickets, dependency chains, diamonds, cycles, topological sort, critical path, stats.

### 3. Thread Scheduling (`lisa-plugin/src/lib.rs` and `scheduler.rs`)

**In `lib.rs` (State):**
- `rebuild_dag()` calls `scan_tickets` then `Dag::from_tickets` and logs activity events.
- `schedule_ready_tickets()` calls `dag.get_ready_tickets()`, computes available thread slots (`max_threads - active_count`), and spawns Claude sessions via `open_command_pane_floating` for each ready ticket up to capacity. Skips tickets that already have threads.
- `handle_pane_exited()` marks threads as completed/failed based on exit code.
- `handle_filesystem_update()` snapshots old phases, rescans tickets on ticket_dir changes, detects phase changes, logs artifact creation.
- Event flow: `load()` -> `rebuild_dag()` -> `schedule_ready_tickets()` (on permission grant); `FileSystemUpdate` -> `handle_filesystem_update()` -> `rebuild_dag()` -> `schedule_ready_tickets()`.

**In `scheduler.rs` (Scheduler):**
- Standalone `Scheduler` struct with its own thread tracking, separate from `State`.
- `get_next_ready_tickets(dag)` filters out already-running, parked, and completed tickets.
- `spawn_thread(ticket_id, pane_id)` creates a `Thread` record, enforces capacity limit.
- `spawn_claude_session(ticket_id)` calls zellij's `open_command_pane_floating` with `claude --dangerously-skip-permissions --print "Read the ticket..."`.
- `park_thread`, `resume_thread`, `complete_thread`, `handle_thread_exit`, `handle_pane_exit` manage lifecycle.
- `CommitLock` uses flock(2) on Unix, placeholder on WASM.
- Existing tests: 7 tests covering creation, lifecycle, capacity, pane exit, work directory.

**Duplication note:** Both `State` (lib.rs) and `Scheduler` (scheduler.rs) implement thread management and Claude session spawning. Currently `State` is the active code path used by the plugin; `Scheduler` is a more modular but not-yet-wired alternative.

### 4. Dashboard Rendering (`lisa-plugin/src/ui.rs`)

- UI has its own type system: `ui::Phase`, `ui::TicketStatus`, `ui::TicketNode`, `ui::ActiveThread`, `ui::ParkedThread`, `ui::ActivityEntry`, `ui::PluginState`.
- `State::to_ui_state()` in lib.rs converts internal state -> `ui::PluginState`.
- Conversion functions: `phase_to_ui_phase`, `ticket_status_to_ui_status`, `activity_event_to_ui_entry`.
- `print_dashboard(state, rows, cols)` renders via `render_dashboard_lines` which calls:
  - `render_dag()` — topological layering via `compute_dag_layers`, ASCII art with phase indicators and status badges.
  - `render_active_threads()` — table with ticket ID, phase, running time, pane ID.
  - `render_parked_threads()` — table with artifact paths and wait times.
  - `render_activity_log()` — newest-first log with icons and colors.
  - `render_quick_jump()` — numbered list of panes for keyboard navigation.
  - `render_status_line()` — compact "Active: N | Parked: N | Done: N/M".
- Also has an unused `render_dashboard()` bridge function and `build_plugin_state()` that duplicates `to_ui_state()`.
- Existing tests: 12 tests covering duration formatting, phase names/indicators, DAG layering, dashboard sections, status line.

## Data Flow Summary

```
scan_tickets(dir) → Vec<Ticket>
    ↓
Dag::from_tickets(tickets) → Dag
    ↓
dag.get_ready_tickets() → Vec<TicketId>
    ↓
schedule_ready_tickets() → spawns Claude sessions via open_command_pane_floating
    ↓
State.to_ui_state() → ui::PluginState
    ↓
print_dashboard(state, rows, cols) → terminal output
```

## Current Test Coverage Gaps (for end-to-end)

1. **No integration test** that feeds tickets through parse -> DAG -> ready-check -> UI rendering in a single flow.
2. **State.to_ui_state()** is untested — the conversion from internal state to UI state has no dedicated test.
3. **schedule_ready_tickets()** in `State` calls zellij APIs directly, making it untestable without mocking. The `Scheduler` in scheduler.rs is more testable but isn't wired to `State`.
4. **render_dashboard** bridge function and `build_plugin_state` in ui.rs are dead code (unused, duplicating `to_ui_state` in lib.rs).
5. **Filesystem-based end-to-end**: `scan_tickets` -> `Dag::from_tickets` -> `get_ready_tickets` could be tested with a temp directory, but no such test exists.

## Constraints

- Tests run on native target (not WASM), so zellij APIs cannot be called.
- The `State` struct derives `Default` and can be constructed, but `schedule_ready_tickets` calls `open_command_pane_floating` which is a zellij API.
- `ui::print_dashboard` uses `println!` directly — output capture for testing would need stdout redirection or refactoring to return strings (which `render_dashboard_lines` already does).
- The `render_dashboard_lines` function is already testable and tested.
