# Design: T-001-03 end-to-end-dashboard

## Goal

Write integration tests proving the full pipeline: ticket parsing -> DAG construction -> ready-ticket identification -> UI state conversion -> dashboard rendering with accurate state.

## Approaches Considered

### A: End-to-end test with temp directory + filesystem

Write tickets to a temp directory, call `scan_tickets`, build DAG, check ready tickets, convert to UI state, render dashboard, and assert on output content.

- **Pros:** Tests the real parsing path. High confidence that file format works.
- **Cons:** Requires filesystem I/O in tests (temp dirs). Slightly slower.

### B: In-memory integration test with pre-constructed tickets

Construct `Ticket` structs programmatically (like existing dag.rs tests do with `make_ticket`), build DAG, simulate thread state, convert to UI, render, and assert.

- **Pros:** No filesystem needed. Fast. Deterministic.
- **Cons:** Doesn't test the parsing layer end-to-end.

### C: Combined approach — both filesystem and in-memory tests

One test exercises the full path from files on disk through rendering. Another test exercises the in-memory path for the DAG-to-UI pipeline, which is faster and more focused.

- **Pros:** Best coverage. Fast in-memory tests plus one filesystem integration test.
- **Cons:** More test code to maintain.

## Decision: Approach C (Combined)

The acceptance criteria require proving the full pipeline works. Approach C gives us:

1. **Filesystem integration test**: Parse real ticket files from a temp dir -> DAG -> verify ready tickets. This tests the contract between the file format and the DAG layer.

2. **In-memory pipeline test**: Construct tickets -> DAG -> simulate threads and activity -> convert to `ui::PluginState` -> call `render_dashboard_lines` -> assert the output contains correct ticket IDs, phases, thread status, and dependency edges.

3. **State conversion test**: Test `to_ui_state()` style conversion (the `phase_to_ui_phase`, `ticket_status_to_ui_status`, `activity_event_to_ui_entry` functions) to verify the bridge between internal types and UI types is accurate.

## What's Rejected

- **Approach A alone**: Missing the in-memory path means slower test suite and less granularity.
- **Approach B alone**: Doesn't test parsing, which is part of the "end-to-end" claim in the ticket.
- **Mocking zellij APIs**: Not needed. The scheduling logic (`get_ready_tickets`, capacity checks) is testable without zellij. We test what we can and document the zellij boundary.

## Key Design Decisions

### Test location

- Filesystem integration test: `lisa-core` (it only uses `ticket` and `dag` modules).
- Pipeline test (DAG -> UI): `lisa-plugin` (it uses `ui::PluginState` and `render_dashboard_lines`).
- Conversion tests: `lisa-plugin` (they're in `lib.rs` scope).

### What the pipeline test asserts

1. Tickets in DAG match input tickets (IDs, phases, dependencies).
2. Ready tickets are exactly those with all deps done and in Ready phase.
3. UI state contains correct ticket nodes with correct phase indicators and status.
4. Active threads appear in the rendered dashboard.
5. Parked threads appear with artifact paths.
6. Activity log entries are rendered.
7. Status line shows correct counts (Active/Parked/Done/Total).
8. DAG layers are topologically ordered (deps before dependents).

### Dead code cleanup

The `render_dashboard()` bridge and `build_plugin_state()` in ui.rs duplicate `to_ui_state()` in lib.rs. Since `lib.rs` uses `to_ui_state()` directly, the ui.rs versions are dead code. We should either remove them or test them — design choice is to remove them to avoid confusion and eliminate dead code warnings.

Similarly, `convert_phase`, `convert_ticket_status`, and `convert_activity_event` in ui.rs are unused duplicates of the functions in lib.rs. Remove them.
