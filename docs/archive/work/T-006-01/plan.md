# Plan: T-006-01 lisa-status-cli-command

## Step 1: Add Display impls to types.rs

Add `impl fmt::Display` for `Phase` and `TicketStatus` in `crates/lisa-core/src/types.rs`.

- Phase: "ready", "research", "design", "structure", "plan", "implement", "review", "done"
- TicketStatus: "open", "in_progress", "blocked", "review", "done", "cancelled"

Add tests: `test_phase_display`, `test_ticket_status_display`.

Verify: `cargo test -p lisa-core`

## Step 2: Add edge_count() and execution_waves() to dag.rs

In `crates/lisa-core/src/dag.rs`:

**edge_count()**: iterate over `self.depends_on` values, sum their lengths.

**execution_waves()**:
1. Run topological sort (returns error on cycle)
2. For each node in topo order, compute wave = max(wave of each dependency) + 1 (or 0 if no deps)
3. Group tickets by wave number, return Vec<Vec<TicketId>>
4. Sort ticket IDs within each wave for deterministic output

Tests:
- `test_edge_count_empty` — empty DAG has 0 edges
- `test_edge_count_chain` — A→B→C has 2 edges
- `test_edge_count_diamond` — A→(B,C)→D has 4 edges
- `test_execution_waves_no_deps` — all in wave 0
- `test_execution_waves_chain` — 3 waves of 1
- `test_execution_waves_diamond` — wave 0: [A], wave 1: [B,C], wave 2: [D]
- `test_execution_waves_cycle_error` — returns DagError

Verify: `cargo test -p lisa-core`

## Step 3: Create status.rs

Create `crates/lisa-cli/src/status.rs` with `pub fn run_status(root: &Path) -> Result<(), String>`.

Logic:
1. Load config via `config::load_config(root)` to get ticket dir
2. Resolve ticket dir (config override or default `docs/active/tickets`)
3. Check ticket dir exists, error if not
4. `scan_tickets(ticket_dir)`
5. Handle empty tickets case (print message, exit 0)
6. `Dag::from_tickets(tickets)` — map DagError to string on failure (exit 1)
7. Check cycles — if found, print cycle info and return Err (exit 1)
8. Print summary: ticket count, edge count, critical path length
9. Compute execution_waves()
10. For each wave, print header + ticket lines
11. Print "Ready to schedule" footer

Per-ticket line format:
```
  {id:<12}  {phase:<12}  {status:<12}  {title}    deps: {deps}  blocks: {blocks}
```

Tests:
- `test_status_no_tickets` — empty dir, prints "No tickets found"
- `test_status_single_ticket` — one ready ticket, wave 0
- `test_status_dependency_chain` — multiple waves
- `test_status_cycle_error` — returns Err
- `test_status_missing_dep_error` — returns Err
- `test_status_respects_config` — uses custom ticket dir from .lisa.toml

Verify: `cargo test -p lisa-cli`

## Step 4: Wire into main.rs

In `crates/lisa-cli/src/main.rs`:
1. Add `mod status;`
2. Add `Status { path: PathBuf }` variant to `Commands` enum
3. Add match arm in `main()` calling `status::run_status()`

Verify: `cargo test --workspace` (all tests pass)

## Step 5: Update ticket phase

Update T-006-01 frontmatter to `phase: implement`, then to `phase: done` after verification.

Final verify: `cargo test --workspace` — all tests pass, no new warnings.

## Testing strategy

- **Unit tests for Display impls**: verify string output matches expected lowercase names.
- **Unit tests for DAG methods**: test edge_count and execution_waves with various graph shapes (empty, chain, diamond, parallel, cycle).
- **Integration-style tests for status module**: create temp dirs with ticket files, run `run_status()`, verify it succeeds or fails as expected. Check output indirectly by verifying Ok/Err return values (not stdout capture — keeping tests simple).
- **Manual test**: run `lisa status` against the actual project's `docs/active/tickets/` directory.
