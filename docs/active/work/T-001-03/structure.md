# Structure: T-001-03 end-to-end-dashboard

## Files Modified

### `crates/lisa-core/src/dag.rs` — Add filesystem integration test

Add a new test at the bottom of the existing `#[cfg(test)] mod tests`:

- `test_end_to_end_scan_to_dag`: Creates a temp directory, writes 3 ticket markdown files (one Done, one Ready with dep on first, one Ready blocked by second), calls `scan_tickets`, builds DAG, asserts ready tickets, dependency edges, and topological order.

### `crates/lisa-plugin/src/ui.rs` — Remove dead code, add pipeline test

**Remove** the following unused functions:
- `render_dashboard()` (line 649) — bridge function never called
- `build_plugin_state()` (line 662) — duplicated by `State::to_ui_state()` in lib.rs
- `convert_phase()` (line 743) — duplicated by `phase_to_ui_phase()` in lib.rs
- `convert_ticket_status()` (line 757) — duplicated by `ticket_status_to_ui_status()` in lib.rs
- `convert_activity_event()` (line 775) — duplicated by `activity_event_to_ui_entry()` in lib.rs

**Remove** the dead `use` imports that were only needed by the removed functions:
- `use lisa_core::dag::Dag;`
- `use lisa_core::types::{self, ActivityEvent, PluginConfig, Thread, TicketId};`

**Add** integration test to the existing `#[cfg(test)] mod tests`:

- `test_pipeline_dag_to_dashboard`: Constructs a `PluginState` with 4 tickets in a diamond DAG (one Done root, two Ready middle nodes, one Blocked leaf), active threads, parked threads, and activity log entries. Calls `render_dashboard_lines`. Asserts:
  - All ticket IDs appear in output
  - Active thread ticket appears with its phase
  - Parked thread ticket appears with artifact path
  - Activity log entries appear
  - Status line shows correct counts
  - DAG layers are in correct order (root before middle before leaf)

### `crates/lisa-plugin/src/lib.rs` — Add conversion tests

Add tests to the existing inline test infrastructure (the file doesn't have a `#[cfg(test)]` block yet, so add one):

- `test_phase_to_ui_phase`: Verifies all 8 Phase variants map correctly.
- `test_ticket_status_to_ui_status`: Verifies all TicketStatus variants map correctly, including the Open+Ready -> ui::Ready and Open+Research -> ui::InProgress cases.
- `test_activity_event_to_ui_entry`: Verifies key activity events convert correctly, and that PluginStarted/DagRecomputed/TicketStatusChanged return None.

## Files Not Modified

- `crates/lisa-core/src/types.rs` — No changes needed.
- `crates/lisa-core/src/ticket.rs` — No changes needed; parsing is exercised via `scan_tickets` in the integration test.
- `crates/lisa-plugin/src/scheduler.rs` — No changes needed; existing tests are sufficient and the `Scheduler` is not wired to `State`.

## Module Boundaries

- The filesystem integration test lives in `lisa-core` because it only uses `ticket::scan_tickets` and `dag::Dag`.
- The pipeline test lives in `lisa-plugin/ui.rs` because it uses UI types.
- The conversion tests live in `lisa-plugin/lib.rs` because the conversion functions are defined there.

## Public Interface Changes

None. All changes are test additions and dead code removal. No public API changes.
