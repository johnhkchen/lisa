# Progress — T-048-01-01 structured-block-schema

## Baseline

Read the ticket, repository agent context, RDSPI workflow, parent story, dependent
tickets, disposition implementation and consumers, provenance implementation
and readers, and existing focused tests.

The ordinary worktree began with Lisa-managed changes to ticket/provenance
state and unrelated published work. Those paths are not owned by this ticket.
Additional unrelated changes appeared while implementation was in progress,
consistent with concurrent Lisa tickets. They remain excluded from every
ticket commit.

Baseline verification passed:

```text
cargo test -p lisa-core disposition::tests --no-fail-fast
9 passed

cargo test -p lisa-core provenance::tests --no-fail-fast
12 passed
```

## Research, Design, Structure, Plan

Completed all four pre-implementation artifacts in the private attempt work
directory.

The design decisions are:

- use one public typed `RemedyOwner` vocabulary;
- expose complete semantic fields directly on `ReviewDisposition::Block`;
- preserve raw reason bytes independently from ask;
- atomically fall back on any missing/malformed remedy structure;
- store checks without executing or shell-parsing them;
- add one typed parking-transition provenance shape with park/unpark kinds;
- bump the ledger schema version for the new durable row shape;
- retain existing completion and scheduler behavior in this ticket.

## Implementation unit 1 — structured Review blocks

Modified `crates/lisa-core/src/disposition.rs`.

Added `RemedyOwner::{Agent, Operator, World}` with lowercase serde names.

Extended `ReviewDisposition::Block` with:

- `remedy_owner`;
- `ask`;
- optional `steps`;
- optional `check`;
- `unstructured`.

Kept `reason` as an exact unnormalized copy of the authored reason.

Added structural parsing after the existing valid-block boundary. A fully
structured block requires a recognized owner and non-whitespace ask. Present
steps must be an array of non-whitespace strings; a present check must be a
non-whitespace string.

Any structural defect now produces the exact fallback:

```text
owner = operator
ask = raw reason
steps = none
check = none
unstructured = true
```

Malformed outer JSON, invalid pass relationships, absent/unusable reason, and
unknown dispositions remain `Invalid`.

Added tests for:

- all three remedy owners;
- absent optional fields;
- retained steps and check;
- exact legacy reason preservation;
- missing and malformed owner/ask/steps/check;
- complete atomic fallback;
- hostile-looking check content remaining inert.

Updated direct legacy-block fixtures in:

- `crates/lisa-core/src/completion.rs`;
- `crates/lisa-core/tests/completion_state_machine.rs`.

The plugin's direct block matches already use or now resolve to `..` in the
current shared HEAD/worktree, so no ticket-owned plugin diff was required for
this unit.

Focused verification passed:

```text
cargo test -p lisa-core disposition::tests --no-fail-fast
14 passed

cargo test -p lisa-core completion::tests --no-fail-fast
25 passed

cargo test -p lisa-core --test completion_state_machine --no-fail-fast
1 passed

cargo check -p lisa-plugin
passed
```

Formatting completed with `cargo fmt --all`. The concurrent/unrelated
`crates/lisa-cli/src/runtime.rs` worktree change is not part of this ticket and
will not be included.

## Remaining

- commit parking provenance through Lisa;
- run focused and workspace verification;
- audit ticket-owned source cleanliness;
- write Review artifacts.

## Commit 1

Committed the structured Review block unit through Lisa's isolated transaction:

```text
57c0ae64aab0e6cdbf86a2a1ee93b86cd83eef1e
feat(core): parse structured review blocks
```

Exact included paths:

- `crates/lisa-core/src/disposition.rs`;
- `crates/lisa-core/src/completion.rs`;
- `crates/lisa-core/tests/completion_state_machine.rs`.

No ordinary-index Git command was used.

## Implementation unit 2 — parking transition provenance

Modified `crates/lisa-core/src/provenance.rs`.

Bumped `SCHEMA_VERSION` from 3 to 4 because the append-only ledger gained a
durable row shape.

Added `ParkingTransitionType::{Park, Unpark}` with kebab-case serde values.

Added `ParkingTransitionRecord` containing:

- schema version;
- park/unpark record type;
- ticket ID;
- complete attempt lease;
- typed remedy owner;
- start and end epoch-second timestamps;
- wall-clock duration.

Added `ProvenanceLedgerRecord::ParkingTransition` while retaining the untagged
mixed-ledger representation and old execution/assignment variants.

Added `append_parking_transition_record`, delegating to the existing generic
compact JSONL append helper.

Added tests for:

- compact park serialization and round-trip;
- compact unpark serialization and round-trip;
- typed remedy-owner serialization;
- two-row append ordering and newline framing;
- typed replay of appended rows;
- heterogeneous replay of schema-v2 execution, schema-v3 assignment,
  schema-v4 park, and schema-v4 unpark rows;
- retained attempt attribution, owner, timestamps, and duration.

Updated the plugin's execution-only provenance filter to ignore the new typed
transition alongside assignment transitions. This is a compatibility-only
match arm; no scheduler emission or parking behavior was added.

Focused verification passed:

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
14 passed

cargo check --workspace
passed after adding the explicit plugin parking-transition match arm

cargo test -p lisa-plugin --no-run
passed
```

The initial workspace check produced the expected compiler error at the one
exhaustive execution-only ledger match. Per the plan, that match was updated to
ignore non-execution parking rows; the next check passed. This was not a design
deviation.

## Commits 2 and 3

Committed the parking provenance implementation through Lisa's isolated
transaction:

```text
706ddbc34cfa4b4c18a56b1a546c96edd6d020e0
feat(core): add parking transition provenance
```

Exact included paths:

- `crates/lisa-core/src/provenance.rs`;
- `crates/lisa-plugin/src/lib.rs`.

After the full suite, corrected the provenance module header so its public
documentation names execution, assignment, and parking rows. Committed that
ticket-owned documentation-only follow-up through Lisa:

```text
d82890849d7997013f0895082fd0deb2fb4f6de2
docs(core): describe parking ledger rows
```

Exact included path:

- `crates/lisa-core/src/provenance.rs`.

No ordinary-index Git command was used for any unit.

## Full verification

The complete command passed:

```text
cargo test --workspace --no-fail-fast
```

Relevant suite totals included:

- `lisa-cli` library: 19 passed;
- `lisa-cli` binary: 318 passed;
- CLI integration suites: all runnable tests passed;
- `lisa-core`: 214 passed;
- completion state machine: 1 passed;
- recorded livelock regression: 1 passed;
- `lisa-plugin`: 396 passed;
- doc tests: passed with no tests;
- real-Zellij delivery boundary: 1 intentionally ignored because it requires
  external Zellij/tooling and the WASM target.

The workspace run emitted one unrelated warning: an unused `PathBuf` import in
the concurrently created `crates/lisa-cli/src/run_summary.rs`. That file is not
owned by this ticket and was not edited.

Repository-wide `cargo fmt --all -- --check` was attempted after the suite. It
reported formatting diffs in concurrent CLI work (`main.rs` and
`run_summary.rs`). The ticket-owned subset passed:

```text
cargo fmt -p lisa-core -- --check
```

The plugin ticket diff was formatted before its commit, and the complete suite
compiled it successfully.

## Final cleanliness audit

The following ticket-owned paths have no worktree diff and no staged entry:

- `crates/lisa-core/src/disposition.rs`;
- `crates/lisa-core/src/completion.rs`;
- `crates/lisa-core/src/provenance.rs`;
- `crates/lisa-core/tests/completion_state_machine.rs`;
- `crates/lisa-plugin/src/lib.rs`.

`git diff --check` over those exact paths passed.

The ordinary index is empty. Remaining modified/untracked paths are Lisa state,
the current ticket frontmatter, concurrent CLI work, and other Lisa-published
work directories. None is ticket-owned source for T-048-01-01.

Implementation and verification are complete. Review artifacts remain.

## Review-found documentation deviation

Review found that `docs/knowledge/provenance-ledger.md`, which is linked from
the schema owner, still claimed schema version 2 and documented only execution
rows. It had already fallen behind schema 3 assignment transitions and would be
more misleading after this ticket's schema 4 parking rows.

This reference was not listed in the original Structure inventory. Updating it
is a narrow deviation necessary to keep the public durable-schema reference
consistent with the implemented contract. The change will document existing
schema-3 assignment rows plus this ticket's schema-4 parking rows without
changing behavior.
