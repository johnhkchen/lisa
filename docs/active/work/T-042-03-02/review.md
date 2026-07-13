# Review: operator modal durable confirmation

## Disposition

The implementation is ready to complete.

The acceptance criterion is satisfied by state, renderer, poll, rejection, and
durable-result tests.

No critical issue remains open.

## Change summary

T-042-03-02 changes MarkDone from a fire-and-close interaction into a visible
request lifecycle.

Submitting the selected ticket now keeps the modal open.

An accepted-for-execution request displays Pending.

Pending displays the ticket and stable completion-generation correlation.

Pending survives timer-driven polling.

Pending cannot be dismissed with Esc/q.

Pending cannot be resubmitted with Enter.

A named rejection displays Rejected.

Rejected displays the exact stable rejection kind.

Rejected displays the exact correlation supplied by the completion adapter.

Rejected displays the adapter's actionable detail.

A durably confirmed completion displays Accepted.

Accepted displays the same stable request correlation.

Accepted and Rejected stay open until the operator explicitly acknowledges
them.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

Added the internal `OperatorModalOutcome` enum.

Added optional outcome state to `MarkDoneModal`.

Added modal ticket matching that isolates background completion activity.

Projected the existing named rejection mapping into matching modal state.

Projected a successful dispatch into Pending using the actual pending
completion generation.

Projected durable command confirmation into Accepted.

Changed MarkDone keyboard behavior to preserve unresolved and terminal
feedback.

Kept ResetTicket and QuitConfirm behavior mode-specific and unchanged.

Mapped internal outcome state into UI presentation state.

Extended the key-path/poll test.

Added the already-pending modal regression.

Extended the operator failure/retry/success regression.

### `crates/lisa-plugin/src/ui.rs`

Added the UI `OperatorModalOutcome` enum.

Added optional outcome state to `ModalState`.

Added Unicode-safe wrapping for long modal values.

Added a dedicated operator outcome renderer.

Added distinct pending, accepted, and rejected labels/colors.

Added waiting and explicit-close footer states.

Added renderer coverage for all three outcomes.

No file was created or deleted by the source change.

No `lisa-core` source was modified.

No journal schema was modified.

No CLI transaction source was modified.

No ticket frontmatter was manually modified.

## Authority and state-machine review

The modal remains presentation state only.

It does not authorize completion.

It does not write ticket state.

It does not launch a command directly.

`mark_ticket_done` still constructs exactly one
`CompletionInput::OperatorRequested`.

`dispatch_completion` remains the sole typed request gateway.

`execute_completion_effect` remains the sole new-effect host-command boundary.

The modal derives Pending from the adapter's inserted pending transaction.

It does not synthesize a separate correlation.

Rejected derives from `log_completion_rejection`.

That method remains the single mapping from core rejection variant to stable
UI kind and detail.

Activity and modal therefore cannot drift in rejection naming.

Accepted is emitted only after durable Done verification.

Accepted is emitted only after the Confirmed journal transition succeeds.

If either condition fails, the request stays Pending.

Seat release still occurs after confirmation.

Dependent scheduling still occurs after confirmation.

Authoritative provenance still occurs after confirmation.

## Named rejection review

The centralized projector covers the predecessor's five named kinds:

- already-pending;
- stale-lease;
- disposition-blocked;
- dependency-blocked;
- launch-failed.

The modal helper receives the exact kind value.

It receives the exact correlation string.

It receives the exact mapped detail.

It updates only an open MarkDone modal targeting the same ticket.

Immediate rejection matches the current selection.

Later rejection matches the existing Pending outcome ticket.

Rejection for an unrelated ticket continues to Activity without changing the
operator modal.

Rejection after explicit modal dismissal does not reopen the UI.

This preserves both durable visibility while engaged and normal background
dashboard behavior after dismissal.

## Poll and lifecycle review

The test submits through `handle_key`, not through a presentation-only helper.

The modal is confirmed open before submission.

After Enter, Pending contains the pending transaction's exact key.

An attempted Esc returns false and leaves it open.

`poll_tick` executes the real periodic scheduler boundary.

After the poll, the modal remains open.

After the poll, the Pending value is unchanged.

The launch-effect count remains one.

The ticket remains in Review.

The thread and slot remain assigned.

This proves the request is neither silently closed nor duplicated across a
poll.

## Already-pending review

The new duplicate test first establishes an operator pending transaction.

It then opens MarkDone and submits the same ticket through Enter.

The pure completion state rejects the duplicate as AlreadyPending.

The modal remains open.

The displayed correlation equals the existing operator completion generation.

The detail contains the named already-pending reason.

The effect count remains one.

Enter then explicitly acknowledges and closes the terminal outcome.

This directly exercises the acceptance criterion's already-pending clause.

## Failed-result review

The existing operator retry regression now uses the modal input path.

The first command result exits unsuccessfully with diagnostic stderr.

The adapter persists its retryable rejection before removing pending state.

The Activity entry remains LaunchFailed and correlated.

The modal also becomes LaunchFailed and correlated.

The modal detail retains both stderr and recoverability text.

The thread remains present.

The slot remains assigned.

No provenance is emitted.

The dependent remains blocked.

The operator acknowledges the rejection before retrying.

## Accepted-result review

The retry is submitted through a fresh MarkDone modal.

The fixture applies the same durable Done bytes the completion command would
write.

The result contains a valid commit-shaped ID.

The handler confirms journal state.

Only then does the modal become Accepted.

The Accepted correlation equals the retry's pending completion generation.

The thread is removed.

The slot is released.

The dependent becomes ready.

Exactly one authoritative provenance record exists.

The Accepted modal remains open through those scheduler mutations.

Esc explicitly dismisses it.

## Rendering review

The outcome renderer is selected only for MarkDone with Some outcome.

QuitConfirm still routes first to its dedicated renderer.

ResetTicket still uses the selection renderer.

Pending, Accepted, and Rejected have distinct human-readable labels.

Every outcome shows ticket and correlation.

Rejected shows named kind and reason.

Long values wrap instead of truncating.

Wrapping splits long tokens without losing bytes.

Wrapping respects Unicode scalar boundaries.

The renderer test covers a one-column Unicode split.

The modal output remains bounded by the existing terminal row clipping.

## Test coverage

Focused MarkDone tests:

- 3 passed;
- pending authority;
- poll persistence;
- already-pending correlation and detail;
- orphaned Review authority.

Focused renderer test:

- 1 passed;
- all outcome variants;
- ticket/correlation visibility;
- named rejection reason;
- Unicode wrapping.

Focused operator result test:

- 1 passed;
- command failure;
- retained slot and thread;
- visible correlated rejection;
- retry;
- durable acceptance;
- explicit dismissal;
- single provenance.

Focused rejection tests:

- 2 passed;
- all five stable rejection labels/correlations remain covered in Activity;
- projection remains exhaustive.

Complete plugin suite:

- 364 passed;
- 0 failed.

Complete workspace suite:

- passed;
- CLI transaction tests passed;
- core completion tests passed;
- plugin tests passed;
- integration and doc tests passed.

Formatting, test compilation, and diff whitespace checks passed.

## Commit and cleanliness

Source commit:

`e178406fd7eb031a0f2590030c96d5ce836bf190`

Message:

`fix(plugin): keep operator completion outcome visible`

The commit contains exactly:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ui.rs`.

It was created by the current in-repository Lisa CLI's isolated
`commit-ticket` transaction.

Both source paths are clean afterward.

The ordinary index is empty and was never used.

Unrelated provenance, ticket-phase, generated docs, and other-ticket artifact
changes remain outside this commit.

## Open concerns and limitations

Modal outcome durability is in-memory across polls, not a new persisted UI
record across plugin restart.

This matches the story's stated boundary: completion transaction persistence
belongs to Story B, while this ticket owns modal semantics.

After explicit dismissal, later background results remain visible through the
existing Activity projection rather than reopening the modal.

Very short terminal heights can clip modal lines through the existing output
row limit.

Typical modal widths wrap the full correlation and reason.

There is no modal scrolling in this ticket.

No acceptance requirement calls for a restart-restored modal or modal
scrolling.

The broader active/orphaned/blocked/stale/already-pending/launch-failure/success
matrix remains assigned to T-042-03-03.

This ticket provides the settled modal state and rendering primitives that
matrix will consume.

No TODO was added to source.

No known correctness defect remains.

No human-blocking decision is required.
