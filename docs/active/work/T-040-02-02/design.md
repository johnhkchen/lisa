# Design: persist pre-ownership failure row

## Objective

Append one truthful `AssignmentTransitionRecord` when each named retained
pre-ownership failure transition succeeds.

The row must be complete, non-execution evidence and must use the append-only
core API introduced by T-040-02-01.

The scheduler transition remains authoritative for deciding whether an append
is permitted.

## Option 1: emit at timeout callers

Each branch of `check_assignment_ack_timeouts_at` could construct and append a
row after receiving a failure outcome.

This makes the injected `now` value directly available for deterministic
timestamps.

It also keeps filesystem work outside the state mutators.

However, the named failure methods have callers outside that timeout loop.

Delivery can fail immediately because an assignment file or reservation is
missing.

Startup failures can be reached from startup recovery invariant checks.

Recovery failure also has provider-error call sites.

Wiring every caller would duplicate record construction and make exact-once
behavior dependent on remembering every present and future call site.

This option does not match the acceptance criterion's explicit statement that
the three helpers append through provenance.

It is rejected.

## Option 2: emit after helper return through a shared outcome dispatcher

The three helpers could remain pure state mutators and a common dispatcher
could map `FailureTransitionOutcome` values into provenance rows.

This centralizes state-to-schema mapping.

It would require every caller to route the returned value through the
dispatcher.

Several callers currently ignore the returned value because it exists mainly
for test and timeout reporting.

Making persistence conditional on return-value handling would be fragile.

It would also complicate call sites that return a nested failure outcome
directly.

This option is rejected.

## Option 3: emit inside each guarded terminal helper

Each of the three named helpers can call one private shared record-construction
method after resolving a valid ticket reservation and before returning.

The source-state guard already guarantees a successful terminal edge happens
at most once.

After the helper stores `DeliveryFailed`, `RecoveryFailed`, or `StartupFailed`,
a repeated call is rejected before emission.

All callers automatically receive durable behavior without duplicated wiring.

The transition helper still performs its existing state, thread, alert, and
activity mutations.

One shared writer prevents drift in lease lookup, provider derivation, timing,
append error handling, and empty-ledger behavior.

This option is selected.

## Emission ordering

The helper first validates its allowed source state.

It then installs the named terminal seat state.

It resolves the ticket reservation.

If no ticket is bound, a complete attempt-scoped row cannot be formed and the
existing repair alert remains the only truthful response.

For a valid reservation, the helper marks the thread failed and maintains the
existing alert.

It invokes the shared provenance writer once using the terminal evidence state
and exact caller-provided reason.

It then writes the existing activity message and returns its path-specific
outcome.

Placing append after the terminal state mutation preserves transition
idempotence even if filesystem append fails.

A write failure must not roll the scheduler back into a source state, because
that would enable repeated scheduling or repeated append attempts from the
same observed terminal edge.

As with execution provenance, persistence failure is logged and swallowed.

## Shared writer interface

Add a private method conceptually shaped as:

```text
emit_assignment_transition(pane_id, ticket_id, state, reason) -> bool
```

The method accepts `AssignmentState`, not `SeatAssignmentState`.

This keeps the durable schema vocabulary explicit at each terminal call site
and prevents leaking the private scheduler enum into the core contract.

The method returns append success for focused tests and diagnostic consistency
with `emit_provenance`.

The three production callers need not branch on the boolean.

## Identity lookup

The pane's bound `AgentSlot` is the physical assignment boundary.

The writer locates the slot by both pane ID and ticket ID.

It takes the exact `AttemptLease` from that slot.

It verifies that the lease's ticket ID matches the supplied ticket ID.

The matching thread supplies provider and start time.

The writer also checks that the thread pane and attempt lease match the slot.

These consistency checks make malformed partial state fail closed rather than
produce misleading evidence.

Current-lease authority is not required for all pre-ownership evidence.

The row describes the exact failed attempt, not permission to publish Done.

The three selected helpers currently retain their attempt leases, so their
normal path passes the consistency checks.

## Provider

Provider is derived from `thread.client` through `Route::from_client`.

Only the `provider` member is copied into the assignment row.

This yields `anthropic` for Claude and `openai` for Codex, matching execution
provenance and the dependency ticket's corrected schema semantics.

The caller cannot supply a free-form provider spelling.

## Named states

`fail_assignment_delivery` maps to `AssignmentState::DeliveryFailed`.

`fail_assignment_recovery` maps to `AssignmentState::RecoveryFailed`.

`fail_startup` maps to `AssignmentState::StartupFailed`.

`fail_startup_recovery` is left unchanged because it is not one of the three
named acceptance sites and represents a separate lease-revoking fence path.

## Authority semantics

The writer constructs `AssignmentTransitionRecord`, never `ProvenanceRecord`.

Consequently the JSON has no `outcome` and no `authoritative` member.

This is stronger than encoding `authoritative: false` on an execution row:
downstream readers cannot confuse the observation with an execution result.

Tests will assert the assignment variant and absence of those execution-only
keys in raw JSON.

## Timing

`ended_at` is captured from `SystemTime::now()` at emission.

`started_at` uses the thread's attempt start time.

The state machine does not retain a dedicated assignment-transition timestamp
across all three selected paths; `AgentSlot::transition_started_at` tracks
shell `/exit` and `/clear` transport and is cleared on fresh launch.

The thread start is therefore the stable common beginning of the pre-ownership
attempt represented by the row.

`wall_clock_secs` uses saturating subtraction.

Focused tests can set the thread start in the past and assert coherent timing
without depending on exact wall-clock equality.

## Exact-once boundary

Exactly once means once per successful named state transition.

The helper source-state guard is evaluated before any append.

The first call changes the state and appends one row.

The second call sees a terminal state, returns `None`, and performs no append.

No separate in-memory deduplication set is needed.

This avoids introducing state that would need persistence or reset semantics.

## Coexistence with later terminal execution

Both assignment and execution writers open the same ledger with append mode.

The test will drive a pre-ownership failure, then append a later terminal
execution row for the same ticket.

The mixed reader will observe two rows in original order.

The assignment row remains non-execution evidence.

The later execution record remains intact and can be authoritative when it is
a current-lease Done record.

No writer truncates, replaces, or updates the earlier line.

## Tests

Add a heterogeneous ledger helper using `ProvenanceLedgerRecord`.

Create a reserved-state test fixture with:

- a pane-bound slot;
- a matching attempt lease in slot and thread;
- a selected provider client;
- a current/high-water lease;
- an enabled temporary ledger.

Drive each of the three real terminal helpers.

For each row assert ticket, lease, pane, vendor provider, durable state, exact
reason, schema and record type, and coherent timing.

Assert raw JSON omits `outcome` and `authoritative`.

Call each helper again and assert the ledger still contains one row.

For one ticket, restore a later current attempt/thread and call the existing
execution writer with Done.

Assert the ledger contains the original assignment row followed by one
authoritative terminal execution row.

Update the existing recovery characterization that currently expects no
ledger; it should instead assert the newly persisted row.

## Scope

Only `crates/lisa-plugin/src/lib.rs` changes.

No core schema, public API, ticket frontmatter, CLI reader, or knowledge
document changes are needed for this ticket.
