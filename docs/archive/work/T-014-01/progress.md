# T-014-01 Progress: Integrate cargo-dist

## Completed

### Step 1: Install cargo-dist
- Installed cargo-dist v0.30.4 (`dist` binary at `/Volumes/ext1/cargo/bin/dist`)

### Step 2: Create `.github/build-setup.yml`
- Created with WASM pre-build steps (rustup target add + cargo build)
- Placed in `.github/` (not `workflows/`) as required

### Step 3: Add `[profile.dist]` to Cargo.toml
- Added `[profile.dist]` inheriting from `release` profile
- `cargo test --workspace` passes (320 tests)

### Step 4: Create `dist-workspace.toml`
- Created with all 4 targets, shell installer, `packages = ["lisa-cli"]`
- `github-build-setup` path is `"../build-setup.yml"` (relative to `.github/workflows/`)
- `install-path = "CARGO_HOME"` for standard cargo bin install

### Step 5: Generate release workflow
- Ran `dist generate-ci` to generate `.github/workflows/release.yml`
- Old workflow backed up and removed (recoverable from git history)
- New workflow has 5 jobs: plan, build-local-artifacts, build-global-artifacts, host, announce
- WASM build steps confirmed injected at lines 130-133

### Step 6: Verify with `dist plan`
- `dist plan` succeeds, lists:
  - `lisa-cli 0.1.6` as the only distributable package
  - All 4 target archives (`*.tar.xz`)
  - `lisa-cli-installer.sh` shell installer
  - SHA256 checksums for all artifacts
  - `source.tar.gz` source archive

### Step 7: Run tests
- All 320 tests pass (111 lisa-cli + 78 lisa-core + 131 lisa-plugin)
- Zero code changes — purely infrastructure

### Step 8: Review generated workflow
- Tag trigger: `**[0-9]+.[0-9]+.[0-9]+*` (matches `v0.1.6`, etc.)
- PR plan-only mode enabled
- Build-setup steps present in build-local-artifacts job
- No Windows targets

## Deviations from Plan

1. **`github-build-setup` path**: The path is relative to `.github/workflows/`, not the config file. Changed from `.github/build-setup.yml` to `../build-setup.yml`.
2. **Binary name**: cargo-dist v0.30.4 installs as `dist`, not `cargo-dist`. The `cargo dist` subcommand doesn't work; use `dist` directly.

## Files Changed

- `dist-workspace.toml` — Created (cargo-dist config)
- `Cargo.toml` — Added `[profile.dist]`
- `.github/build-setup.yml` — Created (WASM pre-build steps)
- `.github/workflows/release.yml` — Replaced with cargo-dist generated version
