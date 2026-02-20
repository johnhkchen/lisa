# T-016-03 Progress: Publish AUR package

## Completed

### PKGBUILD created (`aur/PKGBUILD`)
- `lisa-bin` package targeting x86_64 and aarch64
- Downloads prebuilt binaries from GitHub Releases (cargo-dist artifacts)
- Declares `zellij` and `gcc-libs` as dependencies
- `provides=('lisa')` / `conflicts=('lisa')` set correctly
- Placeholder checksums (`SKIP`) — must be replaced after first release
- Bash syntax validated

## Blocked

### Checksums (blocked on T-014-03)
No GitHub Releases exist yet. Once T-014-03 produces a release:
1. Download both Linux tarballs
2. Compute sha256 checksums
3. Replace `SKIP` in PKGBUILD with real values

### .SRCINFO generation (requires Arch Linux)
Cannot run `makepkg --printsrcinfo` without Arch tooling. Generate on an Arch system before pushing to AUR.

### AUR submission (requires account + release)
Cannot push to AUR until both the account exists and the release artifacts are live.

## Remaining Steps

### After T-014-03 is complete:
1. Verify the exact tarball filenames from the GitHub Release match the PKGBUILD source URLs
2. Download tarballs and compute real sha256 checksums
3. Verify the tarball contains the `lisa` binary and `LICENSE` file
4. Update PKGBUILD with real checksums

### AUR submission:
1. Create AUR account at https://aur.archlinux.org
2. Add SSH key: `ssh-keygen -f ~/.ssh/aur` + add pub key to AUR profile
3. Configure SSH: add `Host aur.archlinux.org` block to `~/.ssh/config`
4. Clone: `git clone ssh://aur@aur.archlinux.org/lisa-bin.git`
5. Copy PKGBUILD into the cloned repo
6. Generate: `makepkg --printsrcinfo > .SRCINFO`
7. Commit: `git add PKGBUILD .SRCINFO && git commit -m "Initial upload: lisa-bin 0.1.6"`
8. Push: `git push`
9. Verify: package appears at https://aur.archlinux.org/packages/lisa-bin

### For each new release:
1. Update `pkgver` in PKGBUILD
2. Run `updpkgsums` (or manually update checksums)
3. Regenerate: `makepkg --printsrcinfo > .SRCINFO`
4. Commit and push to AUR
5. Also update `aur/PKGBUILD` in the main repo
