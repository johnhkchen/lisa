# Lisa stable release checklist

This is the maintainer runbook for cutting a stable Lisa release. It is
version-parameterized: set the block below once and every command derives from
it. Current cut: **v0.5.0** — the stable that closes the 0.5.0 line (prior
prerelease v0.5.0-rc.3, published 2026-08-14; prior stable v0.4.4, 2026-07-19).

Only John authorizes publication. Preparing or reviewing this checklist is not
authorization to tag, dispatch, publish, or update the Homebrew tap or apt
repository.

For the automated route, merging the stable version bump to `main` is the
publication-authorizing action: successful main CI starts Auto Release, which
creates the version tag and dispatches the tagged cargo-dist workflow.

Run the automatic and manual release routes one at a time. Never start `just
release` while an Auto Release or Release run for the same commit is active.

Commands below assume Bash, a checkout of `johnhkchen/lisa`, authenticated `gh`,
`jq`, Docker (for the apt checks), and the repository root as the working
directory.

```bash
set -euo pipefail
REPO=johnhkchen/lisa
VERSION=0.5.0
TAG="v$VERSION"
PRIOR_STABLE=v0.4.4
# Ancestry gates: each must be an ancestor of the release commit.
E045_GATE=c08e755   # completion boundary of the last E-045 ticket
MUSL_GATE=fcdd293   # static-musl linkage + Bullseye execution checks in release.yml
SEAL_GATE=6fcb2f2   # completion boundary of S-049-08 (stable-0.4.4 hotfix: E-049 + E-050 line)
WORKFLOW_GATE=e67491b # completion boundary of S-057-02 (0.5.0 line: one working phase, and the upgrade path to it)
DELIVERY_GATE=f508031 # the pane-delivery fix (0.5.0-rc.2: wait for the provider to leave before typing into its pane)
CHANNEL_GATE=e44dee2 # the channel arrangement (0.5.0-rc.3: lisa upgrade, doctor drift, lisa nightly)
```

`v0.5.0` is a **stable** cut, so the prerelease notes below do not apply to it:
`releases/latest` moves to `$TAG`, and `lisa.rb` in the tap takes it. Expect all
three formulae to read `0.5.0` afterwards — `lisa-nightly` follows the newest
soaked release through `packaging/apt/nightly-tag.txt`, and after a stable cut
that is this release. *Nightly and stable are the same version* is correct until
the next release candidate ships.

`v0.5.0-rc.3` was the release `S-068-01` blocked on: the first cut carrying
`lisa upgrade`, `doctor`'s channel-drift row, and `lisa nightly`, so the first
that could put a machine on a channel at all. See
[its cut record](release-0.5.0-rc.3-cut-record.md), which also records the two
defects it surfaced and the stale-`lisa.rb` repair that followed.

`v0.5.0-rc.1` was prepared under this checklist and never published — it was
superseded by rc.2 before anyone tagged it. See
[its cut record](release-0.5.0-rc.1-cut-record.md) for why, and
[the rc.2 record](release-0.5.0-rc.2-cut-record.md) for the cut that shipped.

For a future cut: update `VERSION`, `PRIOR_STABLE`, and append the new cut's
gate commit; do not delete old gates — they are cumulative lineage proof.

Every `for gate in ...` loop below takes the whole list. Add the new name there
when you add it above; a gate that is declared and never iterated proves
nothing.

## Prerelease cuts

`VERSION` may name a release candidate, as this cut does. The runbook is the
same walk with three deliberate differences, all of them consequences of
`dist-workspace.toml` (`publish-prereleases = true`, apt publishing stable-only):

- **`releases/latest` stays at `$PRIOR_STABLE`.** GitHub does not resolve latest
  to a prerelease. Section 6 reads `repos/$REPO/releases/tags/$TAG` instead, and
  asserts `.prerelease == true`.
- **`publish-apt-repository` runs, and it must.** Since `T-069-01-02` the
  archive carries three suites, and a prerelease publish is how a candidate
  reaches `canary`. A skip is a failed cut on any tag. What a prerelease does
  *not* change is `dists/stable`: Section 10's stable checks should find the
  same `$PRIOR_STABLE` there afterwards, with the candidate visible only under
  `dists/canary`.
- **Section 8's README installer path does not carry the prerelease** — it
  downloads through `releases/latest`, which is `$PRIOR_STABLE`. Install through
  the tagged asset instead:
  `https://github.com/johnhkchen/lisa/releases/download/$TAG/lisa-cli-installer.sh`.

`channel_skew: eliminated` is therefore unreachable for a prerelease. Record
`channel_skew: deliberate` with the exact per-channel versions, the reason, and
the resolution date — the stable cut that supersedes it.

## Channel baseline

Expected skew before this cut:

- `releases/latest`: `$PRIOR_STABLE` (stable);
- newest release of any kind: `v0.5.0-rc.2`, the prerelease this cut supersedes;
- Homebrew tap: three formulae, one per channel — `Formula/lisa.rb` on the newest
  release that is not a prerelease, `Formula/lisa-canary.rb` on the newest release
  of any kind, and `Formula/lisa-nightly.rb` on whatever
  `packaging/apt/nightly-tag.txt` names, the same promotion pointer the apt
  suites read. `publish-prereleases = true` is still set and now means a
  candidate reaches `lisa-canary` instead of `lisa`;
- apt repository: `dists/stable` on `$PRIOR_STABLE` and older stables;
  `dists/nightly` on whatever `packaging/apt/nightly-tag.txt` names;
  `dists/canary` on the newest release of any kind.

Capture the live values instead of assuming that baseline is still current:

```bash
gh api "repos/$REPO/releases/latest" \
  --jq '{tag_name,prerelease,published_at,target_commitish}'

gh api "repos/$REPO/releases?per_page=1" \
  --jq '.[0] | {tag_name,prerelease,published_at,target_commitish}'

for formula in lisa lisa-nightly lisa-canary; do
  printf '%s: ' "$formula"
  gh api "repos/johnhkchen/homebrew-lisa/contents/Formula/$formula.rb" \
    --jq .content | tr -d '\n' | base64 --decode |
    awk -F'"' '/^  version "/ { print $2; exit }' || echo 'ABSENT'
done

curl -fsSL https://johnhkchen.github.io/lisa/dists/stable/main/binary-amd64/Packages \
  | awk '/^Package: lisa$/{p=1; next} /^Package: /{p=0} p&&/^Version:/{print}'
```

The apt index carries every published stable, in publication order, so print all
of them. The earlier single-stanza form stopped at the first `Version:` and
reported the *oldest* version in the repository as the baseline.

**Every command above reads from the publisher on purpose.** Verify a published
artifact against the thing that published it — the GitHub API, the release list,
the served `Packages` index — and never against a local mirror of it. A local
mirror answers confidently and is stale exactly when it matters:

- `brew info` reads this machine's tap clone, not the tap. On 2026-08-14 it
  reported `lisa 0.5.0-rc.2` hours after the tap had been corrected to `0.4.4`.
  Read `gh api repos/johnhkchen/homebrew-lisa/contents/Formula/<name>.rb`.
- A rehearsal is a mirror of the world at the moment it was staged. The same day,
  a tap migration rehearsed correctly against a copy seeded four minutes before
  `v0.5.0-rc.3` published, and the plan it produced would have moved
  `lisa-canary` backwards.

Both were right answers to a question about the wrong world. When a check and the
publisher disagree, the publisher is what shipped.

Stop if a stable `$TAG` already exists. Switch to the post-cut audit rather than
creating or moving the tag.

## 1. Choose and record the release commit

Start from an up-to-date main checkout with no maintainer changes in flight:

```bash
git fetch origin main --tags
git switch main
git pull --ff-only origin main
git status --short
RELEASE_BASE=$(git rev-parse HEAD)
printf 'release_base=%s\n' "$RELEASE_BASE"
```

The status output must be empty for the human cut. Lisa's concurrent ticket
worktrees are not a suitable place to perform the version bump.

Prove the chosen line contains every gate:

```bash
for gate in "$E045_GATE" "$MUSL_GATE" "$SEAL_GATE" "$WORKFLOW_GATE" "$DELIVERY_GATE" "$CHANNEL_GATE"; do
  git merge-base --is-ancestor "$gate" "$RELEASE_BASE"
done
```

Every iteration must exit zero. `$SEAL_GATE` is the completion boundary of
S-049-08 (remedies-that-work); its ancestry proves the release line carries the
full seal ladder, park-and-ask machinery, andon surfaces, common-sense
defaults, and the three stable-gating hotfixes.

Confirm that the stable identity is still unused:

```bash
! git show-ref --verify --quiet "refs/tags/$TAG"
! gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1
```

## 2. Prepare the stable version change

Change `[workspace.package].version` in `Cargo.toml` from the current release
candidate to `$VERSION`. Also confirm `crates/lisa-cli/Cargo.toml`'s internal
`lisa-core` requirement tracks the workspace version. Do not edit package
records in `Cargo.lock` by hand.

Refresh the lockfile through Cargo, then check all Lisa package versions:

```bash
cargo check --workspace
cargo metadata --no-deps --format-version 1 | jq -e --arg v "$VERSION" '
  [.packages[] | select(.name | startswith("lisa-")) | .version]
  | length == 3 and all(. == $v)
'
```

Review and commit only the intended stable-version preparation:

```bash
git diff --check
git diff -- Cargo.toml Cargo.lock crates/lisa-cli/Cargo.toml
git commit -am "chore: release $VERSION"
RELEASE_COMMIT=$(git rev-parse HEAD)
printf 'release_commit=%s\n' "$RELEASE_COMMIT"
```

The ordinary commit above is a human release operation outside a Lisa ticket
transaction. Do not run it from an agent attempt or a shared dirty worktree.

## 3. Run the pre-cut gates

Run the repository gates on the exact release commit:

```bash
just fmt-check
just check
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p lisa-plugin --target wasm32-wasip1 --release
git diff --check
test -z "$(git status --short)"
```

Use the repository-pinned cargo-dist 0.30.4. If `dist` is absent or a different
version, install that pinned release into a temporary tool home from its official
installer before continuing:

```bash
DIST_HOME=$(mktemp -d)
curl --proto '=https' --tlsv1.2 -LsSf \
  -o "$DIST_HOME/cargo-dist-installer.sh" \
  https://github.com/axodotdev/cargo-dist/releases/download/v0.30.4/cargo-dist-installer.sh
CARGO_HOME="$DIST_HOME/cargo" \
  sh "$DIST_HOME/cargo-dist-installer.sh" --no-modify-path
export PATH="$DIST_HOME/cargo/bin:$PATH"

dist --version
test "$(dist --version)" = 'cargo-dist 0.30.4'
dist plan --output-format=json > "/tmp/lisa-$TAG-dist-plan.json"
```

Assert the four platform archives and their adjacent checksums:

```bash
for artifact in \
  lisa-cli-aarch64-apple-darwin.tar.gz \
  lisa-cli-aarch64-apple-darwin.tar.gz.sha256 \
  lisa-cli-x86_64-apple-darwin.tar.gz \
  lisa-cli-x86_64-apple-darwin.tar.gz.sha256 \
  lisa-cli-aarch64-unknown-linux-musl.tar.gz \
  lisa-cli-aarch64-unknown-linux-musl.tar.gz.sha256 \
  lisa-cli-x86_64-unknown-linux-musl.tar.gz \
  lisa-cli-x86_64-unknown-linux-musl.tar.gz.sha256 \
  lisa-cli-installer.sh \
  lisa.rb
do
  jq -e --arg artifact "$artifact" \
    'any(.releases[].artifacts[]; . == $artifact)' \
    "/tmp/lisa-$TAG-dist-plan.json" >/dev/null
done

! jq -e '
  any(.releases[].artifacts[]; test("^lisa-cli-.*unknown-linux-gnu"))
' "/tmp/lisa-$TAG-dist-plan.json" >/dev/null
```

The four `.deb` packages are cargo-dist extra artifacts built by
`scripts/package-debs.sh`; they do not appear in `dist plan` and are asserted on
the published release in section 6 instead.

Assert both musl matrix jobs and their native linker package:

```bash
jq -e '
  [.ci.github.artifacts_matrix.include[]
   | select(.targets | any(. == "aarch64-unknown-linux-musl"))]
  | length == 1 and .[0].runner == "ubuntu-22.04-arm"
    and (.[0].packages_install | contains("musl-tools"))
' "/tmp/lisa-$TAG-dist-plan.json" >/dev/null

jq -e '
  [.ci.github.artifacts_matrix.include[]
   | select(.targets | any(. == "x86_64-unknown-linux-musl"))]
  | length == 1 and .[0].runner == "ubuntu-22.04"
    and (.[0].packages_install | contains("musl-tools"))
' "/tmp/lisa-$TAG-dist-plan.json" >/dev/null
```

Finally, parse both workflow files and inspect the release-specific gates:

```bash
ruby -e 'require "yaml"; YAML.parse_file(".github/workflows/auto-release.yml")'
ruby -e 'require "yaml"; YAML.parse_file(".github/workflows/release.yml")'

rg -n 'github.event.inputs.tag|Build WASM plugin|Verify static musl artifact|verify-homebrew-tap|publish-homebrew-formula|publish-apt-repository' \
  .github/workflows/release.yml
```

The tagged checkout expression must prefer `github.event.inputs.tag`; the WASM
build must precede cargo-dist; the musl verifier must run after `dist build` and
before artifact upload; Homebrew must depend on a successful host job and on
`verify-homebrew-tap`, which rehearses the three formulae on macOS before any of
them reaches the tap; and `publish-apt-repository` must be present, ungated by
prerelease status, and required by `announce`.

## 4. Publication authorization and cut

STOP. Everything above is non-publishing preparation.

John reviews the release commit and explicitly authorizes exactly one route.

### Normal route: merge the version bump

Push the reviewed version bump through the normal main integration path. After
successful main CI:

1. Auto Release reads version `$VERSION` from that exact CI SHA.
2. It creates annotated tag `$TAG` if absent.
3. A tag created with `GITHUB_TOKEN` does not trigger a push workflow, so Auto
   Release explicitly dispatches `release.yml` with `tag=$TAG`.
4. If John already pushed the tag and its Release run is active, Auto Release
   detects the same commit and does not dispatch a duplicate.
5. `release.yml` checks out the immutable tag, builds, verifies, hosts, and
   publishes the Homebrew formula and the apt repository.

### Alternate route: explicit human tag push

Use `just release` only when John deliberately chooses the manual route and has
confirmed that no Auto Release or Release run is active for the commit. The
recipe runs checks, creates the workspace-version tag, and pushes it.

Do not run both routes. Do not manually dispatch while either route has a queued
or in-progress release for the same commit.

## 5. Monitor the pipeline

Locate the two workflow runs:

```bash
gh run list --repo "$REPO" --workflow auto-release.yml --limit 5
gh run list --repo "$REPO" --workflow release.yml --commit "$RELEASE_COMMIT" --limit 5
```

Watch the selected Release run and fail on a non-success result:

```bash
RELEASE_RUN_ID=$(
  gh run list --repo "$REPO" --workflow release.yml \
    --commit "$RELEASE_COMMIT" --limit 5 \
    --json databaseId,status \
    --jq '.[0].databaseId'
)
gh run watch "$RELEASE_RUN_ID" --repo "$REPO" --exit-status
```

Inspect its complete job summary:

```bash
gh run view "$RELEASE_RUN_ID" --repo "$REPO" --json jobs \
  --jq '.jobs[] | {name,conclusion,url,steps:[.steps[]|{name,conclusion}]}'
```

Required successful jobs for a stable cut are:

- `plan`;
- `build-local-artifacts (aarch64-apple-darwin)`;
- `build-local-artifacts (x86_64-apple-darwin)`;
- `build-local-artifacts (aarch64-unknown-linux-musl)`;
- `build-local-artifacts (x86_64-unknown-linux-musl)`;
- `build-global-artifacts`;
- `host`;
- `publish-homebrew-formula`;
- `publish-apt-repository` (every tag — skipped is a FAILED cut, prerelease
  included, because that is how a candidate reaches `canary`);
- `announce`.

Both musl jobs must show `Verify static musl artifact on Debian bullseye` as
successful. That step proves static ELF linkage, embedded WASM and runtime data,
and execution on Bullseye before release upload.

Auto Release may legitimately report that an active same-commit Release run
already exists. That is duplicate suppression, not a skipped release. Follow the
referenced active run through completion.

## 6. Audit the public stable release

Create a dedicated evidence directory outside the repository worktree:

```bash
EVIDENCE=$(mktemp -d)
printf 'evidence_dir=%s\n' "$EVIDENCE"
gh api "repos/$REPO/releases/latest" > "$EVIDENCE/latest-release.json"
```

Require stable identity and public-latest resolution:

```bash
jq -e --arg tag "$TAG" '
  .tag_name == $tag and .prerelease == false and .draft == false
' "$EVIDENCE/latest-release.json" >/dev/null
```

Require the complete asset surface — archives, installer, formula, checksums,
and all four Debian packages:

```bash
jq -e '
  [.assets[].name] as $assets
  | all([
      "dist-manifest.json",
      "lisa-cli-aarch64-apple-darwin.tar.gz",
      "lisa-cli-aarch64-apple-darwin.tar.gz.sha256",
      "lisa-cli-x86_64-apple-darwin.tar.gz",
      "lisa-cli-x86_64-apple-darwin.tar.gz.sha256",
      "lisa-cli-aarch64-unknown-linux-musl.tar.gz",
      "lisa-cli-aarch64-unknown-linux-musl.tar.gz.sha256",
      "lisa-cli-x86_64-unknown-linux-musl.tar.gz",
      "lisa-cli-x86_64-unknown-linux-musl.tar.gz.sha256",
      "lisa-cli-installer.sh",
      "lisa.rb",
      "lisa-amd64.deb",
      "lisa-arm64.deb",
      "lisa-runtime-zellij-amd64.deb",
      "lisa-runtime-zellij-arm64.deb",
      "sha256.sum",
      "source.tar.gz",
      "source.tar.gz.sha256"
    ][]; . as $name | $assets | index($name) != null)
' "$EVIDENCE/latest-release.json" >/dev/null

jq -r '.assets[].name' "$EVIDENCE/latest-release.json" \
  | tee "$EVIDENCE/assets.txt"
```

No released Lisa Linux archive may contain `unknown-linux-gnu`:

```bash
! grep -E '^lisa-cli-.*unknown-linux-gnu' "$EVIDENCE/assets.txt"
```

## 7. Prove the public tag's ancestry

Fetch and peel the public stable tag, then repeat the ancestry gates:

```bash
git fetch origin "refs/tags/$TAG:refs/tags/$TAG"
PUBLIC_TAG_COMMIT=$(git rev-parse "$TAG^{commit}")
for gate in "$E045_GATE" "$MUSL_GATE" "$SEAL_GATE" "$WORKFLOW_GATE" "$DELIVERY_GATE" "$CHANNEL_GATE"; do
  git merge-base --is-ancestor "$gate" "$PUBLIC_TAG_COMMIT"
done
printf 'public_tag_commit=%s\n' "$PUBLIC_TAG_COMMIT" \
  | tee "$EVIDENCE/tag-ancestry.txt"
```

This is the source-level proof that `releases/latest` resolves to the line
carrying every gated fix, not an older configuration.

## 8. Run the exact README installer in isolation

Download through the README's `releases/latest` path, not a versioned substitute:

```bash
mkdir -p "$EVIDENCE/home"
curl --proto '=https' --tlsv1.2 -LsSf \
  -o "$EVIDENCE/lisa-cli-installer.sh" \
  https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh

HOME="$EVIDENCE/home" \
  sh "$EVIDENCE/lisa-cli-installer.sh" --no-modify-path \
  2>&1 | tee "$EVIDENCE/installer-output.txt"
```

Verify the Rust-free destination and stable version:

```bash
LISA_UNDER_TEST="$EVIDENCE/home/.local/bin/lisa"
test -x "$LISA_UNDER_TEST"
test ! -e "$EVIDENCE/home/.cargo"
test "$("$LISA_UNDER_TEST" --version)" = "lisa $VERSION"
"$LISA_UNDER_TEST" --version | tee "$EVIDENCE/installed-version.txt"
```

## 9. Verify Homebrew convergence

Wait for `publish-homebrew-formula` to finish, then read all three formulae. The
cut always moves `lisa-canary`; it writes `lisa` from the newest release that is
not a prerelease, which is `$VERSION` on a stable cut and `$PRIOR_STABLE` on a
candidate; and it never moves `lisa-nightly`, which follows the promotion
pointer — the hourly `promote-nightly.yml` moves that one, roughly a day after
this cut, and [nightly-promotion.md](nightly-promotion.md) is where to look when
it has not:

```bash
for formula in lisa lisa-nightly lisa-canary; do
  gh api "repos/johnhkchen/homebrew-lisa/contents/Formula/$formula.rb" \
    --jq .content | tr -d '\n' | base64 --decode > "$EVIDENCE/$formula.rb"
done

grep -F "version \"$VERSION\"" "$EVIDENCE/lisa-canary.rb"
grep -F 'lisa-cli-aarch64-unknown-linux-musl.tar.gz' "$EVIDENCE/lisa-canary.rb"
grep -F 'lisa-cli-x86_64-unknown-linux-musl.tar.gz' "$EVIDENCE/lisa-canary.rb"

# Each formula refuses the other two, so a machine has one Lisa and one channel.
for formula in lisa lisa-nightly lisa-canary; do
  grep -F 'conflicts_with' "$EVIDENCE/$formula.rb"
done

# A stable cut only. On a release candidate, assert instead that lisa.rb carries
# $PRIOR_STABLE -- the publish writes it from that release rather than skipping
# the file, so this is a convergence check on a candidate cut too, not an
# assumption that nothing touched it.
grep -F "version \"$VERSION\"" "$EVIDENCE/lisa.rb"
```

A formula that does not match after the run has converged is a tap to repair,
not a release to re-cut: [nightly-promotion.md](nightly-promotion.md#when-the-tap-is-already-wrong)
has the one-dispatch republish and the two-dispatch fallback.

## 10. Verify apt convergence — fresh install and upgrade

Both checks run in disposable Debian containers against the live Pages
repository, after `publish-apt-repository` succeeds. The first proves the
README's Debian path installs the new stable; the second proves ordinary
`apt-get upgrade` moves an existing $PRIOR_STABLE machine forward (the audit
item deferred from the $PRIOR_STABLE cut).

Fresh install:

```bash
docker run --rm debian:bookworm bash -ec '
  apt-get update -qq
  apt-get install -y -qq ca-certificates curl gnupg >/dev/null
  curl --proto "=https" --tlsv1.2 -fsSL \
    https://johnhkchen.github.io/lisa/lisa-archive-keyring.asc \
    | gpg --batch --dearmor -o /usr/share/keyrings/lisa-archive-keyring.gpg
  echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/lisa-archive-keyring.gpg] https://johnhkchen.github.io/lisa stable main" \
    > /etc/apt/sources.list.d/lisa.list
  apt-get update -qq
  apt-get install -y -qq lisa lisa-runtime-zellij >/dev/null
  lisa --version
  test -x /usr/libexec/lisa/zellij
' | tee "$EVIDENCE/apt-fresh-install.txt"
grep -F "lisa $VERSION" "$EVIDENCE/apt-fresh-install.txt"
```

Upgrade from the prior stable:

```bash
docker run --rm debian:bookworm bash -ec '
  apt-get update -qq
  apt-get install -y -qq ca-certificates curl gnupg >/dev/null
  curl --proto "=https" --tlsv1.2 -fsSL \
    https://johnhkchen.github.io/lisa/lisa-archive-keyring.asc \
    | gpg --batch --dearmor -o /usr/share/keyrings/lisa-archive-keyring.gpg
  echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/lisa-archive-keyring.gpg] https://johnhkchen.github.io/lisa stable main" \
    > /etc/apt/sources.list.d/lisa.list
  apt-get update -qq
  apt-get install -y -qq lisa='"${PRIOR_STABLE#v}"'* lisa-runtime-zellij >/dev/null || \
    apt-get install -y -qq lisa lisa-runtime-zellij >/dev/null
  before=$(lisa --version)
  apt-get update -qq && apt-get upgrade -y -qq >/dev/null
  after=$(lisa --version)
  echo "before=$before after=$after"
' | tee "$EVIDENCE/apt-upgrade.txt"
grep -F "after=lisa $VERSION" "$EVIDENCE/apt-upgrade.txt"
```

If the repository no longer serves the prior stable version (single-version
repo), the fallback install makes the upgrade leg equal the fresh leg — record
that explicitly in the cut record rather than skipping the check.

Channels stay separate. All three suites are signed by the same key, so one
keyring reads all of them; what must differ is what each one offers. On a
prerelease cut this is the check that matters — the candidate must be in
`canary` and in neither of the others:

```bash
docker run --rm debian:bookworm bash -ec '
  apt-get update -qq
  apt-get install -y -qq ca-certificates curl gnupg >/dev/null
  curl --proto "=https" --tlsv1.2 -fsSL \
    https://johnhkchen.github.io/lisa/lisa-archive-keyring.asc \
    | gpg --batch --dearmor -o /usr/share/keyrings/lisa-archive-keyring.gpg
  for channel in stable nightly canary; do
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/lisa-archive-keyring.gpg] https://johnhkchen.github.io/lisa $channel main" \
      > /etc/apt/sources.list.d/lisa.list
    apt-get update -qq
    echo "$channel=$(apt-cache policy lisa | awk "/Candidate:/ { print \$2 }")"
  done
' | tee "$EVIDENCE/apt-channels.txt"
```

`stable` must never name a candidate version. On a stable cut all three
normally agree on `$VERSION`; on a prerelease cut `canary` is ahead and that is
the design. Record the three values either way.

Compare all stable-facing versions in one record:

```bash
{
  printf 'latest_tag='
  jq -r .tag_name "$EVIDENCE/latest-release.json"
  printf 'shell_version='
  "$LISA_UNDER_TEST" --version
  for formula in lisa lisa-nightly lisa-canary; do
    printf 'brew_%s=' "$formula"
    sed -n 's/^  version "\([^"]*\)"/\1/p' "$EVIDENCE/$formula.rb"
  done
  printf 'apt_version='
  sed -n 's/^lisa \(.*\)/\1/p' "$EVIDENCE/apt-fresh-install.txt" | head -1
} | tee "$EVIDENCE/channel-versions.txt"
```

The stable-facing values — `latest_tag`, `shell_version`, `brew_lisa` and
`apt_version` — must agree on `$VERSION` on a stable cut. On a prerelease cut
`brew_lisa_canary` is `$VERSION` while `brew_lisa` stays on `$PRIOR_STABLE`,
exactly as apt's `canary` and `stable` suites do, and that is the design rather
than skew. `brew_lisa_nightly` follows the promotion pointer either way. The
intended disposition is `eliminated`; an unexplained mismatch is not a passing
release.

## 11. Record the cut

Copy this block into the release ticket's Review or a dated field report and
replace every `PENDING` value from the evidence files:

```text
release: $TAG
cut_at: PENDING
operator: PENDING
release_commit: PENDING
release_run_url: PENDING
latest_api_tag: PENDING
latest_prerelease: PENDING
ancestry_gates: PENDING            # e045 / musl / seal, each "ancestor"
asset_audit: PENDING               # includes all four .deb packages
aarch64_musl_bullseye_step: PENDING
x86_64_musl_bullseye_step: PENDING
readme_installer_path: PENDING
installed_version: PENDING
homebrew_lisa_version: PENDING
homebrew_lisa_nightly_version: PENDING
homebrew_lisa_canary_version: PENDING
apt_fresh_version: PENDING
apt_upgrade_from_prior: PENDING    # before/after line, or "equal-to-fresh (single-version repo)"
channel_skew: pending
```

Set `channel_skew: eliminated` only when latest, the isolated shell install,
the tap, and the apt repository all report `$VERSION`.

If skew is intentionally retained, use `channel_skew: deliberate` and also
record the exact versions, reason, owner, and resolution date. Unexplained skew
is a failed post-cut audit.

## Recovery

Keep release tags immutable.

If `$TAG` exists, no public release exists, and no Release run for its commit
is active, dispatch the existing tag:

```bash
gh workflow run release.yml --repo "$REPO" --ref main --field "tag=$TAG"
```

If a Release run fails before publication, inspect it and rerun failed jobs
against the same tag:

```bash
gh run view "$RELEASE_RUN_ID" --repo "$REPO" --log-failed
gh run rerun "$RELEASE_RUN_ID" --repo "$REPO" --failed
```

If the GitHub release exists but Homebrew or apt publication failed, repair or
rerun the failed publish job before signing off. Record the temporary skew until
convergence.

Never delete and recreate a public stable tag to hide a bad artifact. A product
defect discovered after publication is fixed in a new patch release.

## Why Auto Release still dispatches

GitHub does not start ordinary push-triggered workflows for repository changes
made with the workflow's `GITHUB_TOKEN`. `workflow_dispatch` is an exception.
Auto Release therefore must dispatch after creating its tag. Its same-commit
active-run check exists for the other case: John already pushed the tag, so the
tag-triggered Release run is queued or running before Auto Release reaches the
dispatch step.

References:

- <https://docs.github.com/en/actions/concepts/security/github_token#when-github_token-triggers-workflow-runs>
- <https://cli.github.com/manual/gh_run_list>
- <https://cli.github.com/manual/gh_workflow_run>
