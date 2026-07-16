# T-046-03-01 Progress

## Status

Implementation is complete. Review is in progress.

## Completed phase work

- Read `CLAUDE.md`, the ticket, assignment, and complete RDSPI workflow.
- Completed `research.md`, `design.md`, `structure.md`, and `plan.md`.
- Confirmed every planned ticket-owned source path was clean before editing.
- Recorded implementation baseline HEAD `ce8aedb51d33b653d1d4c05d35a763688c641adf`.
- Retrieved and ran the official cargo-dist 0.30.4 macOS binary.
- Confirmed the pre-change plan contains GNU Linux targets on native Ubuntu runners.
- Confirmed the existing release WASM is non-empty with header `0061736d01000000`.

## Current unit

Final review and disposition.

## Implementation units

### Static musl release and local-bin installer

- Replaced both GNU Linux targets with their static-musl equivalents.
- Changed the shell install path from `CARGO_HOME` to `~/.local/bin`.
- Added the success message: `Lisa is ready in ~/.local/bin. Open a new shell,
  then run lisa doctor.`
- Updated both AUR source URLs to musl archive names.
- Removed the AUR package's direct `gcc-libs` dependency.
- `dist 0.30.4 plan --output-format=json`: pass.
- Plan contains both musl archives and no GNU Linux archive.
- Plan retains both Darwin archives, shell installer, and Homebrew formula.
- The native musl jobs install `musl-tools` on `ubuntu-22.04` and
  `ubuntu-22.04-arm` respectively.
- `dist build --artifacts=global --output-format=json`: pass.
- Generated installer inspection confirms flat `$HOME/.local/bin` layout, Lisa's
  success copy, and exact restart/source PATH guidance.
- Committed as `e4886c6` (`Ship static musl Linux artifacts`) through
  `lisa commit-ticket` with exactly `dist-workspace.toml` and `aur/PKGBUILD`.

### Release WASM and musl artifact verification

- Hardened `crates/lisa-cli/build.rs` to validate the non-empty WASM header.
- Preserved the absent-WASM placeholder for ordinary developer builds.
- Added `LISA_REQUIRE_EMBEDDED_WASM=1` enforcement for release CI.
- Extended `.github/build-setup.yml` to verify the release WASM and export that
  release-only requirement to the following dist build.
- Added `scripts/verify-musl-release.sh` for static ELF, interpreter, `ldd`,
  embedded WASM, managed checksum, and bullseye execution checks.
- Wired the verifier after `dist build` and before artifact upload for musl jobs.
- `bash -n scripts/verify-musl-release.sh`: pass.
- Unsupported-target negative check: pass with exit 2 and named usage.
- Both modified YAML files parse with Ruby/Psych.
- The build-setup shell guard passes against the current release WASM.
- `LISA_REQUIRE_EMBEDDED_WASM=1 cargo check -p lisa-cli`: pass.
- Started Docker Desktop to obtain a native Linux verification environment.
- Native arm64 `ubuntu:22.04` installed the exact plan-selected `musl-tools`.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`: pass in fixture.
- `LISA_REQUIRE_EMBEDDED_WASM=1 cargo build -p lisa-cli --profile dist
  --target aarch64-unknown-linux-musl`: pass; fat LTO completed in 29.85s.
- `file`: ARM aarch64 ELF, statically linked.
- `readelf -l`: no `INTERP` segment.
- Ubuntu `ldd`: `not a dynamic executable`.
- Packaged binary contains the WASM header and arm64 managed-runtime checksum.
- Debian `bullseye-slim` reports glibc 2.31 and `ldd` reports `not a dynamic
  executable`.
- The same binary runs there and prints `lisa 0.4.0-rc.8`.
- Bullseye image digest:
  `debian@sha256:cba95a21c96c1f5fc2470081829363eed57706634f7dc26e8c6712934303d57a`.
- Committed as `fcdd293` (`Verify portable Linux release artifacts`) through
  `lisa commit-ticket` with exactly the four planned CI/source paths.

### Managed-runtime checksum manifest

- Independently downloaded and hashed all four official Zellij 0.43.1 no-web
  archives rather than using mutable or runtime-fetched checksum material.
- Confirmed the concurrent fetch ticket verifies the compressed archive before
  extraction, so the manifest carries tarball hashes rather than upstream's
  separately published unpacked-binary hashes.
- Added `crates/lisa-cli/data/managed-runtime-sha256.json` with version, target,
  archive, immutable URL, and SHA-256 for both Darwin and both musl Linux targets.
- Added `MANAGED_RUNTIME_SHA256_MANIFEST` through `include_str!`.
- Replaced T-046-02-02's duplicated Linux URL/hash constants with a typed,
  lazily parsed view of the embedded manifest.
- Preserved T-046-02-02's explicit Linux-only acquisition boundary while baking
  Darwin release inputs for its eventual platform extension.
- Manifest test validates pinned-version equality, exact four-target coverage,
  uniqueness, archive/target correspondence, immutable URL derivation, and
  lowercase 64-digit hashes.
- Linux managed-release lookup is asserted to return the manifest's exact values.
- `cargo fmt --all -- --check`: pass.
- Focused manifest test: pass.
- All 15 runtime tests, including fetch/checksum/atomic-store fixtures: pass.
- `git diff --check`: pass.
- Committed as `f213b3f` (`Bake managed runtime checksums`) through
  `lisa commit-ticket` with exactly the manifest and runtime module paths.

### Cross-platform manifest retention

- Added one OS-neutral manifest reference to the existing hidden `Version` arm.
- The reference uses `std::hint::black_box` and does not change command output.
- Rebuilt the Darwin CLI with the unchanged fat-LTO dist profile.
- Binary inspection now finds the full WASM header and all four archive hashes.
- `target/dist/lisa version` remains exactly `lisa 0.4.0-rc.8`.
- `cargo test -p lisa-cli`: pass (14 library, 301 binary, 17 integration;
  one explicitly ignored live-Zellij boundary).
- `cargo clippy -p lisa-cli --all-targets -- -D warnings`: pass.
- `cargo fmt --all -- --check` and `git diff --check`: pass.
- Committed as `5457d06` (`Retain runtime manifest across release targets`)
  through `lisa commit-ticket` with exactly `crates/lisa-cli/src/main.rs`.

### Final verification

- `cargo fmt --all -- --check`: pass.
- `cargo test --workspace`: pass across the CLI, core, and plugin crates and
  integration/doc tests; one explicitly ignored live-Zellij integration remains.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- Both workflow YAML files parse with Ruby/Psych.
- `bash -n scripts/verify-musl-release.sh`: pass.
- `git diff --check`: pass.
- Native arm64 Ubuntu 22.04 fat-LTO build and Bullseye execution: pass.
- x86_64 Ubuntu 22.04 fat-LTO build under Docker's amd64 environment: pass.
- x86_64 `file`: x86-64 static PIE ELF.
- x86_64 `readelf -l`: no `INTERP` segment.
- x86_64 Ubuntu `ldd`: not dynamically linked.
- x86_64 binary contains the WASM header and managed-runtime checksum.
- x86_64 binary runs in Debian Bullseye, where glibc reports 2.31, `ldd`
  reports `statically linked`, and `lisa --version` prints `lisa 0.4.0-rc.8`.
- Both Bullseye checks used image digest
  `debian@sha256:cba95a21c96c1f5fc2470081829363eed57706634f7dc26e8c6712934303d57a`.

## Remaining

- Complete Review artifacts and final source audit.

## Deviations

`runtime.rs` and `Cargo.lock` became concurrently modified by T-046-02-02 after
the clean ownership baseline. My manifest constant/test additions were removed
from that shared file without disturbing the other ticket's edits. After that
ticket committed its isolated unit, this ticket integrated only its own manifest
consumer changes and never committed foreign work.

Final binary inspection found a second necessary adjustment: on Darwin, fat LTO
proved capable of removing the Linux-only managed acquisition path and therefore
the embedded manifest bytes. The plan was extended by one narrow `main.rs` change:
the existing hidden `lisa version` arm passes the manifest to `black_box`, retaining
the release data on every OS without changing output or the public command list.
