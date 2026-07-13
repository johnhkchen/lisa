# Design — T-045-04-02 one-authoritative-completion

## Decision summary

Strengthen the existing Codex completion-boundary regression so its predecessor
ticket reaches the boundary through the typed artifact completion transaction.

Retain the fixture's real scheduling, fresh launch, delivery, exact claim,
revocation, exit grace, late-claim rejection, and successor launch.

Add durable completion journal and provenance paths to the fixture.

Model work completion by moving the claimed thread to Review and writing the
attempt-private Review artifacts.

Let `check_artifact_advances` admit those artifacts and dispatch completion.

Repeat completion observations while the transaction is pending.

Assert that only one completion effect is launched.

Publish durable Done and deliver the successful result twice.

Assert one confirmed journal record and one authoritative Done provenance row.

Then retain the existing exit/revoke/fresh-launch assertions.

No production behavior changes are planned.

## Design goals

The test must begin with scheduler-minted authority.

It must prove the exact nonce claim owns the pane before work completion.

It must enter the sole typed completion gateway.

It must observe duplicate suppression before result publication.

It must observe duplicate suppression after result publication.

It must inspect durable records, not only in-memory thread state.

It must cross the Codex exit/revoke boundary.

It must show that a dependent ticket cannot occupy the exiting pane.

It must show a fresh successor TUI after shell readiness.

It must leave provider and lease production code unchanged.

## Option 1 — add only assertions to hostile-order replay coverage

The hostile-order integration already runs the real `complete_ticket`
transaction in a temporary Git repository.

It could add assertions for `WaitingForExit`, revoked authority, and fresh
successor launch.

Advantages:

- uses a real isolated completion commit;
- already asserts a single repository commit;
- already asserts one journal confirmation and provenance record;
- already exercises duplicate sources and duplicate result delivery.

Disadvantages:

- its primary authority is manually installed;
- it does not pass through assignment delivery and exact claim admission;
- its spare pane can schedule the dependent immediately;
- process-boundary assertions would not prove reuse of the completed pane;
- expanding it would blur restart/replay concerns with claim lifecycle concerns.

This option does not satisfy the requested claim→work→boundary continuity by
itself.

## Option 2 — create a new full temporary-Git boundary fixture

A new fixture could initialize a nested Git repository, schedule a Codex ticket,
claim it, admit Review, invoke `complete_ticket`, deliver the result, elapse exit
grace, and schedule a dependent.

Advantages:

- every host and scheduler layer is exercised in one test;
- repository commit count can be asserted directly;
- claim and exit behavior are connected to the real CLI transaction.

Disadvantages:

- duplicates the nested repository helpers in hostile-order coverage;
- duplicates most of the existing Codex boundary fixture;
- significantly increases test setup relative to the missing assertion;
- native host calls still remain stubs, so “more integrated” is bounded;
- creates another large fixture that can drift from adjacent lifecycle tests.

This is viable but carries more maintenance cost than the acceptance criterion
requires.

## Option 3 — strengthen the existing Codex boundary fixture

The existing fixture already owns the exact claim and physical pane lifecycle.

Replace its direct completion simulation with the normal plugin completion path.

Enable the journal and provenance ledger in the same temporary directory.

Write Review artifacts under the exact attempt work directory.

Run artifact advancement to create one pending completion.

Repeat artifact and reconciliation observations to prove no second effect.

Write Done to model the successful host transaction.

Deliver the matching result twice.

Read the journal and ledger to prove a single authoritative publication.

Advantages:

- closes exactly the seam identified in Research;
- retains the strongest existing claim and pane-boundary setup;
- exercises production dispatch and result publication;
- counts both the completion effect and durable completion records;
- keeps one-pane exit grace and fresh successor proof deterministic;
- requires no production change;
- produces a focused failure if the boundary ever double-completes.

Disadvantages:

- the host CLI transaction itself is represented by Done bytes plus a successful
  callback rather than executed inside this test;
- the existing test becomes broader;
- the test remains in the large `lib.rs` native test module.

The real CLI transaction and same-key commit idempotence are already covered by
hostile-order tests.

This option joins those guarantees without duplicating their repository setup.

## Option 4 — add a new production completion-boundary latch

Production state could retain a separate “completion published” set or a
completion counter across release.

Advantages:

- creates an explicit in-memory guard at the boundary;
- could make duplicate callbacks visibly rejected rather than ignored.

Disadvantages:

- duplicates `pending_completions`, durable aggregate state, Done masking, and
  absence-of-pending callback suppression;
- introduces another state restoration concern;
- an in-memory latch is weaker than the existing durable journal;
- no failing behavior demonstrates a production gap;
- changes runtime semantics beyond this test-focused acceptance criterion.

This option is rejected because the current architecture already provides the
authoritative guards.

## Chosen option

Choose Option 3.

The repository already has a single typed completion gateway, a single effect
executor, a durable journal aggregate, current-lease checks, and one result
publisher.

The new clean-exit helper is downstream of journal confirmation and provenance.

The risk introduced by the helper is integration ordering, not missing runtime
state.

A combined regression is the most direct proof.

## Detailed fixture sequence

Create the predecessor and dependent Codex ticket files as today.

Configure one slot, one maximum thread, zero cooldown, and deterministic short
assignment grace.

Add attempt, signal, journal, and provenance paths inside the temp directory.

Mark the completion journal healthy because this native fixture bypasses
`State::load`.

Schedule the predecessor.

Advance startup grace into delivery.

Construct the exact claim from the scheduler's assignment reference.

Admit the claim and verify `Owned`.

Update the ticket phase to Review.

Refresh the fixture DAG.

Set the running thread's phase to Review.

Write `review.md` and passing `review-disposition.json` in the attempt-private
work directory.

Call `check_artifact_advances`.

The first pass admits the Review artifact and dispatches one completion.

Assert the pending source is `Artifact` and its authority is the claimed lease.

Assert `launched_completion_effects` contains exactly one launch.

Call artifact advancement again.

Call reconciliation with the same current lease.

Both observations must return without another effect.

Inspect the journal before success.

It must contain one requested and one command-in-flight record.

Update the predecessor ticket to durable Done.

Deliver a valid successful callback using the pending generation.

The result handler must confirm, emit provenance, revoke, exit, remove the
thread, and schedule.

Deliver the same callback again.

The second delivery must find no pending entry and do nothing.

## Single-completion assertions

`launched_completion_effects.len()` must remain one.

The journal must contain exactly three lines.

It must contain one requested state.

It must contain one command-in-flight state.

It must contain one confirmed state.

The confirmed aggregate must retain the expected commit ID.

The provenance ledger must contain exactly one execution record.

That record must name the predecessor ticket.

Its outcome must be Done.

It must be authoritative.

Its attempt lease must equal the claimed lease.

No second result delivery may alter journal or ledger bytes.

## Boundary assertions

The current predecessor lease must be absent after confirmation.

The high-water record must still contain the predecessor lease.

The slot must contain no predecessor ticket or attempt lease.

The seat assignment must be absent.

The slot must be `WaitingForExit` and have no live session.

The lifecycle trace must contain one lease revoke, one slot release, and one
clean-exit request in order.

The exact predecessor claim must be rejected after release.

The successor must not be minted while exit is pending.

After exit grace, the slot must represent an empty shell.

The next scheduling pass must mint and launch the successor.

The successor assignment path and nonce must differ from the predecessor.

The predecessor claim must remain rejected after successor launch.

## Claude and lease preservation

The change will not call or alter the Claude adapter.

The completion helper's Codex predicate remains unchanged.

The change will not alter `AttemptLease`, `current_leases`, `lease_high_water`,
claim admission, or revocation.

Focused Claude and lease tests will run after implementation.

The complete workspace suite will provide the final regression check.

## Failure interpretation

A second launched effect indicates duplicate completion command injection.

A second confirmed line indicates duplicate durable completion publication.

A second authoritative provenance row indicates duplicate scheduler completion.

A remaining current lease indicates revocation ordering regressed.

An early successor lease indicates exit grace no longer fences pane reuse.

A reused assignment identity indicates fresh-launch isolation regressed.

These assertions make the ticket's guarantee reviewable from one test failure.
