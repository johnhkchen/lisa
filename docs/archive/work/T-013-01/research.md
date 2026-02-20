# Research: T-013-01 — `lisa doctor` subcommand

## Existing CLI Architecture

### Module pattern
Each subcommand lives in its own module under `crates/lisa-cli/src/`:
- `init.rs` — `run_init(&Path, bool) -> Result<(), String>`
- `loop_cmd.rs` — `run_loop(&Path, &ResolvedConfig, bool) -> Result<(), String>`
- `status.rs` — `run_status(&Path) -> Result<(), String>`
- `setup_guide.rs` — `run_setup_guide(&Path) -> Result<(), String>`

All modules are declared in `main.rs` with `mod` statements. The `Commands` enum uses Clap derive. Dispatch happens in `main()` via match. Error pattern: print to stderr, `exit(1)`.

### Existing tool-check code
`loop_cmd.rs` already has two relevant functions:
- `which(name: &str) -> bool` — shells out to `which` command, returns success/failure
- `check_binary(name, install_hint) -> Result<(), String>` — wraps `which()` with error message

`init.rs` validate function also checks tools when `--check-tools` flag is set (lines 345-362), calling `crate::loop_cmd::which()`.

### Version extraction
Neither existing code path extracts version strings. They only check presence. `lisa doctor` needs version output, which means running `zellij --version` and `claude --version` and capturing stdout.

## Dependencies in Cargo.toml

`lisa-cli` depends on: `lisa-core`, `clap` (derive), `toml`, `serde`, `serde_json`. Dev: `tempfile`. No additional crates needed — `std::process::Command` is sufficient for running commands and capturing output.

## Binary locations and version formats

1. **zellij**: `zellij --version` outputs e.g. `zellij 0.43.0`. Install via `cargo install zellij` or from zellij.dev.
2. **claude**: `claude --version` outputs version info. Install from docs.anthropic.com.
3. **rustup** (optional): `rustup target list --installed` lists installed targets; grep for `wasm32-wasip1`. Only relevant if rustup is in PATH (binary installs won't have it).

## Testing patterns

All CLI tests use `tempfile::tempdir()`. The existing `which()` function shells out to the real system, making it inherently environment-dependent. The ticket asks for mock-friendly design — trait or closure for command execution.

Two testable concerns:
1. **Check logic**: Given a command runner, does each check correctly report found/missing/version?
2. **Output formatting**: Given check results, is the output formatted correctly?
3. **Exit code**: All required checks pass → 0; any required check fails → 1.

## Relevant files to modify/create

- **Create**: `crates/lisa-cli/src/doctor.rs` — new module
- **Modify**: `crates/lisa-cli/src/main.rs` — add `mod doctor`, `Doctor` variant, dispatch

## Constraints

- `which()` in `loop_cmd.rs` is `pub(crate)`, already accessible from a new `doctor.rs` module
- The WASM target check is optional — skip gracefully if `rustup` not in PATH
- Exit code 0 vs 1 is the contract; `doctor` itself doesn't need `--path` since it checks system binaries, not project structure
