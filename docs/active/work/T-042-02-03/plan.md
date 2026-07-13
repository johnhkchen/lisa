# Plan: bounded reconciliation replay convergence

## Preconditions

1. Preserve existing Lisa-owned modifications to `.lisa/provenance.jsonl` and
   active ticket files.

2. Preserve the unrelated untracked `crates/lisa-plugin/docs/` tree.

3. Write all phase artifacts only to this attempt's private work directory.

4. Use no ordinary-index staging or commit commands for ticket source.

5. Run source commits only through `lisa commit-ticket` with exact includes.

## Step 1: add durable completion deadline vocabulary

Modify `crates/lisa-core/src/completion.rs`.

Add `CompletionDeadline` as an opaque epoch-millisecond newtype.

Implement raw construction/access and inclusive expiration comparison.

Extend CommandInFlight state and CommandLaunched event with the deadline.

Update reducer forwarding, unexpected-event reconstruction, and pattern names.

Replace the single in-flight reconciliation decision with before-deadline replay
and deadline-exceeded decisions.

Add current time to the pure reconcile signature.

Update the in-module deterministic unit tests.

Verification:

- deadline before now is expired;
- deadline equal to now is expired;
- deadline after now is not expired;
- Requested plus CommandLaunched preserves correlation and deadline;
- reconciliation returns replay only before the deadline;
- reconciliation returns deadline exceeded at and after it;
- correlation mismatch behavior remains unchanged.

## Step 2: update external core regressions

Modify `crates/lisa-core/tests/completion_state_machine.rs`.

Give its generated harness a fixed future deadline and fixed current time.

Update all reconciliation and reducer calls.

Modify `crates/lisa-core/tests/recorded_livelock_regression.rs` similarly.

Keep each prior regression's model and assertions otherwise unchanged.

Verification:

```bash
cargo test -p lisa-core
```

The property state machine must still converge with no more than one live
effect.

The recorded artifact-before-phase trace must still confirm once without a
finish-up prompt or re-request.

## Step 3: persist the bounded deadline

Modify `crates/lisa-plugin/src/completion_journal.rs`.

Add deadline to the typed CommandInFlight transition.

Write it into every new JSON record.

Read the field optionally and map missing legacy values to deadline zero.

Fold the deadline through the core reducer.

Make action-required rejection retain Done masking.

Update all journal tests and helpers.

Add explicit compatibility and masking assertions.

Verification:

- new JSON contains `reconciliation_deadline_unix_ms`;
- load returns the exact original deadline;
- legacy CommandInFlight without the field loads as deadline zero;
- action-required rejection masks Done;
- retryable rejection does not mask Done;
- torn, malformed, and unknown-version behavior remains unchanged.

Run:

```bash
cargo test -p lisa-plugin completion_journal
```

## Step 4: add deterministic plugin time boundary

Modify `crates/lisa-plugin/src/lib.rs`.

Add the named 60-second completion reconciliation timeout.

Add `SystemTime` to `CompletionDeadline` conversion and saturating deadline
calculation.

Add explicit-time internal dispatch/executor forms while retaining production
wrappers that use `SystemTime::now()`.

Compute one deadline for each initial CommandInFlight transition.

Store deadline and replay-origin flag in `PendingCompletion`.

Update every existing test pending literal.

Verification:

- initial journal aggregate contains the expected explicit deadline;
- production callers require no call-site clock plumbing;
- no sleep or timer-event approximation is needed in unit tests.

## Step 5: implement same-key in-flight replay

In the Reconcile branch, pass explicit current time to core reconciliation.

Map the before-deadline replay decision to a new replay adapter method.

Validate current lease, durable generation key, exact correlation, exact
deadline, ticket path, and absence of live pending state.

Build the command from the journal key.

Install pending replay state without appending duplicate Requested or
CommandInFlight journal records.

Launch through the existing host command boundary.

Treat duplicate observations during a live replay as no-ops.

Verification:

- one reconstructed in-flight aggregate creates one replay pending entry;
- a duplicate Stopped input and repeated Reconcile do not create another;
- replay key equals the original generation key byte-for-byte;
- journal line count does not grow at replay launch.

## Step 6: implement bounded terminal transition

Map the deadline-exceeded decision to a focused timeout method.

Revalidate exact aggregate identity.

Append Rejected with the same key, matching correlation, timeout reason, and
ActionRequired retryability.

Remove pending only after the append succeeds.

Rebuild the DAG and log the existing typed rejection.

Change replay result failure handling to retain CommandInFlight and its original
deadline rather than create a fresh retry window.

Keep initial command failure retryable.

Verification:

- exact-deadline reconciliation appends action-required Rejected;
- repeated later reconciliation launches nothing;
- repeated replay failures cannot alter or extend the stored deadline;
- a failed journal append leaves in-flight/pending fencing intact;
- late results after terminal rejection are ignored;
- uncertain Done bytes remain masked.

## Step 7: add real Git lost-result convergence regression

Create a temporary initialized Git repository inside the plugin test.

Write a Review ticket and passing Review artifacts.

Configure the plugin State with real ticket/work paths, journal, ledger, current
lease, thread, and slot.

Dispatch initial completion at a fixed time.

Execute the adapter's durable completion key through the CLI library.

Withhold that successful result from the plugin to simulate loss.

Count commits and assert Done exists in the one new completion commit.

Construct a fresh State, restore the journal, and reinstall current lease/thread
fixtures.

Present duplicate Stopped and regular Reconcile observations before deadline.

Assert exactly one replay pending entry and no duplicate journal intent.

Execute the same CompleteTicketRequest again.

Assert the CLI returns the original commit and commit count remains unchanged.

Deliver that result to the restarted plugin.

Assert one Confirmed record, Confirmed aggregate, no pending state, one
authoritative Done provenance record, and released thread/slot.

Verification target:

```bash
cargo test -p lisa-plugin lost_result
```

## Step 8: add timeout/duplicate-stop regression

Reuse a deterministic in-flight fixture.

Reconcile at the stored absolute deadline.

Assert named action-required Rejected state and no pending replay.

Present a later Reconcile, duplicate Stopped, and another timeout observation.

Assert effect counts and journal counts remain unchanged.

If durable ticket bytes are Done, rebuild and assert they remain masked to the
prior Review state.

Verification target:

```bash
cargo test -p lisa-plugin reconciliation_deadline
```

## Step 9: format and focused verification

Run:

```bash
cargo fmt --all -- --check
cargo test -p lisa-core
cargo test -p lisa-plugin
```

If formatting check fails because of ticket changes, run `cargo fmt --all` and
inspect the exact diff before continuing.

Check no unexpected files changed.

## Step 10: commit core unit

Use:

```bash
lisa commit-ticket \
  --ticket-id T-042-02-03 \
  --message "feat(core): bound completion reconciliation" \
  --include crates/lisa-core/src/completion.rs \
  --include crates/lisa-core/tests/completion_state_machine.rs \
  --include crates/lisa-core/tests/recorded_livelock_regression.rs
```

Confirm those paths are clean afterward.

## Step 11: commit plugin unit

Use:

```bash
lisa commit-ticket \
  --ticket-id T-042-02-03 \
  --message "fix(plugin): converge bounded completion replay" \
  --include crates/lisa-plugin/src/completion_journal.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Confirm those paths are clean afterward.

If implementation mechanics require both units to compile atomically, document
the deviation in progress and use one exact-include commit containing all five
paths rather than leaving an intermediate broken tree.

## Step 12: full verification

Run:

```bash
cargo test --workspace
just check
```

Inspect failures for concurrent/unrelated work before changing any path outside
ticket ownership.

Run targeted acceptance assertions again if the broad suite changes timing or
fixture state.

## Step 13: ownership audit

Run `git status --short`.

Confirm the five ticket-owned source/test paths are neither modified, staged,
nor untracked.

Confirm only Lisa-owned ticket/provenance changes, attempt artifacts, and the
pre-existing unrelated plugin docs tree remain.

Inspect recent commits and source diff against the pre-ticket HEAD.

## Step 14: progress and review handoff

Write `progress.md` in the private attempt directory with completed work,
verification commands, commit IDs, and deviations.

Write `review.md` with source summary, acceptance evidence, test coverage,
limitations, and ownership audit.

Write exactly:

```json
{"disposition":"pass","reason":null}
```

when all acceptance behavior and tests pass.

Use block with a non-empty actionable reason if a critical gap remains.

Do not update ticket phase/status or publish artifacts manually.

Stop on this ticket after Review and wait for Lisa's completion commit.
