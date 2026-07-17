# Design — T-049-06-01

## Decision summary

Add a reusable `DispositionNote` value and `ReviewDisposition::Note` variant in lisa-core.
Use JSON fields `criterion_quote`, `evidence_citation`, and `summary`, alongside `disposition: "note"` and `reason: null`.
Treat note as completion-authorizing at every pass gate.
Capture the note when completion is dispatched and carry it through pending state to the atomic confirmation row and terminal provenance row.
Keep note independent of `CompletionSeal`, so commit and journal tiers follow the exact same mechanics.

## Schema choices

### Chosen shape

```json
{
  "disposition": "note",
  "reason": null,
  "criterion_quote": "...",
  "evidence_citation": "docs/.../run-record.md",
  "summary": "The recorded measurement supports completion, but the written threshold is stale."
}
```

The common `reason` field remains null for an authorizing disposition.
This preserves the existing top-level parser contract that every document has disposition and reason.
Named fields make the restricted purpose explicit.
There is deliberately no `complaint`, `concern`, or generic note-body field.

All three note strings must be nonblank strings.
The criterion and evidence requirements are acceptance-critical.
Requiring the summary too follows the ticket's declared shape and guarantees downstream operator copy exists.
Content is preserved byte-for-byte after JSON decoding; whitespace is used only to test emptiness.
Evidence citations remain inert strings and are never executed or dereferenced by parsing.

### Rejected: encode the note in `reason`

This would blur authorizing and blocking semantics.
It would also make a note indistinguishable from the existing strict pass-with-reason invalid case.
Downstream consumers would need to parse prose to recover criterion and evidence.

### Rejected: optional metadata on Pass

Turning pass into `Pass { note: Option<_> }` would weaken the explicit third-class requirement.
It would also make bare-pass strictness harder to audit and permit accidental generic annotations.

### Rejected: separate sidecar note file

A sidecar would create cross-file admission and atomicity questions.
The disposition already is the authority artifact, so the restricted note belongs in its validated shape.

## Domain representation

`DispositionNote` is a named serializable value with private fields, a validating constructor, and read-only accessors.
The constructor centralizes nonblank validation for parser, journal replay, and provenance deserialization expectations.
`ReviewDisposition::Note(DispositionNote)` clearly distinguishes it from pass and block.
An `authorizes_completion` helper avoids repeated permissive wildcard matches.
It returns true only for Pass and Note.

## Reconciliation

Core `reconcile` calls `authorizes_completion` rather than checking Pass directly.
No seal or transition behavior branches on note.
This proves the pure domain emits the same `LaunchCompletion` effect for pass and note.
Block and Invalid continue returning no effect.

## Adapter flow

The plugin admission function returns `Option<DispositionNote>` rather than unit.
Pass maps to None; Note maps to Some; Block and Invalid remain rejections.
Dispatch stores that value in `PendingCompletion`.
The note therefore stays correlated to the admitted attempt and generation.
It cannot be reread from mutable disk after the completion transaction succeeds.

Any alternate entry point with operator authority has no Review note and records None.
Current-attempt Review reconciliation obtains the note through the same admission boundary as pass.
Timeout checks regard Note as a completed disposition.
Block-only parking logic remains block-only.

## Journal persistence

Extend only the `Confirmed` transition and row with `note: Option<DispositionNote>`.
Confirmation is the correct durable point because it means Done and the selected seal were verified.
Requested and in-flight rows do not claim completion and need not duplicate note data.
Increment the journal schema version.
Mark the serialized field with `default` and omit it when absent.
That keeps all existing rows readable and keeps ordinary pass rows stable.
The aggregate retains `confirmed_note` for replay and test inspection.

Embedding the note in the confirmation row ensures one sealed ticket produces one note-bearing terminal journal row.
It also shares the journal's atomic append/publish behavior.

## Provenance persistence

Add `completion_note: Option<DispositionNote>` to terminal `ProvenanceRecord` with serde default and skip-when-none.
The plugin's terminal emitter accepts the pending note and writes it on the Done execution row.
Non-completion outcomes and historical records carry None.
No separate ledger row is needed: the note is provenance of the completed execution.

### Rejected: separate provenance note row

A separate row would require a new record type and ordering policy.
It could be written without the terminal row or vice versa.
Embedding it gives consumers a single authoritative completed-ticket record.

## Atomicity and seals

The note does not influence `CompletionSealReceipt`.
Commit and journal seals are still resolved and verified exactly as before.
Both converge on `finish_successful_completion`, which appends the note-bearing confirmation before scheduler release.
If confirmation persistence fails, the existing code keeps scheduler state blocked.
Thus note recording shares completion's current atomic failure boundary.

## Test strategy

Parser tests cover a valid note, each missing required content, strict pass, unknown shapes, and unchanged block fixtures.
Core completion tests compare pass and note reconciliation effects and keep block/invalid ineligible.
Plugin tests use the preserved T-046 wording as a fixture.
They exercise commit and journal seals, terminal journal/ledger fields, Done state, dependent eligibility, and absence of parks.
Journal unit tests verify backward compatibility and note round trips.
Existing workspace tests guard exhaustive match changes and unchanged pass/block behavior.

