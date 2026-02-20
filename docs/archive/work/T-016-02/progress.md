# T-016-02 Progress: Add Nix Flake

## Completed

### Step 1: Create flake.nix
- Wrote `flake.nix` at repo root
- Uses crane + rust-overlay (as designed)
- `overrideToolchain` uses function form (crane v0.18+ API)
- Source filtering includes `.md` files for `include_str!` in templates.rs
- Two-stage build: preBuild compiles WASM, cargoBuildCommand builds CLI
- `makeWrapper` wraps binary with zellij on PATH
- Checks: build, clippy, fmt
- devShell: Rust + wasm target, just, zellij, cargo-watch
- Reduced duplication via shared `perSystem` helper
- Nix syntax validated via `nix-instantiate --parse`

### Step 2: Update .gitignore
- Added `result` to `.gitignore`

## Blocked

### Step 3: Generate flake.lock
- Nix daemon not running on this machine (`cannot connect to socket at /nix/var/nix/daemon-socket/socket`)
- `flake.lock` will be generated on first `nix flake lock` or `nix build` by the user
- This is expected — `flake.lock` is auto-generated and can be committed after first run

### Steps 4-5: Validate and test
- Cannot run `nix flake check` or `nix build` without the daemon
- Syntax is validated; structural correctness confirmed by code review

### Existing tests
- All 320 tests pass (111 CLI + 78 core + 131 plugin) — no regressions

## Deviations from Plan

- **No flake.lock committed** — Nix daemon unavailable; user must run `nix flake lock` to generate it
- **makeWrapper instead of propagatedBuildInputs** — as designed, wraps zellij on PATH without polluting user profile

## Remaining for User

1. Start the Nix daemon (`sudo launchctl load /Library/LaunchDaemons/org.nixos.nix-daemon.plist` on macOS)
2. Run `nix flake lock` to generate `flake.lock`
3. Run `nix flake check` to validate the full build
4. Run `nix build` and test `./result/bin/lisa --help`
5. Commit `flake.lock` alongside `flake.nix` and `.gitignore`
