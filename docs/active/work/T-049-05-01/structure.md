# Structure: level-triggered block parking

## Change surface

Only `crates/lisa-plugin/src/lib.rs` requires source changes.

No new module, crate, dependency, public API, or durable schema is needed.

The implementation adds private scheduler helpers, adjusts two internal
boundaries, and extends the existing native test module.

Existing `lisa-core` disposition, ticket, provenance, DAG, and parking APIs are
reused unchanged.

## New internal value

Add a private `DurableReviewBlock` value near `ReviewBlockAction`.

It contains:

- `source_lease: AttemptLease`;
- `remedy_owner: RemedyOwner`;
- `ask: String`.

The value represents a fully correlated, parsed block candidate.

It does not represent current lease authority.

It does not retain reason, steps, or check because the parking consequence
only needs provenance policy and activity copy.

The canonical disposition remains the source for Waiting-on-you details.

## Durable attempt-root helper

Add `State::attempt_ticket_dir(&self, ticket_id: &str) -> PathBuf` beside
`attempt_work_dir`.

It selects the production `.lisa/attempts` root or the existing native-test
fallback and appends the ticket ID.

Refactor `attempt_work_dir` to build on this helper.

This keeps root selection identical for live admission and durable replay.

## Durable generation helper

Add
`State::durable_attempt_high_water(&self, ticket_id: &str) -> Option<AttemptLease>`.

Responsibilities:

- read process-local `lease_high_water` as one candidate;
- read the ticket's attempt directory;
- retain direct child directories with positive numeric names;
- require their `work` child to be a directory;
- select the maximum attempt ID;
- return a reconstructed lease for the requested ticket.

The helper performs no mutations.

Filesystem read errors are treated as absence of additional evidence.

Non-numeric, zero, file, and symlink entries are ignored.

The result is safe as predecessor history and artifact correlation only.

## Latest parking-transition helper

Add
`State::latest_parking_transitions(&self) -> HashMap<TicketId, ParkingTransitionRecord>`
near `reconcile_unpark_transitions` or the durable block helpers.

Responsibilities:

- return an empty map when the ledger path is unset or unreadable;
- parse the mixed JSONL ledger line by line;
- ignore malformed and non-parking rows;
- retain the last parking row for each ticket.

Refactor `reconcile_unpark_transitions` to use this helper.

This prevents two separate replay implementations from drifting.

Ledger ordering remains append order, matching current behavior.

## Current-generation correlation helper

Add a private method with a shape equivalent to:

```text
durable_review_block(
    ticket_id,
    latest_parking_transition,
) -> Option<DurableReviewBlock>
```

The method reads but does not mutate scheduler state.

It checks the DAG ticket's Review phase and schedulable status.

It checks for an authoritative running Review thread whose lease is exact and
current; such a thread remains owned by the live policy.

It resolves durable attempt high water.

It rejects a candidate when the latest parking transition for that ticket has
an attempt ID greater than or equal to the resolved generation.

It reads the private and canonical disposition files.

It requires byte equality.

It parses the canonical path with `parse_review_disposition`.

Only `ReviewDisposition::Block` creates `DurableReviewBlock`.

Structured and legacy blocks therefore share the existing parser boundary.

## Durable reconciliation method

Add `State::reconcile_orphaned_review_blocks(&mut self)` immediately before or
after `apply_review_block_policy`.

Method ordering:

1. build latest parking-transition map;
2. iterate DAG ticket IDs and collect durable block candidates;
3. for each candidate, resolve the ticket file;
4. write blocked status;
5. append Park provenance with the reconstructed lease;
6. release any slot/current lease;
7. remove any thread and finish-up marker;
8. log the park;
9. rebuild the DAG once if any status changed.

Candidate collection occurs before mutation to avoid borrowing the DAG while
writing state.

The method is level-triggered and returns no scheduling token.

Ticket status remains the durable DAG authority.

## Provenance emitter adjustment

Change `State::emit_review_block_transition` to take
`source_lease: &AttemptLease`.

Remove its lookup through `threads` and `current_leases`.

Keep ledger-path handling, timestamps, retry-pair fusion, record construction,
append error logging, and boolean result unchanged.

The live block-policy caller passes its already validated candidate lease.

The orphan reconciliation caller passes its durable generation lease.

All call sites remain private and explicit.

This isolates authority validation in the candidate-producing boundary rather
than the serialization boundary.

## Live policy call sites

Update both live calls in `apply_review_block_policy`.

Retry passes `&source_lease` plus existing owner, Retry type, retry pair,
recheck flag, and thread start time.

Park passes `&source_lease` plus existing owner, Park type, retry pair,
recheck flag, and park time.

Do not change candidate collection, `review_completion_inputs`, block action,
retry counters, status order, teardown, or DAG rebuild.

These constraints preserve E-048 behavior.

## Orphan park metadata

The orphan reconciler appends:

- record type Park;
- the durable source lease;
- parsed remedy owner;
- no retry pair;
- `recheck_eligible: true` only for World;
- current time as park interval start.

No retry count is invented for a session that disappeared before policy could
record a retry.

The activity log includes the parsed ask and identifies durable recovery.

## Scheduler integration

At the start of `schedule_ready_tickets`, before health/permission/pause gates
and before obtaining ready tickets, call
`reconcile_orphaned_review_blocks`.

Running before the early returns ensures a paused or seatless scheduler still
repairs durable board state.

It also makes every scheduler call site safe without duplicating guards.

When minting a lease, replace the direct process-local predecessor lookup with
`durable_attempt_high_water(&ticket_id)`.

Pass the borrowed local predecessor to `AttemptLease::mint`.

Then install the successor in existing maps unchanged.

The mint remains after all existing scheduling admission gates.

## Load integration

In `ZellijPlugin::load`, call `reconcile_orphaned_review_blocks` after the DAG
is assigned.

Place it before `reconcile_unpark_transitions` and completion reconciliation.

If it parks an orphan, its internal rebuild refreshes status before Unpark
observation.

An already reopened generation with an Unpark row is suppressed by the latest
transition map.

No live authority is reconstructed during load.

## Poll integration

In `poll_tick`, call `reconcile_orphaned_review_blocks` after artifact advance
and immediately before `apply_review_block_policy`.

An authoritative running Review thread is skipped by durable reconciliation,
then handled by the existing live policy.

An already missing or inconsistent thread is repaired during the same poll.

The later scheduling call repeats the guard intentionally.

That repetition covers threads removed by any intervening lifecycle logic and
all non-poll scheduling callers.

## Unpark refactor

Replace the inline mixed-ledger parsing in `reconcile_unpark_transitions` with
`latest_parking_transitions()`.

Keep its filtering and append semantics unchanged:

- latest type must be Park;
- ticket must be Open and not Done;
- Unpark preserves lease, owner, retry pair, recheck eligibility, and interval
  start;
- agent retry state clears even if append later fails;
- appended Unpark makes replay idempotent.

No unblock command or world-result handler changes.

## Test fixture additions

Add a helper for writing a private and canonical disposition for an explicit
attempt lease.

Add a helper that creates a Review ticket and a `State` with real temporary
ticket, work, attempt, signal, and ledger paths.

Allow the helper to seed the legacy T-046-06-03 reason.

Do not simulate parking with mocked status or ledger data; call the production
reconciliation methods.

## New tests

### Load-style orphan recovery

Construct a fresh state with empty live lease/thread maps, attempt-one private
legacy block, and matching canonical block.

Call the load-boundary reconciliation method.

Assert:

- ticket file and rebuilt DAG are Blocked;
- exactly one Park row names attempt one and Operator;
- no thread exists;
- no current lease exists;
- dashboard Waiting-on-you contains the legacy ask.

Add a source-order assertion or focused boundary test proving `load` invokes
the reconciliation method before startup completion reconciliation.

### Mid-run orphan recovery

Seed a current lease, assigned slot, and Review thread, then remove the thread
to model session teardown before policy observation.

Call durable reconciliation.

Assert blocked status, Park row, lease revocation, seat release, and no thread.

### Scheduling fence

Start with a ready open Review orphan and an idle slot.

Call `schedule_ready_tickets` directly.

Assert the guard parks the ticket and creates no thread or seat assignment.

### Reopen and fresh generation

Starting from a durable park, write Open status, rebuild, and reconcile Unpark.

Call scheduling.

Assert attempt two is minted from durable attempt-one high water and seated.

Call durable block reconciliation again.

Assert attempt two remains open/running and the old block is ignored.

### Explicit stale prior generation

Seed matching attempt-one block and create an attempt-two work directory
without a disposition.

Call durable reconciliation.

Assert status stays Open and no Park row is appended.

## Verification boundaries

Run focused orphan tests first.

Run the existing block policy and world-recheck tests next.

Run the full `lisa-plugin` test target.

Run workspace formatting, tests, clippy, and `just check` before Review.

Commit only `crates/lisa-plugin/src/lib.rs` with `lisa commit-ticket`.

Phase artifacts remain private and are published by Lisa separately.
