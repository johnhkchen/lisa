# Structure: T-009-04 LLM-Driven Validate Loop

## Files Modified

### `crates/lisa-cli/src/init.rs`
This is the only production file that changes.

#### New types (private to module, placed above `run_validate`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
struct ValidationDiagnostic {
    path: String,
    category: &'static str,
    message: String,
    severity: Severity,
}

/// Result of validation, structured for both display and testing.
struct ValidationResult {
    diagnostics: Vec<ValidationDiagnostic>,
    ticket_count: usize,
    ready_count: usize,
}
```

#### Functions changed:

1. **`run_validate(root, check_tools) -> Result<(), String>`** — Becomes a thin wrapper:
   - Calls `validate(root, check_tools)`
   - Calls `print_diagnostics(&result)`
   - Returns `Ok(())` or `Err(summary)`

2. **New: `validate(root, check_tools) -> ValidationResult`** — Contains all the existing validation logic from `run_validate`, but collects `Vec<ValidationDiagnostic>` instead of `Vec<String>`. Every `errors.push(...)` and `warnings.push(...)` becomes a `diagnostics.push(ValidationDiagnostic { ... })`.

3. **`print_results` → `print_diagnostics`** — Renamed and rewritten:
   - Iterates diagnostics, printing errors then warnings
   - Error format: `{path}: {category}: {message}`
   - Warning format: `{path}: {category} (warning): {message}`
   - Summary line: error count + fix instruction, or success with ticket stats

#### Category assignments per check:

| Check | `path` | `category` |
|---|---|---|
| zellij not found | `(tools)` | `config` |
| claude not found | `(tools)` | `config` |
| CLAUDE.md missing | `CLAUDE.md` | `structure` |
| rdspi-workflow.md missing | `docs/rdspi-workflow.md` | `structure` |
| .lisa.toml parse error | `.lisa.toml` | `config` |
| .lisa.toml warning | `.lisa.toml` | `config` |
| settings.local.json missing | `.claude/settings.local.json` | `structure` |
| settings.local.json no idle_prompt | `.claude/settings.local.json` | `config` |
| on-idle.sh missing | `.lisa/hooks/on-idle.sh` | `structure` |
| on-idle.sh not executable | `.lisa/hooks/on-idle.sh` | `structure` |
| Optional dirs missing | `docs/active/{dir}` | `structure` |
| Ticket dir missing | ticket_dir_rel | `structure` |
| Ticket parse error | relative ticket path | `frontmatter` |
| No tickets found | ticket_dir_rel + `/` | `readiness` |
| Missing acceptance criteria | relative ticket path | `frontmatter` |
| Missing dependency | relative ticket path | `dependency` |
| Cycle detected | ticket_dir_rel + `/` | `dependency` |
| No ready tickets | ticket_dir_rel + `/` | `readiness` |

#### Path computation:

For ticket file paths, use `ticket.file_path.strip_prefix(root).unwrap_or(&ticket.file_path)` to produce relative paths like `docs/active/tickets/T-001.md`.

For the diagnostics referencing parse errors from `scan.errors`, same: `path.strip_prefix(root)`.

For hardcoded paths (CLAUDE.md, etc.), use the string literal directly.

### `crates/lisa-cli/src/main.rs`
**No changes.** The `run_validate` function signature remains `Result<(), String>`, and `main.rs` already handles the exit code correctly.

## Files NOT Modified
- `crates/lisa-core/` — No changes. TicketError, DagError, ScanResult all stay as-is.
- `crates/lisa-plugin/` — No changes.
- `crates/lisa-cli/src/config.rs` — No changes.

## Public Interface
No public API changes. `run_validate` keeps its signature. The `validate` function and types are private to `init.rs`. This is purely an internal refactor of how errors are collected and printed.

## Test Strategy
- All existing tests continue to pass — they check `is_ok()` / `is_err()`, not stdout.
- New tests call `validate()` directly and assert on the returned `ValidationDiagnostic` structs:
  - Check that each diagnostic has the expected path and category
  - Check that error count matches expected
  - Check that a clean project returns zero errors
  - Check format string output via a `format_diagnostic` helper (or test `print_diagnostics` indirectly)
