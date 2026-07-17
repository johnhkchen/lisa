# Plan — T-049-06-01

## 1. Establish the validated note class

Add `DispositionNote` to the core disposition module.
Validate each required string as nonblank without trimming stored content.
Add the Note enum variant and completion-authorization helper.
Extend document validation using `reason: null` and three required named fields.
Keep pass and block branches structurally unchanged.

Verification:

- valid note parses to its own variant;
- missing or blank criterion quote is Invalid;
- missing or blank evidence citation is Invalid;
- missing or blank summary is Invalid;
- non-null note reason is Invalid;
- pass-with-reason and unknown shapes remain Invalid;
- existing pass and block tests remain green.

## 2. Authorize note in pure reconciliation

Use the new authorization helper in `completion::reconcile`.
Add a test using the T-046 criterion/evidence text.
Assert the emitted effect equals the pass effect.
Retain explicit block and invalid ineligibility tests.

Verification:

- run lisa-core disposition tests;
- run lisa-core completion tests;
- run lisa-core integration state-machine tests.

## 3. Carry admitted note through plugin completion

Return note metadata from the Review admission functions.
Store it in pending completion state.
Ensure Reconcile and artifact-driven inputs capture it after lease admission.
Set None for operator paths that do not originate from a note disposition.
Update Review timeout/exhaustive match logic so Note behaves as completed Review.
Do not broaden any `Block`-specific policy.

Verification:

- compilation finds every exhaustive disposition match;
- existing block parking tests still pass;
- existing pass completion tests still pass.

## 4. Persist note in the completion journal

Extend Confirmed transition/record with optional note.
Bump journal schema version.
Use serde default and skip-when-none.
Retain the value in folded aggregate state.
Thread the pending note into successful confirmation.
Add round-trip and old-row compatibility coverage.

Verification:

- pass rows deserialize with None;
- note confirmation serializes the exact criterion, path, and summary;
- journal reload reconstructs the note;
- both seal receipt variants retain their existing validation.

## 5. Persist note in provenance

Add optional completion note to terminal execution records.
Set the field from successful completion pending state.
Set None in unrelated execution construction paths.
Update serialization and backward-compatibility tests.

Verification:

- an old provenance fixture parses;
- a note-bearing Done row round-trips;
- ordinary rows omit the field.

## 6. Add end-to-end completion regressions

Build a fixture from `T-046-06-03`:

- criterion quote: the approximately 200 MiB gate;
- evidence citation: the Codex closing run record path;
- summary: the measured result supports completion while the written gate is stale.

Exercise commit and journal seal paths using existing State test harnesses.
For each tier assert:

- exactly one ticket reaches Done;
- the dependent becomes schedulable;
- the confirmed journal row carries the note;
- terminal execution provenance carries the same note;
- the retained seal equals the configured tier;
- no ParkingTransition row is emitted.

Use the real commit-result harness for commit tier if available.
Use repo-less immediate journal completion for journal tier.

## 7. Quality checks

Run `cargo fmt --all -- --check` after formatting.
Run targeted lisa-core tests first for fast feedback.
Run targeted lisa-plugin note tests.
Run `cargo test --workspace` for regression coverage.
Run `just check` if the environment has the WASM target and time permits.
Inspect `git diff --check` and exact ticket-owned diffs.

## 8. Commit meaningful units

Commit the core parser/reconciliation unit with exact includes.
Commit persistence/plugin integration with exact includes.
Do not use ordinary staging or commit commands.
Confirm ticket-owned source paths are clean after Lisa transactions.
Record commands, outcomes, and deviations in `progress.md`.

## 9. Review

Write `review.md` with changed files, behavior, coverage, and concerns.
Write the exact passing disposition JSON only when all required checks pass and ticket-owned source files are clean.
Remain on this ticket after artifacts are complete.

