# Research: hostile-order real-adapter regression

## Ticket boundary

`T-042-04-01` is the first ticket in story `S-042-04`.

The ticket starts in Research and requires all remaining RDSPI phases.

Its only requested repository change is deterministic regression coverage.

The story explicitly excludes a new production completion contract.

The fixture must exercise `lisa-plugin`, not only the pure reducer.

The required topology is a Git repository containing a Lisa project at
`games/midsummer`.

The recorded sequence originates from Arcade ticket `T-009-01-01`.

The sequence combines artifact ordering, slot transition, timeout/reload,
late or duplicate observations, and operator recovery input.

Passing and blocked Review outcomes are both part of acceptance.

## Dependency surface

`T-042-01-07` supplies Review-timeout suppression regressions.

Its production guard is `State::review_completion_suppresses_finish_up`.

The guard suppresses when a transaction is pending.

It also suppresses when the exact current attempt's `review.md` is admitted.

Admission errors are surfaced as activity and also suppress the false prompt.

`T-042-02-03` supplies bounded replay of durable in-flight commands.

Its journal records Requested, CommandInFlight, Rejected, and Confirmed state.

CommandInFlight retains correlation and an absolute reconciliation deadline.

Reconstruction before the deadline replays the same completion generation.

The CLI transaction treats the generation as an idempotency key.

`T-042-03-03` supplies a seven-row operator recovery test matrix.

The `d` then Enter UI path emits `CompletionInput::OperatorRequested`.

Operator identity is independent of attempt lease identity.

Blocked disposition and AlreadyPending are named refusals.

## Core completion model

`crates/lisa-core/src/completion.rs` owns the pure completion vocabulary.

`DurableCompletionInputs` combines current-lease artifact admission and the
structured Review disposition.

`CompletionState` includes Eligible, Requested, CommandInFlight, Rejected,
and Confirmed.

`EffectCommand::LaunchCompletion` is inert domain output.

`reconcile` is level-triggered over durable inputs and aggregate state.

An admitted passing Review in Eligible produces one launch effect.

Requested and Confirmed do not produce another effect.

CommandInFlight produces replay or deadline-expiry reconciliation.

Blocked and invalid dispositions are ineligible.

The core recorded-livelock test does not cross the plugin adapter boundary.

## Plugin adapter boundary

`crates/lisa-plugin/src/lib.rs` contains scheduler state and native tests.

`CompletionInput` represents Artifact, Reconcile, Stopped, Idle,
ObservedDone, and OperatorRequested observations.

`State::dispatch_completion` is the sole typed production request gateway.

`State::execute_completion_effect` is its sole effect executor.

`State::launch_completion_host_command` is the host command crossing.

Native tests record launches in `launched_completion_effects`.

They may directly execute the exported CLI transaction to cross the real Git
boundary deterministically.

`PendingCompletion` retains the key, correlation, deadline, prior ticket
facts, source, authority, and replay flag.

`handle_completion_result` accepts results only for a live matching pending
entry and current authority.

Successful result processing also requires durable Done frontmatter.

Confirmation is journaled before scheduler release.

## Artifact ordering

`check_artifact_advances` scans Running threads to a fixpoint.

For Implement, `review.md` is the completion artifact.

An Implement thread with private Review evidence advances to Review.

The same fixpoint then observes Review's next phase as Done.

That edge routes through `CompletionInput::Artifact`.

`review-disposition.json` is admitted during completion input derivation.

The ticket file remains Review until the isolated completion transaction.

This allows Review to exist before and through Implement-to-Review.

## Slot-transition ordering

`poll_tick` checks artifacts before transition signals.

It reconciles Review completion before transition handling and timeouts.

`handle_stopped_signal` distinguishes slot transition from Review completion.

A `WaitingForStop` slot receives `/clear` and moves to WaitingForClear.

An Idle Review slot routes Stop into typed completion.

An already pending completion suppresses either source from launching again.

The recorded hostile Stop can therefore occur during an active slot transition.

## Reload and delayed result

`completion_journal::load` reconstructs aggregates in a fresh `State`.

`mask_completion_transaction` hides unconfirmed Done from scheduler authority.

`reconciliation_state` prefers restored aggregate state over raw ticket bytes.

An in-flight generation can be replayed with its original key before deadline.

Duplicate Stop and Reconcile observations are suppressed while replay is live.

Late results without a pending entry are ignored.

Only a valid commit-shaped result plus durable Done can append Confirmed.

## CLI transaction boundary

`crates/lisa-cli/src/commit_transaction.rs` exports `complete_ticket` for tests.

`CompleteTicketRequest` takes a Git root, ticket ID, message, ticket path,
work path, and completion generation.

The transaction uses an isolated index and exact repository-relative paths.

An idempotent replay returns the original commit with no newly committed paths.

The nested-path regression already proves `State::build_completion_command`.

For `games/midsummer`, `--path` is the Git root.

`--ticket-file` is
`games/midsummer/docs/active/tickets/<ticket>.md`.

`--work-dir` is
`games/midsummer/docs/active/work/<ticket>`.

## Completion side effects

After durable confirmation, the plugin marks the thread complete.

It emits authoritative Done provenance through `emit_provenance`.

It revokes the current lease in `release_slot_for_ticket`.

It clears the slot's ticket and attempt records.

It removes the completed thread.

It immediately calls `schedule_ready_tickets`.

The scheduler reads the rebuilt DAG and can assign a dependent.

Native scheduling requires permissions, discovered slots, and an eligible pane.

## Existing native test seams

The parent test module exposes `install_current_attempt`.

It exposes helpers for private Review and disposition files.

`codex_slot` creates a native provider seat.

`read_mixed_ledger` parses authoritative provenance records.

The operator recovery matrix demonstrates real `d` then Enter gestures.

The nested transaction test demonstrates argv decoding into the CLI request.

The lost-result regression demonstrates journal restoration and replay.

Existing tests cover those boundaries separately rather than in one sequence.

## Repository state and constraints

The ordinary worktree contains Lisa-managed ticket and provenance changes.

It also contains an unrelated untracked `crates/lisa-plugin/docs/` directory.

Those paths must remain untouched.

Attempt artifacts belong only under
`.lisa/attempts/T-042-04-01/1/work`.

Ticket phase and status must not be edited by this attempt.

Ticket-owned source must be committed with `lisa commit-ticket` and exact
repository-relative includes.

The likely source ownership is limited to plugin-native test registration and
a focused test module.

No live Codex seat is required by this ticket; that belongs to `T-042-04-03`.

