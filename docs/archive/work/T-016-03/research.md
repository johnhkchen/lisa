# T-016-03 Research: Publish AUR package

## Current State

### Project Release Infrastructure
- **cargo-dist v0.30.4** drives the release pipeline via `.github/workflows/release.yml`
- Configured in `dist-workspace.toml` with `packages = ["lisa-cli"]`
- Linux targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
- Installers: `["shell"]` — no AUR integration from cargo-dist
- Version: `0.1.6` (workspace-level in `Cargo.toml`)
- Binary name: `lisa` (set in `crates/lisa-cli/Cargo.toml` `[[bin]]`)
- License: MIT, `LICENSE` file exists at repo root
- Repository: `https://github.com/johnhkchen/lisa`

### Dependency: T-014-03
- Status: `in_progress` — no GitHub Releases exist yet
- Once complete, releases will produce tarballs like:
  - `lisa-cli-x86_64-unknown-linux-gnu.tar.xz`
  - `lisa-cli-aarch64-unknown-linux-gnu.tar.xz`
- Exact naming depends on cargo-dist conventions (may include version in the name)

### Sibling Tickets
- **T-016-01** (Homebrew tap): `ready`, also depends on T-014-03
- **T-016-02** (Nix flake): `done` — builds from source via crane, wraps binary with zellij on PATH

## AUR `-bin` Package Conventions

### PKGBUILD Structure
Standard `-bin` PKGBUILD for prebuilt GitHub Release binaries:
- `pkgname=lisa-bin` with `_pkgname=${pkgname%-bin}`
- `provides=('lisa')` and `conflicts=('lisa')` — prevents collision with a source variant
- No `build()` function — nothing to compile
- `package()` uses `install -Dm755` for binaries, `install -Dm644` for LICENSE
- Architecture-specific sources via `source_x86_64=()` / `source_aarch64=()` arrays
- Matching `sha256sums_x86_64=()` / `sha256sums_aarch64=()` arrays
- `pkgrel=1` for each new upstream version

### Dependencies
- `zellij` is in the official Arch `extra` repo — can be a direct `depends`
- `gcc-libs` is common for `*-unknown-linux-gnu` binaries (links glibc)
- Claude Code cannot be expressed as a pacman dep — use `optdepends` or a post-install note
- Pattern: `depends=('zellij' 'gcc-libs')`

### .SRCINFO
- Generated with `makepkg --printsrcinfo > .SRCINFO`
- Never hand-edited — always regenerated from PKGBUILD
- Must be committed alongside every PKGBUILD change

### AUR Submission Process
1. Create AUR account at `https://aur.archlinux.org`
2. Add SSH key to account
3. `git clone ssh://aur@aur.archlinux.org/lisa-bin.git` (empty repo)
4. Add PKGBUILD + .SRCINFO, commit to `master` branch, push
5. Updates: edit PKGBUILD, regenerate .SRCINFO, commit, push

### Checksum Management
- `updpkgsums` (from `pacman-contrib`) auto-downloads sources and updates checksums in-place
- Manual: `curl -L <url> | sha256sum`
- For CI automation: download the release, compute sha256, template into PKGBUILD

## Patterns from Other Rust CLI `-bin` Packages

| Package | Architectures | Checksum | Dependencies | LICENSE handling |
|---------|--------------|----------|--------------|-----------------|
| zellij-bin | x86_64 only | sha256 | none | from tarball |
| atuin-bin | x86_64 only | sha512 | gcc-libs | from tarball |
| lazygit-bin | x86_64 + aarch64 | sha256 | none | separate download |
| navi-bin | x86_64 only | sha256 | none | N/A |
| starship-bin | x86_64 only | sha256 | openssl, zlib | N/A |

Key observations:
- Most support x86_64 only, even when upstream provides aarch64
- sha256 is the dominant checksum type
- `provides` + `conflicts` are universal
- Shell completions are generated in `build()` if the binary supports it (e.g., `--generate-completion`)

## Files and Assets Involved

### What needs to be created (in this repo, for reference/automation)
- `aur/PKGBUILD` — the package definition (can live in repo for maintenance)
- `aur/.SRCINFO` — generated metadata

### What needs to be created externally
- AUR account + SSH key setup
- AUR git repo at `aur.archlinux.org/lisa-bin.git`

### What needs to exist first (from T-014-03)
- GitHub Release with Linux tarballs and checksums
- Stable release URL pattern for cargo-dist artifacts

## Constraints and Risks

1. **Blocked on T-014-03**: Cannot generate real checksums or test installation until releases exist
2. **Artifact naming**: cargo-dist naming convention needs verification once first release is cut
3. **No Arch Linux available locally**: Cannot run `makepkg --printsrcinfo` without Arch tooling
4. **AUR account**: Requires manual account creation — cannot be automated in this ticket
5. **Version sync**: Each Lisa release requires a PKGBUILD update (pkgver + checksums)
6. **Lisa doesn't generate shell completions**: No `--generate-completion` flag currently, so no completions to install

## Open Questions

1. Should the PKGBUILD live in this repo (e.g., `aur/PKGBUILD`) or in a separate repo?
2. Should we support aarch64 from day one, or start x86_64-only like most AUR -bin packages?
3. Should we automate PKGBUILD updates via CI on release, or keep it manual?
4. What email should be used for the AUR Maintainer line?
