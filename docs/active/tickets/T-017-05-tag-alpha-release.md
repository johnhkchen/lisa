---
id: T-017-05
title: Tag and cut alpha release
type: task
phase: ready
status: blocked
priority: high
story: S-017
created: 2026-02-20
depends_on:
  - T-017-04
---

# T-017-05: Tag and cut alpha release

## Objective

Create a version tag and push it to trigger the cargo-dist release workflow. This produces the first public release with prebuilt binaries and an install script.

## Version decision

Current version: `0.1.6` (in `Cargo.toml`).

Options:
- **Keep `0.1.6`** — Ship what we have. Simple.
- **Bump to `0.2.0`** — Reflects the significant changes (doctor, cargo-dist, docs rewrite). More appropriate for a milestone.

Decide before tagging. If bumping, update `version` in:
- `Cargo.toml` (workspace version)
- `Cargo.lock` (regenerated automatically)

The tag format must match what cargo-dist expects: `v0.1.6` or `v0.2.0`.

## Steps

1. Verify CI is green on the latest main commit (T-017-04)
2. Decide on version number
3. If version bump needed:
   ```bash
   # Edit Cargo.toml workspace version
   cargo check  # regenerates Cargo.lock
   git add Cargo.toml Cargo.lock
   git commit -m "Bump version to 0.2.0"
   git push origin main
   # Wait for CI green
   ```
4. Create and push the tag:
   ```bash
   git tag v0.2.0  # or v0.1.6
   git push origin v0.2.0
   ```
5. Monitor release workflow at: https://github.com/johnhkchen/lisa/actions
6. Verify the GitHub Release page is created with:
   - Four platform archives (`.tar.xz`)
   - Shell installer script (`lisa-cli-installer.sh`)
   - SHA256 checksums
   - Source archive

## Known risks

- **WASM build in CI** — The `.github/build-setup.yml` must correctly install `wasm32-wasip1` and build the plugin before `lisa-cli`. If this fails, the release will have no binaries.
- **Homebrew publish** — Will fail if `HOMEBREW_TAP_TOKEN` secret isn't set. This is expected and non-blocking — the release itself will still succeed, only the formula publish step will fail.

## Acceptance Criteria

- [ ] Version tag pushed to origin
- [ ] Release workflow completes successfully (all build jobs green)
- [ ] GitHub Release page exists with all four platform binaries
- [ ] Shell installer script is included in the release
- [ ] Checksums are present
