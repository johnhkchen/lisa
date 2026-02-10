# T-006-02 Design: Plugin Startup Diagnostics

## Problem

When the plugin loads, `rebuild_dag()` silently swallows per-file parse errors
and never runs cycle detection. If something is wrong, the operator gets no
signal except "nothing is happening."

## Decision 1: How to Surface Per-File Parse Errors

### Option A: Modify `scan_tickets` to return errors alongside successes

Add a new return type like `ScanResult { tickets: Vec<Ticket>, errors: Vec<(PathBuf, TicketError)> }`.
This changes the existing API surface and requires updating all callers.

### Option B: Add a parallel `scan_tickets_with_errors` function

Leave `scan_tickets` unchanged. Add a new function that returns both tickets and
errors. The plugin calls this in the diagnostic path; the CLI `validate` command
can also use it.

### Option C: Callback-based approach

Pass a closure `on_error: impl FnMut(PathBuf, TicketError)` to scan_tickets.

### Decision: Option B

Option B is lowest risk. It doesn't change existing behavior. The CLI validate
command already wants this (it currently duplicates parse logic). Option A requires
a breaking change to the return type. Option C is more Rustic but harder to test.

The function signature:
```rust
pub fn scan_tickets_with_diagnostics<P: AsRef<Path>>(dir: P)
    -> Result<ScanResult, TicketError>

pub struct ScanResult {
    pub tickets: Vec<Ticket>,
    pub errors: Vec<(PathBuf, TicketError)>,
}
```

The `Result` error case is still reserved for "can't read the directory at all."
Per-file errors go in `errors`.

## Decision 2: Where to Run Diagnostics

### Option A: Inline in `load()`

Add diagnostic code directly to load(). Simple but makes load() longer.

### Option B: Dedicated `run_startup_diagnostics()` method on State

Encapsulates all diagnostic logic. Called from load() after rebuild_dag().
Testable if we make it take inputs rather than reading from self.

### Option C: Free function in a new diagnostics module

Pure function: takes config + scan result + dag → Vec<ActivityEvent>. Maximally
testable, no zellij dependency. Called from load() which feeds it the inputs.

### Decision: Option C

A pure function `startup_diagnostics(config, scan_result, dag_result) -> Vec<ActivityEvent>`
can be tested thoroughly without mocking anything. It lives in lisa-core since
it only depends on types from that crate. The plugin's load() calls it and feeds
the events into `log_activity()`.

Location: `lisa-core/src/diagnostics.rs` (new module).

## Decision 3: ActivityEvent::Warning Variant

Need a `Warning` variant. Two options:

### Option A: Add `Warning { message: String }` to ActivityEvent

Symmetric with `Error` and `Info`. Clean.

### Option B: Reuse `Info` with a "[WARN]" prefix

Avoids adding a variant but loses semantic meaning.

### Decision: Option A

Add `Warning { message: String }`. The UI layer already has `ActivityType::Warning`
rendering with yellow icon. Just need the source variant and the conversion in
`activity_event_to_ui_entry`.

## Decision 4: Edge Count in DAG

The acceptance criteria ask for "DAG edge count" in the startup summary. The
`depends_on` HashMap is private in `Dag`.

### Option A: Add `edge_count()` method to Dag

Simple accessor: sum of all HashSet sizes in `depends_on`.

### Option B: Add edge_count to DagStats

Extend the existing stats struct.

### Decision: Option A

A simple `edge_count()` method is more granular and doesn't bloat `DagStats` with
info only needed at startup. It's one line.

## Decision 5: Cycle Detection at Load Time

Currently `Dag::from_tickets()` does NOT detect cycles — it only validates that
referenced dependencies exist. Cycles are detectable via `dag.detect_cycles()`
but nobody calls it at startup.

### Decision

Call `dag.detect_cycles()` in the diagnostics function. If cycles found, emit
`Error` events. This is cheap (Kahn's algorithm, linear time).

Note: `Dag::from_tickets()` succeeds even with cycles. The cycle just means some
tickets will never become ready (their in-degree never reaches zero). Logging this
is high value — it explains why tickets are "stuck" before they even start.

## Decision 6: Commit Lock Path

The acceptance criteria mention logging "commit lock path." The commit lock is
currently in `scheduler.rs`'s `CommitLock`, not in `PluginConfig`. The plugin
(lib.rs) doesn't use `Scheduler` at all — it does its own scheduling.

### Decision

Log the commit lock path as a derived value: `{repo_root}/.ralph-commit.lock`.
In the WASI plugin, repo_root is `/host`. For the diagnostic function, pass the
commit lock path as a parameter.

## Summary of Approach

1. **lisa-core/ticket.rs**: Add `ScanResult` struct and `scan_tickets_with_diagnostics()`.
2. **lisa-core/types.rs**: Add `ActivityEvent::Warning { message: String }`.
3. **lisa-core/dag.rs**: Add `Dag::edge_count()` method.
4. **lisa-core/diagnostics.rs** (new): Pure function `startup_diagnostics()` that
   takes config, scan result, and DAG result, returns `Vec<ActivityEvent>`.
5. **lisa-plugin/lib.rs**: Update `load()` to call the diagnostics function.
   Add `Warning` mapping in `activity_event_to_ui_entry()`.
6. **Tests**: In `lisa-core/diagnostics.rs` — test all four scenarios: clean load,
   parse errors, cycles, no tickets. Plus unit tests for `edge_count()` and
   `scan_tickets_with_diagnostics()`.
