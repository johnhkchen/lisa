# Lisa apt archive operations

This directory owns the public trust root for Lisa's Debian and Ubuntu package
channels. The repository is published at <https://johnhkchen.github.io/lisa> by
the release workflow, and carries all three channels under one signing key.

## Hosting decision

The channel uses a static GitHub Pages repository reconstructed from GitHub
Release assets. GitHub Releases are the durable package store; a Pages
deployment is a generated view containing the package pool, apt indexes,
signatures, and public key.

Cloudsmith was the other evaluated host. Its open-source policy currently offers
at least 50 GB of artifact data and 200 GB of package delivery, and it manages
Debian indexes and signing. It was not selected because Lisa has no provisioned
Cloudsmith namespace, repository, service account, OIDC trust, or final key URL.
GitHub already owns Lisa's release artifacts and workflow authority, so Pages
made the channel deployable without a second provider account.

The trade-off is capacity and maintenance. [GitHub Pages documents a 1 GB
published-site maximum and a soft 100 GB monthly bandwidth
limit](https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits).
If Lisa approaches either boundary, Cloudsmith remains the preferred migration
target; the four `.deb` release assets are provider-independent inputs.

## Repository shape

The public source names the channel in its suite field:

```text
deb [signed-by=/usr/share/keyrings/lisa-archive-keyring.gpg] https://johnhkchen.github.io/lisa stable main
```

There are three suites — `stable`, `nightly` and `canary` — each with `amd64`
and `arm64` indexes under `dists/<suite>/`. All three are signed by the one
archive key below, so changing channel never means trusting a second key.

Package payloads live in a single shared pool at `pool/main/l/lisa/`. A suite is
an index over a subset of that pool, not a copy of it, and nothing is ever
pruned from it: every version any channel has carried stays fetchable, which is
what makes `apt-get install lisa=<version>` a rollback with no extra machinery.
The pool holding a release candidate does not put that candidate on `stable` —
only the suite index decides what a box can see.

`lisa-runtime-zellij` is published to all three suites, in lockstep with `lisa`.
It carries the same version and pins the Zellij that release was built against,
and `lisa` only *recommends* it, without a version — a single stable runtime
shared by all three would let apt pair a canary `lisa` with a stale runtime.

`scripts/build-apt-repository.sh` takes an input root holding one directory per
suite. It derives package filenames from Debian control metadata, rejects
incomplete architecture/package pairs and conflicting bytes, pools every input
once, cuts each suite's indexes out of one pool-wide index, checks that each
index lists exactly its own suite's packages, and creates `InRelease` plus
`Release.gpg` per suite.

Every publish queries all non-draft GitHub Releases. A release contributes
packages only when it has the complete four-asset set. `stable` takes the
non-prerelease ones, `canary` takes all of them, and `nightly` takes everything
`stable` has plus whichever release `packaging/apt/nightly-tag.txt` names. That
means the complete Pages site can be recovered without a mutable hosting branch
or a prior Pages deployment.

### The nightly pointer

`packaging/apt/nightly-tag.txt` holds one line: a release tag, or the literal
`stable` meaning nothing has been promoted yet. It exists so that soak promotion
and release publishing cannot fight — a publish rebuilds `nightly` from the
pointer rather than from a rule of its own, so shipping a release never undoes a
promotion, and a person can read which release `nightly` is on from the
repository instead of from a box. Writing it is the promotion job's work
(`T-069-01-03`); a promotion takes effect on the served site the next time the
publish job runs.

A tag named there must be a real release with the complete four-asset set, or
the publish fails closed rather than deploying a `nightly` that resolves to
nothing.

## Production signing identity

The committed public key is `packaging/apt/lisa-archive-keyring.asc`.

Its primary fingerprint is:

```text
8FB7 DA4A 79E1 0970 8C44 57C2 E7B9 DBE0 7937 4202
```

The UID is:

```text
Lisa Apt Archive <john.hk.chen@gmail.com>
```

Only the ASCII-armored public export belongs in Git. Never commit a private
export, GnuPG home, revocation certificate, transformed secret, or workflow log
containing private material.

The production private export is stored as the repository Actions secret
`APT_SIGNING_KEY`. It is unencrypted within GitHub's encrypted secret storage so
the noninteractive release job does not need a second passphrase secret. An
unencrypted export is still private key material: do not copy it into an issue,
shell history, workflow file, Actions variable, artifact, or cache.

## One-time key generation

Key generation should happen on a trusted, isolated machine with a new temporary
GnuPG home. The production key currently uses RSA 3072, signing usage only, and
no expiry. The following is the reproducible shape; do not overwrite the current
key unless performing a planned rotation:

```bash
export GNUPGHOME
GNUPGHOME=$(mktemp -d)
chmod 0700 "$GNUPGHOME"

gpg --batch --generate-key <<'EOF'
Key-Type: RSA
Key-Length: 3072
Key-Usage: sign
Name-Real: Lisa Apt Archive
Name-Email: john.hk.chen@gmail.com
Expire-Date: 0
%no-protection
%commit
EOF

fingerprint=$(gpg --batch --with-colons --list-secret-keys |
  awk -F: '$1 == "fpr" { print $10; exit }')
gpg --batch --armor --export "$fingerprint" > lisa-archive-keyring.asc
gpg --batch --armor --export-secret-keys "$fingerprint" > lisa-archive-private.asc
chmod 0600 lisa-archive-private.asc
```

Inspect the public export independently before changing the repository file:

```bash
gpg --batch --show-keys --fingerprint lisa-archive-keyring.asc
grep -q 'BEGIN PGP PRIVATE KEY' lisa-archive-keyring.asc && exit 1
```

Preserve an encrypted offline backup and the generated revocation certificate
according to the maintainer's credential-recovery policy. Neither backup belongs
in this repository.

## Provisioning the Actions secret

With GitHub CLI authenticated as an administrator of `johnhkchen/lisa`, stream
the private export directly into the repository secret:

```bash
gh secret set APT_SIGNING_KEY \
  --repo johnhkchen/lisa \
  < lisa-archive-private.asc
```

Confirm only the secret's presence and update time; GitHub will not reveal its
value:

```bash
gh secret list --repo johnhkchen/lisa | grep '^APT_SIGNING_KEY'
```

Securely remove the plaintext export and temporary GnuPG home after the secret
and offline recovery copy are confirmed.

## CI key handling

`.github/workflows/release.yml` exposes the secret only to the
`publish-apt-repository` job. Pull requests do not use the production key.

**Prerelease tags now do.** A release candidate has to reach the `canary` suite,
and `canary` lives in the same signed archive as `stable`, so the job runs on
every tag rather than only on non-prerelease ones. What keeps `stable` clean is
which releases go into which suite's input, not whether the publish ran.

The job:

1. creates a mode-0700 temporary `GNUPGHOME` on the ephemeral runner;
2. imports `APT_SIGNING_KEY` through standard input without printing it;
3. requires exactly one primary secret key;
4. derives its full fingerprint;
5. derives the fingerprint of the checked-in public export;
6. fails unless the fingerprints match exactly;
7. signs both Release forms with that exact fingerprint;
8. verifies both signatures through a separate public-only keyring; and
9. removes the temporary private keyring through a shell trap.

The Pages artifact contains only `.deb` payloads, indexes, Release signatures,
`.nojekyll`, and `lisa-archive-keyring.asc`. The workflow grants the publish job
`contents: read`, `pages: write`, and `id-token: write`; build jobs do not receive
Pages permissions.

GitHub recommends OIDC-backed Pages deployments and encrypted Actions secrets
for sensitive values. Base64 encoding is not encryption and is unnecessary for
the ASCII-armored key.

## Local verification

The release package directory can be checked end to end with:

```bash
scripts/verify-apt-repository.sh target/distrib
```

The verifier creates a fresh ephemeral key and repacks the real four packages
into three generations: an old version, the current version, and a candidate
version that only `canary` is given. It builds the signed repository, checks
that one public-key import verifies all three suites, and that the candidate is
in the shared pool but in no index except `canary`'s.

A clean bookworm container then installs `lisa` and `lisa-runtime-zellij`
through a `signed-by` file-based source and walks the channels: it upgrades on
`stable` and confirms the candidate is neither visible nor installable there,
rolls back to the old version by exact version and forward again, moves to
`nightly` and then `canary` by editing the suite word alone, and comes back down
to `stable` — where it confirms `apt-get upgrade` does not downgrade and that
`--allow-downgrades` is what completes the move. Finally it disconnects the
network and requires `lisa doctor` to report the packaged runtime and exit zero.

This test never reads `APT_SIGNING_KEY` and is safe for pull requests.

## Key rotation

Rotation changes the trust root and must be planned before the old private key
is retired.

1. Generate and inspect a new dedicated signing key in isolation.
2. Publish the new public key at an additional stable URL while the old key still
   authenticates repository metadata.
3. Give existing clients an authenticated package or documented transition that
   installs the new keyring.
4. Update the checked-in public export, fingerprint documentation, and
   `APT_SIGNING_KEY` together.
5. Publish repository metadata with the new key only after clients have had a
   migration window.
6. Retain the old public key for historical verification and revoke the old
   private key according to the incident or retirement reason.

Replacing the key and URL in one unannounced release strands existing clients:
their pinned old key cannot authenticate the new metadata that tells them about
the replacement. Rotation is therefore a compatibility event, not routine
secret refresh.

## Failure and recovery

A publish fails closed when the secret is missing, imports multiple secret keys,
does not match the committed public key, encounters an incomplete package set,
or cannot verify generated signatures. The prior Pages deployment remains the
served site when a new deployment fails before activation.

Because Pages is reconstructed from GitHub Releases, recovery does not require a
hosting branch. Correct the key or workflow issue and rerun the release workflow
for the same tag. Do not republish different package bytes under an
existing Debian package name, version, and architecture.
