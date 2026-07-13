# Review: attempt transition/failure provenance schema

## Disposition

PASS.

T-040-02-01 satisfies its acceptance criterion.
The provenance schema now has a complete attempt-scoped pre-ownership row at
schema version 3.
The row writes as exactly one JSONL line.
An unchanged literal schema-v2 `ProvenanceRecord` still deserializes directly
and through the new mixed-ledger representation alongside the v3 row.

No blocking issue was found during self-review.

## Commit reviewed

```text
aa9f44e613cbcd3c58cc244b166dd75ec2df6a96
feat(core): add pre-ownership provenance row
```

The commit contains exactly one source file:

```text
crates/lisa-core/src/provenance.rs
```

No active ticket, shared work artifact, provenance ledger, plugin source, CLI
source, or unrelated worktree path is included.
`git show --check` reports no whitespace error.

## Public schema changes

`SCHEMA_VERSION` is now 3.

`ProvenanceRecordType` provides the explicit new JSON discriminator:

```text
assignment-transition
```

`AssignmentState` provides stable terminal pre-ownership state names:

- `delivery-failed`;
- `recovery-failed`;
- `startup-failed`.

`AssignmentTransitionRecord` requires:

- `schema_version`;
- `record_type`;
- `ticket_id`;
- exact `attempt_lease`;
- `pane_id`;
- vendor `provider`;
- named `state`;
- human-readable `reason`;
- `started_at`;
- `ended_at`;
- `wall_clock_secs`.

The type derives serde serialization and deserialization and equality traits.
No field is optional or defaulted.
Incomplete transition evidence therefore fails structural deserialization.

`ProvenanceLedgerRecord` is an untagged mixed-row enum with:

- `AssignmentTransition(AssignmentTransitionRecord)`;
- `Execution(ProvenanceRecord)`.

The untagged representation is deliberate.
Schema-v2 execution objects predate an explicit discriminator, so a normally
tagged enum would require rewriting old bytes or a custom compatibility parser.
The new row's required discriminator/state/reason fields and the execution
row's required outcome/route fields keep the alternatives distinct.

## Existing execution compatibility

`ProvenanceRecord` itself is structurally unchanged.
Existing plugin construction remains source-compatible.
Its current writer stamps version 3 because it uses the shared constant, but
old embedded version numbers remain data rather than validation gates.

The compatibility fixture is a hand-written literal v2 JSON object.
It is not generated from the current sample or version constant.
That matters because a fixture produced by current code could silently move to
v3 and stop proving backward compatibility.

The test parses the v2 object directly into `ProvenanceRecord`, verifies its
version, attempt id, successful outcome, and authority, then parses it as the
first line of a mixed ledger.
The mixed reader selects `Execution` and retains the directly parsed value.

## New row serialization

The compact serialization test verifies:

- no embedded newline;
- `schema_version: 3`;
- `record_type: assignment-transition`;
- top-level ticket and nested attempt attribution;
- pane 12;
- provider `openai`;
- state `delivery-failed`;
- the complete failure reason;
- start, end, and duration values;
- equality after deserialize.

The append test calls the public assignment writer against a temporary ledger.
It verifies the bytes end in a newline, contain exactly one newline, and parse
back into the complete row after removing that delimiter.
This directly covers the acceptance phrase “serializes to one JSONL line,” not
only an in-memory serde string.

## Append API

The existing `append_record(&ProvenanceRecord)` remains unchanged for callers.
The new `append_assignment_transition_record(&AssignmentTransitionRecord)` is
available to T-040-02-02.
Both delegate to a private generic helper with the established behavior:

- create missing parent directories;
- serialize compact JSON;
- append one newline;
- open in create-and-append mode;
- never rewrite existing rows;
- convert serialization failure to invalid-data I/O failure.

This lets the dependent plugin ticket remain confined to
`crates/lisa-plugin/src/lib.rs`, preserving the story's intended parallel
ownership with the CLI reader ticket.

## Provider semantics

The row stores the vendor name, currently `openai` or `anthropic`.
This matches `ProvenanceRecord.requested.provider` and
`ProvenanceRecord.actual.provider`.
It intentionally does not store `AgentClient`, whose serialized values are the
integration methods `codex` and `claude`.

This distinction was caught during implementation review and corrected before
commit.
The downstream writer can derive the vendor through the same mapping already
used by `Route::from_client`.

## Test coverage

Focused command:

```text
cargo test -p lisa-core provenance
```

Result:

```text
12 passed; 0 failed
```

The 12 tests cover new-row round trip, exact one-line append, mixed v2/v3 read,
current execution round trip, route and outcome serialization, usage parsing,
time conversion, true append behavior, attempt attribution, and failed-write
preservation.

Workspace command:

```text
cargo test --workspace
cargo fmt --all -- --check
```

Result: successful.
All CLI, core, plugin, integration, and doc-test targets passed, and formatting
is clean.
Notably, all 333 plugin unit tests passed with the unchanged execution-record
construction sites.

## Acceptance-criterion assessment

“provenance.rs gains the new row type” — met by
`AssignmentTransitionRecord`, its discriminator/state vocabulary, mixed reader,
and append function.

“SCHEMA_VERSION bump” — met; current value is 3.

“round-trip unit tests pass” — met for both direct serde and physical JSONL
append/read.

“forward/backward-compat unit tests pass” — met by the mixed v2 execution and v3
assignment ledger test.

“new row serializes to one JSONL line carrying all required fields” — met by
field-level compact serialization assertions and the one-newline append test.

“existing schema-v2 ProvenanceRecord still deserializes unchanged from the same
ledger” — met with the literal v2 fixture and equality through the mixed reader.

## Known limitations and downstream obligations

The `provider` field is a `String`, matching the existing route schema.
The core type does not reject an unknown vendor spelling.
T-040-02-02 should derive it from the resolved client rather than accept
arbitrary user input, and its tests should pin `openai`/`anthropic` values.

The timestamp triple is writer-supplied.
Serde does not enforce `ended_at >= started_at` or recompute
`wall_clock_secs`.
The plugin writer should use saturating subtraction, matching existing execution
provenance behavior, and unit-test deterministic values.

The schema permits a top-level `ticket_id` that differs from
`attempt_lease.ticket_id`, as the existing execution struct does.
Writer tests must preserve the same-ticket invariant.

The mixed enum uses serde's normal unknown-field tolerance.
Future row shapes must include distinct required fields or introduce explicit
version-aware custom deserialization if shapes begin to overlap.

`docs/knowledge/provenance-ledger.md` still documents the operational terminal
execution ledger at schema v2.
This schema-only ticket does not claim the plugin emits the new row yet.
The story should update that operational documentation when T-040-02-02 and
T-040-02-03 land so it can accurately describe emitted rows and CLI queries in
one pass.

## Scope deliberately not implemented

No scheduler terminal site emits the new row yet.
No retry count or policy changed.
No live provider behavior was exercised.
No CLI command reads or prints the row yet.
No execution outcome or authoritative Done row is fabricated for a ticket that
never reached ownership.

Those boundaries belong to the dependent tickets and the later hostile-path
regression/field-report story.

## Final repository state

`crates/lisa-core/src/provenance.rs` is clean after the isolated commit.
The remaining active-ticket/shared-work changes belong to Lisa and concurrent
ticket processing and were preserved outside this commit.
The six required attempt artifacts now exist in the private work directory.

The ticket is ready for Lisa's completion transaction.
