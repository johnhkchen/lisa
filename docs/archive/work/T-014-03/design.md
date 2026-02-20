# T-014-03 Design: Test install script and release binaries

## Decision: Version and Tag Strategy

### Options Considered

**Option A: Bump to v0.2.0-rc.1**
- Requires changing workspace version to `0.2.0-rc.1` in Cargo.toml
- Signals a new minor version even though no features changed since 0.1.6
- Prerelease suffix marks it correctly on GitHub
- Problem: cargo-dist matches tag version against Cargo.toml version — they must agree

**Option B: Tag as v0.1.6 (current version)**
- No version changes needed
- First actual release of the project
- Simple — tag matches Cargo.toml version exactly
- If the release works, it becomes the real v0.1.6 release
- If it fails, delete the tag/release, fix, and re-tag

**Option C: Bump to v0.1.7-rc.1 for testing, then v0.1.7 for real**
- Small version bump keeps semantic meaning
- Prerelease flag on GitHub
- Requires a Cargo.toml change

### Decision: Option B — Tag as v0.1.6

Rationale:
- No version changes needed — the workspace is already at 0.1.6
- This IS the first release; there's nothing to "protect" with a prerelease
- cargo-dist requires the tag version to match Cargo.toml, so this is the simplest path
- If the release fails, we delete the GitHub release + tag, fix, and re-tag
- The uncommitted changes on main include the cargo-dist setup itself, so they must be committed first anyway

## Decision: Test Execution Approach

### Options Considered

**Option A: Fully automated test script**
- Write a script that downloads, verifies checksums, runs binaries
- Overkill for a one-time verification

**Option B: Manual verification with documented checklist**
- Push the tag, watch the workflow
- Download artifacts manually, verify each item
- Document results in progress.md
- Simple, appropriate for a chore ticket

**Option C: PR-first to test plan step, then tag**
- First open a PR to verify the workflow's plan step runs
- Then tag for the full release
- Two-phase approach reduces risk

### Decision: Option C — PR-first, then tag

Rationale:
- The PR run exercises the plan job without creating any release artifacts
- This catches syntax errors in the workflow before we attempt a real release
- After the plan succeeds on PR, we merge (or just tag on main) for the full release
- Low cost, reduces wasted CI time on a broken workflow

## Execution Sequence

1. Ensure all cargo-dist related changes are committed on main
2. Push to origin
3. Open a test PR or push to main to trigger plan-only mode
4. Verify the plan job succeeds in GitHub Actions
5. Tag `v0.1.6` on main
6. Push the tag to trigger the full release workflow
7. Monitor all 5 jobs: plan → build-local → build-global → host → announce
8. Once the release is created, verify:
   - All 4 platform archives are present
   - Checksums file exists and is valid
   - `lisa-cli-installer.sh` is included
   - dist-manifest.json is correct
9. Test the installer on the local machine (macOS aarch64)
10. Download a binary directly and verify it runs
11. Document install URLs and results

## Verification Checklist

### Artifacts (automated by cargo-dist)
- [ ] `lisa-cli-aarch64-apple-darwin.tar.xz` — Apple Silicon binary
- [ ] `lisa-cli-x86_64-apple-darwin.tar.xz` — Intel Mac binary
- [ ] `lisa-cli-x86_64-unknown-linux-gnu.tar.xz` — Linux x86_64 binary
- [ ] `lisa-cli-aarch64-unknown-linux-gnu.tar.xz` — Linux ARM64 binary
- [ ] `lisa-cli-installer.sh` — Shell installer
- [ ] SHA256 checksums
- [ ] `source.tar.gz`

### Functional verification
- [ ] Downloaded binary is correct architecture (`file lisa`)
- [ ] `lisa --help` works
- [ ] `lisa doctor` works
- [ ] `lisa init` works in a temp directory
- [ ] `lisa validate` works on existing tickets
- [ ] Installer script installs to `~/.cargo/bin/`

### Install URLs to document
- Shell installer: `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh`
- GitHub Releases: `https://github.com/johnhkchen/lisa/releases`
- Direct download pattern: `https://github.com/johnhkchen/lisa/releases/download/v{VERSION}/lisa-cli-{TARGET}.tar.xz`

## What This Ticket Does NOT Do

- Does not test `cargo install lisa-cli` (that's T-014-02)
- Does not set up package managers (that's S-016)
- Does not rewrite the README (that's S-015)
- Does not test on Linux machines directly (CI handles that; we verify the artifacts exist)
