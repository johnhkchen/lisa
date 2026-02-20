---
id: T-014-03
title: Test install script and release binaries
type: chore
phase: done
status: done
priority: medium
story: S-014
created: 2026-02-20
depends_on:
  - T-014-01
---

# T-014-03: Test install script and release binaries

## Objective

End-to-end verification that the cargo-dist release pipeline produces working binaries and a functioning install script.

## Steps

1. **Create a test release**
   - Tag a pre-release version (e.g., `v0.2.0-rc.1`)
   - Push the tag and let the release workflow run
   - Verify the GitHub Release is created with all expected artifacts

2. **Verify artifacts**
   - Download each platform binary and check it's the correct architecture
   - Verify checksums match
   - Confirm the install script is included in the release

3. **Test install script**
   - Run the curl installer on macOS (both Intel and Apple Silicon if available)
   - Run the curl installer on Linux (x86_64 at minimum)
   - Verify the installed binary runs: `lisa --help`, `lisa doctor`
   - Verify the install script handles PATH placement correctly

4. **Test binary directly**
   - Download a release binary manually
   - Make it executable and run it
   - Verify `lisa doctor`, `lisa init`, `lisa validate` work
   - Verify `lisa loop` launches (with deps present)

5. **Document the install URLs**
   - Record the final curl one-liner for the README
   - Record the GitHub Releases URL

## Acceptance Criteria

- [ ] Test release tag produces all four platform binaries
- [ ] Checksums are present and valid
- [ ] Install script works on at least one macOS and one Linux platform
- [ ] Installed binary runs correctly
- [ ] Install URLs are documented for S-015 (README rewrite)
