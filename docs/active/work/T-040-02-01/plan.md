# Plan: attempt transition/failure provenance schema

## Step 1: preserve the compatibility fixture

Add a literal schema-v2 terminal execution JSON object to the provenance unit
tests.
Keep its original field names and required values independent of
`SCHEMA_VERSION`.
Deserialize it directly into `ProvenanceRecord`.

Verification:

- the literal parses successfully;
- its `schema_version` remains 2;
- its attempt lease, outcome, authority, route, and timing fields retain their
  encoded values.

This establishes the backward-compatible baseline before exercising the new
row.

## Step 2: add durable assignment vocabulary

Add `ProvenanceRecordType` with the serialized value
`assignment-transition`.
Add `AssignmentState` with `delivery-failed`, `recovery-failed`, and
`startup-failed` variants.

Verification:

- derive-based serialization produces kebab-case names;
- the record round-trip test observes the exact values.

These enums are one meaningful schema unit with the row that consumes them and
will be committed together rather than separately.

## Step 3: add the assignment-transition row

Add `AssignmentTransitionRecord` with required fields for schema version,
record kind, top-level ticket id, attempt lease, pane, provider, named state,
reason, transition start, transition end, and wall-clock duration.
Use `AgentClient` for provider identity and `AttemptLease` for attempt identity.

Add a deterministic sample helper.
Add the compact-line round-trip test.

Verification:

- serialized JSON contains no newline;
- exactly one JSON object represents the record;
- every acceptance-criterion field is present;
- direct deserialize returns the original value.

## Step 4: add the mixed-ledger read boundary

Add `ProvenanceLedgerRecord` as an untagged enum with assignment-transition and
execution variants.
Place the explicitly discriminated new row first.

Add a mixed JSONL test using the literal v2 line and current v3 assignment line.
Parse line by line as a reader would parse the append-only ledger.

Verification:

- line one selects `Execution`;
- the nested value equals direct v2 `ProvenanceRecord` deserialization;
- line two selects `AssignmentTransition`;
- the nested value equals the v3 sample;
- neither line is rewritten or defaulted into the other shape.

## Step 5: bump the writer schema version

Change `SCHEMA_VERSION` from 2 to 3.
Update the current execution compact-line assertion accordingly.
Do not alter the fields of `ProvenanceRecord`.

Verification:

- current sample execution rows carry version 3;
- literal v2 rows still carry version 2 after reading;
- both coexist through the mixed enum.

## Step 6: align module documentation

Update the provenance module overview to describe heterogeneous lifecycle rows
and name the three public schema types.
Keep the schema module independent of scheduler timing and emission policy.
Do not claim the downstream plugin writer or CLI reader already exists.

Verification:

- rustdoc references resolve during compilation;
- documentation distinguishes terminal execution from pre-ownership evidence.

## Step 7: format and run focused tests

Run `cargo fmt --all` as a mechanical formatter.
Run `cargo test -p lisa-core provenance`.

If a failure exposes ambiguity in untagged deserialization, inspect the exact
missing/extra field behavior before changing the public shape.
Prefer an explicit custom deserializer only if the required-field distinction is
insufficient.

Verification:

- all provenance unit tests pass;
- no unrelated files are formatted or modified.

## Step 8: run workspace tests

Run `cargo test --workspace` to cover existing plugin construction of
`ProvenanceRecord` and all downstream crate compilation.
Run `cargo fmt --all -- --check` after tests.

Verification:

- existing execution writer compiles unchanged;
- plugin provenance tests accept current schema version 3;
- CLI and core tests remain green;
- formatting check is clean.

If unrelated failures occur, record their exact command and evidence in
`progress.md` and do not broaden ticket scope without a demonstrated causal
link.

## Step 9: inspect ticket-owned diff

Use `git diff -- crates/lisa-core/src/provenance.rs`.
Confirm the diff contains only:

- documentation broadening;
- version bump;
- new schema enums and structs;
- mixed-ledger wrapper;
- focused compatibility and round-trip tests.

Inspect `git status --short` separately.
Preserve Lisa-owned changes to active tickets and any ledger state.

## Step 10: commit the meaningful source unit

Commit the schema and tests as one atomic unit with:

`lisa commit-ticket --ticket-id T-040-02-01 --message "feat(core): add pre-ownership provenance row" --include crates/lisa-core/src/provenance.rs`

Do not use ordinary `git add` or `git commit`.
Do not include phase artifacts, ticket files, provenance ledger files, or any
unrelated path.

Verification:

- the transaction succeeds;
- `crates/lisa-core/src/provenance.rs` is no longer modified;
- no ticket-owned source file remains staged, modified, or untracked.

## Step 11: record implementation progress

Write `progress.md` with completed steps, commands, outcomes, commit receipt,
and any deviations.
The artifact remains in the attempt-private work directory.

## Step 12: review

Inspect the committed diff and final status.
Write `review.md` summarizing the public contract, tests, compatibility proof,
and open concerns for T-040-02-02 and T-040-02-03.
Do not change the active ticket phase/status.
Stop after the review artifact and wait for Lisa's completion transaction.

## Testing strategy summary

Unit coverage owns serialized shape because this is a schema ticket.
The literal v2 fixture prevents false compatibility confidence from fixtures
generated by current code.
The mixed two-line parse approximates the production CLI reader without adding
CLI scope.
Workspace tests are the integration check that unchanged plugin writer call
sites still compile and existing execution provenance behavior remains intact.

No live provider, Zellij session, or scheduler fixture is required.
The ticket changes no runtime emission behavior.
