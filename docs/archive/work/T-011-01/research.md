# T-011-01 Research: Build and Install Lisa on a Fresh Device

## Objective

Validate the full build-from-source workflow and document any friction points.

## Build System Overview

### Prerequisites

| Tool | Required | Purpose |
|------|----------|---------|
| Rust toolchain (rustup) | Yes | Compiler and cargo |
| wasm32-wasip1 target | Yes | WASM plugin compilation |
| `just` command runner | Yes | Build orchestration |
| Zellij | Runtime only | For `lisa loop` (not needed for build) |

### Build Flow

`just install` runs two steps:
1. `cargo build -p lisa-plugin --target wasm32-wasip1 --release` — builds the WASM plugin (~993KB)
2. `touch target/wasm32-wasip1/release/lisa.wasm` — ensures mtime refresh for build.rs
3. `cargo install --path crates/lisa-cli --force` — builds CLI with embedded WASM

The CLI's `build.rs` copies the WASM binary from `target/wasm32-wasip1/release/lisa.wasm` into `OUT_DIR`. If the WASM file doesn't exist, it writes an empty placeholder (so `cargo install` from crates.io works but `lisa loop` won't function).

### Workspace Structure

- 3 crates: `lisa-core`, `lisa-plugin`, `lisa-cli`
- `lisa-core` has no platform-specific deps (serde, serde_yaml_ng, serde_json)
- `lisa-plugin` depends on `zellij-tile` 0.43 and conditionally on `libc` (unix only)
- `lisa-cli` depends on `lisa-core`, `clap`, `toml`, `serde`, `serde_json`

### Install Target

Binary installs to `$CARGO_HOME/bin/lisa` (typically `~/.cargo/bin/lisa`).

## Validation Results (This Device)

**Environment:**
- macOS Darwin 25.3.0, Apple Silicon (aarch64-apple-darwin)
- rustc 1.95.0-nightly (6efa357bf 2026-02-08)
- cargo 1.95.0-nightly (fe2f314ae 2026-01-30)
- just 1.46.0

**Build:** `just install` completed successfully in ~7 seconds (incremental; clean build would be longer).

**Warnings:** 3 dead-code warnings in `ui.rs` for `pane_id` fields on `ActiveThread`, `ParkedThread`, `SlotInfo` structs. Not errors, not blocking.

**Binary verification:**
- `which lisa` → `/Volumes/ext1/cargo/bin/lisa` (non-standard CARGO_HOME location, works fine)
- `lisa --help` → shows all 8 subcommands (init, validate, status, setup-guide, doctor, version, loop, help)
- `lisa --version` → `lisa 0.1.6`
- `lisa version` → `lisa 0.1.6`

**Tests:** 332 tests total, all passing:
- lisa-cli: 123 tests
- lisa-core: 78 tests
- lisa-plugin: 131 tests

## Potential Fresh-Device Issues

1. **WASM target not installed by default.** Must run `rustup target add wasm32-wasip1` first. README documents this but the error message from `cargo build` would be cryptic without it.

2. **`just` not in standard Rust toolchain.** Must install separately (`cargo install just` or platform package manager). README mentions it but doesn't provide install command.

3. **No `--version` flag on crates.io install.** The `cargo install lisa-cli` path works but produces a binary where `lisa loop` fails with a clear error about missing embedded WASM. This is documented in README.

4. **Dead code warnings.** The 3 `pane_id` warnings are cosmetic but may concern a new contributor. Not a blocker.

5. **Build order dependency.** The `just install` recipe correctly chains `build` before `cargo install`, but running `cargo install --path crates/lisa-cli` directly without first building the WASM target would embed an empty placeholder.

## Files Examined

- `justfile` — build recipes, `install` target
- `Cargo.toml` — workspace config, version 0.1.6
- `crates/lisa-cli/build.rs` — WASM embedding via copy to OUT_DIR
- `crates/lisa-cli/Cargo.toml` — CLI deps, bin name
- `crates/lisa-plugin/Cargo.toml` — plugin deps, cdylib crate type
- `crates/lisa-core/Cargo.toml` — core deps
- `crates/lisa-cli/src/loop_cmd.rs` — WASM write + layout generation + zellij exec
- `crates/lisa-cli/src/config.rs` — .lisa.toml parsing and resolution
- `README.md` — install instructions (from source, crates.io, prebuilt)
- `.github/workflows/release.yml` — CI release pipeline (cargo-dist based)
