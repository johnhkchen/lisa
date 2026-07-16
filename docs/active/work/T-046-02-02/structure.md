# Structure — T-046-02-02 pinned fetch, verify, and store

## Change inventory

Modify `crates/lisa-cli/Cargo.toml`.

Modify workspace `Cargo.lock` through Cargo's dependency resolution.

Modify `crates/lisa-cli/src/runtime.rs`.

Add no new configuration, CLI command, or shared lisa-core type.

Keep fixture-server tests beside the private runtime implementation so they can
exercise injected release metadata without exposing test-only public APIs.

## `crates/lisa-cli/Cargo.toml`

Add `ureq` as a synchronous HTTPS client.

Pin the compatible major line used by this implementation.

Disable default features and enable TLS only.

Do not enable automatic gzip response decoding.

Add `sha2` for archive SHA-256 calculation.

Add `flate2` for `.gz` decoding after verification.

Add `tar` with default features disabled for archive iteration.

Keep `tempfile` in dev-dependencies for test roots.

No external fixture-server crate is needed because the test protocol is a small
HTTP/1.1 response over `TcpListener`.

## `Cargo.lock`

Record the selected direct dependencies and their transitive TLS/compression
graph.

This file changes mechanically through Cargo.

The lockfile remains workspace-wide and must be committed in the same isolated
ticket transaction as the manifest.

## Existing release constant

Keep `MANAGED_ZELLIJ_VERSION` as the canonical installed version.

Update its documentation from future-installer wording to current ownership.

Do not duplicate a separately parsed version string.

The asset selector's URL literals include the same version and are checked in
tests for consistency.

## New `ManagedRelease`

Add a private structure in `runtime.rs`.

Fields are `url: &str` and `sha256: &str`.

A lifetime-parameterized structure permits both static production records and
test-local fixture records.

No release filename field is necessary because the URL and expected archive
entry fully determine behavior.

## Production asset selection

Add `managed_release() -> Result<ManagedRelease<'static>, String>`.

On Linux x86_64, return the pinned x86_64 no-web static-musl URL and hash.

On Linux aarch64, return the pinned aarch64 no-web static-musl URL and hash.

On every other target, return a named unsupported managed-runtime error.

The selector makes no filesystem or network calls.

Unit assertions cover literal version alignment and digest shape where the host
has a supported production record.

## Managed path helpers

Retain `managed_zellij_path` as the source of the final executable location.

Derive the final version directory with `path.parent()`.

Derive the runtime root with the version directory's parent.

Return internal invariant errors if either parent is unexpectedly absent.

Do not change XDG or HOME behavior.

## Cache predicate

Reuse `is_executable` for the final executable.

Add `ensure_managed_zellij(path, release)`.

Its first operation checks the final executable.

If executable, return success without constructing an HTTP request.

If the final directory exists without a usable executable, return a named
invalid-cache error and do not overwrite it.

If the directory is absent, continue into acquisition.

## Temporary directory naming

Add `create_install_temp_dir(runtime_root, version_name)`.

Ensure the runtime root exists using `create_dir_all`.

Generate candidates using process ID plus a per-process atomic counter.

Create with `create_dir`, not `create_dir_all`, so collisions are detectable.

Retry only local filename collisions for a small bounded number of candidates.

This local collision loop is not a network retry.

## Cleanup guard

Add a private `TempInstall` structure.

It stores the created path and a publication flag.

Its `Drop` removes the temporary tree unless publication disarms it.

Provide private path access and disarm behavior.

HTTP, digest, archive, permission, and rename failures all share this cleanup
path.

## Download step

Add `download_archive(release, destination) -> Result<(), String>`.

Build one Ureq request for `release.url` and set a stable Lisa user agent.

Map HTTP status and transport errors into `Managed Zellij download failed`.

Create the destination only inside the temporary directory.

Copy the response reader into a buffered file.

Flush and sync the file before verification.

Map body read and disk-write failures to the same named category.

All errors include the release URL and expected SHA-256.

## SHA-256 step

Add `sha256_file(path)` and a byte digest helper useful to tests.

Read the downloaded file through a buffered reader.

Feed blocks to `Sha256` and format lowercase hexadecimal.

Compare the result exactly with the compiled expectation.

On mismatch, return `Managed Zellij checksum mismatch`.

Include expected and actual values, plus URL.

Do not begin gzip parsing before this comparison succeeds.

## Extraction step

Add `extract_zellij(archive_path, executable_path, release)`.

Open the verified archive and wrap it in `GzDecoder` and `tar::Archive`.

Iterate every entry.

Accept only one regular-file entry whose path equals `zellij`.

Reject a second matching entry and every other entry type or path.

Copy the accepted entry into a newly created executable file.

Flush and sync that file, then set Unix mode to `0755`.

Require one accepted executable after iteration ends.

Map errors into `Managed Zellij install failed`, including URL and digest.

## Publication step

Remove `download.tar.gz` after extraction succeeds.

Rename the complete temporary directory to the final version directory.

On success, disarm cleanup.

On rename failure, check whether a concurrent process populated a usable final
executable.

If so, clean the losing candidate and return success.

If not, return a named atomic-install failure.

Never remove the final directory.

## Resolver integration

In `resolve_zellij_runtime_in`, change only the `Managed` match arm.

Derive the final path.

If it is not already executable, obtain the production release record.

Call the acquisition function.

Return the same mode and path to shared canonicalization and inspection.

System continues to call `find_system_zellij`.

Pinned continues to use the configured path.

All modes continue through `absolute_executable` and `inspect_zellij`.

## Error formatting helper

Add a helper taking category, detail, and release.

The exact URL and expected checksum appear unmodified in every release-specific
failure.

Checksum mismatch also includes the calculated digest.

Tests assert stable categories and required substrings rather than entire
low-level messages.

## Fixture archive helpers

Inside runtime tests, add `fixture_archive`.

Use `tar::Builder` and `flate2::write::GzEncoder` to produce bytes.

Append one executable entry named `zellij`.

The script prints `zellij 0.43.1` for existing inspection logic.

Compute the fixture digest through `Sha256`.

## Local fixture server

Add a test-only `FixtureServer` backed by `TcpListener`.

Bind to `127.0.0.1:0`.

Expose `url()` and an atomic request count.

Spawn a thread that accepts a bounded configured number of connections.

Read request headers through the blank line.

Successful mode returns HTTP 200 with exact content length and archive bytes.

Interrupted mode advertises a larger content length, writes only a prefix, and
closes.

The server never accesses the internet.

Use a bounded accept deadline so a failed cache test cannot hang.

## Acceptance tests

`successful_fetch_verify_and_atomic_store` asserts one request, a usable final
executable, no downloaded archive, no temporary sibling, and inspected version
0.43.1.

`checksum_mismatch_is_named_and_leaves_no_partial_install` asserts URL,
expected and actual digests, one request, and absent final/temp directories.

`interrupted_download_leaves_no_torn_runtime_directory` asserts one request,
named error with URL/digest, and absent final/temp directories.

`offline_without_cache_is_single_named_error` points at a closed local port and
asserts one bounded failure containing URL and digest.

`second_managed_resolution_is_zero_network` performs installation and then the
same managed resolution against the cached path, asserting the counter remains
one.

## Verification boundaries

Run formatter before tests.

Run runtime-focused tests first.

Run all lisa-cli tests and the full workspace suite.

Run `just check` as the aggregate gate.

Inspect git diff for only the exact owned paths.

Commit source through `lisa commit-ticket` with exact includes.
