# T-046-05-01 Progress

## Phase completion

Research completed.

The codebase map covers cargo-dist, runtime acquisition, resolver tests, doctor,
nFPM, package metadata, and the release workflow artifact graph.

Design completed.

The selected design uses cargo-dist global extra artifacts, two nFPM configs,
a pinned builder, a bookworm verification gate, and Packaged runtime provenance.

Structure completed.

The blueprint identifies eight ticket-owned source paths and their data flow.

Plan completed.

Implementation is split into a resolver/doctor commit and a Debian release commit.

## Completed: packaged-runtime resolver

Added `/usr/libexec/lisa/zellij` as the production companion-package location.

Added `ZellijRuntimeMode::Packaged`, rendered as `packaged`.

Managed intent now chooses an executable companion runtime before consulting the
versioned managed cache or attempting a download.

System and Pinned remain explicit choices and retain their existing branches.

The selected packaged executable still passes canonicalization, version parsing,
supported-range enforcement, and named errors.

The resolver test creates packaged, managed-cache, and PATH candidates and proves
that Packaged wins only for Managed intent.

The doctor mode-table test now proves `mode packaged`, the supported version, and
the exact `/usr/libexec/lisa/zellij` path are reported.

Focused resolver tests: 16 passed.

Full `cargo test -p lisa-cli`: 340 tests passed, one existing live test ignored.

## Completed: Debian package definitions and assembly

Added separate nFPM definitions for `lisa` and `lisa-runtime-zellij`.

The CLI package installs `/usr/bin/lisa`, Depends on Git, and Recommends the
companion package.

The runtime package installs `/usr/libexec/lisa/zellij` and does not claim a
generic PATH-level Zellij package.

The builder consumes both cargo-dist Linux archives and both matching records in
the existing managed-runtime manifest.

It pins and authenticates nFPM 2.47.0, verifies the runtime archive SHA-256 values,
and emits four fixed extra-artifact basenames.

nFPM required `expand: true` on `contents` entries before it would expand staged
binary paths. The first fixture run exposed this and both configs now declare it.

The successful package metadata is:

- package version `0.4.0~rc.8-1` on both architectures;
- `lisa` architectures `amd64` and `arm64`;
- runtime architectures `amd64` and `arm64`;
- CLI Depends `git`;
- CLI Recommends `lisa-runtime-zellij`;
- executable mode 0755 at both required destinations.

## Completed: clean-room acceptance verifier

Added structural checks for all four packages using `dpkg-deb`.

Added a Debian bookworm-slim container install of the amd64 pair through apt.

The verifier supplies only a controlled Claude stub because agent installation is
outside the Debian package pair.

It disconnects the container's only Docker network before running doctor.

The real package-flow test passed with doctor exit zero.

Doctor reported:

`mode packaged, version 0.43.1, supported >= 0.43.0, path /usr/libexec/lisa/zellij`.

It also reported Git, Claude, and embedded WASM OK and all dependencies satisfied.

The packaged runtime independently printed `zellij 0.43.1` without a network.

## Completed: cargo-dist and release workflow wiring

Registered all four packages as cargo-dist 0.30.4 global extra artifacts.

The pinned `dist plan` succeeds and lists 17 total release artifacts, including
the four `.deb` files, four native archives, shell installer, and Homebrew formula.

The native build matrix remains four rows.

Added the bookworm verifier after global build and before post-build upload paths.

The workflow parses as YAML.

## Plan deviation

The Design mentioned a Darwin arm64 nFPM bootstrap as a local convenience.

Implementation deliberately supports only the actual cargo-dist global host,
Linux x86_64. This keeps the checksum/tool branch smaller and ensures the script is
exercised under exactly the release environment. Local acceptance was reproduced
inside an amd64 Linux container instead.

The local package-flow fixture used a fresh Linux amd64 release build containing
the current resolver and embedded WASM. It used an existing real arm64 Linux archive
for structural cross-architecture packaging. Release CI continues to supply and
independently statically verify both real musl inputs.

## Completed: workspace verification

`cargo fmt --all -- --check` passed.

`cargo test -p lisa-cli` passed.

`just check` passed, including the WASM target check and complete workspace tests.

The workspace results included 19 CLI-library tests, 307 CLI-binary tests, 207
core tests, 395 plugin tests, and every non-ignored integration test.

The existing real-Zellij delivery boundary remained ignored by its normal explicit
environment gate.

Both shell scripts pass Bash syntax checks and ShellCheck.

The final workflow parses as YAML and `git diff --check` passed before commit.

The pinned cargo-dist 0.30.4 final plan still includes all four `.deb` files.

## Completed: isolated source commits

Resolver and doctor commit:

`676d520ec3e8264eacc8582432a120d480e20068` — `Prefer packaged Zellij runtime`.

It contains exactly:

- `crates/lisa-cli/src/runtime.rs`;
- `crates/lisa-cli/src/doctor.rs`.

Debian release commit:

`e900e18998b8cf278ee482752d5bac72cba5041e` — `Package Debian release artifacts`.

It contains exactly:

- `.github/workflows/release.yml`;
- `dist-workspace.toml`;
- `packaging/nfpm/lisa.yaml`;
- `packaging/nfpm/lisa-runtime-zellij.yaml`;
- `scripts/package-debs.sh`;
- `scripts/verify-deb-release.sh`.

Both commits were made through `lisa commit-ticket` with exact include paths.

No ordinary index staging or ordinary commit was used.

All eight ticket-owned source paths are clean after commit.

The ordinary worktree still contains only Lisa-managed/concurrent ticket files,
including provenance, completion journal, PM documents, and T-046-06 work.

## Implementation complete

All planned ticket-owned source work is implemented, verified, and committed.

Remaining work is the Review artifact and exact review disposition only.

## Repository safety

The initial worktree contained Lisa-managed provenance changes and untracked PM files.

Those paths are not owned by this ticket and will remain untouched.

All workflow artifacts are being written only under the current attempt work directory.

No ordinary Git index command has been used.

No source commit has been made yet.
