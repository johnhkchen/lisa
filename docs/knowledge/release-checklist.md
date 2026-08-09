# Lisa stable release checklist

This is the maintainer runbook for cutting a stable Lisa release. It is
version-parameterized: set the block below once and every command derives from
it. Current cut: **v0.5.0-rc.2** (published 2026-08-09; prior stable v0.4.4).

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
VERSION=0.5.0-rc.2
TAG="v$VERSION"
PRIOR_STABLE=v0.4.4
# Ancestry gates: each must be an ancestor of the release commit.
E045_GATE=c08e755   # completion boundary of the last E-045 ticket
MUSL_GATE=fcdd293   # static-musl linkage + Bullseye execution checks in release.yml
SEAL_GATE=6fcb2f2   # completion boundary of S-049-08 (stable-0.4.4 hotfix: E-049 + E-050 line)
WORKFLOW_GATE=e67491b # completion boundary of S-057-02 (0.5.0 line: one working phase, and the upgrade path to it)
DELIVERY_GATE=f508031 # the pane-delivery fix (0.5.0-rc.2: wait for the provider to leave before typing into its pane)
```

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
- **`publish-apt-repository` is skipped, and a skip is correct.** For a stable
  cut a skip is a failed release; for a prerelease it is the design. Section 10
  does not apply, and the apt repository legitimately keeps serving the prior
  stable.
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
- newest release of any kind: `$PRIOR_STABLE` — the 0.4.4 line was cut stable
  and no prerelease has been published since;
- Homebrew tap: `$PRIOR_STABLE` without its `v` (deliberate:
  `publish-prereleases = true` means the tap tracks whichever came last);
- apt repository: `$PRIOR_STABLE` and older stables (apt publishing skips
  prereleases).

Capture the live values instead of assuming that baseline is still current:

```bash
gh api "repos/$REPO/releases/latest" \
  --jq '{tag_name,prerelease,published_at,target_commitish}'

gh api "repos/$REPO/releases?per_page=1" \
  --jq '.[0] | {tag_name,prerelease,published_at,target_commitish}'

gh api repos/johnhkchen/homebrew-lisa/contents/Formula/lisa.rb \
  --jq .content | tr -d '\n' | base64 --decode | sed -n '1,45p'

curl -fsSL https://johnhkchen.github.io/lisa/dists/stable/main/binary-amd64/Packages \
  | awk '/^Package: lisa$/{p=1; next} /^Package: /{p=0} p&&/^Version:/{print}'
```

The apt index carries every published stable, in publication order, so print all
of them. The earlier single-stanza form stopped at the first `Version:` and
reported the *oldest* version in the repository as the baseline.

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
for gate in "$E045_GATE" "$MUSL_GATE" "$SEAL_GATE" "$WORKFLOW_GATE" "$DELIVERY_GATE"; do
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

rg -n 'github.event.inputs.tag|Build WASM plugin|Verify static musl artifact|publish-homebrew-formula|publish-apt-repository' \
  .github/workflows/release.yml
```

The tagged checkout expression must prefer `github.event.inputs.tag`; the WASM
build must precede cargo-dist; the musl verifier must run after `dist build` and
before artifact upload; Homebrew must depend on a successful host job; and
`publish-apt-repository` must be present, gated to stable (non-prerelease) tags,
and required by `announce`.

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
- `publish-apt-repository` (stable tags only — skipped is a FAILED stable cut);
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
for gate in "$E045_GATE" "$MUSL_GATE" "$SEAL_GATE" "$WORKFLOW_GATE" "$DELIVERY_GATE"; do
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

Wait for `publish-homebrew-formula` to finish before reading the tap:

```bash
gh api repos/johnhkchen/homebrew-lisa/contents/Formula/lisa.rb \
  --jq .content | tr -d '\n' | base64 --decode \
  > "$EVIDENCE/lisa.rb"

grep -F "version \"$VERSION\"" "$EVIDENCE/lisa.rb"
grep -F 'lisa-cli-aarch64-unknown-linux-musl.tar.gz' "$EVIDENCE/lisa.rb"
grep -F 'lisa-cli-x86_64-unknown-linux-musl.tar.gz' "$EVIDENCE/lisa.rb"
```

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

Compare all stable-facing versions in one record:

```bash
{
  printf 'latest_tag='
  jq -r .tag_name "$EVIDENCE/latest-release.json"
  printf 'shell_version='
  "$LISA_UNDER_TEST" --version
  printf 'brew_version='
  sed -n 's/^  version "\([^"]*\)"/\1/p' "$EVIDENCE/lisa.rb"
  printf 'apt_version='
  sed -n 's/^lisa \(.*\)/\1/p' "$EVIDENCE/apt-fresh-install.txt" | head -1
} | tee "$EVIDENCE/channel-versions.txt"
```

All four values must agree on `$VERSION`. The intended disposition is
`eliminated`; a mismatch is not a passing release.

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
homebrew_version: PENDING
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
