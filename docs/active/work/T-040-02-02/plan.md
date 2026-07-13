# Plan: persist pre-ownership failure row

## Step 1: import the established schema vocabulary

Extend the plugin's provenance imports with the assignment transition record,
state, and discriminator types.

Verification:

- imports come from `lisa_core::provenance`;
- no duplicate local schema is introduced;
- existing execution imports remain intact.

## Step 2: add the shared pre-ownership writer

Implement `State::emit_assignment_transition` next to `emit_provenance`.

Make an empty ledger path a no-op.

Resolve matching slot, lease, and thread facts.

Reject missing or inconsistent facts with an activity warning.

Derive vendor provider from the snapshotted thread client.

Build timing from the thread attempt start and current observation time.

Construct `AssignmentTransitionRecord` using schema version 3 and the explicit
assignment-transition record type.

Append through `append_assignment_transition_record`.

Log and swallow I/O failure consistently with execution provenance.

Verification:

- no ticket/frontmatter writes;
- no scheduler-state mutation;
- no fabricated outcome or authority value;
- duration uses saturating subtraction.

## Step 3: wire delivery failure

Call the writer once in `fail_assignment_delivery` after valid ticket/thread
failure handling.

Pass `AssignmentState::DeliveryFailed` and the exact caller reason.

Preserve existing source-state guard, retained seat, alert deduplication,
activity log, and return outcome.

Verification:

- a first valid transition appends;
- a repeated call from `DeliveryFailed` returns `None` before append;
- missing reservation does not fabricate a row.

## Step 4: wire recovery failure

Call the writer once in `fail_assignment_recovery` with
`AssignmentState::RecoveryFailed`.

Preserve the current successor lease for operator reset.

Verification:

- exact recovery attempt is recorded;
- current/high-water authority maps remain unchanged;
- repeated terminal calls do not append.

## Step 5: wire startup failure

Call the writer once in `fail_startup` with
`AssignmentState::StartupFailed`.

Leave `fail_startup_recovery` unchanged because it is outside the named
acceptance sites.

Verification:

- exact startup attempt and pane are recorded;
- reservation remains in `StartupFailed`;
- repeated calls do not append.

## Step 6: add mixed-ledger test support

Add a test helper that deserializes JSONL through `ProvenanceLedgerRecord`.

Retain the execution-only reader for existing provenance tests.

Add a reserved-state fixture with matching ticket, pane, attempt, provider,
thread, lease maps, source state, and temporary ledger.

Verification:

- fixture reflects production consistency invariants;
- each independent terminal case gets a fresh ledger.

## Step 7: test all three transitions

Drive `fail_assignment_delivery`, `fail_assignment_recovery`, and
`fail_startup` directly through their real state mutations.

For each appended assignment row assert:

- schema version;
- assignment-transition record type;
- ticket ID;
- exact attempt lease;
- pane ID;
- vendor provider;
- named assignment state;
- exact reason;
- started/ended ordering;
- saturating duration relationship.

Read the raw JSON and assert that execution-only `authoritative` and `outcome`
members are absent.

Call each failure helper a second time.

Assert the second call is rejected and the JSONL still contains exactly one
line.

## Step 8: test coexistence with a later terminal record

After one pre-ownership row, install a later attempt for the same ticket.

Ensure the later thread and current lease match.

Append an authoritative Done execution record through the existing writer.

Read both rows using the heterogeneous reader.

Assert the first assignment row remains unchanged and the second row is one
authoritative Done execution.

Verification:

- two total lines;
- original order retained;
- no overwrite;
- no extra authoritative outcome.

## Step 9: update real recovery characterization

Replace the obsolete no-ledger expectation in the recovery timeout test.

Assert the actual timeout transition writes one recovery-failed row using the
successor lease and the production reason.

Retain the test's authority and retained-state assertions.

Verification:

- real deadline evaluator reaches the writer;
- subsequent timeout checks append no duplicate row.

## Step 10: focused verification

Run formatting:

```text
cargo fmt --all
cargo fmt --all -- --check
```

Run targeted plugin tests using relevant name filters where helpful.

Run the full plugin test target:

```text
cargo test -p lisa-plugin
```

Run workspace verification:

```text
cargo test --workspace
```

Run diff hygiene:

```text
git diff --check -- crates/lisa-plugin/src/lib.rs
git diff -- crates/lisa-plugin/src/lib.rs
```

Verification criteria:

- all three path tests pass;
- existing execution provenance tests pass;
- all plugin tests pass;
- workspace tests pass;
- formatting and diff checks pass.

## Step 11: record progress

Write `progress.md` in the attempt-private work directory.

Record completed steps, test commands and results, deviations, and exact
source ownership.

Do not place attempt artifacts in the shared work path.

## Step 12: isolated source commit

Commit the single meaningful source unit with:

```text
lisa commit-ticket \
  --ticket-id T-040-02-02 \
  --message "feat(plugin): persist pre-ownership failures" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use ordinary `git add` or `git commit`.

Verify the source file is no longer modified or staged after the isolated
transaction.

Do not include ticket files, `.lisa/provenance.jsonl`, shared work artifacts,
or unrelated working-tree changes.

## Step 13: review

Inspect the committed diff and verification evidence.

Write `review.md` and the exact valid `review-disposition.json` shape in the
attempt-private work directory.

Assess acceptance coverage, exact-once behavior, non-authority semantics,
coexistence, test gaps, and open concerns.

Remain on this ticket after review and allow Lisa to publish completion.
