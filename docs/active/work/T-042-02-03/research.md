# Research: bounded reconciliation replay convergence

## Ticket boundary

T-042-02-03 is the last ticket in story S-042-02.

The story concerns durable completion intent, result identity, idempotent command
execution, and bounded reconciliation.

The immediate predecessor, T-042-02-02, introduced the append-only completion
journal and restart reconstruction.

T-042-02-01 introduced `CompletionGenerationId` and made the CLI completion
transaction idempotent for that key.

This ticket starts in Research and has one acceptance criterion.

The required hostile sequence includes a lost result, plugin reload, duplicate
stop observation, and timeout.

The expected convergence is one prior completion commit and one authoritative
Done publication.

An unresolved `CommandInFlight` must also leave the retry loop after a bounded
deadline in a named retryable or action-required state.

The story explicitly excludes a live-provider run and the later Story D hostile
ordering harness.

## Core completion domain

`crates/lisa-core/src/completion.rs` owns the adapter-neutral completion model.

Opaque `AttemptId`, `CompletionId`, and `CorrelationId` newtypes prevent string
identities from being interchanged accidentally.

`CompletionGenerationId` binds completion/ticket identity, attempt identity,
and a numeric generation.

Its stable display form is used by both journal records and Git commit metadata.

`CompletionState` currently contains `Eligible`, `Requested`,
`CommandInFlight`, `Rejected`, and `Confirmed`.

`CommandInFlight` already carries a mandatory `CorrelationId`.

It does not carry a launch time, deadline, or remaining retry budget.

`Rejected` carries a typed `CompletionRejection` and `Retryability`.

`Retryability` has the two named outcomes `Retryable` and `ActionRequired`.

`CompletionEvent::CommandLaunched` changes `Requested` into
`CommandInFlight`.

A correlation-matched `CommandSucceeded` changes it to `Confirmed`.

A correlation-matched `CommandFailed` changes it to `Rejected` with the event's
retryability.

Mismatched results are rejected without changing aggregate state.

`reconcile` is level-triggered over durable inputs and aggregate state.

Eligible and retryable-rejected state can emit `LaunchCompletion`.

Requested, Confirmed, and action-required Rejected suppress effects.

CommandInFlight currently returns
`Reconciliation::CommandInFlightActionRequired` immediately.

That decision carries the correlation but does not itself change state.

The core has no clock input and no deadline representation.

Core unit tests validate correlation matching, retryable rejection, and the
current in-flight action-required reconciliation decision.

`crates/lisa-core/tests/completion_state_machine.rs` uses generated event
sequences against a small reference model.

Its harness treats an in-flight action-required decision as an assertion that
one live effect still exists.

`crates/lisa-core/tests/recorded_livelock_regression.rs` replays an older
artifact-before-phase trace.

That regression proves level-triggered eligibility but does not execute Git or
model a lost host command result.

## Durable completion journal

`crates/lisa-plugin/src/completion_journal.rs` owns JSONL persistence.

Each append rereads and folds the complete journal before atomically replacing
the destination through `RustPublication`.

The current schema version is 1.

Record bodies are Requested, CommandInFlight, Rejected, and Confirmed.

Requested stores the generation key plus prior phase and status.

CommandInFlight stores the generation key and correlation ID.

It stores no wall-clock deadline.

Rejected stores optional correlation, reason text, and retryability.

Confirmed stores correlation and commit ID.

`CompletionJournalAggregate` retains the latest completion key, state, prior
phase/status, and optional confirmed commit ID.

Requested and CommandInFlight aggregates mask durable Done ticket bytes back to
their prior phase and status.

`load` treats absence as an empty journal and malformed/torn history as an
error.

`apply_transition` folds journal records through the core reducer.

A Requested record for a new key may follow a Rejected or Confirmed aggregate.

An in-flight Rejected record must carry the matching correlation.

Journal tests cover round-trip reconstruction, invalid histories, atomic
publication, and retryable generation changes.

There is no record that updates only an in-flight deadline or replay count.

## Plugin completion adapter

`crates/lisa-plugin/src/lib.rs` is the scheduler and production adapter.

`State` stores the journal path, journal health, reconstructed aggregates, and
live `pending_completions`.

`PendingCompletion` contains the generation key, correlation, prior phase and
status, source, and authority.

It has no deadline or replay counter.

`restore_completion_journal` runs before initial DAG authority is derived.

A restore error clears aggregates, marks the journal unhealthy, and fails
completion scheduling closed.

`mask_completion_transaction` prevents unconfirmed Done bytes from releasing
the scheduler.

`reconciliation_state` prefers the reconstructed journal aggregate.

`review_completion_inputs` admits attempt-private Review artifacts only after
current-lease validation.

`reconcile_review_completions` scans current Review threads and dispatches a
typed Reconcile input on every scheduler observation boundary.

For a reconstructed CommandInFlight, `dispatch_completion` currently logs a
warning naming the correlation and emits no effect.

It does not relaunch the idempotent command.

It does not append a terminal rejection.

It does not compare the state with elapsed time.

`execute_completion_effect` is the sole initial command launch boundary.

It computes generation 1 from ticket plus authoritative attempt.

It persists Requested and then CommandInFlight before calling Zellij
`run_command`.

It inserts `PendingCompletion` before crossing the host boundary.

The command context currently contains only `lisa_completion=<ticket-id>`.

`RunCommandResult` uses that context to call `handle_completion_result`.

The handler looks up the live pending entry by ticket ID.

A result after plugin reload is ignored when no live pending entry exists.

A nonzero result or malformed commit ID appends retryable Rejected.

A successful result verifies durable Done bytes before appending Confirmed.

Only after durable confirmation does it emit authoritative Done provenance,
release the seat, remove the thread, and schedule dependents.

A lost successful result therefore leaves Git and ticket bytes Done while the
journal remains CommandInFlight and continues masking those bytes.

## Scheduler timing boundary

`poll_tick` already runs every five seconds through `POLL_INTERVAL_SECS`.

It checks artifacts, then calls level-triggered Review reconciliation before
Review timeout follow-ups and later DAG rebuild/audit work.

Existing deadline machinery in `deadline.rs` handles session health, startup,
assignment acknowledgement, transitions, and Review nudges.

Completion reconciliation is not represented in that module.

The plugin already uses `SystemTime` for persisted-independent scheduler
deadlines and accepts explicit time values in several testable helpers.

Plugin restart destroys live `pending_completions` but preserves the JSONL
journal and Git repository.

Current attempt leases and threads are recovered through scheduler-specific
paths rather than the completion journal.

## CLI idempotency boundary

`crates/lisa-cli/src/commit_transaction.rs` owns isolated Git commits.

`complete_ticket` validates that the completion key's ticket matches the
request.

It prepares Done frontmatter and calls the isolated transaction with the exact
ticket file and work directory includes.

Completion commit messages contain an exact `Lisa-Completion-Key:` trailer.

Before creating a commit, `discover_completion_commit` searches reachable Git
history for that marker and verifies an exact message line.

When a prior key is found, the CLI returns that prior commit ID with no committed
paths and does not advance HEAD.

The current CLI regression proves same-key replay remains idempotent even after
an unrelated later commit.

Different generation keys remain independent.

The plugin already builds replay-capable `complete-ticket` argv from the durable
generation key.

No CLI source change is required merely to invoke that existing behavior again.

## Existing restart regression

The plugin test
`completion_journal_reconstructs_restart_states_before_authoritative_provenance`
drives a Review completion into CommandInFlight.

It asserts Requested and CommandInFlight records, exact key reconstruction,
Done masking, and replacement-scheduling fencing.

It then supplies the original live state's successful result and verifies one
Confirmed record and one authoritative provenance record.

The restarted state in that test never replays the command.

The test uses a synthetic 40-character commit ID rather than a real Git
completion transaction.

It therefore does not count commits or prove convergence to the CLI's prior
completion commit after lost result.

## Repository and workflow constraints

The ordinary worktree already contains Lisa-owned modifications to provenance
and active ticket files.

There is also an unrelated untracked `crates/lisa-plugin/docs/` tree.

Those paths are outside this ticket's source ownership and must remain
untouched.

Phase artifacts belong only in this attempt's private work directory.

Ticket-owned source changes must be committed with `lisa commit-ticket` and
exact repository-relative include paths.

The ticket frontmatter must not be manually advanced.

Review requires both `review.md` and the exact disposition JSON shape.

## Observed gap

All pieces needed to identify and idempotently replay a completion already
exist across core, plugin journal, and CLI.

The missing connection is a durable deadline plus a scheduler transition that
replays the same key while bounded and persists a named terminal rejection when
the deadline expires.

The lost-result regression must cross the real CLI transaction boundary to
distinguish convergence from merely suppressing a second in-memory effect.
