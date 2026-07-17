# Structure — T-046-05-02 signed apt repository and README

## Change units

The implementation is divided into four ticket-owned source units.

The first unit establishes the archive key and repository builder.

The second unit adds end-to-end repository verification.

The third unit connects stable release publication to GitHub Pages.

The fourth unit documents client installation and operator key handling.

Each unit can be committed through Lisa's isolated transaction with exact paths.

No Rust crate or Cargo dependency changes.

No existing package definition changes.

No ticket or shared work-artifact path is part of an implementation commit.

## Created file: `packaging/apt/lisa-archive-keyring.asc`

This file contains the ASCII-armored public half of the Lisa apt archive key.

It is safe and necessary to distribute publicly.

It contains one OpenPGP public key.

It does not contain a private-key packet.

Its fingerprint is the stable identity used by CI and operator documentation.

The Pages deployment copies this file to its root without transformation.

The README downloads this exact public artifact.

The file is not a globally trusted keyring by itself.

Clients dearmor it into a dedicated binary keyring.

## Created file: `packaging/apt/README.md`

This file is the archive-operator runbook.

It records the selected GitHub Pages topology.

It records the production key fingerprint and public-key path.

It explains that the private key is stored only as `APT_SIGNING_KEY`.

It includes a reproducible one-time key-generation procedure.

It includes the exact secret-provisioning command.

It describes the workflow's temporary GnuPG home.

It describes public/private fingerprint validation.

It describes stable-only release selection.

It describes how the site is reconstructed from GitHub Releases.

It describes key rotation as an explicit compatibility event.

It records Pages size and bandwidth limits.

It notes Cloudsmith as a future migration target if those limits become material.

It never includes private key bytes or an example that could be mistaken for production material.

## Created file: `scripts/build-apt-repository.sh`

This executable is the deterministic repository builder.

### Command interface

The positional interface is:

```text
scripts/build-apt-repository.sh DEB_INPUT_DIR OUTPUT_DIR SIGNING_FINGERPRINT PUBLIC_KEY
```

`DEB_INPUT_DIR` may contain nested release-specific subdirectories.

The builder recursively discovers files ending in `.deb`.

`OUTPUT_DIR` is replaced with the complete generated site.

`SIGNING_FINGERPRINT` is the full secret-key fingerprint used for both signatures.

`PUBLIC_KEY` is the armored key copied to the site root.

`GNUPGHOME` selects the caller-owned keyring.

### Required tools

The script checks for `apt-ftparchive`.

It checks for `dpkg-deb`, `gpg`, `gzip`, `sha256sum`, `find`, and `sort`.

It fails with a focused diagnostic when a tool is absent.

### Input validation

The input directory must exist.

At least one Debian package must be found.

The output path must not be the input path or nested inside it.

The public key must exist and contain the configured fingerprint.

The GnuPG home must contain the corresponding signing key.

Accepted package names are exactly `lisa` and `lisa-runtime-zellij`.

Accepted architectures are exactly `amd64` and `arm64`.

Package versions must be nonempty and path-safe.

### Pool assembly

The output pool is `pool/main/l/lisa/`.

Each output basename is `<Package>_<Version>_<Architecture>.deb`.

The builder compares checksums before accepting a duplicate identity.

An identical duplicate is ignored.

A conflicting duplicate fails.

The package files retain their input bytes.

### Index assembly

The amd64 index lives at `dists/stable/main/binary-amd64/Packages`.

The arm64 index lives at `dists/stable/main/binary-arm64/Packages`.

Each uncompressed index is generated with the matching architecture filter.

Each index also has a deterministic `Packages.gz` copy.

The release file lives at `dists/stable/Release`.

Its metadata names Lisa as Origin and Label.

Its Suite and Codename are both `stable`.

Its Components field is `main`.

Its Architectures field is `amd64 arm64`.

It includes apt-ftparchive-generated checksums for repository indexes.

### Signing and public site

`dists/stable/InRelease` is an armored clear signature.

`dists/stable/Release.gpg` is an armored detached signature.

Both select the exact configured fingerprint.

Both use batch mode and fail on signing errors.

The armored public key is copied to `/lisa-archive-keyring.asc` in the site.

A `.nojekyll` marker is created at the site root.

No private key or GnuPG home path enters the output.

## Created file: `scripts/verify-apt-repository.sh`

This executable is the integration verifier.

### Command interface

It accepts an optional Debian artifact directory.

The default is `target/distrib` under the repository root.

It expects the real amd64 package pair in that directory.

The arm64 index shape is validated by the builder separately from executable testing.

### Test-tool container

The verifier creates a temporary host directory.

It starts a Debian bookworm tool container with that directory mounted.

It installs `apt-utils` and `gnupg` inside the tool container.

It creates an ephemeral unencrypted archive key.

It exports the ephemeral public key.

It records the full ephemeral fingerprint.

It unpacks the real amd64 package pair.

It replaces only their Debian Version fields with a fixed lower test version.

It rebuilds those directories into valid older `.deb` files.

It retains the original current packages unchanged.

### Initial repository

The verifier invokes the production builder inside the tool container.

The first repository contains the older amd64 pair.

To keep both declared architectures represented, it also creates old arm64 metadata inputs from the real arm64 pair.

The generated Release signatures are checked with GnuPG.

The verifier asserts that no private key material exists beneath the site output.

### Clean bookworm client

The verifier creates a separate `debian:bookworm-slim` client container.

It mounts the generated repository read-only at `/repo`.

It copies the public key into `/usr/share/keyrings/lisa-archive-keyring.gpg`.

It writes a source entry with `signed-by`, `file:/repo`, `stable`, and `main`.

It runs apt update without a trusted-repository bypass.

It installs both package names from the signed local repository.

It asserts both installed versions equal the fixed old version.

### Upgrade repository

The tool container rebuilds the site from both old and current package pairs.

The client sees the replacement through its bind mount.

The client runs apt update again.

It runs apt upgrade non-interactively.

It asserts both installed versions equal their current package metadata.

It asserts apt's candidate versions equal the installed current versions.

### Doctor verification

The client creates the controlled supported Claude stub.

It creates an empty Lisa doctor project.

The verifier disconnects all client networks.

It runs `/usr/bin/lisa doctor` in the upgraded container.

It requires exit status zero.

It requires `mode packaged` and `/usr/libexec/lisa/zellij` in output.

It requires the satisfied-dependencies summary.

Both containers and the temporary directory are cleaned on exit.

## Modified file: `.github/workflows/release.yml`

The workflow retains the existing cargo-dist build and host topology.

The global job adds one focused signed-repository verification step after direct package verification.

That step calls `scripts/verify-apt-repository.sh`.

It therefore runs for pull requests without a production signing secret.

### New `publish-apt-repository` job

The job needs `plan` and `host`.

It uses the same stable-release conditional shape as Homebrew publication.

It has job-local `contents: read`, `pages: write`, and `id-token: write` permissions.

It runs on Ubuntu.

It uses environment `github-pages`.

It exposes the deployment URL from the Pages action.

It uses a shared concurrency group named for apt publication.

It does not cancel an in-progress publish.

The job checks out the exact release ref.

It queries all GitHub Releases through `gh api --paginate`.

It filters out drafts and prereleases.

It downloads each release's four Debian patterns into a tag-specific directory.

Releases without Debian assets are skipped without hiding other errors.

It installs `apt-utils` and `gnupg`.

It imports `APT_SIGNING_KEY` through standard input into a temporary GnuPG home.

It validates exactly one secret fingerprint.

It validates that fingerprint against the checked-in public key.

It calls the production builder.

It configures Pages, uploads the generated site artifact, and deploys it.

The final `announce` job adds this job to `needs`.

Its condition accepts either apt success or a deliberate skip, matching Homebrew semantics.

## Modified file: `README.md`

Add a `### Debian and Ubuntu` subsection under `Install Lisa` after Homebrew.

It names the URL `https://johnhkchen.github.io/lisa`.

It installs key-download prerequisites.

It downloads `lisa-archive-keyring.asc` from the Pages origin.

It dearmors into `/usr/share/keyrings/lisa-archive-keyring.gpg`.

It writes `/etc/apt/sources.list.d/lisa.list` with explicit `signed-by` pinning.

It installs `lisa lisa-runtime-zellij` after apt update.

It tells the user to run `lisa doctor`.

It explains that ordinary apt update and upgrade move stable releases forward.

It identifies the companion runtime's private libexec role.

It links the operator key-handling runbook for maintainers.

It states that the channel is a vendor repository rather than the Debian archive.

It records the selected Pages operational limits without presenting them as package guarantees.

## Deleted files

No file is deleted.

## Ordering constraints

Create the signing key before finalizing public URLs and fingerprint documentation.

Implement and shell-check the builder before writing the integration verifier.

Run the verifier before wiring it into release CI.

Add Pages workflow publication only after repository output is locally validated.

Update README commands from the exact final public paths and source shape.

Provision the private-key secret only after its public half is committed and verified.

Enable the repository's Pages Actions source only after the workflow definition is ready.
