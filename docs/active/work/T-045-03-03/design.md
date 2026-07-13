# Design — T-045-03-03 delivered awaiting claim

## Decision summary

Replace the first expired `Delivering` deadline for a live Codex TUI with a
state transition to `DeliveredAwaitingClaim`.

The transition arms one new finite deadline and performs no pane write.

During that passive window, the existing exact claim, supplemental matching hook, and
current-attempt artifact paths can still promote the seat to `Owned`.

If the passive deadline expires, transition to the retained terminal state
`ClaimTimedOut`.

`ClaimTimedOut` marks the thread failed, emits its own pre-ownership provenance state,
raises the existing operator alert, and directs the operator to reset the ticket.

Claude and non-live delivery paths retain the existing bounded delivery retry and
`DeliveryFailed` behavior.

## Goals

The design must provide:

- a real scheduler-owned delivered-awaiting-claim state;
- no duplicate prompt send on the affected live Codex timeout edge;
- a bounded claim wait using scheduler time;
- acceptance of all current-attempt ownership evidence while waiting;
- a distinct named terminal result;
- operator-visible recovery instructions;
- deterministic native tests;
- no behavioral change to Claude's assignment handshake.

## Non-goals

This ticket does not:

- change the native launcher argv;
- change assignment-file or claim schemas;
- add a new signal;
- prove a real Codex or Zellij session;
- change ticket-boundary exit or lease revocation;
- change artifact admission priority;
- make dashboard text authoritative;
- broadly rewrite startup or recovery states;
- add automatic retries after the claim wait.

## Option 1 — keep retrying and rename the dashboard

The smallest visual change would leave `Delivering` and its retry untouched and display
“delivered-awaiting-claim” after the first deadline.

This is rejected.

The story explicitly requires a scheduler transition, not a dashboard label.
The retry would still invoke `send_line_to_pane`, producing the duplicate prompt the
ticket exists to remove.
The terminal result would remain false `DeliveryFailed` provenance.

## Option 2 — remove every delivery retry globally

The scheduler could change all expired `Delivering` seats directly into one passive
claim wait.

This is structurally simple because `Delivering` already contains a generation and
deadline.

It is rejected as too broad.

Fresh Claude seats also pass through `Delivering` after a positive SessionStart signal.
Claude does not use the Codex claim-first contract and the epic explicitly preserves
Claude's mechanism.

Changing every provider would silently redefine delivery reliability outside this
ticket's boundary.

## Option 3 — treat the first deadline as terminal claim timeout

The live Codex branch could skip the retry and immediately enter `ClaimTimedOut` when
the current acknowledgement deadline expires.

This would prevent duplicate injection and create an honest terminal name.

It is rejected because it does not create the required intermediate
delivered-awaiting-claim state.
It also gives a slow first turn no bounded fallback window after hook evidence is known
to be absent.

## Option 4 — passive state followed by named timeout

The chosen option introduces two private state variants:

```text
Delivering
  -- first live-Codex deadline, no evidence --> DeliveredAwaitingClaim
  -- claim/hook/artifact --------------------> Owned

DeliveredAwaitingClaim
  -- claim/hook/artifact --------------------> Owned
  -- passive deadline -----------------------> ClaimTimedOut
```

The first transition performs no transport action.

The passive deadline reuses `assignment_ack_deadline(now)`.
That preserves the configured finite window and the existing overflow fallback.
It also replaces the old retry's time budget with waiting, rather than silently adding
an unbounded or unrelated duration.

This option is chosen because it maps directly to the ticket vocabulary, isolates the
Codex behavior, and preserves the existing evidence and deadline machinery.

## Live Codex predicate

The passive transition is allowed only when the scheduler still has a live Codex seat.

The predicate should require:

- a slot with the addressed pane ID;
- `has_session == true`;
- `last_client == Some(AgentClient::Codex)`;
- a retained ticket reservation;
- a slot attempt lease whose attempt ID matches the `Delivering` generation;
- exact currency in `current_leases`.

This uses scheduler-owned lifecycle and lease truth.
It does not infer life from missing errors or fabricate a provider hook.

If the predicate fails, the old delivery retry/failure path remains available.
That makes transport failure behavior conservative and avoids labeling a missing or
stale session as safely awaiting a claim.

## Passive deadline

`DeliveredAwaitingClaim` contains:

- `generation: u64`;
- `claim_deadline: SystemTime`.

At the first expired live-Codex `Delivering` deadline, the scheduler computes the new
deadline from the supplied deterministic `now`.

No retry counter is needed because the state has no send operation.

The state is added to the existing acknowledgement candidate extraction.
`DeadlineEvaluator` needs no modification because it is generic over copied state and
absolute time.

## Ownership evidence while waiting

`active_assignment_generation` must return the generation carried by
`DeliveredAwaitingClaim`.

That single integration point keeps all predecessor evidence behavior:

1. an exact claim can establish ownership;
2. a matching hook can accelerate ownership;
3. a current-attempt artifact can establish bounded fallback ownership;
4. stale attempts remain rejected by existing lease checks.

Evidence consumers continue to run before timeout evaluation in `poll_tick`.
If evidence wins at the boundary, the seat becomes `Owned` and the timeout's exact-state
recheck suppresses terminal mutation.

`ClaimTimedOut` is intentionally excluded from the active-generation helper.
A late claim, hook, or artifact cannot resurrect a terminal attempt.

## Terminal state and operator action

`ClaimTimedOut` is a retained assignment state with no deadline.

The failure helper follows the shape of `fail_assignment_delivery`:

- accept only `DeliveredAwaitingClaim`;
- insert `ClaimTimedOut` first as the exact-once guard;
- resolve the retained ticket;
- mark its thread failed;
- append `AssignmentState::ClaimTimedOut` provenance;
- add one existing error alert;
- log that the delivered assignment was not claimed;
- tell the operator to reset the ticket after inspecting the pane;
- return `FailureTransitionOutcome::AssignmentClaimTimedOut`.

The reservation and attempt authority remain retained, matching existing pre-ownership
terminal states.
There is no silent redispatch.

The reason string should describe missing ownership evidence, not failed delivery.

## UI projection

The UI gains:

- `SeatAssignmentStatus::DeliveredAwaitingClaim`, yellow;
- `SeatAssignmentStatus::ClaimTimedOut`, red.

Their labels are exactly:

- `delivered-awaiting-claim`;
- `claim-timed-out`.

The mapping is a projection of private scheduler state.
No UI timer or absence heuristic is introduced.

## Provenance vocabulary

`lisa_core::provenance::AssignmentState` gains `ClaimTimedOut`.
Serde's existing kebab-case policy serializes it as `claim-timed-out`.

This makes CLI/status readers distinguish:

- transport delivery failure;
- passive ownership claim timeout;
- recovery failure;
- startup failure.

The existing untagged mixed-ledger representation remains compatible because the row
shape is unchanged and the enum addition is additive for new writers/readers.

## Test design

Add one explicit acceptance regression around the real scheduler methods.

The fixture should:

1. schedule a Codex ticket into a live slot;
2. advance exit/startup grace into `Delivering`;
3. record the number of delivery logs and queued pane sends;
4. provide no claim and no hook;
5. expire the old acknowledgement deadline;
6. assert `DeliveredAwaitingClaim` with the same generation;
7. assert no new delivery log, no retry state, and no queued duplicate Enter;
8. expire the passive claim deadline;
9. assert `ClaimTimedOut`, a failed retained thread, an alert, and actionable log;
10. assert no `DeliveryFailed` state or delivery-failure provenance;
11. assert later timeout checks are inert.

Also protect evidence admission by showing a valid claim can own from the new state,
or by extending the active-generation coverage.

Existing Claude retry coverage must remain green.
Historical Codex retry tests should be updated only where their expected behavior is
the exact live-Codex branch intentionally replaced by this ticket.

## Tradeoffs

The design adds two enum variants across scheduler, UI, and provenance layers.
That is more explicit code than overloading `Delivering` with a retry sentinel.

The explicit states are worth the cost because they make mutation authority,
operator display, deterministic tests, and durable history agree.

The design still relies on scheduler slot lifecycle as the fixture-level live-TUI
premise.
Real process truth remains deferred to the field-test story as required.

## Final decision

Implement the live-Codex-only passive transition and terminal claim timeout.

Do not send any pane input on either the transition into passive waiting or the
terminal timeout.

Keep all current-attempt evidence valid while waiting, retain the failed reservation
for explicit reset, and preserve Claude's existing delivery retry path.
