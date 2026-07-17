# Design — T-049-01-02 seal visibility and ledger field

## Decision summary

Use the existing `CompletionSeal` as the single value for screen and storage.

Define the human-readable line once in the CLI completion-seal module.

Have doctor and status resolve their observational tier from project context.

Stamp the plugin's pinned `PluginConfig.completion_seal` on all new audit rows.

Deserialize a missing `seal` through `CompletionSeal::default()` as commit.

Retain journal seals through replay and aggregate reconstruction.

Do not bump schema versions for this additive backward-compatible field.

## Shared visibility copy

Add `visibility_line(CompletionSeal) -> &'static str` in `completion_seal.rs`.

The commit variant returns the exact ticket language:

`completion seal: commit-sealed — finished work lands as history`

The journal variant returns:

`completion seal: journal-only — finished work is recorded but not undoable`

One exhaustive match makes wording drift between doctor and status impossible.

The returned journal string will have an explicit regression against `git`.

Doctor appends the line near project diagnostics.

Status prints the line beside its general configuration summary.

## Observational resolution

Loop startup already has strict `resolve_for_run` behavior.

Doctor and status need the same auto environment selection without changing it.

Add an inspection helper that returns the tier in effect for display.

For configured journal it returns journal without probing.

For configured commit it returns commit even when doctor will diagnose support.

For auto it performs the existing read-only probe and returns its selected tier.

This preserves doctor's full missing-Git dependency report.

It also avoids falsely presenting an unavailable explicit commit as journal.

The helper does not expose `auto` because every result is a `CompletionSeal`.

### Alternative: call strict run resolution directly

Rejected for doctor because explicit commit failure would abort before the
dependency report and its installation guidance are assembled.

### Alternative: display configured mode

Rejected because auto is not a seal and does not tell an operator which tier
will bind the run in the current environment.

### Alternative: cache the last loop resolution on disk

Rejected because no such state exists, it can become stale, and both commands
already have enough context for the existing read-only probe.

## Provenance field placement

Add `pub seal: CompletionSeal` to every concrete provenance row structure.

Annotate each with `#[serde(default)]`.

The field is serialized normally on all new writes.

The default is commit, so old rows deserialize as pre-ladder commit history.

This applies uniformly to execution, assignment, and parking evidence.

Keeping the field on each concrete structure makes it available after the
untagged enum selects a row shape.

Plugin constructors use `self.config.completion_seal`.

Core and plugin fixtures set an explicit seal to test new-row behavior.

The literal schema-v2 execution JSON remains unchanged to prove compatibility.

Add literal legacy assignment and parking parse checks as well.

### Alternative: place seal only on successful execution rows

Rejected because the ticket says every provenance record, and failures or
parking evidence occurred under the same completion contract as successes.

### Alternative: infer seal from schema version in readers

Rejected because new commit rows and old pre-ladder rows may share versions;
the additive field is the requested durable evidence.

## Completion-journal field placement

Put `seal` on the common `JournalRecord` envelope.

Annotate it with `#[serde(default)]`.

This stamps every state row without duplicating the field in four variants.

Change `append` to accept a `CompletionSeal` supplied by the plugin adapter.

`JournalRecord::from_transition` records that seal.

`JournalRecord::into_transition` returns the seal with the typed transition.

`apply_transition` receives both values.

Add `seal` to `CompletionJournalAggregate` so reconstruction remains explicit.

On the first requested row, the aggregate adopts the row's seal.

Subsequent rows must match the aggregate seal.

A mismatch fails closed as invalid journal history.

A new generation after a terminal aggregate may use the newly supplied seal.

This matters if configuration changes between loop runs.

Legacy rows all default to commit and therefore remain internally consistent.

Expose an aggregate accessor for tests and future reconciliation consumers.

### Alternative: deserialize seal but discard it during replay

Rejected because that would parse old rows but would not classify reconstructed
history as the acceptance criteria require.

### Alternative: add seal to every transition enum variant

Rejected because it repeats a common invariant at every call site and makes it
easy for adjacent transitions in one generation to accidentally differ.

### Alternative: trust the requested row and ignore later row seals

Rejected because auditors should not accept a mixed-tier completion sequence.

## Schema versioning

Keep provenance `SCHEMA_VERSION` at 5 and journal schema version at 1.

The field is additive and has an unambiguous default.

Bumping would suggest old readers must branch, while Serde-compatible readers
can consume both shapes directly.

Documentation will state that absent means pre-ladder commit-sealed history.

## Test design

CLI tests cover exact copy for both enum variants.

Doctor and status module fixtures each assert that their output assembly includes
the shared line for commit and journal.

The doctor fixture additionally asserts journal wording contains no `git`.

Core provenance tests assert new serialization carries `"seal"`.

They deserialize old execution, assignment, and parking rows without the field.

Each old row must expose `CompletionSeal::Commit` after parsing.

Completion-journal tests assert newly appended rows all carry the selected seal.

Legacy literal rows must load and reconstruct with a commit aggregate seal.

A mixed seal within one aggregate must fail closed.

Existing workspace tests remain the broad regression boundary.

## Documentation

Update the provenance ledger field table and examples with `seal`.

Correct the document's current schema-version statement while touching it.

Explain that missing fields are classified as commit-sealed pre-ladder rows.

## Commit boundaries

First commit core provenance schema and documentation.

Second commit plugin journal persistence and all plugin writer propagation.

Third commit CLI doctor/status visibility and their fixtures.

Each unit uses exact repository-relative includes through `lisa commit-ticket`.
