---
id: T-017-04
title: Push and verify CI green
type: chore
phase: ready
status: open
priority: high
story: S-017
created: 2026-02-20
depends_on:
  - T-017-03
---

# T-017-04: Push and verify CI green

## Objective

Push all commits to `origin/main` and verify that GitHub Actions CI passes. If CI fails, fix the issues and push again.

## CI checks (from `.github/workflows/ci.yml`)

The CI workflow runs:
1. `cargo fmt --check` — formatting
2. `cargo clippy -p lisa-core -- -D warnings` — core lint
3. `cargo clippy -p lisa-cli -- -D warnings` — CLI lint
4. `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings` — plugin lint (WASM target)
5. `cargo test --workspace` — all tests
6. `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM compilation

## Steps

1. Push to main: `git push origin main`
2. Monitor CI at: https://github.com/johnhkchen/lisa/actions
3. If any check fails:
   - Read the failure log
   - Fix locally
   - Push the fix
   - Repeat until green

## Common failure modes

- **clippy on WASM target** — Some clippy lints behave differently with `--target wasm32-wasip1`. The local `cargo clippy --workspace` check may pass but CI's per-crate WASM clippy may not. If this happens, fix the specific lint for the WASM target.
- **Missing wasm32-wasip1 in CI** — The CI workflow should install this target. Verify the workflow has `rustup target add wasm32-wasip1`.
- **Dependency resolution** — `Cargo.lock` must be committed and up to date.

## Acceptance Criteria

- [ ] `git push origin main` succeeds
- [ ] All 6 CI checks pass (green checkmark on the commit)
- [ ] CI URL is recorded for reference
