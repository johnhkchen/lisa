# Structure: operator modal outcome projection

## Change boundary

Modify `crates/lisa-plugin/src/lib.rs`.

Modify `crates/lisa-plugin/src/ui.rs`.

Do not modify `lisa-core` completion reducer code.

Do not modify completion journal schema or persistence code.

Do not modify CLI completion transaction code.

Do not modify provider adapters.

Do not add files outside the attempt artifacts and the two plugin sources.

## `crates/lisa-plugin/src/lib.rs`

### Internal modal outcome type

Define `OperatorModalOutcome` near `ModalMode` and `MarkDoneModal`.

Derive Debug, Clone, PartialEq, and Eq for testability.

Variant `Pending` contains:

- `ticket_id: TicketId`;
- `correlation_id: String`.

Variant `Accepted` contains:

- `ticket_id: TicketId`;
- `correlation_id: String`.

Variant `Rejected` contains:

- `ticket_id: TicketId`;
- `kind: CompletionRejectionKind`;
- `correlation_id: String`;
- `detail: String`.

The enum is private because only plugin state and tests consume it.

Add a ticket accessor if matching logic benefits from one canonical match.

### `MarkDoneModal`

Add `operator_outcome: Option<OperatorModalOutcome>`.

Default remains `None` through the derived default.

The field is meaningful only for `ModalMode::MarkDone`.

All explicit constructors set it to `None` unless restoring current feedback.

The existing ticket list remains available while outcome is present, but the
renderer does not display the list.

### Rejection projection

Extend `State::log_completion_rejection` after mapping the core rejection to
kind and detail.

Before moving those values into `ActivityEvent`, call a modal helper with
cloned display values.

Add `State::show_operator_modal_rejection` near completion projection helpers.

The helper checks:

- the modal is open;
- the mode is MarkDone;
- the currently selected ticket or existing outcome ticket equals the rejected
  ticket.

If matched, set `operator_outcome` to Rejected.

Do not open a closed modal.

Do not modify ResetTicket or QuitConfirm state.

Continue emitting the existing Activity event unchanged.

### Submission projection

Change `State::mark_ticket_done` to continue dispatching
`CompletionInput::OperatorRequested`.

Capture the dispatch boolean.

On true, retrieve the pending entry for the ticket.

Use its `completion_key.to_string()` as the displayed correlation.

Set `operator_outcome` to Pending when the current open MarkDone modal targets
the same ticket.

On false, do not close and do not overwrite the rejection set by
`log_completion_rejection`.

If a future false path emits no named rejection, leave the modal open rather
than silently closing it.

The dispatcher signature remains boolean to avoid changing automatic callers.

### Acceptance projection

Add `State::show_operator_modal_accepted` near the rejection helper.

It accepts ticket ID and correlation.

It only replaces an open MarkDone outcome for the same ticket.

Call it in `handle_completion_result` after the Confirmed journal transition
succeeds.

Gate the call on the pending source being
`CompletionSource::OperatorRequested(_)`.

Use `pending.completion_key` as the visible stable correlation identity.

Do not set Accepted for automatic completion sources.

Do not set Accepted on invalid output, unverified Done, or journal failure.

### Key handler organization

In the non-Quit modal branch, detect a present MarkDone outcome before normal
list controls.

For Pending:

- ignore all keys and return false, preserving the modal;
- timer events still rerender through `State::update`.

For Accepted or Rejected:

- Enter, Esc, or q closes the modal and returns true;
- other keys do nothing.

For no outcome, preserve existing selection controls.

On MarkDone Enter:

- invoke `mark_ticket_done`;
- do not set `modal.open = false`.

On ResetTicket Enter:

- invoke `reset_ticket`;
- retain the existing close behavior.

QuitConfirm remains in its preceding dedicated branch.

### Constructors

`open_mark_done_modal` sets `operator_outcome: None`.

`open_reset_modal` sets `operator_outcome: None`.

`try_quit` sets `operator_outcome: None`.

Any test-only explicit `MarkDoneModal` literals must include the new field.

### UI conversion

Extend `State::to_ui_state` modal conversion.

Map internal Pending to `ui::OperatorModalOutcome::Pending`.

Map internal Accepted to `ui::OperatorModalOutcome::Accepted`.

Map internal Rejected to the UI Rejected variant without relabeling.

Clone strings at the presentation boundary.

Copy `CompletionRejectionKind` directly because it is Copy.

## `crates/lisa-plugin/src/ui.rs`

### UI outcome type

Define public `OperatorModalOutcome` beside `ModalKind` and `ModalState`.

Use the same three variants and field shapes as the internal type.

Derive Debug, Clone, PartialEq, and Eq.

This type is presentation data only.

Add `operator_outcome: Option<OperatorModalOutcome>` to `ModalState`.

Derived Default yields None.

### Rendering helpers

Add a small plain-text wrapping helper local to modal rendering.

Its input is text plus maximum visible width.

It splits at whitespace when possible.

It hard-splits an individual token longer than the width.

It returns at least one line for empty input only when the caller needs one.

Add a centered/padded modal-row helper if that reduces repeated width math.

The helpers must calculate visible widths without ANSI sequences in input.

### Outcome renderer

Add `render_operator_outcome_modal`.

Inputs are outcome, width, and height.

Choose status text and color by variant:

- Pending: `Completion pending`, yellow;
- Accepted: `Completion accepted`, bright green;
- Rejected: `Completion rejected: <kind>`, red.

Render ticket on its own labeled line.

Render correlation with wrapping and its full value.

Render rejected detail with wrapping and its full value.

Pending footer states that Lisa is waiting for the completion result.

Terminal footer states `Enter/Esc=close`.

Use the same box width cap as the selection modal.

Clip vertical output only through the existing `print_dashboard` row limit.

### `render_modal` dispatch

Keep QuitConfirm as the first special case.

If kind is MarkDone and `operator_outcome` is Some, call the outcome renderer.

Otherwise retain the current shared selection renderer.

ResetTicket can never route to the outcome renderer.

### UI fixture updates

Explicit `ModalState` literals in UI tests add `operator_outcome: None`.

Existing `PluginState::default` fixtures remain compatible.

Add a rendering test that loops through all three outcome variants.

Assert ticket and correlation for each.

Assert Pending and Accepted labels.

Assert Rejected exact `already-pending` label and actionable detail.

## Test locations in `lib.rs`

Extend `test_mark_done_keeps_thread_and_slot_until_commit_result`.

Assert open Pending immediately after Enter.

Call one `poll_tick` and assert open Pending is unchanged.

Avoid depending on wall-clock-specific scheduler state beyond the modal.

Add `test_mark_done_already_pending_keeps_named_correlated_rejection_visible`.

Create the first pending request directly.

Open the modal and submit the duplicate through `handle_key`.

Assert open Rejected state with AlreadyPending and full correlation.

Add acceptance assertion to an existing durable operator completion test or a
focused fixture.

Open the MarkDone modal before request.

Produce durable Done and successful result.

Assert Accepted remains open with the request correlation.

Press Enter or Esc and assert explicit dismissal closes it.

Add failed result assertion to verify LaunchFailed detail and correlation in
the modal, not only Activity.

## Ordering

First add types and conversion so the compiler identifies all constructors.

Then add rendering and update UI literals.

Then add submission/result projection and keyboard behavior.

Then add state tests and renderer tests.

Run formatting before focused tests.

Commit both plugin source files together because the internal-to-UI projection
is one meaningful unit and neither file compiles independently.

No source file deletion or public crate API change is expected.
