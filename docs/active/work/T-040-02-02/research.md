# Research: persist pre-ownership failure row

## Ticket scope

T-040-02-02 concerns failures that occur after Lisa has reserved a physical
agent pane and minted an attempt lease, but before the provider has positively
accepted ownership of the ticket.

The three named transition sites are:

- `State::fail_assignment_delivery`;
- `State::fail_assignment_recovery`;
- `State::fail_startup`.

The requested evidence is attempt-scoped and append-only.

The evidence must identify the ticket, attempt, pane, provider, named terminal
state, and caller-supplied reason.

It must not represent a successful or authoritative execution outcome.

It must remain beside any later terminal execution row for the same ticket.

## Provenance schema

`crates/lisa-core/src/provenance.rs` owns ledger schemas and filesystem append
behavior.

The module's current schema version is 3.

The ledger is intentionally heterogeneous.

`ProvenanceRecord` is the established terminal execution row.

It includes an execution `outcome`, an `authoritative` boolean, requested and
actual routes, usage, concurrency, and timing.

`AssignmentTransitionRecord` is the pre-ownership row introduced by dependency
T-040-02-01.

It contains:

- `schema_version`;
- `record_type`;
- `ticket_id`;
- `attempt_lease`;
- `pane_id`;
- `provider`;
- `state`;
- `reason`;
- `started_at`;
- `ended_at`;
- `wall_clock_secs`.

`ProvenanceRecordType::AssignmentTransition` serializes as
`"assignment-transition"`.

`AssignmentState` provides the stable evidence vocabulary.

Its variants are `DeliveryFailed`, `RecoveryFailed`, and `StartupFailed`.

They serialize in kebab case.

The assignment row deliberately has no execution `outcome` and no
`authoritative` field.

That structural distinction prevents a pre-ownership failure from being read
as an authoritative ticket execution result.

`ProvenanceLedgerRecord` is an untagged reader over assignment-transition and
execution rows.

The assignment variant remains distinguishable through its required
`record_type`, `state`, and `reason` fields.

`append_assignment_transition_record` serializes one assignment row and calls
the module's shared append primitive.

The primitive creates a missing parent directory, opens the ledger in append
mode, and writes one newline-terminated JSON object.

It never truncates or rewrites existing rows.

`system_time_to_epoch` converts `SystemTime` to saturating UTC epoch seconds.

## Plugin imports and state

`crates/lisa-plugin/src/lib.rs` contains the scheduler state machine and all
three named terminal helpers.

It currently imports `ProvenanceRecord`, `Route`, and `RunOutcome` from the
core provenance module.

`State::ledger_path` is established under `.lisa/provenance.jsonl` during host
configuration.

An empty ledger path is used by many native unit tests and before plugin load.

The existing execution writer treats an empty path as a no-op.

`State::threads` is keyed by ticket ID.

Each `Thread` retains the client selected at spawn, its pane ID, its
`started_at` time, and its attempt lease.

`State::agent_slots` represents physical seats.

Each bound slot retains pane ID, ticket ID, attempt lease, session and client
facts, and transition metadata.

`State::current_leases` holds current attempt authority.

`State::lease_high_water` preserves monotonically increasing attempt identity
across retries and resets.

The relevant terminal failures retain their slot and thread for operator
inspection rather than immediately tearing them down.

## Assignment state machine

`SeatAssignmentState` is scheduler-owned assignment truth.

`Starting` means the provider command was launched but its exact process-start
signal has not been observed.

`ReadyForAssignment` means process-start was observed and bounded assignment
delivery can begin.

`Delivering` means the chat assignment was submitted and exact provider
acknowledgment is pending.

`AssignedPendingAck` represents a reused Codex session pending acknowledgment.

`Recovering` represents the one permitted fresh-session fallback after an
expired reused-session delivery.

`DeliveryFailed`, `RecoveryFailed`, and `StartupFailed` are retained terminal
states requiring an operator reset.

The failure helpers guard their source states before doing any mutation.

Once a helper changes the seat to a terminal state, a repeated invocation does
not satisfy its source-state guard.

That guard is the existing transition-level idempotence boundary.

## Delivery failure path

`fail_assignment_delivery` accepts `Starting`, `ReadyForAssignment`, or
`Delivering`.

It first changes the seat state to `DeliveryFailed`.

It then resolves the ticket ID through the pane's bound slot.

With no ticket reservation it logs an error and returns a path-specific outcome
whose ticket ID is absent.

With a ticket it marks the thread failed, deduplicates the dashboard error
alert, logs the reason, and returns the ticket-bearing outcome.

The reason is passed in by the concrete timeout or send-failure caller.

The current helper emits no ledger evidence.

## Recovery failure path

`fail_assignment_recovery` accepts only `Recovering`.

It first changes the seat state to `RecoveryFailed`.

It resolves the ticket through the pane's slot.

With a ticket it marks the thread failed, deduplicates the alert, logs the
reason, and retains the current recovery lease for operator reset.

Tests confirm that the successor recovery attempt remains in
`current_leases`, `lease_high_water`, the slot, and the thread.

The current helper emits no ledger evidence.

## Startup failure path

`fail_startup` accepts only `Starting`.

It first changes the seat state to `StartupFailed`.

It resolves the ticket through the pane's bound slot.

With a ticket it marks the thread failed, deduplicates the alert, logs the
reason, and retains the reservation.

The current helper emits no ledger evidence.

`fail_startup_recovery` is a separate, stronger path for exhausted same-pane
shell reset or replacement startup.

That helper revokes authority, clears lifecycle signals, fences and closes the
physical pane, and returns `StartupRecoveryFailed`.

The ticket acceptance criterion names `fail_startup`, not
`fail_startup_recovery`.

## Existing execution writer

`State::emit_provenance` appends terminal execution rows.

It runs while the thread still exists so spawn-time facts remain available.

It rejects a missing thread or attempt lease.

It rejects authoritative Done for a stale attempt.

It derives provider and method from `thread.client` through
`Route::from_client`.

It derives timing from `thread.started_at` and the current system time.

It logs append failures without making them fatal to scheduler operation.

It returns whether a row was appended.

## Test organization

Most plugin unit tests live in the `#[cfg(test)]` module at the bottom of
`crates/lisa-plugin/src/lib.rs`.

`with_ledger` points a state at a temporary ledger and provider usage dirs.

The existing `read_ledger` helper deserializes every line as the execution-only
`ProvenanceRecord` shape.

A mixed-row test therefore needs either a new heterogeneous reader helper or
direct `serde_json::Value` parsing.

`retained_failure_helpers_return_path_specific_outcomes` directly exercises
all terminal helper return variants with a lightweight reserved state.

Its helper currently installs no attempt lease.

`assignment_recovery_failure_retains_authority_for_operator_reset` drives the
real recovery timeout transition and currently asserts that no ledger exists.

That assertion describes the behavior this ticket is intended to replace.

The provenance test section already checks append-not-rewrite behavior for
execution rows and checks that a failed/timed-out predecessor can coexist with
one authoritative Done replacement.

## Constraints

The source work is expected to remain in `crates/lisa-plugin/src/lib.rs`.

The core schema and append API already exist from the dependency ticket.

No ticket frontmatter may be changed by provenance emission.

Ledger write failure must remain non-fatal to scheduling.

Missing reservation or attempt facts cannot produce the required complete
attempt-scoped row without fabrication.

Repeated calls after the named terminal transition must not append duplicates.

Existing unrelated working-tree changes belong to other Lisa activity and
must not be included in this ticket's isolated commit.
