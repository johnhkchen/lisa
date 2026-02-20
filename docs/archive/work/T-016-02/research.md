# T-016-02 Research: Add Nix Flake

## Build Architecture

Lisa has a two-stage build that a Nix flake must reproduce:

1. **Stage 1 — WASM plugin**: `cargo build -p lisa-plugin --target wasm32-wasip1 --release`
   - Produces `target/wasm32-wasip1/release/lisa.wasm` (~993KB)
   - Depends on: `lisa-core`, `zellij-tile 0.43`, `serde`, `serde_json`, `libc` (unix)
   - `lisa-plugin/Cargo.toml` sets `crate-type = ["cdylib"]`, `publish = false`

2. **Stage 2 — CLI binary**: `cargo build -p lisa-cli --release`
   - `build.rs` copies `target/wasm32-wasip1/release/lisa.wasm` into `OUT_DIR`
   - `templates.rs` embeds it via `include_bytes!(concat!(env!("OUT_DIR"), "/lisa.wasm"))`
   - Final binary is ~1.8MB with embedded WASM
   - Binary name: `lisa` (from `[[bin]] name = "lisa"`)

The build.rs has a fallback: if the WASM file doesn't exist, it writes an empty placeholder. This means `cargo build -p lisa-cli` alone "succeeds" but produces a non-functional binary. The flake must ensure Stage 1 runs first.

## Toolchain Requirements

- Rust edition 2021 (workspace `Cargo.toml`)
- No `rust-toolchain.toml` — project currently uses whatever the developer has installed
- Current dev setup: `nightly-aarch64-apple-darwin` with `rustc 1.95.0-nightly`
- Required targets: native host + `wasm32-wasip1`
- The `zellij-tile 0.43` crate compiles under WASI; no special patches needed

Stable vs nightly: nothing in the codebase uses nightly-only features. The developer's use of nightly appears to be preference, not requirement. A flake should use stable Rust.

## Dependency Graph

External crates (from Cargo.toml files):
- `lisa-core`: serde 1.0, serde_yaml_ng 0.9, serde_json 1.0
- `lisa-plugin`: zellij-tile 0.43, serde 1.0, serde_json 1.0, libc 0.2 (unix)
- `lisa-cli`: lisa-core, clap 4 (derive), toml 0.8, serde 1.0, serde_json 1.0

All pure Rust crates — no C dependencies, no system libraries needed for the build itself. `Cargo.lock` is version 4, committed to the repo.

## Runtime Dependencies

- **zellij** — `lisa loop` calls `exec zellij --layout ...` (hard requirement)
- **claude** — `lisa loop` checks for `claude` binary on PATH
- Shell (`/bin/sh`) — for hook scripts

The ticket specifies zellij in `propagatedBuildInputs`. Claude Code is a user-installed tool (not packaged in nixpkgs), so it can't be a Nix dependency.

## Nix Packaging Approaches

### Approach A: crane (Rust-specific Nix build framework)

[crane](https://github.com/ipetkov/crane) is the current standard for building Rust in Nix:
- Handles dependency caching (separates deps build from source build)
- Supports cross-compilation and custom targets
- Integrates with `rust-overlay` for toolchain management
- Can do multi-step builds (build WASM first, then CLI)
- `craneLib.buildPackage` with `cargoArtifacts` for caching

### Approach B: naersk

Alternative Rust builder:
- Simpler API than crane
- Less flexible for cross-compilation
- WASM target support is less documented

### Approach C: nixpkgs rustPlatform.buildRustPackage

Built-in nixpkgs helper:
- Requires `cargoHash` for dependency fetching
- Less flexible for multi-stage builds
- WASM cross-compilation support is more manual
- Would need `preBuild` hooks to compile the WASM step

## WASM Target in Nix

The `wasm32-wasip1` target is supported via `rust-overlay`:
```nix
rust-overlay.packages.${system}.rust.override {
  targets = [ "wasm32-wasip1" ];
}
```

Or via `fenix`:
```nix
fenix.packages.${system}.combine [
  fenix.packages.${system}.stable.toolchain
  fenix.packages.${system}.targets.wasm32-wasip1.stable.rust-std
];
```

With crane, you'd configure the toolchain with the target, then call cargo with `--target wasm32-wasip1` in a custom build step.

## Flake Outputs Expected

Per the ticket:
- `packages.${system}.default` — the `lisa` CLI binary
- `apps.${system}.default` — for `nix run`
- `devShells.${system}.default` — development environment

## devShell Contents

Per ticket: Rust toolchain, wasm target, just. Additionally useful:
- `cargo-watch` (used in justfile `watch` recipe)
- `clippy`, `rustfmt` (in justfile `lint` and `fmt` recipes)
- `zellij` (for testing `lisa loop` during development)

## Existing Nix Ecosystem References

- zellij itself has a flake.nix in its repo (uses crane)
- Many Rust WASM projects use crane with rust-overlay

## Key Constraints

1. **Two-stage build order** — WASM must be built before CLI, and the WASM artifact must be visible at `target/wasm32-wasip1/release/lisa.wasm` when `build.rs` runs
2. **Target multiplexing** — the same workspace builds for two different targets (native + wasm32-wasip1)
3. **Cargo.lock is committed** — Nix reproducibility is satisfied
4. **No system C deps** — simplifies the build; no `buildInputs` for native libraries needed
5. **`zellij` runtime dep** — must be in `propagatedBuildInputs` or wrapped onto PATH
6. **Profile settings** — release profile uses `opt-level = "s"` and `lto = true`; Nix builds should use release mode

## Files That Will Be Created/Modified

- `flake.nix` — new file at repo root
- `flake.lock` — auto-generated by Nix on first `nix flake lock`
- `.gitignore` — may need to add `result` (Nix build symlink)

## Testing Constraints

- `nix flake check` validates the flake structure and runs any checks defined
- The ticket asks for NixOS or nix-on-macOS verification
- `cargo test --workspace` runs on native target (not WASM), so Nix checks can include test execution
