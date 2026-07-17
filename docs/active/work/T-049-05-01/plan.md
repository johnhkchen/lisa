# Plan: level-triggered block parking

## Implementation strategy

Implement the change as one scheduler-owned source unit in
`crates/lisa-plugin/src/lib.rs`.

Keep each internal step independently testable, but commit the coordinated
behavior only after all focused tests pass because generation discovery,
parking replay, and scheduling admission form one invariant.

Do not modify `lisa-core`; its existing types and parsers are sufficient.

Do not touch unrelated Lisa-managed ticket, journal, provenance, or work files.

## Step 1: establish the durable candidate value

Add `DurableReviewBlock` near the existing Review block policy types.

Fields:

- exact reconstructed `source_lease`;
- parsed `remedy_owner`;
- owned `ask` for activity logging.

Derive only the traits needed by collection and tests.

Verification:

- native compilation succeeds;
- no public API changes;
- no new warning for unused fields.

## Step 2: centralize attempt-root construction

Add `attempt_ticket_dir(ticket_id)` beside `attempt_work_dir`.

Use the configured attempt root when present.

Use the current `work_dir/.attempts` fallback otherwise.

Refactor `attempt_work_dir` through the ticket-level helper.

Verification:

- existing artifact publication and assignment-path tests retain exact paths;
- production and native fallback roots are unchanged.

## Step 3: recover durable attempt high water

Implement `durable_attempt_high_water(ticket_id)`.

Start with the in-memory high-water attempt ID when present.

Read direct children of the ticket attempt directory.

For each entry:

1. require a real directory entry;
2. parse its filename as `u64`;
3. reject zero;
4. require `<entry>/work` to be a directory;
5. retain the maximum.

Return an `AttemptLease` for the supplied ticket ID.

Do not install it in current authority.

Add a focused unit test if the later scheduling fixture does not fully pin
non-numeric and newest-directory behavior.

Verification:

- no directory returns process-local high water or None;
- a newer durable directory wins;
- a newer process-local value wins when its directory is not yet visible;
- malformed directory names do not affect the result.

## Step 4: factor latest parking replay

Extract current ledger parsing from `reconcile_unpark_transitions` into
`latest_parking_transitions`.

Preserve append-order last-row-wins behavior.

Return empty for unset or unreadable ledgers.

Ignore malformed and non-parking rows exactly as before.

Refactor Unpark reconciliation to consume the helper.

Verification:

- existing Unpark idempotency tests pass unchanged;
- world recheck tests retain their exact Park/Unpark row counts;
- mixed provenance rows remain accepted.

## Step 5: identify a current-generation durable block

Implement `durable_review_block` as a read-only predicate.

Check the ticket in the DAG:

- phase equals Review;
- status equals Open or InProgress.

Check live policy ownership:

- a Running Review thread;
- matching ticket lease;
- exact current lease.

Return None when that authoritative live combination exists.

Resolve durable attempt high water.

Compare the latest parking transition for the ticket:

- same/newer attempt means consumed or stale, return None;
- older/no transition permits further checks.

Read private and canonical disposition bytes.

Require exact equality.

Parse canonical disposition.

Map only Block into `DurableReviewBlock`.

Verification fixtures:

- structured block matches;
- legacy block matches and becomes Operator;
- missing canonical/private file does not match;
- mismatched bytes do not match;
- invalid/pass does not match;
- newer attempt directory makes old canonical block stale;
- same-attempt Retry/Park/Unpark consumes the candidate.

Prefer behavioral tests over one test per internal branch unless failure
diagnostics need finer isolation.

## Step 6: generalize block provenance serialization

Add `source_lease: &AttemptLease` to
`emit_review_block_transition`.

Remove its thread/current-lease lookup and warning branch.

Clone the supplied lease into `ParkingTransitionRecord`.

Update the live Retry caller to pass its validated source lease.

Update the live Park caller likewise.

Keep retry-pair fusion and all other arguments unchanged.

Verification:

- existing live block tests append the same number and shapes of rows;
- live attempt IDs remain `[1, 2, 3]` in the agent retry test;
- no behavior change occurs before the new durable reconciler is called.

## Step 7: implement orphan reconciliation

Implement `reconcile_orphaned_review_blocks`.

Build the latest transition map once.

Collect candidate `(ticket_id, DurableReviewBlock)` pairs from the DAG.

For each pair:

1. resolve a nonempty ticket file path;
2. write Blocked status;
3. append a Park row using the candidate lease;
4. set World recheck eligibility only for World owner;
5. pass no retry pair;
6. release the slot/current lease;
7. remove thread state;
8. clear finish-up state;
9. log a durable-recovery activity entry.

Track whether any status was written.

Rebuild the DAG once at the end when needed.

Do not roll status back if provenance append fails; preserve existing durable
authority ordering.

Verification:

- fresh no-thread fixture becomes Blocked;
- one Park row names the recovered lease and owner;
- assigned orphan fixture releases its slot and current lease;
- repeat invocation adds no row and makes no new change;
- Waiting-on-you sees an operator/world block through the existing projection.

## Step 8: add load and poll observation boundaries

In `load`, call orphan reconciliation after the initial DAG assignment and
before Unpark and completion reconciliation.

In `poll_tick`, call orphan reconciliation after artifact advances and before
the live policy.

Add a source-boundary regression assertion if direct `load` cannot be safely
invoked by native tests because of Zellij host calls.

Verification:

- the load fixture calls the same production helper on a fresh State;
- production source order pins load repair before completion reconciliation;
- poll source order pins durable repair before live block policy;
- authoritative live attempts remain handled only by live policy.

## Step 9: fence every scheduling pass

At the first line of `schedule_ready_tickets`, invoke orphan reconciliation.

Place it before permission, slot, pause, and journal early returns.

This ensures repair is not conditional on seat availability.

Obtain ready tickets only after reconciliation returns.

Verification:

- direct scheduling of a ready orphan writes Blocked;
- it creates no thread;
- it creates no current lease;
- it leaves the idle slot free;
- it appends one Park row.

## Step 10: mint from durable high water

At the existing late mint point, resolve a local durable predecessor.

Pass `predecessor.as_ref()` to `AttemptLease::mint`.

Keep mint timing after all admission gates.

Keep installation in `lease_high_water` and `current_leases` unchanged.

Verification:

- ordinary first-time tickets still mint attempt one;
- a restarted state with durable attempt one mints attempt two;
- an in-memory higher generation remains monotonic;
- no current lease is reconstructed before dispatch.

## Step 11: build the field regression fixture

Add a helper creating:

- temporary ticket directory;
- open Review ticket;
- canonical work directory;
- attempt root with explicit generation;
- signal and ledger paths;
- real DAG;
- enabled scheduling flags and an idle slot when requested.

Use the preserved legacy reason from T-046-06-03:

`The Codex closing leg measured 225 MiB ... before Review can pass.`

The exact full text should exercise legacy fallback rather than a structured
owner/ask document.

Write identical private and canonical bytes.

Verification:

- parser classifies Operator;
- ask equals the legacy reason;
- no structured fixture accidentally bypasses fallback behavior.

## Step 12: test load-style orphan parking

Construct the field fixture with no live thread, current lease, or high-water
map entry.

Invoke the durable reconciler at the same state boundary used by load.

Assert:

- disk ticket status is Blocked;
- rebuilt DAG status is Blocked;
- no thread and no current lease exist;
- exactly one Park row exists;
- row attempt ID is one and owner is Operator;
- plugin UI has one Waiting-on-you item with the legacy ask.

Invoke reconciliation again and assert one row remains.

## Step 13: test mid-run orphan cleanup

Construct a structured World or Operator block at generation one.

Install generation one in both high-water and current maps.

Assign an agent slot and seat state.

Create then remove the running Review thread to model the reported timing.

Invoke durable reconciliation.

Assert:

- status becomes Blocked;
- Park provenance exists;
- current lease is revoked;
- slot ticket and attempt lease are cleared;
- seat assignment is removed;
- no replacement thread appears.

## Step 14: test scheduling, unpark, and stale generation

First, call `schedule_ready_tickets` on the open orphan fixture.

Assert it parks before seating.

Next, reopen the ticket with the ticket helper.

Rebuild the DAG and run existing Unpark reconciliation.

Call scheduling.

Assert:

- one Unpark follows one Park;
- status remains Open;
- a running Review thread exists;
- its lease is attempt two;
- the idle seat is assigned to attempt two.

Call durable reconciliation once more.

Assert attempt two remains current and running.

Also create an explicit attempt-two work directory without a disposition in a
fresh fixture whose attempt-one block remains canonical.

Assert no Park occurs and status remains Open.

## Step 15: focused verification

Run formatting:

```sh
cargo fmt --all -- --check
```

If formatting is required, run `cargo fmt --all`, inspect the exact diff, and
ensure no unrelated file changed.

Run new tests by their common name filter.

Run existing live policy regressions:

```sh
cargo test -p lisa-plugin review_block -- --nocapture
cargo test -p lisa-plugin park_instead_of_churn -- --nocapture
cargo test -p lisa-plugin agent_owned_block -- --nocapture
cargo test -p lisa-plugin world_recheck -- --nocapture
```

Run the full plugin suite:

```sh
cargo test -p lisa-plugin --no-fail-fast
```

Review failures for interaction with concurrent ticket work before editing any
path outside this ticket's ownership.

## Step 16: record progress and commit

Write `progress.md` in the private attempt directory before committing.

Record:

- implemented helpers and boundaries;
- test commands and outcomes;
- deviations from this plan;
- exact owned source path;
- unrelated dirty files intentionally preserved.

Inspect:

```sh
git diff -- crates/lisa-plugin/src/lib.rs
git status --short
```

Commit only the plugin source through Lisa:

```sh
lisa commit-ticket \
  --ticket-id T-049-05-01 \
  --message "fix(plugin): reconcile orphaned review blocks" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use the ordinary index.

Verify the source file is clean afterward and unrelated files remain as they
were.

## Step 17: full repository verification

Run:

```sh
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
just check
```

If any command overlaps with concurrent work, distinguish ticket-owned failure
from unrelated dirty-state failure and document it honestly.

Do not broaden the commit unless this ticket actually owns the needed fix.

## Step 18: Review artifacts

Inspect the committed diff and test results.

Write `review.md` in the private attempt directory.

Cover:

- durable generation discovery;
- canonical/private correlation;
- provenance consumption rule;
- load, poll, and scheduling boundaries;
- unpark/fresh-attempt behavior;
- live policy non-regression;
- files changed;
- verification evidence;
- limitations and failure semantics.

Write exact passing disposition only if all required behavior and verification
are complete:

```json
{"disposition":"pass","reason":null}
```

Remain on T-049-05-01 afterward.
