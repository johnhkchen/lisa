# T-014-03 Structure: Test install script and release binaries

## Overview

This is a chore ticket — primarily verification, minimal code changes. The "structure" is the sequence of operations and what files are touched.

## Files Modified

### No source code changes expected

This ticket verifies existing infrastructure. The only file modifications are:

1. **`docs/active/tickets/T-014-03-test-release.md`** — phase transitions through RDSPI
2. **`docs/active/work/T-014-03/progress.md`** — test results documented here

## Operations Structure

### Phase 1: Pre-flight checks (local)

Verify the release infrastructure is ready before pushing anything:
- Run `dist plan` locally to confirm manifest is correct
- Verify `lisa-cli` is the only distributable package
- Confirm the 4 target triples are listed
- Confirm the shell installer is listed

### Phase 2: Commit and push

All cargo-dist changes from T-014-01 must be on `main` at `origin`:
- Verify the relevant files are committed: `dist-workspace.toml`, `.github/build-setup.yml`, `.github/workflows/release.yml`, `Cargo.toml` changes
- Push to origin

### Phase 3: Tag and release

- Create tag: `git tag v0.1.6`
- Push tag: `git push origin v0.1.6`
- This triggers the release workflow

### Phase 4: Monitor CI

Watch the GitHub Actions workflow:
- Job 1: `plan` — should complete quickly
- Job 2: `build-local-artifacts` — 4 parallel matrix builds, slowest step
- Job 3: `build-global-artifacts` — installer + checksums
- Job 4: `host` — upload and create release
- Job 5: `announce` — no-op currently

### Phase 5: Verify release artifacts

Once the release is created on GitHub:
- Check the release page for all expected artifacts
- Download the macOS aarch64 binary (matches local machine)
- Verify architecture: `file lisa`
- Run: `./lisa --help`, `./lisa doctor`

### Phase 6: Test installer

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh
```

- Verify it installs to `~/.cargo/bin/lisa`
- Run `lisa --help` from the installed location

### Phase 7: Document results

Record all results in `progress.md`:
- Which artifacts were produced
- Which verifications passed/failed
- Final install URLs for S-015 to use

## Failure Handling

If the release workflow fails:
1. Read the CI logs to identify the failure
2. Fix locally
3. Delete the tag: `git tag -d v0.1.6 && git push origin :refs/tags/v0.1.6`
4. Delete the draft release on GitHub if created
5. Commit the fix, re-tag, re-push

If specific platform builds fail but others succeed:
- The release may still be created with partial artifacts
- Document which platforms failed and why
- File follow-up issues if needed

## Dependencies

- Requires: GitHub Actions runners (GitHub-provided)
- Requires: Network access to push tags and download artifacts
- Requires: `gh` CLI for checking release status
