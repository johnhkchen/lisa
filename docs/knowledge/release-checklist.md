# Lisa stable release checklist

This is the maintainer runbook for cutting a stable Lisa release.

Only John authorizes publication. Preparing or reviewing this checklist is not
authorization to tag, dispatch, publish, or update the Homebrew tap.

For the automated route, merging the stable version bump to `main` is the
publication-authorizing action: successful main CI starts Auto Release, which
creates the version tag and dispatches the tagged cargo-dist workflow.

Run the automatic and manual release routes one at a time. Never start `just
release` while an Auto Release or Release run for the same commit is active.

Commands below assume Bash, a checkout of `johnhkchen/lisa`, authenticated `gh`,
`jq`, and the repository root as the working directory.

```bash
set -euo pipefail
REPO=johnhkchen/lisa
TAG=v0.4.0
E045_GATE=c08e755
MUSL_GATE=fcdd293
```

## v0.4.0 channel baseline

Before this stable cut, the expected skew is:

- `releases/latest`: `v0.3.0` (stable, pre-E-045);
- newest prerelease: `v0.4.0-rc.8`;
- Homebrew tap: `0.4.0-rc.8` because `publish-prereleases = true`.

Capture the live values instead of assuming that baseline is still current:

```bash
gh api "repos/$REPO/releases/latest" \
  --jq '{tag_name,prerelease,published_at,target_commitish}'

gh api "repos/$REPO/releases?per_page=1" \
  --jq '.[0] | {tag_name,prerelease,published_at,target_commitish}'

gh api repos/johnhkchen/homebrew-lisa/contents/Formula/lisa.rb \
  --jq .content | tr -d '\n' | base64 --decode | sed -n '1,45p'
```

Stop if a stable v0.4.0 already exists. Switch to the post-cut audit rather than
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

Prove that the chosen line contains both the completed E-045 claim path and the
static-musl release gate:

```bash
git merge-base --is-ancestor "$E045_GATE" "$RELEASE_BASE"
git merge-base --is-ancestor "$MUSL_GATE" "$RELEASE_BASE"
```

Both commands must exit zero. `c08e755` is the completion boundary for the last
E-045 ticket; `fcdd293` added packaged-musl static linkage, embedded-asset, and
Bullseye execution checks to the release workflow.

Confirm that the stable identity is still unused:

```bash
! git show-ref --verify --quiet "refs/tags/$TAG"
! gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1
```

## 2. Prepare the stable version change

Change `[workspace.package].version` in `Cargo.toml` from the current release
candidate to `0.4.0`. Do not edit package records in `Cargo.lock` by hand.

Refresh the lockfile through Cargo, then check all Lisa package versions:

```bash
cargo check --workspace
cargo metadata --no-deps --format-version 1 | jq -e '
  [.packages[] | select(.name | startswith("lisa-")) | .version]
  | length == 3 and all(. == "0.4.0")
'

awk '
  /^name = "lisa-(cli|core|plugin)"$/ { package=$0; getline; print package " " $0 }
' Cargo.lock
```

The lockfile output must show `version = "0.4.0"` for `lisa-cli`, `lisa-core`,
and `lisa-plugin`.

Review and commit only the intended stable-version preparation:

```bash
git diff --check
git diff -- Cargo.toml Cargo.lock
git commit -am 'chore: release 0.4.0'
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
dist plan --output-format=json > /tmp/lisa-v0.4.0-dist-plan.json
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
    /tmp/lisa-v0.4.0-dist-plan.json >/dev/null
done

! jq -e '
  any(.releases[].artifacts[]; test("^lisa-cli-.*unknown-linux-gnu"))
' /tmp/lisa-v0.4.0-dist-plan.json >/dev/null
```

Assert both musl matrix jobs and their native linker package:

```bash
jq -e '
  [.ci.github.artifacts_matrix.include[]
   | select(.targets | any(. == "aarch64-unknown-linux-musl"))]
  | length == 1 and .[0].runner == "ubuntu-22.04-arm"
    and (.[0].packages_install | contains("musl-tools"))
' /tmp/lisa-v0.4.0-dist-plan.json >/dev/null

jq -e '
  [.ci.github.artifacts_matrix.include[]
   | select(.targets | any(. == "x86_64-unknown-linux-musl"))]
  | length == 1 and .[0].runner == "ubuntu-22.04"
    and (.[0].packages_install | contains("musl-tools"))
' /tmp/lisa-v0.4.0-dist-plan.json >/dev/null
```

Finally, parse both workflow files and inspect the release-specific gates:

```bash
ruby -e 'require "yaml"; YAML.parse_file(".github/workflows/auto-release.yml")'
ruby -e 'require "yaml"; YAML.parse_file(".github/workflows/release.yml")'

rg -n 'github.event.inputs.tag|Build WASM plugin|Verify static musl artifact|publish-homebrew-formula' \
  .github/workflows/release.yml
```

The tagged checkout expression must prefer `github.event.inputs.tag`; the WASM
build must precede cargo-dist; the musl verifier must run after `dist build` and
before artifact upload; Homebrew must depend on a successful host job.

## 4. Publication authorization and cut

STOP. Everything above is non-publishing preparation.

John reviews the release commit and explicitly authorizes exactly one route.

### Normal route: merge the version bump

Push the reviewed version bump through the normal main integration path. After
successful main CI:

1. Auto Release reads version `0.4.0` from that exact CI SHA.
2. It creates annotated tag `v0.4.0` if absent.
3. A tag created with `GITHUB_TOKEN` does not trigger a push workflow, so Auto
   Release explicitly dispatches `release.yml` with `tag=v0.4.0`.
4. If John already pushed the tag and its Release run is active, Auto Release
   detects the same commit and does not dispatch a duplicate.
5. `release.yml` checks out the immutable tag, builds, verifies, hosts, and
   publishes the Homebrew formula.

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

Required successful jobs are:

- `plan`;
- `build-local-artifacts (aarch64-apple-darwin)`;
- `build-local-artifacts (x86_64-apple-darwin)`;
- `build-local-artifacts (aarch64-unknown-linux-musl)`;
- `build-local-artifacts (x86_64-unknown-linux-musl)`;
- `build-global-artifacts`;
- `host`;
- `publish-homebrew-formula`;
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

Require the complete asset surface:

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

## 7. Prove the public tag contains E-045

Fetch and peel the public stable tag, then repeat the ancestry gate:

```bash
git fetch origin "refs/tags/$TAG:refs/tags/$TAG"
PUBLIC_TAG_COMMIT=$(git rev-parse "$TAG^{commit}")
git merge-base --is-ancestor "$E045_GATE" "$PUBLIC_TAG_COMMIT"
git merge-base --is-ancestor "$MUSL_GATE" "$PUBLIC_TAG_COMMIT"
printf 'public_tag_commit=%s\n' "$PUBLIC_TAG_COMMIT" \
  | tee "$EVIDENCE/tag-ancestry.txt"
```

This is the source-level proof that `releases/latest` resolves to the fixed
Codex claim-path line rather than the old v0.3.0 configuration.

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

Verify the Rust-free destination, stable version, and packaged claim command:

```bash
LISA_UNDER_TEST="$EVIDENCE/home/.local/bin/lisa"
test -x "$LISA_UNDER_TEST"
test ! -e "$EVIDENCE/home/.cargo"
test "$("$LISA_UNDER_TEST" --version)" = 'lisa 0.4.0'
"$LISA_UNDER_TEST" claim --help > "$EVIDENCE/claim-help.txt"
"$LISA_UNDER_TEST" --version | tee "$EVIDENCE/installed-version.txt"
```

This records both the exact one-liner redirect's installer and the installed
binary without replacing the operator's normal Lisa or editing shell profiles.

## 9. Verify Homebrew convergence

Wait for `publish-homebrew-formula` to finish before reading the tap:

```bash
gh api repos/johnhkchen/homebrew-lisa/contents/Formula/lisa.rb \
  --jq .content | tr -d '\n' | base64 --decode \
  > "$EVIDENCE/lisa.rb"

grep -F 'version "0.4.0"' "$EVIDENCE/lisa.rb"
grep -F 'lisa-cli-aarch64-unknown-linux-musl.tar.gz' "$EVIDENCE/lisa.rb"
grep -F 'lisa-cli-x86_64-unknown-linux-musl.tar.gz' "$EVIDENCE/lisa.rb"
```

Compare all stable-facing versions in one record:

```bash
{
  printf 'latest_tag='
  jq -r .tag_name "$EVIDENCE/latest-release.json"
  printf 'shell_version='
  "$LISA_UNDER_TEST" --version
  printf 'brew_version='
  sed -n 's/^  version "\([^"]*\)"/\1/p' "$EVIDENCE/lisa.rb"
} | tee "$EVIDENCE/channel-versions.txt"
```

For v0.4.0, all three values must agree. The intended disposition is
`eliminated`; a mismatch is not a passing release.

An optional macOS smoke check after the formula audit is:

```bash
brew update
brew upgrade johnhkchen/lisa/lisa || brew install johnhkchen/lisa/lisa
brew list --versions lisa
lisa --version
```

## 10. Record the v0.4.0 cut

Copy this block into the release ticket's Review or a dated field report and
replace every `PENDING` value from the evidence files:

```text
release: v0.4.0
cut_at: PENDING
operator: PENDING
release_commit: PENDING
release_run_url: PENDING
latest_api_tag: PENDING
latest_prerelease: PENDING
e045_gate_ancestor: PENDING
musl_gate_ancestor: PENDING
asset_audit: PENDING
aarch64_musl_bullseye_step: PENDING
x86_64_musl_bullseye_step: PENDING
readme_installer_path: PENDING
installed_version: PENDING
claim_help: PENDING
homebrew_version: PENDING
channel_skew: pending
```

Set `channel_skew: eliminated` only when latest, the isolated shell install, and
the tap all report v0.4.0.

If skew is intentionally retained, use `channel_skew: deliberate` and also
record the exact versions, reason, owner, and resolution date. Unexplained skew
is a failed post-cut audit.

## Recovery

Keep release tags immutable.

If `v0.4.0` exists, no public release exists, and no Release run for its commit
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

If the GitHub release exists but Homebrew publication failed, repair or rerun
the tap job before signing off. Record the temporary skew until convergence.

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
