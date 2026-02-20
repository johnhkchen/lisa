# T-014-03 Progress: Test install script and release binaries

## Completed

### Step 1: Local pre-flight
- `dist plan` succeeds locally
- Output confirms: `lisa-cli 0.1.6` as only distributable
- All 4 targets listed: x86_64-apple-darwin, aarch64-apple-darwin, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu
- Shell installer (`lisa-cli-installer.sh`) listed
- SHA256 checksums for all artifacts
- Binary name: `lisa` (correct, from `[[bin]] name = "lisa"`)

### Step 2: Commit and push release infra
- Committed 6 files (cargo-dist infrastructure only):
  - `Cargo.toml` (version 0.1.6 + profile.dist)
  - `crates/lisa-cli/Cargo.toml` (lisa-core version dep bump)
  - `Cargo.lock` (version updates)
  - `dist-workspace.toml` (cargo-dist config, new)
  - `.github/build-setup.yml` (WASM pre-build steps, new)
  - `.github/workflows/release.yml` (cargo-dist generated, replaced)
- Pushed to origin/main: commit 7ea8a6e
- CI run failed (pre-existing `cargo fmt` issue in ui.rs, unrelated to release infra)
- Release workflow is independent of CI — format issues won't block the release

## Remaining

### Step 3: Tag and push
- Waiting for user to create tag: `git tag v0.1.6 && git push origin v0.1.6`

### Step 4-8: Monitor, verify, test
- Pending tag push

## Notes

- CI has pre-existing `cargo fmt` failures in `crates/lisa-plugin/src/ui.rs` (from uncommitted changes that were partially committed in earlier sprints). This is a separate issue from the release pipeline.
- 332 tests pass locally (123 cli + 78 core + 131 plugin).
- The release workflow triggers on tag pushes matching `**[0-9]+.[0-9]+.[0-9]+*`, NOT on regular pushes to main.
