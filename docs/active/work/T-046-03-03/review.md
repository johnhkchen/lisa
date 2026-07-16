# T-046-03-03 Review — xz-free-shell-installer

## What changed

Three commits, all via `lisa commit-ticket`:

1. **`ab9ecba` — Serve gzip release archives so install needs no xz**
   - `dist-workspace.toml`: `unix-archive = ".tar.gz"` (+ comment tying it
     to the Chromebook fixture's tar+gzip floor). This single key renames
     all four unix archives, their `.sha256` sidecars, the generated
     installer's archive table, and the Homebrew formula URLs at release
     time. Root cause addressed: the installer's *host decompressor*
     dependency, which static-musl (T-046-03-01) had not touched.

2. **`9f132bb` — Verify no-xz extraction and installer rehearsal in release CI**
   - `scripts/verify-musl-release.sh` (modified): expects
     `lisa-cli-<target>.tar.gz`; extraction now happens inside a
     `debian:bookworm-slim` container that first asserts `xz` is absent
     (with a distinct "proof is vacuous" failure if the image ever ships
     xz). Runs per Linux target on the native amd64/arm64 release runners —
     both Linux archive paths proven xz-free. All prior assertions (single
     static ELF, no INTERP, ldd, embedded non-empty WASM, embedded
     managed-runtime sha256, bullseye execution) preserved verbatim.
   - `scripts/verify-shell-installer.sh` (new): rehearses the *generated*
     installer end-to-end. Static checks: installer offers both Linux
     `.tar.gz` archives and contains zero `.tar.xz` references. Runtime:
     private docker network, `python:3-slim` artifact server,
     `debian:bookworm-slim` client with only ca-certificates+curl added
     (the fixture's declared floor), installer piped from the server
     through `sh` as an unprivileged user with `INSTALLER_DOWNLOAD_URL`
     overridden. Asserts exit 0, checksum verification actually ran (fails
     on cargo-dist's "no checksums to verify"), `~/.local/bin/lisa`
     executable, `lisa --version` output, `.local/bin` PATH guidance, and
     xz/rustc/cargo/rustup/gcc/cc/g++/make/git all absent.
   - `.github/workflows/release.yml`: one step in `build-global-artifacts`
     running the rehearsal right after the installer is generated
     (`allow-dirty = ["ci"]` already covers this file).

3. **`d952205` — Point deb packaging, AUR, and release checklist at gzip archives**
   - `scripts/package-debs.sh`: consumes `.tar.gz` (`tar -xzf`).
   - `aur/PKGBUILD`: source URLs → `.tar.gz`, with a note that gzip names
     start at v0.4.0 (template's `pkgver=0.1.6` is pre-existing staleness).
   - `docs/knowledge/release-checklist.md`: all 18 asset-name occurrences
     across the dist-plan assertion, released-asset assertion, and
     homebrew-formula greps.

Deliberately untouched: fixture Dockerfile/justfile/runbook (xz absence
stays an invariant), README (the one-liner is unchanged and now true),
`docs/archive/**`, all Rust code.

## Test coverage and evidence

- **Pinned dist 0.30.4 locally**: `dist plan` shows exactly the four
  `.tar.gz` archives + sidecars, zero `.tar.xz` artifacts.
- **Real installer generated locally and inspected**: gzip-only archive
  table, `tar xf` extraction, `curl -sSfL` downloader, sha256
  `verify_checksum`, documented URL overrides.
- **Full rehearsal executed locally** (Docker): real static musl binaries
  (from T-046-03-01-era local builds) repacked as cargo-dist-shaped
  `.tar.gz` with true sha256 values injected into the installer copy;
  `scripts/verify-shell-installer.sh` passed end-to-end — download,
  checksum verification, gzip extraction, install to `~/.local/bin`,
  brand-voice success message, `lisa 0.4.0-rc.8` runs, forbidden binaries
  absent.
- **No-xz extraction leg** of `verify-musl-release.sh` dry-ran locally
  against a real musl archive: extracts cleanly, host-owned files, clean
  teardown.
- `bash -n` + shellcheck clean on all three scripts; `release.yml` parses;
  `cargo test --workspace` 395/395 green (no Rust changes; collateral guard).
- Repo-wide `tar.xz` sweep: remaining mentions are historical archives,
  other tickets' artifacts, and this ticket's own comment/error strings.

## Acceptance criteria assessment

- **AC1 (fixture install, both arches, exact README command)**: mechanics
  fully proven — the archives are gzip, the fixture floor (tar+gzip+curl)
  suffices, and the rehearsal ran the real installer to a successful
  install in a no-xz bookworm container. The *literal* criterion (fresh
  fixture containers against a published release) is observable only after
  the next release cut, since `releases/latest` still serves the old
  `.tar.xz` assets until then. T-046-03-02's checklist + chromebook runbook
  cover that post-cut probe. This is the same honest boundary S-046-03
  declared for the stable cut itself.
- **AC2 (no xz/Rust/compiler added or invoked; CI verifies both Linux
  archive paths; checksum + static-musl preserved)**: met structurally —
  per-target no-xz extraction proof on both native Linux runners,
  installer rehearsal with forbidden-binary assertions, checksum
  verification asserted at runtime (the rehearsal *fails* if the installer
  reports nothing to verify), and the static-musl/WASM/runtime-manifest
  checks retained unchanged.

## Open concerns

- **First real exercise is the next release run.** The new CI steps have
  not run on a GitHub runner yet; they were exercised locally with Docker
  and mirror the conventions of the existing verify scripts (which run
  there today). Risk is low but nonzero (e.g., image pull limits).
- **`python:3-slim` becomes a release-CI dependency** for the artifact
  server. Pinning to a digest was considered and skipped to match the
  repo's existing unpinned `debian:*` usage; worth revisiting if release
  runs ever flake on pulls.
- **AUR template remains stale by design** (`pkgver=0.1.6`, SKIP
  checksums, pre-existing TODO); it now carries a note that gzip names
  begin at v0.4.0 so a future bump cannot silently 404.
- **Archive size grows** (gzip vs xz, roughly a few hundred KB per
  artifact at current binary sizes) — accepted in design; GitHub Releases
  bandwidth, not the Pages budget.
- Reminder for the v0.4.0 cut: the updated checklist assertions now
  require `.tar.gz` assets, so cutting from a commit *before* this change
  would fail the checklist — correct behavior, just worth knowing.
