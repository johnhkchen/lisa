# T-046-03-02 Design: stable-channel repair

## Goals

Give John one explicit, auditable stable-cut procedure.

Keep publication behind John's chosen version-bump merge or manual action.

Prove the current release graph through every boundary short of publication.

Ensure a stable tag builds Darwin and static-musl Linux archives.

Prevent the observed tag-push/manual-dispatch duplicate race.

Provide a bounded recovery path for an existing tag with no release.

Make post-cut E-045, installer, and channel checks executable rather than narrative.

Leave a durable evidence section for the real public cut.

## Decision 1: deliverable location

### Option A: keep the checklist only in the ticket Review artifact

This would satisfy the immediate handoff within the RDSPI history.

Review artifacts are organized by ticket rather than operator task.

Future stable cuts would have to rediscover the ticket identifier.

It would also mix implementation review with operational procedure.

This option is rejected.

### Option B: add the procedure to README

README is the user-facing install and product overview.

A maintainer release runbook would interrupt that audience and purpose.

The release commands include repository-specific commit and workflow details.

This option is rejected.

### Option C: add `docs/knowledge/release-checklist.md`

The knowledge directory already holds durable operational protocols.

The checklist can link directly from future tickets without user-facing noise.

It can distinguish reusable checks from the v0.4.0-specific evidence boundary.

This option is chosen.

## Decision 2: publication authority

The checklist will not instruct an agent to push automatically.

It will identify the exact point at which John authorizes publication.

For the automated route, that point is merging the stable version bump to main.

Successful CI then starts auto-release against the merge SHA.

For the manual route, John may invoke the existing `just release` recipe.

The normal route will be the version-bump merge and auto-release.

The manual recipe will be documented as an alternate, not a simultaneous action.

No implementation command in this ticket will bump the version, tag, or dispatch.

## Decision 3: duplicate-run repair

### Option A: remove the release workflow's tag-push trigger

Then every release would have to arrive through workflow dispatch.

Bot-created tags already need dispatch because of `GITHUB_TOKEN` suppression.

Human-created tags, however, currently support cargo-dist's normal push path.

Removing it would silently break direct tag pushes and the existing just recipe.

This option is rejected.

### Option B: remove auto-release dispatch and rely on the tag push

Tags pushed by the workflow use the repository `GITHUB_TOKEN`.

GitHub intentionally suppresses a new push-triggered workflow in that case.

The automated release would stop after tagging.

This option is rejected.

### Option C: add release-level concurrency only

A per-tag concurrency group could serialize the two runs.

With cancellation disabled, the second run would still rebuild everything.

It would then fail at release creation unless more host logic changed.

With cancellation enabled, a late duplicate could cancel a nearly complete run.

Neither behavior is appropriate for publication.

This option is rejected as the primary fix.

### Option D: make release creation idempotent

The host job could skip `gh release create` when the release exists.

The duplicate run would still consume four platform builds.

Its generated artifacts might differ in timestamps while targeting the same tag.

The Homebrew job would then need its own idempotence treatment.

This is broader than the observed dispatch decision defect.

This option is rejected.

### Option E: suppress dispatch while a same-commit release run is active

Auto-release already owns the dispatch decision.

After checking for a completed public release, it can resolve the tag commit.

It can query release workflow runs filtered by that commit.

If any matching run is not completed, dispatch is unnecessary and unsafe.

If no matching run is active, dispatch remains necessary for a bot-created tag.

It also remains a recovery path after a prior run completed without a release.

This exactly covers the rc.8 race without changing release triggers.

This option is chosen.

## Active-run guard details

Use `git rev-list -n 1 "$TAG"` to peel the annotated tag to its commit.

Use `gh run list` with repository, workflow, commit, and bounded result count.

Request JSON fields `databaseId`, `status`, and `url`.

Treat every status other than `completed` as active.

This future-proofs the guard across queued, requested, waiting, and pending states.

Log the selected run JSON so the skipped dispatch is diagnosable.

Do not treat a completed failed run as active.

If no release exists after failure, auto-release may dispatch a recovery run.

Retain the release-exists check before the active-run query.

This preserves cheap idempotence on later successful main CI runs.

The query is advisory against external state, so a theoretical sub-second race remains.

The relevant human tag run is normally registered before post-CI auto-release begins.

Bot-created tags cannot cause the competing push run because GitHub suppresses it.

That makes the remaining race bounded by platform behavior, not local sleeps.

## Decision 4: pipeline proof

### Option A: publish a throwaway prerelease

That would exercise every production secret and destination.

It would also create a release and mutate the Homebrew tap.

Publication is expressly outside this ticket's authority.

This option is rejected.

### Option B: rely only on static YAML inspection

Inspection shows the intended jobs and conditions.

It does not prove cargo-dist accepts the current configuration.

It also does not materialize the expected artifact graph.

This option is rejected.

### Option C: combine current planning with historical live runs

Run pinned cargo-dist planning against the current source.

Assert both musl archives, both Darwin archives, installer, and formula.

Parse both workflows and exercise the active-run decision with fixtures.

Use the successful rc.8 dispatch as evidence for live secrets and job wiring.

Use T-046-03-01's two-architecture build proof for the new musl boundary.

This covers each link without creating a release.

This option is chosen.

## Decision 5: reusable automated verifier

### Option A: create a new release-plan script

A script could wrap cargo-dist and assert filenames.

Cargo-dist's JSON structure is generated and may change with its pinned version.

The workflow already consumes the plan dynamically.

The stable-cut checklist only needs a small set of direct jq assertions.

Another script would add maintenance without a runtime consumer.

This option is rejected for now.

### Option B: keep exact commands in the checklist

The operator can see the generated plan path and each assertion.

The commands can be copied into release evidence unchanged.

Any cargo-dist upgrade can update the adjacent commands with the config.

This option is chosen.

## Decision 6: E-045 verification

The source boundary will use completion commit `c08e755`.

Before the cut, require that commit to be an ancestor of the release candidate.

After the cut, fetch the public tag and repeat the ancestry check.

Then run `lisa claim --help` on the isolated installed public binary.

The two checks address different failure modes.

Ancestry catches tagging the wrong line.

Binary behavior catches installing or resolving the wrong public release.

## Decision 7: installer verification

Download the public installer to a temporary directory first.

Do not pipe it directly into the operator's normal HOME during verification.

Set HOME to a temporary `home` directory.

Pass the cargo-dist option that disables shell profile modification.

Execute the installed `~/.local/bin/lisa` by its exact temporary path.

Capture installer output and version output in the evidence section.

Check that no `.cargo` directory was created in the temporary HOME.

This proves both the README redirect and the new install location safely.

## Decision 8: channel policy

The intended stable-cut result is zero skew.

`releases/latest`, the installed shell version, and the tap version must be v0.4.0.

The tap must also point to the new musl Linux archives.

Because prerelease publication remains enabled, future RCs may move Homebrew ahead.

That policy is already deliberate and documented in `dist-workspace.toml`.

For this stable cut, a post-run mismatch is a failure, not an acceptable exception.

The checklist will include a deliberate-skew record only as a fail-closed escape hatch.

It must name version, reason, owner, and resolution date.

## Decision 9: failure recovery

Do not move or recreate `v0.4.0` after it is pushed.

If no release exists and no release run is active, dispatch the existing tag manually.

If a release run fails, inspect and rerun the failed jobs against the same tag.

If a public release exists but Homebrew fails, repair or rerun publication before signoff.

Do not declare success while assets or channels disagree.

A code defect found after public release requires a new patch version.

## Verification strategy

Parse both changed workflow YAML files.

Extract and shell-parse the modified auto-release run block.

Mock `gh` to cover release exists, active run, and dispatch-needed branches.

Run pinned dist planning and inspect its current artifact graph.

Verify HEAD ancestry from E-045 and the musl release verifier.

Check every checklist command for safe quoting and non-destructive defaults.

Run formatting, workflow diff checks, and the relevant repository suite.

Commit the workflow repair separately from the operator checklist.
