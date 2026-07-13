# Structure — T-045-03-03 delivered awaiting claim

## Change set overview

Modify three existing source files:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ui.rs`;
- `crates/lisa-core/src/provenance.rs`.

Create no production module.
Delete no file.
Change no configuration schema, signal schema, claim schema, launcher, or adapter.

The scheduler remains the authority.
UI and provenance changes project the scheduler's two new named states.

## `crates/lisa-plugin/src/lib.rs`

### `SeatAssignmentState`

Add a deadline-bearing state after `Delivering`:

```rust
DeliveredAwaitingClaim {
    generation: u64,
    claim_deadline: SystemTime,
}
```

Its documentation states:

- the assignment was delivered to a live Codex TUI;
- hook evidence did not arrive in the initial window;
- Lisa is waiting passively for claim or fallback evidence;
- no assignment reinjection is allowed from this state.

Add a terminal state near other retained failures:

```rust
ClaimTimedOut
```

Its documentation states that the delivered assignment never produced admissible
ownership evidence before the passive bound and requires operator reset.

### `FailureTransitionOutcome`

Add:

```rust
AssignmentClaimTimedOut {
    pane_id: u32,
    ticket_id: Option<TicketId>,
}
```

This is the typed result of an already-completed terminal mutation.
It mirrors `AssignmentDeliveryFailed` without conflating the reason.

### Active-generation boundary

Extend `active_assignment_generation` so
`SeatAssignmentState::DeliveredAwaitingClaim` returns its generation.

No evidence-specific admission method changes shape.

The existing claim, hook, and artifact methods continue to validate:

- pane route;
- slot ticket;
- state generation;
- slot lease;
- current lease;
- retained assignment identity where applicable.

### Live Codex predicate

Add a private helper beside assignment deadline/state helpers:

```rust
fn is_live_codex_delivery(&self, pane_id: u32, generation: u64) -> bool
```

It inspects the matching `AgentSlot` and requires:

- ticket reservation present;
- `has_session` true;
- `last_client == Some(AgentClient::Codex)`;
- slot attempt ID equals the supplied generation;
- slot lease ticket equals the reservation;
- slot lease is current in `current_leases`.

The helper performs no mutation and no provider I/O.

### Passive transition helper

Add a small private mutation helper or an inline timeout branch that:

1. receives pane, generation, and injected `now`;
2. confirms the exact `Delivering` state remains present;
3. computes `assignment_ack_deadline(now)`;
4. inserts `DeliveredAwaitingClaim`;
5. logs one informational or warning event naming the ticket, pane, and passive wait;
6. performs no `send_line_to_pane` call.

Prefer a named helper if it improves direct testability and keeps timeout matching
readable.

### Terminal failure helper

Add:

```rust
fn fail_assignment_claim_wait(
    &mut self,
    pane_id: u32,
    reason: &str,
) -> Option<FailureTransitionOutcome>
```

The helper accepts only `DeliveredAwaitingClaim`.

It inserts `ClaimTimedOut` before any fallible evidence write.
That insertion is the exact-once terminal guard.

It then:

- resolves the ticket reservation;
- handles a missing reservation with an actionable error log and typed outcome;
- marks the thread failed when present;
- calls `emit_assignment_transition` with `AssignmentState::ClaimTimedOut`;
- deduplicates the existing `(ticket, pane)` error alert;
- logs an actionable reset instruction;
- returns `AssignmentClaimTimedOut`.

It does not revoke the lease, release the slot, relaunch, or send input.

### Deadline candidate extraction

Extend `check_assignment_ack_timeouts_at` candidate matching with:

```rust
SeatAssignmentState::DeliveredAwaitingClaim {
    claim_deadline: deadline,
    ..
}
```

The existing generic `AcknowledgementInput` remains unchanged.

### Deadline action matching

Change the `Delivering` branch ordering:

1. for a live current Codex delivery, transition to passive waiting;
2. otherwise, if retry budget remains, retain the current retry behavior;
3. otherwise, retain current delivery failure behavior.

Add a `DeliveredAwaitingClaim` branch that invokes the new terminal helper with a
reason such as:

`delivered Codex assignment was not claimed before the bounded deadline`

The branch pushes the typed outcome when the helper succeeds.

### UI projection

Extend `to_ui_state` matching:

- `DeliveredAwaitingClaim` maps to
  `ui::SeatAssignmentStatus::DeliveredAwaitingClaim`;
- `ClaimTimedOut` maps to `ui::SeatAssignmentStatus::ClaimTimedOut`.

No timing logic belongs in this projection.

### Scheduler tests

Add the ticket acceptance test beside current claim/evidence and delivery timeout
regressions.

Use existing fixtures:

- `pane_name_schedule_state`;
- `exit_then_deliver_fresh_codex`;
- exact injected deadlines;
- activity log counts;
- `pending_enters` after flushing already-due launch/delivery sends where necessary;
- mixed provenance ledger reader.

The test owns the complete sequence from `Delivering` through passive waiting to
terminal claim timeout.

Add or adapt a focused test proving an exact current claim can still promote
`DeliveredAwaitingClaim` to `Owned` and suppress later timeout action.

Update historical live-Codex tests that expect the old retry/DeliveryFailed sequence.
Do not weaken Claude tests or non-live delivery-failure characterization.

## `crates/lisa-plugin/src/ui.rs`

### `SeatAssignmentStatus`

Add:

```rust
DeliveredAwaitingClaim,
ClaimTimedOut,
```

Place the passive variant with pending assignment statuses.
Place the terminal variant with failure statuses.

### Label mapping

Map to stable strings:

- `DeliveredAwaitingClaim` → `delivered-awaiting-claim`;
- `ClaimTimedOut` → `claim-timed-out`.

### Color mapping

Map passive waiting to yellow.
Map terminal timeout to red.

Existing dashboard layout and rendering functions remain unchanged.

## `crates/lisa-core/src/provenance.rs`

### `AssignmentState`

Add:

```rust
ClaimTimedOut,
```

The existing `#[serde(rename_all = "kebab-case")]` produces
`"claim-timed-out"` in append-only records and status output.

No schema version bump is needed because:

- the record shape is unchanged;
- the enum addition is a new writer value;
- current workspace readers compile against the same vocabulary;
- old rows remain unchanged and parse as before.

Add or extend serialization tests if the existing module has direct enum JSON coverage.

## Unchanged boundaries

Do not modify:

- `crates/lisa-core/src/claim.rs`;
- `crates/lisa-cli/src/claim.rs`;
- `crates/lisa-plugin/src/signal.rs`;
- `crates/lisa-plugin/src/assignment.rs`;
- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-plugin/src/deadline.rs`;
- CLI configuration or setup text;
- ticket frontmatter.

The current poll order remains claim, hook, artifact, then timeout.

## Ordering of source changes

1. Add provenance vocabulary so scheduler terminal emission compiles.
2. Add UI vocabulary so scheduler projection compiles.
3. Add scheduler state, predicates, timeout transitions, and failure helper.
4. Add/update focused tests.
5. Format and run focused suites.
6. Run complete workspace verification.
7. Commit all three exact source paths in one meaningful scheduler-state unit.

One source commit is appropriate because the private states, UI projection, and durable
terminal vocabulary form one inseparable exhaustive-match change.

## Completion invariants

After the source unit:

- live current Codex `Delivering` expiry sends zero pane input;
- state becomes `DeliveredAwaitingClaim` with a finite future deadline;
- claim, hook, and current artifact can still produce `Owned`;
- passive expiry produces only `ClaimTimedOut`;
- terminal timeout sends zero pane input and performs no retry/relaunch;
- the thread and alert are actionable and retained;
- provenance names `claim-timed-out`, never `delivery-failed`;
- late evidence cannot own the terminal seat;
- Claude retains existing delivery retry behavior;
- all exhaustive state matches compile and render.
