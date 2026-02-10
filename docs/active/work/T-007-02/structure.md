# Structure: T-007-02 validate-pre-loop-readiness

## Files Modified

### 1. `crates/lisa-cli/src/main.rs`

**Change**: Add `--check-tools` flag to `Commands::Validate` variant.

```rust
Commands::Validate {
    #[arg(long, default_value = ".")]
    path: PathBuf,
    /// Also check that zellij and claude are on PATH
    #[arg(long)]
    check_tools: bool,
}
```

Update the match arm to pass `check_tools` to `run_validate`.

### 2. `crates/lisa-cli/src/loop_cmd.rs`

**Change**: Make `which` function `pub(crate)`.

```rust
pub(crate) fn which(name: &str) -> bool { ... }
```

No other changes.

### 3. `crates/lisa-cli/src/init.rs`

**Change**: Rewrite `run_validate` to be the comprehensive pre-loop readiness check.

Signature: `pub fn run_validate(root: &Path, check_tools: bool) -> Result<(), String>`

Internal structure:
```
fn run_validate(root, check_tools):
    let mut errors = Vec::new()
    let mut warnings = Vec::new()

    // 1. Tool checks (conditional on check_tools)
    if check_tools:
        check zellij on PATH -> error if missing
        check claude on PATH -> error if missing

    // 2. CLAUDE.md
    if !root.join("CLAUDE.md").exists() -> error

    // 3. docs/rdspi-workflow.md
    if !root.join("docs/rdspi-workflow.md").exists() -> error

    // 4. .lisa.toml
    load_config(root) -> errors/warnings

    // 5. Ticket directory
    resolve ticket_dir from config (default: docs/active/tickets)
    if !ticket_dir.exists() -> error, return early

    // 6. Scan tickets with diagnostics
    scan_tickets_with_diagnostics(ticket_dir)
    for each parse error -> error
    if no tickets found -> error

    // 7. DAG
    Dag::from_tickets -> handle MissingDependency, CycleDetected
    dag.detect_cycles() -> error if cycle

    // 8. Ready tickets
    if dag.get_ready_tickets().is_empty() -> error

    // 9. Acceptance criteria (warning)
    for each ticket without "Acceptance Criteria" -> warning

    // 10. Output
    print errors grouped
    print warnings grouped
    print summary: "Ready for `lisa loop`" or "N error(s) must be fixed"
    return Ok(()) if no errors, Err(...) if errors
```

Update `run_init` call: `run_validate(root, false)`.

## Files NOT Modified

- `crates/lisa-core/` — no changes needed. `scan_tickets_with_diagnostics` and `Dag::get_ready_tickets()` already exist.
- `crates/lisa-plugin/` — no changes.
- `crates/lisa-cli/src/config.rs` — already has `load_config`.
- `crates/lisa-cli/src/status.rs` — independent command.

## Public Interface Changes

| Change | Before | After |
|--------|--------|-------|
| `run_validate` signature | `(root: &Path)` | `(root: &Path, check_tools: bool)` |
| `loop_cmd::which` visibility | `fn` (private) | `pub(crate) fn` |
| CLI `lisa validate` | `--path` only | `--path` + `--check-tools` |

## Module Dependencies

```
main.rs -> init::run_validate(path, check_tools)
         -> loop_cmd::which (for tool checks)
         -> config::load_config (for ticket_dir resolution)
         -> lisa_core::ticket::scan_tickets_with_diagnostics
         -> lisa_core::dag::Dag::from_tickets, detect_cycles, get_ready_tickets
```

No new crate dependencies. No new modules.
