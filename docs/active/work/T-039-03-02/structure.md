# Structure: named failure transition outcomes

## Files modified

### `crates/lisa-plugin/src/lib.rs`

This is the only ticket-owned source file. Scheduler states, transition
implementations, poll orchestration, and native invariant tests already live in
this module. Keeping the result private avoids creating a public API for an
internal scheduling concern.

Add `FailureTransitionOutcome` beside `FenceOutcome` and
`AttemptLifecycleEvent`, the existing outcome/observation types for failure
boundaries.

The enum derives `Debug`, `Clone`, `PartialEq`, and `Eq` so tests and future
internal consumers can inspect it without conversion.

Variants and fields:

```text
AssignmentDeliveryFailed { pane_id, ticket_id: Option<TicketId> }
AssignmentRecoveryFailed { pane_id, ticket_id: Option<TicketId> }
StartupFailed { pane_id, ticket_id: Option<TicketId> }
StartupRecoveryFailed { pane_id, ticket_id: TicketId }
ErrorReclaimed { pane_id, ticket_id: TicketId }
SessionTimedOut { pane_id, ticket_id: TicketId, fenced: bool }
StaleThreadReclaimed { pane_id, ticket_id: TicketId, fenced: bool }
```

## Retained helper signatures

Change these signatures from `()` to `Option<FailureTransitionOutcome>`:

```text
fail_assignment_delivery
fail_assignment_recovery
fail_startup
fail_startup_recovery
```

Every current early return becomes `None`. Malformed reservation branches that
already commit a seat transition and log an error return `Some` with a missing
ticket. Normal terminal branches return `Some` after their final log.

No caller uses the value to authorize follow-on mutation.

## Automatic reclaim scanner signatures

Change these methods to return `Vec<FailureTransitionOutcome>`:

```text
check_error_signals
check_session_timeouts
detect_stale_threads
```

Each initializes an empty vector, preserves all existing selection and mutation
logic, pushes one named variant after each completed failure/reclaim, and
returns the vector.

`check_session_timeouts` returns the empty vector when both timeout systems are
disabled instead of a bare early return.

For timeout and stale paths, capture `pane_id` from the thread before removal.
This is existing scheduler identity, not a new source of authority.

## Orchestration call sites

Calls inside the update/poll path remain sequencing-only. Returned values may be
ignored because state mutation and logs remain their operational effects.

Calls from other transition helpers likewise remain sequencing-only. The typed
return is primarily a truthful boundary and an assertion seam.

## Tests in `lib.rs`

Modify existing T-039-03-01 matrix-bearing tests rather than building a
parallel fixture suite. Capture the returned outcome at the terminal invocation
or scanner call and compare it to the exact enum variant.

Where a test invokes a scanner more than once, assert the reclaiming call
returns one outcome and later idempotence calls return an empty vector when
useful.

Retain every existing assertion for current/high-water lease, seat state,
thread presence/status, pane reservation/fencing, provenance, alerts, retries,
and lifecycle ordering.

## Files not modified

`lisa-core` is unchanged because the result is scheduler-local and not a
persisted domain record.

`ui.rs` is unchanged because the dashboard already renders terminal seat state
and alerts; this ticket names transition results without adding retained state.

`signal.rs` is unchanged because ingestion classifies transport records, while
the scheduler decides whether an error is a retained recovery failure, an
ordinary reclaim, or a no-op.

No ticket, shared work artifact, workflow file, configuration, serialization
schema, or CLI surface changes.

## Change ordering

1. Add the enum so signature and test edits compile against one definition.
2. Convert four retained failure helpers and their early returns.
3. Convert three reclaim scanners and preserve mutation order.
4. Add exact outcome assertions to matrix tests.
5. Format and run focused tests.
6. Run complete gates.
7. Commit only `crates/lisa-plugin/src/lib.rs` through `lisa commit-ticket`.

## Architectural invariants

The type describes completed transitions; it does not replace authoritative
state.

`SeatAssignmentState`, `threads`, `current_leases`, `lease_high_water`, and
`agent_slots` remain scheduler truth.

`RunOutcome` remains provenance truth.

`FenceOutcome` remains the lower-level pane fencing result.

The new enum is not serialized and creates no compatibility obligation outside
the plugin implementation.
