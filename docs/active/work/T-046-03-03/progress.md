# T-046-03-03 Progress — xz-free-shell-installer

## Completed

### Step 0 — pinned local dist 0.30.4 (scratchpad, no commit)
Installed the standalone cargo-dist 0.30.4 into the session scratchpad
(`CARGO_DIST_FORCE_INSTALL_DIR`, `--no-modify-path`); `dist --version`
confirmed `cargo-dist 0.30.4`, matching the CI and checklist pin.

### Step 1 — archive format flip (`dist-workspace.toml`)
Added `unix-archive = ".tar.gz"` with a comment tying it to the fixture's
tar+gzip floor. Verified with the pinned dist:

- `dist plan --output-format=json`: all four `lisa-cli-<target>.tar.gz`
  archives + `.sha256` sidecars planned; jq assertion confirms **zero**
  `.tar.xz` artifacts anywhere in the plan.
- Generated the real installer locally (`dist build --artifacts=global`;
  extra-artifacts deb build temporarily disabled for the local run only —
  it requires a Linux amd64 host — config restored and diff-verified
  afterward). Inspection of the generated `lisa-cli-installer.sh`:
  - zero `.tar.xz` references; all archive cases are `.tar.gz`;
  - extraction is `tar xf` (compression auto-detect; gzip suffices);
  - download-URL overrides honored: `LISA_CLI_DOWNLOAD_URL`,
    `INSTALLER_DOWNLOAD_URL`, `LISA_CLI_INSTALLER_GITHUB_BASE_URL`;
  - downloader is plain `curl -sSfL` / wget fallback (no proto pinning, so
    a local-HTTP rehearsal is possible);
  - `verify_checksum` uses `sha256sum`, present in bookworm (coreutils);
    when generated without embedded checksums it says "no checksums to
    verify" — that exact string became the rehearsal's integrity assertion.

### Step 2 — `scripts/verify-musl-release.sh`
Archive name → `.tar.gz`; extraction now runs inside `debian:bookworm-slim`
(bind-mounted archive read-only, work dir read-write, `--user $(id -u)`),
asserting `xz` is absent in the container first so the proof can never go
vacuous. All downstream assertions (single static ELF, no INTERP, ldd,
embedded WASM magic, embedded runtime-manifest sha, bullseye execution)
unchanged. Locally dry-ran the container extraction leg against a repacked
real musl archive: extraction succeeds, files land host-owned, cleanup OK.

### Step 3 — `scripts/verify-shell-installer.sh` (new) + `release.yml`
New rehearsal script per structure.md: generation-time assertions (installer
offers both Linux `.tar.gz` archives, zero `.tar.xz` references; both
archives present in distrib), then a runtime rehearsal — `python:3-slim`
serving the distrib dir on a private docker network, `debian:bookworm-slim`
client that installs only ca-certificates+curl (the fixture floor), pipes
the installer from the local server through `sh` as an unprivileged
`tester` user with `INSTALLER_DOWNLOAD_URL` pointed at the server, then
asserts: no "no checksums to verify" in the log, `.local/bin` PATH guidance
present, forbidden binaries (xz/rustc/cargo/rustup/gcc/cc/g++/make/git)
absent, `~/.local/bin/lisa` executable and `--version` prints `lisa …`.
Added the one workflow step in `build-global-artifacts` between the global
build and the deb verification.

**Local end-to-end proof:** repacked the two real musl archives from stale
local `.tar.xz` builds into cargo-dist-shaped `.tar.gz` (root directory
included), injected their true sha256 values into a copy of the generated
installer (standing in for what CI embeds from the local-build manifests),
and ran `scripts/verify-shell-installer.sh` against that distrib dir. Full
pass: checksum verified, gzip extraction, install to `/home/tester/.local/
bin`, brand-voice success message, `lisa 0.4.0-rc.8` version output, exit 0.

### Step 4 — downstream consumers and docs
- `scripts/package-debs.sh`: consumes `lisa-cli-<target>.tar.gz`, `tar -xzf`.
- `aur/PKGBUILD`: both source lines → `.tar.gz` with an applicability note
  (gzip names begin at v0.4.0; template's pkgver must be bumped past it).
- `docs/knowledge/release-checklist.md`: 18 occurrences rewritten across the
  dist-plan assertion list, the released-asset assertion list, and the
  homebrew-formula grep checks; no other xz mentions remain.

## Verification summary
- `dist plan` (pinned 0.30.4): four `.tar.gz` archives, no `.tar.xz`.
- Full installer rehearsal passed locally in a no-xz bookworm container.
- No-xz container extraction leg of verify-musl-release.sh passed locally.
- `bash -n` + shellcheck clean on all three touched/added scripts.
- `release.yml` parses (ruby YAML).
- `cargo test --workspace`: 395 passed, 0 failed.
- Repo-wide `tar.xz` grep: remaining hits are docs archives, other tickets'
  work artifacts, this ticket's own comment/error-message strings only.

## Deviations from plan
- Plan's Step 3 anticipated the local rehearsal might only be partially
  achievable (option (b) stand-ins); it was in fact achieved fully (option
  "real archives + injected checksums"), because real static musl binaries
  existed in the local `target/distrib` from T-046-03-01 work.
- `verify-musl-release.sh` gained an absolute-path normalization for the
  archive (docker bind mounts require absolute sources) — mechanical,
  not in structure.md.

## Remaining
- Commits via `lisa commit-ticket` (3 units), then Review artifacts.
