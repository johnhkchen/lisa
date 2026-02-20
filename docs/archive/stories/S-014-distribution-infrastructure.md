---
id: S-014
title: Distribution infrastructure
type: story
status: Active
created: 2026-02-20
---

# S-014: Distribution infrastructure

## Problem

Lisa's current release pipeline (`release.yml`) is hand-rolled. For public distribution we need a robust, automated pipeline that produces cross-platform binaries, a curl installer script, and proper crates.io packaging. `cargo-dist` handles most of this out of the box.

## Goal

Set up `cargo-dist` to replace the existing release workflow. Produce prebuilt binaries for all four target triples (x86_64/aarch64 for macOS/Linux), a platform-detecting shell installer, and a verified `cargo install` path. This is the launch-phase distribution — package managers (Homebrew, Nix, AUR) come in S-016.

## Tickets

- **T-014-01:** Integrate `cargo-dist` into the project (task)
- **T-014-02:** Verify `cargo install` and package metadata (task)
- **T-014-03:** Test install script and release binaries (chore)

## Dependencies

```
T-014-01 (cargo-dist setup)
  └── T-014-03 (test release)
T-014-02 (cargo install / metadata)
```

T-014-01 and T-014-02 can be worked in parallel. T-014-03 depends on T-014-01.

## Success Criteria

1. `cargo dist init` has been run and config is committed
2. GitHub Actions release workflow builds all four targets on tag push
3. Releases include a shell installer script (`install.sh` or equivalent)
4. `cargo install lisa-cli` works from a clean environment
5. Cargo.toml has complete metadata (authors, description, keywords, categories, repository, homepage, license)
6. A test tag produces a valid GitHub Release with checksums
