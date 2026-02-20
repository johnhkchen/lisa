# T-014-03 Review: Test install script and release binaries

## Summary

This ticket verified the cargo-dist release infrastructure set up by T-014-01. The work completed the local pre-flight validation and committed/pushed the release infrastructure to origin/main. The full end-to-end release test (tagging, artifact verification, installer testing) was not completed — the ticket reached review timeout before the tag was pushed.

## What Was Done

1. **Local pre-flight passed** — `dist plan` confirms correct manifest:
   - `lisa-cli 0.1.6` as only distributable package
   - All 4 targets: x86_64-apple-darwin, aarch64-apple-darwin, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu
   - Shell installer (`lisa-cli-installer.sh`)
   - SHA256 checksums for all artifacts
   - Binary name: `lisa` (correct)

2. **Committed and pushed release infrastructure** — 6 files committed as `7ea8a6e`:
   - `Cargo.toml` — version 0.1.6 + `[profile.dist]`
   - `crates/lisa-cli/Cargo.toml` — lisa-core version dep bump
   - `Cargo.lock` — version updates
   - `dist-workspace.toml` — cargo-dist config (new)
   - `.github/build-setup.yml` — WASM pre-build steps (new)
   - `.github/workflows/release.yml` — cargo-dist generated workflow (replaced)

3. **RDSPI artifacts produced** — research.md, design.md, structure.md, plan.md, progress.md

## Files Modified (committed)

| File | Change |
|------|--------|
| `Cargo.toml` | Version 0.1.0 → 0.1.6, added `[profile.dist]` |
| `crates/lisa-cli/Cargo.toml` | lisa-core dep 0.1.0 → 0.1.6 |
| `Cargo.lock` | Version updates |
| `dist-workspace.toml` | New — cargo-dist configuration |
| `.github/build-setup.yml` | New — WASM pre-build steps for CI |
| `.github/workflows/release.yml` | Replaced hand-rolled with cargo-dist generated |

## Open Concerns / TODOs

1. **Tag not pushed yet** — `v0.1.6` tag has not been created or pushed. The release workflow has not been triggered. To complete the actual release test:
   ```bash
   git tag v0.1.6
   git push origin v0.1.6
   ```

2. **Installer not tested** — The shell installer (`lisa-cli-installer.sh`) has not been tested on any platform since no release exists yet.

3. **CI formatting failures** — Pre-existing `cargo fmt` issues in `crates/lisa-plugin/src/ui.rs` cause CI to fail. This is unrelated to the release pipeline (release workflow doesn't run fmt checks) but should be fixed separately.

4. **Install URLs documented but unverified** — The README already contains the install URL pattern but the `latest` download URL won't resolve until the first release is published.

5. **Acceptance criteria partially met:**
   - [x] `dist plan` confirms all four platform binaries will be produced
   - [ ] Actual release tag has not been pushed
   - [ ] Checksums not yet verified (no release exists)
   - [ ] Install script not tested on any platform
   - [ ] Installed binary not tested
   - [x] Install URLs documented in README (from prior work)
