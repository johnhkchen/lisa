# Structure: persist pre-ownership failure row

## Modified file

`crates/lisa-plugin/src/lib.rs`

No source files are created or deleted.

No public crate interface changes.

## Import boundary

Extend the existing `lisa_core::provenance` import list with:

- `AssignmentState`;
- `AssignmentTransitionRecord`;
- `ProvenanceRecordType`.

Keep the module-qualified append and time helpers under `provenance::`.

The plugin remains the policy owner for when a durable row is emitted.

The core remains the schema and filesystem-append owner.

## Terminal transition call sites

Modify `State::fail_assignment_delivery`.

After a valid ticket/thread transition, call the shared assignment-transition
writer with:

- the physical `pane_id`;
- the resolved `ticket_id`;
- `AssignmentState::DeliveryFailed`;
- the original `reason`.

Modify `State::fail_assignment_recovery` similarly with
`AssignmentState::RecoveryFailed`.

Modify `State::fail_startup` similarly with
`AssignmentState::StartupFailed`.

Do not alter source-state guards or returned `FailureTransitionOutcome` values.

Do not alter reset, fencing, alert, or scheduling policy.

Do not add emission to `fail_startup_recovery` in this ticket.

## New private writer

Place `State::emit_assignment_transition` adjacent to the existing
`State::emit_provenance` method.

This keeps all ledger-writing policy in one plugin section.

Inputs:

```text
&mut self
pane_id: u32
ticket_id: &str
state: AssignmentState
reason: &str
```

Output:

```text
bool
```

An empty `ledger_path` returns false without filesystem access.

The writer resolves a slot whose pane and ticket both match.

It clones the slot attempt lease.

It resolves the matching thread.

It checks:

- lease ticket equals `ticket_id`;
- thread ticket equals `ticket_id` by map/key invariant;
- thread pane equals `pane_id`;
- thread attempt lease equals the slot lease.

If required evidence is missing or inconsistent, log a warning and return
false.

Use `thread.client` to derive `Route::from_client(client).provider`.

Convert `thread.started_at` and current time with
`provenance::system_time_to_epoch`.

Construct `AssignmentTransitionRecord` with current schema version and
`ProvenanceRecordType::AssignmentTransition`.

Compute duration with `ended_at.saturating_sub(started_at)`.

Append through `provenance::append_assignment_transition_record`.

On append error, add an `ActivityEvent::Error` that identifies ticket and
failure type, then return false.

On success, return true.

The method must not mutate the ticket, lease, slot, thread, or seat assignment.

## Test helpers

Keep the execution-only `read_ledger` helper for existing tests.

Add `read_mixed_ledger` returning
`Vec<lisa_core::provenance::ProvenanceLedgerRecord>`.

This prevents churn in tests that only exercise execution records.

Add or adapt a local reserved-state constructor for transition tests.

The fixture installs:

- one `AgentSlot` for pane 10 and ticket `T-NAME`;
- a minted attempt lease in the slot and thread;
- the lease in current and high-water maps;
- a `Thread` stamped with the chosen `AgentClient`;
- the requested source `SeatAssignmentState`;
- a temporary ledger path.

## Transition test organization

One table-like test may create three independent state/temporary-directory
instances because each transition is terminal.

Delivery case:

- provider client Claude;
- source state Delivering;
- exact reason unique to delivery;
- expected vendor `anthropic`;
- expected durable state `DeliveryFailed`.

Recovery case:

- provider client Codex;
- source state Recovering;
- exact reason unique to recovery;
- expected vendor `openai`;
- expected durable state `RecoveryFailed`.

Startup case:

- provider client Claude or Codex;
- source state Starting;
- exact reason unique to startup;
- expected matching vendor;
- expected durable state `StartupFailed`.

For every case, assert one row and exact identity/state/reason fields.

For every case, invoke the same helper again and assert it returns `None` and
the ledger length remains one.

Parse raw first-line JSON to assert no `authoritative` and no `outcome` keys.

## Coexistence test organization

Use the delivery state after its first transition or a dedicated state.

Preserve the initial assignment-transition row.

Install a later matching current lease and running thread for the same ticket.

Invoke `emit_provenance(ticket, RunOutcome::Done, false)`.

Read through the mixed enum.

Assert:

- there are exactly two rows;
- row zero is the original assignment transition;
- row one is an execution record;
- row one is Done and authoritative;
- row one does not replace row zero.

## Existing characterization update

In `assignment_recovery_failure_retains_authority_for_operator_reset`, replace
the assertion that the ledger does not exist.

Read the new row and assert recovery state, successor lease, pane, provider,
and reason from the real timeout path.

Retain all existing assertions about state, authority, slot retention, thread
status, and no automatic repeated retry.

## Ownership and commit boundary

The only ticket-owned source path is:

```text
crates/lisa-plugin/src/lib.rs
```

The isolated Lisa commit must include exactly that repository-relative path.

Attempt-private phase artifacts remain outside the source commit and are
published later by Lisa.

Unrelated ticket and provenance working-tree changes must remain untouched.
