# Structure: T-006-01 lisa-status-cli-command

## Files modified

### `crates/lisa-core/src/dag.rs`
Add two methods to `Dag`:

```rust
/// Returns the total number of dependency edges in the DAG.
pub fn edge_count(&self) -> usize

/// Groups tickets into execution waves (levels of the topological sort).
/// Wave 0: tickets with no dependencies.
/// Wave N: tickets whose maximum dependency wave is N-1.
/// Returns Err(CycleDetected) if the graph has cycles.
pub fn execution_waves(&self) -> Result<Vec<Vec<TicketId>>, DagError>
```

Add tests for both methods in the existing `mod tests` block.

### `crates/lisa-core/src/types.rs`
Add `Display` impl for `Phase` and `TicketStatus`:

```rust
impl fmt::Display for Phase { ... }      // "ready", "research", etc.
impl fmt::Display for TicketStatus { ... } // "open", "in_progress", etc.
```

### `crates/lisa-cli/src/status.rs` (new file)
New module containing:

```rust
/// Run the status command: scan tickets, build DAG, print status.
pub fn run_status(root: &Path) -> Result<(), String>
```

Internal helpers:
- Format the summary header (ticket count, edge count, cycles, critical path)
- Format each wave section
- Format individual ticket lines
- Format the "ready to schedule" footer

Tests using tempfile fixtures.

### `crates/lisa-cli/src/main.rs`
- Add `mod status;`
- Add `Status` variant to `Commands` enum with `--path` arg
- Add match arm calling `status::run_status()`

## Files NOT modified

- `crates/lisa-plugin/` — no changes, this is CLI-only
- `crates/lisa-cli/src/init.rs` — validate stays separate
- `crates/lisa-cli/src/config.rs` — reused as-is
- `crates/lisa-cli/Cargo.toml` — no new dependencies needed

## Module boundaries

```
main.rs
  └── Commands::Status { path }
        └── status::run_status(&path)
              ├── config::load_config(&path) → ticket_dir
              ├── lisa_core::ticket::scan_tickets(ticket_dir)
              ├── lisa_core::dag::Dag::from_tickets(tickets)
              ├── dag.detect_cycles()
              ├── dag.execution_waves()
              ├── dag.edge_count()
              ├── dag.stats()
              └── dag.critical_path()
```

## Public interface changes

### lisa-core (dag.rs)
- `Dag::edge_count(&self) -> usize` — new public method
- `Dag::execution_waves(&self) -> Result<Vec<Vec<TicketId>>, DagError>` — new public method

### lisa-core (types.rs)
- `impl Display for Phase` — new trait impl
- `impl Display for TicketStatus` — new trait impl

### lisa-cli
- `status::run_status(root: &Path) -> Result<(), String>` — new public function
- `Commands::Status` — new CLI subcommand variant

## Ordering of changes
1. Add `Display` impls to types.rs (no dependencies)
2. Add `edge_count()` and `execution_waves()` to dag.rs (no dependencies)
3. Create status.rs (depends on 1 and 2)
4. Wire into main.rs (depends on 3)
