# Review — T-046-02-02 pinned fetch, verify, and store

## Outcome

T-046-02-02 is ready to pass.

Managed mode now fills the versioned runtime path established by T-046-02-01.

On a valid cache hit, resolution performs no network operation.

On a cache miss, Lisa downloads one pinned no-web static-musl Zellij archive,
verifies its baked SHA-256, extracts a constrained executable, and publishes the
complete version directory through same-filesystem rename.

Failures are bounded and do not fall back to a system binary.

The ticket's source was committed through Lisa's isolated transaction.

## Source commit

Commit: `f747eca345174e9f53918b05fec403d066111dd2`.

Message: `Install pinned managed Zellij atomically`.

The commit contains exactly:

- `Cargo.lock`;
- `crates/lisa-cli/Cargo.toml`;
- `crates/lisa-cli/src/runtime.rs`.

No ticket-owned source path remains staged, modified, or untracked.

No ordinary index commit workflow was used.

## Manifest and lockfile changes

The CLI now directly depends on four acquisition components.

Ureq 2.12 provides synchronous HTTP and HTTPS.

Its default features are disabled and Rustls TLS is enabled explicitly.

Automatic response gzip decoding is not enabled; the release archive remains
byte-identical through checksum verification.

SHA-2 provides the SHA-256 implementation.

Flate2 decodes the already-verified gzip stream.

Tar iterates the archive with default features disabled.

Dependency-tree inspection found no native-tls, OpenSSL, or xattr dependency.

The lockfile records 15 newly selected packages, including Rustls and Ring.

## Pinned release provenance

The managed version remains Zellij 0.43.1.

The x86_64 Linux artifact is the upstream no-web static-musl archive at the
exact versioned GitHub release URL.

Its baked SHA-256 is
`bac0728945e8f5a28f2647e2b9b0cfe4591d71abfe227336b1318937241f071d`.

The aarch64 Linux artifact is the corresponding upstream no-web static-musl
archive.

Its baked SHA-256 is
`8ced877df27a8fe9112607dd3d772442aefa5e42359cda1baba53e78c4ae46aa`.

These values were verified against official GitHub release metadata.

The x86_64 archive was independently downloaded during Research, hashed, and
inspected; its digest matched and it contained one top-level `zellij` entry.

Runtime code never requests release metadata or checksum sidecars.

## Platform selection

The release selector keys on operating system and architecture.

Linux x86_64 and Linux aarch64 map to the two pinned records.

Other target pairs return a named unsupported managed-runtime error before any
network access.

This matches the ticket and story boundary, which specifies static-musl Linux
runtime assets for Chromebook-class Debian environments.

System and absolute pinned runtime modes remain available on other targets.

## Cache behavior

Managed resolution derives the existing XDG/HOME versioned executable path.

It checks that exact file for executable status before selecting release
metadata or constructing the HTTP agent.

A valid cache hit proceeds directly to canonicalization and normal
`zellij --version` inspection.

That preserves compatibility enforcement without network activity.

An existing version directory without a usable executable is not overwritten.

It produces a named invalid-cache error on supported managed platforms.

Version upgrades remain isolated because every release uses its own directory.

## Download behavior

One blocking Ureq call is made on a cache miss.

Connect, read, and write timeouts are finite.

No retry loop wraps the request.

Normal bounded redirect handling permits GitHub's release-asset delivery.

The response is streamed into `download.tar.gz` inside a unique sibling
temporary directory.

The archive file is flushed and synced before hashing.

HTTP, transport, body-read, and storage failures use the stable
`Managed Zellij download failed` category.

Release-specific errors print the exact requested URL and expected SHA-256.

## Verification behavior

SHA-256 covers the complete compressed archive bytes.

The calculated digest is formatted in lowercase hexadecimal.

Extraction cannot begin until it equals the baked expected digest.

Mismatch uses the stable `Managed Zellij checksum mismatch` category.

The error includes exact URL, expected digest, and actual digest.

No remotely supplied checksum participates in the decision.

## Extraction behavior

The verified archive is decoded with Flate2 and iterated with Tar.

The extractor accepts exactly one regular top-level entry named `zellij`.

It rejects nested paths, other filenames, non-file entry types, links, and
duplicate entries.

It does not call general-purpose archive unpacking.

The executable is created with create-new semantics in the private candidate
directory.

Its contents are flushed and synced.

Unix permissions are set to 0755.

The downloaded archive is removed before publication.

## Atomicity and cleanup

Each candidate directory is a sibling of the final version directory.

Its name combines the version, process ID, and an atomic per-process sequence.

The final path remains absent through download, digest, and extraction.

A drop guard recursively removes the candidate after any failure.

Successful installation renames the entire complete directory into place.

Because source and destination share the runtime root, publication stays on one
filesystem.

If another process wins the rename race, the loser accepts only a final path
that is already executable and removes its own candidate.

No code removes or replaces an existing published version directory.

## Resolver integration

Only the Managed match arm changed behavior.

System mode retains PATH discovery and version enforcement.

Pinned mode retains exact configured path behavior.

All modes still canonicalize and inspect the selected executable.

Managed errors never switch to PATH.

Loop already resolves the runtime before plugin and layout side effects, so an
acquisition error remains early and actionable.

Doctor uses the same resolver, so it can populate a missing managed cache and
then report the resulting mode, version, and path.

## New acceptance-test harness

Runtime tests now contain a bounded local HTTP/1.1 fixture server.

It binds an ephemeral loopback port and records accepted requests atomically.

It can send a complete archive or close after a deliberately truncated body.

The server has a bounded accept deadline to prevent test hangs.

Fixture archives are generated in memory.

They contain a runnable shell stub reporting `zellij 0.43.1`.

No automated acceptance test contacts GitHub.

## Acceptance coverage

`successful_fetch_verify_and_atomic_store` covers a complete response, matching
digest, constrained extraction, executable permissions, version inspection,
archive removal, and absent temporary state.

`checksum_mismatch_is_named_and_leaves_no_partial_install` covers the stable
category, URL, expected/actual digest visibility, exactly one request, absent
final directory, and absent temporary state.

`interrupted_download_leaves_no_torn_runtime_directory` covers a truncated
body, one accepted request, named provenance-bearing error, absent final
directory, and cleanup.

`offline_without_cache_is_one_bounded_named_error` uses a closed local address,
asserts the download category, exact URL and digest, a sub-two-second local
failure, and no final/temp directory.

`second_managed_resolution_performs_zero_network_calls` installs once, resolves
again from the same path, and proves the server request counter remains one.

The existing managed/system/pinned selection tests continue to pass.

## Verification results

Focused runtime suite: 15 passed, 0 failed.

Strict Clippy for all lisa-cli targets passed with warnings denied.

Formatting check passed.

Git whitespace/diff validation passed.

The pre-commit lisa-cli suite passed 301 binary unit tests, 14 library tests,
and 17 active integration tests.

The post-commit `just check` gate passed.

Its `wasm32-wasip1` lisa-plugin check succeeded.

Its full workspace suite passed all 941 active tests.

One existing real-Zellij delivery test remained intentionally ignored under
its manual prerequisite guard.

## Acceptance assessment

Successful fetch and verification: covered and passing.

Checksum mismatch named error and no partial install: covered and passing.

Interrupted download and no torn runtime directory: covered and passing.

Offline no-cache bounded error with exact URL and SHA-256: covered and passing.

Second loop-equivalent managed resolution with zero additional network calls:
covered through an observable fixture request count and passing.

No retry loop, checksum fetch, or silent system fallback exists.

## Open concerns and limits

Automated tests intentionally use localhost and do not execute a live GitHub
download, matching the story's honest CI boundary.

The real x86_64 asset and digest were manually inspected during Research, but a
real Linux execution of Lisa's installer remains part of the human-operated
Chromebook acceptance story.

The development host has macOS and WASM Rust targets installed, not Linux cross
targets, so the full Linux native binary was not cross-compiled in this attempt.

The platform table itself is host-independent and both Linux mappings are unit
tested.

Future Zellij version bumps must update the managed version, both versioned URLs,
and both baked hashes together.

Cache eviction and automatic repair of a corrupt published version directory
remain explicitly outside this ticket.

No critical issue remains for T-046-02-02.
