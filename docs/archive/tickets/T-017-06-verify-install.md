---
id: T-017-06
title: Verify release artifacts and install paths
type: chore
phase: done
status: done
priority: medium
story: S-017
created: 2026-02-20
depends_on:
  - T-017-05
---

# T-017-06: Verify release artifacts and install paths

## Objective

End-to-end verification that at least one install path works. This proves the release pipeline is functional and users can actually install Lisa.

## Install paths to verify

### 1. Direct binary download (must verify)

- Download the macOS arm64 binary from the GitHub Release
- Extract and run:
  ```bash
  tar xf lisa-cli-aarch64-apple-darwin.tar.xz
  ./lisa --help
  ./lisa --version
  ./lisa doctor
  ```

### 2. Shell installer (must verify)

- Run the curl one-liner from the README
- Verify `lisa` is installed and on PATH
- Run `lisa --help` and `lisa doctor`

### 3. Cargo install (should verify)

- This requires `lisa-core` to be published to crates.io first
- If not publishing to crates.io yet, verify `cargo install --path crates/lisa-cli` still works locally
- Note: `cargo install lisa-cli` from crates.io can be deferred

### 4. Homebrew (deferred)

- Requires `HOMEBREW_TAP_TOKEN` secret
- Will be verified in a follow-up once the PAT is created

## Verification checklist

For each verified path:
- [ ] Binary installs without errors
- [ ] `lisa --help` shows expected output
- [ ] `lisa --version` prints the correct version
- [ ] `lisa doctor` runs and checks for dependencies
- [ ] `lisa init --dry-run` works in a temp directory

## Record results

Document in `docs/active/work/T-017-06/progress.md`:
- Which paths were tested
- Platform and architecture tested on
- Any issues encountered
- The final install URL/command that works

## Acceptance Criteria

- [x] At least one install path verified end-to-end (direct download or shell installer)
- [x] Installed binary is the correct version
- [x] `lisa doctor` runs correctly from the installed binary
- [x] Results documented
