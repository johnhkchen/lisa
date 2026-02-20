# Review: T-013-01 — `lisa doctor` subcommand

## Changes made

Added `lisa doctor` subcommand that checks the user's environment for runtime dependencies and reports their status with actionable install guidance.

### Files created

- **`crates/lisa-cli/src/doctor.rs`** (~230 lines) — New module containing:
  - `CheckResult` enum: `Found { version }`, `NotFound { install_hint }`, `Skipped { reason }`
  - `DependencyCheck` struct with closure-based check execution (extensible via `Vec::push`)
  - `CheckReport` struct with `Display` impl for formatted output
  - Three production checks: zellij (required), claude (required), wasm32-wasip1 target (optional)
  - `run_doctor()` public entry point
  - 12 unit tests using mock closures — no dependency on real binaries being installed

### Files modified

- **`crates/lisa-cli/src/main.rs`** — Added `mod doctor`, `Doctor` variant to `Commands` enum, dispatch arm in `main()`

## Acceptance criteria status

- [x] `lisa doctor` runs and checks for `zellij` and `claude`
- [x] Prints version info when dependencies are found
- [x] Prints install instructions when dependencies are missing
- [x] Exit code 0 when all required deps present, 1 otherwise
- [x] `cargo test --workspace` passes with new tests (332 total, 0 failures)

## Test results

- `cargo test --workspace`: 332 tests pass (123 CLI, 78 core, 131 plugin)
- `cargo check -p lisa-plugin --target wasm32-wasip1`: clean (pre-existing ui.rs warnings only)

## Open concerns

- `is_on_path()` shells out to `which`, same approach as `loop_cmd::which()`. These two functions are duplicated rather than shared. Low priority — could consolidate later if desired.
- The optional WASM target check uses `rustup target list --installed` which may be slow on some systems. Not a concern in practice since it only runs when `rustup` is found.
- No integration test that actually runs `lisa doctor` end-to-end. Unit tests cover all logic paths via mock closures, which is sufficient for CI reliability.
