# Progress — T-046-05-02 signed apt repository and README

## Final status

Implementation is complete.

All ticket-owned source changes are committed through Lisa's isolated transaction.

All ticket-owned source paths are clean.

The production Actions secret is provisioned.

GitHub Pages is enabled with workflow deployments.

No stable release or release workflow was dispatched from this ticket.

The first subsequent stable release will create the initial public Pages deployment.

## Phase completion

- Completed repository and external-state Research.
- Evaluated Cloudsmith and GitHub Pages in Design.
- Selected GitHub Pages based on available authority and provider state.
- Defined exact file ownership and interfaces in Structure.
- Defined atomic implementation, test, commit, and provisioning steps in Plan.
- Implemented the four source units below.
- Completed final source, test, secret, Pages, and cleanliness checks.

## Unit 1: archive key and repository builder

Created `packaging/apt/lisa-archive-keyring.asc`.

The file contains one public RSA 3072 signing key.

The UID is `Lisa Apt Archive <john.hk.chen@gmail.com>`.

The full fingerprint is:

```text
8FB7DA4A79E109708C4457C2E7B9DBE079374202
```

No private-key packet or private-key block is present in repository source.

Created `scripts/build-apt-repository.sh`.

The builder recursively consumes release `.deb` files.

It accepts only `lisa` and `lisa-runtime-zellij`.

It accepts only `amd64` and `arm64`.

It requires all four package/architecture pairs.

It derives pool basenames from package control metadata.

It ignores identical duplicate identities.

It fails on different bytes for one package/version/architecture identity.

It creates separate Packages and deterministic Packages.gz indexes.

It creates explicit stable/main Release metadata.

It clear-signs `InRelease` and detach-signs `Release.gpg`.

It verifies both signatures with a separate public-only keyring before publication.

It publishes the armored public key and `.nojekyll` in the site root.

Committed as:

```text
2c9fa57 Build signed apt repository metadata
```

Exact committed paths:

- `packaging/apt/lisa-archive-keyring.asc`;
- `scripts/build-apt-repository.sh`.

## Unit 2: signed install and upgrade verifier

Created `scripts/verify-apt-repository.sh`.

The verifier uses the real four release packages.

It generates an ephemeral test-only signing key.

It repacks the package set at Debian version `0.0.0-1`.

It builds and verifies an initial signed repository.

It creates a clean amd64 Debian bookworm client.

The client installs a dedicated keyring and `signed-by` source.

It installs both old packages through apt.

The repository is rebuilt with old and current packages.

The same client runs apt update and apt upgrade.

It requires both packages' installed and candidate versions to become current.

The client is disconnected from every Docker network.

It requires doctor to report packaged provenance and the libexec runtime path.

It requires doctor to exit zero and report satisfied dependencies.

Committed as:

```text
e9df0c0 Verify signed apt installs and upgrades
```

Exact committed path:

- `scripts/verify-apt-repository.sh`.

## Unit 3: release publication

Modified `.github/workflows/release.yml`.

The global build now runs the signed repository verifier after direct package verification.

Added `publish-apt-repository` after the GitHub Release host job.

It skips prerelease announcements.

It serializes Pages publication without cancelling an active deployment.

It reads all non-draft, non-prerelease GitHub Releases.

Only releases carrying the complete four-package set contribute to the site.

It imports the private key into a temporary mode-0700 GnuPG home.

It requires exactly one primary secret key.

It requires an exact match to the checked-in public fingerprint.

It uses current official Pages action major versions at implementation time:

- `actions/configure-pages@v6`;
- `actions/upload-pages-artifact@v5`;
- `actions/deploy-pages@v5`.

The job has only contents-read, Pages-write, and OIDC-token permissions.

The final announcement now waits for apt publication success or intentional skip.

Committed as:

```text
96ae7e0 Publish stable apt channel to GitHub Pages
```

Exact committed path:

- `.github/workflows/release.yml`.

## Unit 4: client and operator documentation

Modified `README.md` with a Debian and Ubuntu installation section.

The commands install the armored public key as a dedicated binary keyring.

The source entry pins that exact keyring with `signed-by`.

The install command names both packages.

The README explains normal apt upgrades and the private packaged runtime.

It identifies the channel as a vendor repository.

It records the selected Pages capacity boundary.

Created `packaging/apt/README.md`.

The operator runbook records the hosting trade-off, repository topology, fingerprint,
private-key handling, secret provisioning, CI exposure, local verification, rotation,
and recovery behavior.

Committed as:

```text
1c7691b Document the Debian apt channel
```

Exact committed paths:

- `README.md`;
- `packaging/apt/README.md`.

## External provisioning

Generated the production key in a temporary GnuPG home inside Debian bookworm.

Discarded an initial unused key after correcting its UID to the existing maintainer email.

Validated the final public and private fingerprints before provisioning.

Streamed the final private export directly into repository secret `APT_SIGNING_KEY`.

Secret listing confirms it was updated at `2026-07-16T20:45:36Z`.

Enabled GitHub Pages for Actions workflow deployments.

The Pages API now reports:

```text
https://johnhkchen.github.io/lisa/    workflow
```

Removed the temporary plaintext private export and temporary GnuPG home.

The current key value is not present in a local file or repository source.

## Verification evidence

### Real release package production

Built current release-profile musl Lisa binaries natively for arm64 and under amd64 Docker.

Both builds required and embedded the nonempty release WASM plugin.

Assembled the real four nFPM packages for version `0.4.0-rc.8`.

The ignored local package outputs were:

- `lisa-amd64.deb`;
- `lisa-arm64.deb`;
- `lisa-runtime-zellij-amd64.deb`;
- `lisa-runtime-zellij-arm64.deb`.

### Builder tests

Built a four-package signed fixture repository.

Both architecture indexes contained both package names.

Release metadata named `amd64 arm64`, `stable`, and `main`.

Both signature forms verified using the public-only keyring.

A same-identity package with changed bytes failed with the expected collision diagnostic.

### Full apt integration

`scripts/verify-apt-repository.sh target/distrib` passed.

Bookworm installed both packages at `0.0.0-1` through the signed repository.

Bookworm upgraded both to `0.4.0~rc.8-1` through the regenerated repository.

Both apt candidates equaled the installed current versions.

After network removal, doctor reported:

```text
mode packaged
path /usr/libexec/lisa/zellij
All dependencies satisfied.
```

### Static checks

- `bash -n` passed for both new scripts.
- ShellCheck passed for both new scripts.
- Ruby YAML parsing passed for the release workflow.
- All four ticket commits passed `git show --check`.
- No literal private-key block exists in ticket-owned source.
- The public key independently resolved to the documented fingerprint.
- All ticket-owned source paths are tracked and clean.
- Generated package outputs are ignored by `/target`.

Actionlint continues to report only pre-existing ShellCheck diagnostics in cargo-dist-owned
workflow blocks at lines outside the new apt job. The new job has no Actionlint diagnostic.

The pre-existing `scripts/verify-deb-release.sh` could not be launched directly from the
macOS host because `dpkg-deb` is unavailable there. Its stricter relevant package, install,
doctor, and network boundaries were exercised by the new Docker-driven verifier with real
packages. T-046-05-01 also recorded the direct verifier passing in Linux CI-equivalent tests.

## Deviations

The Plan proposed retaining an encrypted offline backup before deleting the plaintext key.
This automated attempt provisioned the GitHub secret and removed plaintext local material,
but did not create or select a human-controlled offline credential store. The operator
runbook makes that recovery step explicit for John. The live CI secret is sufficient for
publication; loss of that GitHub secret without an offline copy would require planned key
rotation rather than recovery of the current key.

The public Pages key URL currently returns 404 because enabling Pages does not create a site
artifact. No release was dispatched or tag created: doing so would broaden the ticket into a
release operation. The next normal stable release runs the committed publisher and creates
the initial site atomically after all package and repository verification gates pass.

## Remaining

- Write `review.md`.
- Write `review-disposition.json`.
- Remain on T-046-05-02 and wait for Lisa completion handling.
