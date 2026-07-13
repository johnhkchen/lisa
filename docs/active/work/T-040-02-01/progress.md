# Progress: attempt transition/failure provenance schema

## Status

Implementation is complete.
The ticket-owned source unit is committed through Lisa's isolated transaction.
Focused and workspace verification are green.
No ticket-owned source file remains modified, staged, or untracked.

## Completed phase work

- [x] Read `CLAUDE.md`, `AGENTS.md`, the active ticket, and the complete RDSPI
  workflow.
- [x] Mapped the existing provenance schema, append path, readers, tests, and
  pre-ownership scheduler failure vocabulary.
- [x] Wrote `research.md` in the attempt-private work directory.
- [x] Evaluated optional-field, tagged-enum, and separate-row alternatives.
- [x] Chose separate row structs with an untagged mixed-ledger reader.
- [x] Wrote `design.md`.
- [x] Defined the one-file source change and public interfaces.
- [x] Wrote `structure.md`.
- [x] Sequenced implementation, compatibility proof, verification, and isolated
  commit steps.
- [x] Wrote `plan.md`.

## Schema implementation

- [x] Bumped `SCHEMA_VERSION` from 2 to 3.
- [x] Left the existing `ProvenanceRecord` field shape unchanged.
- [x] Added `ProvenanceRecordType::AssignmentTransition`, serialized as
  `assignment-transition`.
- [x] Added durable `AssignmentState` variants:
  - `delivery-failed`;
  - `recovery-failed`;
  - `startup-failed`.
- [x] Added `AssignmentTransitionRecord`.
- [x] Required schema version, discriminator, ticket id, attempt lease, pane id,
  provider, state, reason, start time, end time, and duration.
- [x] Used vendor strings (`openai`/`anthropic`) for consistency with the
  existing provenance `Route.provider` meaning.
- [x] Kept execution-only outcome, authority, routing, tokens, and cost fields
  out of the pre-ownership row.
- [x] Added `ProvenanceLedgerRecord` with untagged `AssignmentTransition` and
  `Execution` variants.
- [x] Preserved literal schema-v2 execution parsing without defaults or data
  rewriting.

## Append implementation

- [x] Preserved the existing public `append_record` signature.
- [x] Added `append_assignment_transition_record` for the dependent plugin
  writer ticket.
- [x] Factored the common compact serialization, newline, parent creation, and
  append behavior into a private generic `append_serialized` helper.
- [x] Kept true append semantics and existing failure behavior unchanged.

## Unit tests

- [x] Updated the current execution-writer version assertion to 3.
- [x] Added a complete deterministic assignment-transition sample.
- [x] Asserted the new row serializes without embedded newlines.
- [x] Asserted exact discriminator, attempt, pane, provider, named-state,
  reason, and timestamp JSON fields.
- [x] Asserted direct assignment-row round trip equality.
- [x] Asserted the public assignment append entry point writes exactly one
  newline-terminated JSONL row.
- [x] Added a hand-written schema-v2 execution JSON fixture independent of the
  current schema constant.
- [x] Asserted direct v2 `ProvenanceRecord` deserialization retains version,
  attempt, outcome, and authority.
- [x] Asserted a two-line v2/v3 ledger parses into the correct mixed enum
  variants without changing the legacy value.

## Focused verification

Command:

```text
cargo fmt --all
cargo test -p lisa-core provenance
```

Result:

```text
12 passed; 0 failed; 0 ignored; 148 filtered out
```

This covers all provenance module tests, including existing append and hostile
path behavior.

## Workspace verification

Command:

```text
cargo test --workspace
cargo fmt --all -- --check
```

Result:

- command exited successfully;
- `lisa-cli` unit tests passed;
- `lisa-core` unit tests passed: 160 passed;
- `lisa-plugin` unit tests passed: 333 passed;
- integration and doc-test targets passed;
- formatting check passed.

The workspace run proves unchanged plugin `ProvenanceRecord` constructors remain
source compatible after the version bump.

## Diff inspection

`git diff --check -- crates/lisa-core/src/provenance.rs` passed before commit.
The final source diff contains one file:

```text
crates/lisa-core/src/provenance.rs | 168 lines changed
163 insertions, 5 deletions
```

The deletions are the module wording, schema constant, old literal assertion,
and append body moved to the common helper.

## Isolated commit

The globally installed `/opt/homebrew/bin/lisa` did not support the required
subcommand and returned:

```text
error: unrecognized subcommand 'commit-ticket'
```

The repository's freshly tested CLI binary did expose the required isolated
transaction, so the plan used:

```text
target/debug/lisa commit-ticket \
  --ticket-id T-040-02-01 \
  --message "feat(core): add pre-ownership provenance row" \
  --include crates/lisa-core/src/provenance.rs
```

Resulting commit:

```text
aa9f44e613cbcd3c58cc244b166dd75ec2df6a96
feat(core): add pre-ownership provenance row
```

`git show --check` passed for the commit.
The source path is clean after the transaction.
No ordinary Git add, index staging, or ordinary commit was used.

## Deviations from plan

One design refinement occurred during focused diff review.
The initial artifacts proposed `AgentClient` for the `provider` field and left
the assignment append choice to T-040-02-02.
That would have serialized method names (`codex`/`claude`) under a field whose
existing ledger meaning is vendor (`openai`/`anthropic`), and would have forced
the dependent plugin ticket to edit the shared schema file.

Before commit, the implementation changed `provider` to the established vendor
string and added the dedicated append entry point.
The Design, Structure, and Plan artifacts were updated to record the refined
contract.

## Preserved concurrent state

The worktree contains Lisa-owned active-ticket and published-work changes from
the running scheduler and another ticket.
They were not included in this ticket's isolated commit and were not reverted.
Only `crates/lisa-core/src/provenance.rs` was owned and committed here.

## Remaining work outside this ticket

- T-040-02-02 wires `append_assignment_transition_record` into the terminal
  pre-ownership failure sites and proves exact-once emission.
- T-040-02-03 reads `ProvenanceLedgerRecord` from the CLI and renders retained
  state, reason, provider, and timing.
- This ticket does not change retry policy, scheduler authority, or live
  provider behavior.
