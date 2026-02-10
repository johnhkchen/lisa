# T-006-02 Research: Plugin Startup Diagnostics

## Current Load Flow

The plugin's `load()` method (lib.rs:812-852) does this:

1. Parse `PluginConfig` from Zellij config map
2. Prefix relative paths with `/host/` for WASI sandbox
3. Subscribe to events (`PaneUpdate`, `PermissionRequestResult`, `Timer`, `Key`)
4. Request permissions (`WriteToStdin`, `ChangeApplicationState`, `ReadApplicationState`)
5. Call `rebuild_dag()` — scans tickets, builds DAG
6. Set `initialized = true`
7. Log `ActivityEvent::PluginStarted`

No diagnostic summary is produced. If ticket parsing fails, DAG has cycles, or no
tickets exist, the operator sees nothing.

## rebuild_dag() (lib.rs:151-204)

- Calls `ticket::scan_tickets(&self.config.ticket_dir)`
- On scan failure: logs `Error { message }` and returns
- On success: builds `Dag::from_tickets(tickets)`
- On DAG build failure: logs `Error { message }` and returns
- On DAG success: logs `DagRecomputed { ticket_count }`

**Gaps:**
- Individual ticket parse errors are silently swallowed (only `eprintln!`)
- No cycle detection at startup (only detected if `Dag::from_tickets` fails for
  missing deps — cycle detection is a separate step that nobody calls here)
- No logging of ready ticket count, edge count, or config values
- No special case for zero tickets found

## scan_tickets() (ticket.rs:316-344)

Returns `Result<Vec<Ticket>, TicketError>`. On per-file parse errors, does:
```rust
Err(e) => {
    eprintln!("Warning: Failed to parse ticket {:?}: {}", path, e);
}
```

This means the caller (rebuild_dag) never sees individual parse errors. Only a
total I/O failure on `read_dir` surfaces as an error to the caller.

**Key gap:** Need a scan variant or a callback mechanism that reports per-file
errors back to the caller so they can be logged as `ActivityEvent` entries.

The `scan_tickets_recursive` function has the same pattern.

## TicketError (ticket.rs:13-27)

Has good Display impl covering:
- `Io(io::Error)` — file I/O
- `MissingFrontmatter` — no `---` delimiters
- `YamlParse(String)` — YAML parse failure
- `MissingField(String)` — required field absent (id, title, type, status, priority, phase)
- `InvalidField { field, value, reason }` — bad enum value
- `InvalidPath(PathBuf)` — non-UTF-8 path

These errors already have good messages. Just need to surface them.

## Dag API (dag.rs)

Relevant methods for diagnostics:
- `Dag::from_tickets()` — returns `Err(DagError::MissingDependency { ticket_id, missing_dep })` or `Err(DagError::CycleDetected(nodes))`
- `dag.detect_cycles()` → `CycleDetectionResult::Cycle(Vec<TicketId>)` or `NoCycle`
- `dag.get_ready_tickets()` → `Vec<TicketId>`
- `dag.stats()` → `DagStats { total_tickets, done_tickets, ready_tickets, in_progress_tickets, blocked_tickets, critical_path_length }`
- `dag.len()` — ticket count

**Missing:** No method to get edge count. The `depends_on` HashMap is private.
Need to add `edge_count()` method or extend `DagStats`.

## ActivityEvent Variants (types.rs:504-573)

Existing relevant variants:
- `Info { message: String }` — renders as `ActivityType::Info` in UI
- `Error { message: String }` — renders as `ActivityType::Error` in UI
- `DagRecomputed { ticket_count }` — filtered from UI (returns `None`)
- `PollSummary { ready, running, idle_slots }` — filtered from UI

**Missing variant:** `Warning { message: String }` — needed for "no tickets found"
case. The UI layer already has `ActivityType::Warning` with yellow icon rendering
(ui.rs:840-852), but there's no source `ActivityEvent::Warning` to produce it.
Currently warnings in the UI are only generated from `HealthStateChanged { Stuck }`.

## UI Activity Rendering (ui.rs:788-873, lib.rs:1084-1164)

`activity_event_to_ui_entry()` converts internal events to UI entries. For the
new diagnostic events we need:
- `Info` → already mapped to `ui::ActivityType::Info` with cyan "i" icon
- `Error` → already mapped to `ui::ActivityType::Error` with red "x" icon
- `Warning` → need new variant + mapping → `ui::ActivityType::Warning` with yellow "!" icon

## PluginConfig Values (types.rs:407-493)

Fields available for diagnostic logging:
- `ticket_dir: PathBuf` (default: "docs/active/tickets")
- `story_dir: PathBuf` (default: "docs/active/stories")
- `work_dir: PathBuf` (default: "docs/active/work")
- `max_threads: usize` (default: 2)
- `auto_advance: bool` (default: false)
- `stuck_threshold_secs: u64` (default: 600)

No commit lock path in `PluginConfig`. The commit lock lives in `scheduler.rs`'s
`SchedulerConfig` which is not used by the plugin (lib.rs manages scheduling directly).
The acceptance criteria mention "commit lock path" — this would be derived from the
repo root: `<repo_root>/.ralph-commit.lock`. The repo root in the WASI plugin is
`/host`, so the path would be `/host/.ralph-commit.lock`.

## Test Infrastructure

- Tests run on native target, not WASM. Zellij APIs (`subscribe`, `request_permission`,
  `set_timeout`, etc.) are not called.
- lib.rs tests verify `phase_to_ui_phase`, `ticket_status_to_ui_status`, and
  `activity_event_to_ui_entry` conversion functions.
- lib.rs does NOT test `load()`, `rebuild_dag()`, or `poll_tick()` directly because
  they call zellij APIs.
- dag.rs has comprehensive tests: empty DAG, cycles, missing deps, topo sort, stats.
- ticket.rs tests `parse_ticket_content` with various error cases.
- Diagnostic logic can be tested as a pure function: given config + scan results +
  DAG result → produce Vec<ActivityEvent>. No zellij dependency needed.

## Boundaries

The diagnostic feature touches these modules:
- **lisa-core/types.rs** — add `Warning` variant to `ActivityEvent`
- **lisa-core/ticket.rs** — new scan function or modify existing to report errors
- **lisa-core/dag.rs** — add edge count to `Dag` or `DagStats`
- **lisa-plugin/lib.rs** — add `run_startup_diagnostics()` method, call from `load()`
  after `rebuild_dag()`, convert new events for UI
- **lisa-plugin/lib.rs** — add mapping for `Warning` in `activity_event_to_ui_entry`

## Constraints

- `scan_tickets` uses `std::fs::read_dir` which is slow in WASI. The plugin already
  uses it, so the diagnostic scan doesn't add a new bottleneck — it runs once at load.
- The activity log has a 100-entry cap (`MAX_ACTIVITY_LOG`). If many tickets have parse
  errors, diagnostics could fill the log. Should be fine for typical projects.
- WASI sandbox means file paths shown in diagnostics will start with `/host/`.
