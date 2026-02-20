# T-016-03 Plan: Publish AUR package

## Step 1: Create `aur/` directory and PKGBUILD

Create `aur/PKGBUILD` with:
- All metadata fields (pkgname, pkgver, pkgdesc, arch, url, license, depends, provides, conflicts)
- Architecture-specific source arrays pointing to cargo-dist release URLs
- Placeholder checksums (will be updated after T-014-03 produces a release)
- `package()` function that installs the binary and LICENSE

**Verification:** Review the PKGBUILD for correctness against AUR conventions. No automated test possible without Arch tooling, but the file should be syntactically valid bash.

**Commit:** "Add AUR PKGBUILD for lisa-bin package"

## Step 2: Document the AUR submission and update process

Add maintenance instructions to the PKGBUILD as comments and to progress.md:
- How to generate .SRCINFO
- How to submit to AUR for the first time
- How to update for new releases (pkgver bump + checksums)
- What to do after T-014-03 produces the first release

**Verification:** Instructions are clear and complete.

**Commit:** Part of Step 1 commit (single atomic commit).

## Step 3: Update ticket phase to `implement` → `review`

After the PKGBUILD is written:
- Update progress.md with completion status
- Note that actual AUR submission is blocked on T-014-03 (no releases yet)
- Note that .SRCINFO generation requires Arch Linux

**Verification:** Ticket frontmatter reflects current phase.

## Testing Strategy

### What can be verified now
- PKGBUILD is valid bash syntax (`bash -n aur/PKGBUILD`)
- All required PKGBUILD fields are present
- Source URLs follow the cargo-dist naming pattern
- `provides` and `conflicts` are correctly set

### What requires T-014-03 completion
- Real checksums (download actual tarballs)
- `makepkg --printsrcinfo` (needs Arch Linux)
- `makepkg -si` (needs Arch Linux + release artifacts)
- `yay -S lisa-bin` (needs AUR submission)

### What requires manual action
- AUR account creation
- SSH key setup
- Initial `git push` to AUR

## Sequencing

Step 1 and Step 2 are a single commit. Step 3 is metadata updates only.
