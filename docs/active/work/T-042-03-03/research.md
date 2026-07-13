# Research: operator recovery test matrix

## Ticket position

T-042-03-03 is the final task in story S-042-03.

The story makes the `[d]one` operator gesture a first-class completion source.

T-042-03-01 introduced explicit `OperatorRequested` authority.

T-042-03-02 introduced durable visible modal outcomes.

Both predecessor tickets are complete at the current repository HEAD.

This ticket is a verification barrier over their settled behavior.

Its acceptance criterion names seven lifecycle cases.

Those cases are active Review, orphaned Review, blocked disposition, stale
attempt, already-pending, launch failure, and successful recovery.

Each case must expose either an accepted transition or a named rejection with
correlation.

The story explicitly limits the tests to the native UI/adapter boundary.

It does not require a live Codex seat or provider token use.

## Operator input path

`crates/lisa-plugin/src/lib.rs` owns plugin state and keyboard handling.

Normal-mode `d` calls `State::open_mark_done_modal`.

The modal selects non-Done tickets from the current DAG.

A Review ticket remains selectable even when it has a running thread.

An orphaned Review is selectable because it has no running thread.

Enter in MarkDone mode calls `State::mark_ticket_done`.

That method emits `CompletionInput::OperatorRequested`.

The input carries `OperatorRequestSource::MarkDoneKey`.

The input does not carry an `AttemptLease`.

`State::dispatch_completion` is the single typed completion gateway.

Operator input becomes `CompletionAuthority::Operator`.

Its stable attempt identity is the string `operator`.

The completion ID is the selected ticket ID.

Generation 1 combines those values into `CompletionGenerationId`.

The same generation ID is the visible operator correlation.

## Eligibility and rejection boundaries

Operator requests first parse the canonical Review disposition.

A Pass disposition continues to the reducer.

A Block disposition becomes `CompletionRejection::DispositionBlocked`.

Missing or invalid disposition data also becomes disposition-blocked.

The pure reducer receives `CompletionEvent::Request`.

An eligible aggregate returns one `LaunchCompletion` effect.

A requested or command-in-flight aggregate returns `AlreadyPending`.

The effect executor rechecks authority, dependencies, and ticket identity.

Unmet dependencies become `DependencyBlocked`.

A stale attempt authority becomes `StaleLease` for attempt-driven input.

Operator authority is independent of the thread's attempt lease.

Therefore a stale thread record does not itself make an operator request stale.

Command construction failures become `LaunchFailed`.

Command result failures also become `LaunchFailed`.

`State::log_completion_rejection` maps domain errors to stable UI kinds.

The five stable kinds include already-pending, stale-lease,
disposition-blocked, dependency-blocked, and launch-failed.

Every structured rejection contains ticket ID, kind, correlation, and detail.

## Accepted request representation

An accepted request appends requested and in-flight journal transitions when a
real journal path is configured.

It then inserts `PendingCompletion` keyed by ticket ID.

`PendingCompletion` retains completion generation, command correlation,
deadline, prior ticket state, source, and authority.

In native tests an empty journal path preserves disk-free fixture behavior.

The test-only executor also permits a missing command when the journal path is
empty.

That path still records the inert launch effect and live pending entry.

It does not invoke a host command.

This is the intended stubbed-executor seam described by the story.

## Modal outcome representation

`MarkDoneModal` has optional `OperatorModalOutcome` state.

The variants are Pending, Accepted, and Rejected.

All three carry ticket ID and full correlation string.

Rejected additionally carries the stable rejection kind and detail.

An accepted dispatch projects the live pending generation into Pending.

An immediate rejection projects the structured rejection into Rejected.

A successfully verified result projects the same generation into Accepted.

Pending prevents repeated submission and silent dismissal.

Terminal outcomes remain visible until explicit acknowledgement.

Modal updates are isolated to the selected ticket.

Background completion activity for another ticket cannot overwrite the modal.

## Durable success boundary

`State::handle_completion_result` consumes command results.

The result must correspond to a live pending entry.

The source authority must still be valid.

For operator requests, validity is tied to the operator source rather than an
attempt lease.

The exit code must be zero.

Stdout must contain a 40- or 64-character hexadecimal commit ID.

The ticket must scan from disk with both phase and status equal to Done.

When a journal is configured, confirmation must persist.

Only then does the modal become Accepted.

The pending entry is removed after confirmation.

The DAG is rebuilt and the thread and slot are released.

Done provenance is emitted and dependents may be scheduled.

A nonzero result retains the ticket for retry and yields launch-failed.

## Existing test coverage

The main `#[cfg(test)]` module in `lib.rs` contains operator-focused tests.

`test_mark_done_keeps_thread_and_slot_until_commit_result` covers an active
Review through the key handler.

It verifies explicit operator authority, Pending modal state, and poll
stability.

`test_mark_done_without_active_attempt_uses_operator_authority` covers an
orphaned Review by invoking the action method directly.

`test_mark_done_already_pending_keeps_named_correlated_rejection_visible`
covers duplicate submission and modal feedback.

`test_operator_requested_refuses_blocked_disposition_and_unmet_dependencies`
covers two gate failures by invoking the action method directly.

`failed_operator_completion_retries_without_early_release_or_duplicate_provenance`
covers command-result failure, retry, and success.

The tests collectively exercise much of the required behavior.

They are separated across distant sections of a very large test module.

They do not present the seven named story cases as one auditable matrix.

Not every case drives the real `d` then Enter gesture.

There is no focused stale-thread-attempt operator case.

## Test module organization

The parent test module already loads focused child modules from
`crates/lisa-plugin/src/tests/`.

`signal_consumer_characterization.rs` and
`signal_ingestion_regression.rs` use `use super::*`.

Child modules can access private parent-module helpers and production-private
types through Rust's descendant privacy rules.

The parent module contains `install_current_attempt` and canonical disposition
writers.

It also exposes the state, modal, completion, thread, slot, and key types needed
by a focused matrix fixture.

`tempfile` is already a plugin development dependency.

Native plugin tests run through `cargo test -p lisa-plugin`.

The workspace verification command is `cargo test --workspace`.

The project quick check is `just check`.

## Repository constraints

The worktree already contains Lisa-owned modifications to provenance and the
current ticket.

It also contains untracked plugin-relative artifact files created outside this
ticket's scope.

Those paths must not be included in ticket commits.

Ticket-owned source must be committed with `lisa commit-ticket` and exact
repository-relative includes.

Ordinary `git add` and `git commit` are prohibited by the assignment.

Phase artifacts belong in the private attempt work directory.

Lisa publishes admitted artifacts after lease verification.

Ticket phase and status frontmatter are Lisa-owned and must not be edited by
this attempt.
