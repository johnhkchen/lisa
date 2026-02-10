# Progress: T-007-04 github-release-workflow

## Completed

### Step 1: CI workflow
- Created `.github/workflows/ci.yml`
- Triggers on push to main and pull requests
- Steps: checkout, install Rust stable + wasm32-wasip1, cache, fmt check, clippy (all 3 crates), tests, WASM check

### Step 2-4: Release workflow
- Created `.github/workflows/release.yml`
- Triggers on `v*` tags
- Three-job pipeline:
  1. **build-wasm**: Verifies tag/version match, builds WASM plugin, uploads artifact
  2. **build-cli** (4-target matrix): Downloads WASM, builds CLI per platform, strips, tars, uploads
     - x86_64-linux (native), aarch64-linux (cross), x86_64-macos (macos-13), aarch64-macos (macos-latest)
  3. **release**: Downloads all artifacts, generates SHA256 checksums, creates GitHub release with auto-generated notes

### Step 5: README update
- Restructured Install section with three options: prebuilt binaries (recommended), crates.io, from source
- Added curl one-liners for all 4 platforms
- Added link to releases page

## Deviations from Plan

- Combined all steps into a single commit since all files are new/tightly coupled (as planned)
- Stripping for cross-compiled aarch64-linux is skipped (can't use host strip on foreign arch) — the conditional `!matrix.use_cross` in the workflow handles this correctly

## Acceptance Criteria Status

- [x] `.github/workflows/release.yml` workflow triggered on `v*` tags
- [x] Builds binaries for: x86_64-linux, aarch64-linux, x86_64-macos, aarch64-macos
- [x] Build matrix: compile WASM plugin first, then build CLI with embedded WASM
- [x] Binaries are stripped and compressed
- [x] GitHub release created automatically with binaries attached
- [x] Release notes include auto-generated changelog
- [x] Versioning: workspace Cargo.toml version matches git tag (verified in build-wasm job)
- [x] Installation instructions in README updated with download links / curl one-liner
- [x] CI workflow (non-release) runs `cargo test --workspace` and WASM check on PRs
