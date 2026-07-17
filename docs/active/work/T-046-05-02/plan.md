# Plan — T-046-05-02 signed apt repository and README

## Execution rules

Work only in files assigned by the Structure artifact.

Preserve all unrelated modified and untracked files in the shared worktree.

Use `apply_patch` for repository file edits.

Do not use the ordinary Git index.

Commit each meaningful source unit with `lisa commit-ticket`.

Pass only exact repository-relative ticket-owned paths.

Record implementation state and deviations in `progress.md`.

Do not update ticket phase or status frontmatter.

## Step 1: create a dedicated archive signing key

Use an isolated temporary directory outside the repository.

Run GnuPG in a Debian bookworm container because the host lacks GnuPG.

Generate an RSA signing key dedicated to the Lisa apt archive.

Use a descriptive identity with no personal authentication role.

Use no passphrase because the exported private key will live inside the encrypted Actions secret.

Export the public half in ASCII armor.

Export the private half in ASCII armor to the temporary directory.

List the full fingerprint from the public key.

Inspect the public export and require exactly one public key.

Inspect the secret export inside the isolated container and require exactly one secret key.

Never emit the private export to command output.

Add only the public export to `packaging/apt/lisa-archive-keyring.asc`.

Keep the private export until the workflow secret is provisioned.

Verification:

- public key imports in a clean temporary GnuPG home;
- full fingerprint is stable across independent public-key inspection;
- `gpg --list-packets` reports no secret-key packet in the repository file;
- temporary private file permissions are owner-only.

## Step 2: implement the repository builder

Create `scripts/build-apt-repository.sh` with strict Bash options.

Resolve and validate all four positional parameters.

Check required commands before mutating output.

Canonicalize input, output, and public-key paths.

Reject overlapping input and output directories.

Create a temporary output sibling.

Install a trap that removes incomplete temporary output.

Recursively enumerate Debian inputs in byte-stable sorted order.

For each package, query Package, Version, and Architecture through `dpkg-deb`.

Accept only the two ticket package names and two supported architectures.

Validate version characters before using them in a basename.

Calculate a SHA-256 for duplicate comparison.

Copy packages into the pool under derived unique basenames.

Ignore byte-identical repeated inputs.

Fail on identity collisions whose checksums differ.

Require at least one package for each package name and architecture.

Generate each Packages index with apt-ftparchive's architecture filter.

Require each index to contain both expected package names.

Create gzip copies with timestamp suppression.

Generate Release metadata with explicit project fields.

Check that the configured public key carries the requested fingerprint.

Check that the current GnuPG home carries the requested secret fingerprint.

Create `InRelease` and `Release.gpg` using the exact fingerprint.

Verify both signatures immediately against the provided public key in a clean keyring.

Copy the armored public key and create `.nojekyll`.

Atomically replace the destination directory with the completed temporary output.

Run `bash -n` on the script.

Run ShellCheck if available.

## Step 3: test the builder's focused failure boundaries

Create temporary minimal `.deb` fixtures in a Debian tool container.

Build one fixture for each package and architecture.

Generate an ephemeral test key.

Run the builder and inspect the complete output tree.

Require both architecture indexes to mention both package names.

Require Release to name both architectures and stable/main.

Verify `InRelease` and `Release.gpg` with a public-only keyring.

Repeat with identical duplicate input and require success.

Replace a duplicate with different bytes under the same control identity and require failure.

Run with a foreign package name and require failure.

Run with a missing architecture pair and require failure.

Record results in progress.

## Step 4: commit archive key and builder

Run `git diff --check` on the two source paths and operator runbook if already present.

Confirm no private-key marker exists anywhere under `packaging/apt`.

Commit the first unit with:

```text
lisa commit-ticket --ticket-id T-046-05-02 \
  --message "Build signed apt repository metadata" \
  --include packaging/apt/lisa-archive-keyring.asc \
  --include scripts/build-apt-repository.sh
```

Confirm both paths are clean after the isolated commit.

## Step 5: implement the bookworm repository verifier

Create `scripts/verify-apt-repository.sh` with strict Bash options.

Require Docker and the four real Debian release artifacts.

Create an isolated temporary work area and cleanup trap.

Start the tool container and install apt-utils plus GnuPG.

Generate an ephemeral signing key inside the mounted temporary home.

Export its public key and capture its full fingerprint.

Unpack each real package with `dpkg-deb --raw-extract`.

Replace only the Version control field with the old fixture version.

Rebuild the four older packages.

Invoke the production builder with only old inputs.

Verify both signature forms using the ephemeral public key.

Assert the site does not contain private key material.

Create a clean bookworm client container with the site mounted.

Install prerequisites from Debian while networking is available.

Dearmor the repository public key into the dedicated keyring path.

Write the source list entry using `signed-by` and file transport.

Remove or disable unrelated Debian sources before repository-specific apt assertions if needed.

Run apt update and install both package names.

Assert the installed pair has the old version.

Rebuild the site from old and current inputs.

Run apt update and noninteractive apt upgrade in the same client.

Assert installed and candidate versions for both packages equal current metadata.

Install the controlled Claude stub and doctor project.

Disconnect the client from all networks.

Run Lisa doctor and assert packaged runtime provenance, path, and success summary.

Run `bash -n` and ShellCheck when available.

Run the complete verifier against `target/distrib` artifacts if present.

If artifacts are absent, reproduce them through the established package builder inputs rather than weakening the test.

## Step 6: commit the verifier

Run `git diff --check -- scripts/verify-apt-repository.sh`.

Confirm the integration script creates no tracked output.

Commit the second unit with:

```text
lisa commit-ticket --ticket-id T-046-05-02 \
  --message "Verify signed apt installs and upgrades" \
  --include scripts/verify-apt-repository.sh
```

Confirm the path is clean.

## Step 7: wire release publication

Modify `.github/workflows/release.yml` after the existing Debian verification step.

Call the new repository verifier in the global build job.

Add the post-host `publish-apt-repository` job.

Give the job only the three required permissions.

Use the `github-pages` environment and official Pages actions.

Serialize deployments with a stable concurrency group.

Download non-draft, non-prerelease Debian assets into per-tag directories.

Do not treat pre-package stable releases as errors.

Require at least one complete four-package release before building.

Install apt-utils and GnuPG.

Write `APT_SIGNING_KEY` to GnuPG only through standard input.

Use a temporary mode-0700 GnuPG home.

Require exactly one secret fingerprint.

Import the checked-in public key into a separate public-only keyring.

Compare exact fingerprints.

Invoke the repository builder with the production key.

Configure, upload, and deploy the Pages artifact.

Add apt publication to the announce job dependency and result condition.

Parse the final workflow with Ruby's YAML parser or an available equivalent.

Check all action versions against current official action repositories.

Inspect the expression and dependency graph for pull-request and prerelease behavior.

## Step 8: commit release wiring

Run `git diff --check -- .github/workflows/release.yml`.

Commit the third unit with:

```text
lisa commit-ticket --ticket-id T-046-05-02 \
  --message "Publish stable apt channel to GitHub Pages" \
  --include .github/workflows/release.yml
```

Confirm the workflow path is clean.

## Step 9: write client and operator documentation

Create `packaging/apt/README.md` from the implemented key and workflow behavior.

Include the exact public fingerprint.

Include safe private-key generation and export commands.

Include `gh secret set APT_SIGNING_KEY < ...`.

Explain why the unencrypted export is still secret and how GitHub protects it at rest.

Explain the temporary runner keyring and exact fingerprint check.

Explain rotation ordering and old-client impact.

Explain stable release reconstruction and Pages constraints.

Modify README's Install Lisa section.

Add the exact public Pages key URL.

Use `gpg --dearmor` into a dedicated keyring.

Use a source line with exact `signed-by` pinning.

Install both package names.

Describe normal apt upgrades and packaged runtime behavior.

Link the operator runbook.

Identify the channel as a vendor repository.

Check all Markdown code blocks and relative links.

Run the README command shape inside the integration verifier or an equivalent clean container.

## Step 10: commit documentation

Run `git diff --check -- README.md packaging/apt/README.md`.

Commit the fourth unit with:

```text
lisa commit-ticket --ticket-id T-046-05-02 \
  --message "Document the Debian apt channel" \
  --include README.md \
  --include packaging/apt/README.md
```

Confirm both paths are clean.

## Step 11: provision external GitHub state

Set `APT_SIGNING_KEY` from the temporary private export with `gh secret set`.

List repository secret names and confirm the new name exists without reading its value.

Delete the temporary private export and its GnuPG home.

Confirm the deleted path no longer exists.

Configure the repository Pages build type as Actions through the GitHub API.

Read the Pages configuration back and require the expected public URL and workflow build type.

Do not dispatch a release or create a tag from this ticket.

The next normal stable release will perform the first public deployment.

## Step 12: final verification

Run `bash -n` on both new scripts.

Run ShellCheck if available.

Run the full signed repository verifier.

Run the existing direct package verifier when its Docker inputs are available.

Parse `.github/workflows/release.yml` as YAML.

Run `git diff --check` repository-wide for ticket changes.

Inspect the committed diff for private material and secret expansion mistakes.

Search ticket-owned paths for `PRIVATE KEY` markers.

Require only the documented private-secret name, never its value.

Inspect Git status and distinguish unrelated pre-existing changes.

Require all ticket-owned implementation paths to be tracked and clean.

## Step 13: review artifacts

Write `progress.md` with commits, tests, external provisioning, and deviations.

Write `review.md` with file inventory, behavior, coverage, and limitations.

Use a pass disposition only if all ticket-owned source is committed and clean.

Use a block disposition if the secret, Pages configuration, repository verification, or public instructions remain incomplete.

Write the disposition JSON in the exact required shape.

Remain on T-046-05-02 after Review.

Do not publish shared work artifacts or change ticket frontmatter.
