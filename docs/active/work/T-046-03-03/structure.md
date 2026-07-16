# T-046-03-03 Structure — xz-free-shell-installer

## Change inventory

No files are deleted. One file is created; six are modified. No Rust code
changes — this ticket is packaging config, verification scripts, release CI,
and release documentation.

### Modified: `dist-workspace.toml`

Add one key to the existing `[dist]` table, adjacent to `targets`:

```toml
# The Chromebook fixture's floor is tar+gzip (Debian Priority: required).
# xz is deliberately absent there; .tar.gz keeps the one-command install
# inside the prerequisites the README actually declares (T-046-03-03).
unix-archive = ".tar.gz"
```

Everything else (targets, installers, install-path, extra-artifacts) is
untouched. This single key changes the four unix archive names to
`lisa-cli-<target>.tar.gz`, their `.sha256` sidecars, the generated
installer's extraction command, and the Homebrew formula URLs — all at
release-build time.

### Modified: `scripts/verify-musl-release.sh`

Same contract (arg: musl target; env: `LISA_DISTRIB_DIR`), same check order,
two changes:

1. `archive="$distrib_dir/lisa-cli-$target.tar.gz"` (was `.tar.xz`).
2. Extraction moves from host `tar -xJf` into a no-xz proof container:
   `docker run` `debian:bookworm-slim` mounting the archive read-only and an
   empty work dir read-write; inside, assert `! command -v xz` (fail loudly if
   the image ever starts shipping xz, since the check would then prove
   nothing), then `tar -xzf`. Extraction output lands in the mounted host work
   dir, and all downstream checks (single `lisa` binary, `file` static ELF,
   `readelf` no-INTERP, `ldd`, embedded WASM magic + runtime-manifest sha via
   python, bullseye `--version` execution in docker) continue on the host
   exactly as today.

Docker is already a hard dependency of this script (bullseye execution step),
so the new container step adds no new runner requirement, and the script's
per-target invocation from `release.yml` line 159–162 is unchanged — both
Linux archive paths get the no-xz extraction proof natively (amd64 and arm64
runners).

### Created: `scripts/verify-shell-installer.sh`

New verification script, same conventions as its siblings (`#!/usr/bin/env
bash`, `set -euo pipefail`, loud failures, final `echo "verified ..."`).

Responsibilities, in order:

1. Locate `lisa-cli-installer.sh` and the linux-musl `.tar.gz` +
   `.tar.gz.sha256` artifacts in `${LISA_DISTRIB_DIR:-target/distrib}`; fail
   if missing.
2. Static guard: `grep` the generated installer to assert it references
   `.tar.gz` artifacts and contains no `.tar.xz` reference — catches a silent
   cargo-dist default regression at generation time, independent of runtime.
3. Runtime rehearsal (host-arch leg, amd64 on the global CI job):
   - `docker run debian:bookworm-slim` with `target/distrib` mounted
     read-only at a known path.
   - Inside the container: assert `xz`, `rustc`, `cargo`, `gcc`, `cc`, `make`
     are absent (fixture-invariant mirror); install nothing; start
     `python3 -m http.server` — **not available in bookworm-slim**, so
     instead serve without python: run the installer with the artifact
     directory exposed via a `file://`-style local URL is unsupported by
     curl-pipe installers, therefore: run a second sibling container
     (`python:3-slim`) as the HTTP server on a shared docker network, and the
     bookworm container fetches from it. Both containers, one bridge network,
     torn down on exit.
   - Execute `lisa-cli-installer.sh` as a non-root user with `HOME` set, with
     the installer's artifact-base-URL override env var (name confirmed from
     the generated installer during implementation; cargo-dist supports
     overriding the download base for exactly this kind of rehearsal)
     pointed at the sibling server.
   - Assert: exit 0; `"$HOME/.local/bin/lisa"` exists and is executable;
     `"$HOME/.local/bin/lisa" --version` prints `lisa `; installer stdout
     mentions `~/.local/bin` (PATH guidance accurate).

Arg-less; the host arch selects which musl artifact the installer picks —
by design, since the installer's own platform detection is part of what is
being rehearsed.

### Modified: `.github/workflows/release.yml`

One added step in `build-global-artifacts`, after `Build global artifacts`
and before `Verify Debian packages on Debian bookworm` (keeping the
installer check adjacent to the artifact build that produces it):

```yaml
- name: Verify shell installer on no-xz Debian bookworm
  shell: bash
  run: scripts/verify-shell-installer.sh
```

No other workflow edits. `allow-dirty = ["ci"]` already covers this file.

### Modified: `scripts/package-debs.sh`

Line 66: `lisa_archive="$distrib_dir/lisa-cli-${rust_target}.tar.gz"`; the
matching extraction (`tar -xJf` → `tar -xzf`) a few lines below. Nothing else
— nfpm download handling is already `.tar.gz`.

### Modified: `aur/PKGBUILD`

`source_x86_64`/`source_aarch64`: local-name and remote-asset extensions
`.tar.xz` → `.tar.gz`, with a one-line comment that gzip asset names apply to
releases after the format change (the pinned `pkgver=0.1.6` is already stale
with SKIP checksums; the file is a template the maintainer bumps at release).

### Modified: `docs/knowledge/release-checklist.md`

Mechanical rename in the two expected-asset lists (dist-plan assertion block
~line 155, released-asset assertion block ~line 310): every
`lisa-cli-*.tar.xz[.sha256]` → `.tar.gz[.sha256]`. `source.tar.gz` entries
are GitHub's source archives and stay as-is. Prose around the lists is checked
for any "xz" mention while editing.

## Boundaries and ordering

- **Interface stability**: `verify-musl-release.sh <target>` and
  `package-debs.sh` keep their CLI/env contracts; `release.yml` calls them
  unchanged (musl verify) or not at all directly (package-debs runs via
  dist extra-artifacts).
- **Ownership**: all seven paths above are owned by this ticket. Concurrent
  tickets (T-046-05-02, T-046-06-02 in flight) own other areas; none of the
  git-status-dirty files overlap this set.
- **Order of implementation**:
  1. `dist-workspace.toml` (the behavior change) — then verify locally with a
     pinned dist 0.30.4: `dist plan` shows `.tar.gz` names; `dist build
     --artifacts=global` (or host-target build) yields an installer whose
     extraction/override behavior the rehearsal script is written against.
  2. Verification scripts + `release.yml` step (the proof).
  3. Consumers and docs (`package-debs.sh`, `aur/PKGBUILD`,
     `release-checklist.md`).
- **Commit units** (via `lisa commit-ticket`): (1) config flip, (2)
  verification layer [verify-musl-release.sh, verify-shell-installer.sh,
  release.yml], (3) consumers/docs [package-debs.sh, PKGBUILD, checklist].

## What is deliberately not changed

- `docker/chromebook-test/Dockerfile`, `justfile`, runbook invariants — xz
  absence remains a fixture invariant; nothing new is required in the image.
- `README.md` — the one-liner is already correct; the fix makes it true.
- `docs/archive/**` — historical records keep their `.tar.xz` mentions.
- `crates/**` — no runtime behavior involved.
- Homebrew tap, apt pipeline — consume regenerated metadata; no hardcoded
  archive names exist there (verified in research).
