# Design: T-007-04 github-release-workflow

## Decision 1: Release Workflow Architecture

### Option A: Single workflow, build matrix with WASM artifact passing

Build WASM once in a setup job, upload as artifact, then fan out to a matrix of CLI builds that download the artifact.

**Pros**: WASM built once, clean separation, no redundant work.
**Cons**: Slightly more complex workflow (job dependencies, artifact passing).

### Option B: Each matrix job builds WASM + CLI

Every matrix entry runs both build stages.

**Pros**: Simpler workflow, no inter-job dependencies.
**Cons**: Redundant WASM builds (4x), slower aggregate time.

### Option C: Build WASM + all cross-compiled CLIs in a single job using cross

Single job using `cross` tool for cross-compilation.

**Pros**: Simplest workflow structure.
**Cons**: `cross` adds Docker complexity, macOS targets require native runners (can't cross-compile from Linux easily due to linker/SDK requirements).

**Decision: Option A.** The WASM build is cheap but the architecture is cleaner. macOS targets need native runners, so cross-compilation from a single job doesn't work. Artifact passing between jobs is well-supported in GitHub Actions.

## Decision 2: ARM Linux Builds

### Option A: Native ARM runner (`ubuntu-24.04-arm`)

GitHub provides ARM runners for public repos.

**Pros**: Native compilation, no cross-compile complexity.
**Cons**: ARM runners may have limited availability or higher cost.

### Option B: Cross-compile from x86_64 Linux using `cross`

Use the `cross` tool with Docker to cross-compile for aarch64-linux.

**Pros**: Only needs x86_64 runner.
**Cons**: Adds Docker dependency, potentially slower.

### Option C: Use `cargo-zigbuild` or target-specific linker

Cross-compile using zig as the linker/compiler.

**Pros**: Fast, reliable cross-compilation for Linux targets.
**Cons**: Extra tool dependency.

**Decision: Option B (`cross`) for aarch64-linux only.** Lisa has no C dependencies in the CLI path, so `cross` will work cleanly. Use native runners for everything else. This avoids needing ARM runners which may not be available on all plans.

*Update: Actually, since all dependencies are pure Rust, we can use `cargo build --target aarch64-unknown-linux-gnu` with just the right linker. But `cross` handles the sysroot/linker automatically. Let's use `cross` for simplicity.*

## Decision 3: Binary Packaging

### Option A: Bare binaries

Upload raw `lisa` binaries to the release.

**Pros**: Simplest.
**Cons**: No checksum files, no archive wrapping.

### Option B: Tarball per platform

`lisa-v0.1.0-x86_64-linux.tar.gz` containing the binary.

**Pros**: Standard format, can include LICENSE/README.
**Cons**: Extra step for users.

### Option C: Tarball + SHA256 checksum file

Same as B but with a checksums file.

**Pros**: Verifiable downloads, standard for security-conscious users.
**Cons**: Slightly more complex.

**Decision: Option C.** Tarballs with SHA256 checksums. Standard practice, minimal extra work. Archive naming: `lisa-{version}-{target}.tar.gz`.

## Decision 4: Version Validation

The workflow should verify that the git tag (e.g., `v0.1.0`) matches the workspace `Cargo.toml` version (`0.1.0`). This prevents publishing a release with mismatched versions.

**Approach**: Extract version from `Cargo.toml` in the WASM build job, compare to the tag. Fail the workflow if they don't match.

## Decision 5: Release Notes

### Option A: Auto-generated from commits

Use GitHub's built-in auto-generate release notes feature.

**Pros**: Zero maintenance, always up to date.
**Cons**: May include noisy commit messages.

### Option B: Manual CHANGELOG.md

Require a CHANGELOG.md that gets extracted for each release.

**Pros**: Curated, high quality.
**Cons**: Maintenance burden, easy to forget.

### Option C: Tag message as release body

Use annotated tag message as the release body, with auto-generated notes as fallback.

**Pros**: Flexible — can write nice notes when desired, falls back to auto.
**Cons**: Requires annotated tags for good notes.

**Decision: Option A** with `generate_release_notes: true` in `gh release create` / `softprops/action-gh-release`. Low maintenance, good enough for early project. Can add CHANGELOG later.

## Decision 6: CI Workflow

Separate `ci.yml` triggered on PRs and pushes to main:
- `cargo fmt --all -- --check`
- `cargo clippy` for all crates (WASM target for plugin, native for others)
- `cargo test --workspace`
- `cargo check -p lisa-plugin --target wasm32-wasip1`

Use a single job with caching for speed.

## Rejected Alternatives

- **Windows builds**: Not in acceptance criteria, user base likely dev-oriented macOS/Linux.
- **musl static linking for Linux**: Not needed initially, glibc is fine for most users.
- **cargo-dist**: Automated release tool, but adds a heavy dependency and opinionated config. The workflow is simple enough to write directly.
- **Nix/Homebrew packages**: Out of scope for this ticket.

## Overall Design

```
ci.yml:
  trigger: push to main, PRs
  jobs:
    check:
      - setup rust (stable) + wasm target
      - cargo fmt --check
      - clippy (all crates)
      - cargo test --workspace
      - cargo check -p lisa-plugin --target wasm32-wasip1

release.yml:
  trigger: push tag v*
  jobs:
    build-wasm:
      - setup rust (stable) + wasm target
      - verify tag matches Cargo.toml version
      - cargo build -p lisa-plugin --target wasm32-wasip1 --release
      - upload wasm artifact

    build-cli (matrix: 4 targets):
      needs: build-wasm
      - download wasm artifact
      - setup rust (stable) + target
      - (for aarch64-linux: install cross)
      - build cli with embedded wasm
      - strip binary
      - tar + gzip
      - upload release asset

    release:
      needs: build-cli
      - create github release with all assets
      - generate sha256 checksums
      - auto-generate release notes
```
