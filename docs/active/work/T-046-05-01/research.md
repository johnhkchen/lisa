# T-046-05-01 Research

## Ticket boundary

The ticket begins in Research and is the first task in story S-046-05.

Its release output is four Debian packages per Lisa version.

There are two package names: `lisa` and `lisa-runtime-zellij`.

There are two Debian architectures: `amd64` and `arm64`.

The CLI package owns `/usr/bin/lisa`.

The runtime package owns `/usr/libexec/lisa/zellij`.

The CLI package must recommend, rather than require, the runtime package.

The second story ticket owns signed repository hosting and README instructions.

This ticket owns local package construction, release attachment, installation proof,
and the resolver behavior needed for an offline-complete package pair.

The ticket explicitly excludes the Debian archive and treats this as vendor packaging.

## Existing release topology

`dist-workspace.toml` is the cargo-dist configuration source.

It pins cargo-dist 0.30.4.

It declares four native targets.

The Darwin targets are x86_64 and aarch64.

The Linux targets are x86_64 and aarch64 static musl targets.

The only dist package is `lisa-cli`.

Cargo-dist names the binary `lisa` from the crate's `[[bin]]` entry.

Linux archives are `lisa-cli-<target>.tar.xz` under `target/distrib`.

Each archive contains exactly one Lisa executable under an archive root directory.

The configured installers are shell and Homebrew.

The shell installer places Lisa in `~/.local/bin`.

The publish job list currently contains only Homebrew.

Prerelease publication is deliberately enabled because main releases release candidates.

`.github/workflows/release.yml` is cargo-dist-generated with retained local changes.

`allow-dirty = ["ci"]` permits those workflow customizations to survive dist init.

The workflow accepts tag pushes, pull requests, and a required tag dispatch input.

The auto-release workflow dispatches it with an existing `v<workspace-version>` tag.

Every checkout uses the event tag or ref, so builds are tied to one source revision.

## Cargo-dist artifact flow

The plan job emits a JSON dist manifest and a platform matrix.

Four local jobs build the configured target archives.

The two Linux jobs run on native Ubuntu x86_64 and arm64 runners.

The local jobs upload their output and a granular dist manifest to Actions storage.

The global job downloads every local artifact into `target/distrib`.

It then asks dist to build global artifacts.

The host job downloads all scratch artifacts before calling `dist host`.

The host step uploads dist-known artifacts to the release staging area.

The same job downloads the Actions artifacts again and passes `artifacts/*` to `gh release create`.

An extra artifact therefore must be present in the dist plan and upload-file list.

Cargo-dist 0.30.4 supports package-local `[[dist.extra-artifacts]]` entries.

Each entry has a build command and a fixed list of repository-relative output paths.

Extra artifacts are global artifacts.

Their build runs in the global build after local archives have been downloaded.

Cargo-dist copies every declared output basename into `target/distrib`.

It includes those basenames in the release manifest and upload list.

The output paths themselves cannot encode a dynamically discovered package version.

Stable release-asset filenames can still contain Debian metadata with the real version.

## Existing release verification

`.github/build-setup.yml` builds the real release WASM before every native CLI build.

It checks the WASM magic header and exports the release-enforcement environment flag.

`crates/lisa-cli/build.rs` rejects absent or invalid WASM when that flag is present.

`scripts/verify-musl-release.sh` runs after each Linux local build.

It checks for a static ELF with no interpreter.

It checks embedded WASM bytes and the expected managed-runtime checksum.

It mounts the x86_64 executable into Debian bullseye and checks `lisa --version`.

The verifier currently accepts one target argument, so each native Linux matrix row is independent.

No Debian-package builder, nFPM config, package verifier, or `.deb` fixture exists.

No release workflow step installs nFPM.

## nFPM interface

nFPM is a single Go binary and does not require Ruby or a packaging toolchain.

Its YAML config requires a name, architecture, version, maintainer, and contents.

Environment variables are expanded in scalar config fields.

The `recommends` array maps to Debian's Recommends control field.

The `contents` array maps a staged source file to its absolute installed destination.

File mode can be declared with `file_info.mode`.

`nfpm package --packager deb --config <file> --target <file>` writes one package.

nFPM 2.47.0 publishes static Linux x86_64 and arm64 release archives.

It also publishes a checksum manifest.

The cargo-dist global job always runs on Ubuntu x86_64 in the current plan.

A pinned Linux x86_64 nFPM binary is sufficient for release CI.

Using a pinned archive checksum avoids trusting an unversioned bootstrap download.

## CLI package inputs

The workspace version is in `[workspace.package]` in the root Cargo.toml.

The current version is `0.4.0-rc.8`.

The cargo package metadata supplies the MIT license, homepage, repository, and maintainer.

Cargo metadata can expose the exact version without parsing TOML with shell expressions.

Cargo-dist downloads both Linux archives before the global artifact build starts.

The packaging step can extract the single `lisa` executable from each archive.

The source archive is static musl and already passed the local artifact verification step.

The Debian package does not need libc dependencies for Lisa itself.

Doctor now requires Git as a runtime tool.

A clean slim Debian base does not guarantee Git is installed.

Agent clients remain user-selected external tools and cannot both be package dependencies.

## Managed Zellij source of truth

`crates/lisa-cli/src/runtime.rs` pins managed Zellij to 0.43.1.

`crates/lisa-cli/data/managed-runtime-sha256.json` is embedded at compile time.

It contains immutable URLs and compressed-archive SHA-256 values.

The Linux target records use the upstream no-web static-musl archives.

The x86_64 target maps to Debian `amd64`.

The aarch64 target maps to Debian `arm64`.

The runtime installer downloads exactly one archive on a managed cache miss.

It hashes the compressed bytes before extraction.

It accepts exactly one top-level regular file named `zellij`.

The Debian builder can consume the same JSON rather than duplicate URLs or hashes.

The manifest is retained in release builds even though its data is compile-time embedded.

`jq` and standard archive tools are available on the Ubuntu global runner.

## Runtime resolution today

`crates/lisa-cli/src/runtime.rs` owns all Zellij selection and inspection.

`ZellijRuntimeRequest` has Managed, System, and Pinned variants.

`ZellijRuntimeMode` has corresponding Managed, System, and Pinned provenance.

The mode's Display implementation supplies the word shown by doctor.

Managed mode derives a versioned XDG or HOME data path.

It downloads and atomically installs Zellij if that path is not executable.

System mode searches PATH in order.

Pinned mode uses the configured path exactly.

Every selected path is canonicalized and then queried with `--version`.

Unsupported, unparseable, or failing binaries are rejected.

There is no current lookup at `/usr/libexec/lisa/zellij`.

There is no provenance variant for a distribution-installed companion runtime.

## Resolver test seams

`RuntimeEnvironment` already injects PATH, XDG_DATA_HOME, and HOME into resolver tests.

The production constructor copies those fields from the process environment.

Unit tests create executable shell stubs with controlled version output.

They cover managed versus system selection, explicit pinned precedence, and support failures.

The hard-coded package location is not currently an injected field.

Adding it to `RuntimeEnvironment` permits a temporary-path unit test without root access.

Explicit System and Pinned requests have established intent semantics.

A packaged runtime preference must not silently override either explicit request.

Managed is the default request and the acquisition-owning request.

It is the natural branch in which an installed companion can avoid network acquisition.

## Doctor reporting

`crates/lisa-cli/src/doctor.rs` calls the production resolver with resolved config.

Successful resolution is converted by `resolved_zellij_check`.

Its version string includes mode, version, supported range, and canonical path.

Doctor therefore reports every enum mode through the shared Display implementation.

The existing doctor test iterates all three runtime modes.

Extending that table will directly assert the new provenance wording and libexec path.

Doctor also checks Git, the selected agent, embedded WASM, and optional Rust WASM support.

A package integration test must provide a supported agent executable to make doctor fully green.

It must also use a real release-built Lisa, because ordinary developer builds may embed empty WASM.

## Installation acceptance environment

Docker is available on the development host.

Debian bookworm containers provide `dpkg`, `apt`, and package metadata inspection.

`apt install ./one.deb ./two.deb` processes local packages and resolves declared dependencies.

Both architecture packages can be structurally inspected on one host with `dpkg-deb`.

Only the native architecture pair can be executed in a normal container without emulation.

The release pipeline's x86_64 global runner can run the amd64 pair in bookworm.

A clean-room verifier can disable the network after a one-time base-image and package setup.

The decisive offline assertion is that doctor selects `/usr/libexec/lisa/zellij` before managed fetch.

Loop-time zero network follows from using that same resolved runtime path.

## Repository state and ownership constraints

The ordinary worktree contains Lisa-managed modified and untracked project-management files.

Those files predate this ticket and are not ticket-owned.

The ticket must use exact-path isolated commits through `lisa commit-ticket`.

Phase artifacts belong only in the current attempt work directory.

Lisa will publish admitted artifacts to the shared work directory later.

Ticket phase and status frontmatter must remain untouched.

Every source file created or changed by this ticket must be included in an isolated commit.

Generated `.deb` files and temporary extracted tools must not remain in the worktree.

## Constraints surfaced by the map

The package version must come from the same checkout used for cargo-dist archives.

The runtime bytes and checksum must come from the existing pinned manifest.

The four fixed extra-artifact basenames must remain stable across version bumps.

The release workflow must fail before hosting if package construction or clean install fails.

Package validation must cover control metadata, file ownership, modes, architectures, and doctor output.

Managed cache behavior and explicit runtime requests must remain compatible.

The signed apt repository and its credentials remain outside this ticket.
