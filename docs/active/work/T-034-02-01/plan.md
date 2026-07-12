# T-034-02-01 Plan — bind Codex ack to lease

## Objective

Make the Codex acknowledgement marker and promotion decision use the ticket's
exact current `AttemptLease`.

A prior-attempt payload must never promote a replacement seat.

Preserve the existing bounded recovery by making its fresh process a successor
lease attempt.

## Preconditions

- `AttemptLease` and its exact `is_current` helper exist in `lisa-core`.
- Dispatch stamps one lease across current authority, slot, and thread.
- Release revokes current authority while retaining high-water history.
- Hard timeout fences the old pane before redispatch.
- E-033 pending, owned, recovering, and recovery-failed states are present.
- `LISA_ASSIGNMENT` already transports ticket plus numeric generation.

## Step 1 — remove the independent generation source

Modify `State` in `crates/lisa-plugin/src/lib.rs`.

- remove `next_assignment_generation`;
- remove `allocate_assignment_generation`;
- update assignment-state comments to identify generation as the attempt ID;
- retain `active_assignment_generation` for state projection.

Verification:

- `rg next_assignment_generation` returns no source matches;
- no saturating acknowledgement counter remains;
- `State::default` still compiles.

## Step 2 — source dispatch markers from the minted lease

In `schedule_ready_tickets`:

- delete pre-mint generation allocation;
- keep the awaiting-human gate ahead of minting;
- keep lease mint and map installation ahead of provider side effects;
- derive reused-Codex generation from `attempt_lease.attempt_id`;
- pass it through the existing `SpawnContext`;
- retain immediate ownership for fresh Codex and Claude assignments.

Verification:

- scheduler tests observe pending generation equal to the slot/current lease;
- fresh Codex remains `Owned`;
- reused Claude remains `Owned`;
- adapter marker strings contain the lease attempt ID.

## Step 3 — gate promotion on exact current authority

Refactor `acknowledge_codex_assignment`.

- call `seat_is_owned` to reject duplicate ownership first;
- require pending/recovering generation;
- read ticket reservation and lease stamp from one slot;
- reject ticket/lease inconsistency;
- reject generation/attempt inconsistency;
- validate the stamped lease with `current_leases` and `is_current`;
- build the detector reference from the validated lease;
- mutate only after detector success.

Remove `#[allow(dead_code)]` from `seat_is_owned`.

Verification:

- absent current lease rejects;
- stale stamped lease rejects;
- stale marker rejects;
- exact marker promotes;
- duplicate exact marker is inert.

## Step 4 — make fresh recovery a successor lease

Refactor `begin_assignment_recovery`.

- resolve the assigned slot and ticket;
- mint from retained high-water history;
- fail actionably without provider input on missing reservation or mint error;
- install the successor as high-water and current;
- stamp the slot and logical thread with the same successor;
- enter `Recovering` with the successor attempt ID;
- preserve the existing `/exit`, transition timer, state cleanup, and log;
- preserve the single recovery-attempt limit.

Verification:

- original lease stops validating at recovery start;
- recovery lease is strictly newer;
- current map, high-water map, slot, thread, and recovery state agree;
- old acknowledgement is rejected;
- recovery acknowledgement is accepted;
- only one fresh fallback launch occurs;
- second acknowledgement timeout ends in `RecoveryFailed`.

## Step 5 — update direct scanner coverage

Update `test_codex_ack_signal_promotes_matching_pending_seat`.

- construct one `AttemptLease`;
- stamp the slot;
- install it as current/high-water where appropriate;
- set pending generation from its attempt ID;
- write the matching ack file;
- run the real scanner;
- assert file removal, promotion, activity bump, and one log.

Add a stale-current case if it can share the fixture without obscuring the
acceptance test.

## Step 6 — add the replacement-generation acceptance regression

Use a real scheduler fixture with a reused Codex seat.

- dispatch attempt 1;
- capture its lease and assert pending/unowned;
- release the first assignment and remove its thread;
- restore deterministic slot eligibility while retaining a resident Codex
  session;
- redispatch the same ticket;
- capture attempt 2 and assert it is current and strictly newer;
- submit attempt-1 acknowledgement to the replacement pane;
- assert false, pending state unchanged, and not owned;
- submit attempt-2 acknowledgement;
- assert true and `Owned`;
- resubmit attempt 2 and assert no second transition.

This test is the direct acceptance criterion.

## Step 7 — reconcile recovery and harness tests

Update assertions that encode the old process-global counter.

- obtain expected values from installed leases;
- assert recovery changes current lease;
- retain old-generation fencing assertions;
- retain dashboard state snapshots;
- retain consecutive-reuse outcome counts;
- retain Claude parity assertions.

Do not weaken behavioral assertions merely to accommodate the new identity
source.

## Step 8 — focused verification

Run formatting first so compiler output reflects final code shape:

```text
cargo fmt --all
```

Run focused tests:

```text
cargo test -p lisa-plugin codex_ack
cargo test -p lisa-plugin recycled_codex_ownership
cargo test -p lisa-plugin bounded_ack_wait
cargo test -p lisa-plugin consecutive_reused_panes
```

If test names change, use the closest exact filters and record them in
`progress.md`.

## Step 9 — complete verification

Run:

```text
cargo test -p lisa-plugin
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
cargo clippy -p lisa-plugin --lib -- -D warnings
cargo fmt --all -- --check
git diff --check
```

Run `just check` if the explicit commands do not already exercise an equivalent
repository gate or if project convention requires the combined result.

Record exact counts and any warnings.

## Step 10 — inspect ticket ownership

Inspect:

```text
git diff -- crates/lisa-plugin/src/lib.rs
git status --short
git diff --cached --name-only
```

Confirm:

- only `crates/lisa-plugin/src/lib.rs` is ticket-owned source;
- ticket frontmatter is unchanged;
- no ordinary-index entries were created;
- unrelated working-tree changes remain untouched;
- all six artifacts are under the ticket work directory.

## Step 11 — commit the implementation unit

Create one meaningful isolated source commit:

```text
lisa commit-ticket \
  --ticket-id T-034-02-01 \
  --message "fix: bind Codex acknowledgements to attempt leases" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not include RDSPI artifacts in the source commit.

Lisa will publish them with final ticket completion.

After the command, verify:

- the source path is clean;
- the ordinary index is empty;
- the commit contains exactly the included path.

## Step 12 — write Review handoff

Update `progress.md` throughout implementation with completed work, deviations,
tests, and commit result.

Write `review.md` after source commit verification.

The review must cover:

- source and artifact file inventory;
- lease-to-marker behavior;
- exact promotion guard;
- recovery successor behavior;
- acceptance-test evidence;
- full verification results;
- coverage gaps and open concerns;
- critical human-review issues, if any.

Stop after `review.md`.

Do not edit ticket phase or status.

Do not start another ticket.

## Expected atomic unit

This ticket has one cohesive implementation unit because dispatch generation,
promotion validation, and recovery successor minting establish one invariant:

```text
Codex acknowledgement generation == exact current attempt lease generation
```

Splitting those edits across commits would leave an intermediate state where
markers and validation disagree.
