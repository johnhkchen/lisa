# Review — T-049-06-01

## Summary

This change adds the requested done-with-a-note severity class without lowering the completion guard.
A note can only be expressed through explicit criteria/evidence fields.
It completes exactly like pass under the already-pinned commit or journal seal.
It remains distinct from block throughout parsing, reconciliation, scheduling, journaling, and provenance.

## Source changes

### `crates/lisa-core/src/disposition.rs`

Added validated `DispositionNote` and `ReviewDisposition::Note`.
Added completion authorization vocabulary.
Added strict parser coverage for required fields, null reason, unknown fields, and byte preservation.
Existing pass/block fixtures remain intact.

### `crates/lisa-core/src/completion.rs`

Changed level-triggered eligibility from Pass-only matching to the explicit Pass-or-Note domain predicate.
Added equivalence coverage using the T-046 field dispute.

### `crates/lisa-core/src/provenance.rs`

Added backward-compatible optional `completion_note` to execution rows.
Added note and legacy-row serialization coverage.

### `crates/lisa-plugin/src/completion_journal.rs`

Advanced the current schema to version 5.
Requested and Confirmed rows now carry optional note metadata.
Aggregate replay retains the note.
Confirmation refuses note mutation within a completion generation.
Legacy journal rows remain readable with None.

### `crates/lisa-plugin/src/lib.rs`

Threaded admitted note metadata through every Review completion entry point and pending transaction.
Kept Note out of block/park policy.
Recorded note metadata on journal confirmation and authoritative terminal provenance.
Extended commit-tier and journal-tier end-to-end fixtures.

### `crates/lisa-plugin/src/ownership.rs`

Updated a test provenance constructor for the new optional field.

## Acceptance assessment

Parser acceptance is satisfied.
A well-formed note parses to its own variant.
Missing criterion, missing evidence, missing summary, non-null reason, and generic work-complaint fields are Invalid.
Pass-with-reason and unknown dispositions remain Invalid.
Existing pass/block tests pass unchanged.

Completion acceptance is satisfied.
Core reconciliation produces the same effect for pass and note.
Commit and journal end-to-end paths both preserve their configured seal.
Both reach one confirmed Done state and allow dependent scheduling.
Journal and provenance records carry the note.
The journal-tier mixed ledger explicitly contains zero parks.
The commit-tier boundary retains one authoritative completion record and schedules the successor without a park transition.
The fixture uses the preserved T-046-06-03 approximately-200-MiB criterion and 225-MiB evidence path.

## Test coverage

The core suite covers constructor/parser behavior, pure reconciliation, serialization compatibility, and all existing DAG/domain behavior.
Completion-journal tests cover version compatibility, request/confirmation note stability, seal receipts, corruption, and restart reconstruction.
Plugin tests cover admission, duplicate suppression, both seal tiers, terminal provenance, dependency release, and block-policy non-regression.
The entire workspace test suite passes.
The WASM target check passes.

## Atomicity review

The note is admitted only after the current attempt artifact passes the lease boundary.
It is recorded on the Requested generation before command launch and retained across restart.
Confirmed must carry the identical value.
Scheduler release still occurs only after seal evidence, durable Done verification, and durable confirmation.
If journal confirmation fails, existing behavior keeps scheduler state blocked.
No note-specific side effect can independently release a ticket.

## Compatibility review

Old completion journal schemas 1–4 still load.
Old provenance rows default to no note.
Pass journal/provenance JSON omits optional note fields.
Completion seal receipt formats are unchanged.
Block fallback and parking records are unchanged.
The note parser rejects extra fields only for the new class, so it does not alter legacy pass/block tolerance.

## Open concerns

No blocking concern remains.
The future Notes-for-you queue is intentionally outside this mechanism ticket; durable journal and provenance data now provide its input.
This parser validates shape and nonblank content but does not dereference the evidence path or compare the quote to ticket text.
That is appropriate for a fail-closed schema parser and leaves semantic authoring checks to the later disposition-check workflow.

## Worktree and commit review

Ticket source changes are committed in two Lisa-isolated commits.
No ticket-owned source file remains modified, staged, or untracked.
Remaining worktree changes belong to Lisa runtime publication and another active ticket.
The ticket frontmatter was not manually changed.
Phase artifacts were written only to this attempt's private work directory.

## Disposition

Pass.
The implementation meets both acceptance criteria and is ready for Lisa's completion transaction.

