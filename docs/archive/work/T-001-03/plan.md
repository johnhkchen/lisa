# Plan: T-001-03 end-to-end-dashboard

## Step 1: Remove dead code from `ui.rs`

Remove the unused functions and imports in `crates/lisa-plugin/src/ui.rs`:
- Delete `render_dashboard()` bridge function
- Delete `build_plugin_state()`
- Delete `convert_phase()`
- Delete `convert_ticket_status()`
- Delete `convert_activity_event()`
- Remove the now-unused imports: `Dag`, `types::{self, ActivityEvent, PluginConfig, Thread, TicketId}`

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1` compiles cleanly with fewer dead code warnings. `cargo test -p lisa-plugin` passes.

## Step 2: Add conversion tests in `lib.rs`

Add `#[cfg(test)] mod tests` block to `crates/lisa-plugin/src/lib.rs` with:
- `test_phase_to_ui_phase`: Map all 8 Phase variants and assert correct UI phase.
- `test_ticket_status_to_ui_status`: Test all TicketStatus variants including the Open+Ready vs Open+active-phase distinction.
- `test_activity_event_to_ui_entry`: Test ThreadSpawned, PhaseCompleted, Error events convert correctly; PluginStarted and DagRecomputed return None.

**Verify:** `cargo test -p lisa-plugin` passes with new tests.

## Step 3: Add filesystem integration test in `dag.rs`

Add `test_end_to_end_scan_to_dag` to `crates/lisa-core/src/dag.rs`:
1. Create a `tempfile::tempdir()`.
2. Write 3 ticket files: T-001 (Done, no deps), T-002 (Ready, depends on T-001), T-003 (Ready, depends on T-001 and T-002).
3. Call `ticket::scan_tickets(dir)`.
4. Call `Dag::from_tickets(tickets)`.
5. Assert: `dag.len() == 3`.
6. Assert: `get_ready_tickets()` returns only T-002 (T-003 is blocked by T-002 which is not Done).
7. Assert: topological sort has T-001 first.
8. Assert: `get_dependencies("T-003")` includes both T-001 and T-002.

Need to add `tempfile` as a dev-dependency to `lisa-core`.

**Verify:** `cargo test -p lisa-core` passes with new test.

## Step 4: Add pipeline test in `ui.rs`

Add `test_pipeline_dag_to_dashboard` to `crates/lisa-plugin/src/ui.rs`:
1. Construct a `PluginState` with 4 tickets in a diamond pattern:
   - T-001: Done, no deps, blocks T-002 and T-003.
   - T-002: Research phase, InProgress, depends on T-001, blocks T-004.
   - T-003: Ready, depends on T-001, blocks T-004.
   - T-004: Ready, Blocked, depends on T-002 and T-003.
2. Add an active thread for T-002 (Design phase, pane 5).
3. Add a parked thread for some ticket (Research phase, with artifact path).
4. Add activity entries: PhaseCompleted, ThreadStarted, Error.
5. Call `render_dashboard_lines(&state, 80, 50)`.
6. Assert: output contains "T-001", "T-002", "T-003", "T-004".
7. Assert: output contains "Dashboard" header.
8. Assert: output contains "Active Threads" section with "T-002".
9. Assert: status line has correct counts.
10. Assert: DAG layers show T-001 before T-002/T-003 before T-004.

**Verify:** `cargo test -p lisa-plugin` passes with new test.

## Step 5: Run full workspace tests and WASM check

- `cargo test --workspace` — all tests pass.
- `cargo check -p lisa-plugin --target wasm32-wasip1` — clean compilation.

## Testing Strategy

- Unit tests for conversion functions (Step 2).
- Filesystem integration test for parse-to-DAG (Step 3).
- In-memory pipeline test for DAG-to-dashboard (Step 4).
- Full workspace check to ensure no regressions (Step 5).
