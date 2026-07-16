# T-046-03-03 Research — xz-free-shell-installer

## Problem restated

The README one-command install pipes `lisa-cli-installer.sh` from the latest
GitHub release into `sh`. On the Chromebook baseline fixture (Debian bookworm,
deliberately without `xz`, Rust, compilers, or build tools) the installer
resolved the Linux archive as `lisa-cli-<arch>-unknown-linux-gnu.tar.xz` and
failed inside `tar`: `tar (child): xz: Cannot exec: No such file or directory`.
T-046-03-01 (done) made Linux artifacts static-musl and moved the install
location to `~/.local/bin`, but the archives — and therefore the generated
installer's extraction step — still use `.tar.xz`. Static linkage fixed target
glibc; it did not fix the *host decompressor* dependency during unpacking.

## Release pipeline as it exists

### cargo-dist configuration — `dist-workspace.toml`

- `cargo-dist-version = "0.30.4"`, `ci = "github"`, `installers = ["shell",
  "homebrew"]`, `packages = ["lisa-cli"]`.
- Targets after T-046-03-01: `x86_64-apple-darwin`, `aarch64-apple-darwin`,
  `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`. No gnu targets
  remain.
- `install-path = "~/.local/bin"`, brand-voice `install-success-msg` pointing
  at `lisa doctor`.
- `allow-dirty = ["ci"]` because `release.yml` carries hand-maintained
  extensions (auto-tag dispatch path, verification steps).
- **No `unix-archive` key is set**, so cargo-dist uses its default unix archive
  format, `.tar.xz`. This is the root of the bug. cargo-dist exposes the
  archive format directly as `unix-archive` (accepted values include
  `".tar.gz"`, `".tar.xz"`, `".tar.zstd"`, `".zip"`), and the generated shell
  installer adapts its extraction command to the configured format.
- `[[dist.extra-artifacts]]` builds four `.deb` files via
  `scripts/package-debs.sh`.

### The generated shell installer

`lisa-cli-installer.sh` is a **global artifact generated at release time** by
`dist build --artifacts=global` in `release.yml`; it is not checked into the
repo. It embeds per-artifact sha256 checksums (cargo-dist's built-in integrity
verification) and the artifact filenames, including their extension. Whatever
archive format the config selects flows into both the archive names and the
installer's `tar` invocation. Overriding the download base URL for testing is
possible via the installer's documented env override
(`LISA_CLI_INSTALLER_GITHUB_BASE_URL` / `INSTALLER_DOWNLOAD_URL`).

### Release CI — `.github/workflows/release.yml`

- `build-local-artifacts`: matrix computed by dist. Per T-046-03-01's review,
  Linux musl legs run natively on `ubuntu-22.04` and `ubuntu-22.04-arm`
  runners (no cross/qemu needed), with `musl-tools` installed and the WASM
  plugin pre-built via `.github/build-setup.yml` (which also verifies a
  non-empty WASM header and sets `LISA_REQUIRE_EMBEDDED_WASM=1`).
- After `dist build`, Linux legs run `scripts/verify-musl-release.sh <target>`
  (line 159–162): extracts `target/distrib/lisa-cli-$target.tar.xz` with
  `tar -xJf`, asserts exactly one static ELF `lisa` (file/readelf/ldd), asserts
  embedded WASM magic and the target's managed-runtime sha256 are present in
  the binary, then executes the binary in a `debian:bullseye-slim` container
  (`docker run` — Docker is available on these runners, natively per-arch).
- `build-global-artifacts` (ubuntu-22.04): downloads local artifacts, runs
  `dist build --artifacts=global` (produces the installer + checksum files),
  then `scripts/verify-deb-release.sh` and `scripts/verify-apt-repository.sh`.
- `host` uploads everything and creates the GitHub release; Homebrew formula
  and apt Pages publication follow.

## Every `.tar.xz` coupling point outside docs archives

| File | Line(s) | Coupling |
| --- | --- | --- |
| `dist-workspace.toml` | (absent) | default `unix-archive` = `.tar.xz` |
| `scripts/verify-musl-release.sh` | 19, 27 | archive name + `tar -xJf` |
| `scripts/package-debs.sh` | 66 | reads `lisa-cli-${rust_target}.tar.xz` from distrib dir to build debs |
| `aur/PKGBUILD` | 15–16 | release-asset URLs `lisa-cli-<target>-unknown-linux-musl.tar.xz` |
| `docs/knowledge/release-checklist.md` | ~155–163, ~310–317 | expected release asset name lists |

`README.md` never names the archives (it only shows the installer one-liner and
brew/apt paths). `auto-release.yml` and `ci.yml` have no archive references.
`crates/lisa-cli/src/runtime.rs` already handles the *zellij runtime* download
as `.tar.gz` in-process — unrelated to this ticket but confirms the project's
only xz dependence is the cargo-dist archive format. The Homebrew formula is
regenerated per release by dist with matching URLs/checksums, so a format
change propagates automatically (macOS `tar` handles any of the formats).

## The Chromebook fixture and its invariants

`docker/chromebook-test/Dockerfile`: `FROM debian:bookworm` + ca-certificates,
curl, procps, sudo, NodeSource Node 22, claude/codex CLIs, `tester` user with
passwordless sudo. The image build **fails if any of** `git rustc cargo rustup
xz gcc cc g++ make` is present — absence is a fixture invariant, re-asserted in
the `just emulate-debian` preflight and the runbook
(`docs/knowledge/chromebook-install-test.md`). Notably `debian:bookworm` base
ships `tar` and `gzip` (both Priority: required in Debian) but **not**
`xz-utils` — which is exactly why the probe failed and why gzip-based
extraction is inside the fixture's declared floor. Run records live under the
runbook's process; containers are kept as evidence.

## Constraints and assumptions surfaced

- **Verified-release property to preserve:** installer-embedded sha256
  verification (cargo-dist), the `.sha256` sidecar assets, and
  `verify-musl-release.sh`'s static-ELF + embedded-WASM + embedded
  runtime-manifest + bullseye-execution checks. The managed-runtime sha256
  constants in `verify-musl-release.sh` are content hashes of the *zellij
  runtime* manifest baked into the binary — independent of archive format.
- **Static-musl property to preserve:** targets stay musl; only packaging
  changes.
- **Fixture floor:** `curl`, `sh/bash`, `tar`, `gzip`, coreutils, sudo. No
  `unzip`, no `zstd`, no `xz`. Any solution must extract with only these.
- **Installer is generated, not authored:** we cannot patch the installer
  script itself without forking cargo-dist templates; we can only steer it
  through `dist-workspace.toml` config.
- **`allow-dirty = ["ci"]`** means dist tolerates our hand-edits to
  `release.yml`; adding verification steps there is an established pattern
  (musl verify, deb verify, apt verify all live there already).
- **AUR PKGBUILD** points at versioned release asset URLs; if the asset
  extension changes and PKGBUILD does not, the next AUR bump 404s.
- **Latest published release (v0.3.0 line) still serves `.tar.xz`**; the fix
  becomes user-visible only at the next release cut (T-046-03-02 prepared that
  checklist). Full AC1 (fresh fixture containers install via the exact README
  command) is provable end-to-end only against a published release; release CI
  can prove the equivalent pre-publication by exercising the generated
  installer/archives against locally hosted artifacts.
- Local machine has Docker 29.x and no `dist` binary; `dist` 0.30.4 can be
  installed standalone to run `dist plan` locally for config verification.

## Relevant prior work

- `docs/active/work/T-046-03-01/` — musl target/install-path design; its
  verifier (`verify-musl-release.sh`) is the natural extension point for
  archive-path checks ("It will locate exactly one matching
  `lisa-cli-<target>.tar.*` archive" per its design).
- T-046-03-02 (done) — release checklist + pipeline verification for the
  stable v0.4.0 cut; its checklist asset lists are among the files that name
  `.tar.xz`.
- Ticket context records the real probe: agent recovered only via Python
  `lzma` improvisation — the failure is at extraction, before any binary runs.
