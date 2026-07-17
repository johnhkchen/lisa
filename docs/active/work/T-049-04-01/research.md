# Research: bounded park on completion failure

## Ticket boundary

T-049-04-01 addresses the scheduler response after an isolated completion
command fails or becomes unobservable.

The successful completion transaction is not in scope.

Lock cleanup and empty-history behavior inside `commit_transaction.rs` belong
to the dependent T-049-04-02.

The active failure evidence is the preserved 2026-07-16 journal under
`docs/active/work/T-046-06-03/cbt-0716-211915-variant-xdg/`.

That journal contains repeated requested, command-in-flight, and retryable
rejected triples for one completion generation.

Every rejection carries the same `git log --grep` failure from an unborn
branch.

The failure occurs before Git consults commit identity.

The current scheduler has no count attached to those repeated failures.

## Completion domain

`crates/lisa-core/src/completion.rs` owns provider-neutral completion state.

`CompletionState` has Eligible, Requested, CommandInFlight, Rejected, and
Confirmed variants.

CommandInFlight retains a correlation and one absolute reconciliation
deadline.

Rejected retains a typed `CompletionRejection` and `Retryability`.

Retryable rejected state accepts another Request.

ActionRequired rejected state refuses another Request with its retained
reason.

`reconcile` returns ReplayCommandInFlight before the deadline.

At or after the deadline it returns CommandInFlightDeadlineExceeded.

The core reducer does not know Git text, ticket files, Review artifacts,
parking, or provenance.

## Completion adapter

`crates/lisa-plugin/src/lib.rs` is the scheduler adapter for the core domain.

`PendingCompletion` retains the generation key, correlation, deadline, source,
authority, and pre-completion phase/status.

`State::execute_completion_effect` is the sole new-command launch boundary.

It journals Requested and CommandInFlight before launching the host command.

`State::handle_completion_result` owns correlated exit processing.

A successful exit with a commit-shaped stdout value is confirmed only after
Done frontmatter can be rescanned.

A nonzero exit, missing exit, or malformed stdout currently becomes a single
technical failure string.

For an initial command it journals Rejected with Retryable.

It then removes the pending entry and rebuilds the DAG.

The next poll re-derives completion from the passing Review artifacts and
launches the same generation again.

There is no counter, classification, backoff decision, or park consequence.

A failed reconciliation replay is not journaled; its pending entry is removed
and the absolute deadline remains the only bound.

## Durable completion journal

`crates/lisa-plugin/src/completion_journal.rs` owns the append-only journal.

Schema version 2 records the pinned completion seal on every row.

Journal rows are requested, command-in-flight, rejected, or confirmed.

The file is atomically republished after validating and folding all rows.

`CompletionJournalAggregate` retains only the latest reducer state, generation,
seal, prior phase/status, and optional confirmed commit.

It does not retain failure or retry counts.

The fold accepts a new generation after a rejected or confirmed aggregate.

An action-required rejection masks durable Done bytes.

That mask explains why the current deadline terminal state can hide Done.

## Deadline path

`State::expire_in_flight_completion` handles the core deadline result.

It constructs a technical reason containing the Unix-millisecond deadline and
correlation.

It journals an ActionRequired rejection.

It removes pending state and rebuilds the DAG.

It does not write a blocking disposition or blocked ticket status.

It does not release the current attempt through parking.

Because ActionRequired refuses requests, automated reconciliation and operator
MarkDone cannot leave this state.

The current native regression explicitly asserts this terminal behavior.

## Existing E-048 parking contract

Parking already uses ordinary ticket status as durable scheduling authority.

`apply_review_block_policy` admits a complete private Review block disposition.

Operator and world remedies park immediately.

Agent remedies receive two bounded retries before parking.

Parking writes `status: blocked`, appends provenance, releases the slot, removes
the thread, and rebuilds the DAG.

The canonical `review-disposition.json` is the durable human remedy payload.

`lisa_core::parking::collect_parked_remedies` projects blocked tickets and their
canonical dispositions for status and dashboard surfaces.

`lisa unblock` validates optional checks and changes `status: blocked` to
`status: open`.

The scheduler observes that status change, appends Unpark provenance, and
returns the ticket to ordinary DAG eligibility.

## Review disposition schema

`crates/lisa-core/src/disposition.rs` parses pass and block documents.

A structured block carries reason, remedy owner, ask, optional steps, and an
optional check.

A block containing only disposition and reason is accepted as a legacy safe
fallback.

That fallback copies the raw reason to the ask, assigns operator ownership, and
sets `unstructured: true` in the parsed value.

This existing fallback exactly represents the ticket's conservative behavior
for unrecognized Git failures.

No new persisted field is required to flag an unknown failure as unstructured.

## Parking provenance

`ParkingTransitionRecord` lives in `crates/lisa-core/src/provenance.rs`.

It carries seal, record type, ticket, attempt lease, remedy owner, optional
retry count/limit, recheck eligibility, and interval timestamps.

Record types are Retry, Park, and Unpark.

`State::emit_review_block_transition` validates current attempt evidence and
appends the row.

`State::reconcile_unpark_transitions` finds a latest Park whose ticket is open
and appends exactly one matching Unpark row.

The existing schema is sufficient for completion-failure parking.

## Failure text boundary

The plugin receives only exit code, stdout, and stderr from the host command.

It does not receive a typed Git error from the CLI.

Classification therefore occurs at this adapter boundary and necessarily uses
conservative text recognition.

Known unborn-history text includes `does not have any commits yet`.

Known identity text includes `Author identity unknown`, `Please tell me who
you are`, and auto-detection failures.

Known permission text includes permission denied, read-only filesystem, and
unable-to-write variants.

Known lock text includes the completion lock and Git lock-file diagnostics.

Text not matching a narrow known class cannot safely receive a guessed remedy.

## Operator surfaces

Completion rejections enter `ActivityEvent::CompletionRejected`.

The dashboard and operator modal render its detail.

Recent T-049 work already formats these surfaces as a plain lead sentence with
technical detail following in brackets.

The full journal reason is a separate audit surface and can retain the complete
authority/source/exit/stderr envelope.

## Tests and constraints

Most completion scheduler fixtures are native tests in `lisa-plugin`.

They construct State directly, install leases, write Review artifacts, and
call `handle_completion_result` or deadline dispatch deterministically.

Journal tests inspect exact JSONL rows and restart reconstruction.

Parking tests inspect canonical dispositions, ticket status, seat release, and
mixed provenance ledgers.

The worktree already contains Lisa-owned ticket and ledger edits plus unrelated
T-049-02-01 changes.

Ticket source ownership must use exact paths and Lisa's isolated commit command.

Likely owned source paths are `crates/lisa-plugin/src/lib.rs` and
`crates/lisa-plugin/src/completion_journal.rs`.

Phase artifacts remain in this attempt-private directory for Lisa to publish.
