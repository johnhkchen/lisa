---
id: S-017
title: Alpha release preparation
type: story
status: done
created: 2026-02-20
---

# S-017: Alpha release preparation

## Problem

The S-012 through S-016 work (hygiene, doctor, cargo-dist, docs, package managers) is complete but none of it is committed. The working tree has ~30 modified files and ~60 untracked files. CI would fail today due to 218 formatting diffs and 17 clippy warnings across the workspace. Completed stories (S-010, S-011) and test tickets (T-TEST-*) haven't been archived. The repo is not in a state where pushing a tag would produce a credible release.

This story takes the project from "work done but messy" to "alpha release with green CI and working install pipeline."

## Goal

1. All code passes `cargo fmt`, `cargo clippy`, tests, and WASM check
2. All pending work is committed in clean, logical commits
3. Completed stories and tickets are archived
4. CI is green on main
5. A tagged release produces working binaries and an install script
6. At least one install path is verified end-to-end

## Tickets

- **T-017-01:** Fix formatting and clippy warnings (chore)
- **T-017-02:** Archive completed stories and tickets (chore)
- **T-017-03:** Commit all pending work (chore)
- **T-017-04:** Push and verify CI green (chore)
- **T-017-05:** Tag and cut alpha release (task)
- **T-017-06:** Verify release artifacts and install paths (chore)

## Dependencies

```
T-017-01 (fmt + clippy)  ──┐
T-017-02 (archive)       ──┼── T-017-03 (commit)
                            │
                            └── T-017-04 (CI green)
                                  └── T-017-05 (tag release)
                                        └── T-017-06 (verify install)
```

T-017-01 and T-017-02 are independent and can be worked in parallel. Everything else is sequential.

## Success Criteria

1. `cargo fmt --check` exits 0
2. `cargo clippy --workspace` exits 0 with no warnings
3. `cargo test --workspace` passes all tests
4. `cargo check -p lisa-plugin --target wasm32-wasip1` is warning-free
5. No active tickets from S-010, S-011, or T-TEST-* remain
6. CI (GitHub Actions) is green on main
7. A version tag triggers the release workflow and produces artifacts for all four targets
8. At least one of: curl installer, cargo install, or direct binary download works end-to-end
