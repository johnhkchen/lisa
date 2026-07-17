# Structure — T-049-06-01

## Modified files

### `crates/lisa-core/src/disposition.rs`

Add public `DispositionNote`.
Fields: criterion quote, evidence citation, summary.
Expose a validating constructor and immutable accessors.
Derive serde traits so the validated value can be embedded in durable rows.
Add `ReviewDisposition::Note`.
Add `ReviewDisposition::authorizes_completion`.
Extend `validate_document` with the strict note branch.
Keep pass and block branches behaviorally unchanged.
Add focused parser tests using the existing temporary-file helper.

### `crates/lisa-core/src/completion.rs`

Replace the Pass-only reconciliation predicate with the disposition authorization method.
Add a note reconciliation test beside existing disposition eligibility tests.
Do not modify seal resolution, receipt types, reducer transitions, or rejection types.

### `crates/lisa-core/src/provenance.rs`

Import `DispositionNote`.
Add optional `completion_note` to `ProvenanceRecord`.
Use serde default for old rows and omit None during serialization.
Update record constructors in unit tests.
Add or extend serialization coverage for a note-bearing terminal record.

### `crates/lisa-plugin/src/completion_journal.rs`

Import `DispositionNote`.
Bump the current schema version while retaining the existing minimum.
Add optional note data to the Confirmed transition and durable row.
Retain it in `CompletionJournalAggregate` as `confirmed_note`.
Pass it through transition conversion and fold application.
Keep None as the default for historical records.
Add test-only accessor and note round-trip assertions.

### `crates/lisa-plugin/src/lib.rs`

Import `DispositionNote`.
Change Review admission to return the admitted note metadata.
Make Note accepted anywhere Pass indicates a finished Review.
Keep Block-only parking and human-wait logic unchanged.
Add optional completion note to `PendingCompletion`.
Pass it to journal confirmation and terminal provenance emission.
Update all pending constructors and provenance record constructors with None where no note exists.
Add a fixture helper for the T-046 criteria/evidence discrepancy.
Add completion-path tests for both seal tiers.

### Core integration tests

Update explicit exhaustive `ReviewDisposition` construction only if compilation requires it.
The state-machine model can continue modeling authorizing versus blocking behavior; add Note if useful to ensure equivalence.

## No new files or modules

The note is small shared domain vocabulary and belongs beside disposition parsing.
It does not justify a new module.
No CLI or UI surface is introduced in this ticket.
No ticket or shared work artifact is edited by the agent.

## Data flow

1. The attempt writes `review-disposition.json`.
2. The parser validates it into `ReviewDisposition::Note(DispositionNote)`.
3. Core reconciliation treats the variant as eligible.
4. Plugin lease admission returns the validated note.
5. Dispatch stores the note in the pending completion generation.
6. Existing commit or journal seal mechanics produce a receipt.
7. Successful completion appends one Confirmed row containing the note.
8. Terminal execution provenance contains the same note.
9. DAG rebuild observes Done and schedules dependents.
10. No parking transition is emitted.

## Compatibility rules

Existing JSON pass shape remains exactly accepted.
Pass with a string reason remains Invalid.
Existing block documents retain their structured or legacy fallback behavior.
Unknown dispositions remain Invalid.
Old completion journal rows deserialize with note None.
Old provenance rows deserialize with completion note None.
Pass confirmation rows omit the new field.
Seal receipt serialization is unchanged.

## Ownership and commit boundaries

Unit one owns `disposition.rs` and `completion.rs`: validated class plus pure authorization.
Unit two owns `completion_journal.rs`, `provenance.rs`, and the plugin adapter integration.
If compilation requires mechanical constructor updates in other exact paths, include them with the owning unit.
Each unit is formatted, tested, and committed with exact repository-relative includes through `lisa commit-ticket`.
The pre-existing modification in `crates/lisa-plugin/src/lib.rs` must be preserved; the isolated transaction includes the final file only because this ticket also necessarily modifies it.

