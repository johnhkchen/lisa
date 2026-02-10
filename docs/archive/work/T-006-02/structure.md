# T-006-02 Structure: Plugin Startup Diagnostics

## Files Modified

### 1. `crates/lisa-core/src/ticket.rs`

**Add:**
- `ScanResult` struct:
  ```rust
  pub struct ScanResult {
      pub tickets: Vec<Ticket>,
      pub errors: Vec<(PathBuf, TicketError)>,
  }
  ```
- `scan_tickets_with_diagnostics<P: AsRef<Path>>(dir: P) -> Result<ScanResult, TicketError>`:
  Same logic as `scan_tickets` but collects errors into `ScanResult.errors`
  instead of eprintln-ing them. The `Result` error case remains for directory-level
  I/O failures.

Existing `scan_tickets` is unchanged (backward compatible).

### 2. `crates/lisa-core/src/types.rs`

**Add variant to `ActivityEvent`:**
```rust
/// A warning (not an error, but something the operator should notice)
Warning { message: String },
```

### 3. `crates/lisa-core/src/dag.rs`

**Add method to `Dag`:**
```rust
/// Returns the total number of dependency edges in the DAG.
pub fn edge_count(&self) -> usize {
    self.depends_on.values().map(|deps| deps.len()).sum()
}
```

**Extend `DagStats`** (optional, since edge_count is on Dag directly).

### 4. `crates/lisa-core/src/diagnostics.rs` (NEW)

**New module** with a single public function:

```rust
pub struct DiagnosticInput {
    pub config: &PluginConfig,
    pub scan_result: &ScanResult,
    pub dag_result: Result<&Dag, &DagError>,
    pub commit_lock_path: &Path,
}

pub fn startup_diagnostics(input: DiagnosticInput) -> Vec<ActivityEvent>
```

Logic:
1. Log `Info` with config values: ticket_dir, max_threads, commit_lock_path
2. If `scan_result.errors` is non-empty, log each as `Error` with filename + error
3. If `scan_result.tickets` is empty, log `Warning` "No tickets found"
4. If `dag_result` is `Err`, log `Error` with DAG error details
5. If `dag_result` is `Ok(dag)`:
   a. Run `dag.detect_cycles()` — if cycle found, log `Error` with cycle path
   b. Log `Info` summary: ticket count, edge count, ready count, max_threads

**Tests in module:**
- `test_diagnostics_clean_load` — 3 tickets, no errors → Info events only
- `test_diagnostics_parse_errors` — 2 good + 1 bad ticket → Error for bad + Info summary
- `test_diagnostics_cycles` — cyclic DAG → Error with cycle nodes
- `test_diagnostics_no_tickets` — empty scan → Warning "no tickets found"

### 5. `crates/lisa-core/src/lib.rs`

**Add:**
```rust
pub mod diagnostics;
```

### 6. `crates/lisa-plugin/src/lib.rs`

**Modify `load()`:**
After `rebuild_dag()`, call diagnostics. The current `rebuild_dag()` does its own
scan_tickets call. For startup, we call `scan_tickets_with_diagnostics` first,
then pass results to both the DAG builder and diagnostics function.

Refactored flow:
```
load():
  1. parse config, prefix paths
  2. subscribe/request_permissions
  3. scan_result = scan_tickets_with_diagnostics(ticket_dir)
  4. dag_result = Dag::from_tickets(scan_result.tickets)
  5. diagnostics = startup_diagnostics(config, scan_result, dag_result)
  6. for event in diagnostics { self.log_activity(event) }
  7. self.dag = dag (on success)
  8. self.initialized = true
  9. log PluginStarted
```

This replaces the call to `self.rebuild_dag()` in load() with inline logic, since
rebuild_dag() does its own scan_tickets and doesn't report per-file errors. The
rebuild_dag() method is unchanged — it's still used by poll_tick().

**Modify `activity_event_to_ui_entry()`:**
Add case for `ActivityEvent::Warning`:
```rust
ActivityEvent::Warning { message } => ui::ActivityType::Warning {
    ticket_id: String::new(),
    message: message.clone(),
},
```

## Files NOT Modified

- `crates/lisa-plugin/src/ui.rs` — already has `ActivityType::Warning` rendering
- `crates/lisa-plugin/src/scheduler.rs` — not involved
- `crates/lisa-cli/` — not involved (could use `scan_tickets_with_diagnostics` in
  `validate` later, but that's a separate ticket)

## Module Dependency Graph

```
diagnostics.rs  →  types.rs (ActivityEvent, PluginConfig)
                →  ticket.rs (ScanResult)
                →  dag.rs (Dag, DagError, CycleDetectionResult)
```

No new crate dependencies. All within lisa-core.

## Public Interface Summary

New public items:
- `lisa_core::ticket::ScanResult`
- `lisa_core::ticket::scan_tickets_with_diagnostics()`
- `lisa_core::diagnostics::startup_diagnostics()`
- `lisa_core::diagnostics::DiagnosticInput`
- `lisa_core::types::ActivityEvent::Warning`
- `lisa_core::dag::Dag::edge_count()`
