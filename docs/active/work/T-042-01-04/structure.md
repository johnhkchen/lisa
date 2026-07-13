# Structure: rejection and correlation activity state

## Modified files

Three source files form one connected change:

- `crates/lisa-core/src/types.rs` defines the serializable activity fact;
- `crates/lisa-plugin/src/lib.rs` projects adapter/reducer refusals into that
  fact and converts it to UI state;
- `crates/lisa-plugin/src/ui.rs` renders the typed state in both dashboard
  activity surfaces.

No production file is created or deleted. No manifest or reducer source file
changes.

## `crates/lisa-core/src/types.rs`

Add public enum `CompletionRejectionKind` immediately before ActivityEvent.
It derives Debug, Clone, Copy, PartialEq, Eq, Serialize, and Deserialize.

Implement Display with stable kebab-case labels matching the ticket:

- AlreadyPending → `already-pending`;
- StaleLease → `stale-lease`;
- DispositionBlocked → `disposition-blocked`;
- DependencyBlocked → `dependency-blocked`;
- LaunchFailed → `launch-failed`.

Add `ActivityEvent::CompletionRejected` near Error/Warning activity variants.
Fields are ticket ID, kind, correlation ID string, and detail string.

The type remains presentation-oriented. It does not import or wrap
`completion::CompletionRejection`, avoiding a serialized dependency on reducer
implementation types.

Add a focused unit test for the stable kind labels if no existing Display test
pattern covers the new enum.

## `crates/lisa-plugin/src/lib.rs`: imports and helper

Import `CompletionRejection`, `LaunchFailure`, and shared
`CompletionRejectionKind` alongside existing completion and activity types.

Add a State helper near completion dispatch:

`log_completion_rejection(ticket_id, correlation, rejection)`.

The correlation parameter uses `CompletionGenerationId` by reference. The
helper converts it to its stable string exactly once.

Match each named rejection to the corresponding shared kind and detail. Log
one `ActivityEvent::CompletionRejected`.

For non-ticket-owned rejection variants, retain an ActivityEvent::Warning with
the correlation included. This keeps the match exhaustive without broadening
the public rejection-kind contract.

Add a small constructor helper for generation-1 correlation if repeated
construction would otherwise drift between dispatcher and executor.

## `crates/lisa-plugin/src/lib.rs`: disposition admission

Change `admit_passing_review` from `bool` with internal logging to a Result
whose error is a `CompletionRejection`.

The success value is unit. Missing disposition, read/admission errors, explicit
block, and invalid disposition return `DispositionBlocked` with the existing
operator-visible reason included.

The dispatcher owns activity emission. It has ticket and correlation context,
so it can project the error once without generic fallback logging.

No other production caller exists, so the signature change stays within the
completion adapter boundary.

## `crates/lisa-plugin/src/lib.rs`: dispatch

Normalize the input as today.

Derive AttemptId and CompletionId before Review admission. Construct the
`CompletionGenerationId` correlation from those values and generation 1.

If the source requires Review admission and its attempt lease is not current,
log a `StaleLease` structured rejection and return false. Otherwise call the
Result-returning disposition helper; project any error and return false.

Construct Request and reduce as today. Replace generic reducer-error Warning
with the structured projection helper. Pass the accepted effect to the same
single executor.

The one-gateway source invariant remains valid: dispatch is still the only
production caller of the executor.

## `crates/lisa-plugin/src/lib.rs`: executor

Construct the completion-generation correlation immediately after extracting
effect IDs, before validation gates.

For a non-current Attempt authority, create `StaleLease` from the effect
attempt and project it. Keep the existing generic authority message for
Operator or missing-authority shapes that do not describe a stale attempt.

For incomplete dependencies, create `DependencyBlocked` with the current
reason and project it.

For command-construction error in production, remove pending state as today,
then create `LaunchFailed(LaunchFailure::new(error))` and project it. Preserve
the native test short circuit unless focused helper tests cover projection
directly.

No host command call, pending insertion order, or result behavior changes.

## `crates/lisa-plugin/src/lib.rs`: snapshot and UI conversion

Extend `format_activity_event` with one stable line format containing ticket,
kind, correlation, and detail.

Extend `activity_event_to_ui_entry` to map the structured shared event into
the dedicated UI type without loss.

Update tests adjacent to these helpers. Prefer structural matches over message
substring parsing for rejection cases.

## `crates/lisa-plugin/src/ui.rs`: activity type

Import shared `CompletionRejectionKind` or use it through an existing
`lisa_core::types` import.

Add `ActivityType::CompletionRejected` with ticket ID, kind, correlation ID,
and detail.

This is internal UI state. It need not derive serde.

## `crates/lisa-plugin/src/ui.rs`: common formatting

Add a private formatter that produces the operator-visible rejection message
from the four structured fields.

The message must include the exact stable kind label and exact correlation.
Detail follows after those identity fields.

Do not route this message through generic Warning/Error truncation helpers.

## `crates/lisa-plugin/src/ui.rs`: full activity rendering

Add a CompletionRejected arm to `render_activity_log`.

Use an alert icon and color consistent with a refused obligation. Render the
common formatted message in full.

## `crates/lisa-plugin/src/ui.rs`: Operations filtering

Include CompletionRejected in `render_filtered_activity_log`'s filter.

Add its render arm and reuse the common formatter. This guarantees the default
alerts-only dashboard retains every rejection entry and correlation.

Update comments describing the filtered set.

## Test organization

In plugin `lib.rs` tests:

- add five-case projection coverage for core rejection → activity kind;
- extend activity conversion coverage for one structured entry;
- extend snapshot formatting coverage;
- migrate stale/disposition/dependency assertions where they exercise named
  rejection paths.

In `ui.rs` tests:

- create one state with all five CompletionRejected entries;
- use distinct correlations and details;
- render the dedicated Activity view and assert all labels and correlations;
- render Operations and assert the same facts remain visible;
- pattern-match entries if needed to prove they are not Error/Warning.

## Change ordering

1. Add shared activity vocabulary.
2. Add UI state and exhaustive render arms so the type has a destination.
3. Add plugin projection and adapter classification.
4. Add and migrate tests.
5. Format, run focused tests, then full verification.

The three source files compile as one atomic unit and should be committed in
one isolated ticket transaction with exact includes.

## Unchanged boundaries

`crates/lisa-core/src/completion.rs` stays unchanged.
`PendingCompletion` shape stays unchanged.
Command context and argv stay unchanged.
Completion result publication stays unchanged.
Reconciliation policy stays unchanged.
Ticket and artifact publication remain Lisa-owned.
