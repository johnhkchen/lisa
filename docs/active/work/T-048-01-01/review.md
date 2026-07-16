# Review — T-048-01-01 structured-block-schema

## Disposition

Pass.

The ticket's two acceptance areas are implemented, committed through Lisa's
isolated transaction, and covered by focused plus workspace tests.

## Summary

Review blocks now carry typed remedy ownership and actionable structure.

The parser recognizes agent-, operator-, and world-owned remedies. It stores an
ask, optional steps, and an optional future check probe without executing it.

Legacy or structurally malformed blocks remain valid non-passing blocks and
collapse to one safe representation:

- operator owned;
- raw reason copied exactly into ask;
- no steps;
- no check;
- flagged `unstructured`.

The provenance ledger now has typed park/unpark rows with ticket and attempt
identity, remedy owner, interval timestamps, and duration. Rows append and
replay through the mixed-ledger type alongside historical execution and
assignment rows.

No scheduling, parking, retry, unpark, rendering, or check-execution behavior
was added. Those remain owned by dependent tickets.

## Files changed

### `crates/lisa-core/src/disposition.rs`

Added public `RemedyOwner`:

- `Agent` serialized as `agent`;
- `Operator` serialized as `operator`;
- `World` serialized as `world`.

Extended `ReviewDisposition::Block` with:

- `remedy_owner: RemedyOwner`;
- `ask: String`;
- `steps: Option<Vec<String>>`;
- `check: Option<String>`;
- `unstructured: bool`.

The existing `reason: String` remains and preserves the authored bytes without
trimming or replacement.

Added structural parsing after the existing outer block validation boundary.

A structured block requires:

- a recognized lowercase owner;
- a non-empty, non-whitespace ask;
- when present, an array of non-empty, non-whitespace step strings;
- when present, a non-empty, non-whitespace check string.

Any defect in that structure discards all supplied structure and returns the
complete operator fallback. Partial structure is never trusted.

The parser still returns `Invalid` for:

- unreadable files;
- malformed JSON;
- non-object documents;
- absent or non-string disposition;
- invalid pass/reason relationships;
- block without a usable raw reason;
- unknown disposition values.

Only the remedy structure of an otherwise valid block receives the fallback.

### `crates/lisa-core/src/completion.rs`

Updated the direct non-passing block test fixture to the complete legacy
fallback shape.

Production reconciliation is unchanged. Exact `ReviewDisposition::Pass`
remains the only disposition that satisfies completion eligibility.

### `crates/lisa-core/tests/completion_state_machine.rs`

Updated the generated reference-model block fixture to the complete legacy
fallback shape.

The model still treats both Block and Invalid as non-passing.

### `crates/lisa-core/src/provenance.rs`

Bumped `SCHEMA_VERSION` from 3 to 4 for the new durable ledger shape.

Added public `ParkingTransitionType`:

- `Park` serialized as `park`;
- `Unpark` serialized as `unpark`.

Added public `ParkingTransitionRecord` with:

- schema version;
- record type;
- ticket ID;
- complete `AttemptLease`;
- typed `RemedyOwner`;
- `started_at`;
- `ended_at`;
- `wall_clock_secs`.

Added `ProvenanceLedgerRecord::ParkingTransition` to the existing untagged
mixed-row reader.

Added `append_parking_transition_record`, which reuses the existing compact,
newline-delimited, parent-creating, true-append helper.

Updated module documentation to name all three ledger families: execution,
assignment transition, and parking transition.

### `crates/lisa-plugin/src/lib.rs`

Updated one exhaustive execution-only ledger filter. It ignores parking rows in
the same way it ignores assignment rows.

There is no parking emission or policy change in the plugin in this ticket.

### `docs/knowledge/provenance-ledger.md`

Updated the linked durable-schema reference from its stale schema-v2-only
description.

It now documents:

- current schema version 4;
- terminal execution rows;
- schema-3 assignment transition rows;
- schema-4 park/unpark rows;
- mixed-ledger replay expectations;
- version history.

This documentation update was identified during Review and recorded as a
narrow plan deviation before implementation.

## Disposition test coverage

The disposition module now has 14 focused tests.

New coverage proves:

- all three owner strings parse to their typed variants;
- a structured block can omit steps and check;
- present steps and check are stored exactly without whitespace normalization;
- a two-field legacy block remains a Block;
- raw reason leading/trailing whitespace is byte-preserved;
- fallback ask exactly matches raw reason;
- missing owner falls back;
- missing ask falls back;
- unknown and non-string owners fall back;
- blank and non-string asks fall back;
- non-array steps fall back;
- non-string and blank step entries fall back;
- non-string and blank checks fall back;
- malformed structure discards otherwise valid optional fields;
- fallback is always operator-owned and unstructured;
- shell-looking check content is preserved but never executed.

The inert-check test embeds a `touch` command naming a temporary sentinel,
parses the file, and proves the sentinel does not exist afterward. The parser
contains no process or shell execution boundary.

Existing invalid-document coverage remains green.

## Provenance test coverage

The provenance module now has 14 focused tests.

New coverage proves:

- park rows serialize to one compact JSON line;
- unpark rows serialize to one compact JSON line;
- both row kinds round-trip exactly;
- typed remedy ownership survives serde;
- ticket and attempt identities survive serde;
- timestamps and duration survive serde;
- park then unpark append as two newline-terminated rows;
- append order is preserved;
- appended rows replay through `ProvenanceLedgerRecord`;
- a mixed ledger replays schema-v2 execution, schema-v3 assignment,
  schema-v4 park, and schema-v4 unpark variants correctly.

Existing append integrity, cross-ticket attribution, failure preservation,
usage extraction, route, and epoch conversion tests remain green.

## Verification results

Baseline:

```text
cargo test -p lisa-core disposition::tests --no-fail-fast
9 passed

cargo test -p lisa-core provenance::tests --no-fail-fast
12 passed
```

Focused after implementation:

```text
cargo test -p lisa-core disposition::tests --no-fail-fast
14 passed

cargo test -p lisa-core provenance::tests --no-fail-fast
14 passed

cargo test -p lisa-core completion::tests --no-fail-fast
25 passed

cargo test -p lisa-core --test completion_state_machine --no-fail-fast
1 passed

cargo check --workspace
passed

cargo test -p lisa-plugin --no-run
passed
```

Complete verification:

```text
cargo test --workspace --no-fail-fast
passed
```

Notable suite totals:

- CLI library: 19 passed;
- CLI binary: 318 passed;
- core: 214 passed;
- completion state-machine integration: 1 passed;
- recorded-livelock integration: 1 passed;
- plugin: 396 passed;
- all runnable CLI integration suites passed;
- one real-Zellij integration test was intentionally ignored because it needs
  external Zellij/tooling and the WASM target.

The workspace run emitted one unrelated unused-import warning in the concurrent
untracked `crates/lisa-cli/src/run_summary.rs` work. This ticket did not edit or
commit that path.

Ticket-owned formatting passed with:

```text
cargo fmt -p lisa-core -- --check
```

The plugin change was formatted before commit and compiled/tested in the full
workspace run.

A repository-wide format check was not clean because concurrent CLI changes in
`main.rs` and `run_summary.rs` are not yet rustfmt-formatted. Those paths are
outside this ticket and were left untouched.

## Commits

All ticket-owned source/documentation changes were committed through
`lisa commit-ticket` with exact includes:

```text
57c0ae64aab0e6cdbf86a2a1ee93b86cd83eef1e
feat(core): parse structured review blocks

706ddbc34cfa4b4c18a56b1a546c96edd6d020e0
feat(core): add parking transition provenance

d82890849d7997013f0895082fd0deb2fb4f6de2
docs(core): describe parking ledger rows

65b9c3e21c256e364b44ec2106202ecde63034f5
docs: document mixed provenance ledger
```

No ordinary `git add` or `git commit` command was used.

## Cleanliness

The ordinary Git index is empty.

No ticket-owned source or documentation path remains modified, staged, or
untracked.

Remaining worktree entries belong to Lisa state, concurrent CLI work, current
ticket orchestration, and other admitted work directories. They were present or
appeared under concurrent ticket execution and were not included in this
ticket's isolated commits.

## Compatibility assessment

Existing exact pass JSON remains Pass.

Existing exact two-field block JSON remains Block and keeps its raw reason
bytes. It gains the required safe semantic defaults rather than becoming
Invalid.

Completion remains fail-closed: only Pass grants authority.

Existing schema-v2 execution rows and schema-v3 assignment rows replay without
rewriting. New writers stamp schema 4.

Execution-only provenance readers do not misclassify parking rows.

## Open concerns and limitations

No blocking concerns remain for this ticket.

Intentional limitations are assigned to dependent tickets:

- no scheduler retry bound or parking policy;
- no park/unpark provenance emission yet;
- no ticket status mutation;
- no world-check execution or timeout enforcement;
- no read-only check sandbox;
- no automatic recheck;
- no operator unblock command;
- no dashboard/status ask rendering;
- no Review authoring guidance.

The parser validates JSON shape and non-whitespace content. It intentionally
does not attempt natural-language “one sentence” enforcement or prove that a
stored command is read-only. Those properties belong to authoring and execution
boundaries, not parse time.

The implementation is ready for T-048-01-02 to consume.
