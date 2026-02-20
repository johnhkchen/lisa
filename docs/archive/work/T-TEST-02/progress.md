# T-TEST-02: Build System Summary

## Build Pipeline Overview

Lisa has a **two-stage build**: the WASM plugin must be compiled before the CLI, because the CLI embeds the plugin binary at compile time.

```
Stage 1: cargo build -p lisa-plugin --target wasm32-wasip1 --release
           → target/wasm32-wasip1/release/lisa.wasm (~993KB)

Stage 2: cargo build -p lisa-cli --release
           → target/release/lisa (~1.8MB, WASM embedded)
```

**How embedding works**: `crates/lisa-cli/build.rs` copies `lisa.wasm` from the WASM target directory into Cargo's `OUT_DIR`. The CLI source then uses `include_bytes!` to bake the WASM into the binary. At runtime, `lisa loop` writes this blob to `/tmp/lisa-plugin.wasm` and hands it to Zellij.

A plain `cargo build --workspace` does NOT work because it doesn't cross-compile the plugin to wasm32-wasip1. The build ordering must be explicit.

## Workspace Structure

Cargo workspace with `resolver = "2"`, three crates:

| Crate | Type | Target | Purpose |
|-------|------|--------|---------|
| **lisa-core** | lib (rlib) | native | Shared types, ticket YAML parsing, DAG computation |
| **lisa-plugin** | lib (cdylib) | wasm32-wasip1 | Zellij WASM plugin: scheduler, UI, lifecycle |
| **lisa-cli** | bin (`lisa`) | native | CLI: init, validate, loop, doctor |

Internal dependencies:
```
lisa-cli ───depends_on──→ lisa-core (via Cargo)
lisa-plugin ─depends_on──→ lisa-core (via Cargo)
lisa-cli ───embeds──────→ lisa-plugin (via build.rs + include_bytes!, NOT Cargo)
```

Shared workspace metadata: version `0.1.6`, edition 2021, MIT license. Release profile uses `opt-level = "s"` (size-optimized) with LTO enabled.

## Build Tools

### Cargo

The primary build system. Key commands:

- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` — compile WASM plugin
- `cargo build -p lisa-cli --release` — compile CLI (requires WASM built first)
- `cargo test --workspace` — run all tests on native target
- `cargo check -p lisa-plugin --target wasm32-wasip1` — fast WASM type-check

Tests run on native only. They avoid zellij APIs, so the WASM target isn't needed for testing.

### just (Task Runner)

`justfile` wraps Cargo with ergonomic recipes. The default recipe is `check`.

| Recipe | What It Does |
|--------|-------------|
| `just check` | WASM type-check + all tests (default) |
| `just build` | Build WASM plugin |
| `just build-cli` | Build WASM, then CLI (enforces ordering) |
| `just release` | Full distribution build |
| `just install` | Build WASM + `cargo install` to ~/.cargo/bin |
| `just test` | Run all workspace tests |
| `just lint` | Clippy for all 3 crates (correct targets) |
| `just fmt` | Format all code |
| `just watch` | Continuous check via cargo-watch |

### Nix Flake

`flake.nix` provides reproducible builds using crane + rust-overlay:

- **Build**: Two-stage via `preBuild` (WASM) + `cargoBuildCommand` (CLI). Wraps the binary to put zellij on PATH.
- **Checks**: Package build, clippy (`-D warnings`), cargo fmt.
- **Dev shell**: Rust (with wasm32-wasip1 target), just, zellij, cargo-watch.
- **Platforms**: x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin.

### cargo-dist (Release Distribution)

`dist-workspace.toml` configures cargo-dist v0.30.4 for GitHub releases:

- **CI**: GitHub Actions (with custom `build-setup.yml` for WASM pre-build)
- **Installer**: Shell script
- **Targets**: x86_64/aarch64 for macOS and Linux (4 platforms)
- **Packages**: Only `lisa-cli` (the plugin is embedded)
- **Install path**: CARGO_HOME

## Quick Reference

| Task | Command |
|------|---------|
| Type-check + test | `just check` |
| Build WASM only | `just build` |
| Build everything | `just build-cli` |
| Run tests | `just test` |
| Install locally | `just install` |
| Lint all crates | `just lint` |
| Format code | `just fmt` |
| Watch mode | `just watch` |
| Nix build | `nix build` |
| Nix dev shell | `nix develop` |

## Completion

All acceptance criteria met:

- [x] `docs/active/work/T-TEST-02/research.md` exists
- [x] `docs/active/work/T-TEST-02/design.md` exists
- [x] `docs/active/work/T-TEST-02/structure.md` exists
- [x] `docs/active/work/T-TEST-02/plan.md` exists
- [x] `docs/active/work/T-TEST-02/progress.md` exists documenting completion
