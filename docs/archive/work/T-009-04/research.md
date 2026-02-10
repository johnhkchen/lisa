# Research: T-009-04 LLM-Driven Validate Loop

## Current Validate Implementation

### Entry Point
- `main.rs:80-85` — `Commands::Validate` calls `init::run_validate(&path, check_tools)`. On `Err(e)`, prints `"Error: {e}"` to stderr and exits with code 1. On `Ok(())`, exits 0.
- Exit code semantics already correct: 0 = pass, 1 = fail.

### Validation Logic (`init.rs:237-426`)
`run_validate(root, check_tools) -> Result<(), String>` collects two parallel vectors:
- `errors: Vec<String>` — blocking problems
- `warnings: Vec<String>` — informational notes

#### Checks in order:
1. **Tool checks** (optional, `check_tools` flag): zellij and claude on PATH
2. **CLAUDE.md exists**
3. **docs/rdspi-workflow.md exists**
4. **`.lisa.toml` validation** — parse config, surface warnings, extract ticket_dir
5. **Hook infrastructure**:
   - `.claude/settings.local.json` exists and contains `idle_prompt`
   - `.lisa/hooks/on-idle.sh` exists and is executable (unix)
6. **Directory structure** — `docs/active/stories`, `docs/active/work` (warnings only)
7. **Ticket directory** — must exist, early return on missing
8. **Ticket scanning** — `scan_tickets_with_diagnostics()`, surfaces per-file parse errors
9. **At least one ticket** — early return if empty
10. **Acceptance criteria** — warning if ticket body lacks "Acceptance Criteria"
11. **DAG construction** — `Dag::from_tickets()`, catches `MissingDependency` and `CycleDetected`
12. **Cycle detection** — `dag.detect_cycles()` after successful construction
13. **Ready tickets** — at least one ticket must be ready with deps satisfied

### Output Format (`init.rs:429-456`)
`print_results(errors, warnings)` produces:
```
Errors:
  x error message 1
  x error message 2

Warnings:
  ! warning message 1

3 error(s) must be fixed.
```

When clean: `Ready for \`lisa loop\`.`

### Problems for LLM Consumption
1. **No file paths** — Most error messages don't include the file path. Tool check errors, CLAUDE.md existence, settings.json, hook checks all mention what's wrong but with inconsistent path format.
2. **No category tag** — No structured prefix like `frontmatter:`, `dependency:`, `structure:`.
3. **Grouped output** — Errors printed under "Errors:" header with `  x ` prefix, not one-per-line with path:category:message.
4. **Warnings mixed in** — Some checks that should arguably be errors are warnings (missing directories).
5. **Per-file ticket parse errors** — `init.rs:361-366` uses filename only (not relative path), and formats as `"Failed to parse {filename}: {error}"`.
6. **No color/TTY handling** — No `--no-color` flag or TTY detection currently exists.
7. **Summary line** — Currently `"N error(s) must be fixed."` — no ticket count, no ready count.

## Error Source Analysis

### Where file paths are known:
- Ticket parse errors: have `PathBuf` from `scan.errors` — currently only uses filename
- Ticket acceptance criteria: have `ticket.id` — could use `ticket.file_path`
- DAG errors: have `ticket_id` and `missing_dep` strings — not file paths
- `.lisa.toml` errors: path is known (`root.join(".lisa.toml")`)
- Hook files: paths are known (`.lisa/hooks/on-idle.sh`, `.claude/settings.local.json`)
- CLAUDE.md: path known
- rdspi-workflow.md: path known

### Where categories map:
| Check | Path | Category |
|-------|------|----------|
| Tool check (zellij, claude) | `(tools)` | `config` |
| CLAUDE.md missing | `CLAUDE.md` | `structure` |
| rdspi-workflow.md missing | `docs/rdspi-workflow.md` | `structure` |
| .lisa.toml parse error | `.lisa.toml` | `config` |
| .lisa.toml unknown keys | `.lisa.toml` | `config` (warning) |
| settings.local.json missing | `.claude/settings.local.json` | `structure` |
| settings.local.json no idle_prompt | `.claude/settings.local.json` | `config` |
| on-idle.sh missing | `.lisa/hooks/on-idle.sh` | `structure` |
| on-idle.sh not executable | `.lisa/hooks/on-idle.sh` | `structure` |
| Optional dirs missing | `docs/active/stories` etc. | `structure` (warning) |
| Ticket dir missing | `docs/active/tickets` | `structure` |
| Ticket parse error | `docs/active/tickets/T-xxx.md` | `frontmatter` |
| No tickets found | `docs/active/tickets/` | `readiness` |
| Missing acceptance criteria | `docs/active/tickets/T-xxx.md` | `frontmatter` (warning) |
| Missing dependency | `docs/active/tickets/T-xxx.md` | `dependency` |
| Cycle detected | `docs/active/tickets/` | `dependency` |
| No ready tickets | `docs/active/tickets/` | `readiness` |

## Key Interfaces

### `TicketError` (`ticket.rs:14-27`)
Already has structured variants: `MissingField(String)`, `InvalidField { field, value, reason }`, `MissingFrontmatter`, `YamlParse(String)`, `Io(io::Error)`, `InvalidPath(PathBuf)`.

### `DagError` (`dag.rs:37-46`)
Has `MissingDependency { ticket_id, missing_dep }` and `CycleDetected(Vec<TicketId>)`.

### `ScanResult` (`ticket.rs:347-353`)
Returns `tickets: Vec<Ticket>` and `errors: Vec<(PathBuf, TicketError)>` — per-file errors with paths.

## Existing Test Coverage
- `init.rs` has 20 tests covering validate scenarios: missing CLAUDE.md, valid setup, invalid .lisa.toml, tickets with dependencies, missing deps, cycles, empty ticket dir, no ready tickets, parse errors, acceptance criteria warnings, hook infrastructure, init-then-validate roundtrip.
- Tests assert `result.is_ok()` or `result.is_err()` — they don't assert on stdout content.

## Constraints and Boundaries
- `run_validate` returns `Result<(), String>` — the `String` in `Err` is currently just the summary line (`"N error(s) must be fixed."`).
- `main.rs` prints `"Error: {e}"` on Err and exits 1.
- The actual errors are printed to stdout by `print_results`, not returned in the Result.
- All paths used in errors are relative to `root` (e.g. `CLAUDE.md`, `docs/active/tickets/T-001.md`), which is what the ticket asks for.

## Summary
The core validation logic is solid and comprehensive. The change is purely an output format transformation: instead of collecting `Vec<String>` error messages and printing them grouped, we need to collect structured `ValidationError { path, category, message }` items and print them in the `{path}: {category}: {message}` format. The validation checks themselves don't change. The `Err` return from `run_validate` needs to propagate the exit code 1 behavior, which already works via `main.rs`.
