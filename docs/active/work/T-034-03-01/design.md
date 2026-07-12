# Design: T-034-03-01 deterministic split-brain regression

## Goal

Add one deterministic regression that composes the complete T-031-02 field
timeline using the scheduler's real state transitions.

The test must prove:

- the predecessor lease is revoked and its pane fenced before redispatch;
- the replacement receives a strictly newer lease on another pane;
- a missed Codex acknowledgement leaves the replacement unowned;
- resumed predecessor signals cannot affect replacement state;
- predecessor artifacts cannot be attributed to the replacement;
- stale completion cannot enter the isolated completion transaction;
- only the current replacement can publish authoritative Done provenance.

The production scheduler is unchanged.

## Option 1: extend several narrow existing tests

The timeout, stale-heartbeat/artifact, acknowledgement, and provenance tests
could each gain one or two assertions.

### Benefits

- Small changes per test.
- Failures point to a narrow subsystem.
- Little additional fixture code.

### Costs

- Does not prove ordering across subsystem boundaries.
- Can pass even if the real timeout path and replacement path do not compose.
- Does not reproduce the field sequence in one state machine.
- Leaves the exact ticket acceptance criterion implicit across test names.

### Decision

Rejected as the primary solution.

Existing narrow tests remain useful unit diagnostics, but the ticket explicitly
asks for a deterministic reproduction of the timeline rather than a checklist
of disconnected facts.

## Option 2: add an external shell harness

Extend the T-031-03 provider-contract harness or add a sibling shell script that
launches Lisa processes and manipulates fixture repositories.

### Benefits

- Exercises process and Git boundaries.
- Produces retained filesystem evidence.
- Resembles the later live proof.

### Costs

- The plugin scheduler runs as WASM under Zellij, so a shell harness cannot
  deterministically mutate its internal clocks or inspect lease maps.
- Forcing timeout and missed pane injection would require terminal orchestration,
  timing, and a live client.
- Such a harness would be slower and potentially flaky.
- T-034-03-02 explicitly owns the fresh-loop live proof.

### Decision

Rejected for this ticket.

The existing external harness remains adjacent evidence for the durable Git
contract, while this ticket supplies the missing scheduler-state regression.

## Option 3: add a second simulation model

Create a dedicated test-only struct representing old attempt, fence,
replacement, signals, artifacts, and provenance.

### Benefits

- Compact scenario language.
- Exact virtual time.
- Easy to enumerate theoretical states.

### Costs

- Tests the model rather than Lisa.
- Lease checks removed from production would not necessarily fail the model.
- Duplicates scheduler semantics and can drift.
- Violates the acceptance requirement that the test fail when a lease check is
  removed.

### Decision

Rejected.

The regression must execute production methods directly.

## Option 4: one composed native plugin test

Add a single test in `crates/lisa-plugin/src/lib.rs` using the existing private
test helpers and real scheduler methods.

### Benefits

- Calls production timeout, fencing, scheduling, signal, artifact, completion,
  and provenance code.
- Injects timestamps without sleeping.
- Requires no Zellij host or provider credentials.
- Can inspect test-only lifecycle ordering.
- Fails locally when any of the exercised lease checks is removed.
- Runs automatically under `cargo test --workspace`.

### Costs

- A long test has more fixture setup than the narrow unit tests.
- Assertions must be grouped clearly so a failure still identifies the boundary.
- Host-side pane writes are intentionally no-ops in native tests, so the test
  proves scheduler intent rather than actual terminal delivery.

### Decision

Chosen.

It matches the ticket boundary and composes the already-shipped mechanisms
without inventing another layer.

## Fixture design

Use a temporary project with one open Review-phase Codex ticket.

Review is selected because it allows the final leg to cross the real
commit-gated completion boundary without needing to advance every workflow
phase.

Create two physical slots:

- pane 1: the slow predecessor, assigned and Owned;
- pane 2: an idle resident Codex session eligible for reuse.

The second slot must be quiet and out of cooldown so scheduler selection is
deterministic.

Set `max_threads = 1`, `session_timeout_secs = 1`,
`stuck_threshold_secs = 1`, and `wind_down_secs = 0`.

Install the predecessor lease through `install_current_attempt`.

Set predecessor `started_at` and `last_activity` sufficiently in the past.

## Timeline design

### Leg A: slow old attempt

The predecessor thread is Running, Codex-routed, Review-phase, and Owned.

Write a predecessor `review.md` sentinel into its private attempt staging
directory, but do not admit it before timeout.

This models useful late work that exists outside scheduler observation.

### Leg B: timeout

Call `check_session_timeouts`.

Assert the lifecycle vector contains, in order:

1. `LeaseRevoked`;
2. `PaneFenced`;
3. `SlotReleased`.

Assert no current lease, no thread, and no assignment remain for pane 1.

Assert pane 1 remains Fenced and the timeout provenance row is non-authoritative.

### Leg C: replacement with missed injection

Call `schedule_ready_tickets`.

The ready ticket must land on pane 2 because pane 1 is Fenced.

Assert the successor attempt ID is predecessor plus one.

Assert the slot, thread, and current registry all carry the successor lease.

Because pane 2 is a resident Codex seat, assert it is
`AssignedPendingAck` and not Owned.

Do not submit the successor acknowledgement.

That deliberate omission is the missed-injection condition.

No recovery timeout is required for this ticket: the field split-brain risk is
present during the pending window, and E-033 recovery already has dedicated
coverage.

### Leg D: old pane resumes

Emit predecessor evidence from pane 1:

- stale heartbeat with predecessor lease;
- stale ack carrying predecessor ticket/generation;
- idle signal;
- stopped signal;
- cleared signal;
- error signal;
- predecessor private `review.md` bytes;
- direct stale completion request.

Run the corresponding production consumers.

Do not include `.awaiting`: Codex's native signal vocabulary does not emit it,
and it is a question-UI injection guard rather than ownership or completion
evidence.

Assert all signal files are consumed.

Snapshot replacement state before replay and prove after replay:

- its thread and current lease still exist;
- its liveness clocks did not move;
- it remains pending and unowned;
- no error alert is attributed to it;
- no pending completion exists;
- the canonical review artifact is absent;
- the predecessor bytes remain only in predecessor staging.

This demonstrates no cross-pane attribution.

### Leg E: one winner

Submit a matching successor ack and assert the only Owned seat is pane 2.

Write distinct replacement review bytes into successor staging.

Run artifact advancement and assert canonical bytes equal replacement bytes.

This should create pending completion with successor authority.

Simulate the already-tested isolated command success by marking the fixture
ticket Done and calling `handle_completion_result` with a valid 40-hex commit
ID.

Call the result handler twice to exercise duplicate callback suppression.

Assert:

- no active thread or slot ownership remains;
- exactly one authoritative Done record exists;
- the only Done record carries the successor lease;
- the predecessor timeout row remains fenced and non-authoritative;
- there are two total ledger rows: history plus the single winner.

## Lease-check mutation sensitivity

The composed test crosses multiple independent checks.

Removing the timeout revocation/fence ordering fails lifecycle and pane
eligibility assertions.

Removing ack generation validation lets the predecessor promote the wrong
assignment.

Removing heartbeat lease validation moves replacement clocks.

Removing artifact lease validation publishes predecessor bytes or advances
before the successor writes.

Removing completion admission validation creates stale pending completion.

Removing provenance lease validation permits another authoritative Done row or
misattributes its lease.

The test therefore guards the fence-and-reject boundary rather than merely
asserting final Done.

## Determinism

All state is temporary and local to one test.

No fixed filesystem path is shared with parallel tests.

No sleep or external command is used.

The only current-time comparisons use timestamps deliberately far beyond the
configured one-second thresholds.

Pane selection is deterministic because pane 1 is Fenced and pane 2 is the only
eligible Idle slot.

## Verification

Run the focused test first.

Then run the full plugin suite and workspace suite.

Run formatting, plugin Clippy with warnings denied, the WASM target check, and
diff whitespace checks.

Commit only `crates/lisa-plugin/src/lib.rs` through Lisa's isolated transaction.

Workflow artifacts remain for Lisa to publish with the ticket completion.

## Final decision

Implement one named, narrative native plugin test that drives real production
boundaries from timeout through authoritative completion.

Keep the test self-contained, use existing fixture helpers, and make every
safety assertion explicit enough to identify which lease boundary regressed.
