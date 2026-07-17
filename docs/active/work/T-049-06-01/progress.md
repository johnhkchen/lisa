# Progress — T-049-06-01

## Outcome

Implementation is complete.
The note disposition is a strict third Review class.
It authorizes completion like pass, never enters block parking, uses the configured seal unchanged, and is retained in completion journal and provenance records.

## Completed: core disposition

Added `DispositionNote` in `crates/lisa-core/src/disposition.rs`.
The value retains:

- `criterion_quote`;
- `evidence_citation`;
- `summary`.

All fields must be nonblank strings.
The parser preserves decoded string content without trimming.
The JSON shape requires `disposition: "note"` and `reason: null`.
Unknown note fields are rejected, which prevents a generic `work_complaint` escape hatch.
Missing criterion, evidence, or summary fields parse Invalid.
Non-null note reason parses Invalid.
Strict pass and existing block behavior remain unchanged.

Added `ReviewDisposition::authorizes_completion`.
Only Pass and Note return true.
Block and Invalid remain false.

## Completed: pure completion

Changed `completion::reconcile` to use the narrow authorization method.
Added a T-046-derived note fixture.
The test proves note emits the same completion effect as pass.
Existing tests continue to prove block and invalid dispositions are ineligible.
No seal-selection or receipt behavior changed.

## Completed: adapter flow

The plugin now returns note metadata from current-lease Review admission.
Event-driven and level-triggered completion paths store the admitted note in `PendingCompletion`.
Operator completion of a canonical note also retains its metadata.
Review timeout suppression accepts Note as a complete Review verdict.
Review protocol rendering treats Note as complete rather than blocked.
Block-only parking patterns were left narrow and therefore do not match Note.

## Completed: completion journal

Advanced the journal writer schema from 4 to 5 while retaining readers for versions 1 through 5.
Requested and Confirmed records carry an optional `note`.
Old records default the field to None.
Pass rows omit it.
The in-memory aggregate retains the admitted note.
Confirmation rejects a note that differs from the note admitted with Requested.
Restart replay reconstructs the exact admitted note from the journal rather than rereading mutable Review text.

## Completed: provenance

Added optional `completion_note` to terminal execution provenance.
Historical ledger records default to None.
Ordinary pass/failure/timeout rows omit the field.
Successful note completion writes the same validated value retained by the pending generation.
No separate best-effort sidecar record is used.

## Completed: field regressions

Re-expressed the T-046-06-03 approximately-200-MiB versus 225-MiB measurement dispute as a note fixture.
The evidence citation points to the preserved Codex closing run record.

Commit-seal regression proves:

- one completion effect;
- one Confirmed journal transition;
- one authoritative Done provenance execution row;
- note data retained in the aggregate and provenance;
- dependent scheduling continues;
- no repeated completion on duplicate result.

Journal-seal regression proves:

- repository-less completion reaches Done;
- the dependent becomes dependency-satisfied;
- the receipt carries final ticket and artifact hashes;
- journal rows carry the note;
- terminal provenance carries the same note and journal seal;
- the mixed ledger contains no ParkingTransition.

## Deviation from plan

The design initially placed note data only on Confirmed.
During implementation, restart replay exposed a durability gap: an in-flight completion could otherwise lose the admitted note before confirmation.
The journal therefore stores the optional note on both Requested and Confirmed and verifies equality.
This is narrower and safer than reparsing the mutable artifact after a lost result.
No other implementation deviation was required.

## Verification completed

- `cargo test -p lisa-core` — passed: 232 unit tests plus integration regressions.
- `cargo test -p lisa-plugin review_disposition_gates_artifact_completion_and_dependents --lib` — passed.
- `cargo test -p lisa-plugin completion_journal::tests --lib` — passed: 15 tests.
- T-046 journal-seal end-to-end regression — passed.
- commit-seal dependent scheduling regression — passed.
- `cargo test --workspace` — passed.
- `just check` — passed, including `cargo check -p lisa-plugin --target wasm32-wasip1` and the workspace suite.
- `git diff --check` — passed.

## Ticket commits

- `14639915a2daa0d9c15e9d49d2910906fd5c8025` — strict note disposition and pure completion eligibility.
- `e604921a172d9c7d5e7c4103efed10323c4c369a` — journal/provenance persistence and both-tier regressions.

Both commits were created with `lisa commit-ticket` and exact include paths.
No ordinary Git staging or commit command was used.
All ticket-owned source paths are clean.

