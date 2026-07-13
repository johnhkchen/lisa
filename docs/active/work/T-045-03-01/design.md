# Design — T-045-03-01 claim is ownership proof

## Decision target

The scheduler must consume the CLI's typed claim evidence and perform the final
authority checks that the producer cannot perform.
A current delivered assignment must become `Owned` from that valid claim without a
`UserPromptSubmit` hook file.
Delivery and ownership must remain separately visible before and after admission.

The change must preserve the existing lease fence and exact assignment nonce.
It must not introduce T-045-03-02's evidence ranking or T-045-03-03's new waiting
and timeout states.

## Authority model

The claim is a self-assertion until the scheduler admits it.
Admission is an intersection of six independently retained facts:

1. the pane encoded by the strict signal filename;
2. the ticket reserved in that physical slot;
3. the slot's complete attempt lease;
4. the authoritative `current_leases` entry;
5. the exact assignment reference retained after publication;
6. the ticket, attempt, and nonce asserted in the claim body.

No single durable file replaces the in-memory authority registry.
No filename scan selects the newest nonce.
The transition occurs only when all six facts identify the same assignment.

## Option 1 — trust every typed claim

The consumer could deserialize `AssignmentClaim` and insert `Owned` for the pane.

Advantages:

- very small implementation;
- uses the shared payload type;
- proves the happy path.

Disadvantages:

- a stale signal can own a successor attempt;
- a pane can claim another slot's ticket;
- an old same-attempt assignment file can supply the wrong nonce;
- revocation in `current_leases` has no effect;
- it discards the explicit producer/consumer authority boundary.

Decision: rejected because typed shape is not authority.

## Option 2 — compare only the current lease

The consumer could construct an `AttemptLease` from the claim and require it to be
current in `current_leases`.

Advantages:

- rejects revoked and stale generations;
- follows the E-034 registry;
- remains provider-neutral.

Disadvantages:

- does not bind the claim to the addressed pane's slot;
- does not distinguish two immutable assignment files from one attempt;
- ignores the retained nonce-bearing reference introduced for this boundary;
- a routing mismatch can affect another physical seat.

Decision: rejected because lease identity alone is coarser than assignment identity.

## Option 3 — compare the retained assignment only

The consumer could look up `assignment_refs[claim.ticket_id]` and compare attempt and
nonce.

Advantages:

- distinguishes the exact immutable assignment;
- rejects stale same-attempt file residue;
- uses the scheduler's live reference rather than directory contents.

Disadvantages:

- a retained entry may outlive current authorization during teardown;
- it does not prove the signal arrived through the assigned pane;
- it omits the established slot and current-lease fence.

Decision: rejected as incomplete authority admission.

## Option 4 — add a synchronous plugin RPC

The CLI could send a request directly to the plugin and receive an admission result.

Advantages:

- immediate response to the agent;
- no polling delay;
- no one-shot file lifecycle.

Disadvantages:

- the CLI contract already publishes atomic filesystem evidence;
- Lisa has no matching RPC response protocol for this command;
- correlation, timeout, and Zellij coupling would expand the ticket;
- fixture tests would need a new transport seam;
- it duplicates work deliberately completed in T-045-01-02.

Decision: rejected because the established signal boundary is sufficient.

## Option 5 — strict signal ingestion plus full scheduler admission

Add a `Claims` signal request and a typed `AssignmentClaim` record.
The consumer deletes recognized claim files once, then passes the pane ID and claim
to a focused scheduler method.
That method validates state, slot, lease, and assignment reference before inserting
`Owned`.

Advantages:

- completes the intended producer/consumer protocol;
- keeps filesystem normalization separate from scheduler policy;
- rejects stale ticket, attempt, pane, and nonce evidence;
- uses the same one-shot convention as other native signals;
- needs no live provider or transport process;
- supports claim-only ownership in the same poll after delivery.

Costs:

- adds one signal family and poll operation;
- requires source-order characterization updates;
- the current timeout policy remains hook-era behavior until later story tickets.

Decision: selected.

## Claim ingestion contract

Extend `SignalRequest` with `Claims`.
Extend `SignalRecord` with:

```text
Claim { pane_id, claim: AssignmentClaim }
```

Recognition uses exact `pane-<u32>.claim` filenames.
The body is parsed as the shared `lisa_core::claim::AssignmentClaim`.
The recognized path is removed whether parsing succeeds or fails.
This keeps malformed or rejected claims one-shot and avoids repeated dashboard work.

The ingestion layer validates only:

- filename structure;
- pane ID numeric shape;
- JSON/schema shape.

It does not inspect slots, states, current leases, or retained references.

## Scheduler admission contract

Add a focused method taking `pane_id` and `&AssignmentClaim`.
It returns `true` only when it performs the pending-to-owned transition.

Admission order:

1. reject an already-owned seat;
2. require an active unowned assignment generation for the pane;
3. require the addressed slot and its ticket and attempt lease;
4. require claim ticket equals slot ticket;
5. require claim attempt equals the state generation;
6. require slot lease equals claim ticket/attempt;
7. require slot lease is current in `current_leases`;
8. require a retained assignment reference for the ticket;
9. require retained lease equals the slot/current lease;
10. require retained nonce equals the claim nonce;
11. insert `SeatAssignmentState::Owned`.

The method does not inspect the assignment file again.
Successful publication is already represented by the retained `AssignmentRef`.
Reading the file would introduce a second, weaker authority source and a new race.

## Eligible predecessor states

Reuse `active_assignment_generation` for this ticket.
It already identifies the unowned states in which exact assignment evidence can
arrive:

- `Delivering`;
- `AssignedPendingAck`;
- `Recovering`.

The acceptance path uses `Delivering`.
Accepting the same exact identity in the other existing acknowledgement-gated states
keeps the authority rule consistent across fresh, reused, and recovery assignments.

Claims do not promote:

- `Starting`;
- `ReadyForAssignment`;
- reset states;
- terminal failures;
- unassigned seats;
- already-owned seats.

This prevents a claim from proving receipt before scheduler delivery has occurred.

## Consumer side effects

On successful admission:

- insert `Owned`;
- bump pane and thread activity clocks;
- log an information event naming the pane and claim.

Rejected claims remain silent at the activity level, matching stale hook evidence.
The signal is still consumed once.
No retry or terminal state is added here.

The log should say the pane claimed its assignment rather than acknowledged it.
This makes the evidence source distinguishable in operator diagnostics.

## Poll order

Run `check_claim_signals` immediately after `check_shell_ready_signals` and before
the existing hook acknowledgement consumer.

Rationale:

- `deliver_ready_assignments` already ran, so a claim for a just-delivered seat can
  own it in the same poll;
- process-start and shell-reset admission remain earlier lifecycle prerequisites;
- claim authority is evaluated before legacy supplemental hook evidence;
- both evidence consumers still run before timeout evaluation;
- artifact and idle phase processing sees the admitted ownership state.

Update both source-order characterization tests to pin the new call.

## Hook behavior in this slice

Do not redesign or remove the existing hook consumer in T-045-03-01.
The acceptance criterion is specifically claim-only with the hook absent.
T-045-03-02 owns ranking a matching hook as supplemental fast evidence and adding
artifact fallback, and it edits the same serialized state-machine path next.

This ticket establishes that hook evidence is no longer required for ownership.
It does not prematurely define the final relationship among all evidence tiers.
Changing the hook semantics here would overlap the dependent ticket and force broad
rewrites of existing timeout tests before the replacement tier model exists.

## Cleanup behavior

Add `claim` to `clear_pane_lifecycle_signals`.
Although authoritative admission rejects stale evidence, best-effort cleanup should
remove unconsumed predecessor claim files alongside lease, ack, heartbeat, and
transition files when a pane lifecycle is reset.

No assignment file deletion or nonce revocation is added.
Those operations have separate ticket ownership later in E-045.

## Acceptance test design

Use the existing higher-level scheduler fixture.
Drive a fresh assignment through:

1. scheduling and exact process start;
2. `ReadyForAssignment` observation;
3. delivery to `Delivering`;
4. dashboard assertion containing `delivering` and not `owned`;
5. write exact `AssignmentClaim` to `pane-10.claim`;
6. run only the claim consumer, with no `.ack` file;
7. assert `Owned` and dashboard `owned`.

The fixture obtains attempt and nonce from the scheduler's current lease and retained
assignment reference rather than inventing a parallel identity.

Add focused negative checks for a wrong nonce before the exact claim.
The rejected file must be consumed, state must remain `Delivering`, and activity
must not be bumped.
The exact claim then proves the one-shot rejection did not poison later admission.

## Ingestion regression coverage

Extend signal tests to cover:

- typed claim record production;
- malformed claim body consumption;
- strict filename policy;
- legacy ticket-name non-recognition;
- poll ordering.

The shared core module already covers JSON round trips including large `u128` nonces.
No duplicate serialization unit test is needed in the plugin.

## Verification strategy

Run, in increasing breadth:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin signal::tests
cargo test -p lisa-plugin signal_consumer_characterization
cargo test -p lisa-plugin signal_ingestion_regression
cargo test -p lisa-plugin claim
cargo test -p lisa-plugin
cargo test --workspace
just check
```

The exact test filter may match more tests; the package and workspace runs remain
the authoritative regressions.

## Rejected scope expansions

- no new `DeliveredAwaitingClaim` variant;
- no timeout or reinjection policy change;
- no artifact scanning or fallback evidence;
- no final hook evidence hierarchy;
- no launcher argv changes;
- no assignment text changes;
- no CLI claim validation changes;
- no dashboard label additions;
- no live Codex/Zellij field test;
- no ticket phase/status edits.

## Final decision

Implement a typed one-shot claim consumer and a full scheduler admission method that
matches pane, state generation, slot lease, current lease, retained assignment
lease, and nonce.
Place it after delivery/lifecycle consumers and before timeout evaluation.
Prove the visible `delivering` → `owned` transition with no hook signal.
