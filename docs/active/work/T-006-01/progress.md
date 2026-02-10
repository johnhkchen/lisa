# Progress: T-006-01 lisa-status-cli-command

## Completed

### Step 1: Display impls in types.rs
- Added `impl Display for Phase` — maps to lowercase strings
- Added `impl Display for TicketStatus` — maps to lowercase strings (in_progress with underscore)
- Added tests: `test_phase_display`, `test_ticket_status_display`
- All 68 core tests pass (now 77 with external additions)

### Step 2: DAG methods in dag.rs
- Added `edge_count()` — sums all dependency set sizes
- Added `execution_waves()` — groups tickets into wave levels via BFS over topo order
- Added 7 tests: empty, chain, diamond for edge_count; no_deps, chain, diamond, cycle_error for waves
- All core tests pass

### Step 3: status.rs module
- Created `crates/lisa-cli/src/status.rs` with `run_status()` function
- Loads config, scans tickets, builds DAG, prints summary + waves + ready list
- Output format: header (ticket/edge/cycle counts), wave sections with aligned columns, ready summary
- 7 tests: no_tickets, single_ticket, dependency_chain, cycle_error, missing_dep, missing_dir, respects_config

### Step 4: Wire into main.rs
- Added `mod status;` declaration
- Added `Status { path }` variant to `Commands` enum
- Added match arm calling `status::run_status()`

### Bonus fix
- Added missing `ActivityEvent::Warning` match arm in `lisa-plugin/src/lib.rs` (was added by another ticket, caused compilation failure)

## Verification
- `cargo test --workspace`: 214 tests pass (56 CLI + 77 core + 81 plugin)
- `cargo check -p lisa-plugin --target wasm32-wasip1`: passes
- No new warnings introduced

## Deviations from plan
- Had to handle `ActivityEvent::Warning` variant added by concurrent work
- Linter kept relocating `edge_count()` causing duplicate definition; resolved by keeping single copy
