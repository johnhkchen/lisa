# Research: operator modal durable confirmation

## Ticket and story position

T-042-03-02 is the second task in story S-042-03.

The story makes operator recovery through the `[d]one` UI a first-class
completion source.

The predecessor T-042-03-01 introduced explicit operator authority.

The predecessor T-042-01-04 introduced named, correlated rejection activity.

This ticket begins with both predecessors present at HEAD.

Its acceptance criterion is specifically about modal lifetime and visible
request outcome.

The ticket does not own the completion reducer's eligibility rules.

It does not own journal persistence across plugin restart.

It does not own the story's complete seven-case recovery matrix.

T-042-03-03 follows this ticket and owns that broader matrix.

## Current operator input path

`crates/lisa-plugin/src/lib.rs` contains the plugin state and key handling.

`State::handle_key` handles modal input before normal dashboard input.

Normal-mode `d` calls `State::open_mark_done_modal`.

The modal lists eligible non-Done tickets from the current DAG.

It normally excludes tickets with running threads.

Review tickets remain selectable even when a thread is running.

Implement tickets with an existing `review.md` also remain selectable.

The modal stores a sorted list and a cursor.

Enter clones the selected ticket ID before invoking the action.

For MarkDone mode, Enter calls `State::mark_ticket_done`.

`mark_ticket_done` constructs `CompletionInput::OperatorRequested`.

The request source is `OperatorRequestSource::MarkDoneKey`.

The request enters `State::dispatch_completion`.

There is no alternate completion command launch in the key handler.

After the action returns, the current handler unconditionally sets
`self.modal.open = false`.

The handler does not inspect whether dispatch launched, rejected, or remained
unresolved.

That unconditional close is the observable gap named by this ticket.

## Current modal model

The internal modal type is `MarkDoneModal` in `lib.rs`.

Despite its name, it represents MarkDone, ResetTicket, and QuitConfirm modes.

Its fields are `open`, `ticket_ids`, `cursor`, `mode`, and
`new_ticket_ids`.

`ModalMode` distinguishes the three action families.

No internal field records a submitted completion ticket.

No internal field records completion correlation.

No internal field records pending, accepted, or rejected status.

The UI representation is `ui::ModalState` in
`crates/lisa-plugin/src/ui.rs`.

It mirrors the same list, cursor, kind, and quit-only ticket list.

`State::to_ui_state` copies internal modal fields into the UI model.

The UI model likewise has no completion-request outcome.

`ui::render_modal` renders MarkDone and ResetTicket with one shared layout.

The title differs by `ModalKind`.

The body is always the selectable ticket list.

The footer is always `Enter=confirm  Esc=cancel`.

QuitConfirm uses a separate renderer and separate key branch.

## Completion adapter boundary

`State::dispatch_completion` is the typed request gateway.

It returns a boolean indicating whether an effect was executed.

Operator requests use `CompletionAuthority::Operator`.

Their attempt identity is the stable string `operator`.

The completion ID is the ticket ID.

`State::completion_correlation` combines completion ID, attempt ID, and
generation 1 into `CompletionGenerationId`.

That generation identity is already used as the visible operator correlation.

Before reduction, operator requests must pass the canonical Review
disposition.

A blocked or invalid disposition returns a typed `DispositionBlocked`
rejection.

The pure reducer receives `CompletionEvent::Request`.

Requested or in-flight aggregate state rejects a duplicate as
`AlreadyPending`.

An accepted reducer transition yields the inert launch effect.

`State::execute_completion_effect` is the sole host-command launch boundary.

It rechecks source authority, dependencies, ticket identity, and command
construction.

Successful launch preparation inserts `PendingCompletion`.

`PendingCompletion` retains the completion generation, command correlation,
prior ticket state, request source, and authority.

For operator input, its source stays
`OperatorRequested(MarkDoneKey)`.

The pending entry exists until a correlated command result is durably handled.

## Request result boundary

Zellij delivers completion command results as `Event::RunCommandResult`.

The command context stores `lisa_completion=<ticket-id>`.

`State::update` routes such results to `handle_completion_result`.

The result handler first looks up the live pending entry.

A result with no pending entry is ignored.

Non-current authority is journaled as retryable rejection before removal.

A nonzero exit or invalid commit ID is journaled as retryable rejection.

That failure becomes the named `LaunchFailed` activity kind.

Its detail includes exit state, stderr, source, and recoverability.

A zero exit and commit-shaped stdout are not sufficient alone.

The ticket must also scan as durably Done.

The confirmed journal transition must persist.

Only after both checks does the handler remove pending state.

It then rebuilds the DAG, logs completion activity, releases the seat, emits
provenance, and schedules dependents.

This is the existing durable acceptance boundary.

## Poll behavior

Periodic `Event::Timer` calls `State::poll_tick` when the poll deadline fires.

`poll_tick` processes signals, artifacts, reconciliation, timeouts, scheduling,
and DAG rebuilding.

The timer event requests a rerender.

Polling does not currently reset modal state.

An open modal therefore survives a poll already.

The current defect occurs before polling because Enter closes it.

Pending completion masks prematurely written Done frontmatter during DAG
rebuild.

Journal aggregate state supplies the same mask after restart.

Pending completion also prevents audit code from reclaiming the thread.

Thus the scheduler already treats the request as unresolved across polls.

Only the modal fails to expose that lifecycle.

## Named rejection projection

`State::log_completion_rejection` exhaustively maps five operator-relevant
reducer/adapter outcomes.

The stable kinds are `already-pending`, `stale-lease`,
`disposition-blocked`, `dependency-blocked`, and `launch-failed`.

The mapped type is `lisa_core::types::CompletionRejectionKind`.

Every mapped activity event carries ticket ID, kind, correlation ID, and
detail.

Unexpected-event and correlation-mismatch currently remain warning events.

Those are reducer protocol errors rather than the named operator matrix.

`activity_event_to_ui_entry` preserves all four rejection fields.

Both full Activity and filtered Operations views render the exact kind and
correlation.

The modal does not consume or duplicate that projection today.

Immediate operator rejection is therefore visible only after the modal closes
and the dashboard becomes visible again.

An in-flight request has an Info activity entry but no modal confirmation.

## Existing tests

`test_mark_done_keeps_thread_and_slot_until_commit_result` drives the actual
key handler.

It opens the modal, presses Enter, and asserts the operator pending entry.

It currently does not assert modal state after Enter.

That test is a direct regression point for this ticket.

`test_mark_done_without_active_attempt_uses_operator_authority` calls the
action directly.

`test_operator_requested_refuses_blocked_disposition_and_unmet_dependencies`
asserts named correlated activity entries.

`failed_operator_completion_retries_without_early_release_or_duplicate_provenance`
asserts result failure and successful retry behavior.

UI tests cover MarkDone and ResetTicket titles.

UI tests cover all five rejection kinds in both activity views.

No test renders a pending, accepted, or rejected modal outcome.

No test presses Enter and then advances a timer poll while inspecting the
modal.

## Constraints and ownership

The worktree contains unrelated changes in core completion files,
completion-journal code, provenance, and another ticket's artifacts.

`crates/lisa-plugin/src/lib.rs` and `crates/lisa-plugin/src/ui.rs` are clean at
research time.

Ticket-owned edits must avoid the unrelated modified paths.

Source changes must be committed with `lisa commit-ticket` and exact include
paths.

The ordinary Git index must not be used.

Phase artifacts belong only in this attempt's private work directory.

The ticket frontmatter phase is Lisa-owned and must remain untouched.

The core reducer, journal schema, CLI, and provider adapters are outside the
necessary change boundary.

The implementation must preserve the sole typed completion gateway.

The implementation must preserve explicit operator authority.

The modal status is presentation state, not a new source of scheduling or
completion authority.
