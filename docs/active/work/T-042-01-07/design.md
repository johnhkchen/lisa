# Design: suppress false Review timeout

## Objective

Make the Review timeout contract explicit and executable at the real plugin
adapter boundary.

The finish-up prompt is valid only when the exact current attempt has no
`review.md`.

Once Review exists, the timeout must not replace completion state with a
generic request to rewrite the artifact.

Pending, confirmed, launch-rejected, and command-failed paths must keep their
own correlated evidence visible.

## Existing implementation baseline

The predecessor `T-042-01-03` already placed
`review_completion_suppresses_finish_up` immediately before pane I/O.

That helper gives the timeout the correct evidence boundary:

- pending completion suppresses immediately;
- exact current-attempt Review admission suppresses;
- admission failure logs an actionable error and suppresses;
- exact current-attempt absence permits the prompt.

The production policy therefore does not need another independent artifact
check or a second completion state machine.

The remaining ticket gap is acceptance-level composition coverage.

One small test seam is also needed because native tests currently turn every
command-builder error into a successful stub launch.

## Option A: retain the existing policy and add composed adapter regressions

This option keeps the production timeout guard unchanged.

It adds an explicit native-test control that allows a selected test state to
exercise production command-construction rejection behavior.

It then adds focused tests that drive the real State methods for:

- exact current-attempt missing Review;
- admitted Review with pending completion;
- nested-project command path rejection;
- retryable completion command-result failure;
- durable confirmed completion.

The launch and command failure assertions inspect structured activity entries,
their correlation IDs, and UI conversion.

Advantages:

- preserves the already-correct admission boundary;
- directly covers the ticket's named incidents and acceptance cases;
- keeps changes inside the plugin adapter file;
- avoids duplicating the core reducer;
- avoids introducing persistence that belongs to Story B;
- makes the existing native effect stub honest and selectable;
- fails if timeout suppression is removed from the real call path.

Costs:

- most production logic was introduced by a dependency, so this ticket is
  primarily a convergence/regression unit;
- rejected state is represented through activity rather than a durable
  aggregate snapshot;
- the native host command still remains stubbed after successful construction.

## Option B: add a plugin-owned completion aggregate map

This option would retain `CompletionState` for every ticket in State.

The adapter would fold CommandLaunched, CommandLaunchFailed, CommandSucceeded,
and CommandFailed events into that map.

Timeout policy could then inspect exact Requested, CommandInFlight, Rejected,
and Confirmed variants.

Advantages:

- state naming would align exactly with `lisa-core`;
- retryability and action-required status could be queried directly;
- correlation would live beside the aggregate state.

Costs:

- it duplicates facts already split between pending state, DAG Done, and
  activity;
- without durable restoration it still loses state on reload;
- durable restoration is explicitly assigned to the following journal story;
- it broadens a local timeout ticket into completion lifecycle rearchitecture;
- it risks changing retry timing and existing event-driven behavior;
- it would overlap the reducer-wiring work completed by earlier tickets.

This option is rejected for this ticket.

The story's honest boundary explicitly excludes durable journalling.

A transient map would create the appearance of a solved persistence problem
without solving it.

## Option C: move artifact presence into `DeadlineEvaluator`

This option would add a `review_present` field to `ReviewInput` and filter
actions inside the pure deadline module.

Advantages:

- action selection would produce no candidate for an existing artifact;
- the deadline unit tests could express the rule as pure data.

Costs:

- artifact presence is not a simple caller-provided boolean;
- it must be derived through exact lease validation and admission;
- flattening admission to a boolean loses error evidence;
- pending completion suppression would still remain a plugin concern;
- file I/O or lease types do not belong in the deadline evaluator;
- the plugin would still need post-selection checks against state races.

This option is rejected.

The current two-stage design correctly separates time eligibility from
authority/evidence eligibility.

## Option D: check only private path existence

This option would skip `admit_artifact` and call `Path::is_file` on the current
attempt path.

Advantages:

- simple;
- directly answers whether bytes exist at the staged location;
- avoids canonical publication side effects during timeout evaluation.

Costs:

- bypasses the repository's established lease boundary;
- can trust a stale generation if the surrounding lease checks regress;
- cannot surface publication/admission failure consistently;
- disagrees with the completion reconciler's definition of admitted Review;
- weakens the meaning of “Lisa has already admitted.”

This option is rejected.

Timeout and completion reconciliation should use the same evidence authority.

## Chosen approach

Choose Option A.

The production policy remains a post-deadline admission guard.

The implementation adds only the test-control needed to reach the production
launch-rejection branch and composed regression coverage.

## Test seam design

Add a `#[cfg(test)]` boolean field to State named to describe its behavior,
such as `enforce_completion_launch_errors`.

Its default is false through derived Default.

Existing native tests therefore retain their current inert effect-executor
behavior.

When a configured test sets it true, a command-builder error follows the same
cleanup and correlated rejection code as production.

The production build contains no field and no behavioral branch.

The error handling itself stays shared.

The test-only bypass should be reduced from an unconditional compile-time
return to a condition on this field.

No successful native test launches a real command; the Zellij host shim remains
inert.

## Fixture design

Use temporary directories and real ticket scanning.

Every timeout fixture has:

- a ticket in Review phase;
- a Running Review thread;
- a minted exact current lease;
- elapsed phase/activity clocks beyond both timeout and wind-down;
- an attempt-private work directory.

The missing Review case leaves `review.md` absent and expects exactly one
FinishUpPromptSent event.

The pending case writes Review and Pass disposition, dispatches reconciliation,
and verifies one pending completion before timeout evaluation.

The confirmed case writes Review, places durable Done in the ticket/DAG with no
pending command, and verifies timeout silence.

## Nested-path launch rejection fixture

Use a Git-root directory with a nested Lisa project at `games/midsummer`.

Place the scanned ticket outside that Git root while leaving project and work
configuration nested.

Configure `lisa_bin` and strict native launch-error handling.

The typed Reconcile input admits Review and returns a launch effect.

The real builder rejects the ticket path as outside the Git root.

The executor must:

- remove the just-created pending entry;
- return false;
- log one LaunchFailed rejection;
- retain the exact completion-generation correlation;
- leave the admitted Review available;
- emit no finish-up prompt after the timeout check.

The activity event should convert to the UI CompletionRejected variant without
losing ticket, kind, correlation, or detail.

This is action-required operator evidence even though the current adapter uses
the common LaunchFailed rejection kind.

## Retryable command failure fixture

Use valid nested project and Git-root paths so command construction succeeds.

After reconciliation, assert a pending attempt-authorized completion.

Feed `handle_completion_result` a nonzero exit and diagnostic stderr.

The handler must:

- remove pending state;
- retain the thread and lease;
- log LaunchFailed with “recoverable for retry” detail;
- preserve the completion-generation correlation;
- leave Review admitted;
- emit no finish-up prompt.

A later reconciliation may retry because durable passing evidence remains.

The timeout assertion occurs before that later retry so it proves that artifact
admission, rather than pending membership alone, suppresses the false prompt.

## Correlation and rendering assertions

Expected correlations use `CompletionGenerationId` constructed from:

- ticket ID as CompletionId;
- current lease generation as AttemptId;
- generation 1.

Tests should match the structured ActivityEvent rather than formatted strings
alone.

They should also pass the event through `activity_event_to_ui_entry` and match
`ui::ActivityType::CompletionRejected`.

This connects failure preservation to the dashboard adapter without retesting
all UI formatting already covered by `T-042-01-04`.

## Non-goals

No change to `lisa-core` reducer semantics.

No new durable completion journal.

No retry backoff policy.

No change to CLI transaction isolation.

No change to completion path normalization.

No change to deadline timing thresholds.

No change to ticket or workflow publication.

## Verification

Run the new focused timeout tests first.

Run all `lisa-plugin` library tests because the native executor seam affects
many existing adapter tests.

Run the full workspace suite.

Run formatting, diff hygiene, native Clippy, WASM Clippy, and the release WASM
build.

Confirm the source commit contains only
`crates/lisa-plugin/src/lib.rs`.

Confirm the ordinary index and ticket-owned source path are clean afterward.
