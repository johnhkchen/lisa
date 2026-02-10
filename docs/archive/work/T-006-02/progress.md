# T-006-02 Progress: Plugin Startup Diagnostics

## Completed

### Step 1: `ActivityEvent::Warning` variant (types.rs)
Added `Warning { message: String }` variant between `HealthStateChanged` and `Info`.

### Step 2: `Dag::edge_count()` (dag.rs)
Already existed in codebase (added by a prior sprint). Verified it works and has tests.

### Step 3: `scan_tickets_with_diagnostics()` (ticket.rs)
- Added `ScanResult` struct with `tickets` and `errors` fields
- Added `scan_tickets_with_diagnostics()` function that collects per-file parse errors
  instead of silently skipping them
- 3 new tests: clean, parse_error, empty_dir

### Step 4: `diagnostics.rs` module (lisa-core)
- Created `crates/lisa-core/src/diagnostics.rs` with `startup_diagnostics()` pure function
- Takes config, scan result, DAG result, and commit lock path
- Produces `Vec<ActivityEvent>`:
  - `Info` with config values (ticket_dir, max_threads, commit_lock)
  - `Error` for each per-file parse error (with filename)
  - `Warning` if no tickets found
  - `Error` if DAG has cycles or missing dependencies
  - `Info` summary: ticket count, edge count, ready count, max_threads
- 6 new tests: clean_load, parse_errors, cycles, no_tickets, config_values, missing_dependency

### Step 5: Plugin wiring (lib.rs)
- Updated `load()` to call `scan_tickets_with_diagnostics()` then `startup_diagnostics()`
- Diagnostic events fed into activity log before `PluginStarted`
- `activity_event_to_ui_entry()` already had `Warning` mapping (auto-added by linter)
- DAG stored on success, empty DAG on failure

## Verification
- `cargo test --workspace` — 214 tests pass (77 + 81 + 56)
- `cargo check -p lisa-plugin --target wasm32-wasip1` — compiles clean
- No new warnings introduced

## Acceptance Criteria Coverage
- [x] On load, logs Info with: ticket count, edge count, ready count, max_threads
- [x] Parse errors logged as Error with filename and error message
- [x] DAG cycles logged as Error with cycle path
- [x] No tickets found → Warning (not silent)
- [x] Config values logged: ticket_dir, max_threads, commit_lock path
- [x] Diagnostic messages in activity log before PluginStarted → visible on first render
- [x] Tests: clean load, parse errors, cycles, no tickets
