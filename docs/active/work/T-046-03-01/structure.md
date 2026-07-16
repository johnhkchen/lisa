# T-046-03-01 Structure

## Change map

This ticket modifies seven repository paths.

It creates one managed-runtime data file.

It creates one release-verification script.

It modifies the dist configuration.

It modifies the AUR package's artifact references.

It modifies the CLI build script.

It modifies the runtime module.

It modifies the generated release workflow through its preserved extension seam.

No scheduler, plugin logic, config syntax, or public command is redesigned.

## `dist-workspace.toml`

### Target list

The two Darwin entries remain unchanged and in their current order.

`x86_64-unknown-linux-gnu` is removed.

`aarch64-unknown-linux-gnu` is removed.

`x86_64-unknown-linux-musl` is added.

`aarch64-unknown-linux-musl` is added.

The list continues to contain exactly four targets.

The target ordering groups Darwin before Linux as it does today.

### Installer settings

`install-path = "CARGO_HOME"` is replaced.

The new value is `install-path = "~/.local/bin"`.

An adjacent `install-success-msg` is added.

The message names Lisa, the destination, a fresh shell, and `lisa doctor`.

No custom environment-variable cascade is introduced.

### Preserved settings

The cargo-dist pin stays at `0.30.4`.

`allow-dirty = ["ci"]` remains.

Shell and Homebrew remain enabled.

The package selector stays `lisa-cli`.

The build setup path stays `../build-setup.yml`.

Tap, formula, publish job, and prerelease behavior stay unchanged.

## `aur/PKGBUILD`

Both source URL filenames change from GNU to musl triples.

The logical source aliases remain architecture-specific.

Archive format and extraction assumptions remain unchanged.

The `zellij` runtime dependency remains.

The direct `gcc-libs` dependency is removed.

Version, release, checksums, license handling, and install destination remain.

No AUR publication action is part of this ticket.

## `crates/lisa-cli/data/managed-runtime-sha256.json`

This is a new checked-in release input.

The file is UTF-8 JSON ending in one newline.

Its top-level object contains `version` and `artifacts`.

`version` is the exact string `0.43.1`.

`artifacts` is an array of four objects.

Each artifact object has exactly three fields.

`target` is a Rust target triple.

`archive` is an official Zellij no-web tarball filename.

`sha256` is the upstream-published hash of the extracted executable.

Entries are ordered by the same target ordering as dist configuration.

The Darwin entries use no-web Darwin archives.

The Linux entries use no-web static-musl archives.

There are no mutable URLs, `latest` aliases, comments, or fetched metadata.

## `crates/lisa-cli/src/runtime.rs`

### Embedded constant

A public constant is added beside `MANAGED_ZELLIJ_VERSION`.

Its name is `MANAGED_RUNTIME_SHA256_MANIFEST`.

Its type is `&str`.

Its value uses `include_str!("../data/managed-runtime-sha256.json")`.

Its documentation states that hashes apply to unpacked executables.

Its documentation names T-046-02-02 as the acquisition consumer.

No parsing occurs in the production resolver in this ticket.

### Tests

Tests parse the embedded constant with `serde_json::Value`.

They do not read the source path with filesystem APIs.

One assertion compares manifest version to `MANAGED_ZELLIJ_VERSION.to_string()`.

One assertion requires an array length of four.

One assertion builds a set of observed targets.

The expected set contains both Darwin and both musl Linux triples.

Duplicate targets fail through set insertion assertions.

Every archive starts with `zellij-no-web-`.

Every archive ends with `.tar.gz`.

Every archive contains its target triple.

Every hash has 64 characters.

Every hash character is lowercase hexadecimal.

The test stays inside the existing `runtime.rs` test module.

No new runtime dependency is added because serde_json already exists.

## `crates/lisa-cli/build.rs`

### Validation helper

The build script gains one small helper for WASM validity.

Validity means non-empty and beginning with `\0asm\x01\0\0\0`.

The helper reads metadata or the small fixed header as needed.

Error messages name the concrete source path.

### Release requirement flag

The environment input is `LISA_REQUIRE_EMBEDDED_WASM`.

The only enabled value is `1`.

The script emits `cargo:rerun-if-env-changed` for the flag.

It keeps the existing `rerun-if-changed` source declaration.

When the source exists, the script validates before copying.

An empty or malformed existing source is always an error.

When the source is absent and the flag is set, the build fails.

When the source is absent and the flag is not set, it writes the placeholder.

The destination path and `include_bytes!` contract do not change.

## `.github/build-setup.yml`

The existing target-install and WASM-build steps remain first.

A third step named for WASM verification is added.

It runs under bash on all current Unix release runners.

It uses `test -s` on the release WASM.

It reads the first eight bytes with standard Unix tools.

It compares those bytes to the WebAssembly module header.

It appends `LISA_REQUIRE_EMBEDDED_WASM=1` to `$GITHUB_ENV`.

The exported flag is visible to the subsequent `dist build` step.

The setup does not mutate Cargo profiles or target triples.

## `scripts/verify-musl-release.sh`

This new executable script is CI-oriented but locally runnable.

It uses POSIX tools under a bash shebang with strict mode.

Its only required argument is an exact musl target triple.

It rejects non-musl or unsupported triples early.

It accepts an optional distribution directory through an environment variable.

The default distribution directory is `target/distrib`.

### Archive discovery

The expected basename starts `lisa-cli-<target>.tar`.

The script supports cargo-dist's configured `.tar.xz` output.

It requires exactly one matching archive.

Zero or multiple matches are named failures.

Temporary extraction uses `mktemp -d` and a cleanup trap.

The executable is discovered below the extracted archive root.

Exactly one regular file named `lisa` is required.

### Static-link proof

`file` output must contain `ELF` and `statically linked`.

`readelf -l` output must not contain an `INTERP` program header.

`ldd` output is captured without treating its expected nonzero exit as failure.

Accepted output must name a static or non-dynamic executable.

This triangulates rather than trusting any one tool's wording.

### Embedded-data proof

A small Python 3 check reads the packaged native executable.

It requires the eight-byte WebAssembly header to occur in its bytes.

It requires one known manifest checksum to occur as ASCII.

The checksum selected is an immutable entry from the checked-in manifest.

This confirms both compile-time assets survived LTO and packaging.

### Bullseye execution

The script requires Docker only for its final execution boundary.

It mounts the extracted executable read-only into `debian:bullseye-slim`.

It invokes `/lisa --version` without installing packages.

The container uses the native runner architecture.

The output must begin with `lisa ` and exit successfully.

The script prints a concise success summary including target and version.

## `.github/workflows/release.yml`

The dynamic plan and build matrices stay untouched.

The existing `Build artifacts` step stays responsible for `dist build`.

A new step follows it.

Its condition checks whether joined matrix targets contain `unknown-linux-musl`.

It invokes `scripts/verify-musl-release.sh` with the sole matrix target.

The step runs before the `Post-build` manifest/upload collection.

No job permissions, runner labels, containers, or artifact names are hardcoded.

Manual dispatch and tag handling remain unchanged.

## Artifact-only paths

The attempt directory receives `progress.md` during implementation.

It later receives `review.md` and `review-disposition.json`.

Research, design, structure, plan, progress, and review artifacts are not passed
to `lisa commit-ticket`; Lisa publishes admitted artifacts after lease checks.

## No-change boundaries

`Cargo.toml` retains fat LTO and the dist profile exactly as written.

`Cargo.lock` is unchanged because no dependency is added.

`templates.rs` keeps the existing embedded WASM constant.

`loop_cmd.rs` keeps its empty-WASM runtime refusal.

Network download, retry, checksum comparison, unpack, and cache behavior remain
owned by T-046-02-02.

README and broader agent-facing copy remain owned by S-046-04.

Stable release publication remains owned by T-046-03-02 and John.

## Ordering constraints

The manifest file must exist before `runtime.rs` references it.

The WASM build script validation must exist before CI exports its requirement.

The verifier script must exist before the workflow invokes it.

The dist target change must precede final `dist plan` evidence.

All ticket-owned source paths must be committed before Review artifacts are written.
