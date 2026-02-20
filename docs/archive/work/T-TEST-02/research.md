# T-TEST-02 Research: Build System

## Source: T-TEST-01 Research

T-TEST-01 mapped every top-level file and directory. This research zooms in on the build system components identified there: Cargo workspace, justfile, cargo-dist, Nix flake, and the WASM embedding pipeline.

## Workspace Structure

Lisa is a Cargo workspace (`resolver = "2"`) with three crates under `crates/`:

| Crate | Type | Target | Purpose |
|-------|------|--------|---------|
| `lisa-core` | lib (rlib) | native | Shared types, ticket parsing, DAG computation |
| `lisa-plugin` | lib (cdylib) | wasm32-wasip1 | Zellij WASM plugin |
| `lisa-cli` | bin (`lisa`) | native | CLI entry point |

Shared metadata in workspace root: version `0.1.6`, edition 2021, MIT license.

### Dependency Graph (Internal)

```
lisa-cli ──depends──> lisa-core
lisa-plugin ──depends──> lisa-core
```

`lisa-cli` does NOT depend on `lisa-plugin` via Cargo. The plugin is embedded as raw bytes via `build.rs` + `include_bytes!`, not as a Rust dependency.

### Key External Dependencies

- **lisa-core**: serde, serde_yaml_ng, serde_json
- **lisa-plugin**: zellij-tile 0.43, serde, serde_json, libc (unix only)
- **lisa-cli**: clap 4 (derive), toml, serde, serde_json

## Build Tools

### 1. Cargo (Primary)

Core build commands:
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` — builds WASM plugin (~993KB)
- `cargo build -p lisa-cli --release` — builds CLI binary (~1.8MB with embedded WASM)
- `cargo test --workspace` — runs all tests on native target (tests avoid zellij APIs)
- `cargo check -p lisa-plugin --target wasm32-wasip1` — fast type-check for WASM

Release profile: `opt-level = "s"` (size-optimized), LTO enabled. A `dist` profile inherits from release for cargo-dist.

### 2. just (Task Runner)

`justfile` wraps Cargo commands with ergonomic recipes:
- `just check` (default) — WASM check + workspace tests
- `just build` — WASM plugin only
- `just build-cli` — WASM first, then CLI (sequential dependency)
- `just release` — full distribution build
- `just install` — build WASM + `cargo install` to ~/.cargo/bin
- `just lint` — clippy for all three crates (each with correct target)
- `just fmt` / `just fmt-check` — format all
- `just watch` — cargo-watch for continuous feedback

### 3. cargo-dist (Release Distribution)

`dist-workspace.toml` configures cargo-dist v0.30.4:
- CI: GitHub Actions
- Installer: shell script
- Targets: x86_64/aarch64 for macOS and Linux (4 platforms)
- Packages: only `lisa-cli`
- Install path: CARGO_HOME
- Custom build setup via `.github/build-setup.yml`

### 4. Nix Flake (Reproducible Builds)

`flake.nix` uses crane + rust-overlay:
- Two-stage build: WASM plugin first (`preBuild`), then CLI (`cargoBuildCommand`)
- Checks: package build, clippy, fmt
- Dev shell: Rust (with wasm32-wasip1), just, zellij, cargo-watch
- Supports same 4 platforms as cargo-dist
- Wraps `lisa` binary to put zellij on PATH

## WASM Embedding Pipeline

This is the most architecturally interesting part of the build:

1. `cargo build -p lisa-plugin --target wasm32-wasip1 --release` produces `target/wasm32-wasip1/release/lisa.wasm`
2. `crates/lisa-cli/build.rs` copies `lisa.wasm` from the WASM target dir into `OUT_DIR`
3. `lisa-cli` source uses `include_bytes!(concat!(env!("OUT_DIR"), "/lisa.wasm"))` to embed the binary
4. At runtime, `lisa loop` writes the embedded WASM to `/tmp/lisa-plugin.wasm`

The `build.rs` writes an empty placeholder if the WASM file doesn't exist (allows dev builds without pre-building the plugin). The `justfile` uses `touch` to ensure the rerun-if-changed trigger fires.

## Build Order Constraint

The two-stage build is mandatory:
1. WASM plugin must be built first
2. CLI must be built second (it embeds the WASM)

Both `justfile` (recipe dependency: `build-cli: build`) and `flake.nix` (`preBuild` + `cargoBuildCommand`) enforce this ordering. A plain `cargo build --workspace` would NOT work correctly because it doesn't build the WASM target.

## Testing

All tests run on native target (`cargo test --workspace`). The test suite avoids zellij APIs so WASM compilation isn't needed for testing. Dev-dependencies across all three crates: `tempfile = "3"`.

## Observations

- No Makefile — `just` is the sole task runner
- No Docker — builds are native Cargo or Nix
- The WASM embedding is a custom pipeline (build.rs), not a standard Cargo feature
- cargo-dist and Nix serve overlapping but different audiences: cargo-dist for GitHub releases, Nix for reproducible local builds
- The `dist` profile exists solely for cargo-dist; all other builds use `release`
