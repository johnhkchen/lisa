# T-046-03-01 Design

## Goals

Make one Linux release artifact run independently of the host's glibc release.

Make that artifact the shell installer's Linux selection, not merely a fallback.

Install Lisa under the user's ordinary local binary directory.

Leave a short, direct message explaining the fresh-shell PATH boundary.

Keep the embedded plugin non-empty in every CI-built native artifact.

Create the baked checksum input required by managed-runtime acquisition.

Make fat-LTO and static-link claims observable in the release pipeline.

Preserve Darwin releases, Homebrew publishing, and workflow dispatch behavior.

## Decision 1: Linux target set

### Option A: add musl and retain both GNU targets

This would produce six platform archives.

It would preserve target-specific GNU URLs for direct consumers.

It would also give the installer a musl fallback on old systems.

Cargo-dist's preference ordering makes GNU host-native on GNU Linux.

Static musl is deliberately ranked below that as imperfect-native.

The shell installer would therefore still serve GNU on compatible new hosts.

That conflicts with the ticket's requirement that musl be what it serves.

It also leaves two Linux artifact classes with different runtime floors.

Option A is rejected.

### Option B: retain both and patch installer selection

A generated installer could theoretically be rewritten to prefer musl.

Cargo-dist has no project setting for inverting its platform quality ordering.

The generated script is a global artifact composed from release manifests.

Patching it would couple Lisa to an internal v0.30.4 template shape.

That patch would need regeneration-aware tests and ongoing maintenance.

It would retain GNU archives without a demonstrated consumer need.

Option B is rejected.

### Option C: replace GNU targets with musl targets

This leaves four release targets overall.

The Darwin targets remain unchanged.

Each Linux architecture has exactly one static archive candidate.

Cargo-dist maps static-musl archives as compatible with GNU Linux hosts.

The shell installer therefore selects musl for normal Linux installs.

There is no glibc runtime condition to gate that candidate.

The plan and release artifact list remain easy to audit.

Option C is chosen.

## Decision 2: other target-specific consumers

The AUR recipe currently hardcodes GNU archive URLs.

Leaving those URLs unchanged would knowingly break the next published package.

The recipe will switch both URL triples to musl.

Its explicit `gcc-libs` dependency describes the old dynamic Lisa artifact.

The independently packaged Zellij dependency owns its own runtime dependencies.

The static Lisa binary no longer needs `gcc-libs` directly.

The recipe will retain `zellij` and remove direct `gcc-libs`.

No Nix or source-build configuration needs a Linux archive rename.

## Decision 3: install location

### Option A: retain `CARGO_HOME`

This is cargo-dist's historical default.

It often points to a directory already present on developer PATH values.

On a Rust-less machine it creates a `.cargo` namespace solely for Lisa.

That is the exact user-facing implication this ticket removes.

Option A is rejected.

### Option B: custom wrapper installer

A Lisa-owned wrapper could download the cargo-dist script or archives.

It could print arbitrary messages and avoid generated path behavior.

It would duplicate platform selection, checksum, extraction, and receipt logic.

It would also create another public release artifact to maintain.

The pinned tool already supports the required home-relative path.

Option B is rejected.

### Option C: configure `~/.local/bin`

Cargo-dist accepts this value directly.

The generated installer creates the directory and edits shell profiles.

It writes no `.cargo` directory for the flat installation strategy.

It prints explicit source-or-restart guidance when profile activation is needed.

`~/.local/bin` matches the managed runtime's broader XDG-style directory choice.

Option C is chosen.

## Decision 4: PATH copy

Cargo-dist's generated remediation is technically correct but generic.

Its `install-success-msg` setting is an intended branding seam.

The message will say where Lisa landed and what a current-shell user should do.

The phrasing will be short, verb-forward, and free of Rust terminology.

Chosen copy:

`Lisa is ready in ~/.local/bin. Open a new shell, then run lisa doctor.`

This is plain enough for a person or an agent to act on.

The generated installer may additionally print exact `source` commands.

Those details complement rather than contradict the success message.

The source text can be asserted from the generated installer.

## Decision 5: managed-runtime checksum manifest

### Storage alternatives

Hardcoding four constants in `runtime.rs` is simple but awkward to audit.

A Rust match table would be immediately consumable but mix data and fetch logic.

A checked-in JSON manifest is language-neutral and release-reviewable.

It can be embedded with `include_str!` without build-time network access.

JSON also matches the crate's existing serde and serde_json dependencies.

The JSON manifest is chosen.

### Manifest shape

The top level will carry one explicit Zellij version.

It will carry an `artifacts` array rather than target-named object keys.

Each entry will include `target`, `archive`, and `sha256`.

Targets will match Rust target triples and dist target vocabulary.

Archive names will be exact official no-web release asset names.

Hashes will be the official upstream hashes of the unpacked Zellij executable.

The future installer can download, unpack into a temporary directory,
verify the executable, and only then rename the completed runtime directory.

This order is compatible with T-046-02-02's atomic-install requirement.

The manifest will include both Darwin and Linux release architectures.

Linux entries use the required static-musl assets.

Darwin entries keep managed mode viable on Lisa's existing Darwin releases.

The manifest file contains no URL base.

The dependent downloader can derive the immutable versioned release URL.

### Compile-time boundary

`runtime.rs` will expose the manifest text as a public crate constant.

The constant will use `include_str!` against the checked-in data file.

Unit tests will parse the embedded bytes, not reopen a working-tree path.

They will verify the pinned runtime version matches the manifest version.

They will verify all four supported release targets are unique and present.

They will verify archive naming and 64-character lowercase hexadecimal hashes.

This creates a stable handoff for T-046-02-02 without implementing its network path.

## Decision 6: WASM release enforcement

### Existing behavior to preserve

Developer CLI builds are permitted to use an empty placeholder.

That supports fast work on CLI-only changes.

The distribution pipeline first builds the actual release WASM.

The CLI build script then copies it into `OUT_DIR`.

### Enforcement mechanism

The build setup will assert the source is a non-empty regular file.

It will also assert the standard eight-byte WebAssembly header.

After validation it will export `LISA_REQUIRE_EMBEDDED_WASM=1` through GitHub's
job environment file.

`build.rs` will honor that flag during the following dist build.

When required, missing, empty, or invalid-header WASM will fail the CLI build.

When not required, a missing file will retain the developer placeholder path.

An existing empty or malformed file is never a useful developer artifact.

The build script can reject that case even outside CI.

This makes the embedding precondition explicit across every target matrix job.

## Decision 7: artifact-level Linux verification

### Why configuration-only checks are insufficient

A musl target name does not prove the final binary is statically linked.

`lto = true` does not prove the selected linker completes on both CI runners.

A successful WASM prebuild does not by itself inspect the packaged CLI.

The acceptance claims need checks after `dist build` creates the archive.

### Verification script

A repository script will accept one target triple.

It will locate exactly one matching `lisa-cli-<target>.tar.*` archive.

It will extract into an isolated temporary directory.

It will locate exactly one executable named `lisa`.

It will use `file` to require a statically linked ELF binary.

It will use `readelf -l` to reject a program interpreter segment.

It will use `ldd` and require the static/non-dynamic result.

It will scan the packaged binary for the WebAssembly magic header.

It will scan for a checksum unique to the baked runtime manifest.

It will run `lisa --version` inside `debian:bullseye-slim`.

The runner architecture matches each matrix target architecture.

The container invocation therefore needs no cross-architecture emulation.

### Workflow placement

The verification step belongs immediately after `dist build`.

It runs only when the matrix contains an `unknown-linux-musl` target.

The step precedes upload path collection and artifact upload.

Failure therefore prevents publication.

The completed `dist build` itself is the fat-LTO link observation.

The following static and bullseye checks prove the linked output's properties.

## Decision 8: generated workflow handling

The current workflow has a deliberate manual-dispatch extension.

Blind regeneration could overwrite or rearrange that supported customization.

The dynamic matrix already follows `dist plan`; target literals are not embedded.

Only one project-owned verification step needs insertion.

The workflow will be patched narrowly rather than regenerated wholesale.

`dist plan` remains the configuration validation authority.

## Test strategy

Run the pinned `dist plan` and assert both musl artifacts are present.

Inspect the plan matrix for native x86_64 and arm64 Ubuntu jobs.

Generate the global installer locally when feasible and inspect its copy.

Run the manifest parsing tests through the CLI test target.

Run `cargo fmt --check` after Rust changes.

Run the workspace test suite.

Build the release WASM and confirm its header and non-zero size.

Cross-build at least the host-architecture musl CLI with the dist profile.

Run the artifact verifier wherever a compatible Linux/Docker runner exists.

Treat unavailable local Docker as an environment limitation, not a false pass.

Leave the mandatory native-runner verifier in CI for both architectures.

## Commit units

The dist configuration, AUR URL alignment, and generated installer assertion form
one release-target/install-surface unit.

The managed-runtime manifest, embedding constant, and unit tests form one data
contract unit.

The WASM build enforcement, Linux verifier script, and workflow hook form one CI
release-proof unit.

Each unit will be committed through `lisa commit-ticket` with exact paths only.
