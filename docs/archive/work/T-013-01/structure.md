# Structure: T-013-01 — `lisa doctor` subcommand

## Files to create

### `crates/lisa-cli/src/doctor.rs`

New module. Contains all doctor logic.

**Public interface:**
- `pub fn run_doctor() -> Result<(), String>` — entry point called from main.rs

**Internal types:**
```
struct DependencyCheck {
    name: &'static str,
    required: bool,
    check: Box<dyn Fn() -> CheckResult>,
}

enum CheckResult {
    Found { version: String },
    NotFound { install_hint: String },
    Skipped { reason: String },
}

struct CheckReport {
    name: &'static str,
    required: bool,
    result: CheckResult,
}
```

**Internal functions:**
- `fn build_checks() -> Vec<DependencyCheck>` — constructs production checks
- `fn run_checks(checks: Vec<DependencyCheck>) -> Vec<CheckReport>` — executes each check
- `fn format_report(reports: &[CheckReport]) -> String` — builds formatted output
- `fn has_failures(reports: &[CheckReport]) -> bool` — true if any required check is NotFound
- `fn get_command_version(cmd: &str, args: &[&str]) -> Option<String>` — helper: runs command, captures stdout first line
- `fn check_zellij() -> CheckResult` — checks zellij presence and version
- `fn check_claude() -> CheckResult` — checks claude presence and version
- `fn check_wasm_target() -> CheckResult` — checks rustup + wasm32-wasip1

## Files to modify

### `crates/lisa-cli/src/main.rs`

1. Add `mod doctor;` to module declarations (line ~1)
2. Add `Doctor` variant to `Commands` enum:
   ```rust
   /// Check that all runtime dependencies are installed
   Doctor,
   ```
3. Add dispatch in main match:
   ```rust
   Commands::Doctor => {
       if let Err(e) = doctor::run_doctor() {
           eprintln!("Error: {}", e);
           std::process::exit(1);
       }
   }
   ```

## Module boundaries

- `doctor.rs` is self-contained. It does NOT import from `loop_cmd.rs` — it has its own `get_command_version()` helper that captures output (not just presence check).
- No changes to `lisa-core` or `lisa-plugin`.
- No new dependencies in Cargo.toml.

## Test organization

All tests in `doctor.rs` `#[cfg(test)] mod tests`. Test strategy:
- Build `DependencyCheck` with mock closures returning fixed `CheckResult` values
- Pass to `run_checks()` and `format_report()`
- Assert output formatting and failure detection
- No integration tests that depend on real binaries being installed
