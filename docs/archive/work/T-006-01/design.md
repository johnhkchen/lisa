# Design: T-006-01 lisa-status-cli-command

## Decision: Output structure

### Option A: Flat ticket list + summary header
Print a summary block (ticket count, edge count, cycles), then list each ticket with its fields, sorted by topo order.

### Option B: Wave-grouped display
Same summary, but group tickets into execution waves (Wave 0 = no deps, Wave 1 = depends only on Wave 0, etc.). Within each wave, tickets are listed alphabetically.

### Option C: Tree-like dependency view
Show the DAG as an indented tree structure.

**Decision: Option B (wave-grouped).** Waves directly answer the scheduling question ("what runs when?"). A flat list loses the parallelism information. A tree is harder to read with diamond dependencies (a ticket appearing multiple times). Waves map directly to how Lisa schedules work.

## Decision: Where to add wave computation

### Option A: Add `execution_waves()` to `Dag` in lisa-core
This is a general-purpose DAG operation that the plugin might also want.

### Option B: Compute waves in the CLI status module
Keep it local to the display logic. If the plugin needs it later, promote it.

**Decision: Option A.** Waves are a natural DAG property. Adding it to `Dag` keeps the status module focused on display. It also makes it testable independently. Also add an `edge_count()` method while we're there.

## Decision: Config loading

The status command should respect `.lisa.toml` ticket directory overrides. Reuse `config::load_config()` and `config::resolve_config()` just like the loop command does. The `--path` flag works the same way as other commands.

## Decision: Phase/Status display formatting

Add `Display` impls for `Phase` and `TicketStatus` in lisa-core/types.rs. These types are used across the codebase and lacking Display is an oversight. The display strings match the serde lowercase names.

## Decision: Module placement

New file: `crates/lisa-cli/src/status.rs`. Follows the pattern of `init.rs` and `loop_cmd.rs`. Contains `run_status(root: &Path) -> Result<(), String>`.

## Decision: Exit code semantics

- Exit 0: DAG is valid, status printed successfully
- Exit 1: DAG has errors (cycles, missing deps, no ticket dir, parse failures)

This matches the ticket's acceptance criteria and is consistent with `validate`.

## Output format

```
DAG: 5 tickets, 4 edges, no cycles
Critical path: 3 tickets

Wave 0 (ready to schedule):
  T-001  implement  in_progress  first-ticket
  T-002  ready      open         second-ticket

Wave 1 (depends on wave 0):
  T-003  ready      open         third-ticket    deps: T-001, T-002

Wave 2 (depends on wave 1):
  T-004  ready      open         fourth-ticket   deps: T-003
  T-005  ready      open         fifth-ticket    deps: T-003

Ready to schedule: T-001, T-002
```

Key formatting decisions:
- Columns: ID, phase, status, title, deps (if any)
- Pad columns for alignment
- "Ready to schedule" summary at the bottom for quick scanning
- If cycles exist, print them and exit 1

## Rejected alternatives

1. **JSON output flag**: Adds complexity. Not needed for v1. Can be added later if scripting use cases emerge.
2. **Color output**: Nice to have but not in acceptance criteria. Complicates testing (ANSI codes in assertions). Defer.
3. **Combining with validate**: The commands serve different purposes. Validate checks project setup correctness. Status shows scheduling state. Keep them separate.
4. **Showing blocked-by (computed)**: The acceptance criteria asks for `blocked_by (computed)`. Will show this in the per-ticket line as `blocks: T-005, T-006` to show what each ticket is blocking downstream.

## New methods on Dag

1. `edge_count(&self) -> usize` — sum of dependency set sizes
2. `execution_waves(&self) -> Result<Vec<Vec<TicketId>>, DagError>` — returns waves, Err if cycle
