# Review — T-045-03-02 evidence tiers: hook and artifact

## Disposition

Pass.

The acceptance criterion is implemented and covered by focused scheduler tests.

All ticket-owned source changes are committed through Lisa's isolated
transaction.

No ticket-owned source file remains staged, modified, or untracked.

## Summary

The scheduler now represents the intended three-level ownership evidence order:

1. exact agent-issued assignment claim;
2. matching provider `UserPromptSubmit` hook;
3. admitted current-attempt private workflow artifact.

The claim path remains unchanged and is polled first.

The matching hook remains a supplemental fast path while the claim is pending.

The new behavior makes successfully admitted current-attempt workflow output a
bounded fallback that changes the actual seat state to `Owned`.

Predecessor hook and artifact evidence remain fenced from a replacement.

## Source change

Modified:

`crates/lisa-plugin/src/lib.rs`

No source files were created or deleted.

No CLI, shared claim schema, signal schema, launcher, adapter, UI type, or
configuration file changed.

## Artifact ownership admission

Added private scheduler method:

`State::admit_artifact_ownership`

It is called only after the existing artifact admission boundary has returned
success.

It requires all of the following:

- evidence ticket equals candidate lease ticket;
- candidate lease is exactly current;
- a physical slot is reserved for that ticket;
- the slot retains the exact candidate lease;
- the pane is in an active delivered pre-ownership state;
- the assignment generation equals the candidate attempt.

The existing `active_assignment_generation` helper limits admission to:

- `Delivering`;
- `AssignedPendingAck`;
- `Recovering`.

It excludes startup, shell-reset, ready, owned, and terminal failure states.

The method changes exactly one pane to `SeatAssignmentState::Owned` and returns
that pane ID.

Redundant or invalid evidence returns no transition.

## Fallback activity

Added private wrapper:

`State::record_artifact_ownership`

On the one successful transition it:

- refreshes pane activity;
- refreshes the associated thread activity;
- emits an information event naming pane, ticket, attempt, and artifact.

Rejected stale evidence does not refresh replacement liveness.

Already-owned seats do not emit duplicate fallback success events.

Unleased compatibility fixtures cannot invent a pane owner.

## Bounded artifact set

The implementation does not scan arbitrary private-directory contents.

Fallback is offered only after `check_artifact_advances` successfully admits a
recognized artifact through `State::admit_artifact`.

That boundary already requires:

- exact current lease authority;
- expected attempt-private path;
- regular file existence;
- successful read and atomic canonical publication.

Recognized fallback inputs are consequently bounded to workflow artifacts the
scheduler already requests for the current phase.

The existing Implement `progress.md` special case is included because it is a
recognized, admitted living workflow artifact.

It remains explicitly non-advancing.

`review.md` remains the Implement-to-Review phase edge.

## Poll order

Production calls remain ordered as:

```text
check_claim_signals
check_codex_ack_signals
check_artifact_advances
... later timeout policy
```

No consumer was reordered.

Comments now name the evidence roles directly.

If several forms are visible during one poll, the exact claim receives the
first opportunity to perform the pending-to-owned edge.

If no claim owns, the matching hook can accelerate.

If neither signal owns, admitted workflow output is the final positive fallback
before timeout evaluation.

## New hook regression

Added scheduler test:

`matching_hook_accelerates_pending_claim_ownership`

It uses the full scheduled Codex fixture and drives a fresh assignment to
`Delivering`.

It confirms the claim file is absent.

It publishes a matching pane-scoped `UserPromptSubmit` hook record.

It proves:

- hook evidence is consumed once;
- the pending seat becomes `Owned`;
- no claim is required for the accelerated transition;
- the acknowledgment activity event is visible.

## New stale/fallback regression

Added scheduler test:

`current_artifact_is_bounded_fallback_and_stale_evidence_is_ignored`

The test schedules a predecessor and then a monotonic replacement for the same
ticket.

It drives the replacement to `Delivering`.

It supplies a predecessor-generation hook to the replacement pane and a
predecessor `research.md` in the old attempt-private directory.

It proves:

- the stale hook is consumed but cannot own;
- stale hook evidence cannot refresh replacement activity;
- predecessor output remains private;
- predecessor output cannot publish canonically;
- predecessor output cannot advance the replacement phase;
- direct predecessor admission fails current-lease validation;
- the replacement remains pending.

It then supplies distinct `research.md` bytes in the replacement's exact
attempt-private directory.

It proves:

- current output is admitted;
- the seat becomes `Owned`;
- Research advances to Design;
- canonical bytes come from the replacement;
- predecessor bytes remain unchanged;
- fallback activity identifies the current attempt and artifact.

## Existing coverage retained

The exact claim-only regression remains green:

`delivered_assignment_becomes_owned_on_exact_claim_without_hook`

Existing exact-ack, stale-generation, revoked-authority, and duplicate-ack
coverage remains green.

Existing stale-attempt heartbeat and artifact publication coverage remains
green.

Existing split-brain, timeout, completion, Claude, and dashboard coverage
remains green.

The Implement progress regression remains green and still confirms progress
does not advance Implement.

## Verification results

Focused new tests:

- 2 passed;
- 0 failed.

Claim-filtered plugin tests:

- 8 passed;
- 0 failed.

Acknowledgment-filtered plugin tests:

- 26 passed;
- 0 failed.

Artifact-advance-filtered plugin tests:

- 9 passed;
- 0 failed.

Stale-attempt-filtered plugin tests:

- 3 passed;
- 0 failed.

Complete plugin suite:

- 393 passed;
- 0 failed;
- 0 ignored.

Complete workspace suite:

- 896 passed;
- 0 failed;
- 1 intentionally ignored real-Zellij environment test.

Repository gate:

```text
just check
```

passed both:

- WASM target check;
- workspace test suite.

Formatting and whitespace checks pass.

## Commit

Ticket-owned source was committed with `lisa commit-ticket`.

Commit:

```text
de308795c2e2af37d240e392cf8192dedaf08c2b
```

Subject:

```text
feat(plugin): tier hook and artifact ownership evidence
```

The commit contains exactly:

`crates/lisa-plugin/src/lib.rs`

Post-commit diff checks show that source path clean in both worktree and
ordinary index.

No ordinary `git add`, `git commit`, or broad staging command was used.

## Compatibility assessment

Claim nonce validation is unchanged.

Hook payload validation is unchanged.

Signal one-shot behavior is unchanged.

Artifact atomic publication is unchanged.

Lease revocation and monotonic replacement authority are unchanged.

Claude behavior is unchanged; already-owned Claude seats make the fallback a
no-op.

Timeout and retry state names are unchanged for T-045-03-03 to address.

No new dashboard label substitutes for scheduler state.

## Open concerns and limitations

No blocking issue remains.

The real-Zellij integration test remains environment-gated and was not required
by this fixture-level ticket.

T-045-03-03 still owns the delivered-awaiting-claim state, zero reinjection,
and named timeout resolution.

This ticket deliberately does not retain the winning evidence tier as a new
field after the seat reaches `Owned`; it records the fallback event and relies
on serialized poll order for rank.

That is sufficient for the current acceptance criterion and avoids broadening
the state machine immediately before the next dependent ticket.

## Handoff

The implementation is ready for Lisa's completion transaction.

Lisa should publish the private RDSPI artifacts and update ticket lifecycle
state after verifying the current attempt lease.

The agent remains on T-045-03-02 and does not start its dependent ticket.
