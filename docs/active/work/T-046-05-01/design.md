# T-046-05-01 Design

## Goals

Produce four installable Debian packages from every cargo-dist release checkout.

Reuse the already-built static Lisa archives instead of compiling Lisa again.

Reuse the existing pinned managed-runtime manifest instead of copying Zellij provenance.

Make the package pair sufficient to resolve Zellij without a first-run download.

Give doctor a stable, explicit provenance label for the companion runtime.

Fail release CI before hosting when package metadata, contents, or installation are wrong.

Keep signed apt hosting, credentials, and repository documentation in T-046-05-02.

## Decision 1: release integration point

### Option A: handwritten workflow job outside cargo-dist

A standalone job could download local Actions artifacts and upload `.deb` files with `gh`.

It would need to reconstruct cargo-dist's tag, publication, and prerelease gates.

The `.deb` files would not be members of the dist manifest.

The host job's failure ordering would not automatically include package construction.

It would duplicate scratch-storage and release-upload behavior.

This option is rejected.

### Option B: build packages in each Linux local job

Each native Linux row already has the matching Lisa archive.

It could produce two packages for its one architecture.

That duplicates the nFPM bootstrap and package validation across runners.

The local dist invocation does not plan global extra artifacts.

Attaching arbitrary outputs would require additional upload-path customization.

Cross-architecture control checks and the clean amd64 install would be split from assembly.

This option is rejected.

### Option C: cargo-dist global extra artifacts

The global job runs after both Linux local archives are present.

Cargo-dist 0.30.4 natively plans, copies, manifests, and uploads extra artifacts.

One build command can construct both package names for both architectures.

A failed command or missing declared output fails `dist build` before host.

The existing host and GitHub Release flow will treat `.deb` files like other assets.

This option is chosen.

The extra artifact names will be stable and version-independent:

- `lisa-amd64.deb`;
- `lisa-arm64.deb`;
- `lisa-runtime-zellij-amd64.deb`;
- `lisa-runtime-zellij-arm64.deb`.

Debian control metadata, not the release-asset basename, carries the exact release version.

Stable basenames avoid editing dist config on every workspace version bump.

## Decision 2: package builder organization

### Option A: inline YAML and shell in `release.yml`

Inline here-documents could create nFPM configs and stage files.

That would make local reproduction difficult.

It would also put source-of-truth package metadata inside generated CI YAML.

This option is rejected.

### Option B: one generic nFPM YAML with many environment switches

A single config could switch name, description, dependency fields, and destination.

It would reduce line count but hide the two packages' ownership contracts.

An incorrectly supplied environment variable could cross package contents.

This option is rejected.

### Option C: two declarative configs and one orchestration script

One config will describe `lisa`.

One config will describe `lisa-runtime-zellij`.

Both use environment expansion only for version, architecture, and staged source path.

A checked-in Bash script will acquire nFPM, stage inputs, and call each config twice.

The split keeps package identity reviewable while centralizing repetitive mechanics.

This option is chosen.

## Decision 3: nFPM bootstrap

The release will pin nFPM 2.47.0.

The global job is Linux x86_64, so its official Linux x86_64 archive is sufficient in CI.

The builder will download that immutable release URL into a temporary directory.

It will compare the archive to a checked-in expected SHA-256 before extraction.

It will execute the temporary binary without installing anything on the runner.

The temporary directory will be removed through a shell trap.

For local reproduction, the script will also recognize the host's official Darwin arm64 asset.

No unpinned `go install`, Homebrew state, or mutable `latest` URL will participate.

## Decision 4: release version source

### Option A: parse the Git tag

The workflow input and tag push formats include a leading `v` and may be namespaced.

Cargo-dist already resolves those formats against package metadata.

Duplicating that parser would create avoidable disagreement.

This option is rejected.

### Option B: parse Cargo.toml with grep or sed

The crate inherits the workspace version rather than writing a literal crate version.

Text parsing would be sensitive to formatting and comments.

This option is rejected.

### Option C: query cargo metadata

The global runner has Rust because it runs dist.

`cargo metadata --no-deps` exposes `lisa-cli`'s resolved version.

That is the same package version validated against the release tag by cargo-dist.

The script will permit an explicit `LISA_VERSION` only as a local test seam.

This option is chosen.

nFPM's semver version schema will translate Cargo prerelease ordering to Debian ordering.

Both package configs use release revision `1`.

The verifier will require both packages to carry the same nonempty version.

## Decision 5: CLI package metadata

The package name is `lisa` because that is the Debian install and command name.

It installs one executable at `/usr/bin/lisa` with mode 0755.

It declares `git` as a required dependency because doctor and loop require Git.

It declares `lisa-runtime-zellij` as a Recommendation.

It does not require Claude or Codex because Lisa supports either and users select one.

It uses the workspace maintainer, MIT license, and project homepage.

It belongs to Debian's `utils` section with optional priority.

## Decision 6: runtime package provenance and contents

The package name is `lisa-runtime-zellij`.

It installs one executable at `/usr/libexec/lisa/zellij` with mode 0755.

It does not place Zellij on PATH or claim the generic `zellij` package name.

The builder reads the Linux records from the checked-in managed-runtime JSON.

For each record it downloads the immutable URL and validates the compressed SHA-256.

It then extracts exactly the top-level `zellij` file into private staging.

The runtime package version matches Lisa's version, not Zellij's version.

The description names the pinned Zellij version for human inspection.

This keeps the two companion packages upgradeable as one Lisa release unit.

## Decision 7: resolver semantics

### Option A: check `/usr/libexec` before every request

This would make package installation override explicit `system` and absolute-path config.

It would violate the established meaning of those user choices.

This option is rejected.

### Option B: copy the package runtime into the managed cache

The resolver could seed the XDG cache from `/usr/libexec` and retain Managed mode.

That duplicates bytes, hides package provenance, and introduces filesystem writes.

Doctor could not distinguish a Debian package from a downloaded managed cache.

This option is rejected.

### Option C: prefer the package runtime inside Managed resolution

Managed is the default and already means Lisa chooses a verified runtime source.

When the libexec path is executable, it will be selected before cache lookup or download.

System and Pinned branches will continue honoring explicit user intent.

The selected executable still passes canonicalization and version inspection.

An installed but incompatible packaged runtime will fail closed as packaged provenance.

This option is chosen.

The new enum variant will be `Packaged` and Display as `packaged`.

That label describes provenance without hard-coding one repository host or package manager.

The production location is the constant `/usr/libexec/lisa/zellij`.

The test environment will inject a temporary equivalent path.

## Decision 8: resolver and doctor coverage

A Unix unit test will create packaged, managed-cache, and PATH stubs.

It will request Managed and assert the packaged mode, version, and canonical path win.

Existing tests continue proving explicit Pinned and System behavior.

The doctor mode-table test will add Packaged and the real libexec path.

That test proves doctor renders `mode packaged` and the exact selected path.

No doctor-only detection path will be added; doctor must consume production resolution.

## Decision 9: release package verification

A separate checked-in verifier will run after the global dist build.

It will use `dpkg-deb` to check all four package identities and architectures.

It will assert both package names use one version per architecture and one version globally.

It will inspect Recommends and Depends on the CLI package.

It will inspect installed paths and executable modes without executing arm64 on x86_64.

For amd64 it will create a clean Debian bookworm container.

The container will install both local packages through apt.

Install time may use the network to obtain the declared Git dependency.

The verifier will install a controlled supported Claude stub because agent delivery is external.

It will then disconnect the container network before running doctor.

Doctor must exit zero and print `mode packaged` plus `/usr/libexec/lisa/zellij`.

Because managed resolution returns the packaged executable before acquisition,
this network-disabled success is the observable zero-fetch boundary.

The container will be removed through a trap on success or failure.

## Decision 10: workflow customization and future publishing

The global build step remains cargo-dist-owned.

One explicit verification step will follow it and precede artifact upload.

`allow-dirty = ["ci"]` already documents why custom workflow changes survive regeneration.

This ticket will not add a custom publish job.

T-046-05-02 owns the signed apt repository and will add its provider-specific publish job.

The `.deb` extra artifacts and manifest entries created here are that job's stable input.

## Failure policy

Missing cargo-dist archives fail with the expected exact path.

Unsupported builder hosts fail before downloading nFPM.

nFPM checksum mismatch fails before executing the downloaded tool.

Managed-runtime checksum mismatch fails before extracting package contents.

Unexpected archive contents fail package construction.

Missing any declared `.deb` fails the cargo-dist extra-artifact build.

Bad control metadata or file layout fails the verification step.

Failed apt installation, doctor exit, provenance, or path checks fail release CI.

No failure falls back to a network-managed Zellij during acceptance verification.

## Non-goals

This design does not publish to Cloudsmith or Pages.

It does not create GPG keys, apt metadata, or sources-list instructions.

It does not alter Homebrew, shell installer, AUR, or Nix behavior.

It does not make the companion runtime replace a general Zellij installation.

It does not package either supported agent client.
