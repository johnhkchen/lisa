# Design: durable visible operator completion outcome

## Decision summary

Keep the MarkDone modal open after submission and give it an explicit outcome
state.

The modal will show the selected ticket, stable completion correlation, and one
of pending, accepted, or rejected.

Pending remains visible across timer polls and cannot be silently dismissed by
the submission path.

Accepted and rejected outcomes remain visible until the operator explicitly
closes the modal.

Rejected state carries the existing named rejection kind and exact detail.

The existing completion adapter remains the only authority and command path.

## Outcome semantics

`Pending` means the operator request passed reducer/adapter validation and its
completion command is unresolved.

It is established only when `dispatch_completion` returns true and the
operator-owned pending transaction exists.

It does not mean the ticket is Done.

`Accepted` means the completion command result passed all existing durable
confirmation checks.

Those checks include a successful exit, commit-shaped result, observable Done
frontmatter, and persisted confirmed journal transition.

It is set at the same boundary where the scheduler currently releases the seat
and schedules dependents.

`Rejected` means the existing named rejection projector emitted a typed
completion rejection for the modal's ticket.

It includes already-pending, stale-lease, disposition-blocked,
dependency-blocked, and launch-failed.

Its correlation is the correlation supplied by the same projector.

Its detail is the same actionable detail logged to Activity.

The modal outcome is a projection of adapter facts, not an independent state
machine controlling completion.

## Option 1: leave modal behavior unchanged and rely on Activity

The predecessor already renders named correlations in Operations and Activity.

This option would require no additional state.

It does not satisfy the ticket because Enter still closes before the operator
can distinguish pending from rejection.

It also requires the operator to infer that an Info event means unresolved.

The acceptance criterion explicitly forbids silent close on unresolved input.

This option is rejected.

## Option 2: keep the ticket list modal open without outcome data

Enter could stop setting `modal.open = false`.

This would make an unresolved request survive polling.

The operator would still see the original selectable list and confirm footer.

Nothing would indicate whether the request launched, was already pending, or
was rejected.

Repeated Enter could generate duplicate requests while the first is in flight.

Named reason and correlation would remain absent from the modal.

This option only addresses lifetime and is rejected.

## Option 3: close the modal and add a transient dashboard toast

A dashboard toast could show request outcome after submission.

This separates modal selection from feedback.

The plugin has no existing toast lifetime or acknowledgement model.

A timer-based toast would introduce a new transient timeout whose durability is
weaker than keeping the modal state.

It would also compete with existing dashboard activity rendering.

The ticket allows durable visible confirmation, but a transient toast is not
durable enough across arbitrary polls.

This option is rejected.

## Option 4: model outcome in the existing modal

The internal modal gains optional operator outcome state.

The UI modal gains the corresponding presentation type.

The ticket selection layout is used before submission.

After submission, the layout switches to a status view.

The pending status disables repeated submission.

The result status requires explicit dismissal.

Timer-driven rerenders naturally preserve the same modal struct.

No new timer, global notification queue, or completion path is necessary.

This option directly covers every acceptance clause and is selected.

## Internal outcome representation

Add an internal enum dedicated to operator modal feedback.

Each variant carries the ticket ID and full correlation string.

Rejected additionally carries `CompletionRejectionKind` and detail.

Accepted additionally carries the confirmed commit ID only if useful to the
operator; the acceptance criterion does not require it.

The chosen minimal shape omits commit ID and presents accepted plus
correlation.

The correlation string is stored instead of a core ID wrapper because this is
display state and the UI boundary already uses strings.

The outcome lives on `MarkDoneModal`, not on `State` separately.

ResetTicket and QuitConfirm construct the field as `None`.

Opening a fresh MarkDone modal also starts with `None`.

## Submission transition

When Enter is pressed in MarkDone selection mode, the selected ticket is
cloned.

The modal must remain open.

`mark_ticket_done` computes the same generation-1 operator correlation used by
dispatch.

It calls the existing typed dispatcher exactly once.

If dispatch accepts the request for execution, it records `Pending` using the
actual pending completion key.

If dispatch rejects, `log_completion_rejection` records `Rejected` as part of
its existing projection.

The rejection hook avoids reverse-searching Activity and guarantees that UI
feedback uses the same kind, correlation, and detail as the dashboard.

The submission path must not overwrite a rejection with Pending.

The boolean dispatcher result is sufficient to distinguish successful launch
from immediate rejection.

## Result transition

`handle_completion_result` already owns final acceptance and result rejection.

Failed command results call `log_completion_rejection` and therefore update the
modal to Rejected automatically.

Successful durable confirmation explicitly updates a matching operator modal
to Accepted.

That update occurs only after the confirmed journal transition persists.

It occurs before or alongside removal of pending state; ordering is not an
authority concern because the modal is observational.

If durable Done cannot be verified, pending remains pending.

If confirmation cannot be journaled, pending remains pending.

Those unresolved cases therefore keep the modal in Pending rather than falsely
announcing acceptance or rejection.

## Matching and isolation

Modal updates apply only when mode is MarkDone.

They apply only when the modal outcome ticket matches the event ticket.

Immediate rejection may arrive before an outcome exists, so selection of the
same ticket is also a valid match during submission.

Background automatic completion for another ticket cannot overwrite the
operator's modal.

An operator result arriving after explicit modal dismissal does not reopen the
modal.

Its outcome remains available in Activity through the predecessor behavior.

This maintains user control without manufacturing unsolicited overlays.

## Key behavior

Before submission, Up/Down and j/k continue to move selection.

Before submission, Esc and q continue to cancel.

Before submission, Enter submits MarkDone or executes ResetTicket.

While MarkDone is Pending, navigation and repeated Enter do nothing.

Esc and q also do not dismiss Pending, ensuring the modal persists until a
terminal request outcome.

After Accepted or Rejected, Enter, Esc, or q explicitly closes the modal.

The footer communicates waiting in Pending state.

The terminal footer communicates explicit acknowledgement controls.

ResetTicket behavior stays unchanged, including close after Enter.

QuitConfirm behavior stays unchanged in its separate key branch.

## Rendering

The selection renderer remains unchanged when no outcome exists.

Pending renders a clear `completion pending` label.

Accepted renders a clear `completion accepted` label.

Rejected renders `completion rejected: <named-kind>`.

Every status renderer shows the ticket ID.

Every status renderer shows the full correlation ID.

Rejected renders the detail text beneath the named kind.

Long detail and correlation values must wrap within the box rather than being
silently truncated.

ANSI decoration is kept outside the stored text.

The layout can use a small line-wrapping helper for plain strings.

The box remains bounded by available width and rendered lines are clipped by
the existing dashboard output path.

## Testing design

Extend the real key-handler test instead of only testing a helper.

After Enter, assert the modal is open and outcome is Pending.

Call `poll_tick` and assert the same pending ticket/correlation remains.

Drive a failed result and assert Rejected with `launch-failed`, detail, and the
same correlation.

Add an immediate duplicate-request test for `already-pending`.

It should open a modal over a ticket with an existing pending transaction,
submit, and assert named kind plus correlation without modal close.

Add or extend UI tests to render Pending, Accepted, and Rejected content.

The rejection rendering assertion must include exact name and correlation.

Drive successful durable confirmation in an existing result-oriented fixture
and assert Accepted stays visible until explicit dismissal.

Focused tests are followed by the full plugin suite and workspace suite.

## Compatibility and risk

The internal and UI modal structs have many test literals.

Adding a non-optional field requires updating those literals.

Using `Option` with default minimizes construction risk while keeping state
explicit.

No serialized format changes because modal state is in-memory only.

No WASM host API changes are needed.

No core API changes are needed.

The main behavioral risk is accidentally changing ResetTicket keyboard flow.

Mode-specific branches and existing ResetTicket tests constrain that risk.

The other risk is declaring Accepted before durable confirmation.

Placing the update after journal confirmation prevents that error.
