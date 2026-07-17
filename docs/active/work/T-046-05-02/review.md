# Review — T-046-05-02 signed apt repository and README

## Disposition

Pass.

The implementation is merged into `main` and the first stable apt repository is live.

Release v0.4.3 exercised the repository builder, signed install/upgrade verifier,
release hosting, and GitHub Pages publication successfully.

The public key, signed metadata, both architecture indexes, and package payloads
are available over the README's HTTPS origin.

The ticket-owned source files are tracked and clean.

No source changes were required during this re-review.

## Summary of changes

The ticket adds a project-controlled Debian and Ubuntu vendor channel.

GitHub Pages was selected over the unprovisioned Cloudsmith option.

GitHub Releases remain the durable package store.

Every stable release reconstructs the Pages site from complete Debian package
sets attached to non-draft, non-prerelease releases.

The apt suite is `stable`, the component is `main`, and the supported
architectures are `amd64` and `arm64`.

Prereleases do not publish into the stable suite.

## Files created

### `packaging/apt/lisa-archive-keyring.asc`

Contains only the public Lisa Apt Archive signing key.

The independently resolved primary fingerprint is:

```text
8FB7DA4A79E109708C4457C2E7B9DBE079374202
```

### `packaging/apt/README.md`

Records the Pages-versus-Cloudsmith decision and capacity trade-offs.

Documents repository layout, signing identity, key generation, GitHub secret
provisioning, CI exposure, local verification, rotation, and recovery.

The production private export is held only in the `APT_SIGNING_KEY` repository
secret and is fingerprint-checked before use.

### `scripts/build-apt-repository.sh`

Validates the package name, architecture, version, and complete package matrix.

Copies uniquely named payloads into a Debian pool.

Builds deterministic `Packages` and `Packages.gz` indexes for both architectures.

Builds suite Release metadata and signs it as both `InRelease` and `Release.gpg`.

Verifies both signatures with a public-only keyring before publishing output.

### `scripts/verify-apt-repository.sh`

Creates an isolated ephemeral test key.

Repackages the real release payloads at a lower Debian version.

Installs the older package pair through a signed repository in clean bookworm.

Adds the current package pair, runs apt update and upgrade, and asserts that
both installed and candidate versions advance together.

Disconnects the client and verifies packaged Zellij resolution and doctor.

## Files modified

### `.github/workflows/release.yml`

Runs the signed apt install/upgrade verifier in the global release build.

Adds a stable-only `publish-apt-repository` job after GitHub Release hosting.

The job imports the private key into a temporary GnuPG home, requires one
primary secret key, and matches it to the checked-in public fingerprint.

The job has scoped `contents: read`, `pages: write`, and `id-token: write`
permissions and uses the official GitHub Pages actions.

The final announcement waits for apt publication success or an intentional
prerelease skip.

### `README.md`

Adds a Debian and Ubuntu section using a dedicated binary keyring.

The source is constrained with `signed-by`; deprecated global `apt-key` trust
is not used.

The commands install both `lisa` and `lisa-runtime-zellij` and then run doctor.

The copy explains normal apt upgrades, packaged runtime behavior, the vendor
repository boundary, Pages capacity, and the operator runbook.

## Commits

All ticket-owned source units were committed through `lisa commit-ticket` with
exact include paths:

```text
2c9fa57 Build signed apt repository metadata
e9df0c0 Verify signed apt installs and upgrades
96ae7e0 Publish stable apt channel to GitHub Pages
1c7691b Document the Debian apt channel
```

All four commits are ancestors of current `main`.

All four pass `git show --check`.

## Verification performed during re-review

### Public release and workflow

GitHub release v0.4.3 is stable, published, and carries the complete four-deb
asset set.

Release run `29551103151` completed successfully.

Its global build passed direct Debian package verification and the signed apt
install/upgrade test.

Its `publish-apt-repository` job built, uploaded, and deployed the Pages
artifact successfully.

### Public metadata and signatures

The README key URL returns HTTP 200.

The live `InRelease`, `Release`, `Release.gpg`, and amd64 package index are
available from the documented Pages origin.

Both live signature forms were independently verified with the downloaded
public key.

The live amd64 index contains both packages at `0.4.3-1` and its SHA-256 values
match the v0.4.3 GitHub Release assets.

### Clean bookworm public install

A fresh `debian:bookworm-slim` amd64 container followed the README's repository
setup using root-equivalent forms of the documented `sudo` commands.

APT accepted the live `InRelease` through the pinned keyring.

APT installed `lisa 0.4.3-1` and `lisa-runtime-zellij 0.4.3-1` from the public
HTTPS repository.

Doctor found Git, the embedded WASM plugin, and packaged Zellij 0.43.1 at
`/usr/libexec/lisa/zellij`.

As expected for a bare image, doctor reported the separately documented Claude
Code prerequisite as absent and exited 1. The release integration test supplies
a controlled agent executable and asserted a zero exit after disconnecting the
container from all networks. Agent delivery is outside the Debian package pair.

### Upgrade behavior

The v0.4.3 release integration test installed both packages at `0.0.0-1`,
regenerated signed metadata with the current packages, ran apt upgrade, and
confirmed that both packages and candidates advanced to `0.4.3-1`.

The public repository currently has one stable release, so a naturally
occurring public-channel upgrade can only be observed after the next stable
release. This is an operational follow-up, not an implementation blocker:
the exact repository builder and upgrade path already passed in release CI.

### Static checks

Both new shell scripts pass `bash -n` and ShellCheck.

The release workflow parses as YAML.

No literal private-key block exists in ticket-owned source.

All ticket-owned source paths are clean.

## Acceptance assessment

Repository metadata is GPG-signed in both supported apt forms.

The README pins trust to `/usr/share/keyrings/lisa-archive-keyring.gpg` with
`signed-by`.

CI private-key handling and operator custody are documented.

Public clean-bookworm installation of both packages succeeds.

Packaged runtime discovery succeeds without a Zellij network download.

Signed apt upgrade behavior is covered in the stable release workflow with real
package payloads and a lower installed version.

The implementation therefore satisfies the ticket's apt-channel boundary and
is ready for Lisa's completion commit.

## Open concerns

The first public repository contains only v0.4.3. Record a public-origin upgrade
observation during the next stable release audit.

The clean-bookworm doctor command requires an agent client in addition to the
two apt packages. README Prerequisites documents Claude Code as required; the
package verifier uses a controlled executable so it can isolate repository and
packaging behavior without installing an external agent product.

The Pages site remains subject to its documented 1 GB site and soft 100 GB per
month bandwidth limits. Cloudsmith is the documented migration target if either
limit becomes material.

Key rotation remains a compatibility event requiring an overlap window; the
operator runbook describes the required sequence.
