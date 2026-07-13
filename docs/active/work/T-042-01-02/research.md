# Research: fold all completion sources and quarantine the boolean path

## Assignment boundary

T-042-01-02 is the fourth completed dependency step in story S-042-01.
T-042-01-01 introduced the plugin-side typed completion adapter for artifact
polling and stopped Review sessions. T-042-01-05 and T-042-01-06 subsequently
fixed and tested the Git-root command contract used by that adapter. This
ticket folds the remaining production origins through the same seam and
removes the temporary boolean request bridge.

The named origins in the ticket and story are idle, timeout/reload
reconciliation, externally observed Done, and the manual UI. The existing
artifact/poll and stopped origins are already typed and remain in scope as
compatibility constraints. The lisa-core reducer is a read-only dependency.

T-042-01-03 owns new level-triggered eligibility behavior on poll and load.
This ticket owns the typed vocabulary and routing boundary that reconciliation
will use; it does not add the later eligibility derivation policy. T-042-01-04
owns detailed reducer rejection and correlation rendering. Story B owns
durable completion journaling, and Story C owns broader operator-authority
semantics.

## Core completion aggregate

`crates/lisa-core/src/completion.rs` defines `CompletionEvent`,
`CompletionState`, `EffectCommand`, identifiers, transitions, and rejections.
The request event is `CompletionEvent::Request { attempt_id, completion_id }`.
An eligible request produces `EffectCommand::LaunchCompletion`; a duplicate
request in Requested state produces the typed AlreadyPending rejection.

The core event does not carry scheduler-origin metadata. Origin remains a
plugin diagnostic concern represented by `CompletionSource`. The plugin
therefore needs a typed input enum that retains origin-specific evidence and
then maps every input to the common typed core request.

The reducer is pure and performs no command launch or filesystem mutation.
The plugin executor is responsible for consuming returned effects.

## Current plugin vocabulary

`crates/lisa-plugin/src/lib.rs` imports `reduce`, `AttemptId`,
`CompletionEvent`, `CompletionId`, `CompletionState`, and `EffectCommand`.
`CompletionSource` currently has Artifact, Idle, Stopped(pane), Manual, and
ObservedDone variants. It is stored in `PendingCompletion` and appears in
diagnostic messages and result-authority checks.

`CompletionInput` currently has only Artifact and Stopped variants. Both carry
a ticket ID and current `AttemptLease`; Stopped additionally carries pane ID.
The enum comment explicitly says remaining origins migrate in successor work.

`CompletionAuthority` distinguishes an attempt lease and an operator.
`PendingCompletion` stores the prior phase/status, diagnostic source, and
authority. `State::pending_completions` is the current in-memory aggregate
bridge and duplicate mask.

## Existing typed dispatcher

`State::dispatch_completion` accepts `CompletionInput` and returns bool. It
maps each typed input into ticket, diagnostic source, and source lease. It
admits and parses the passing Review disposition before reducing the request.

The dispatcher derives core state as Requested when the pending map contains
the ticket and Eligible otherwise. It constructs AttemptId from the lease's
numeric generation and CompletionId from the ticket ID. It sends the typed
event to the pure reducer. If the reducer returns an effect, the dispatcher
passes it to `execute_completion_effect`; otherwise it returns false.

Artifact polling calls this dispatcher from `check_artifact_advances` when
Review advances toward Done. Stopped signals call it through
`auto_complete_review` after recovering the exact pane lease. Missing leases
are rejected before dispatch with visible warnings.

## Review admission boundary

`admit_passing_review` publishes `review-disposition.json` from an
attempt-private work directory when a lease is supplied. Admission validates
that the candidate is the exact current lease. Historical unleased fixtures
are accepted only when the ticket has no registered current lease.

The canonical disposition is parsed as Pass, Block, or Invalid. Pass returns
true. Block and Invalid log distinct activity messages and return false.
Artifact and stopped inputs call this helper inside typed dispatch.

Manual and externally observed Done currently do not call Review admission.
That is existing behavior: manually observed durable Done may no longer have a
Review artifact available to admit, and the UI path historically permits an
operator request. Origin-specific admission therefore cannot be assumed to be
identical for every new typed input.

## Temporary boolean bridges

`request_review_completion` is a temporary helper returning bool. It admits a
passing Review, then calls `request_completion`. The only production callers
are two idle-signal branches: a catch-up path after Implement advances to
Review with `review.md` already present, and a Review idle path whose artifact
was admitted.

`request_completion` also returns bool. It accepts untyped ticket, source, and
optional authority arguments. It fabricates a LaunchCompletion effect without
calling the reducer, then calls the effect executor. Its production callers
are the idle bridge, externally observed Done reconciliation in `poll_tick`,
and `mark_ticket_done` from the manual modal.

The fabricated-effect wrapper is the second path around the reducer. Although
the actual host command launch is already centralized, these callers can reach
the executor without emitting `CompletionEvent::Request`. The acceptance
criterion specifically requires this boolean path to be deleted or no longer
return bool, and requires a regression guard against its reintroduction.

## Idle completion origin

`check_idle_signals` consumes pane or legacy-ticket idle records for running
threads. For Implement it publishes progress, advances disk and thread state
to Review, and checks whether Review already exists. When it does, the method
calls `request_review_completion` with CompletionSource::Idle.

For Research through Review, idle requires the current phase artifact. If the
next phase is Done, the Review branch recovers the thread lease and calls the
same boolean Review helper with CompletionSource::Idle. Both branches already
have attempt-scoped evidence available, though one carries it in the local
snapshot and the other looks it up again.

## Externally observed Done and reconciliation

`poll_tick` rebuilds the DAG after signal, transition, health, and timeout
processing. It then collects running threads whose rescanned ticket phase is
Done. The comment describes this as externally observed Done entering the
same commit transaction and notes that the pending mask prevents premature
publication while a command result is outstanding.

Each collected item contains a ticket ID and optional thread lease. The loop
calls `request_completion` with CompletionSource::ObservedDone and converts an
existing lease into attempt authority. A missing lease reaches the executor
and is rejected by its authority gate.

This poll section is also the existing timeout/reload reconciliation boundary:
it runs after timeout handling and DAG rebuild and reconciles scheduler thread
state with durable ticket state observed from disk. Plugin `load` currently
builds the DAG but does not derive passing-Review eligibility or dispatch a
completion request. The new load/poll level-triggered behavior belongs to the
dependent T-042-01-03.

## Manual UI origin

`mark_ticket_done` is invoked by the mark-done modal confirmation path. If a
thread exists, it takes that thread's optional attempt lease as authority. If
no thread exists, it supplies explicit Operator authority. It then calls the
untyped boolean request wrapper with CompletionSource::Manual.

The executor currently accepts Operator only when source is Manual. An
attempt-authorized manual request must still present the current lease. These
rules are also checked when a command result arrives: operator result authority
is current only when the stored pending source is Manual.

## Sole effect executor

`execute_completion_effect` exhaustively matches LaunchCompletion and validates
effect ticket/attempt identity against the supplied authority. It rejects
duplicates, stale or missing leases, unauthorized operators, incomplete
dependencies, and missing ticket files. It inserts PendingCompletion, records
the effect in native tests, builds the command, and contains the only
production call to Zellij's command runner for completion.

The executor returns bool because many existing tests assert admission or
rejection directly. The acceptance criterion names `request_completion`, not
the executor. Removing boolean dispatch does not require rewriting all
executor tests or result semantics.

## Tests and structural guard

Plugin tests are colocated in `lib.rs`. Existing tests directly call
`request_completion` four times, so deleting the wrapper requires migrating
those tests to typed dispatch or direct effect-executor tests according to the
behavior under test.

The State test field `launched_completion_effects` records effects accepted at
the sole production executor. Existing artifact/stopped tests assert one
effect and duplicate suppression. Manual tests inspect pending source and
completion results.

A behavioral test alone would not fail merely because a future developer
introduced an unused second boolean method. A source-shape invariant can read
the plugin source with `include_str!("lib.rs")` and reject the legacy method
name/signature and direct effect-executor call sites outside the dispatcher.
Because the test is compiled from the same file, it is deterministic and does
not depend on repository working-directory discovery.

## Repository and workflow constraints

At assignment start, Lisa-managed provenance and active ticket files were
already modified. `crates/lisa-plugin/docs/` was already untracked. These paths
are unrelated and must remain untouched by the source transaction.

The expected ticket-owned source unit is
`crates/lisa-plugin/src/lib.rs`. It must be committed only through
`lisa commit-ticket` with that exact repository-relative include. Phase
artifacts remain private in this attempt directory for Lisa to admit and
publish. Native workspace tests, formatting, and WASM lint remain required
verification boundaries.
