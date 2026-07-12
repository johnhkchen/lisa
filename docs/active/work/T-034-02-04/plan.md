# Plan: T-034-02-04 one authoritative provenance record

## Step 1: establish schema-v2 attempt attribution

Modify `crates/lisa-core/src/provenance.rs`.

- Import `AttemptLease`.
- Set `SCHEMA_VERSION` to 2.
- Add required `attempt_lease`, `authoritative`, and `fenced` fields.
- Keep all existing metric and route fields unchanged.
- Update the sample record with attempt one.
- Assert new JSON fields in the compact serialization test.
- Preserve enum wire names and append behavior.

Verification:

- `cargo test -p lisa-core provenance`;
- serialized record round-trips;
- schema-v2 JSON exposes the complete lease and flags.

Atomic outcome: the core record can express required evidence.

## Step 2: enforce provenance publisher identity

Modify `State::emit_provenance` in `crates/lisa-plugin/src/lib.rs`.

- Accept `fenced: bool`.
- Read and clone `thread.attempt_lease`.
- Log and reject an unleased record.
- For Done, require exact current-lease equality.
- Derive `authoritative` only from accepted Done.
- Populate all schema-v2 fields.
- Return whether append succeeded.
- Preserve route, usage, timestamps, and non-fatal I/O logging.

Verification:

- compiler identifies every caller needing explicit fence state;
- a stale direct Done publication appends nothing;
- current Done publishes with `authoritative: true`.

Atomic outcome: ledger publication itself is an authority boundary.

## Step 3: capture actual fencing outcomes

Update teardown call sites.

- Normal verified completion passes `fenced: false`.
- Error-signal failure passes `fenced: false`.
- Session/per-phase timeout fences before emitting provenance.
- Hard-silence reclamation fences before emitting provenance.
- Map `Fenced` and `AlreadyFenced` to true.
- Map `NoAssignedPane` to false.
- Keep the thread until after record construction.
- Preserve revoke-before-release ordering.

Verification:

- existing lifecycle order test remains green;
- timeout record carries the timed-out attempt lease;
- timeout record reports `fenced: true` for a confirmed closed pane.

Atomic outcome: predecessor history is reconstructable from ledger rows.

## Step 4: close asynchronous completion authority gap

Protect an admitted completion transaction from replacement.

- Exclude `pending_completions` from session timeout candidates.
- Exclude `pending_completions` from hard-silence candidates.
- Revalidate the pending authority at result handling.
- Reject a stale attempt result before completion lifecycle publication.
- Remove rejected pending state and rebuild the DAG mask.
- Leave operator-only manual behavior unchanged.
- Retain command failure and durable-Done verification behavior.

Verification:

- a pending current attempt is not timed out or redispatched;
- a forged/replaced stale pending result publishes no Done record;
- a current result follows the existing isolated completion path.

Atomic outcome: request admission and result publication share one lease.

## Step 5: migrate provenance fixtures

Update tests that directly construct threads.

- Install a current attempt before successful record emission.
- Ensure helper ordering does not overwrite custom thread fields.
- Update all direct `emit_provenance` calls with fence state.
- Retain the unset-ledger no-panic/no-write check.
- Retain ticket-frontmatter byte equality.
- Retain provider usage assertions.
- Retain retry append assertions using distinct attempt generations.

Verification:

- no production-like record is emitted without a lease;
- existing metrics and route expectations remain unchanged.

Atomic outcome: old coverage models the strengthened production invariant.

## Step 6: add the acceptance regression

Create a focused plugin test for a fenced predecessor and replacement.

Setup:

- configure a temporary ledger;
- construct the ticket, thread, and slot;
- mint/stamp predecessor lease;
- fence it and append TimedOut history;
- release/remove predecessor;
- construct replacement and mint successor lease.

Exercise:

- attempt stale Done publication with predecessor identity;
- request completion from the current successor;
- simulate durable ticket Done and a valid commit result;
- deliver the result callback twice.

Assertions:

- predecessor row is attributable to attempt one;
- predecessor row is timed-out and fenced;
- predecessor row is non-authoritative;
- replacement row is attributable to attempt two;
- replacement row is Done and authoritative;
- exactly one row satisfies Done plus authoritative;
- duplicate/stale paths do not add rows.

Atomic outcome: the ticket acceptance criterion is executable.

## Step 7: document schema-v2 semantics

Modify `docs/knowledge/provenance-ledger.md`.

- Update version and example.
- Add the three fields to the table.
- Explain complete lease attribution.
- Explain `fenced` as confirmed scheduler action.
- Explain `authoritative` as ticket-level Done publication.
- Add an authoritative-success query.
- Explain mixed v1/v2 history and version-aware readers.
- Preserve provider token fidelity guidance.

Verification:

- example parses as JSON;
- field table matches the Rust struct;
- terminology matches ticket acceptance.

Atomic outcome: downstream readers can interpret the new evidence correctly.

## Step 8: focused verification

Run formatting and narrow suites.

Commands:

```sh
cargo fmt --all -- --check
cargo test -p lisa-core provenance
cargo test -p lisa-plugin provenance
cargo test -p lisa-plugin completion
cargo test -p lisa-plugin timeout
```

If filtering misses relevant tests, run the complete plugin test suite.

Verification criteria:

- no formatting diff;
- schema and append tests pass;
- stale/current completion tests pass;
- timeout/fence ordering tests pass;
- existing provider usage tests pass.

## Step 9: workspace verification

Run the project-standard checks proportionate to the shared schema change.

Commands:

```sh
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
```

Use `just check` if it adds no destructive or unrelated mutation.

Verification criteria:

- native workspace tests pass;
- WASM plugin compiles;
- no ticket-owned file remains unexpectedly modified after commit.

## Step 10: isolated ticket commit

Inspect exact diffs and status.

Commit the meaningful implementation unit through:

```sh
lisa commit-ticket \
  --ticket-id T-034-02-04 \
  --message "Attribute authoritative provenance to attempt leases" \
  --include crates/lisa-core/src/provenance.rs \
  --include crates/lisa-plugin/src/lib.rs \
  --include docs/knowledge/provenance-ledger.md
```

Do not include unrelated dirty files or ordinary index entries.

Confirm the three source/documentation paths are clean afterward.

## Step 11: implementation and review artifacts

Write `progress.md` throughout implementation.

Record:

- completed steps;
- commands and results;
- deviations and rationale;
- isolated commit identity;
- remaining concerns.

Then write `review.md` with:

- exact file inventory;
- behavior summary;
- acceptance-criterion evidence;
- test coverage and commands;
- compatibility notes;
- open concerns and human-review focus.

Do not edit ticket phase/status and do not publish completion directly.
