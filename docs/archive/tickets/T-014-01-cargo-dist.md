---
id: T-014-01
title: Integrate cargo-dist into the project
type: task
phase: done
status: done
priority: high
story: S-014
created: 2026-02-20
depends_on: []
---

# T-014-01: Integrate `cargo-dist` into the project

## Objective

Replace the hand-rolled `release.yml` with `cargo-dist`, which automates cross-platform binary builds, release artifact packaging, and install script generation.

## Requirements

### Setup

1. Install cargo-dist: `cargo install cargo-dist`
2. Run `cargo dist init` in the repo root
3. Configure for the four target triples:
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu`
4. Only distribute `lisa-cli` (not `lisa-plugin` or `lisa-core` as standalone binaries)

### GitHub Actions

- `cargo-dist` generates its own release workflow — this replaces the existing `.github/workflows/release.yml`
- Keep the existing `.github/workflows/ci.yml` as-is (cargo-dist doesn't touch CI)
- Verify the generated workflow triggers on version tags (`v*`)

### Install script

- `cargo-dist` generates a shell installer script automatically
- Verify it handles platform detection for all four targets
- Test the installer URL format and document it in the README (S-015 will use this)

### WASM plugin embedding

The `lisa-cli` build requires the WASM plugin to be built first (`build.rs` copies it). Ensure the cargo-dist build pipeline handles this:
- The WASM target (`wasm32-wasip1`) must be available in the CI runner
- `lisa-plugin` must be built before `lisa-cli`
- This may require custom build steps in the cargo-dist config

## Notes

- Refer to cargo-dist docs: https://opensource.axo.dev/cargo-dist/
- The existing `release.yml` can be kept as a backup initially and removed once cargo-dist is verified
- Windows targets are explicitly out of scope

## Acceptance Criteria

- [ ] `cargo dist init` config is committed (`dist.toml` or `Cargo.toml` `[dist]` section)
- [ ] Generated GitHub Actions workflow builds all four targets
- [ ] Install script is generated and accessible from the release
- [ ] WASM plugin is correctly embedded in CLI binaries built by cargo-dist
- [ ] A test tag produces a valid GitHub Release with all artifacts and checksums
