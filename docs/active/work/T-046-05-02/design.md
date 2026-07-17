# Design — T-046-05-02 signed apt repository and README

## Goals

Publish the four release-built Debian packages through one stable apt source.

Authenticate repository indexes with a project-controlled OpenPGP key.

Give Debian and Ubuntu users explicit `signed-by` keyring instructions.

Preserve older package versions so apt can calculate and perform upgrades.

Keep publication downstream of the existing package and doctor verification gates.

Verify repository installation and upgrade behavior on clean Debian bookworm.

Keep prerelease packages out of the stable apt suite.

Document signing-key custody, provisioning, rotation, and CI exposure.

## Decision 1: hosting provider

### Option A: Cloudsmith open-source repository

Cloudsmith is the story's default proposal.

It provides native Debian ingestion, index generation, signing, and edge delivery.

Its current open-source policy includes at least 50 GB of stored artifacts.

It currently includes 200 GB of package delivery for qualifying public projects.

That is substantially more artifact storage than GitHub Pages.

It also removes the need to maintain apt metadata-generation code.

The provider can use a managed signing key or a custom project key.

GitHub Actions can authenticate through short-lived OIDC credentials.

Those are strong operational advantages once the provider account exists.

The current project state has no Cloudsmith namespace or repository.

The presumed `johnhkchen/lisa` provider path returns 404.

There is no service account, OIDC trust, API key, repository fingerprint, or final client URL.

The README cannot truthfully provide a working pinned-key command before those exist.

Using a provider-managed key would also make the final fingerprint an external provisioning output.

Cloudsmith's open-source policy asks qualifying projects for provider attribution.

The channel would depend on a third-party account and ongoing policy eligibility.

This option is viable but not independently completable from the current repository and GitHub authority.

It is not chosen for this ticket execution.

### Option B: static repository on GitHub Pages

The existing `johnhkchen/lisa` repository already owns the release assets and workflow.

GitHub Actions can build and deploy a static Pages artifact from those releases.

The public endpoint follows the existing project namespace.

The project owns the signing key and can publish the public half in the site artifact.

No second service account, repository namespace, API token, or attribution is required.

Pages has a 1 GB maximum published-site size and a soft 100 GB monthly bandwidth limit.

Those limits are smaller than Cloudsmith's open-source allowances.

The project must maintain apt index construction and signing itself.

A signing-key Actions secret remains a one-time operator requirement.

The release workflow can be tested completely with an ephemeral key before public deployment.

The current release volume and four-package footprint fit comfortably inside the stated limits.

The GitHub Release remains the durable artifact authority even if Pages is rebuilt.

This option is chosen.

### Decision rationale

The delegated ticket execution provides authority and authenticated access to GitHub, not Cloudsmith.

Choosing Pages makes the channel deployable using infrastructure that already exists.

It also keeps the public key, packages, source, release history, and hosting under one project owner.

The extra maintenance is bounded to a small deterministic builder and verifier.

If traffic or site size approaches Pages limits, the same `.deb` inputs can later move to Cloudsmith.

The README will document Pages as the selected host, not leave both providers ambiguous.

## Decision 2: repository suite and compatibility

Use one suite named `stable` and one component named `main`.

Expose `amd64` and `arm64` indexes.

Use one package pool shared by both architectures.

The packages are static-musl at the Lisa and Zellij boundary.

The CLI still depends on the distribution's `git` package.

One vendor suite is therefore sufficient for supported Debian and Ubuntu clients.

Do not create per-codename copies for bookworm, trixie, or Ubuntu releases.

The README source line will name `stable main` explicitly.

The suite label expresses channel stability, not a Debian distribution codename.

Prerelease tags will not deploy into this suite.

## Decision 3: durable package history

### Option A: retain the previous deployed Pages artifact

A publish job could download or clone its previous site and append packages.

Pages Actions deployments are artifacts, not a convenient mutable repository checkout.

Depending on the live site as build input makes recovery depend on serving state.

It also makes a clean rebuild harder to audit.

This option is rejected.

### Option B: commit binary packages to a hosting branch

A dedicated branch would provide history and straightforward incremental updates.

It would duplicate all `.deb` payloads already stored as GitHub Release assets.

Git history would retain removed packages and grow beyond the published tree.

It would require branch mutation and conflict handling in every release.

This option is rejected.

### Option C: reconstruct Pages from stable GitHub Releases

Each publish run queries non-draft, non-prerelease releases.

For each release carrying Debian assets, it downloads all four `.deb` files.

The builder derives package name, version, and architecture from control metadata.

It renames payloads to unique Debian-style pool basenames.

It then regenerates every index and signature from the complete set.

Existing releases predating Debian assets are harmless and contribute no packages.

GitHub Releases remain the single durable binary store.

The Pages deployment is reproducible from immutable public release inputs.

This option is chosen.

## Decision 4: repository builder

Use a checked-in Bash program around Debian's `apt-ftparchive` and GnuPG.

Accept an input directory containing any number of `.deb` files.

Accept an empty destination directory or replace generated repository contents deterministically.

Read package identity with `dpkg-deb --field`.

Reject unexpected package names and architectures.

Reject empty or malformed versions.

Copy packages into a Debian-style `pool/main/l/lisa/` hierarchy.

Use `Package_Version_Architecture.deb` basenames.

Reject two different bytes for one derived package identity.

Permit identical duplicates from overlapping artifact downloads.

Generate separate amd64 and arm64 Packages indexes.

Generate deterministic gzip indexes without embedded timestamps.

Generate a Release file for suite `stable`.

Clear-sign it as `InRelease` and detach-sign it as `Release.gpg`.

Require the signing fingerprint as an explicit input.

Never select an arbitrary secret key from the runner keyring.

## Decision 5: signing key model

Use a dedicated Lisa apt archive signing key.

Commit only its ASCII-armored public key.

Store the ASCII-armored private key in the GitHub Actions secret `APT_SIGNING_KEY`.

Use an unencrypted export inside the encrypted Actions secret.

This avoids exposing a passphrase in a second secret or process argument.

Import the secret into a temporary `GNUPGHOME` owned by the publish job.

Derive the full fingerprint after import.

Require exactly one imported secret-key fingerprint.

Compare it to the fingerprint of the checked-in public key.

Fail publication if the two halves do not match.

Delete the temporary keyring through the runner's normal ephemeral teardown and an explicit trap.

Do not print the secret, exported key material, or transformed secret.

Generate ephemeral signing keys for tests.

The production key never enters pull-request builds.

## Decision 6: public key installation

Publish the armored public key at the Pages root as `lisa-archive-keyring.asc`.

The README downloads it over HTTPS to a temporary path.

It runs `gpg --dearmor` into `/usr/share/keyrings/lisa-archive-keyring.gpg`.

It removes the temporary armored file.

It writes a source entry whose option is exactly the binary keyring path.

This constrains the Lisa source instead of trusting the key globally.

The README will not use deprecated `apt-key`.

The README will install `ca-certificates`, `curl`, and `gnupg` before key setup.

## Decision 7: workflow placement

Add a `publish-apt-repository` job after the existing `host` job.

Run it only for non-prerelease announcements.

Give the job `contents: read`, `pages: write`, and `id-token: write`.

Do not expose Pages permissions to build jobs.

Use a repository-wide apt publication concurrency group with cancellation disabled.

Download stable release assets with the existing GitHub token.

Install only the Debian metadata tools required by the builder.

Import and validate the production signing key.

Build the complete site into a temporary staging directory.

Upload that directory with the official Pages artifact action.

Deploy it with the official Pages deployment action and `github-pages` environment.

Make the final announce job wait for apt publication as it already waits for Homebrew.

A failed apt publication will therefore prevent a fully successful announcement.

## Decision 8: upgrade verification

Extend release verification with a distinct signed-repository script.

Use the real amd64 packages created by the global dist build.

Create older test packages by unpacking each current package and replacing only Version.

Use a fixed old version that always sorts below released Lisa versions.

Build a repository containing only the older pair.

Install its public key into a clean bookworm container's dedicated keyring.

Configure the same `signed-by`, suite, and component shape as the README.

Install both package names through apt.

Record their installed old versions.

Rebuild the mounted repository with the current pair added.

Run apt update and apt upgrade.

Require both packages to reach the current version.

Add the controlled Claude stub and disconnect the network.

Require `lisa doctor` to succeed with packaged runtime provenance.

This complements rather than replaces the existing direct-package verifier.

## Decision 9: README content

Add `Debian and Ubuntu` immediately after Homebrew installation.

Describe it as the signed stable apt channel.

Provide one copyable root shell block for prerequisites, keyring, source, update, and install.

Use `apt-get` inside the administrative block for predictable scripting behavior.

Name both packages explicitly in the install command.

Tell users to run `lisa doctor` after installation.

State that normal `apt update` and `apt upgrade` deliver later stable releases.

State that `lisa-runtime-zellij` supplies the pinned private runtime.

Document the Pages channel's vendor-repository status.

Document the GitHub Pages 1 GB site and soft 100 GB/month constraints.

Document production private-key setup without including private material.

Include the exact `gh secret set APT_SIGNING_KEY` operator command.

Include public-key fingerprint verification and rotation guidance.

## Failure policy

Missing or malformed Debian assets fail before deployment.

An unexpected package name or architecture fails repository construction.

Conflicting package bytes for one identity fail repository construction.

A missing production signing secret fails publication.

Multiple imported private keys fail publication.

A private/public fingerprint mismatch fails publication.

Unsigned or invalid Release metadata fails local verification.

Failure to install the older pair fails release CI.

Failure to upgrade either package fails release CI.

Failure of doctor after upgrade fails release CI.

The prior Pages deployment remains live if a new deployment fails before activation.

## Non-goals

This design does not seek Debian archive inclusion.

It does not publish source packages.

It does not provide nightly or prerelease apt suites.

It does not change package contents or runtime resolution.

It does not alter Homebrew, AUR, Nix, or the shell installer.

It does not implement automatic signing-key rotation.

It does not promise service beyond GitHub Pages' documented limits.
