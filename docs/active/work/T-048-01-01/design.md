# Design — T-048-01-01 structured-block-schema

## Goals

The change must make a valid blocking Review disposition answer four questions
as typed data:

1. why completion was blocked;
2. who owns the remedy;
3. what that owner is being asked to do;
4. how later code may describe actions and detect that the remedy happened.

It must also provide append-only evidence types for park and unpark transitions
without implementing either transition in the scheduler.

The design keeps completion fail-closed. Only `Pass` grants completion. A valid
block remains non-passing whether its remedy structure is fully valid or has
fallen back to the legacy-safe representation.

## Decision 1: use a public remedy-owner enum

Introduce `RemedyOwner` in `lisa_core::disposition` with these variants:

- `Agent`;
- `Operator`;
- `World`.

Derive debug, clone/copy, equality, serde serialization, and serde
deserialization. Serialize and deserialize with lowercase names.

### Alternatives

Keeping the owner as `String` would minimize type declarations, but every
consumer would need to repeat validation and handle typos. The scheduler and
provenance requirements both branch on the same closed vocabulary, so a string
would move the parser's responsibility downstream.

Defining separate owner enums in disposition and provenance would isolate the
modules, but it would permit impossible conversion failures between two types
that intentionally name the same concept.

### Rationale

One public enum makes invalid owners unrepresentable after parsing and gives
future scheduler matches exhaustiveness checking. Copy semantics are suitable
because the type is a three-value classification with no owned payload.

## Decision 2: extend the existing Block variant directly

Use this conceptual shape:

```text
ReviewDisposition::Block {
    reason,
    remedy_owner,
    ask,
    steps,
    check,
    unstructured,
}
```

`reason`, `ask`, and each step are owned strings. `steps` is
`Option<Vec<String>>`; `check` is `Option<String>`; `unstructured` is a bool.

### Alternatives

A `BlockDisposition` struct wrapped by `ReviewDisposition::Block(block)` would
provide a named unit and shorter enum patterns. It would also replace every
existing struct-variant pattern and make the API change more disruptive.

A nested optional `remedy: Option<StructuredRemedy>` would distinguish legacy
and structured blocks structurally. It would force every consumer to unwrap a
second layer and would not directly express the required deterministic fallback
values. The acceptance contract asks for operator ownership, raw-reason ask,
and an unstructured flag even when input is missing.

Making `remedy_owner` and `ask` optional in the parsed result would mirror the
wire format but export uncertainty that the parser is specifically required to
resolve. It would also invite downstream callers to invent different defaults.

### Rationale

Direct fields preserve the recognizable `Block { reason, .. }` API and make
the downstream scheduler's required branch simple. The parser always returns a
complete semantic block. Optionality remains only where the ticket explicitly
permits absence: steps and check.

## Decision 3: preserve reason as compatibility data

For every otherwise valid block, store the exact input `reason` string. Do not
trim, normalize, or replace it with `ask`.

Existing completion rejection and plugin attention diagnostics continue to use
`reason`. This ticket will update only their match syntax to ignore the added
fields. Changing user-visible output to prefer `ask` belongs to S-048-02.

The legacy block `{"disposition":"block","reason":"..."}` therefore remains
a valid block with identical raw reason bytes. It gains deterministic semantic
defaults but is not converted to `Invalid`.

## Decision 4: validate structure atomically

A block is structured only if all supplied structural fields are valid.

Required structured fields:

- `remedy_owner` is one of `agent`, `operator`, or `world`;
- `ask` is a non-empty, non-whitespace string.

Optional structured fields:

- absent `steps` maps to `None`;
- present `steps` must be an array of non-empty, non-whitespace strings and
  maps to `Some`, including an explicitly empty array;
- absent `check` maps to `None`;
- present `check` must be a non-empty, non-whitespace string.

If any required field is absent or any present field is malformed, discard all
supplied remedy structure and return:

```text
remedy_owner = Operator
ask = raw reason
steps = None
check = None
unstructured = true
```

A fully valid structure returns `unstructured = false`.

### Alternatives

Partially retaining valid fields would preserve more input, such as a valid
check beside an invalid owner. It would also trust a combination the author did
not successfully specify. A world check accidentally reclassified as operator
data could later be surfaced or executed under the wrong policy.

Treating malformed structure as `ReviewDisposition::Invalid` would remain
fail-closed for completion, but it violates the churn-safe requirement and
drops the ownership signal needed to park safely.

Accepting blank asks, steps, or checks would maintain types but not actionable
content. The existing reason rule already establishes non-whitespace content as
the repository's minimum string validity boundary.

Enforcing “one sentence” at parse time would require a language heuristic that
will reject valid commands or accept unclear prose. This ticket's parser can
enforce data shape and non-emptiness; authoring guidance and review own language
quality.

### Rationale

Atomic fallback is deterministic and fail-closed at the policy level. No
downstream consumer needs to determine which fragments of malformed input are
safe to believe. Dropping `check` on fallback is especially important because
later code must not run a command that arrived in a malformed contract.

## Decision 5: parsing stores check bytes and never evaluates them

The parser treats `check` exactly like inert string data. It performs only JSON
type and non-whitespace validation.

It will not invoke a shell, split arguments, expand environment variables,
inspect the filesystem referenced by the string, or attempt to classify the
command as read-only.

A regression test will use check content that would create a sentinel if run,
parse the document, assert the content is preserved, and assert the sentinel is
absent. The sentinel path will be embedded as data without passing the content
to a process API.

Command read-only enforcement and timeout behavior belong to the later check
execution ticket.

## Decision 6: retain fail-closed outer-document rules

The new fallback begins only after the parser has established a valid block
relationship:

- the document is a JSON object;
- `disposition` exists and is the string `block`;
- `reason` exists and is a non-empty, non-whitespace string.

Malformed JSON, absent or invalid disposition, absent or invalid reason,
contradictory pass data, and unknown disposition values remain `Invalid`.

Unknown extra object fields remain ignored, matching current additive behavior.

This boundary prevents the fallback from manufacturing an actionable block
when the parser cannot establish that the author actually issued one.

## Decision 7: one typed parking-transition row shape

Add a public `ParkingTransitionType` with `Park` and `Unpark`, serialized as
`park` and `unpark`.

Add `ParkingTransitionRecord` with:

- `schema_version`;
- `record_type: ParkingTransitionType`;
- `ticket_id`;
- `attempt_lease`;
- `remedy_owner`;
- `started_at`;
- `ended_at`;
- `wall_clock_secs`.

Use the existing UTC epoch-second and saturating-duration conventions. The
future writer can represent a park observation as a zero-duration interval and
an unpark as the closed parked interval, or populate both rows from known park
state. This ticket defines storage, not scheduler timestamp policy.

### Alternatives

Two structs, `ParkRecord` and `UnparkRecord`, could give each transition a
distinct field set. Querying stranded time would then require joining a
point-in-time park row to an unpark row and deciding how to handle repeated or
missing transitions.

Adding `Park` and `Unpark` to the existing `ProvenanceRecordType` would reuse
one enum, but `AssignmentTransitionRecord` could then deserialize with a
semantically invalid record type. A dedicated enum makes the record's
discriminator closed over exactly the variants its struct supports.

A single `record_type: parking-transition` plus separate `transition` field
would add another field without improving replay. The values `park` and
`unpark` are already unambiguous ledger row types.

Point timestamps such as only `parked_at` or `unparked_at` are compact, but they
do not independently carry a queryable duration. Existing provenance already
uses interval triples, so matching that convention reduces reader complexity.

### Rationale

One shape is easy for the next scheduler ticket to append and for ledger
readers to replay. It carries all acceptance fields in both transitions and
keeps duration directly queryable without reconstructing pairs.

## Decision 8: add a third untagged ledger variant

Extend `ProvenanceLedgerRecord` with
`ParkingTransition(ParkingTransitionRecord)`.

Place it before `Execution`, alongside the explicitly discriminated assignment
variant. Its required `record_type` and `remedy_owner` distinguish it from old
execution records; its record-type enum distinguishes it from assignment rows.

Update exhaustive readers to ignore the new variant where their domain remains
execution-only or assignment-only. No current production reader will treat a
park row as terminal execution evidence.

## Decision 9: bump provenance schema version to 4

Adding a new durable ledger row shape is a schema evolution. Increment
`SCHEMA_VERSION` from 3 to 4 for newly created records.

Do not rewrite historical rows. Tests will retain explicit schema-v2 execution
JSON and add/retain schema-v3 assignment evidence so mixed replay proves old
rows remain valid.

Existing structs do not reject historical numeric schema versions during
deserialization. New writers use 4; readers can branch on each row's value.

### Alternative

Keeping version 3 because the existing execution struct is unchanged would
make the new parking row indistinguishable at the version-policy level from the
assignment-only schema generation. The module documentation explicitly says to
bump when record shape changes, and the prior new row type established that
precedent.

## Decision 10: share the append implementation

Add `append_parking_transition_record`, parallel to `append_record` and
`append_assignment_transition_record`.

Delegate to the existing generic `append_serialized` helper. This preserves
true append, compact one-row JSON, parent creation, and error behavior.

No generic public append function will be exposed. Typed entry points prevent
callers from appending arbitrary JSON values through the core schema owner.

## Testing design

Disposition tests will assert:

- each owner parses to its enum variant;
- missing optional fields become `None`;
- valid steps and check are retained exactly;
- legacy reason bytes remain exact;
- missing owner or ask falls back;
- unknown owner, blank ask, malformed steps, and malformed check fall back;
- fallback discards all structure and is operator-owned/unstructured;
- outer invalid documents remain invalid;
- hostile-looking check content is not executed.

Provenance tests will assert:

- park and unpark serialize as compact schema-v4 rows;
- both rows round-trip;
- append produces complete newline-terminated rows without rewriting;
- mixed schema-v2 execution, schema-v3 assignment, schema-v4 park, and
  schema-v4 unpark rows replay to the correct enum variants;
- owner classification and attempt attribution survive replay;
- timestamps and duration survive replay exactly.

Core and plugin tests will catch public enum-constructor and match updates. The
workspace suite is the final no-behavior-change check for completion and
scheduling.

## Resulting boundary

After this ticket, core can represent structured and legacy-safe Review blocks
and can persist parking transitions. It still does not decide whether to retry,
park, unpark, render an ask, or execute a check. Those remain explicit consumers
in dependent tickets.
