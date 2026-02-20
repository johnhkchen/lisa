---
id: T-016-03
title: Publish AUR package
type: task
phase: done
status: done
priority: low
story: S-016
created: 2026-02-20
depends_on:
  - T-014-03
---

# T-016-03: Publish AUR package

## Objective

Publish a `lisa-bin` package to the Arch User Repository (AUR) that installs the prebuilt binary from GitHub Releases.

## Requirements

### PKGBUILD (`lisa-bin`)

Create a `PKGBUILD` that:
- Downloads the `x86_64-unknown-linux-gnu` binary from the GitHub Release
- Verifies checksum
- Installs to `/usr/bin/lisa`
- Declares `depends=('zellij')`
- Includes a `.SRCINFO` generated from the PKGBUILD

### Publishing

- Create an AUR account if needed
- Push the `lisa-bin` package to the AUR
- Verify installation: `yay -S lisa-bin` or `paru -S lisa-bin`

### Optional: source variant (`lisa`)

A second PKGBUILD that builds from source (`cargo build --release`) for users who prefer it. Lower priority than the `-bin` variant.

### Maintenance

Document the update process for new releases:
- Update `pkgver` and checksums
- Push to AUR
- Consider automating with a CI step

## Acceptance Criteria

- [ ] `lisa-bin` PKGBUILD exists and passes `makepkg --printsrcinfo`
- [ ] Package is published to AUR
- [ ] `yay -S lisa-bin` or `paru -S lisa-bin` installs lisa
- [ ] Zellij declared as dependency
- [ ] `lisa --help` works after install
