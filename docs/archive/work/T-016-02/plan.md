# T-016-02 Plan: Add Nix Flake

## Step 1: Create flake.nix

Write `flake.nix` at the repo root with:

- **Inputs**: nixpkgs (unstable), crane, rust-overlay (follows nixpkgs)
- **System iteration**: `nixpkgs.lib.genAttrs` over 4 systems (x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin)
- **Rust toolchain**: stable from rust-overlay with `wasm32-wasip1` target, clippy, rustfmt
- **craneLib**: initialized with the custom toolchain
- **Source filtering**: `craneLib.cleanCargoSource` with an additional filter to include `.md` files (for `include_str!("../data/rdspi-workflow.md")` in templates.rs)
- **cargoArtifacts**: `craneLib.buildDepsOnly` for dependency caching
- **lisa package**: `craneLib.buildPackage` with:
  - `preBuild` to compile `lisa-plugin --target wasm32-wasip1 --release`
  - `cargoBuildCommand` targeting `lisa-cli`
  - `postInstall` wrapping the binary with zellij on PATH via `makeWrapper`
- **checks**: the package build itself, cargoClippy, cargoFmt
- **devShell**: mkShell with rustToolchain, just, zellij, cargo-watch
- **apps.default**: points to `lisa` binary

Verification: `nix flake show` lists all expected outputs.

## Step 2: Update .gitignore

Add `result` line to `.gitignore` (the Nix build output symlink).

Verification: `grep result .gitignore` succeeds.

## Step 3: Generate flake.lock

Run `nix flake lock` to generate the lock file pinning all input versions.

Verification: `flake.lock` exists and is valid JSON.

## Step 4: Validate the flake

Run `nix flake check` to validate:
- Flake schema is correct
- Checks (clippy, fmt) pass
- Package builds successfully

This is the main verification step. If it passes, the flake is structurally correct and the two-stage build works.

## Step 5: Test the build output

Run `nix build` and verify:
- `./result/bin/lisa` exists and is executable
- `./result/bin/lisa --help` produces expected output
- `zellij` is available via the wrapper (check `./result/bin/.lisa-wrapped` exists or the wrapper script references zellij)

## Testing Strategy

- **Build correctness**: `nix build` succeeds and produces a working binary
- **Flake validity**: `nix flake check` passes
- **Runtime deps**: wrapped binary has zellij on PATH
- **Dev shell**: `nix develop --command rustc --version` returns the expected stable version
- **WASM embedding**: `./result/bin/lisa loop --dry-run` should not error about missing WASM (though it will error about missing CLAUDE.md, which is expected)

## Commit Plan

Single commit: "Add Nix flake for building and installing Lisa"
- `flake.nix` (new)
- `flake.lock` (new, auto-generated)
- `.gitignore` (modified, add `result`)
