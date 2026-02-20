---
id: T-013-01
title: Implement lisa doctor subcommand
type: feature
phase: done
status: done
priority: high
story: S-013
created: 2026-02-20
depends_on: []
---

# T-013-01: Implement `lisa doctor` subcommand

## Objective

Add a `lisa doctor` command that checks the user's environment for all runtime dependencies Lisa needs and reports their status with actionable guidance.

## Requirements

### Dependencies to check

1. **Zellij** — look for `zellij` in PATH
   - If found: print version (`zellij --version`)
   - If missing: print install instructions (cargo install zellij, or link to zellij.dev)

2. **Claude Code** — look for `claude` in PATH
   - If found: print version (`claude --version`)
   - If missing: print install instructions (link to Claude Code install docs)

3. **WASM target** (optional, only for building from source) — check if `wasm32-wasip1` target is installed via `rustup target list --installed`
   - This check should only run if `rustup` is in PATH (skip gracefully for binary installs)

### Output format

```
lisa doctor

Checking dependencies...

  zellij     v0.43.0   OK
  claude     v1.2.3    OK

All dependencies satisfied.
```

Or on failure:

```
lisa doctor

Checking dependencies...

  zellij     not found
    Install: cargo install zellij
    Or visit: https://zellij.dev/documentation/installation

  claude     v1.2.3    OK

Some dependencies are missing. Lisa requires all of the above to run.
```

### Implementation

- Add `Doctor` variant to the CLI enum in `crates/lisa-cli/src/main.rs`
- Implement in a new `crates/lisa-cli/src/doctor.rs` module
- Use `std::process::Command` to check for binaries and get versions
- Exit code 0 if all required deps are present, exit code 1 otherwise
- Keep the check list extensible (vec of check structs/closures) for future additions

### Tests

- Test the individual check functions (mock-friendly: accept a closure or trait for "run command")
- Test output formatting
- Test exit code logic

## Acceptance Criteria

- [ ] `lisa doctor` runs and checks for `zellij` and `claude`
- [ ] Prints version info when dependencies are found
- [ ] Prints install instructions when dependencies are missing
- [ ] Exit code 0 when all required deps present, 1 otherwise
- [ ] `cargo test --workspace` passes with new tests
