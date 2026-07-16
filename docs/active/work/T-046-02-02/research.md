# Research — T-046-02-02 pinned fetch, verify, and store

## Ticket boundary

T-046-02-02 completes the managed Zellij runtime path introduced by its
predecessor.

Managed mode must obtain one exact Zellij release when its versioned runtime is
not cached.

The release is Zellij 0.43.1, matching the existing managed-version constant.

The selected artifacts are the upstream `no-web` static-musl Linux archives.

The supported machine architectures named by the ticket are x86_64 and
aarch64.

Acquisition must verify an expected SHA-256 compiled into Lisa.

It must not fetch a checksum file or derive trust from the download server.

Installation must publish a complete version directory atomically.

A valid cached runtime must avoid all network activity on later loop runs.

Failures must be bounded, named, and leave no partial runtime directory.

## Predecessor runtime contract

`crates/lisa-cli/src/runtime.rs` owns native Zellij selection and inspection.

`MANAGED_ZELLIJ_VERSION` is `0.43.1`.

`ZellijRuntimeRequest` distinguishes managed, system, and pinned modes.

`ResolvedZellijRuntime` retains mode, parsed version, and canonical path.

`RuntimeEnvironment` captures PATH, XDG data home, and HOME.

`managed_zellij_path` derives the final executable location.

With an absolute `XDG_DATA_HOME`, the location is:

`$XDG_DATA_HOME/lisa/runtime/zellij-0.43.1/zellij`.

With HOME fallback, the location is:

`$HOME/.local/share/lisa/runtime/zellij-0.43.1/zellij`.

The current managed resolver requires that path to exist before it canonicalizes
and inspects it.

That missing-file boundary is the insertion point for acquisition.

System and pinned modes already resolve independently and must remain free of
managed download behavior.

## Runtime inspection

After path selection, `absolute_executable` canonicalizes the executable.

`inspect_zellij` invokes the selected path with `--version`.

The output is classified through `lisa_core::version`.

Unsupported and unparseable binaries fail closed with mode and path context.

Successful managed acquisition must therefore install an executable file whose
mode permits invocation before normal resolution continues.

An existing cache is currently accepted only if canonicalization and version
inspection succeed.

The ticket's zero-network cache rule is compatible with this behavior because
the final executable path can be checked before any acquisition request.

## Loop integration

`crates/lisa-cli/src/loop_cmd.rs::run_loop` validates project structure and
protocol before resolving Zellij.

Dry-run exits before runtime resolution.

A real loop discovers the Git root, then calls
`runtime::resolve_zellij_runtime`.

Runtime resolution occurs before provider checks, plugin extraction, cache
cleaning, permission setup, layout writing, or Zellij execution.

This makes acquisition failure early relative to loop side effects.

The returned path is printed and passed directly to `exec_zellij`.

No additional managed-runtime hook exists in the loop module.

Doctor calls the same public resolver and therefore observes the same cache and
acquisition behavior as a real loop.

## Configuration boundary

`crates/lisa-cli/src/config.rs` defaults an absent runtime setting to managed.

The literal `managed` selects the same request.

`system` and absolute paths opt out of managed acquisition.

No URL, version, checksum, architecture, or retry option is configurable.

The ticket therefore adds no required configuration surface.

Keeping release identity compiled into the binary preserves that boundary.

## Release assets

The official Zellij GitHub release tag is `v0.43.1`.

The release exposes no-web static-musl archives for both supported Linux
architectures.

The x86_64 asset name is
`zellij-no-web-x86_64-unknown-linux-musl.tar.gz`.

Its exact URL is
`https://github.com/zellij-org/zellij/releases/download/v0.43.1/zellij-no-web-x86_64-unknown-linux-musl.tar.gz`.

GitHub release metadata publishes its SHA-256 digest as
`bac0728945e8f5a28f2647e2b9b0cfe4591d71abfe227336b1318937241f071d`.

The aarch64 asset name is
`zellij-no-web-aarch64-unknown-linux-musl.tar.gz`.

Its exact URL is
`https://github.com/zellij-org/zellij/releases/download/v0.43.1/zellij-no-web-aarch64-unknown-linux-musl.tar.gz`.

GitHub release metadata publishes its SHA-256 digest as
`8ced877df27a8fe9112607dd3d772442aefa5e42359cda1baba53e78c4ae46aa`.

The inspected x86_64 tarball contains one top-level entry named `zellij`.

Its locally calculated SHA-256 matches the published digest.

No checksum fetch is needed at runtime.

## Target matrix

`dist-workspace.toml` builds Lisa for macOS and Linux, each on x86_64 and
aarch64.

The managed assets required by this ticket are explicitly Linux static-musl
binaries.

Rust compile-time `target_arch` and `target_os` can select a release record.

The two Linux architectures map directly to the two verified asset names.

Other operating systems or architectures have no ticket-specified artifact.

An unsupported target must therefore fail before attempting a URL.

Unit and integration tests run on the current development platform, including
macOS, so platform-independent acquisition logic needs a test-injected release
record.

## Current dependency surface

`crates/lisa-cli/Cargo.toml` has no direct HTTP, archive, compression, or digest
dependencies.

Its direct runtime dependencies are lisa-core, clap, directories, toml, serde,
serde_json, and fs2.

`sha2` is already present transitively in the workspace lockfile but is not
available to lisa-cli without a direct declaration.

Neither `tar` nor `flate2` nor an HTTP client is currently locked as a direct
lisa-cli facility.

The standard library has TCP and filesystem primitives but no HTTPS client,
gzip decoder, tar reader, or SHA-256 implementation.

Using an external `curl`, `tar`, or checksum executable would add undeclared
runtime dependencies to the supposedly self-contained Lisa binary.

The existing CLI already distributes as native glibc Linux and macOS binaries,
so a Rust HTTP/TLS stack can remain inside Lisa's artifact.

## Atomicity and filesystem observations

The final installation boundary is the version directory, not just the binary.

Publishing by renaming a sibling temporary directory keeps the rename on the
same filesystem.

Before rename, the final directory must not exist for a cache miss.

Downloaded archive bytes and extracted contents must stay outside the final
directory until verification and extraction both succeed.

Checksum mismatch must remove temporary state and leave the final directory
absent.

Interrupted reads are ordinary I/O failures and have the same cleanup
requirement.

The standard library's `rename` is atomic for a same-filesystem directory
publication when the destination does not exist.

Concurrent Lisa processes can both observe a cache miss.

Their temporary names must not collide.

Only one rename can win; a loser can accept a concurrently completed valid
destination after its rename fails.

No long-lived archive belongs in the cache contract.

## Failure vocabulary

The acceptance criteria distinguish checksum mismatch and offline with no
cache.

Both require the exact tarball URL and expected SHA-256 in the error.

An interrupted response is a download failure rather than a checksum mismatch
when the body read itself fails.

An HTTP status failure is also a download failure.

There is no retry loop in the existing resolver.

One resolver call should issue at most one HTTP request on a cache miss.

The error should name managed Zellij acquisition rather than exposing only a
low-level transport or archive message.

Unsupported platform errors have no applicable tarball URL or digest.

## Existing test conventions

Most runtime tests live beside `runtime.rs` under `#[cfg(test)]`.

They use `tempfile` and executable shell stubs on Unix.

Loop tests use temporary project roots and a `default_config` helper.

The current tests do not launch a real loop because `exec` would replace the
test process.

An acquisition integration seam can exercise the exact managed resolution
operation invoked by loop without reaching `exec_zellij`.

A local fixture server can count accepted connections and control response
status and truncation.

Tests can generate a small tar.gz containing a version-printing executable,
compute its digest, and provide a fixture release record.

Running resolution twice against the same data root makes the cache rule
observable through an unchanged server request counter.

Checksum and interruption tests can assert the final version directory is
absent and sibling temporary directories are removed.

## Repository state constraints

The ordinary worktree contains Lisa-owned provenance changes and untracked
planning documents unrelated to this ticket.

They must remain untouched.

Ticket source ownership is expected to include `runtime.rs`, dependency
manifests and lockfile, plus a focused integration-test file if added.

Every source unit must be committed with exact paths through
`lisa commit-ticket`.

Phase artifacts belong only in this attempt's private work directory until Lisa
publishes them.

Ticket phase and status frontmatter are Lisa-managed and must not be edited.
