# Design: T-039-06-02

## Decision

Produce the field report as the Implement-phase `progress.md` artifact.

Treat it as a forensic synthesis of already-recorded evidence.

Do not launch Codex, Zellij, a provider harness, or predecessor tests again.

Separate each claim into one of four evidence classes:

1. deterministic proof;
2. live execution observation;
3. repository/provenance observation;
4. inference or evidence gap.

Give every acceptance-criterion concern an explicit disposition.

Record the rc.7 reuse hotfix as a behavior change.

Record the missing failure ledger/UI event as an unexplained provenance gap.

Mark the ticket blocked for Done rather than attempting a repair.

## Reporting objective

The report must let a reviewer answer two different questions independently:

- Does the final checked-in tree pass its deterministic contracts?
- Did the live end-to-end pass complete without anomalous intervention?

The first answer is yes.

The second answer is no.

Combining those answers into a single green/red claim would lose the most
important information in this ticket.

The report therefore leads with the blocking verdict and then preserves both
evidence tracks.

## Option 1: rerun a clean provider harness

This option would rebuild or reuse the current binary, launch new live sessions,
and check whether rc.7 now schedules every ticket without intervention.

Potential advantages:

- it could demonstrate current behavior on the final tree;
- it could capture the error boundary more deliberately;
- it could yield a pass after the hotfix.

This option is rejected.

The ticket expressly prohibits a redundant provider harness.

A new run would not change the historical fact that the completed pass required
a mid-run behavior change.

A successful rerun could incorrectly obscure the anomaly instead of explaining
it.

It would also spend more live provider tokens and expand the ticket beyond a
field report.

## Option 2: report only the green final gates

This option would treat `T-039-06-01` as the complete result and summarize the
768 passing tests, release builds, hashes, Clippy, formatting, and WASM checks.

Potential advantages:

- concise;
- based on admitted predecessor evidence;
- straightforward acceptance mapping for deterministic proof.

This option is rejected.

It would omit the 1,029-second scheduling gap and intervening rc.7 hotfix.

It would blur deterministic proof with live execution behavior.

It would violate the explicit requirement that any behavior change block Done.

## Option 3: treat all successful ledger rows as proof of provider parity

This option would count the 14 authoritative `done` records and conclude that
assignment, reuse, timeouts, retries, and completion all behaved correctly.

Potential advantages:

- uses structured machine-readable evidence;
- avoids subjective dashboard interpretation;
- confirms exact terminal attribution.

This option is rejected.

Terminal provenance does not encode every intermediate scheduling failure.

The lack of a failed row during the reuse gap is itself relevant.

The hotfix explains why later tickets could finish, not why the original reuse
transition failed.

Successful terminal records cannot prove that no manual intervention occurred.

## Option 4: forensic two-track field report

This option combines:

- final deterministic gate evidence;
- per-ticket live usage evidence;
- terminal provenance rows;
- first-parent Git history;
- predecessor progress/review statements;
- current index/worktree and lease-marker observations.

It explicitly labels inferences and unavailable evidence.

It does not claim the UI state or provider event that was not persisted.

It records the hotfix and missing failure event as blocking.

This option is selected.

It matches the story's honest boundary: report anomalies; do not fix them here.

## Evidence hierarchy

Machine-readable sources take priority for exact counts and identities:

1. Git object history for committed changes and ordering;
2. `.lisa/provenance.jsonl` for terminal attempt outcomes;
3. `.lisa/codex/*.usage.json` for live Codex token evidence;
4. ticket frontmatter for published lifecycle state;
5. admitted progress/review artifacts for commands and interpretation;
6. current runtime marker files for present residue only.

No source is used outside the fact it can establish.

For example, a lease marker establishes file residue but not current authority.

A `done` ledger row establishes a terminal outcome but not absence of an earlier
unrecorded reuse stall.

A commit message and diff establish a behavior change; temporal placement makes
its relation to the gap a strong inference rather than a recorded failure event.

## Required concern matrix

The report will contain a compact matrix with these rows:

- assignment/reuse failures;
- retries;
- timeouts;
- stale panes;
- false delivery errors;
- staged/index residue;
- done-not-committed residue;
- provenance integrity;
- behavior changes/anomalies.

Each row will state:

- deterministic proof available;
- live/repository observation;
- disposition;
- whether it blocks Done.

This prevents “not observed” from being confused with “proved impossible.”

## Deterministic proof presentation

The report will cite the final tree gates from `T-039-06-01`:

- release WASM build;
- release CLI build;
- matching embedded-WASM identity;
- formatting;
- native Clippy;
- WASM Clippy;
- workspace tests;
- canonical `just check`.

It will also summarize the relevant predecessor regression surfaces:

- typed signal admission and stale-attempt rejection;
- bounded named assignment/startup failures;
- timeout and stale-thread policies;
- atomic publication hostile paths;
- isolated Git transaction residue tests;
- provenance attribution/integrity tests.

The report will not reproduce every predecessor test name.

Representative names and counts are sufficient to show the contract boundary.

## Live observation presentation

The report will state that 14 real Codex sessions are evidenced by 14 usage files.

It will report the aggregate token counts.

It will report the 14 authoritative terminal rows and attempt ID 1 throughout.

It will describe pane 0/pane 1 alternation.

It will identify the 1,029-second gap exactly.

It will place commit `0f850b3` inside that interval.

It will describe the change from `/clear` reuse to exit-then-fresh behavior.

It will state that later tickets completed after this change.

It will avoid claiming that the pass validates the pre-hotfix implementation.

## False-delivery-error treatment

No structured evidence identifies a false delivery error during E-039.

No predecessor artifact reports that `lisa commit-ticket` returned failure after
successfully creating its commit.

All cited source hashes resolve in the final linear history.

The observed reuse failure lacks its exact UI/error event.

The report therefore classifies false delivery errors as “not observed in
retained evidence; exact original reuse error unavailable.”

This is deliberately weaker than “none occurred.”

## Stale-pane treatment

No E-039 terminal row is fenced or duplicated.

No second attempt exists.

Old lease marker files remain for completed panes.

The report will distinguish stale marker residue from stale scheduler authority.

It will not inspect or manipulate the live Zellij session because the ticket is
about the completed predecessor pass, not a new interactive experiment.

## Repository-safety treatment

The report will use both historical and current evidence.

Historical reviews consistently record empty ordinary-index state after source
transactions.

All ticket source commits are ancestors of their completion commits.

Current cached diff is empty.

Every predecessor has a completion commit and done frontmatter.

The current uncommitted provenance tail and active ticket phase are Lisa-owned.

Those lifecycle changes are not reported as ticket-owned residue.

## Blocking semantics

The report will use an explicit final verdict:

`BLOCKING DONE`.

Two reasons support it:

1. the end-to-end pass required a provider lifecycle behavior change midway;
2. the initiating failure has no persisted failed/timed-out provenance or exact
   error event, so the original transition is not fully reconstructable.

This verdict follows the ticket wording directly.

The final green gates do not downgrade the verdict.

## No remediation in this ticket

No source change will be made.

No follow-up ticket will be created automatically.

No ticket frontmatter will be edited by the agent.

No source commit is appropriate for a private phase artifact.

Lisa will publish the six artifacts during its lifecycle transaction.

The Review artifact will hand off the blocking issue for human triage.

## Rejected claims

The report will not claim provider parity is fully proven live.

It will not claim the rc.6 reuse path succeeded.

It will not claim a timeout occurred merely because there was a long gap.

It will not claim a stale pane executed work based only on marker residue.

It will not claim the failure was a false delivery error without the event.

It will not claim token counts from provenance because those fields are null.

It will not claim the current active ticket's outcome before completion.

## Verification strategy

Implementation verification is documentary and read-only:

- recount E-039 ledger rows;
- validate requested/actual route equality;
- recount usage files and aggregate tokens;
- verify source and completion commits resolve in the final chain;
- verify the hotfix timestamp lies in the assignment gap;
- verify current cached diff is empty;
- run `git diff --check` for authored artifacts;
- confirm no new provider process or harness was launched.

The report itself will be checked for every acceptance concern and an explicit
blocking verdict.

## Design outcome

The selected design preserves the historical record, keeps deterministic and
live evidence honest, and prevents a green final test suite from concealing the
mid-pass behavior change.

It satisfies the report scope while deliberately refusing to certify Done.
