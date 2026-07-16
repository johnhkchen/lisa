# T-046-03-02 Structure: stable-channel repair

## Change map

This ticket modifies one workflow file.

It creates one durable release-operations document.

It does not change Rust source, package versions, or distribution targets.

It does not change the generated release workflow.

It does not create a tag, release, or Homebrew commit.

The source paths are:

- `.github/workflows/auto-release.yml`
- `docs/knowledge/release-checklist.md`

The attempt-private phase artifacts are not ticket-owned source units.

## `.github/workflows/auto-release.yml`

### Existing boundary retained

Keep the `workflow_run` trigger for successful CI on main.

Keep manual `workflow_dispatch` support.

Keep the `auto-release-main` concurrency group.

Keep permissions for contents and actions writes.

Keep exact-SHA checkout for workflow-run events.

Keep workspace version discovery.

Keep annotated tag creation and remote existence idempotence.

Keep the public-release existence check.

Keep manual dispatch of `release.yml` with an explicit tag input.

### Modified release-start step

Rename the step to describe both suppression and dispatch.

Continue to provide `GH_TOKEN` and `TAG` through the environment.

Retain this first branch:

```text
release exists -> log -> success without dispatch
```

Add tag peeling:

```text
tag_commit = first commit reachable from TAG
```

The tag exists by construction or by the preceding remote-tag check.

Add a release workflow query:

```text
gh run list
  repository = GITHUB_REPOSITORY
  workflow = release.yml
  commit = tag_commit
  limit = 20
  JSON = databaseId,status,url
```

Filter the JSON to the first row whose status is not `completed`.

Use compact JSON so shell emptiness is unambiguous.

Add the second branch:

```text
active run exists -> log run JSON -> success without dispatch
```

Retain the final branch:

```text
no release and no active run -> workflow_dispatch release.yml for TAG
```

### Behavioral cases

Case 1: later CI on an already released version.

The public-release check short-circuits before querying workflow history.

Case 2: John pushed the tag and its tag-triggered release is running.

The same-commit query finds that run and avoids duplicate dispatch.

Case 3: auto-release created the tag with `GITHUB_TOKEN`.

No tag-push release run exists, so auto-release dispatches explicitly.

Case 4: an earlier release run failed before creating the release.

The completed run is excluded and a recovery dispatch is allowed.

Case 5: a manually dispatched run is active for the same commit.

The same-commit query finds it regardless of event or head branch.

### Non-boundaries

Do not add sleeps or polling loops.

Do not cancel existing release runs.

Do not change release workflow concurrency.

Do not make host publication idempotent in this ticket.

Do not query by tag branch alone because dispatch runs report `main` as head branch.

Do not treat a completed failure as permanently blocking recovery.

## `docs/knowledge/release-checklist.md`

### Document header

Name the document as Lisa's stable release checklist.

State that it is for maintainers and that only John authorizes publication.

Explain that the normal authorization point is merging the stable version bump.

Warn against running the automatic and manual release routes together.

Define reusable variables for repository, tag, and E-045 boundary commit.

### Section: channel baseline

Provide commands for `releases/latest`, newest release, and tap formula version.

Explain the expected pre-cut v0.3.0 versus rc.8 skew for this specific repair.

State the intended post-cut policy: all stable-facing surfaces resolve to v0.4.0.

### Section: choose the release commit

Require origin main synchronization.

Require a clean ordinary worktree for the human release operation.

Require no existing stable tag or release.

Require E-045 completion commit ancestry.

Require T-046-03-01 verifier commit ancestry.

Record the chosen commit SHA.

### Section: stable version preparation

Change workspace version from rc.8 to 0.4.0.

Refresh Cargo.lock through Cargo, not manual lockfile edits.

Assert all three Lisa package records are stable 0.4.0.

Commit the version preparation as its own reviewed change.

Do not embed an automatic push command in the preparation section.

### Section: pre-cut checks

Run formatting, checks, tests, and Clippy.

Build the release WASM with the project-supported command.

Install pinned dist 0.30.4 into an isolated temporary directory if absent.

Run `dist plan` for the stable version.

Assert exact platform archive names.

Assert both musl names and absence of GNU Linux archives.

Assert shell installer and Homebrew formula artifacts.

Parse both workflow YAML files.

### Section: authorization and cut

Put an explicit STOP marker immediately before the mutating action.

Describe the normal route: John approves and merges the version bump to main.

Explain CI -> auto-release -> annotated tag -> dispatch -> release workflow.

Describe `just release` as an alternate human route only.

State that the two routes must not run together.

### Section: monitor the pipeline

List commands to locate Auto Release and Release runs.

List required release jobs in dependency order.

Name the two musl matrix jobs and their verifier step.

Require host, Homebrew, and announce success.

Explain the active-run suppression message from auto-release.

### Section: public asset audit

Provide the expected stable asset list.

Require four archives and four adjacent checksums.

Require both Linux archives to use musl names.

Require installer, formula, aggregate checksum, manifest, and source assets.

Require `prerelease=false` and stable tag identity.

### Section: E-045 proof

Fetch the public tag.

Peel it to a commit.

Assert `c08e755` is an ancestor.

Record both SHAs.

### Section: shell install proof

Use a newly created temporary HOME.

Download through the exact README `releases/latest` path.

Run with profile modification disabled.

Capture output.

Assert `~/.local/bin/lisa` exists under the fixture HOME.

Assert version `0.4.0`.

Assert `lisa claim --help` succeeds.

Assert the fixture HOME has no `.cargo` directory.

### Section: Homebrew convergence

Fetch the tap formula after the publication job completes.

Assert formula version `0.4.0`.

Assert both musl Linux target strings.

Optionally run a Homebrew upgrade/install smoke check on a disposable prefix or host.

Compare shell version, latest tag, and formula version in one output block.

### Section: evidence record

Provide fields for date, operator, release run URL, tag SHA, latest API output,
asset audit, installer output, installed version, claim help, and tap version.

Default channel disposition to `pending` before the cut.

Define `eliminated` as all three surfaces at v0.4.0.

Allow `deliberate` only with reason, owner, and resolution date.

### Section: recovery

If the tag exists without a release and no run is active, dispatch that tag.

If a run failed, rerun failed jobs against the immutable tag.

If release exists but Homebrew failed, resolve the tap before signoff.

Never delete and recreate a public stable tag.

Use a patch release for defects discovered after publication.

## Verification boundaries

YAML parsing validates syntax, not GitHub expression semantics.

Mocked shell cases validate the local branch decision, not GitHub availability.

Pinned dist planning validates the current artifact graph, not publication secrets.

The rc.8 run validates live workflow and secret wiring, but on GNU artifacts.

T-046-03-01 validates the musl link and Bullseye behavior before publication.

The post-cut checklist is the only proof of the actual stable public endpoints.

That proof remains pending until John performs the cut.

## Commit units

Commit the auto-release active-run guard as one exact-path source unit.

Commit the release checklist as a second exact-path source unit.

Use `lisa commit-ticket` for both.

Do not include attempt artifacts in either source commit.
