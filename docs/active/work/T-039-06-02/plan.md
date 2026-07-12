# Plan: T-039-06-02

## Goal

Produce a complete field report from the finished E-039 Codex-seat pass.

Keep deterministic proof distinct from live and repository observations.

Account for every concern named in the acceptance criterion.

Record the mid-pass behavior change and missing failure event as blocking Done.

Do not rerun predecessors, launch a provider harness, or fix the anomaly.

## Step 1: freeze the evidence boundary

Use completed tickets `T-039-01-01` through `T-039-06-01` as the population.

Exclude the active `T-039-06-02` attempt from completion totals.

Record the pre-pass Git base and final predecessor completion revision.

Confirm that all predecessor ticket frontmatter is `done/done`.

Confirm that each predecessor has admitted RDSPI artifacts.

Verification:

- population contains exactly 14 tickets;
- no predecessor ticket is open or in an intermediate phase;
- final predecessor HEAD is `c18efaa`.

## Step 2: validate live usage evidence

Enumerate `.lisa/codex/T-039-*.usage.json` for predecessors.

Validate JSON shape and key/ticket equality.

Sum input and output token counts.

Record aggregate counts without copying sensitive provider content.

Verification:

- 14 files;
- 14 unique expected keys;
- 31,193,999 input tokens;
- 304,533 output tokens;
- no missing predecessor usage file.

## Step 3: validate terminal provenance

Filter `.lisa/provenance.jsonl` to the E-039 predecessor population.

Check schema version, attempt lease, outcome, authority, fence state, requested
route, actual route, timestamps, and pane IDs.

Check duplicate/missing tickets and attempt multiplicity.

Compute the boundary gap between adjacent dependent tickets.

Verification:

- 14 rows;
- all schema version 2;
- all attempt ID 1;
- all `done`, authoritative, and unfenced;
- requested route equals actual OpenAI Codex route;
- no failed/timed-out E-039 row;
- one 1,029-second gap between 02-01 and 02-02.

## Step 4: validate Git history

Walk first-parent history from the pre-pass base to final predecessor completion.

Resolve every ticket-owned source hash cited by predecessor reviews.

Count completion commits.

Verify each source commit is included before its completion commit.

Identify commits not attributable to the planned ticket sequence.

Verification:

- 14 `Complete T-039-*` commits;
- all cited source hashes resolve and are ancestors of final HEAD;
- no done predecessor lacks a completion commit;
- intervening `0f850b3` exists inside the pass.

## Step 5: characterize the intervening change

Inspect `0f850b3` metadata and diff.

Record its author, timestamp, subject, changed file categories, version bump, and
provider lifecycle semantics.

Compare its timestamp to ledger end/start boundaries.

Separate direct facts from causal inference.

Verification:

- commit timestamp is `1783889301`;
- it is 911 seconds after `T-039-02-01` completion;
- it is 118 seconds before `T-039-02-02` start;
- native Codex reset changes from `ClearHandshake` to `ExitThenFresh`;
- version changes rc.6 to rc.7.

## Step 6: collect deterministic proof

Use admitted predecessor artifacts rather than rerunning gates.

Extract final gate commands, counts, hashes, and results from `T-039-06-01`.

Map earlier regression families to each required concern.

Distinguish injected/native fixtures from real provider observations.

Verification:

- final release WASM and CLI builds passed;
- embedded-WASM identity matched;
- formatting and both Clippy targets passed;
- 768 tests passed, zero failed, one live test ignored;
- `just check` passed.

## Step 7: inspect repository residue

Check current worktree and ordinary index.

Classify Lisa-owned lifecycle changes separately from ticket-owned source.

Inspect current lease markers only as file residue, not scheduler authority.

Verify all predecessor source paths are committed.

Verification:

- cached diff has no path;
- no crate or manifest is dirty;
- no predecessor work artifact is dirty;
- visible modifications are active Lisa lifecycle state;
- old lease markers are described without overclaiming pane authority.

## Step 8: write the canonical field report

Create attempt-private `progress.md`.

Lead with `BLOCKING DONE`.

State that no redundant provider run was launched.

Include evidence taxonomy and population totals.

Include deterministic and live sections separately.

Include a matrix covering every required concern.

Document the assignment/reuse gap and rc.7 hotfix.

Document the unavailable initiating failure event.

Document clean staged/index and completion residue results.

Document provenance strengths and limitations.

Map the report back to acceptance.

Verification:

- every named concern has an explicit row;
- behavior change is marked blocking;
- missing failure detail is marked blocking/unexplained;
- no claim treats absence as deterministic proof;
- no fix or rerun is implied.

## Step 9: documentary quality checks

Run `git diff --check`.

Search the private artifacts for all acceptance terms.

Check that factual hashes, counts, and timestamps agree across artifacts.

Check that the verdict is consistent in `progress.md` and `review.md`.

Verification:

- no whitespace error;
- all required categories present;
- no contradictory `PASS/Done` conclusion;
- no unsupported claim of a live timeout, stale reclaim, or false delivery error.

## Step 10: source transaction decision

Inspect whether any ticket-owned source change exists.

Expected result: none.

Do not call `lisa commit-ticket` for an empty implementation unit.

Do not use ordinary `git add` or `git commit`.

Record the no-source-commit disposition in `progress.md` and `review.md`.

Verification:

- no source path staged, modified, or untracked;
- ordinary index remains empty;
- only Lisa-managed artifact publication/lifecycle state is visible.

## Step 11: review the field report

Create attempt-private `review.md`.

Summarize files produced and evidence checked.

Assess coverage for deterministic proof and live observations.

State that the report itself is complete but its verdict blocks ticket Done.

Name the exact blocker:

- live reuse/assignment could not proceed under rc.6;
- out-of-band `0f850b3` changed provider lifecycle behavior;
- the initiating failure has no persisted terminal failure row or exact event.

State that remediation belongs to a separate authorized ticket.

## Step 12: stop on this ticket

After `review.md` is written, remain on `T-039-06-02`.

Do not edit ticket phase or status.

Do not publish Done.

Do not start another ticket.

Lisa owns artifact admission, completion gating, and seat release.

Because the report verdict is blocking, handoff must make human triage explicit.

## Required concern verification matrix

### Assignment/reuse failures

Deterministic coverage: provider reset/delivery state tests.

Live evidence: 1,029-second gap plus rc.7 `ExitThenFresh` hotfix.

Expected disposition: observed and blocking.

### Retries

Deterministic coverage: one bounded chat retry and finite failure outcomes.

Live evidence: no attempt ID above 1, no duplicate terminal row.

Expected disposition: no Lisa-level retry observed; deterministic budget proven.

### Timeouts

Deterministic coverage: injected deadline policies and terminal actions.

Live evidence: no `timed-out` predecessor provenance.

Expected disposition: not observed live; no timeout inferred from wall-clock gap.

### Stale panes

Deterministic coverage: stale-attempt rejection and fenced reclaim tests.

Live evidence: no fenced/duplicate E-039 record; old lease markers remain.

Expected disposition: no stale authority observed; marker residue is limited data.

### False delivery errors

Deterministic coverage: typed delivery failures and commit transaction tests.

Live evidence: no retained false-error report; initiating reuse error unavailable.

Expected disposition: not observed, incomplete evidence.

### Staged/index residue

Deterministic coverage: isolated transaction fixture suite.

Live evidence: every review and current index check report empty.

Expected disposition: clear.

### Done-not-committed residue

Deterministic coverage: completion rollback/restoration fixtures.

Live evidence: one completion commit per done predecessor.

Expected disposition: clear.

### Provenance integrity

Deterministic coverage: append, attribution, hostile target, and lease tests.

Live evidence: complete authoritative terminal set with exact route/lease identity.

Expected disposition: terminal integrity clear; intermediate failure observability
gap blocks a clean field verdict.

## No-rerun assertion

Implementation must not execute:

- `codex`;
- `lisa loop`;
- a live-provider integration harness;
- the ignored real-Zellij integration test;
- predecessor ticket workflows;
- a replacement full build/test suite.

Read-only inspection is sufficient and required by the ticket boundary.

## Completion criteria

The plan is complete when:

- `progress.md` is a self-contained field report;
- every acceptance concern is accounted for;
- deterministic and live evidence are visibly separate;
- the behavior change and unexplained event are marked blocking;
- repository/source state is clean;
- `review.md` provides a concise handoff;
- the agent remains on this ticket.
