# Research: suppress false Review timeout

## Ticket and workflow boundary

Ticket `T-042-01-07` is the final convergence ticket in story `S-042-01`.

Its dependencies are complete:

- `T-042-01-03` added level-triggered Review completion reconciliation;
- `T-042-01-04` added correlated rejection activity and dashboard rendering;
- `T-042-01-06` connected nested-project command construction to the real
  completion transaction.

The ticket concerns the plugin adapter and its Review timeout policy.

It does not own the pure reducer in `lisa-core`.

It does not own the CLI transaction implementation.

It does not own durable completion journalling, which belongs to later story
work.

The current attempt writes workflow artifacts under
`.lisa/attempts/T-042-01-07/1/work`.

Lisa owns admission and publication of those artifacts.

## Production files in the path

The principal file is `crates/lisa-plugin/src/lib.rs`.

It contains plugin state, scheduler polling, artifact admission, completion
dispatch, command execution, command-result handling, timeout policy, activity
conversion, and native tests.

`crates/lisa-plugin/src/deadline.rs` contains pure deadline selection.

Its `DeadlineEvaluator::reviews` method selects Review threads only when:

- the timeout is nonzero;
- thread status is Running;
- phase is Review;
- no earlier finish-up was recorded;
- the pane is not awaiting human input;
- phase elapsed time exceeds the Review timeout;
- activity silence exceeds the wind-down duration.

The deadline evaluator does not inspect files or completion state.

It returns ticket and pane identities as `ReviewAction` values.

`crates/lisa-plugin/src/ui.rs` renders plugin activity.

Completion rejection entries carry ticket ID, rejection kind, correlation ID,
and detail.

The regular and alert-only activity views both include these entries.

`crates/lisa-core/src/completion.rs` defines the pure completion vocabulary.

Its state variants are Eligible, Requested, CommandInFlight, Rejected, and
Confirmed.

Rejected state carries both a typed reason and `Retryability`.

Retryability is either Retryable or ActionRequired.

The core reconciler emits a launch effect only for an eligible, admitted,
passing Review or a retryable rejection with the same durable inputs.

Requested and Confirmed reconcile to no effect.

CommandInFlight reconciles to an action-required observation.

Action-required rejection reconciles to no effect.

## Attempt-scoped Review evidence

`State::attempt_work_dir` derives a private directory from ticket ID and lease
generation.

In production the root is `.lisa/attempts` mounted below `/host`.

Directly constructed native tests fall back below the configured work tree.

`State::admit_artifact` is the authority boundary for workflow evidence.

With an attempt lease, it first verifies that the lease ticket matches and is
the current lease.

It then requires the staged artifact to be a file.

When present, it reads the private file and atomically publishes the exact
bytes to the canonical work directory.

It returns `Ok(false)` when the private current-attempt file is absent.

It returns an error for stale authority, read failure, directory creation
failure, or publication failure.

The canonical work tree alone is not sufficient evidence while a current
attempt exists.

## Completion adapter state

`CompletionInput` is the sole typed scheduler/operator entry vocabulary.

Artifact, Reconcile, Stopped, Idle, ObservedDone, and Manual inputs converge on
`dispatch_completion`.

`dispatch_completion` is the only production caller of
`execute_completion_effect`.

`execute_completion_effect` is the only production site that launches the
`complete-ticket` host command.

`pending_completions` retains the prior phase/status, source, and authority for
an outstanding command.

Pending membership is used as the adapter's Requested reconstruction.

Durable Done in the DAG is used as Confirmed reconstruction when no pending
command masks it.

Otherwise reconciliation currently reconstructs Eligible.

Rejected and CommandInFlight core states are not durably reconstructed by the
plugin adapter.

Their operator-visible evidence is retained in the activity log.

That boundary is consistent with the story's explicit exclusion of durable
completion journalling.

## Completion command boundary

`State::completion_repository_relative_path` maps paths from the plugin's
`/host` view into host project paths.

It normalizes both the candidate and Git root.

It rejects paths outside the Git root and paths selecting the root itself.

`State::build_completion_command` requires configured `lisa_bin`, project
root, and valid Git-root-relative ticket/work paths.

The generated `--path` is the Git root.

The generated ticket and work arguments are Git-root-relative.

This supports a Lisa project nested at paths such as `games/midsummer`.

`T-042-01-06` already has a real transaction regression for that valid nested
shape.

There is also a direct unit test for rejecting a path outside the Git root.

## Command launch and test seam

`execute_completion_effect` inserts a PendingCompletion before command
construction.

In a non-test build, command-construction failure removes that pending entry,
logs a correlated LaunchFailed activity event, and returns false.

In a test build, every command-construction error currently returns true early.

That test-only branch keeps the pending entry and omits the rejection event.

The branch allows many native adapter tests to use default state without a
configured host command.

It also prevents native tests from exercising the production launch-rejection
behavior through the real typed dispatcher.

Successful native command construction calls the Zellij host function stub;
the test target does not run a real completion process through that boundary.

The separately connected nested transaction test calls the CLI transaction
function directly after decoding real builder output.

## Completion result boundary

`handle_completion_result` first looks up and clones the pending record.

It derives a completion-generation correlation from ticket and authority.

Stale authority removes pending state and logs a correlated stale-lease
rejection.

A nonzero exit, absent exit, or non-commit stdout removes pending state,
rebuilds the DAG, and logs correlated LaunchFailed evidence.

That message states that the ticket remains recoverable for retry.

The thread and slot remain assigned on command failure.

No Done provenance is emitted on failure.

A valid commit result is accepted only after durable Done is visible.

If durable Done cannot be verified, the pending record is restored and an
error is logged.

Verified Done releases the slot, removes the thread, emits provenance, and
schedules dependents.

## Review completion reconciliation

`review_completion_inputs` re-admits current-attempt `review.md` and structured
`review-disposition.json`.

It creates `CurrentLeaseArtifactAdmission` only on successful Review
admission.

Disposition parsing is fail-closed.

`reconcile_review_completions` collects non-completed threads with exact
current leases and Review/Done observations.

It runs after artifact and idle phase advancement on every poll.

It runs before transition and Review timeout handling.

It also runs at plugin load, although a fresh state normally has no restored
thread/lease authority.

Repeated reconciliation cannot emit a second effect while pending.

After a retryable command-result failure removes pending state, a later poll
can derive eligibility again from the still-admitted passing Review.

## Review timeout suppression

`check_review_timeouts` first asks the deadline evaluator for time-eligible
actions.

Before resolving the client adapter or writing to a pane, it calls
`review_completion_suppresses_finish_up`.

That helper immediately suppresses when a pending completion exists.

Otherwise it obtains the thread's attempt lease and checks current authority.

It re-admits `review.md` for that exact lease.

An admitted Review suppresses the prompt regardless of disposition.

An admission error logs an Error activity and also suppresses the prompt.

Only `Ok(false)` permits the generic finish-up flow for a current lease.

When permitted, the adapter follow-up is sent to the pane, activity clocks are
updated, the ticket enters `finish_up_sent`, and FinishUpPromptSent is logged.

The predecessor implementation therefore already contains the central guard
needed by this ticket.

## Existing tests and gaps

The old timeout unit test proves that an aged Review thread with no lease and
no artifact receives the prompt.

It does not prove current-attempt absence through the admission boundary.

The `poll_then_reload_reconciles_review_once_without_finish_up` regression
proves an admitted Review with pending completion suppresses the prompt.

It also checks confirmed and blocked reconciliation effects at a high level.

It does not drive launch rejection or command-result failure into timeout
evaluation.

The nested-path tests prove valid nested argv/transaction behavior and a direct
builder rejection.

They do not connect a builder rejection to correlated activity and finish-up
suppression.

`failed_manual_completion_retries_without_early_release_or_duplicate_provenance`
proves retry after command failure and correlated activity.

It uses manual authority without an admitted attempt Review and does not call
Review timeout handling.

No single acceptance regression currently covers all four named cases:

- genuinely missing current-attempt Review;
- admitted Review with pending completion;
- nested-path launch rejection;
- retryable command-result failure.

## Repository constraints

The worktree contains Lisa-managed changes to provenance and active tickets.

It also contains unrelated ticket work and untracked plugin test artifacts.

Those paths are outside this ticket's ownership.

Any source change must be committed with `lisa commit-ticket` and exact include
paths.

The ordinary Git index must remain untouched.

The ticket frontmatter must not be edited by this attempt.
