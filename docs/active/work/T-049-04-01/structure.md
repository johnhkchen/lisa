# Structure: bounded park on completion failure

## Modified file: `crates/lisa-plugin/src/completion_journal.rs`

This module remains the sole durable completion-journal schema and fold.

Advance its current schema version while continuing to accept versions 1 and
2.

Add a journal-visible `CompletionFailureClass` enum.

Its serialized values are stable kebab-case diagnostic labels.

Add a journal-visible `FailureConsequence` enum.

Its values distinguish retry scheduled, retry exhausted, and park.

Extend `CompletionJournalTransition` with `FailureObserved`.

The variant carries generation key, correlation, technical reason, class,
failure count, failure limit, and consequence.

Extend `JournalRecordBody` with the corresponding `failure-observed` shape.

Existing row shapes remain byte-compatible.

Extend `JournalRecord::from_transition` and `into_transition` for the new row.

Extend `CompletionJournalTransition::key` for the new variant.

Extend `CompletionJournalAggregate` with private failure-count, failure-limit,
and retry-exhausted fields.

Expose narrow accessors needed by the adapter:

- `failure_count()`;
- `failure_limit()`;
- `retries_exhausted()`.

Requested transitions initialize count to zero, limit to none, and exhaustion
to false.

FailureObserved transitions validate and update those fields without changing
the core CompletionState.

Rejected and Confirmed transitions retain the audit counters.

The fold validation requires:

- a matching generation and seal;
- current CommandInFlight correlation equality;
- positive limit;
- first count equal to one;
- every later count exactly previous plus one;
- a stable limit;
- count no greater than limit;
- retry-scheduled only below the limit;
- retry-exhausted only at the limit;
- park at any valid count.

Tests in this module add round-trip and invalid-sequence coverage.

## Modified file: `crates/lisa-plugin/src/lib.rs`

### Constants

Add `MAX_COMPLETION_FAILURES: u8 = 2` adjacent to the existing reconciliation
timeout.

Add the required history/identity ask as a shared constant so exact tests and
runtime use one string.

### Failure policy types

Add a private `CompletionFailureDisposition` projection.

It carries the journal class and a remedy representation.

The remedy representation distinguishes a structured operator ask from raw
unstructured reason fallback.

Add a private `CompletionFailureAction` enum:

- Retry;
- WaitForDeadline;
- Park.

Add a pure `classify_completion_failure(detail)` function.

Add a pure `completion_failure_action(class, next_count)` function.

Classification has no filesystem or scheduler side effects.

Action selection enforces the fixed bound centrally.

### Pending completion

No new retry count is stored in `PendingCompletion`.

The journal aggregate is the durable and in-memory source of count truth.

Retain source and authority when relaunching the same generation.

Remove the special behavior that silently drops failed reconciliation replay
results.

Every failed command observation enters the same classification path.

### Replay boundary

Generalize `replay_in_flight_completion` to accept the retained
`CompletionAuthority` and `CompletionSource` rather than only an attempt lease.

Attempt authority must still hold the current lease.

Operator authority must still originate from an operator request.

The method must refuse replay when the journal aggregate says the retry bound
is exhausted.

It continues to reuse the exact generation, correlation, and absolute
deadline.

### Result boundary

Split failed-result formatting from failed-result consequence.

`handle_completion_result` constructs one full technical reason exactly once.

It classifies the stderr/raw reason.

It derives the next count from the journal aggregate.

For Retry:

1. append FailureObserved with retry-scheduled;
2. remove the completed pending host invocation;
3. relaunch the exact generation;
4. log a plain lead sentence plus technical detail.

For WaitForDeadline:

1. append FailureObserved with retry-exhausted;
2. remove pending invocation;
3. retain journal CommandInFlight state;
4. log that Lisa is waiting until the existing deadline;
5. do not launch again.

For Park:

1. append FailureObserved with park consequence;
2. invoke the common completion park helper.

The success branch remains unchanged except for aggregate field compatibility.

### Reconciliation boundary

When core reconciliation returns ReplayCommandInFlight, inspect journal retry
exhaustion first.

If exhausted and before deadline, return without a host launch.

At deadline, core reconciliation already selects expiry and parking.

### Completion park helper

Add `park_failed_completion` near expiry/result handling.

Inputs include ticket ID, generation key, optional correlation, technical
reason, remedy shape, and optional retry progress.

The helper appends the ActionRequired Rejected journal transition.

It publishes canonical `review-disposition.json` atomically with
`RustPublication`.

Structured documents contain disposition, reason, operator owner, and ask.

Unstructured documents contain only disposition and reason.

The helper obtains prior phase/status from the matching journal aggregate.

It updates the ticket phase back to the prior non-Done phase.

It sets ticket status Blocked.

It appends `ParkingTransitionType::Park` through the existing provenance writer.

It releases the slot and removes current thread/pending state.

It logs the ask before technical bracket detail and rebuilds the DAG.

The helper returns false on any pre-park durability error and true on a durable
park.

### Parking provenance helper

Rename `emit_review_block_transition` to the more general
`emit_parking_transition`, or retain its name and reuse it if the call remains
clear.

No provenance schema changes are required.

Completion parks pass current attempt evidence and retry count/limit.

Unrecognized and deadline parks use operator ownership and are not
recheck-eligible.

Existing Review block call sites remain behaviorally unchanged.

### Deadline expiry

Replace the standalone terminal ActionRequired implementation.

Build its complete technical reason as today.

Use the structured deadline recovery ask.

Call the same completion park helper.

This yields Review/blocked state and ordinary unpark eligibility.

### Tests in `lib.rs`

Add a table fixture for all narrow classifier patterns.

Assert unborn and identity map to the exact shared ask.

Assert permission and stale-lock patterns map to their structured asks.

Assert index contention maps transient.

Assert unmatched text maps unrecognized.

Add an operator-owned completion fixture that observes two failures.

Assert the first failure relaunches and leaves status Review/open.

Assert the second failure parks, releases the seat, writes structured ask, and
records retry limit in journal and provenance.

Add a transient fixture with two failures.

Assert one retry, no third launch, no immediate park, and retained in-flight
state awaiting deadline.

Add an unrecognized fixture.

Assert immediate park and parser-visible `unstructured: true` raw ask.

Replace the old deadline dead-end regression with recovery coverage.

Assert expiry writes blocked Review state and a structured ask.

Run the ordinary ticket status reopen operation, rebuild, and reconcile unpark.

Assert Unpark provenance and schedulable ordinary DAG state without journal
editing.

## No new files

No new core module, CLI command, UI component, or persisted state directory is
created.

The existing completion journal, Review disposition, ticket status, and
provenance ledger remain the four durable surfaces.

## Commit units

The journal schema/fold and plugin policy are coupled by a private module
interface and form one meaningful source unit.

Commit both exact paths in one isolated Lisa transaction after focused and
workspace verification.

Do not include phase artifacts, ticket frontmatter, live ledgers, or unrelated
T-049-02-01 work.
