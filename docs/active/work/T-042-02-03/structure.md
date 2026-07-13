# Structure: bounded reconciliation replay convergence

## Change set overview

No production file is created or deleted.

The implementation modifies the completion domain, its external tests, the
plugin journal, and the plugin scheduler adapter.

The CLI transaction remains source-compatible and unchanged.

## `crates/lisa-core/src/completion.rs`

### New durable time type

Add public `CompletionDeadline` near the completion identity types.

Shape:

```rust
pub struct CompletionDeadline(u64);
```

Public methods:

```rust
pub fn from_unix_millis(value: u64) -> Self;
pub fn unix_millis(&self) -> u64;
pub fn is_expired_at(&self, now: CompletionDeadline) -> bool;
```

The type derives the ordering and equality traits required by state and tests.

It is adapter-neutral and does not depend on `SystemTime` or serde.

### State shape

Change:

```rust
CommandInFlight { correlation: CorrelationId }
```

to:

```rust
CommandInFlight {
    correlation: CorrelationId,
    deadline: CompletionDeadline,
}
```

The stable state name remains `command-in-flight`.

### Event shape

Change `CompletionEvent::CommandLaunched` to carry both correlation and
deadline.

Every reducer match reconstructs or forwards both fields.

The Requested transition installs both values into CommandInFlight.

Correlation matching behavior is unchanged.

### Reconciliation shape

Replace `CommandInFlightActionRequired` with two explicit decisions:

```rust
ReplayCommandInFlight {
    correlation: CorrelationId,
    deadline: CompletionDeadline,
}
CommandInFlightDeadlineExceeded {
    correlation: CorrelationId,
    deadline: CompletionDeadline,
}
```

Change the public `reconcile` signature to accept `now: CompletionDeadline`.

For CommandInFlight, compare the stored deadline inclusively with now.

All other state decisions keep current behavior and ignore the time value.

### In-module tests

Update every event/state literal with a deterministic deadline.

Replace the old immediate-action-required test with before/equal/after boundary
tests.

Add direct tests for the deadline newtype accessors and comparison.

## `crates/lisa-core/tests/completion_state_machine.rs`

Add a constant generated deadline and explicit generated current time.

Update the harness's CommandLaunched event and reconstructed state.

Handle `ReplayCommandInFlight` as the current live-effect assertion.

Handle deadline-exceeded as unreachable in the existing generated scenario
because its generated current time stays before the deadline.

Keep the existing model's one-live-effect behavior.

The property remains focused on event ordering rather than wall-clock
generation.

## `crates/lisa-core/tests/recorded_livelock_regression.rs`

Add a fixed deadline later than the fixed reconciliation time.

Pass explicit time to every core reconciliation call.

Update the launched event and CommandInFlight equality assertion.

Treat `ReplayCommandInFlight` as the former in-flight assertion.

Treat deadline exceeded as unreachable for this historical trace.

This keeps the previous livelock regression semantically unchanged.

## `crates/lisa-plugin/src/completion_journal.rs`

### Transition shape

Extend `CompletionJournalTransition::CommandInFlight` with
`deadline: CompletionDeadline`.

No other transition variants change.

### JSON shape

Extend `JournalRecordBody::CommandInFlight` with:

```rust
#[serde(default)]
reconciliation_deadline_unix_ms: Option<u64>
```

`from_transition` always writes `Some(deadline.unix_millis())`.

With serde JSON, `Some` serializes as the numeric field.

`into_transition` maps a missing legacy value to zero and a present value to
its exact `CompletionDeadline`.

The schema version constant remains 1.

### Folding

Pass correlation and deadline into `CompletionEvent::CommandLaunched`.

Update state pattern matches to tolerate both fields.

The aggregate needs no separate deadline field because the typed state owns it.

### Done masking

Extend `masks_durable_done` to return true for action-required Rejected state.

Requested and CommandInFlight behavior remains unchanged.

Retryable Rejected and Confirmed remain unmasked.

### Journal tests

Update transition helpers and expected CommandInFlight values.

Assert serialized new records contain the deadline field.

Add a legacy JSONL fixture with no deadline field and assert reconstruction to
deadline zero.

Assert action-required rejection masks Done while retryable rejection does not.

## `crates/lisa-plugin/src/lib.rs`

### Constants and imports

Import `CompletionDeadline` from `lisa_core::completion`.

Add:

```rust
const COMPLETION_RECONCILIATION_TIMEOUT_SECS: u64 = 60;
```

Keep the constant next to the poll and other scheduler timeout constants.

### Pending state

Extend `PendingCompletion` with:

```rust
deadline: CompletionDeadline,
is_reconciliation_replay: bool,
```

Initial launch sets the flag false.

Restart replay sets the flag true.

### Time helpers

Add a pure conversion from `SystemTime` to `CompletionDeadline`.

Add a deadline addition helper using saturating millisecond arithmetic.

Production wrappers call `SystemTime::now()`.

Testable internal methods accept explicit `SystemTime` or
`CompletionDeadline`.

### Dispatch boundary

Keep `dispatch_completion(input)` as the production wrapper.

Add `dispatch_completion_at(input, now)` for deterministic tests.

Pass `now` to the core reconciler.

Map `ReplayCommandInFlight` to the replay adapter method.

Map `CommandInFlightDeadlineExceeded` to the timeout transition method.

All non-Reconcile inputs retain reducer behavior and initial execution flow.

### Initial effect executor

Keep `execute_completion_effect` as the sole new-generation boundary.

Add an explicit-time internal form if required by tests.

Compute the deadline once before appending CommandInFlight.

Persist that deadline and install it in pending state.

Do not alter command argv or generation calculation.

### Replay method

Add:

```rust
fn replay_in_flight_completion(
    &mut self,
    ticket_id: TicketId,
    source_lease: AttemptLease,
    correlation: CorrelationId,
    deadline: CompletionDeadline,
) -> bool
```

Validate journal health and no current pending entry.

Clone the matching aggregate data needed before mutating State.

Require aggregate key attempt to match the current source lease.

Require in-flight correlation and deadline to match the core decision.

Resolve the existing ticket file and build the same completion command.

Install a replay pending entry using aggregate prior phase and status.

Record the inert test effect consistently with the initial executor.

Run the command with the same context and cwd.

Do not append journal records at replay launch.

### Timeout method

Add a method that accepts ticket, correlation, and deadline.

Revalidate the aggregate is the matching in-flight state.

Append a correlated Rejected transition with ActionRequired retryability.

Only after append succeeds, remove live pending state and rebuild the DAG.

Log the existing typed `LaunchFailed` rejection with a stable timeout message.

Return whether it durably transitioned.

### Result handling

Keep authority validation and success confirmation shared.

In the nonzero/malformed-result branch, inspect
`is_reconciliation_replay`.

For an initial command, retain the current retryable Rejected append.

For replay, remove the live pending entry, log a warning, and leave the durable
CommandInFlight state unchanged.

This branch must not rebuild into a fresh deadline-bearing Requested state.

Duplicate or late results continue to no-op when no pending entry exists.

### Existing tests

Update all `PendingCompletion` literals with deadline and replay flag.

Update CommandInFlight assertions and journal field expectations.

Use a common fixed deadline helper where it improves readability.

### New convergence test

Place the regression beside the existing restart journal test.

Reuse the plugin crate's existing `lisa-cli` dev-dependency.

Create a temporary Git repository with configured identity.

Create Review ticket, canonical work files, attempt-private Review files, a
thread, current lease, slot, journal, and ledger.

Drive initial adapter dispatch with explicit time.

Execute the built key through real `complete_ticket` and withhold the result.

Assert Git contains exactly one new completion commit and Done bytes.

Create a restarted State and restore its journal.

Reinstall only the current scheduler authority fixtures required by the adapter.

Dispatch duplicate Stopped and Reconcile observations before the deadline.

Assert only one replay is pending.

Execute `complete_ticket` with the journal's same key again.

Assert returned commit equals the first and Git commit count is unchanged.

Deliver the replay result.

Assert Confirmed, one authoritative provenance record, one completion commit,
and released scheduler state.

### New timeout test

Drive a completion in-flight at a fixed initial time.

Reconcile at the exact stored deadline.

Assert the journal reconstructs action-required Rejected.

Assert pending state is removed and no further launch is recorded after later
Reconcile and duplicate Stopped inputs.

Assert Done masking remains active for this uncertain terminal outcome.

## `crates/lisa-cli/src/commit_transaction.rs`

No source modification.

Its public `complete_ticket` and `CompleteTicketRequest` APIs are consumed by
the plugin integration test.

Its existing marker discovery is the production convergence mechanism.

## Commit units

Commit core domain plus external core tests as one meaningful unit because the
public signature and all callers must compile together.

Commit plugin journal and adapter/tests as one meaningful unit because replay
requires the durable deadline representation and scheduler boundary together.

Use exact includes only.

Phase artifacts remain uncommitted in the private attempt directory for Lisa to
admit later.
