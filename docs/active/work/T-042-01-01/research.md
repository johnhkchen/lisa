# Research: completion effect adapter seam

## Assignment boundary

T-042-01-01 is the first ticket in S-042-01. It introduces a gateway in
`lisa-plugin` for two production completion origins: artifact polling and
stopped Review sessions. Later tickets own idle, reload/reconciliation,
externally observed Done, and the manual operator path. The E-041 reducer is a
read-only dependency.

## Existing core domain

`crates/lisa-core/src/completion.rs` contains the pure completion aggregate.
Its states are `Eligible`, `Requested`, `CommandInFlight`, `Rejected`, and
`Confirmed`. `CompletionEvent::Request` carries typed `AttemptId` and
`CompletionId` values. `reduce(Eligible, Request)` returns `Requested` plus one
`EffectCommand::LaunchCompletion`. Repeated requests in a pending state return
the typed `AlreadyPending` rejection. The reducer performs no I/O.

## Existing plugin state

`crates/lisa-plugin/src/lib.rs` owns completion orchestration.
`CompletionSource` distinguishes Artifact, Idle, Stopped, Manual, and
ObservedDone origins. `CompletionAuthority` distinguishes a current
`AttemptLease` from an operator. `PendingCompletion` retains prior phase/status,
source, and authority. `State::pending_completions` is the existing in-memory
duplicate gate and masks premature Done observations. It does not yet store
E-041 states or correlation IDs.

## Review admission

`State::request_review_completion` admits `review-disposition.json` from the
attempt-private directory. `State::admit_artifact` checks the supplied lease is
current before publishing attempt bytes. The canonical disposition is parsed
with `parse_review_disposition`. Only `ReviewDisposition::Pass` proceeds;
Block and Invalid are logged and return false. The method then calls the
boolean `request_completion` path.

## Current command path

`State::request_completion` rejects an already-pending ticket, validates
attempt/operator authority, checks dependencies, resolves the ticket file,
captures prior phase/status, and inserts `PendingCompletion`. It builds
`lisa complete-ticket` argv and directly calls Zellij
`run_command_with_env_variables_and_cwd`. In native tests, unavailable command
configuration short-circuits after the accepted pending edge.

## Artifact/poll origin

`State::check_artifact_advances` loops over running threads so multiple existing
artifacts catch up in one poll. For Review, `review.md` is admitted through the
current attempt lease. When the next phase is Done, it calls
`request_review_completion` with `CompletionSource::Artifact`. The pending
transaction, not the poller, owns Done publication.

## Stopped origin

`signal.rs` normalizes `pane-<id>.stopped` into `SignalRecord::Stopped`.
`check_transition_signals` calls `handle_stopped_signal`. For an idle slot whose
assigned ticket is in Review and whose thread remains completable,
`auto_complete_review` recovers the slot lease and calls
`request_review_completion` with `CompletionSource::Stopped(pane_id)`.
Artifact and stopped inputs therefore converge only at an untyped Review helper
and boolean completion function.

## Result path

Zellij command results are attributed using `lisa_completion` context.
`handle_completion_result` verifies authority remains current, rejects nonzero
exits or invalid commit IDs, rebuilds the DAG on success, and verifies durable
Done frontmatter. Only then does it finish the thread, release the seat, and
allow dependents to schedule. This ticket does not own result correlation or
durable reconstruction.

## Tests and constraints

Plugin tests are colocated in `lib.rs`. Helpers create temporary tickets,
install a current attempt, and write attempt-private artifacts. Existing tests
assert a passing Review creates a pending entry while disk remains Review.
Native Zellij calls are shims, so a new explicit observable is needed to prove
effect execution rather than only pending insertion.

The ticket must preserve lease, disposition, dependency, pending, and result
gates. Only artifact and stopped inputs move through the typed seam now. The
actual completion host launch must have one production location. Legacy callers
remain functional until successor T-042-01-02 folds them. No new dependency is
needed. The plugin must continue targeting `wasm32-wasip1`.

At assignment start, Lisa-managed provenance and ticket frontmatter were dirty,
and `crates/lisa-plugin/docs/` was already untracked. These are unrelated. The
expected owned source path is `crates/lisa-plugin/src/lib.rs`, committed only
through exact-path `lisa commit-ticket`.
