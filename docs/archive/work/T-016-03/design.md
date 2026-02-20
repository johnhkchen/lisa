# T-016-03 Design: Publish AUR package

## Decision 1: Where should the PKGBUILD live?

### Option A: In this repo under `aur/`
- Pros: versioned alongside the code, easy to update in the same PR as a version bump
- Cons: PKGBUILD still needs to be pushed separately to AUR git; two sources of truth

### Option B: Separate AUR-only repo
- Pros: clean separation, AUR git is the single source of truth
- Cons: harder to maintain, easy to forget updates

### Option C: In this repo + CI copies to AUR on release
- Pros: single source of truth in this repo, automated sync
- Cons: requires AUR SSH key in CI secrets, more complex setup

**Decision: Option A** — Keep the PKGBUILD in this repo under `aur/PKGBUILD` for reference and version control. The actual AUR submission is a manual `git push` to `aur.archlinux.org`. CI automation (Option C) can be added later once the manual process is proven. This is what most projects do initially.

## Decision 2: Architecture support

### Option A: x86_64 only
- Simpler PKGBUILD, matches majority of AUR -bin packages
- cargo-dist produces aarch64 anyway, so this artificially limits users

### Option B: x86_64 + aarch64 from day one
- Both tarballs are produced by cargo-dist already
- Architecture-specific `source_x86_64` / `source_aarch64` arrays are well-supported
- Slightly more checksum maintenance (two per release instead of one)

**Decision: Option B** — Support both architectures. The cost is minimal (two extra lines in PKGBUILD per release) and cargo-dist already produces both. lazygit-bin shows this pattern works cleanly.

## Decision 3: Dependencies

### Mandatory
- `zellij` — hard runtime dependency, available in official `extra` repo
- `gcc-libs` — required for glibc-linked binaries

### Optional
- Claude Code — not packaged for Arch, cannot be a `depends`. Not worth an `optdepends` since AUR users know what they're installing.

**Decision:** `depends=('zellij' 'gcc-libs')`. No optdepends — keep it minimal.

## Decision 4: LICENSE handling

### Option A: Download LICENSE separately from raw GitHub
- Extra `source=()` entry pointing to raw.githubusercontent.com
- Adds a network request and a checksum to maintain

### Option B: Include LICENSE in the cargo-dist tarball
- cargo-dist includes LICENSE in the archive by default
- No extra download needed
- Need to verify this with actual release artifacts

### Option C: Skip LICENSE installation
- Some -bin packages do this, but it's bad practice

**Decision: Option B preferred, Option A as fallback.** cargo-dist typically includes LICENSE in the tarball. If it doesn't (verified once T-014-03 is done), fall back to downloading it separately. The PKGBUILD template will assume Option B and note the fallback.

## Decision 5: Checksum type

**Decision: sha256** — It's the most common convention for AUR -bin packages. sha512 adds no practical security benefit for this use case and is less conventional.

## Decision 6: Shell completions

Lisa doesn't currently have `--generate-completion`. No completions to install.

**Decision:** Skip completions. Can be added later when/if Lisa gains completion generation.

## Decision 7: Update automation

**Decision:** Document a manual update process for now. Automation via CI can be a follow-up ticket. The manual process is:
1. Update `pkgver` in PKGBUILD
2. Update checksums (download new tarballs, compute sha256)
3. Regenerate .SRCINFO
4. Commit and push to AUR

## Final Design

Create `aur/PKGBUILD` in this repo with:
- `pkgname=lisa-bin`, dual-arch (x86_64 + aarch64)
- `depends=('zellij' 'gcc-libs')`
- `provides=('lisa')`, `conflicts=('lisa')`
- sha256 checksums, architecture-specific source arrays
- `package()` installs binary to `/usr/bin/lisa` and LICENSE to `/usr/share/licenses/`
- Placeholder checksums until first release is cut

Create `aur/.SRCINFO` placeholder (must be generated on Arch with `makepkg --printsrcinfo`).

Document the AUR submission and update process in the ticket's progress.md.

## What Was Rejected

- **Separate repo for PKGBUILD**: Unnecessary overhead for a single file
- **CI-automated AUR push**: Premature — prove manual workflow first
- **x86_64-only**: No reason to limit when both tarballs exist
- **optdepends for Claude Code**: Adds noise, users already know the context
- **sha512**: Non-standard for the AUR -bin ecosystem
- **Shell completions**: Lisa doesn't support them yet
