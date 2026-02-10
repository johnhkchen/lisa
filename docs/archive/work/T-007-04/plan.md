# Plan: T-007-04 github-release-workflow

## Step 1: Create CI workflow

**File**: `.github/workflows/ci.yml`

Write the CI workflow that runs on PRs and pushes to main:
- Checkout, setup Rust stable with caching
- Add wasm32-wasip1 target
- Format check, clippy (all three crates), tests, WASM check

**Verify**: Read the file, confirm YAML structure is valid. The workflow can be fully tested by pushing this commit and opening a PR (or pushing to main).

## Step 2: Create release workflow — build-wasm job

**File**: `.github/workflows/release.yml`

Write the release workflow with the first job:
- Trigger on `v*` tags
- `build-wasm` job: checkout, verify tag matches Cargo.toml version, setup Rust, build WASM plugin, upload artifact

**Verify**: YAML structure is valid. Version check logic extracts tag and compares.

## Step 3: Add build-cli matrix job to release workflow

**File**: `.github/workflows/release.yml` (append)

Add the `build-cli` job with matrix strategy:
- 4 targets: x86_64-linux, aarch64-linux (cross), x86_64-macos, aarch64-macos
- Download WASM artifact, build CLI, strip binary, tar+gzip, upload artifact

**Verify**: Matrix includes all 4 targets. Cross is only used for aarch64-linux. Artifact paths are correct.

## Step 4: Add release job to release workflow

**File**: `.github/workflows/release.yml` (append)

Add the `release` job:
- Download all build artifacts
- Generate SHA256 checksums
- Create GitHub release with `gh release create`
- Attach tarballs and checksum file
- Auto-generate release notes

**Verify**: Job depends on build-cli. Release uses `GITHUB_TOKEN` for auth.

## Step 5: Update README with download instructions

**File**: `README.md`

Add a "Download" section with curl one-liners for each platform. Keep the existing "Install" section for cargo install. Add a link to the releases page.

**Verify**: URLs use `releases/latest/download/` pattern. All 4 platforms covered.

## Step 6: Final review

Review both workflow files end-to-end for:
- Correct artifact names and paths
- Proper job dependencies (`needs:`)
- Permission settings for release creation
- Caching configuration
- That the WASM artifact lands in the right directory for build.rs to find

## Testing Strategy

- **ci.yml**: Fully testable by pushing to a branch/PR. GitHub Actions will run it.
- **release.yml**: Can only be fully tested by pushing a `v*` tag. The YAML structure can be validated locally with `actionlint` if available, but the real test is a tag push.
- **README**: Visual review of the download commands.

## Commit Plan

1. Commit: Add CI and release workflows + README update (single commit since the files are all new and tightly coupled)
