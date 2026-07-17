# Design: level-triggered block parking

## Decision summary

Add a durable Review-block reconciliation path beside the existing live
Review-block policy.

The live path remains responsible for bounded agent retries and for normal
current-thread artifact admission.

The durable path is responsible for an already-admitted block whose writing
thread is no longer authoritative.

It identifies the current durable generation from numbered attempt
directories, correlates that private disposition with the canonical copy, and
uses parking provenance to decide whether that exact generation was already
consumed.

It runs during plugin load, on every poll, and defensively at the scheduling
entry point.

The scheduling path also uses durable attempt high water when minting a new
lease, so reopening generation N produces generation N+1 after a restart.

No new durable file format or core schema is introduced.

## Goals

- Convert every orphaned current-generation Review block into blocked status.
- Append a Park provenance row naming the producing generation.
- Release any still-held seat and revoke any surviving current lease.
- Remove residual thread state for the parked ticket.
- Rebuild the DAG so Waiting-on-you and readiness see the new status.
- Prevent scheduling from seating a verdict that should be parked.
- Let an explicitly reopened ticket create a fresh Review attempt.
- Ignore canonical blocks left behind by a prior generation.
- Preserve live-thread retry behavior and existing E-048 tests.
- Accept structured blocks and the parser's legacy operator fallback.

## Non-goals

- Do not change pass completion or completion journal behavior.
- Do not change the block JSON schema.
- Do not change Waiting-on-you copy or layout.
- Do not change `lisa unblock` or world-recheck commands.
- Do not reconstruct live lease authority on plugin startup.
- Do not retain parked threads.
- Do not alter the two-retry policy for a live agent-owned block.

## Options considered

### Option A: parse every canonical block without generation evidence

This would be the smallest implementation.

It would correctly catch the original orphan on first observation.

It would also immediately re-park a ticket after an operator reopened it,
because unpark intentionally leaves the old canonical disposition in place.

It cannot distinguish a verdict from attempt N after attempt N+1 begins.

This option fails both fresh re-review and stale-generation acceptance.

### Option B: add a canonical sidecar naming the publishing lease

Artifact admission could publish a new source-lease marker beside the canonical
disposition.

That gives explicit generation metadata for future admissions.

The original incident predates the marker, so startup still needs a fallback.

Publishing artifact and sidecar as separate renames also introduces a
two-file crash interval that needs another recovery rule.

It expands a narrow fix into a durable artifact-protocol migration.

This option is unnecessary because private attempt copies already persist.

### Option C: correlate canonical bytes with the newest private attempt

Artifact admission copies the private bytes to the canonical path.

The newest numbered attempt directory is durable generation high water.

If its private disposition exists and exactly matches canonical bytes, the
canonical verdict belongs to that generation.

If a newer attempt lacks a disposition, the old canonical verdict is stale.

If an operator replaced canonical content, the old private block no longer
claims it.

Existing parking provenance can mark a generation consumed without adding a
new file.

This option uses current storage invariants and recovers pre-fix incidents.

Option C is selected.

## Durable generation discovery

Add a helper that computes high water for one ticket.

It considers the process-local `lease_high_water` entry when present.

It scans `<attempt_dir>/<ticket>/` for positive numeric directory names whose
`work` child is a directory.

When `attempt_dir` is unset in native fixtures, it uses the same fallback root
as `attempt_work_dir`.

The maximum attempt ID becomes a reconstructed `AttemptLease` value.

This value is evidence and minting history, not current authority.

It is never inserted into `current_leases` merely because it exists on disk.

At scheduling time, it is used as the predecessor passed to
`AttemptLease::mint`.

The newly minted successor is then installed in both existing maps exactly as
today.

## Current-generation block predicate

For a ticket to present a durable current-generation block:

1. its durable ticket phase is Review;
2. its status remains schedulable, Open or InProgress;
3. it does not have an authoritative running Review thread holding the exact
   current lease;
4. durable generation high water exists;
5. that generation's private `review-disposition.json` is a regular file;
6. canonical `review-disposition.json` is a regular file;
7. the two files have identical bytes;
8. parsing the canonical file returns `ReviewDisposition::Block`;
9. no latest parking transition at the same or newer attempt generation has
   already consumed it.

The authoritative-live-thread exclusion preserves the existing policy's
requirement that a live block also have an admitted `review.md` and preserves
bounded agent retries.

A thread with missing or inconsistent lease evidence is not authoritative and
does not suppress durable recovery.

Pass, Invalid, missing, mismatched, and stale dispositions produce no action.

## Parking-transition consumption

Read the mixed provenance ledger once per reconciliation pass.

Retain the latest `ParkingTransitionRecord` for each ticket, matching the
existing Unpark replay convention.

Any Retry, Park, or Unpark row for the same attempt ID consumes that
generation's block.

A row for a newer attempt also proves an older disposition is stale.

A row for an older attempt does not suppress a new generation's block.

Retry consumption is essential for live agent behavior.

After a live attempt writes Retry and removes its thread, the scheduling guard
must not reinterpret the same bytes as an orphan that should park immediately.

Park consumption gives idempotence if reconciliation is repeated before a DAG
refresh is visible.

Unpark consumption permits the reopened ticket to reach scheduling while its
old canonical disposition remains on disk.

Once scheduling creates the next attempt directory, newest-generation lookup
also makes the prior disposition stale structurally.

## Orphan parking consequence

Collect candidates before mutating state.

For each candidate, resolve its ticket file from the DAG.

Write `TicketStatus::Blocked` with the existing ticket update helper.

Only after that durable scheduling authority succeeds, append a Park row.

Generalize the provenance emitter to receive an explicit `AttemptLease`
instead of looking it up from a live thread.

The existing live caller supplies its already-validated source lease.

The orphan caller supplies the reconstructed durable lease.

Operator and Agent orphan parks are not world-recheck eligible.

World orphan parks are world-recheck eligible.

An orphan park carries no fabricated retry count; live exhausted-agent parks
retain their existing `2/2` metadata.

Release the ticket's slot through `release_slot_for_ticket`.

That revokes a surviving current lease even when no matching slot remains.

Remove thread and finish-up state defensively.

After all successful parks, rebuild the DAG once.

## Observation boundaries

During `load`, invoke durable block reconciliation after the initial DAG is
stored and paths are configured.

Run it before ordinary scheduling can occur through permission or pane events.

During `poll_tick`, invoke it as a distinct level-triggered observation after
artifact signal handling and before the live policy.

The live path skips authoritative threads in the durable predicate and then
continues unchanged.

At the top of `schedule_ready_tickets`, invoke durable reconciliation again.

This is a safety gate covering every scheduling caller rather than relying on
each call site's ordering.

The guard runs before `get_ready_tickets`, so a successful status write and DAG
rebuild remove the parked ticket from that same pass.

Repeated invocations are safe because status and generation-consumption checks
make the operation idempotent.

## Unpark sequence

An operator, `lisa unblock`, or a world check writes status Open.

Existing `reconcile_unpark_transitions` sees the latest Park and appends an
Unpark row for its attempt.

The scheduling guard sees the old canonical block but also sees that Unpark,
so it does not park the old generation again.

Scheduling uses durable attempt high water as predecessor and creates N+1.

That new attempt has no private block disposition yet.

Subsequent polls therefore ignore the canonical N block as stale.

If N+1 later publishes a Block, normal live policy or orphan reconciliation
handles N+1 independently.

## Failure behavior

Unreadable attempt directories or disposition files fail closed to no parking;
they do not guess generation ownership.

Malformed disposition content follows the existing Invalid parser result.

Ticket status write failure logs an error and leaves the ticket available for
the next reconciliation attempt.

Provenance append remains best effort after status, matching existing parking
semantics.

Slot release and thread cleanup still occur after a successful status write.

No failure path manufactures current lease authority from disk.

## Test design

Add a real-path orphan fixture with an open Review ticket, numbered private
attempt, matching canonical block, and empty live maps.

Use the preserved legacy field reason to prove fallback parsing.

Exercise the load reconciliation helper and assert blocked status, one Park
row, no thread, and one Waiting-on-you entry.

Create a mid-run variant retaining a slot and current lease but removing the
thread; assert reconciliation revokes and releases them.

Call scheduling directly on an orphan fixture; assert it parks without minting
or seating a reviewer.

Reopen a parked fixture, reconcile Unpark, and schedule; assert attempt N+1 is
seated and status remains open.

Call durable reconciliation again after that fresh attempt exists; assert the
prior block does not park it.

Add an explicit newer-directory fixture where only the prior attempt has the
matching block; assert no parking.

Retain and rerun all existing live block-policy tests, especially the E-048
two-seat and agent retry sequences.
