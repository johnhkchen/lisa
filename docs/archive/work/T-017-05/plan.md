# T-017-05 Plan: Tag and cut alpha release

## Prerequisites
- T-017-04 must be complete (CI green on main, all prior work committed and pushed)
- Working tree must be clean (no uncommitted changes that affect the build)

## Steps

### Step 1: Verify Preconditions
- Confirm the latest commit on main is pushed to origin
- Confirm CI is green (check GitHub Actions or `gh run list`)
- Confirm working tree is clean for tracked files that affect the build

### Step 2: Bump Version to 0.2.0
- Edit `Cargo.toml` line 6: `version = "0.1.6"` → `version = "0.2.0"`
- Edit `crates/lisa-cli/Cargo.toml` line 19: `lisa-core = { version = "0.1.6"` → `lisa-core = { version = "0.2.0"`
- Run `cargo check --workspace` to regenerate `Cargo.lock` and verify compilation

### Step 3: Commit and Push Version Bump
- Stage: `Cargo.toml`, `crates/lisa-cli/Cargo.toml`, `Cargo.lock`
- Commit: "Bump version to 0.2.0"
- Push: `git push origin main`
- Wait for CI green on the bump commit

### Step 4: Create and Push Tag
- `git tag v0.2.0`
- `git push origin v0.2.0`
- This triggers the release workflow in `.github/workflows/release.yml`

### Step 5: Monitor Release Workflow
- Check `gh run list --workflow=release.yml` or GitHub Actions UI
- Expected jobs: plan → build-local-artifacts (4 targets) → build-global-artifacts → host → publish-homebrew-formula (expected fail) → announce
- The `host` job creates the GitHub Release
- `publish-homebrew-formula` will likely fail (no HOMEBREW_TAP_TOKEN) — this is expected

### Step 6: Verify Release Artifacts
- Check GitHub Release page: `gh release view v0.2.0`
- Confirm presence of:
  - 4 platform archives (tar.xz)
  - Shell installer script
  - SHA256 checksums
- This overlaps with T-017-06 but a quick check here confirms the pipeline worked

## Rollback Plan
If the release workflow fails:
1. Investigate the failure from workflow logs
2. Fix the issue locally
3. Delete the tag: `git tag -d v0.2.0 && git push origin :refs/tags/v0.2.0`
4. Delete the draft release if one was created: `gh release delete v0.2.0 --yes`
5. Fix, commit, push, re-tag

## Testing Strategy
- `cargo check --workspace` after version bump (Step 2)
- CI runs automatically on push (Step 3)
- Release workflow runs on tag push (Step 4)
- Manual verification of release artifacts (Step 6)

No unit tests are added — this ticket is purely operational (version bump + tag).
