---
id: T-011-01
title: Build and install lisa on a fresh device
type: chore
phase: ready
status: Todo
priority: high
story: S-011
created: 2026-02-11
depends_on: []
---

# T-011-01: Build and install lisa on a fresh device

## Objective

Verify the full build-from-source workflow on a second machine. This is a manual chore — no code changes expected, just validation that the documented steps work.

## Steps

1. **Clone the repo**
   ```bash
   git clone https://github.com/johnhkchen/lisa
   cd lisa
   ```

2. **Install prerequisites** (if not present)
   - Rust toolchain via `rustup`
   - WASM target: `rustup target add wasm32-wasip1`
   - `just` command runner
   - Zellij terminal multiplexer

3. **Build and install**
   ```bash
   just install
   ```

4. **Verify installation**
   ```bash
   which lisa
   lisa --help
   lisa --version   # if supported
   ```

5. **Run tests**
   ```bash
   cargo test --workspace
   ```

## Record any issues

Note in your work artifact (`docs/active/work/T-011-01/progress.md`):
- Did `just install` succeed on the first try?
- Were there missing dependencies or unclear error messages?
- Did all tests pass?
- How long did the build take?
- Any friction with the README instructions?

## Acceptance Criteria

- [ ] `just install` completes without errors
- [ ] `lisa` binary is available on PATH
- [ ] `cargo test --workspace` passes all tests
- [ ] Any issues encountered are documented
