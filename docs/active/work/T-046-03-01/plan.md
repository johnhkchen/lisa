# T-046-03-01 Plan

## Execution principles

Implement in three source units that each leave a coherent repository state.

Use only exact ticket-owned paths in commit transactions.

Do not stage through the ordinary Git index.

Record completed checks and deviations in `progress.md` as they occur.

Preserve unrelated worktree state throughout.

## Step 1: capture the clean ownership baseline

Inspect status for every planned source path.

Confirm none is already modified, staged, or untracked by another ticket.

Record the current HEAD.

Record the pinned dist version used for plan verification.

Do not include Lisa bookkeeping paths in ticket ownership.

Verification:

- planned source paths have no pre-existing changes;
- ordinary staged state remains untouched;
- `dist 0.30.4` is available from its official artifact.

## Step 2: switch the release and install surface

Edit `dist-workspace.toml`.

Replace x86_64 GNU Linux with x86_64 musl Linux.

Replace arm64 GNU Linux with arm64 musl Linux.

Change install path to `~/.local/bin`.

Add the selected Lisa success message.

Edit `aur/PKGBUILD`.

Change both target-specific archive URLs to musl.

Remove the direct `gcc-libs` dependency.

Verification:

- TOML parses through `dist plan`;
- the plan contains exactly both requested musl archives;
- the plan contains no GNU Linux archive;
- the plan retains both Darwin archives;
- the matrix assigns x86_64 musl to an x86_64 Ubuntu runner;
- the matrix assigns arm64 musl to an arm64 Ubuntu runner;
- the plan retains shell and Homebrew global artifacts;
- `rg` finds no stale GNU archive URL in `aur/PKGBUILD`.

## Step 3: inspect generated installer behavior

Use pinned dist to generate or build the global shell installer without publishing.

If the global build requires local artifact manifests, use plan JSON or a controlled
fixture rather than editing generated release output by hand.

Inspect the emitted script for the configured home-relative destination.

Inspect it for the Lisa success message.

Inspect it for restart/source PATH guidance.

Run the script against local fake archives only if cargo-dist offers an offline-safe
fixture boundary; do not point it at a live release with mismatched rc.8 artifacts.

Verification:

- installer source resolves `$HOME/.local/bin`;
- installer source does not select CargoHome layout;
- installer source contains `lisa doctor` guidance;
- generic source/restart guidance remains present.

If exact generated-script execution is infeasible without a release manifest,
record source inspection plus pinned upstream template evidence as the limitation.

## Step 4: commit release and install surface

Run:

`lisa commit-ticket --ticket-id T-046-03-01 --message "Ship static musl Linux artifacts" --include dist-workspace.toml --include aur/PKGBUILD`

Use the repository's actual Lisa binary resolution for the command.

Verify HEAD advances once.

Verify both paths are clean afterward.

Verify no unrelated path appears in the commit.

## Step 5: add the managed-runtime checksum manifest

Create `crates/lisa-cli/data/managed-runtime-sha256.json`.

Enter version `0.43.1`.

Enter official no-web archive names for both Darwin architectures.

Enter official no-web static-musl archive names for both Linux architectures.

Enter hashes from each release's official `.sha256sum` file.

Keep target ordering aligned with `dist-workspace.toml`.

Do not fetch any checksum in production code.

## Step 6: embed and validate the manifest

Edit `crates/lisa-cli/src/runtime.rs`.

Add the public include-based manifest constant beside the managed version.

Document the checksum subject as the unpacked executable.

Add a test that parses the embedded string.

Compare the manifest version to `MANAGED_ZELLIJ_VERSION`.

Assert four unique target entries.

Assert exact expected target coverage.

Assert archive naming and target correspondence.

Assert lowercase 64-digit SHA-256 formatting.

Verification:

- `cargo fmt --check` passes;
- the focused runtime manifest test passes;
- all existing runtime resolver tests pass;
- no lockfile diff appears;
- no production network access was introduced.

## Step 7: commit the managed-runtime manifest unit

Run:

`lisa commit-ticket --ticket-id T-046-03-01 --message "Bake managed runtime checksums" --include crates/lisa-cli/data/managed-runtime-sha256.json --include crates/lisa-cli/src/runtime.rs`

Verify the commit contains exactly the two named paths.

Verify both paths are clean afterward.

## Step 8: harden the CLI build script

Edit `crates/lisa-cli/build.rs`.

Declare rerun behavior for `LISA_REQUIRE_EMBEDDED_WASM`.

Read the source WASM when it exists.

Validate non-zero content and the full eight-byte WASM header.

Copy validated bytes to `OUT_DIR`.

If the source is missing and the requirement flag is enabled, panic clearly.

If it is missing without the flag, preserve the zero-byte developer placeholder.

Keep the workspace-root derivation unchanged.

Verification:

- a CLI build with the current valid WASM succeeds;
- an isolated build with the source unavailable still permits developer mode;
- a required build with the source unavailable fails with the named message;
- an existing invalid WASM fails rather than embedding junk.

Avoid mutating or deleting the repository's real WASM during negative tests.

Use a temporary copied fixture crate or unitized helper when necessary.

## Step 9: enforce WASM setup in CI

Edit `.github/build-setup.yml`.

Retain the install-target and build steps.

Add a bash verification step.

Assert the release WASM is non-empty.

Assert its exact eight-byte module prefix.

Append the release requirement flag to `$GITHUB_ENV`.

Verification:

- YAML remains valid;
- the shell fragment passes against the current release WASM;
- the fragment fails against an empty temporary file;
- the environment export uses GitHub's job-scoped mechanism.

## Step 10: add artifact verification script

Create `scripts/verify-musl-release.sh`.

Validate arguments and distribution directory.

Discover a single target-specific tar.xz archive.

Extract it into a temporary directory with cleanup.

Discover a single `lisa` executable.

Require static ELF evidence from `file`.

Reject an ELF interpreter with `readelf`.

Require a static/non-dynamic `ldd` result.

Require embedded WASM magic in the native binary.

Require a known managed-runtime checksum string in the binary.

Execute `lisa --version` in a native Debian bullseye container.

Make the file executable through the patch/tooling path.

Verification:

- `bash -n scripts/verify-musl-release.sh` passes;
- unsupported target input fails;
- missing archive input fails by name;
- a real built archive passes when a Docker daemon is available;
- local Docker unavailability is reported honestly.

## Step 11: wire the release workflow

Edit `.github/workflows/release.yml` narrowly.

Insert the verifier after the `dist build` step.

Condition it on a musl Linux matrix target.

Pass the joined target value as the script argument.

Keep post-build upload collection after verification.

Verification:

- YAML parses;
- step ordering is build, verify, post-build;
- Darwin jobs skip the verifier;
- both musl matrix jobs run it;
- workflow dispatch modifications remain byte-for-byte outside the insertion.

## Step 12: prove fat-LTO linking as far as the environment allows

Install both Rust musl standard-library targets if absent.

Attempt a dist-profile musl build using the same Cargo profile.

Prefer native or cargo-dist-selected tooling over ad hoc profile changes.

Do not disable LTO to make a build pass.

Do not alter optimization or codegen settings.

If local macOS lacks a Linux linker, use a native Linux runner/container if available.

Record the exact linker result.

The committed CI verification remains mandatory for both native runner legs.

Verification:

- a successful build emits the target-specific Lisa executable;
- failure, if any, is identified as environment/toolchain rather than hidden;
- no workaround weakens fat LTO.

## Step 13: commit the release-proof unit

Run one exact-path transaction containing:

- `crates/lisa-cli/build.rs`;
- `.github/build-setup.yml`;
- `scripts/verify-musl-release.sh`;
- `.github/workflows/release.yml`.

Use message `Verify portable Linux release artifacts`.

Verify the commit contains exactly those four paths.

Verify all four paths are clean afterward.

## Step 14: full verification

Run pinned `dist plan` again from committed HEAD.

Run `cargo fmt --check`.

Run `cargo test --workspace`.

Run `cargo build -p lisa-plugin --target wasm32-wasip1 --release`.

Run `cargo clippy --workspace --all-targets -- -D warnings` if time and host allow.

Run `bash -n` on the release verifier.

Parse both modified YAML files.

Inspect `git diff` and ticket-owned status.

Confirm no ticket-owned source remains modified, staged, or untracked.

## Step 15: review handoff

Update `progress.md` with every command and result.

Write `review.md` with file changes, evidence, gaps, and concerns.

Write `review-disposition.json` with the exact required shape.

Use pass only if source is committed and deterministic checks are green.

Use block with an actionable reason if a required acceptance boundary cannot be met.

Do not update ticket frontmatter.

Do not publish a release.

Do not start T-046-03-02.
