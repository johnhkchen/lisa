---
id: T-016-02
title: Add Nix flake to repository
type: task
phase: done
status: done
priority: medium
story: S-016
created: 2026-02-20
depends_on:
  - T-014-03
---

# T-016-02: Add Nix flake to repository

## Objective

Add a `flake.nix` to the repo root so Nix users can install Lisa directly from GitHub.

## Requirements

### Flake structure

Create `flake.nix` that:
- Builds `lisa-cli` from source using the Rust toolchain
- Handles the WASM plugin build step (needs `wasm32-wasip1` target)
- Declares `zellij` as a runtime dependency in `propagatedBuildInputs`
- Provides a `devShell` with the full development toolchain (Rust, wasm target, just)

### Install flow

```sh
# Direct install
nix profile install github:johnhkchen/lisa

# Try without installing
nix run github:johnhkchen/lisa -- --help

# Dev shell
nix develop github:johnhkchen/lisa
```

### Testing

- Verify the flake builds on NixOS or with nix-on-macOS
- Verify `nix flake check` passes

## Acceptance Criteria

- [ ] `flake.nix` exists at repo root
- [ ] `nix profile install github:johnhkchen/lisa` installs the `lisa` binary
- [ ] `nix flake check` passes
- [ ] Zellij is available as a runtime dependency
- [ ] `devShell` provides a working development environment
