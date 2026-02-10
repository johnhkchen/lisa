# T-006-02 Plan: Plugin Startup Diagnostics

## Implementation Steps

### Step 1: Add `Warning` variant to `ActivityEvent` (types.rs)

Add `Warning { message: String }` to the `ActivityEvent` enum. No test needed —
it's a data variant. Verify with `cargo check`.

### Step 2: Add `edge_count()` to Dag (dag.rs)

Add `pub fn edge_count(&self) -> usize` that sums dependency edge counts.
Add unit test: `test_edge_count` — build a known DAG, assert edge count.

### Step 3: Add `ScanResult` and `scan_tickets_with_diagnostics()` (ticket.rs)

Add the struct and function. Tests:
- `test_scan_with_diagnostics_clean` — all valid tickets, errors vec empty
- `test_scan_with_diagnostics_parse_error` — mix of valid + invalid, errors non-empty

### Step 4: Create `diagnostics.rs` module (lisa-core)

Create `crates/lisa-core/src/diagnostics.rs` with `startup_diagnostics()`. Register
in `lib.rs`. Tests:
- `test_diagnostics_clean_load` — produces Info events, no errors/warnings
- `test_diagnostics_parse_errors` — produces Error events for each bad file
- `test_diagnostics_cycles` — produces Error with cycle node IDs
- `test_diagnostics_no_tickets` — produces Warning "No tickets found"
- `test_diagnostics_config_values` — verifies config values appear in Info messages

### Step 5: Wire into plugin `load()` (lib.rs)

Replace `self.rebuild_dag()` call in load() with:
1. `scan_tickets_with_diagnostics()`
2. `Dag::from_tickets()`
3. `startup_diagnostics()`
4. Feed diagnostic events into `self.log_activity()`

Add `Warning` case to `activity_event_to_ui_entry()`.

### Step 6: Verify

Run `cargo test --workspace` — all existing + new tests pass.
Run `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM compiles.

## Testing Strategy

All diagnostic logic is tested in lisa-core (native target):
- **Unit**: `edge_count`, `scan_tickets_with_diagnostics`
- **Integration**: `startup_diagnostics` with synthetic inputs

The plugin wiring (Step 5) is not unit-tested directly because it calls zellij APIs.
It is verified by WASM compilation + manual testing.

## Commit Plan

- **Commit 1**: Steps 1-4 (all lisa-core changes + tests)
- **Commit 2**: Step 5 (plugin wiring)
