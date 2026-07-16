# Design — T-046-02-02 pinned fetch, verify, and store

## Decision summary

Extend the existing native runtime module with a small managed-acquisition
pipeline.

Represent each supported release asset as a compiled URL and SHA-256 pair.

On a managed cache miss, make one blocking HTTPS request, stream it into a
sibling temporary directory, verify the whole archive, extract only the
expected executable, and atomically rename the directory into place.

Keep system and pinned resolution unchanged.

Use injected release metadata and a local HTTP fixture server for deterministic
tests on every development host.

## Option 1 — shell out to curl, shasum, and tar

Lisa could construct a shell pipeline around tools commonly present on Linux.

This approach would add almost no Rust dependencies.

The commands are individually familiar and can stream large archives.

However, the executable would no longer be self-contained.

Minimal Chromebook environments may lack one or more tools.

Command behavior and flags vary between GNU and BSD userlands.

Pipeline interruption and exit attribution complicate the named-error
contract.

Secure extraction would depend on external tar behavior.

Testing an interrupted body would exercise process orchestration more than the
installer state machine.

Rejected because runtime acquisition is a core Lisa capability and should not
silently acquire new system dependencies.

## Option 2 — invoke a GitHub release API at runtime

Lisa could query release metadata, find the matching asset, and use GitHub's
digest field.

This would reduce manual release-table maintenance.

It would also make the checksum mutable network input.

An attacker or upstream-account compromise controlling both metadata and asset
could satisfy that check.

It would require two requests and create more offline failure modes.

The ticket explicitly requires a checksum baked into the Lisa binary at release
time and never fetched.

Rejected because it violates the provenance boundary and bounded one-request
failure behavior.

## Option 3 — fetch archive and checksum sidecar

Zellij publishes `.sha256sum` assets beside release tarballs.

Lisa could download both and validate conventional checksum-file syntax.

This still fetches trust material from the same server at runtime.

It doubles network calls on every uncached installation.

Offline and partial-server cases become less predictable.

Rejected for the same baked-trust reason as the release API.

## Option 4 — embedded archive in the Lisa distribution

Each Lisa target could bundle the corresponding Zellij binary or tarball.

Managed mode would then require no network.

The installation path could remain atomic.

Lisa release artifacts would grow by roughly the full Zellij archive size.

The current build embeds a WASM plugin already, but the ticket explicitly
describes managed mode as downloading a pinned release.

Release packaging would need per-target asset assembly outside the current
ticket boundary.

Rejected because it changes the delivery model and does not implement the
required fetch behavior.

## Option 5 — Rust-native HTTP, digest, and archive pipeline

A blocking HTTP client fits the synchronous CLI call path.

The archive can be streamed to disk and hashed without loading it all into
memory.

Rust gzip and tar readers allow exact entry validation.

Filesystem cleanup and rename can be controlled in one state machine.

Local fixture servers can test request counts and interrupted reads directly.

This is the selected approach.

## HTTP client choice

Use `ureq` 2.12 with TLS enabled and response decompression disabled.

Ureq provides a synchronous reader matching the existing synchronous resolver.

Its Rustls-backed TLS avoids a dependency on platform OpenSSL packages.

Disabling the optional HTTP content-decoding feature avoids conflating transfer
encoding with the `.tar.gz` payload.

The gzip archive remains opaque to the client and is decoded only after digest
verification.

The request uses the exact compiled URL and follows the client's bounded normal
redirect policy for GitHub release delivery.

No retry loop wraps the call.

One cache miss therefore causes at most one logical request from Lisa.

## Digest and archive choices

Use `sha2` for SHA-256.

The expected digest remains a lowercase 64-character compile-time string.

Hash the complete downloaded archive bytes, not extracted content.

This matches the GitHub release-asset digest.

Use `flate2` to decode gzip after verification.

Use `tar` with default features disabled to avoid unnecessary xattr behavior.

Do not call general-purpose `Archive::unpack`.

Iterate the archive and require exactly one regular-file entry named `zellij`.

Reject absolute paths, nested paths, duplicate executables, links, directories,
and unexpected extra entries.

Copy that one entry to the temporary install directory.

Set Unix permissions to `0755` after extraction.

This makes archive shape part of the pinned release contract.

## Compiled asset table

Define a small `ManagedRelease` value with URL and expected SHA-256.

The production selector uses compile-time target predicates.

Linux x86_64 maps to the official no-web x86_64 static-musl URL and
`bac0728945e8f5a28f2647e2b9b0cfe4591d71abfe227336b1318937241f071d`.

Linux aarch64 maps to the official no-web aarch64 static-musl URL and
`8ced877df27a8fe9112607dd3d772442aefa5e42359cda1baba53e78c4ae46aa`.

All other targets return a named unsupported-target error before networking.

The version remains sourced from `MANAGED_ZELLIJ_VERSION`.

URLs include `v0.43.1`, keeping identity visibly aligned.

Future release work must update version, URLs, and hashes together.

## Cache-first behavior

Managed resolution derives the final executable path first.

If an executable file already exists there, acquisition returns immediately.

This branch runs before asset selection and HTTP client construction.

Normal canonicalization and `--version` inspection still run afterward.

Thus a second real loop with a valid cache causes zero network calls while still
checking that the cached runtime is runnable and protocol-compatible.

A non-executable or malformed final cache is not overwritten implicitly.

It fails through the existing inspection/canonicalization boundary or a named
cache error, preserving upgrade safety.

## Temporary layout

Let the final executable be `<runtime-root>/zellij-0.43.1/zellij`.

Create `<runtime-root>` if it is absent.

Create a unique sibling directory whose name begins
`.zellij-0.43.1.install-`.

Place the raw response at `download.tar.gz` inside it.

After checksum verification, extract `zellij` beside the archive.

Remove `download.tar.gz` before publication.

Rename the entire temporary directory to `zellij-0.43.1`.

The final directory is therefore either absent or complete.

No `.part` file appears inside the final directory.

## Cleanup behavior

Use a small cleanup guard holding the temporary directory.

Its drop implementation removes the directory recursively unless publication
disarms it.

HTTP status errors, transport failures, interrupted reads, checksum mismatches,
gzip errors, tar errors, archive-shape errors, permission errors, and rename
errors all pass through the same cleanup path.

Cleanup failure does not replace the primary actionable error.

Temporary debris is never treated as a cache hit because only the exact final
path qualifies.

## Concurrent acquisition

Unique temporary directories allow multiple processes to download without
writing the same partial file.

Each process verifies and extracts its own archive.

The final same-filesystem rename is the publication arbitration point.

If rename fails because another process has populated the destination, check
for the final executable.

If it exists and is executable, treat acquisition as successful and discard the
losing temporary directory.

Normal runtime inspection then validates the winner.

If the destination is not valid, return a named atomic-install error.

No process removes or replaces an already published version directory.

## Error contract

Create one formatter for managed-acquisition failures.

Every release-specific failure begins with a stable category such as
`Managed Zellij download failed`, `Managed Zellij checksum mismatch`, or
`Managed Zellij install failed`.

Every such error includes `URL: <exact-url>`.

Every such error includes `expected sha256: <exact-digest>`.

Checksum mismatch additionally includes the actual digest.

Transport and interrupted-body errors include their source message.

There is no fallback to PATH and no automatic retry.

Unsupported-target errors name OS and architecture but cannot print a URL that
does not exist.

## Test seam

Keep production selection private and provide an internal function receiving a
`ManagedRelease` and explicit final path.

Tests invoke this same acquisition pipeline with an HTTP URL served from
`127.0.0.1`.

The fixture server owns a request counter and configurable response behavior.

Tests create a tiny tar.gz containing a shell stub that prints
`zellij 0.43.1`.

The digest is calculated by the same SHA-256 library but passed as expected
metadata, modeling a baked fixture digest.

The successful test resolves and inspects the installed stub.

The checksum test supplies a wrong expected digest and asserts the named error,
URL, digest, absent final directory, and absent temp siblings.

The interruption test advertises a body longer than it sends and asserts no
torn runtime directory.

The offline test points at a closed local port, asserts one failure path with
URL and digest, and relies on the lack of retry for bounded behavior.

The cache test invokes the same loop-level resolver operation twice while the
fixture server is configured for only one request, then asserts its counter is
one.

## Scope exclusions

Do not change `.lisa.toml` syntax.

Do not add a runtime version override.

Do not select full web-enabled Zellij archives.

Do not fetch latest-release aliases.

Do not use system Zellij as fallback.

Do not change plugin SDK support policy.

Do not add background updates, retry policy, cache eviction, or repair of an
already published invalid version directory.

Do not modify Lisa-managed ticket frontmatter or shared phase artifacts.
