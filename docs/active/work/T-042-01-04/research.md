# Research: rejection and correlation activity state

## Assignment boundary

T-042-01-04 belongs to story S-042-01, the plugin completion effect-adapter
story. Its single acceptance criterion is an observability requirement: the
five named completion rejection outcomes must remain distinct through activity
state and dashboard rendering, and each rendered outcome must carry a
correlation identity.

The named outcomes are already-pending, stale-lease, disposition-blocked,
dependency-blocked, and launch-failed. The ticket does not ask for new
completion policy, retry policy, durable journaling, or reducer behavior.

The sibling T-042-01-03 owns level-triggered reconciliation. T-042-01-07 owns
Review timeout suppression based on visible pending or rejected state. This
ticket therefore needs to leave a structured, reusable activity surface rather
than implementing either sibling's scheduling decisions.

## Core completion vocabulary

`crates/lisa-core/src/completion.rs` contains the pure completion aggregate.
The plugin imports its `reduce` function as `reduce_completion` and consumes
its typed events, states, effects, identifiers, and rejections.

`CompletionRejection` is an enum, not a boolean. Its variants include the five
outcomes named by this ticket plus `UnexpectedEvent` and
`CorrelationMismatch`. The named five carry different evidence:

- `AlreadyPending` carries the `CompletionId` already owning the aggregate;
- `StaleLease` carries the stale `AttemptId`;
- `DispositionBlocked` carries operator-visible disposition detail;
- `DependencyBlocked` carries dependency detail;
- `LaunchFailed` owns a `LaunchFailure` source.

The enum implements `Display` through `thiserror`. Its prose is useful to an
operator, but matching or classifying by formatted prose would discard the
typed domain distinction that already exists.

The reducer returns `AlreadyPending` for a Request applied to Requested,
CommandInFlight, or Confirmed. A rejected action-required state returns its
stored rejection when requested again. A launch-failed transition is created
when Requested consumes `CommandLaunchFailed` or CommandInFlight consumes a
matching failed result.

`CompletionState::CommandInFlight` contains a mandatory `CorrelationId`.
`CorrelationMismatch` preserves expected and actual IDs. The plugin adapter
does not currently retain the reducer's full lifecycle state, so it does not
currently surface these state transitions directly.

## Completion generation identity

The same core module defines `CompletionGenerationId`. It binds a
`CompletionId`, authoritative `AttemptId`, and generation number. Its stable
Display format is ASCII and intended for durable command attribution.

The current plugin creates generation 1 immediately before completion command
validation and launch. It passes the component values to `complete-ticket` as
`--ticket-id`, `--attempt-id`, and `--completion-generation`.

The generated identity is stable for a given ticket, attempt, and generation.
It is therefore available before host I/O and remains meaningful for launch
failure, pending collision, admission failure, or dependency refusal.

The Zellij command-result context currently retains only `lisa_completion`
with the ticket ID. `handle_completion_result` looks pending state up by ticket.
The pending transaction struct retains prior phase/status, diagnostic source,
and authority, but not a separate correlation field.

## Plugin completion adapter

`crates/lisa-plugin/src/lib.rs` owns `CompletionInput`, `CompletionSource`,
`CompletionAuthority`, `PendingCompletion`, `dispatch_completion`, and
`execute_completion_effect`.

Every production completion origin now constructs a `CompletionInput` and
passes it through `dispatch_completion`. The source variants are Artifact,
Stopped, Idle, ObservedDone, and Manual. This was established by predecessor
T-042-01-02.

The dispatcher normalizes source evidence into ticket ID, source, authority,
and optional Review lease. Artifact, Stopped, and Idle call
`admit_passing_review`; ObservedDone and Manual do not.

The dispatcher maps `pending_completions.contains_key(ticket)` to Requested;
otherwise it supplies Eligible. It then creates a typed Request and calls the
core reducer. A reducer error currently becomes `ActivityEvent::Warning` with
a formatted string and a false return.

The returned launch effect is the only route to `execute_completion_effect`.
The executor independently validates effect identity, current attempt or
operator authority, dependency completion, ticket path, and command
construction before issuing host I/O.

Several executor refusals correspond semantically to core rejection variants,
but are not currently represented that way. A stale attempt becomes a generic
Warning. Incomplete dependencies become a generic Error. Command construction
failure becomes a generic Error in production.

`admit_passing_review` also logs directly. Missing disposition, admission
failure, explicit block disposition, and invalid disposition all become
generic Error or Warning events. Explicit and malformed blocks do not survive
as a structured disposition-blocked outcome.

Native tests short-circuit after recording an effect when command construction
fails. This lets adapter tests avoid real Zellij host calls, but it means a
launch-failure test needs a deliberate testable boundary rather than relying
on the existing native short circuit.

## Current activity state

`crates/lisa-core/src/types.rs` defines `ActivityEvent`, the scheduler's
activity-state enum. It derives Debug, Clone, equality, Serialize, and
Deserialize. `State.activity_log` is a `Vec<ActivityEvent>` capped at 100.

Activity events cover lifecycle, ticket changes, artifacts, commits, generic
Error, generic Warning, generic Info, poll summaries, launch, timeout, and
health. There is no completion-rejection event or rejection-kind type.

Because the activity enum is serialized, any structured rejection payload
added here needs serializable field types. Core completion identifiers and
`CompletionRejection` do not derive serde. String correlation and detail fields
fit the existing serialized activity boundary without altering reducer types.

`State::format_activity_event` exhaustively renders ActivityEvent for textual
state snapshots. Any new event must be added to this match and should retain
its stable rejection label and correlation value.

## UI projection

`activity_event_to_ui_entry` in plugin `lib.rs` converts core activity events
to `ui::ActivityEntry`. Some events are filtered out. Generic Error and Warning
remain generic `ui::ActivityType` variants with empty ticket IDs.

`crates/lisa-plugin/src/ui.rs` defines `ActivityType`. Its variants are
PhaseCompleted, Commit, Error, Warning, ThreadStarted, and Info. There is no
typed completion rejection entry.

The dedicated Activity view renders every UI activity entry. The Operations
view filters to PhaseCompleted, Error, and Warning. Consequently a new
rejection UI type must be included in both renderers if it is to be visible in
the dashboard's alert-oriented default surface as well as the full feed.

Error, Warning, and Info messages are truncated to 40 characters in the full
activity view and 50 characters in the filtered view. A correlation appended
to one of those strings can disappear after truncation. A dedicated renderer
can preserve the full identity.

The UI activity renderer selects icons and colors by variant. Distinct
rejection kinds can share an alert color while retaining distinct stable labels
and details. Distinction need not require five top-level UI enum variants if a
nested rejection-kind enum remains structurally matchable.

## Existing test surfaces

Core type tests exercise ActivityEvent equality and serde-adjacent shapes but
do not cover completion activity because none exists.

Plugin tests include `test_activity_event_to_ui_entry`,
`test_format_activity_event_variants`, and many behavioral assertions against
generic Warning/Error message substrings. New structured events can require
focused assertion migrations where the old event represented a named
rejection.

UI tests include `test_render_activity_log`, full-dashboard view tests, and
Operations filtering tests. The acceptance criterion explicitly asks for a
UI/activity test covering every named rejection and correlation identity.

A strong test can build five structured entries, render the full Activity
view and Operations view, and assert every stable label and every correlation
is present. It can also pattern-match the converted UI variants to prove the
events did not collapse into a generic boolean, Error, or Warning outcome.

## Repository and concurrency state

At research time, `crates/lisa-plugin/src/lib.rs`, `ui.rs`, and
`crates/lisa-core/src/types.rs` are clean. Lisa-managed provenance and ticket
frontmatter are modified by the scheduler. `crates/lisa-plugin/docs/` is an
untracked pre-existing path and is outside this ticket.

HEAD includes predecessor completion adapter and generation-idempotency work.
The stable completion generation can therefore be reused rather than adding a
second identity scheme.

## Constraints

The core reducer in `completion.rs` is a read-only dependency for this story.
Activity vocabulary in `types.rs` is outside the reducer and is the existing
cross-module boundary for dashboard facts.

The normal Git index must not be used. Source changes must be committed through
`lisa commit-ticket` with exact repository-relative paths. Private artifacts
remain under this attempt work directory for Lisa publication.

All existing generic lifecycle warnings and errors remain outside the five
named rejection outcomes. The ticket does not require converting unrelated
activity strings into new structured types.

The acceptance language requires both identity and categorical distinction to
survive rendering. Mere prose interpolation into a generic warning would not
provide a structurally distinct activity/dashboard entry and may lose identity
to current truncation.
