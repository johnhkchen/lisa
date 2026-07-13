# Progress: operator modal durable confirmation

## Outcome

Implementation is complete.

The MarkDone modal now remains open throughout the operator completion
request lifecycle.

It shows a durable in-memory Pending outcome while the command is unresolved.

It shows Accepted only after the existing durable confirmation boundary.

It shows Rejected with the exact named kind, correlation ID, and actionable
detail.

Terminal outcomes remain visible until explicit operator acknowledgement.

Pending cannot be silently dismissed or resubmitted from the modal.

## Step 1: modal outcome vocabulary

Completed in `crates/lisa-plugin/src/lib.rs` and
`crates/lisa-plugin/src/ui.rs`.

Added internal `OperatorModalOutcome`.

Added presentation `ui::OperatorModalOutcome`.

Both types model Pending, Accepted, and Rejected.

Every variant retains ticket ID and correlation ID.

Rejected retains `CompletionRejectionKind` and detail.

Added optional outcome fields to the internal and UI modal structs.

Updated every explicit modal constructor and test literal.

## Step 2: named rejection projection

Completed in `State::log_completion_rejection`.

Added `operator_modal_targets` to isolate feedback by open mode and ticket.

Added `show_operator_modal_rejection`.

The modal receives the same kind, correlation, and detail emitted to Activity.

No second rejection mapping or string label table was added.

The existing Activity event remains unchanged.

Background rejection for another ticket cannot replace the current modal.

A closed modal is never reopened by background completion work.

## Step 3: visible pending submission

Completed in `mark_ticket_done` and `handle_key`.

The action still calls the sole typed dispatcher exactly once.

Successful dispatch projects the actual pending completion generation into the
modal.

MarkDone Enter no longer closes the modal.

Immediate rejection is left intact rather than overwritten with Pending.

Direct non-modal calls retain their prior behavior.

ResetTicket still closes after its action.

QuitConfirm remains in its existing dedicated branch.

## Step 4: keyboard lifecycle

Completed in `handle_key`.

Pre-submission MarkDone selection still supports navigation and cancellation.

Pending feedback ignores Enter, Esc, q, and navigation.

This prevents duplicate request emission.

This also guarantees the unresolved request remains visible through polls.

Accepted and Rejected accept Enter, Esc, or q as explicit acknowledgement.

Other keys do not disturb terminal feedback.

## Step 5: durable acceptance projection

Completed in `handle_completion_result`.

The operator correlation is captured before journal transition values move.

Accepted is set only after the Confirmed journal transition succeeds.

It is restricted to `CompletionSource::OperatorRequested`.

Automatic completion does not create operator modal feedback.

Unverified durable Done remains Pending.

Confirmation persistence failure remains Pending.

Failed result follows the existing named LaunchFailed projection into
Rejected.

Seat release, provenance, thread cleanup, and dependent scheduling retain their
existing order.

## Step 6: outcome rendering

Completed in `ui.rs`.

Added a dedicated operator outcome modal renderer.

Pending renders `Completion pending` and a waiting footer.

Accepted renders `Completion accepted` and explicit close controls.

Rejected renders `Completion rejected: <kind>` and explicit close controls.

All states render ticket and correlation.

Rejected also renders the actionable reason.

Long correlation and reason text wrap within the modal.

Wrapping is Unicode-safe at scalar boundaries.

The original MarkDone selection and ResetTicket renderers remain available when
no outcome is present.

## Step 7: poll regression

Extended `test_mark_done_keeps_thread_and_slot_until_commit_result`.

The test submits through the real `d` / Enter key path.

It asserts explicit operator pending authority.

It asserts the modal remains open with Pending and exact correlation.

It verifies Esc cannot dismiss the unresolved request.

It invokes `poll_tick`.

It asserts the same Pending state survives the poll.

It asserts the poll does not launch a duplicate effect.

The ticket remains Review and its thread/slot remain retained.

## Step 8: already-pending regression

Added
`test_mark_done_already_pending_keeps_named_correlated_rejection_visible`.

The fixture establishes one operator pending transaction.

It then submits the same ticket through the MarkDone modal.

The modal remains open.

It renders `AlreadyPending` with the exact correlation and reason.

No second completion effect launches.

The test explicitly acknowledges and closes terminal feedback.

## Step 9: failed and successful result regression

Extended
`failed_operator_completion_retries_without_early_release_or_duplicate_provenance`.

The first request is now submitted through the modal.

A failing command result produces visible LaunchFailed feedback.

The test asserts stderr detail, recoverability text, and original correlation.

The modal stays open until Enter acknowledges the rejection.

The retry is submitted through a fresh MarkDone modal.

After durable Done and successful correlated result, Accepted stays visible.

The test asserts the retry correlation and closes with Esc.

Existing slot release, dependent scheduling, and single-provenance assertions
remain intact.

## Step 10: renderer regression

Added
`operator_modal_outcomes_render_ticket_correlation_and_named_reason`.

It covers Pending, Accepted, and AlreadyPending rejection presentation.

It asserts exact status labels.

It asserts ticket and correlation for every state.

It asserts every word of wrapped rejection detail remains present.

It asserts the correct waiting/close footer.

It includes a Unicode hard-wrap boundary check.

## Verification

`cargo fmt --all -- --check` passed.

`cargo check -p lisa-plugin --tests` passed.

`cargo test -p lisa-plugin test_mark_done -- --nocapture` passed:

- 3 passed;
- 0 failed.

`cargo test -p lisa-plugin operator_modal_outcomes_render -- --nocapture`
passed:

- 1 passed;
- 0 failed.

`cargo test -p lisa-plugin failed_operator_completion_retries -- --nocapture`
passed:

- 1 passed;
- 0 failed.

`cargo test -p lisa-plugin completion_rejections -- --nocapture` passed:

- 2 passed;
- 0 failed.

`cargo test -p lisa-plugin --lib` passed:

- 364 passed;
- 0 failed.

`cargo test --workspace` passed across CLI, core, plugin, integration, and doc
tests.

`git diff --check` passed before commit.

## Commit

The installed `/opt/homebrew/bin/lisa` was version 0.4.0-rc.5 and did not
recognize `commit-ticket`.

The current repository binary at `target/debug/lisa` exposed the required
transaction.

The exact command used was:

```text
target/debug/lisa commit-ticket \
  --ticket-id T-042-03-02 \
  --message "fix(plugin): keep operator completion outcome visible" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ui.rs
```

The transaction produced commit:

`e178406fd7eb031a0f2590030c96d5ce836bf190`

Both ticket-owned source paths are clean after the transaction.

The ordinary Git index remained untouched.

## Deviations and concurrency handling

At implementation start, T-042-02-03 had uncommitted changes in `lib.rs`.

This was a live file-level ownership overlap despite the DAG dependencies.

Editing or committing the path at that point could have claimed another
ticket's work.

Implementation therefore completed the clean `ui.rs` half and waited.

T-042-02-03 then committed its core unit and plugin unit through Lisa.

This ticket applied its `lib.rs` patch only after that path became clean.

The implementation was compiled and tested against the resulting bounded
reconciliation behavior.

No unrelated source or artifact path was included in this ticket's commit.

There are no remaining implementation steps.
