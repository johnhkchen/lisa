# T-017-05 Research: Tag and cut alpha release

## Current State

### Version
- Workspace version: `0.1.6` (Cargo.toml line 6)
- All three crates inherit via `version.workspace = true`
- Hard-coded dependency: `lisa-cli/Cargo.toml` line 19: `lisa-core = { version = "0.1.6", path = "../lisa-core" }`
- If bumping version, both workspace `[workspace.package].version` and the `lisa-core` dependency version in `lisa-cli/Cargo.toml` must be updated

### Git State
- Latest commit: `7ea8a6e` — "Add cargo-dist release infrastructure (T-014-01)"
- ~30 modified files and ~60 untracked files in working tree (per S-017 story)
- Upstream tickets T-017-01 through T-017-04 are all still `phase: ready`, `status: open`
- **None of the pending work has been committed or pushed yet**

### Dependency Chain
```
T-017-01 (fmt+clippy) ──┐
T-017-02 (archive)    ──┼── T-017-03 (commit) ── T-017-04 (CI green) ── T-017-05 (tag) ── T-017-06 (verify)
```
T-017-05 depends on T-017-04 (CI green). T-017-04 requires all code committed and pushed first (T-017-03).

### Release Infrastructure (cargo-dist)
- **dist-workspace.toml**: cargo-dist v0.30.4, targets: x86_64-apple-darwin, aarch64-apple-darwin, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu
- **Installers**: shell script + homebrew formula
- **Package**: only `lisa-cli` is distributed (correct — plugin is embedded)
- **build-setup.yml**: installs `wasm32-wasip1` and builds WASM plugin before dist runs
- **release.yml**: Triggers on tags matching `**[0-9]+.[0-9]+.[0-9]+*`, also runs on PRs (plan only)

### Release Workflow Jobs
1. **plan** — `dist plan` or `dist host --steps=create` depending on PR vs tag push
2. **build-local-artifacts** — matrix build across 4 targets, installs wasm32-wasip1, builds WASM plugin, then runs `dist build`
3. **build-global-artifacts** — generates checksums, shell installer
4. **host** — uploads artifacts, creates GitHub Release via `gh release create`
5. **publish-homebrew-formula** — pushes formula to `johnhkchen/homebrew-lisa` (requires `HOMEBREW_TAP_TOKEN` secret)
6. **announce** — final step, runs after host + homebrew

### CI Workflow (ci.yml)
Runs on push to main and PRs:
1. cargo fmt --all -- --check
2. cargo clippy -p lisa-core -- -D warnings
3. cargo clippy -p lisa-cli -- -D warnings
4. cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
5. cargo test --workspace
6. cargo check -p lisa-plugin --target wasm32-wasip1

### Tag Format
cargo-dist expects tags like `v0.1.6` or `v0.2.0`. The regex in release.yml is `**[0-9]+.[0-9]+.[0-9]+*` which matches both `v0.1.6` and `0.1.6` (the `**` prefix catches the `v`).

### Known Issues
- **Homebrew publish will fail** without `HOMEBREW_TAP_TOKEN` secret — this is expected and non-blocking per the ticket
- **WASM build in CI** — build-setup.yml handles this, but it's the first real test of this pipeline
- **Uncommitted work** — T-017-03 and T-017-04 must complete first; this ticket cannot proceed until the working tree is clean and CI is green on main

### Version Decision Factors
- `0.1.6` is the current version; keeping it means no additional commits needed
- `0.2.0` would signal the milestone (doctor, cargo-dist, docs, distribution infra) but requires updating:
  1. `Cargo.toml` workspace version
  2. `crates/lisa-cli/Cargo.toml` lisa-core dependency version
  3. Regenerating `Cargo.lock`
  4. An extra commit + CI cycle

## Files Involved
- `Cargo.toml` — workspace version
- `crates/lisa-cli/Cargo.toml` — lisa-core dependency version (if bumping)
- `dist-workspace.toml` — cargo-dist config
- `.github/workflows/release.yml` — release workflow
- `.github/build-setup.yml` — WASM build setup for dist
- `.github/workflows/ci.yml` — CI checks

## Constraints
- This ticket is blocked by T-017-04 (CI must be green first)
- The actual tagging operation requires all prior commits pushed to origin/main
- First-ever release — no prior releases to compare against
- The version in Cargo.toml at the tagged commit must match the tag (cargo-dist enforces this)
