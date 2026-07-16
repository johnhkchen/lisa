# Progress — T-046-02-02 pinned fetch, verify, and store

## Phase completion

- Research completed in `research.md`.
- Design completed in `design.md`.
- Structure completed in `structure.md`.
- Plan completed in `plan.md`.
- Implement started.

## Confirmed release inputs

- Managed Zellij version: 0.43.1.
- x86_64 Linux asset: `zellij-no-web-x86_64-unknown-linux-musl.tar.gz`.
- x86_64 SHA-256:
  `bac0728945e8f5a28f2647e2b9b0cfe4591d71abfe227336b1318937241f071d`.
- aarch64 Linux asset: `zellij-no-web-aarch64-unknown-linux-musl.tar.gz`.
- aarch64 SHA-256:
  `8ced877df27a8fe9112607dd3d772442aefa5e42359cda1baba53e78c4ae46aa`.
- The inspected upstream x86_64 archive contains one top-level `zellij` entry.
- A direct local digest calculation matched GitHub's release metadata.

## Implementation checklist

- [x] Add direct HTTP, SHA-256, gzip, and tar dependencies to lisa-cli.
- [x] Resolve and inspect the lockfile.
- [x] Add the compiled platform release table.
- [x] Add cache-first acquisition.
- [x] Add bounded single-request download.
- [x] Add checksum verification.
- [x] Add constrained extraction.
- [x] Add atomic directory publication and cleanup.
- [x] Integrate managed runtime resolution.
- [x] Add local fixture-server acceptance tests.
- [x] Run focused tests.
- [x] Commit ticket-owned source through `lisa commit-ticket`.
- [x] Run broad regression gates.
- [x] Complete Review artifacts.

## Deviations

None so far.

The implementation remains within the three planned source paths:
`crates/lisa-cli/Cargo.toml`, `Cargo.lock`, and
`crates/lisa-cli/src/runtime.rs`.

## Dependency implementation

Added Ureq 2.12 with Rustls TLS and without automatic response decompression.

Added direct SHA-2, Flate2, and Tar dependencies.

Tar default features are disabled, so no xattr dependency was added.

Dependency inspection found no native-tls or OpenSSL dependency.

Cargo check completed successfully after locking 15 packages.

## Managed acquisition implementation

Added a platform-keyed production table for Linux x86_64 and aarch64.

Both records use exact versioned GitHub URLs and baked SHA-256 values.

Other operating-system/architecture pairs fail before networking with a named
unsupported managed-runtime error.

Managed resolution checks the exact final executable before selecting a release
or constructing an HTTP client.

On cache miss, acquisition creates a unique sibling temporary directory.

One Ureq call streams the response into `download.tar.gz` there.

The archive file is flushed and synced before hashing.

SHA-256 is calculated over the complete compressed archive.

Mismatch reports the exact URL, expected digest, and actual digest.

Only a verified archive is decoded.

Extraction accepts exactly one regular top-level `zellij` entry and rejects all
other paths, types, or duplicate entries.

The extracted executable is flushed, synced, and chmodded to 0755.

The archive is removed before the complete temporary directory is renamed to
the final version directory.

A drop guard removes temporary state for every pre-publication error.

A concurrent rename loser accepts an already-published executable and removes
only its own temporary candidate.

No managed failure falls back to PATH.

## Fixture-server coverage

Added a bounded local HTTP/1.1 server inside runtime tests.

The fixture archive is generated in memory and contains a runnable Zellij 0.43.1
version stub.

Focused runtime result: 15 passed, 0 failed.

New acceptance tests cover:

- successful fetch, checksum verification, constrained extraction, and atomic
  store;
- checksum mismatch with URL, expected digest, actual digest, and no partial
  install;
- interrupted response with one request and no torn final/temp directory;
- offline cache miss with one bounded named error containing URL and digest;
- second managed resolution with the fixture request count remaining exactly
  one.

## Pre-commit verification

`cargo clippy -p lisa-cli --all-targets -- -D warnings` passed.

`cargo fmt --all -- --check` passed.

`git diff --check` passed.

Search found no managed retry loop, checksum-sidecar fetch, or bare
`Command::new("zellij")` launch path.

`cargo test -p lisa-cli` passed:

- library unit tests: 14 passed;
- binary unit tests: 301 passed;
- integration tests: 17 passed;
- one explicitly ignored real-Zellij boundary remained ignored;
- doc tests: 0 failures.

## Isolated source commit

Committed through `lisa commit-ticket` with exact includes:

- `Cargo.lock`;
- `crates/lisa-cli/Cargo.toml`;
- `crates/lisa-cli/src/runtime.rs`.

Commit: `f747eca345174e9f53918b05fec403d066111dd2`.

Message: `Install pinned managed Zellij atomically`.

No ordinary `git add` or `git commit` was used.

The ordinary index contained no ticket-owned staged path before or after the
transaction.

## Aggregate verification

`just check` passed.

Its WASM check completed for `lisa-plugin` on `wasm32-wasip1`.

Its `cargo test --workspace` run passed all 941 active tests:

- lisa-cli library: 19 passed;
- lisa-cli binary: 301 passed;
- lisa-cli integration: 17 passed;
- lisa-core unit: 207 passed;
- lisa-core integration: 2 passed;
- lisa-plugin unit: 395 passed.

One real-Zellij delivery test remained intentionally ignored under its existing
manual-dependency guard.

All doc-test targets passed with no failures.

## Final implementation status

All ticket acceptance criteria are covered by deterministic localhost tests.

The source commit is durable and its three owned paths are clean.

Real GitHub download execution on Linux is deliberately deferred to the
story-defined human-operated Chromebook test; automated tests never touch the
real network.
