# Review: level-triggered block parking

## Outcome

T-049-05-01 is implemented and ready to complete.

Orphaned Review blocks are now level-triggered durable obligations rather than
events that require a surviving thread.

A current-generation canonical Block is reconciled into blocked ticket status,
Park provenance, released lease/seat state, and the existing Waiting-on-you
projection.

The reconciliation runs during plugin load, every poll, and at every scheduling
entry point.

Unpark still restores ordinary Review scheduling.

Stale prior-generation dispositions cannot park a newer attempt.

## Committed source

One source file changed:

- `crates/lisa-plugin/src/lib.rs`.

The exact source unit was committed through Lisa's isolated ticket transaction.

Commit:

`5b423801dbbdc873b87d062e42cbc59715d0a38e`

Subject:

`fix(plugin): reconcile orphaned review blocks`

No ordinary-index staging or ordinary Git commit was used.

## Durable attempt history

The plugin now has one helper for selecting the private attempt root.

Production continues to use `.lisa/attempts`.

Direct native fixtures retain the prior work-directory fallback.

Durable attempt high water scans a ticket's direct numeric generation
directories.

Only positive generations with a real `work` directory qualify.

Malformed names, files, symlinks, zero, and unreadable roots cannot manufacture
history.

This durable high water is generation evidence, not live execution authority.

The helper does not populate `current_leases`.

Immediately before ordinary dispatch, scheduling merges durable high water
with process-local high water.

The existing mint operation then creates the checked successor.

After a plugin restart, attempt N is therefore followed by N+1 rather than a
new attempt one that collides with old artifacts.

## Current-generation verdict correlation

Canonical `review-disposition.json` has no generation field.

The existing admission protocol copies exact bytes from a private attempt into
the canonical work directory.

The reconciler uses that invariant directly.

It resolves the current live lease when one remains after thread loss.

Otherwise it resolves the newest durable attempt generation.

It reads that generation's private disposition and the canonical disposition.

The two byte sequences must match exactly.

It then parses the canonical path through
`parse_review_disposition`.

This supports structured Block documents and the existing legacy
operator/unstructured fallback without duplicate parsing logic.

Missing, unreadable, different, Pass, and Invalid documents do not park.

The equality requirement is an important safety property.

An old private Block cannot overwrite a canonical disposition that an operator
has amended.

If a newer attempt directory exists without a verdict, the old canonical Block
does not correlate with that newer generation.

## Transition consumption

Latest parking-transition replay is now a shared helper.

Both orphan reconciliation and existing Unpark reconciliation consume it.

The helper reads the mixed provenance ledger and retains the last parking row
per ticket.

A Retry, Park, or Unpark row at the candidate generation or a newer generation
means the old Block has already been consumed or superseded.

This prevents repeated Park rows.

It also preserves bounded live Agent retry behavior.

After a live block produces Retry and removes its thread, the scheduling guard
does not reinterpret the same verdict as an orphan requiring immediate Park.

After a Park is reopened, its Unpark row permits scheduling despite the old
canonical document remaining on disk.

## Orphan parking consequence

`reconcile_orphaned_review_blocks` examines Review-phase tickets that retain a
schedulable status.

It skips a running Review thread only when that thread holds an exact current
lease.

A missing, stopped, or inconsistent thread does not suppress recovery.

For each correlated unconsumed Block, the method:

1. writes `status: blocked` through the existing ticket helper;
2. appends a Park transition attributed to the producing attempt;
3. preserves World recheck eligibility for World-owned remedies;
4. releases any matching slot;
5. revokes any surviving current lease through the shared release boundary;
6. removes residual thread and finish-up state;
7. logs the recovered ask;
8. rebuilds the DAG once after all successful parks.

Blocked status remains the scheduling authority.

No separate parked registry or retained Parked thread was added.

The existing dashboard and CLI Waiting-on-you projection immediately see the
blocked ticket plus canonical disposition.

## Provenance boundary

The existing live block emitter still validates a running thread and exact
current lease.

Serialization is now delegated to a private append helper receiving an
explicit attempt lease.

The orphan path can therefore attribute its Park row to durable generation
evidence without pretending that lease is live authority.

The append helper derives ticket identity from the lease, avoiding a redundant
argument and satisfying workspace Clippy policy.

Status is still written before best-effort provenance, matching existing
parking authority ordering.

An orphan Agent park does not invent two retries that were never observed.

If process-local retry progress exists, the row may retain the honestly known
bounded progress.

## Observation boundaries

Plugin load calls orphan reconciliation after initial ticket/DAG discovery.

It runs before permission or pane events can schedule a reviewer.

`poll_tick` calls it every cadence in addition to the existing live block
policy.

`schedule_ready_tickets` calls it before journal, permission, slot, or pause
early returns.

This scheduling guard is deliberately redundant with load and poll.

Completion handlers, permission events, pane discovery, world-result handling,
and keep-working can all reach scheduling outside one specific poll ordering.

They now share one durable admission fence.

The ready-ticket list is collected only after reconciliation, so a newly
blocked ticket is excluded from the same pass.

## Unpark and re-review

No unblock command or world-check behavior changed.

When durable status returns to Open, existing Unpark reconciliation appends an
Unpark row carrying the Park generation and interval.

The scheduling guard sees that old generation as consumed.

The scheduler then mints the next attempt from durable high water.

The fresh attempt proceeds through the normal Review prompt, thread, lease, and
seat lifecycle.

Once its attempt directory exists, a retained predecessor disposition is stale
by construction.

A new Block from the fresh generation remains eligible for ordinary live or
orphan handling.

## Acceptance coverage

### Load with no live thread

`orphaned_legacy_block_parks_at_load_boundary_without_spawning` uses the exact
T-046-06-03 incident reason in legacy block shape.

It begins with empty thread/current-lease maps and matching generation-one
private/canonical bytes.

It verifies:

- ticket file and DAG status become Blocked;
- one Operator Park row names generation one;
- no Review thread is spawned;
- no current lease exists;
- the slot remains free;
- Waiting-on-you contains the legacy fallback ask;
- repeated reconciliation remains exact-once.

### Mid-run thread loss

`orphaned_block_appearing_after_thread_loss_parks_and_releases_seat` begins with
a current attempt, admitted canonical Block, assigned slot, and owned seat.

It removes only the thread before reconciliation.

It verifies Park provenance, blocked status, lease revocation, slot release,
seat-assignment removal, and absence of a replacement thread.

### Scheduling fence and unpark

`scheduling_parks_durable_block_then_unpark_seats_fresh_generation` calls the
scheduler directly against an orphaned Block.

It verifies the ticket parks before any reviewer is seated.

It then writes Open status, reconciles Unpark, and schedules normally.

The new thread carries attempt two and the ticket stays Open.

The ledger contains exactly Park then Unpark.

### Stale prior generation

`stale_prior_generation_disposition_does_not_park_fresh_attempt` retains a
generation-one canonical/private Block and creates a newer durable attempt
without a verdict.

It verifies no blocked status and no Park row.

Subsequent scheduling advances beyond durable high water and remains runnable.

## Non-regression coverage

The existing live-thread E-048 tests were not rewritten.

They continue to prove:

- Operator and World blocks park immediately;
- two seats are released for ready work;
- World parks retain recheck eligibility;
- Agent blocks retry exactly twice, then park;
- live transition attempt IDs remain monotonic;
- status-open Unpark clears retry state and seats the next attempt;
- the dashboard reads the canonical operator ask;
- automatic world checks retain exact Park/Unpark behavior.

Pass completion and completion journal tests also remain green.

## Verification evidence

- Focused orphan tests: passed.
- Focused scheduling/unpark test: passed.
- Focused stale-generation test: passed.
- `cargo test -p lisa-plugin --no-fail-fast`: 427 passed.
- `cargo test --workspace --no-fail-fast`: all unit, integration, and doc tests
  passed; one real-Zellij environment test remained intentionally ignored.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all`: passed.
- `git diff --check`: passed.
- `just check` after commit: WASM check and complete workspace tests passed.

## Repository hygiene

The committed plugin source is clean after commit.

The ordinary index has no staged files.

Remaining modified/untracked paths are Lisa-managed journal, provenance,
ticket, and shared phase-publication state.

They were not included in the ticket-owned source commit.

Generation one briefly left same-ticket uncommitted work while generation two
was active.

That older process exited without committing.

Generation two audited the source, corrected canonical correlation and edge
coverage, verified it, and committed it under the current lease.

## Open concerns and limitations

No acceptance blocker remains.

The native load fixture exercises the same post-DAG method ordering rather than
invoking Zellij's host-bound `load` directly.

Production source contains the call at the required load boundary, and the
WASM target compiles.

Reconciliation reads the attempt tree and provenance ledger at level-triggered
boundaries.

That is linear in durable attempts/ledger rows and consistent with existing
Unpark replay; no performance issue appeared in the test suite.

Provenance append remains best effort after blocked status, as it was before
this ticket.

If the ledger is unwritable, scheduling safety still comes from durable blocked
status, while provenance failure remains visible in activity logging.

No TODO, follow-up source edit, or operator action is required for completion.
