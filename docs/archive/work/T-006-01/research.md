# Research: T-006-01 lisa-status-cli-command

## Relevant Code Locations

### CLI entry point
- `crates/lisa-cli/src/main.rs` — Clap-derived `Cli` struct with `Commands` enum. Three subcommands: `Init`, `Validate`, `Loop`. Each takes `--path` (default `.`). `resolve_path()` helper canonicalizes relative paths. Pattern: match on `Commands` variant, call into module function, `exit(1)` on error.

### Existing validate command (closest analog)
- `crates/lisa-cli/src/init.rs:164-298` — `run_validate()` does:
  1. Check CLAUDE.md exists
  2. Check workflow file exists
  3. Validate .lisa.toml if present (via `config::load_config()`)
  4. Check directory structure
  5. Scan tickets via `lisa_core::ticket::scan_tickets()`
  6. Build DAG via `lisa_core::dag::Dag::from_tickets()`
  7. Detect cycles, print stats
  8. Collect errors/warnings, return `Err` if any errors
- Validate already calls `dag.stats()` which returns `DagStats { total_tickets, done_tickets, ready_tickets, in_progress_tickets, blocked_tickets, critical_path_length }`.
- Validate already handles the error cases (missing deps, cycles).

### Config resolution
- `crates/lisa-cli/src/config.rs` — `load_config()` reads `.lisa.toml`, `resolve_config()` merges defaults. `ResolvedConfig` has `ticket_dir` (default `"docs/active/tickets"`). The `status` command needs to know where tickets live; it can use the same config loading.

### Ticket parsing
- `crates/lisa-core/src/ticket.rs` — `scan_tickets(dir)` reads all `*.md` files from a directory, parses YAML frontmatter, returns `Vec<Ticket>`. Each `Ticket` has: `id`, `story`, `title`, `ticket_type`, `status`, `priority`, `phase`, `depends_on`, `blocks`, `file_path`, `content`.

### DAG computation
- `crates/lisa-core/src/dag.rs` — `Dag::from_tickets(tickets)` builds graph. Key methods:
  - `topological_sort()` — returns `Vec<TicketId>` in topo order (Kahn's algorithm)
  - `detect_cycles()` — returns `CycleDetectionResult::NoCycle` or `Cycle(Vec<TicketId>)`
  - `get_dependencies(id)` — returns `HashSet<TicketId>` (forward edges)
  - `get_blocked_by(id)` — returns `HashSet<TicketId>` (reverse edges)
  - `get_ready_tickets()` — tickets with all deps done + startable phase
  - `stats()` — returns `DagStats`
  - `critical_path()` — returns longest dependency chain
  - `tickets()` — iterator over all tickets
  - `len()` — ticket count

### Edge count
- The DAG stores `depends_on: HashMap<TicketId, HashSet<TicketId>>`. Edge count = sum of all set sizes. No existing method for this; needs to be added or computed externally.

### Execution waves
- No existing method. Concept: Wave 0 = tickets with no dependencies. Wave N = tickets whose max dependency wave is N-1. Computed via BFS over topo-sorted nodes. This is a new computation.

## Types involved
- `Ticket` (types.rs) — all frontmatter fields
- `Phase` (types.rs) — Ready, Research, Design, Structure, Plan, Implement, Review, Done
- `TicketStatus` (types.rs) — Open, InProgress, Blocked, Review, Done, Cancelled
- `Dag` (dag.rs) — graph structure
- `DagStats` (dag.rs) — summary stats
- `DagError` (dag.rs) — MissingDependency, CycleDetected
- `CycleDetectionResult` (dag.rs) — NoCycle, Cycle(Vec)

## Display formatting
- Phase and TicketStatus derive `Serialize` with `rename_all = "lowercase"`, but neither implements `Display`. The `status` command will need to format these as strings. Can use serde or match manually. `phase_to_string()` exists in ticket.rs but is private.

## Patterns and conventions
- CLI commands live in dedicated modules: `init.rs`, `loop_cmd.rs`. The `status` command should get its own `status.rs` module.
- Functions return `Result<(), String>` for CLI commands.
- Error output goes to stderr via `eprintln!`, normal output to stdout via `println!`.
- Tests use `tempfile::tempdir()` to create isolated filesystem fixtures.
- Exit code convention: 0 success, 1 error (set in main.rs).

## Constraints
- Must work without zellij (pure CLI) — already satisfied since lisa-core has no zellij dependency.
- Read-only diagnostic — no state mutation needed.
- The DAG's `depends_on` and `blocks` fields are private; access is through `get_dependencies()` and `get_blocked_by()`.
- Edge count: the Dag doesn't expose total edge count. Options: add a method to Dag, or compute from iterating `get_dependencies()` over all tickets.

## Dependencies
- No new crate dependencies needed. Everything builds on lisa-core types and existing CLI infrastructure.
- `tempfile` already in dev-dependencies for tests.

## Assumptions and open questions
- Ticket directory: should `status` respect `.lisa.toml` `dirs.tickets` override? Yes, for consistency with validate/loop. But validate currently hardcodes `docs/active/tickets`. The status command should load config and use the configured ticket dir.
- Output format: plain text to stdout. No color/ANSI codes needed for v1 (keeps it simple, pipe-friendly).
- Should `status` show the same validation warnings as `validate`? No — status focuses on DAG state, not project setup validation. Keep them separate.
