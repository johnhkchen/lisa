# T-016-03 Structure: Publish AUR package

## Files Created

### `aur/PKGBUILD`
The AUR package definition. This lives in the main repo for version control but is pushed separately to the AUR git repo.

```
aur/
  PKGBUILD          # Package build instructions
```

Contents:
- Maintainer comment header
- `pkgname=lisa-bin` with `_pkgname` helper variable
- `pkgver`, `pkgrel` fields
- `pkgdesc` — one-line description
- `arch=('x86_64' 'aarch64')`
- `url` — project homepage
- `license=('MIT')`
- `depends=('zellij' 'gcc-libs')`
- `provides=('lisa')`, `conflicts=('lisa')`
- `source_x86_64=()`, `source_aarch64=()` — GitHub Release tarball URLs
- `sha256sums_x86_64=()`, `sha256sums_aarch64=()` — placeholder until first release
- `package()` function:
  - `install -Dm755 "lisa" "$pkgdir/usr/bin/lisa"`
  - `install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"` (if LICENSE is in tarball)

### No `.SRCINFO` in this repo
`.SRCINFO` must be generated on an Arch Linux system via `makepkg --printsrcinfo`. It cannot be reliably created without Arch tooling. It will be generated during the AUR submission step, not stored in this repo.

## Files Modified

None. This ticket creates new files only.

## External Actions (not code changes)

### AUR Account Setup
1. Create account at aur.archlinux.org
2. Add SSH public key to account profile

### AUR Git Repository
1. `git clone ssh://aur@aur.archlinux.org/lisa-bin.git`
2. Copy PKGBUILD into the cloned repo
3. Generate .SRCINFO with `makepkg --printsrcinfo > .SRCINFO`
4. Commit PKGBUILD + .SRCINFO to `master`
5. Push to AUR

## Module Boundaries

This ticket is entirely self-contained:
- No changes to any Rust crate
- No changes to CI/CD pipeline
- No changes to existing build process
- The PKGBUILD is a standalone shell script consumed by pacman/makepkg

## Dependency on T-014-03

The PKGBUILD references GitHub Release URLs that don't exist yet. The structure uses placeholder checksums (`SKIP` or `0000...`) that must be replaced once the first release is available. The PKGBUILD is still valid as a template — `makepkg --printsrcinfo` will generate .SRCINFO from it even with placeholder checksums.

## Directory Layout After Implementation

```
aur/
  PKGBUILD              # New: AUR package definition for lisa-bin
docs/active/work/T-016-03/
  research.md           # Existing
  design.md             # Existing
  structure.md          # This file
  plan.md               # Next phase
  progress.md           # Implementation tracking
```
