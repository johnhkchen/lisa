# Design: structured completion rejection activity

## Goal

Preserve the five named completion rejection outcomes as typed activity facts
from the adapter through UI projection and rendering. Attach one stable
completion-generation correlation to every fact and keep it visible in both
the full Activity feed and the Operations alert feed.

Do not change reducer policy, add reconciliation behavior, or introduce a
second completion launch path.

## Option 1: formatted generic warnings

The smallest code change would keep `ActivityEvent::Warning` and standardize
messages such as `completion stale-lease correlation=...`.

This is compatible with the current activity enum and renderers. It would also
avoid core type changes.

It is rejected because the activity and UI state would still classify every
rejection as the same generic Warning. Tests would need to parse strings to
know which rejection occurred. Current Warning truncation can remove the
correlation. This does not satisfy the requirement that no rejection collapse
to a generic boolean failure.

## Option 2: five independent event variants

`ActivityEvent` and `ActivityType` could each gain one top-level variant for
every named rejection.

This provides strong exhaustiveness and direct matching. Each renderer could
select independent labels and colors.

It is rejected because every shared field would be repeated five times and
each match would repeat nearly identical rendering logic. Adding a sixth
operator-visible rejection would require more top-level match arms even though
the data shape is unchanged.

## Option 3: structured event with typed kind

Add a serializable `CompletionRejectionKind` enum to the shared activity types
with exactly the five ticket-owned categories. Add one
`ActivityEvent::CompletionRejected` carrying ticket ID, kind, correlation, and
detail. Add a corresponding UI activity variant with the same fields.

This is the chosen design. The nested kind is structurally matchable and
exhaustive while common identity fields remain centralized. It keeps the
activity boundary independent of non-serializable reducer internals.

## Shared activity vocabulary

`CompletionRejectionKind` uses variants `AlreadyPending`, `StaleLease`,
`DispositionBlocked`, `DependencyBlocked`, and `LaunchFailed`.

Its Display representation uses the acceptance-language labels:
`already-pending`, `stale-lease`, `disposition-blocked`,
`dependency-blocked`, and `launch-failed`.

`ActivityEvent::CompletionRejected` carries:

- `ticket_id: TicketId` for ownership;
- `kind: CompletionRejectionKind` for categorical identity;
- `correlation_id: String` for command/obligation attribution;
- `detail: String` for actionable evidence.

String correlation is deliberate. `ActivityEvent` is serializable while the
core reducer's opaque ID newtypes are not. The activity fact is a presentation
and snapshot boundary, so retaining the lossless stable Display value is
appropriate.

## Correlation derivation

Use the existing `CompletionGenerationId` as the adapter-visible correlation.
It binds completion ticket, attempt authority, and generation 1, matching the
identity already supplied to `complete-ticket`.

The dispatcher can derive this before Review admission or reduction because it
already normalizes ticket and AttemptId. The executor derives the same value
from the reducer-returned effect. No random number, global counter, or second
identity registry is introduced.

For missing authority, the adapter's existing `missing-authority` AttemptId
continues to produce a deterministic correlation. Missing authority is not one
of the five named variants, so its generic rejection behavior need not be
reclassified in this ticket.

## Rejection projection

Add one helper that accepts ticket, completion-generation correlation, and a
borrowed `CompletionRejection`. It matches the five named core variants and
logs the structured activity event with variant-specific detail.

`AlreadyPending` detail identifies the owning completion.
`StaleLease` detail identifies the rejected attempt.
`DispositionBlocked` and `DependencyBlocked` retain their reason strings.
`LaunchFailed` retains the source message rather than the generic outer error.

UnexpectedEvent and CorrelationMismatch remain generic warnings in this slice.
They are real reducer variants but are not in the ticket's explicit named set.
The helper remains exhaustive by routing them through the existing generic
warning surface with their Display text.

## Adapter gates

The reducer already emits `AlreadyPending`, so the dispatch error arm will use
the structured helper instead of always logging Warning.

Current-lease validation in the executor will construct
`CompletionRejection::StaleLease` for an attempt authority that is not current
and project it through the helper. Operator misuse and missing authority remain
generic because they are not stale attempt leases.

Dependency validation will construct `DependencyBlocked` with the existing
actionable reason and log it structurally.

Completion command construction failure will construct `LaunchFailed` with a
`LaunchFailure` source and log it structurally in production. A small pure
helper or test-visible projection test will cover launch-failed without
depending on WASM host I/O.

Review admission will return a typed rejection instead of logging internally.
Missing, unreadable, explicit block, and invalid Review disposition all map to
`DispositionBlocked`, because each means durable disposition does not authorize
completion. The dispatcher supplies the already-derived correlation and logs
the structured result once.

If Review admission proves the lease stale before parsing disposition, the
adapter should use `StaleLease`; otherwise admission errors remain disposition
blocked. Existing `admit_artifact` authority validation can be classified at
the dispatcher boundary from the source lease and current lease.

## UI projection and rendering

`activity_event_to_ui_entry` maps the shared structured event to a dedicated
`ui::ActivityType::CompletionRejected`. It preserves all four fields rather
than formatting early.

The full Activity renderer gives the type an alert icon/color and emits the
ticket, exact kind label, full correlation, and detail. It does not apply the
generic 40-character truncation to this identity-bearing entry.

The Operations filter includes `CompletionRejected` alongside Warning, Error,
and PhaseCompleted. Its renderer uses the same stable label and full
correlation. This makes rejected completion obligations visible on the default
operator dashboard as well as the dedicated log.

The renderer can centralize message formatting in a small helper to avoid the
two views drifting. Both views remain independently tested.

## Compatibility

Existing message-substring tests for named rejection paths will migrate to
structured pattern matching. Tests for unrelated generic errors and warnings
remain unchanged.

The new ActivityEvent variant requires exhaustive additions to snapshot
formatting and UI conversion. Serde derives remain valid because every new
field is serializable.

No pending map, command argv, result lookup, lease mutation, scheduler release,
or completion publication behavior changes.

## Test design

Add a projection test covering all five core rejection values. For each case,
invoke the projection helper with a distinct correlation and assert the logged
ActivityEvent carries the exact kind and identity.

Add or extend adapter behavior tests for already-pending, stale lease,
disposition blocked, and dependency blocked so production gates are proven to
use the structured event rather than generic Warning/Error.

Add a UI/activity test containing all five structured events. Convert them to
UI entries, assert every entry uses `CompletionRejected`, render Activity and
Operations views, and assert all five stable labels and all five correlations
are present in both outputs.

Add the structured event to textual snapshot formatting tests. This pins the
non-dashboard state surface as well.

## Rejected shortcuts

Do not modify `completion.rs` or add serde derives to its reducer types.
Do not treat correlation as the ticket ID alone when a stable attempt-bound
generation already exists.
Do not infer rejection kind from Display prose.
Do not truncate correlation-bearing rejection entries.
Do not add scheduling consequences to activity logging.
