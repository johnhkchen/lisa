# T-046-03-02 Plan: stable-channel repair

## Execution policy

Work from the admitted T-046-03-01 completion tree.

Keep the stable version at rc.8 during this ticket.

Do not create or push any tag.

Do not dispatch either release workflow.

Do not mutate the public GitHub release or Homebrew tap.

Keep foreign worktree changes outside every command include list.

Track results and deviations in the private `progress.md`.

## Step 1: capture the release baseline

Record the current workspace version.

Record `releases/latest` tag, prerelease flag, target, and asset names.

Record rc.8's equivalent release data.

Record the current Homebrew formula version and Linux target names.

Record recent Auto Release and Release run summaries.

Confirm v0.3.0 is latest while the tap is rc.8.

Confirm neither published release contains the new musl artifact shape.

Verification:

- API queries succeed with authenticated read access.
- No public mutation command is issued.
- Baseline values are captured in progress.

## Step 2: establish source ancestry

Resolve the final E-045 ticket completion commit.

Use `c08e755` as the release ancestry gate.

Resolve the musl workflow verifier commit as `fcdd293`.

Assert both are ancestors of HEAD.

Assert stable `v0.4.0` does not already exist.

Record the current HEAD for short-of-publication evidence.

Verification:

- both `git merge-base --is-ancestor` commands return zero;
- no local or public stable tag is observed.

## Step 3: reproduce the duplicate release race from history

Inspect rc.8's tag-push Release run.

Inspect rc.8's workflow-dispatch Release run.

Compare event, tag commit, job results, and timing.

Confirm both built all local and global artifacts.

Confirm the dispatched host created the release first.

Confirm the push host failed only on duplicate release creation.

Record the exact run IDs and URLs.

Verification:

- the failed log names an existing release with the same tag;
- the successful run includes Homebrew and announce success.

## Step 4: repair the dispatch decision

Modify only `.github/workflows/auto-release.yml`.

Keep the existing release-exists early exit.

Peel `TAG` to `tag_commit`.

List release workflow runs for that commit.

Select the first non-completed run as active.

If active, log it and return success without dispatch.

Otherwise dispatch the tagged release as before.

Do not change release triggers or permissions.

Verification:

- Ruby/Psych parses the workflow;
- shell syntax for the extracted block is valid;
- fixture cases cover existing release, active run, and dispatch-needed states;
- the diff contains no release workflow or target changes.

## Step 5: commit the workflow repair

Review the exact diff.

Check that `.github/workflows/auto-release.yml` is the only included path.

Run:

```bash
lisa commit-ticket \
  --ticket-id T-046-03-02 \
  --message "Avoid duplicate release dispatches" \
  --include .github/workflows/auto-release.yml
```

Record the resulting commit hash.

Verify the workflow file is clean after the transaction.

## Step 6: author the stable release checklist

Create `docs/knowledge/release-checklist.md`.

State authority and the no-publication ticket boundary first.

Describe the normal stable version-bump merge route.

Describe `just release` only as a mutually exclusive alternate.

Add pre-cut ancestry, version, checks, and dist-plan commands.

Add the expected workflow jobs and asset list.

Add public latest and prerelease-state checks.

Add public E-045 ancestry verification.

Add the isolated README installer test.

Add Homebrew version and musl URL checks.

Add a fill-in evidence record and channel disposition.

Add immutable-tag recovery guidance.

Verification:

- every command uses stable shell quoting;
- the install smoke test changes only a temporary HOME;
- the asset list matches current dist planning;
- the expected path is `~/.local/bin`;
- the version expectation is stable `0.4.0`;
- the claim behavior check uses the public installed binary.

## Step 7: generate the current distribution plan

Obtain pinned cargo-dist 0.30.4 in a temporary tool directory.

Do not install or build Lisa itself merely to use it.

Run plan generation from the repository root.

Save JSON outside ticket-owned source paths.

Inspect the planned local artifact matrix.

Assert these archives:

- `lisa-cli-aarch64-apple-darwin.tar.xz`
- `lisa-cli-x86_64-apple-darwin.tar.xz`
- `lisa-cli-aarch64-unknown-linux-musl.tar.xz`
- `lisa-cli-x86_64-unknown-linux-musl.tar.xz`

Assert the plan contains the shell installer and Homebrew formula.

Assert no `unknown-linux-gnu` Lisa archive remains.

Record the runner and package-install fields for both musl targets.

Verification:

- pinned dist exits zero;
- all six named deliverables are found;
- both musl matrix entries exist;
- GNU archive count is zero.

## Step 8: verify release workflow wiring

Parse `.github/workflows/release.yml`.

Confirm tag-push and tagged workflow-dispatch inputs remain.

Confirm dispatch input precedes event ref in checkout and plan arguments.

Confirm the local matrix comes from the plan output.

Confirm the release WASM build precedes dist build.

Confirm the musl verifier follows dist build and precedes upload.

Confirm global, host, Homebrew, and announce dependencies.

Connect those static observations to the successful rc.8 run evidence.

Verification:

- workflow YAML parses;
- required strings and job edges are present;
- no publishing command is run locally.

## Step 9: exercise auto-release branches with mocks

Extract the release-start script into a temporary file.

Provide a temporary mock `gh` earlier on PATH.

Fixture A reports an existing public release.

Assert no run listing or dispatch occurs.

Fixture B reports no release and one active same-commit run.

Assert the active JSON is logged and no dispatch occurs.

Fixture C reports no release and only completed/no matching runs.

Assert exactly one workflow dispatch uses `tag=v0.4.0`.

Use a temporary Git repository or mock tag resolver as needed.

Verification:

- all fixture scripts return zero;
- captured mock calls match the branch contract;
- no real `gh workflow run` command is reachable.

## Step 10: commit the checklist

Review the exact document diff.

Run:

```bash
lisa commit-ticket \
  --ticket-id T-046-03-02 \
  --message "Document the stable release cut" \
  --include docs/knowledge/release-checklist.md
```

Record the resulting commit hash.

Verify the checklist path is clean after the transaction.

## Step 11: final verification

Run workflow YAML parsing again on committed files.

Run formatting checks.

Run the workspace test suite because workflow source does not affect Rust behavior,
but release readiness should not bless a failing current line.

Run Clippy across all workspace targets.

Run the release WASM build.

Run `git diff --check`.

Audit git status only for the two ticket-owned source paths.

Confirm both are committed and clean.

Confirm foreign dirty paths remain untouched.

## Step 12: Review

Write `review.md` with file changes, commits, and verification evidence.

Explain the live/history plus current-plan proof composition.

State clearly that public stable post-cut fields remain pending John's action.

Point reviewers to the checklist evidence record.

Use a passing disposition if the pipeline and checklist are ready for the cut.

Use a blocking disposition only for an actionable defect in this preparation.

Write `review-disposition.json` with the exact required shape.

Remain on T-046-03-02 after Review.
