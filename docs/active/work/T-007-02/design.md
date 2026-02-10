# Design: T-007-02 validate-pre-loop-readiness

## Decision Summary

Enhance `run_validate` in init.rs with a `check_tools` parameter, use `scan_tickets_with_diagnostics` for better error surfacing, collect all output into vectors before printing grouped, and add the missing checks.

## Options Considered

### Option A: Refactor validate into a structured result type
Create a `ValidationResult { errors, warnings, stats }` struct, separate the validation logic from output formatting, return the struct, and let the caller format.

**Pros**: Testable against the struct, reusable by other commands, clean separation.
**Cons**: Over-engineering for what the ticket asks. The validate command is a CLI tool — testing exit code and error presence is sufficient.

### Option B: Enhance run_validate in place (CHOSEN)
Keep the existing function shape but:
- Add `check_tools: bool` parameter
- Fix the severity levels (rdspi-workflow.md -> error)
- Add missing checks (no tickets, no ready tickets)
- Use `scan_tickets_with_diagnostics` instead of `scan_tickets`
- Collect all output, print grouped at the end
- Update summary line

**Pros**: Minimal change, focused on what the ticket asks, backward compatible.
**Cons**: The function does both validation and output — but that's acceptable for a CLI command handler.

### Option C: Extract tool checking into a shared module
Move `check_binary`/`which` from loop_cmd.rs into a shared module that both validate and loop can use.

**Rejected**: The `which` function is 5 lines. Making it `pub(crate)` in loop_cmd.rs is sufficient. No need for a new module.

## Chosen Approach: Option B with pub(crate) reuse

### Signature change

```rust
pub fn run_validate(root: &Path, check_tools: bool) -> Result<(), String>
```

This adds `check_tools` parameter. The `run_init` call site passes `false` (init shouldn't require zellij/claude to be installed). The `Commands::Validate` handler in main.rs passes the CLI flag value.

### CLI change

```rust
Commands::Validate {
    #[arg(long, default_value = ".")]
    path: PathBuf,
    #[arg(long)]
    check_tools: bool,
}
```

### Check additions (in order of validation)

1. **Tool checks** (if `check_tools` is true):
   - `zellij` on PATH -> error if missing
   - `claude` on PATH -> error if missing

2. **CLAUDE.md exists** -> error (existing)

3. **docs/rdspi-workflow.md exists** -> **error** (was warning)

4. **.lisa.toml valid** if present -> error/warning (existing, load via config::load_config)

5. **Ticket directory exists** -> error (upgrade from warning for the tickets dir specifically)

6. **Ticket directory has at least one .md file** -> error if empty

7. **All tickets parse** -> error for each parse failure (use scan_tickets_with_diagnostics)

8. **DAG acyclic, no missing deps** -> error (existing)

9. **At least one ready ticket** -> error if `dag.get_ready_tickets().is_empty()`

10. **Acceptance criteria section** -> warning per ticket (existing)

### Output grouping

Collect into `errors: Vec<String>`, `warnings: Vec<String>`, `info: Vec<String>`. Print in order:
1. Errors (prefixed with `  x `)
2. Warnings (prefixed with `  ! `)
3. Summary line

Remove intermediate println calls (like "Found N ticket(s)", "DAG validation: no cycles detected") — these become info items or are folded into the summary.

### Summary line

```
Ready for `lisa loop`    (0 errors)
N error(s) must be fixed (N > 0)
```

### Reuse from loop_cmd.rs

Make `which` pub(crate):
```rust
pub(crate) fn which(name: &str) -> bool { ... }
```

In init.rs, call `loop_cmd::which("zellij")` and `loop_cmd::which("claude")`.

### Config-aware ticket directory

Use `config::load_config` to resolve the ticket directory (matching how status.rs does it), rather than hardcoding `docs/active/tickets`. This handles custom `[dirs] tickets = "..."` in .lisa.toml.

## Test Plan

| Test | Asserts |
|------|---------|
| `test_validate_missing_claude_md` | existing, still passes |
| `test_validate_missing_rdspi_workflow` | **new** — error (was warning) |
| `test_validate_no_ticket_dir` | **new** — error |
| `test_validate_empty_ticket_dir` | **new** — error ("no tickets") |
| `test_validate_no_ready_tickets` | **new** — all tickets done or blocked |
| `test_validate_ticket_parse_error` | **new** — malformed ticket surfaces as error |
| `test_validate_ready_for_loop` | **new** — full valid setup, returns Ok |
| `test_validate_check_tools_skipped_by_default` | **new** — check_tools=false doesn't check tools |
| `test_validate_acceptance_criteria_warning` | **new** — ticket without AC triggers warning but not error |
| `test_validate_valid_setup` | existing, updated for new signature |
| `test_validate_with_tickets` | existing, updated |

## Risks

- Changing `run_validate` signature breaks `run_init` call site — trivial fix (pass `false`).
- Making `which` pub(crate) in loop_cmd.rs is a minor visibility change.
- The `which` command doesn't exist on all platforms — acceptable since zellij is unix-only.
