# Research: T-007-02 validate-pre-loop-readiness

## Current Validate Implementation

`run_validate` lives in `crates/lisa-cli/src/init.rs:164-298`. It returns `Result<(), String>` — `Ok(())` on pass, `Err(count)` on failure, which main.rs maps to exit code 1.

### What it currently checks

| Check | Severity | Location |
|-------|----------|----------|
| CLAUDE.md exists | error | init.rs:169 |
| docs/rdspi-workflow.md exists | **warning** | init.rs:174 |
| .lisa.toml valid if present | error/warning | init.rs:179-192 |
| Required dirs exist (tickets, stories, work) | warning | init.rs:195-200 |
| Tickets parse successfully | warning (via scan_tickets) | init.rs:204-274 |
| Acceptance criteria section present | warning | init.rs:213-220 |
| DAG is acyclic | error | init.rs:228-238 |
| No missing dependencies | error | init.rs:253-260 |

### Output behavior

Currently mixes informational prints (`"Found N ticket(s)"`, `"DAG validation: no cycles detected"`, `"DAG stats: ..."`) with grouped warnings/errors at the end. Not cleanly sectioned.

## What the Ticket Requires vs What Exists

### Gap 1: rdspi-workflow.md is warning, should be error
- Line 174: `warnings.push(...)` — ticket says this should be an error

### Gap 2: No "at least one .md file" check
- Current behavior: prints `"No tickets found in docs/active/tickets/"` to stdout, but this is not an error. Ticket says ticket directory must contain at least one .md file.

### Gap 3: No ticket parse error surfacing
- `scan_tickets` (ticket.rs:316) silently skips parse failures with `eprintln`. The `scan_tickets_with_diagnostics` variant (ticket.rs:363) returns per-file errors but is only used by the plugin's diagnostics module, not by validate.

### Gap 4: No "at least one ready ticket" check
- The DAG stats show `ready_tickets` count but there's no error if it's zero. Ticket requires: "At least one ticket has `phase: ready` and all deps satisfied."

### Gap 5: No `--check-tools` flag
- `loop_cmd.rs` has `check_binary()` (line 130) and `which()` (line 137) for zellij/claude. These are private to loop_cmd. Validate doesn't have access to them and doesn't check for tools.

### Gap 6: Output not grouped
- Current output mixes println info lines with the error/warning sections. Ticket requires: errors first, then warnings, then summary.

### Gap 7: Summary line
- Currently says `"Validation passed."`. Ticket wants `"Ready for \`lisa loop\`"` or `"N errors must be fixed"`.

## Related Code and Reuse Opportunities

### `check_binary` / `which` in loop_cmd.rs
```rust
fn check_binary(name: &str, install_hint: &str) -> Result<(), String> { ... }
fn which(name: &str) -> bool { ... }
```
These are `fn` (private). They could be made `pub(crate)` for reuse in init.rs, or the logic could be duplicated (it's trivial: calls `which` command via Command).

### `scan_tickets_with_diagnostics` in ticket.rs
Returns `ScanResult { tickets, errors }` — exactly what validate needs to report per-file parse errors properly instead of swallowing them.

### `Dag::get_ready_tickets()` in dag.rs
Returns `Vec<TicketId>` of tickets where `can_start()` is true (startable phase + all deps done). This is the check for "at least one ticket has phase: ready and all deps satisfied."

### `config::load_config` in config.rs
Already called by validate. Returns `ConfigValidation { config, warnings }`. The validate function currently uses the default ticket dir, not the one from config. This should be aligned with how `run_status` does it (status.rs:10-17).

### CLI definition in main.rs
`Commands::Validate` currently has only `--path`. Need to add `--check-tools` flag.

## Existing Test Coverage

init.rs tests (7 tests):
- `test_validate_missing_claude_md` — error case
- `test_validate_valid_setup` — happy path
- `test_validate_valid_lisa_toml` — config valid
- `test_validate_invalid_lisa_toml` — config error
- `test_validate_with_tickets` — tickets parse ok
- `test_validate_detects_missing_dependency` — DAG error
- (init tests also exercise validate indirectly)

Missing test coverage per acceptance criteria:
- Missing rdspi-workflow.md as error (not warning)
- No tickets in directory
- No ready tickets
- Ticket parse error surfacing
- `--check-tools` flag behavior
- Output grouping (harder to test via return value)

## Constraints

- `run_validate` is called by both `Commands::Validate` and `run_init` (at the end of init). Changes must work for both call sites.
- The function signature `pub fn run_validate(root: &Path) -> Result<(), String>` may need to accept a `check_tools: bool` parameter or the flag could be handled in main.rs with a separate call.
- The `which` approach uses the system `which` command — works on macOS and Linux but not Windows. This is fine since zellij itself is unix-only.
