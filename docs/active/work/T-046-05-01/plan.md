# T-046-05-01 Plan

## Execution policy

Work through implementation and review without pausing for another phase transition.

Keep all phase artifacts in the current attempt directory.

Do not edit ticket phase or status.

Preserve every pre-existing modified or untracked file outside ticket ownership.

Use `apply_patch` for source changes.

Use exact repository-relative paths for every isolated ticket commit.

Never use ordinary `git add`, broad staging, or ordinary `git commit`.

## Step 1: add packaged-runtime provenance

Modify `crates/lisa-cli/src/runtime.rs`.

Add the production libexec path constant.

Add Packaged to the runtime mode enum and Display implementation.

Inject the packaged path through `RuntimeEnvironment`.

Initialize the production environment with `/usr/libexec/lisa/zellij`.

Keep test helper environments isolated from any real host package installation.

Change Managed resolution to prefer an executable packaged path.

Keep System and Pinned resolution unchanged.

Continue canonicalizing and inspecting the selected packaged binary.

Add a selection test with packaged, managed, and PATH candidates.

Assert packaged mode, packaged version, and canonical packaged path.

Verification:

- `cargo test -p lisa-cli runtime::tests::managed_mode_prefers_packaged_runtime`;
- existing runtime selection tests;
- formatting after the edit.

## Step 2: extend doctor's provenance contract

Modify `crates/lisa-cli/src/doctor.rs` only in the mode table test.

Add Packaged, its lowercase label, and `/usr/libexec/lisa/zellij`.

Let the existing shared assertions cover mode, version, range, path, and success.

Do not add doctor-specific resolution logic.

Verification:

- `cargo test -p lisa-cli doctor::tests::test_runtime_report_names_mode_version_and_path_for_every_mode`;
- confirm the test fails to compile if a future mode is not handled by Display.

## Step 3: define the CLI Debian package

Create `packaging/nfpm/lisa.yaml`.

Set package identity and Debian metadata.

Use environment variables for architecture, version, and staged binary.

Declare semver version normalization and release revision 1.

Declare Git as Depends.

Declare the companion runtime as Recommends.

Install only `/usr/bin/lisa` at mode 0755.

Verification:

- inspect the YAML manually against nFPM's schema;
- run nFPM package through the builder;
- inspect final control fields with `dpkg-deb -f`.

## Step 4: define the runtime Debian package

Create `packaging/nfpm/lisa-runtime-zellij.yaml`.

Use the same architecture, Lisa version, and release revision inputs.

Interpolate the pinned Zellij version into the description.

Install only `/usr/libexec/lisa/zellij` at mode 0755.

Do not provide or conflict with generic Zellij.

Verification:

- run nFPM package through the builder;
- inspect final control fields and archive table with `dpkg-deb`.

## Step 5: implement deterministic package assembly

Create `scripts/package-debs.sh` and make it executable.

Add strict-mode and repository-root setup.

Add the distribution-directory and version test seams.

Resolve the normal version through cargo metadata.

Pin nFPM 2.47.0 and official archive SHA-256 values.

Map Linux x86_64 for production and Darwin arm64 for local reproduction.

Download and authenticate nFPM before execution.

Create temporary staging and cleanup on every exit path.

For each Linux architecture, require the exact cargo-dist archive.

Extract and require exactly one Lisa binary.

Read the matching runtime record from the embedded manifest source file.

Download and hash the compressed runtime archive.

Inspect its member list and require exactly one top-level regular Zellij file.

Extract and chmod both staged executables.

Invoke the two nFPM configs for the architecture.

Write all four stable package outputs at repository root.

Verification:

- `bash -n scripts/package-debs.sh`;
- `shellcheck scripts/package-debs.sh`;
- a missing-archive run fails with a named exact path;
- a fixture/full run creates exactly four nonempty packages;
- rerunning overwrites the four outputs successfully;
- temporary directories are absent after completion.

## Step 6: implement package acceptance verification

Create `scripts/verify-deb-release.sh` and make it executable.

Add strict-mode, dependency, package-presence, and cleanup guards.

Add helpers for package field lookup and assertions.

Check both package names and both Debian architectures.

Check one identical Debian version across the four files.

Check CLI Depends and Recommends.

Check exact installed paths and executable modes.

Create and start a clean Debian bookworm-slim container.

Install both amd64 packages through local apt paths.

Allow install-time network only for declared Debian dependencies.

Create a supported controlled Claude executable.

Disconnect the network before doctor.

Run doctor, capture its status and output, and fail on nonzero.

Assert packaged provenance, exact libexec path, and satisfied summary.

Run packaged Zellij's version command.

Verification:

- `bash -n scripts/verify-deb-release.sh`;
- `shellcheck scripts/verify-deb-release.sh`;
- execute it against full amd64 and arm64 package outputs where Docker is available;
- manually inspect failure cleanup by checking no named verifier container remains.

## Step 7: register cargo-dist extra artifacts

Modify `dist-workspace.toml`.

Add one `[[dist.extra-artifacts]]` block.

Point build at the package assembly script.

Declare the four fixed root output paths.

Keep existing Homebrew publication configuration unchanged.

Verification:

- run pinned cargo-dist 0.30.4 `plan --output-format=json`;
- assert all four `.deb` names occur in the Lisa release artifacts;
- assert the four native target archives remain planned;
- assert Homebrew and shell installer artifacts remain planned;
- inspect the matrix remains four native rows.

## Step 8: gate release upload on Debian acceptance

Modify `.github/workflows/release.yml`.

Add one Bash step after global `dist build` and before upload-path calculation.

Run the package verifier there.

Do not change host, Homebrew publish, announce, permissions, or triggers.

Verification:

- parse workflow YAML with an available YAML parser;
- inspect ordering around global build, verifier, post-build, and upload;
- regenerate or diff with pinned cargo-dist only as a diagnostic because intentional CI edits are allowed dirty;
- confirm `dist plan` still succeeds with the retained workflow.

## Step 9: focused Rust verification

Run `cargo fmt --all -- --check` after formatting.

Run the complete runtime unit-test module.

Run the complete doctor unit-test module.

Run `cargo test -p lisa-cli`.

Inspect failures before broadening scope.

Do not edit unrelated code to silence pre-existing failures.

## Step 10: package and container verification

Prepare real cargo-dist-shaped Linux archives if not already present.

Prefer building the real release artifacts because doctor checks embedded WASM.

Run package assembly.

Run the Debian verifier.

Record the exact package control versions and doctor provenance output in progress.

Remove root `.deb` outputs after verification if cargo-dist did not move/copy them.

Do not commit generated package binaries.

## Step 11: workspace regression verification

Run `cargo test --workspace`.

Run `just check` if the required WASM target and toolchain support it.

Run shell syntax and ShellCheck for both new scripts again.

Run the pinned cargo-dist plan assertion again from the final tree.

Inspect `git diff --check`.

Inspect ordinary index and worktree status.

Distinguish pre-existing Lisa-managed dirt from ticket-owned paths.

## Step 12: isolated commit for resolver behavior

Once focused tests pass, commit the runtime unit as one meaningful source unit:

`lisa commit-ticket --ticket-id T-046-05-01 --message "Prefer packaged Zellij runtime" --include crates/lisa-cli/src/runtime.rs --include crates/lisa-cli/src/doctor.rs`

Use the available installed or repository-built Lisa executable.

Confirm the commit contains exactly those two paths.

Confirm neither path remains staged, modified, or untracked afterward.

## Step 13: isolated commit for Debian release packaging

Once packaging and release verification pass, commit the release unit:

`lisa commit-ticket --ticket-id T-046-05-01 --message "Package Debian release artifacts" --include dist-workspace.toml --include .github/workflows/release.yml --include packaging/nfpm/lisa.yaml --include packaging/nfpm/lisa-runtime-zellij.yaml --include scripts/package-debs.sh --include scripts/verify-deb-release.sh`

Confirm the commit contains exactly those six paths.

Confirm none remains staged, modified, or untracked afterward.

Do not include generated `.deb` files.

## Step 14: final reconciliation

Update `progress.md` throughout implementation with completed work and deviations.

After both commits, inspect `git log` and `git show --stat`.

Run final focused tests against committed HEAD.

Run `git status --short` and document unrelated pre-existing entries.

Ensure every ticket-owned source path is clean.

Ensure phase artifacts remain private and uncommitted by ticket commits.

## Step 15: Review

Read both ticket commits and final source, not only the working diff.

Check acceptance criteria against test evidence.

Describe files, behavior, package metadata, CI ordering, and provenance semantics.

Report exact commands and outcomes.

Call out any verification that could only run in release CI.

Write `review.md` in the attempt directory.

Write the exact valid `review-disposition.json` shape.

Use pass only if source is committed, focused tests pass, and no actionable blocker remains.

Remain on T-046-05-01 after Review and wait for Lisa's completion commit.
