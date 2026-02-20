# T-016-02 Design: Add Nix Flake

## Decision: crane + rust-overlay

### Options Evaluated

**A. crane + rust-overlay** — Rust-native Nix build framework with explicit toolchain overlay
- Pros: Best cross-compilation support, dependency caching, multi-stage build support, well-documented for WASM targets, actively maintained, used by zellij itself
- Cons: Three flake inputs (nixpkgs, crane, rust-overlay)

**B. naersk** — Simpler Rust builder
- Pros: Less boilerplate for simple projects
- Cons: WASM cross-compilation poorly documented, less flexible for two-stage builds, less active than crane

**C. nixpkgs rustPlatform.buildRustPackage** — Built-in approach
- Pros: No extra inputs beyond nixpkgs
- Cons: `cargoHash` management is brittle on updates, multi-target builds require manual `preBuild` scripting, no dependency caching between builds

### Choice: Option A

crane + rust-overlay is the right fit because:
1. The two-stage build (WASM then CLI) maps directly to crane's `cargoArtifacts` pattern
2. `rust-overlay` provides declarative wasm32-wasip1 target configuration
3. This is the same approach zellij uses, so the ecosystem familiarity is high
4. Dependency caching means iterative builds are fast during development

## Build Strategy

### Two-Phase Derivation

The flake will use two crane build steps within a single derivation:

```
1. Build cargoArtifacts (deps-only, both targets)
2. In the final buildPackage:
   - preBuild: compile lisa-plugin for wasm32-wasip1
   - Main build: compile lisa-cli for the host, which embeds the WASM
```

This is simpler than two separate derivations because `build.rs` expects the WASM at a relative path within the cargo target directory. Keeping it in one derivation avoids symlinking across Nix store paths.

Alternative considered: two derivations (one for WASM, one for CLI that takes WASM as buildInput). Rejected because `build.rs` looks for `target/wasm32-wasip1/release/lisa.wasm` relative to the workspace root — patching this path for a Nix store input would be invasive and fragile.

### Toolchain Configuration

```nix
rust-overlay stable with extensions: [ "rust-src" "clippy" "rustfmt" ]
targets: [ "wasm32-wasip1" ]
```

Using stable, not nightly. Nothing in the codebase requires nightly features.

## Flake Structure

### Inputs

```nix
inputs = {
  nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  crane.url = "github:ipetkov/crane";
  rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };
};
```

### Outputs

1. **`packages.${system}.default`** — the `lisa` binary (lisa-cli with embedded WASM)
   - `propagatedBuildInputs = [ zellij ]` — makes zellij available at runtime
   - `nativeBuildInputs = [ rustToolchain ]` — Rust with wasm target for the build

2. **`apps.${system}.default`** — wraps the package for `nix run`

3. **`devShells.${system}.default`** — development environment containing:
   - Rust toolchain (stable + wasm32-wasip1 target)
   - clippy, rustfmt, rust-src
   - just
   - zellij
   - cargo-watch

4. **`checks.${system}`** — runs `cargo test --workspace` and `cargo clippy` for CI

### System Support

Use `flake-utils` or `nixpkgs.lib.genAttrs` for multi-system support:
- `x86_64-linux`
- `aarch64-linux`
- `x86_64-darwin`
- `aarch64-darwin`

Decision: use `crane.lib.${system}` which handles this pattern. Avoid adding `flake-utils` as a fourth input — iterate over systems manually with `nixpkgs.lib.genAttrs`.

## Runtime Wrapping

The `lisa` binary calls `exec zellij` at runtime. Two options:

**A. `propagatedBuildInputs`** — adds zellij to the user's profile when they install lisa
**B. `makeWrapper`** — wraps the binary to prepend zellij to PATH

Decision: **B (makeWrapper)**. `propagatedBuildInputs` pollutes the user's profile with zellij even if they already have it installed. `makeWrapper` is self-contained — the binary always finds zellij regardless of the user's PATH, and doesn't install zellij into the user's profile.

## .gitignore Addition

Add `result` to `.gitignore` — this is the default symlink created by `nix build`.

## What We Won't Do

- No `rust-toolchain.toml` — the flake is self-contained; rustup users keep their existing workflow
- No NixOS module — this is a user tool, not a system service
- No `flake-utils` dependency — manual system iteration with `genAttrs` is simpler
- No `flake-compat` for non-flake Nix — keep it flakes-only for now
