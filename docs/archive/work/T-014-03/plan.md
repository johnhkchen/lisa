# T-014-03 Plan: Test install script and release binaries

## Step 1: Local pre-flight

Run `dist plan` locally to verify the manifest is correct before pushing anything.

**Verify:**
- Output lists `lisa-cli 0.1.6` as the distributable
- All 4 targets listed
- Shell installer listed
- No errors

## Step 2: Check commit state

Verify all cargo-dist files are committed:
- `dist-workspace.toml`
- `.github/build-setup.yml`
- `.github/workflows/release.yml`
- `Cargo.toml` (profile.dist section)

If not committed, this ticket cannot proceed — the changes need to be on main first. Note: this ticket's role is to *test* the release, not to commit infrastructure changes. If the infrastructure isn't committed, flag it and stop.

## Step 3: Tag and push

```bash
git tag v0.1.6
git push origin v0.1.6
```

This triggers the release workflow.

## Step 4: Monitor the workflow

Use `gh` CLI to watch the release run:

```bash
gh run list --workflow=release.yml
gh run watch <run-id>
```

Wait for all jobs to complete. Expected duration: 10-20 minutes.

## Step 5: Verify the GitHub Release

```bash
gh release view v0.1.6
```

Check:
- [ ] Release exists and is not a draft
- [ ] Title is correct
- [ ] All 4 platform archives are listed
- [ ] `lisa-cli-installer.sh` is listed
- [ ] Checksum files are listed

## Step 6: Download and test local binary

```bash
# Download the binary for this machine (aarch64-apple-darwin)
gh release download v0.1.6 --pattern 'lisa-cli-aarch64-apple-darwin.tar.xz' --dir /tmp/lisa-test
cd /tmp/lisa-test
tar xf lisa-cli-aarch64-apple-darwin.tar.xz
file lisa-cli-aarch64-apple-darwin/lisa
./lisa-cli-aarch64-apple-darwin/lisa --help
./lisa-cli-aarch64-apple-darwin/lisa doctor
```

## Step 7: Test the installer

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/johnhkchen/lisa/releases/download/v0.1.6/lisa-cli-installer.sh | sh
```

After install:
```bash
which lisa
lisa --help
lisa doctor
```

## Step 8: Document results

Record all outcomes in `progress.md`:
- CI job statuses
- Artifact list from the release
- Binary verification results
- Installer test results
- Final install URLs

## Step 9: Update ticket

Set phase to `done`, status to `done`.

## Testing Strategy

This ticket IS the test. Each step above is a verification step. Results are documented in progress.md. There are no unit tests to write — this is end-to-end infrastructure verification.

## Failure Recovery

If any step fails:
1. Document the failure in progress.md
2. If the tag was pushed, delete it: `git push origin :refs/tags/v0.1.6 && git tag -d v0.1.6`
3. If a release was created, delete it: `gh release delete v0.1.6 --yes`
4. Fix the issue
5. Re-tag and retry from Step 3
