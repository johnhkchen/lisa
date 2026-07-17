# Structure — T-048-01-02 park-instead-of-churn

## Change inventory

Modify two source files:

- `crates/lisa-core/src/provenance.rs`;
- `crates/lisa-plugin/src/lib.rs`.

No source file is created, deleted, or moved.

Private phase artifacts remain under the assigned attempt work directory.

Ticket frontmatter is not manually edited by this implementation session; only
runtime tests exercise scheduler-owned status mutation on temporary fixtures.

## Core provenance changes

File: `crates/lisa-core/src/provenance.rs`.

### Schema version

Advance `SCHEMA_VERSION` from 4 to 5.

Historical fixtures retain explicit older versions and remain replayable.

### Transition discriminator

Extend:

```rust
pub enum ParkingTransitionType {
    Retry,
    Park,
    Unpark,
}
```

Serde remains kebab-case, producing `retry`, `park`, and `unpark`.

### Record fields

Extend `ParkingTransitionRecord` after `remedy_owner` with:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub retry_count: Option<u8>,

#[serde(default, skip_serializing_if = "Option::is_none")]
pub retry_limit: Option<u8>,

#[serde(default, skip_serializing_if = "std::ops::Not::not")]
pub recheck_eligible: bool,
```

The exact boolean skip helper may use a local `is_false` function if the method
path is less readable or unsupported by serde's predicate signature.

Retry records require count and limit by writer convention.

Agent park records also carry the final count and limit.

Operator/world park and all unpark records may omit retry metadata.

Only world park/unpark episodes carry `recheck_eligible = true`.

### Documentation

Broaden module and type comments from only park/unpark to block retry and
parking transitions.

Clarify that interval fields on Unpark quantify the parked duration.

### Core tests

Update the sample parking constructor with explicit new fields.

Retain park/unpark compact round-trip and mixed-ledger coverage.

Add a retry row assertion covering:

- `record_type: retry`;
- attempt lease;
- owner agent;
- `retry_count`;
- `retry_limit`;
- false/omitted recheck marker;
- round-trip equality.

Add a world park assertion covering explicit `recheck_eligible: true`.

Add a schema-4 JSON fixture without the new fields and assert defaulted replay.

## Plugin imports and constants

File: `crates/lisa-plugin/src/lib.rs`.

Import `RemedyOwner` beside `ReviewDisposition`.

Import `ParkingTransitionRecord` and `ParkingTransitionType` beside existing
provenance types.

Add a scheduler-local constant near other bounded retry constants:

```rust
const MAX_AGENT_BLOCK_RETRIES: u8 = 2;
```

Its comment defines the count as fresh Review re-attempts allowed after an
agent-owned block during one loop process.

## Pure policy types

Add a small private enum near other scheduler transition results:

```rust
enum ReviewBlockAction {
    Retry { retry_count: u8, retry_limit: u8 },
    Park {
        retry_count: Option<u8>,
        retry_limit: Option<u8>,
        recheck_eligible: bool,
    },
}
```

Add a pure function:

```rust
fn review_block_action(
    owner: RemedyOwner,
    retries_consumed: u8,
) -> ReviewBlockAction
```

This function owns exact bounded policy and can be unit tested without file or
Zellij effects.

## State extension

Add one defaultable field to `State`:

```rust
agent_block_retries: HashMap<TicketId, u8>,
```

The field is documented as per-loop policy memory, not scheduling authority.

It is absent from config, snapshots, ticket files, and completion state.

## Provenance writer method

Add a private method beside existing provenance writers:

```rust
fn emit_parking_transition(
    &mut self,
    ticket_id: &str,
    owner: RemedyOwner,
    record_type: ParkingTransitionType,
    retry_count: Option<u8>,
    retry_limit: Option<u8>,
    recheck_eligible: bool,
    started_at: SystemTime,
) -> bool
```

For retry/park, it obtains the attempt lease from the current thread and checks
ticket/lease consistency before append.

It uses current time for `ended_at` and saturating duration.

Unpark reconstruction may append through a second method accepting a complete
prior park record, because no live thread/lease exists after parking.

All failures log an activity event and return false, matching existing writer
style.

## Review block policy method

Add:

```rust
fn apply_review_block_policy(&mut self)
```

The method snapshots current Review tickets and current leases.

For each candidate it invokes `review_completion_inputs` so artifacts pass
through existing current-attempt admission.

It destructures only `ReviewDisposition::Block`, preserving all structured
fields in the canonical admitted file.

The owner and current counter feed `review_block_action`.

### Retry branch

- emit `Retry` row with exact count/limit;
- update `agent_block_retries`;
- release slot/lease;
- remove thread;
- log an informational bounded-retry event.

### Park branch

- locate ticket path;
- call `ticket::update_ticket_status(..., TicketStatus::Blocked)`;
- on failure log and continue without teardown;
- emit `Park` row;
- release slot/lease;
- remove thread;
- log owner, ask, recheck marker, and bound where applicable.

After one or more successful parks, rebuild the DAG once.

The method does not call completion or schedule directly.

## Unpark reconciliation

Add:

```rust
fn reconcile_unpark_transitions(&mut self)
```

Return immediately when `ledger_path` is empty or unreadable.

Deserialize each line as `ProvenanceLedgerRecord` and retain the latest
`ParkingTransitionRecord` per ticket in ledger order.

For each latest `Park` record:

- look up the ticket in the current DAG;
- require `status == Open` and `phase != Done`;
- append one `Unpark` row with the park's owner, attempt lease, start timestamp,
  retry metadata, and recheck marker;
- clear `agent_block_retries` for that ticket.

Retry as the latest record is not an unpark candidate because the ticket was
never durably parked.

Unpark as the latest record makes the method idempotent.

## Poll integration

In `poll_tick`, call `apply_review_block_policy` after artifact advancement and
before existing level-triggered completion reconciliation and timeout policy.

This gives valid blocks an immediate seat-releasing consequence.

After the ordinary `rebuild_dag`, call `reconcile_unpark_transitions` before
the final `schedule_ready_tickets`.

On plugin load, call `reconcile_unpark_transitions` after the initial DAG is
installed and ledger path is known. This captures an external open edit that
occurred while the loop was stopped.

Scheduling remains status-driven and does not consult provenance.

## Test fixture helpers

Reuse `fresh_slot`, `install_current_attempt`, and mixed-ledger readers in the
existing inline test module.

Add a helper to write temporary ticket markdown with configurable phase/status.

Add a helper to attach a Review thread and slot to a ticket, install a current
attempt, and write attempt-local `review.md` plus a supplied block disposition.

The helper must stamp slot, thread, and lease consistently so production
validation paths are exercised.

## Scheduler tests

### Pure policy test

Assert:

- operator at count zero parks;
- world at count zero parks and is recheck eligible;
- agent at zero retries as 1/2;
- agent at one retries as 2/2;
- agent at two parks with 2/2;
- counts above the limit saturate to a park.

### Two-seat 2026-07-16 replay

Build four independent tickets:

- operator-owned Review block occupying pane 10;
- world-owned Review block occupying pane 11;
- ready ticket A;
- ready ticket B.

Set `max_threads = 2`, install exact leases/artifacts, and use a temporary
ledger.

Apply block policy, rebuild, reconcile unpark, and schedule.

Assert both blocker statuses are durably blocked, neither has a thread or slot,
both ready tickets own the two seats, repeated scheduling never selects either
blocker, two Park rows exist, and only the world row is recheck eligible.

### Agent bound and unpark replay

Start one agent-owned Review ticket with one slot.

For attempts 1 and 2, apply policy and assert a new attempt is schedulable.

On attempt 3, assert the ticket becomes blocked and no further attempt starts.

Assert ledger order Retry 1/2, Retry 2/2, Park 2/2.

Change temporary frontmatter to `status: open`, rebuild, reconcile unpark, and
schedule. Assert one Unpark row and a new running attempt without any parked
allow-list mutation.

## Compatibility checks

`read_usage` continues ignoring every `ParkingTransition` variant through its
existing wildcard over the record enum variant.

No UI public type changes are required; blocked ticket status is already
rendered.

No CLI configuration or template change is required.

No check execution boundary is added.

No ticket Done/phase logic is changed.
