# Design: attempt transition/failure provenance schema

## Goals

The schema must represent a durable pre-ownership event without pretending that
an agent run completed.
Every new row must identify the ticket, exact attempt, pane, provider, named
state, reason, and timing.
New writers must stamp a bumped schema version.
Readers must accept both the new row and unchanged schema-v2 execution rows in
one append-only ledger.
Existing execution writers and tests should not require shape changes.

## Option 1: extend `ProvenanceRecord` with optional fields

This approach would add fields such as `state` and `reason` to the existing
execution record and make unrelated execution fields optional for pre-ownership
rows.

Advantages:

- one Rust struct and one append function;
- existing readers keep a single nominal type;
- no row wrapper is required.

Costs:

- invalid combinations become representable;
- a pre-ownership row could accidentally carry an execution outcome;
- consumers must infer row kind from missing fields;
- making current required fields optional weakens established guarantees;
- the existing v2 type no longer models a v2 row directly;
- downstream code can fabricate nonsensical partial terminal records.

This is rejected because the two rows describe different lifecycle boundaries.
Optional-field unions would make state mean less precisely what it says.

## Option 2: internally tagged ledger enum

This approach would define an enum serialized with a discriminator such as
`record_type: execution | assignment-transition`.

Advantages:

- explicit row kinds;
- exhaustive matching for readers;
- straightforward future extension.

Costs:

- existing schema-v2 JSON has no discriminator;
- deserializing old rows would require a custom compatibility implementation;
- wrapping all new execution rows would change their serialized shape;
- retaining their old shape while accepting absent tags adds custom branching
  whose only purpose is migration.

An explicit discriminator is useful on the new row itself, but a fully tagged
enum cannot directly represent untouched v2 ledger bytes.
This option is rejected as the mixed-ledger representation.

## Option 3: separate row structs plus an untagged ledger enum

Define `AssignmentTransitionRecord` alongside the unchanged
`ProvenanceRecord`.
Define `ProvenanceLedgerRecord` with `AssignmentTransition` and `Execution`
variants and `#[serde(untagged)]`.
Give the new row a required `record_type` field serialized to a fixed enum value.

Advantages:

- preserves exact v2 execution deserialization;
- keeps execution construction and append call sites unchanged;
- provides exhaustive mixed-ledger matching;
- required fields prevent ambiguous null-based variants;
- the new row's discriminator is visible to non-Rust JSON consumers;
- the new row can evolve independently from execution metrics.

Costs:

- untagged deserialization tries variants in order;
- two public row types exist;
- the existing append helper remains execution-specific until writer wiring
  chooses a generalized append boundary.

The shapes have multiple unique required fields, including `record_type`,
`state`, and `reason` on the assignment row versus `outcome`, routes, and usage
fields on the execution row.
Variant ambiguity is therefore controlled.
This is the chosen approach.

## Row identity

The new type is `AssignmentTransitionRecord`.
The mixed row type is `ProvenanceLedgerRecord`.
The explicit discriminator type is `ProvenanceRecordType`, initially containing
`AssignmentTransition` serialized as `assignment-transition`.

`ProvenanceRecord` retains its existing name because renaming it would churn the
plugin and contradict backward-compatible reads.
The wrapper variant uses the clearer name `Execution`.

## Attempt identity

The new row carries `attempt_lease: AttemptLease`, not separate ticket and
generation primitives.
This reuses the established authority value and makes the attempt scope
unambiguous.
It also carries top-level `ticket_id` for ledger filtering consistency with
execution rows.
Writers are responsible for supplying matching ticket ids, as they are today.

## Provider identity

The new row carries `provider: AgentClient`.
This is the shared provider/client vocabulary already used by scheduler routes.
It serializes to stable lowercase `claude` or `codex` values.
A full requested/actual `Route` is not used because no successful execution
route has yet been established at the pre-ownership boundary.
A free-form string is rejected because it would bypass the existing parser and
permit spelling drift.

## Named state

The durable state type is `AssignmentState` with terminal variants:

- `DeliveryFailed` → `delivery-failed`;
- `RecoveryFailed` → `recovery-failed`;
- `StartupFailed` → `startup-failed`.

These names align with the scheduler's retained named failure states while
remaining independent of its private data-bearing enum.
The initial writer ticket only targets terminal sites, so exposing every
intermediate scheduler implementation state now would enlarge the contract
without a writer requirement.

An enum is chosen over `String` so the downstream writer and CLI cannot silently
disagree about spelling.
Future schema versions may add variants as durable transition capture expands.

## Reason

`reason` is a required `String`.
Failure methods already receive human-readable reasons and include them in
activity logs.
The ledger stores that evidence verbatim rather than deriving it from the named
state.
An empty string remains structurally possible because serde has no semantic
validator; writer tests can enforce the operational invariant.

## Timestamps

The row uses `started_at`, `ended_at`, and `wall_clock_secs`, all UTC epoch
seconds.
This matches existing ledger conventions and supplies both absolute timing and
query-friendly duration.
The downstream plugin can map the slot's transition start to `started_at`, the
failure observation to `ended_at`, and compute a saturating difference.

Alternative names such as `transition_started_at` are more explicit but make
generic ledger queries less uniform.
The row type and record discriminator already establish that these timestamps
belong to the assignment transition.

## Authority semantics

The new row has no `authoritative` or `outcome` fields.
Those fields describe terminal execution publication, which did not happen.
The row's type and named failure state are the truthful evidence.
This prevents the writer from fabricating `authoritative: false` execution
outcomes merely to reuse the old shape.

## Schema version

`SCHEMA_VERSION` becomes `3`.
New assignment rows and newly emitted execution rows both use the current writer
version constant.
Literal schema-v2 execution rows remain accepted because `schema_version` is a
plain field and deserialization does not reject older numeric values.
The mixed enum does not use the version alone as the discriminator; it uses row
shape so old data needs no rewrite.

## Serialization and test decisions

The new row derives `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, and
`Deserialize`.
The wrapper derives the same traits.
`f64` prevents `Eq` on the existing execution row and therefore on the wrapper;
the wrapper derives `PartialEq` only.

Tests will assert:

- schema version is 3;
- the new record is one compact JSON line;
- all required fields appear with exact serialized names;
- round trip returns an equal assignment record;
- a literal schema-v2 execution row still deserializes directly into
  `ProvenanceRecord`;
- a two-line mixed ledger deserializes through `ProvenanceLedgerRecord` into one
  execution variant and one assignment-transition variant;
- the v2 execution value is unchanged after parsing.

## Documentation scope

Module comments will be broadened from terminal execution only to heterogeneous
provenance rows.
The knowledge document is outside the ticket's explicit acceptance criterion
and is likely to be updated with the completed writer/reader story.
This ticket will avoid editing it so downstream parallel tickets do not inherit
an incomplete operational description claiming rows are already emitted.
