# T-046-03-03 Plan — xz-free-shell-installer

## Verification tooling first (no commit)

**Step 0 — pin a local dist 0.30.4.**
Install cargo-dist 0.30.4 into the session scratchpad (standalone installer,
`--no-modify-path`, custom home) so nothing leaks into `~`. Confirm
`dist --version` = `cargo-dist 0.30.4`. This is the same pin the release
checklist uses. Purpose: validate config behavior locally instead of
discovering it on a release tag.

## Step 1 — flip the archive format

Edit `dist-workspace.toml`: add `unix-archive = ".tar.gz"` with the
fixture-floor comment.

Verify:
- `dist plan --output-format=json` succeeds; assert all four
  `lisa-cli-<target>.tar.gz` artifacts (plus `.sha256` sidecars) and **no**
  `.tar.xz` artifact anywhere in the plan.
- Generate the real installer locally (`dist build --artifacts=global`, tag
  implied from Cargo version) and inspect it:
  - extraction path uses gzip (`tar` invocation / `_unpack` logic);
  - no `xz` reference;
  - confirm the exact env var the installer honors for overriding the
    artifact download base URL (expected: `LISA_CLI_INSTALLER_GITHUB_BASE_URL`
    and/or `INSTALLER_DOWNLOAD_URL`) — the rehearsal script uses what the
    script actually contains;
  - confirm embedded sha256 checksum verification is present for the
    `.tar.gz` artifacts (verified-release property).
- Note: local `dist build` on macOS builds host-target archives only; that is
  enough to inspect installer logic. Linux archives are exercised in Step 2's
  container test using a locally cross-checkable path: run `dist build
  --artifacts=global` output installer against **locally fabricated**
  distrib contents only if real ones are unavailable — prefer building the
  actual linux-musl archive via the existing local toolchain only if cheap;
  otherwise the docker rehearsal in Step 3 runs in CI where real artifacts
  exist. (Decision point recorded in progress.md during implementation.)

Commit 1 (`lisa commit-ticket`): `dist-workspace.toml`.

## Step 2 — verify-musl-release.sh: gz + no-xz extraction proof

Edit `scripts/verify-musl-release.sh`:
- archive name → `.tar.gz`;
- replace host `tar -xJf` with extraction inside `debian:bookworm-slim`:
  mount archive read-only + work dir read-write; inside: assert
  `! command -v xz`, then `tar -xzf`. Fail with a distinct message if the
  no-xz assertion itself fails (container drift).
- Keep all downstream assertions byte-identical.

Verify locally:
- `bash -n` + shellcheck (if available) on the script.
- Dry-run the docker extraction leg with a stand-in `.tar.gz` (a locally
  created archive containing a `lisa` file) to prove the mount/extract
  mechanics; the full script needs a real musl artifact and runs in release
  CI (it already did — bullseye execution has the same property).

## Step 3 — new scripts/verify-shell-installer.sh + release.yml step

Write `scripts/verify-shell-installer.sh` per structure.md:
1. locate installer + linux `.tar.gz`/`.sha256` in `LISA_DISTRIB_DIR`;
2. static assertions on the installer text (has `.tar.gz`, lacks `.tar.xz`);
3. runtime rehearsal: docker bridge network; `python:3-slim` container
   serving the distrib dir via `http.server`; `debian:bookworm-slim` client
   container asserting xz/rustc/cargo/gcc/cc/make absent, creating an
   unprivileged user, running the installer with the base-URL override from
   Step 1's inspection; assert exit 0, `~/.local/bin/lisa` executable,
   `--version` output starts with `lisa `, stdout mentions `.local/bin`.
4. Cleanup trap for containers/network regardless of outcome.

Edit `.github/workflows/release.yml`: insert the one verification step in
`build-global-artifacts` between the global build and the deb verification.

Verify locally:
- `bash -n`/shellcheck both scripts.
- Run the rehearsal end-to-end on this machine (Docker available): needs a
  distrib dir containing a real installer (from Step 1's local `dist build`)
  plus a linux-musl `.tar.gz` whose checksum matches what the installer
  embeds. If the local global build embeds only mac checksums (no linux
  archives present locally), fabricate the rehearsal fixture instead: build
  the linux archive is not possible without cross toolchain — in that case
  validate the script's mechanics with the mac... mac archive cannot run in
  a linux container. Fallback validation: run rehearsal against a locally
  built linux-musl archive **iff** `cargo` + musl target cross-compile
  cleanly via cargo-zigbuild/etc is NOT to be added — instead accept:
  (a) script syntax + static checks proven locally,
  (b) container mechanics proven with a stand-in tarball and a stand-in
      "installer" invocation replaced by direct download+extract, and
  (c) the real end-to-end run happens in release CI where genuine linux
      artifacts and the genuine installer coexist (that is where the check
      lives permanently).
  Record which of these was achieved in progress.md.
- `yamllint`-equivalent sanity: workflow YAML parses (python yaml or actionlint
  if available).

Commit 2: `scripts/verify-musl-release.sh`,
`scripts/verify-shell-installer.sh`, `.github/workflows/release.yml`.

## Step 4 — downstream consumers and docs

- `scripts/package-debs.sh`: `.tar.xz` → `.tar.gz`, `tar -xJf` → `tar -xzf`.
- `aur/PKGBUILD`: both source lines → `.tar.gz` names + applicability note.
- `docs/knowledge/release-checklist.md`: both asset lists → `.tar.gz`;
  scan for other xz mentions.

Verify:
- `bash -n` package-debs.sh; grep the repo (excluding `docs/archive`,
  `docs/active/work`, `target`) for remaining `tar.xz` — expect zero hits.
- `cargo test --workspace` and `just check` as a repo-health smoke (no Rust
  changes, so this guards against accidental collateral only).

Commit 3: `scripts/package-debs.sh`, `aur/PKGBUILD`,
`docs/knowledge/release-checklist.md`.

## Testing strategy summary

| Property | Where proven |
| --- | --- |
| Config produces `.tar.gz` (all 4 targets, no xz) | local `dist plan` (Step 1); checklist assertion block (Step 4) |
| Installer extracts with gzip, verifies checksums, honors override | local installer inspection (Step 1) |
| Linux archives extract with no xz on host, both arches | `verify-musl-release.sh` in per-arch release CI (Step 2) |
| Generated installer end-to-end on no-xz bookworm | `verify-shell-installer.sh` in release CI (Step 3); local run if artifacts allow |
| Static-musl + embedded WASM + runtime manifest preserved | unchanged assertions in `verify-musl-release.sh` |
| deb pipeline consumes new format | `package-debs.sh` (runs in release CI extra-artifacts + verify-deb-release.sh) |
| Fixture invariants untouched | no fixture edits; `just emulate-debian` preflight unchanged |

## Out-of-band residue

AC1's literal wording (exact README command in fresh fixture containers)
is fully observable only against a *published* release containing this
change; T-046-03-02's checklist + the chromebook runbook cover that at the
v0.4.0 cut. This ticket makes release CI prove the same mechanics
pre-publication. Called out in review.md as the honest boundary.

## Rollback

Single-property revert: removing `unix-archive` restores `.tar.xz`; each
commit is independent and revertible without touching the others' areas.
