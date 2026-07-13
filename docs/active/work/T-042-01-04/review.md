# Review: rejection and correlation activity state

## Disposition

Pass.

T-042-01-04 now preserves every named completion rejection as a distinct,
typed activity/dashboard entry with stable correlation identity. The full
Activity feed and the Operations alerts feed render all five exact labels and
their complete correlations without generic Warning/Error collapse or message
truncation.

Focused, plugin, workspace, formatting, whitespace, and WASM lint verification
pass. The isolated source commit contains exactly the three ticket-owned paths,
which are clean afterward. No blocking concern remains.

## Source commit

Implementation commit:

`e322a754163e73d2f24fcd14640f04cf786e289d`

It was created with `lisa commit-ticket` and exact repository-relative
includes:

- `crates/lisa-core/src/types.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ui.rs`.

`git show --name-only` confirms no other path is present in the commit. No
ordinary `git add`, ordinary `git commit`, or broad staging command was used.

## File changes

Modified `crates/lisa-core/src/types.rs`.

Added the shared serializable `CompletionRejectionKind` enum and its stable
kebab-case operator labels. Added structured
`ActivityEvent::CompletionRejected` with ticket ID, kind, correlation ID, and
detail.

Modified `crates/lisa-plugin/src/lib.rs`.

Added correlated rejection projection, typed disposition admission, adapter
gate classification, result-failure classification, activity snapshot
formatting, UI conversion, and focused tests. Migrated old message-parsing
assertions for the named rejection paths to structured activity assertions.

Modified `crates/lisa-plugin/src/ui.rs`.

Added the structured UI activity type, common full-identity formatting, full
Activity rendering, Operations alert filtering/rendering, and the five-case
UI acceptance regression.

No production file was created or deleted. No manifest, lockfile, command argv,
pending-state shape, reducer source, or persistence schema changed.

## Activity state delivered

The shared rejection-kind vocabulary contains exactly the five outcomes named
by the ticket:

- `AlreadyPending` / `already-pending`;
- `StaleLease` / `stale-lease`;
- `DispositionBlocked` / `disposition-blocked`;
- `DependencyBlocked` / `dependency-blocked`;
- `LaunchFailed` / `launch-failed`.

Each `CompletionRejected` fact retains:

- ticket ownership;
- typed rejection kind;
- stable correlation identity;
- actionable detail.

The nested kind remains directly matchable, so consumers and tests never need
to infer state from a boolean or parse a generic warning string.

## Correlation contract

The plugin reuses `CompletionGenerationId` as its visible correlation. The
identity binds the completion/ticket ID, authoritative attempt ID, and
generation 1.

This is the same stable attribution already supplied to `complete-ticket`.
The implementation therefore adds no random identity, process-local counter,
or parallel correlation registry.

The correlation is derived before Review admission and reduction, allowing
pre-launch refusal paths to carry the same obligation identity. The executor
derives it from the reducer-returned effect. Asynchronous result failures
derive it from retained pending authority.

The activity boundary stores the stable Display string because ActivityEvent
is serializable while the pure reducer's opaque ID types are not. No identity
information is lost.

## Adapter classification

Reducer AlreadyPending errors now enter the structured activity surface.

Review-bound sources validate their current attempt before admission. A stale
attempt becomes StaleLease with its attempt-bound correlation.

Review disposition admission now returns typed refusal evidence. Missing,
unreadable, explicitly blocked, and invalid disposition states become
DispositionBlocked with operator-visible detail, logged once by the correlated
dispatcher.

The executor maps non-current attempt authority to StaleLease and incomplete
dependencies to DependencyBlocked.

Completion command construction failures in the production WASM path become
LaunchFailed while preserving pending rollback.

Failed correlated completion transactions also become LaunchFailed with exit,
authority, source, stderr, and retry/recovery detail. This mirrors the pure
reducer's CommandFailed-to-LaunchFailed state transition and gives native tests
a real behavior path for the named outcome.

Stale asynchronous result authority becomes StaleLease. Generic operator or
identity mismatch errors outside the named five retain their existing generic
warning behavior.

## Reducer and launch boundaries

`crates/lisa-core/src/completion.rs` is unchanged. Its event/state/effect and
rejection policy remain read-only dependencies, as required by story scope.

`dispatch_completion` remains the sole typed request gateway.

`execute_completion_effect` remains its sole production effect-executor edge
and the sole completion host-command launch site. The predecessor
`completion_has_one_typed_request_gateway` source invariant still passes.

No direct completion launch or secondary boolean request path was introduced.

## Dashboard rendering

`activity_event_to_ui_entry` preserves structured fields in
`ActivityType::CompletionRejected`.

The full Activity view renders ticket, exact rejection kind, full correlation,
and detail with a rejection-specific icon and alert color.

The Operations alerts-only filter includes CompletionRejected alongside its
existing high-priority entries. The same common formatter is used in both
views, preventing label or correlation drift.

Unlike generic Warning/Error messages, rejection entries are not truncated at
40 or 50 characters. This guarantees the stable generation identity remains
visible at a glance.

State snapshot formatting also includes the exact kind and correlation for
non-dashboard inspection.

## Acceptance mapping

### Every named rejection is distinct

Satisfied by `CompletionRejectionKind` and structured
ActivityEvent/ActivityType variants. Core-to-activity projection tests all five
reducer variants and exact resulting kinds.

### Every rejection carries correlation identity

Satisfied by the required `correlation_id` field derived from the stable
completion generation. Projection tests assert the exact same identity for
all five cases; stale behavior asserts its expected attempt-bound generation.

### Every rejection renders as an activity/dashboard entry

Satisfied in both the dedicated full Activity feed and Operations alerts feed.
The UI acceptance regression renders all five cases and asserts all five
labels and all five distinct correlations in both outputs.

### No generic boolean failure collapse

Satisfied structurally. The UI regression first asserts every entry remains
`ActivityType::CompletionRejected`, not Error or Warning. Adapter tests match
typed activity kinds rather than boolean results or message categories.

## Test coverage

Focused core label test passed.

Focused core-rejection-to-activity projection test passed.

Focused UI conversion and snapshot formatting tests passed.

Focused UI acceptance regression passed before and after the isolated
transaction.

Full plugin library suite:

- passed: 347;
- failed: 0;
- ignored: 0.

Full workspace suite passed:

- CLI library: 14;
- CLI binary: 267;
- CLI integration suites: passed;
- core library: 195;
- core completion integrations: 2;
- plugin library: 347;
- doc tests: passed.

The real-Zellij integration remained ignored under its explicit external
environment contract.

Quality checks passed:

- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`;
- `cargo fmt --all -- --check`;
- `git diff --check`.

## Repository preservation

All three ticket-owned source paths are clean after commit. The ordinary Git
index is empty.

Lisa-managed `.lisa/provenance.jsonl`, ticket frontmatter, and shared admitted
work directories remain outside the source transaction. The pre-existing
untracked `crates/lisa-plugin/docs/` path was preserved.

No unrelated path was included in commit `e322a75`.

## Open concerns and limitations

No blocking concern exists for this ticket.

The public activity rejection-kind enum intentionally covers the five outcomes
explicitly assigned here. Core `UnexpectedEvent` and `CorrelationMismatch`
still render as correlated generic warnings. Expanding the structured public
kind set for those variants should be a deliberate follow-up rather than an
unrequested acceptance expansion.

The generation is currently 1 because that is the existing adapter transaction
generation contract. Later durable retry-generation work can change the source
value without changing the activity or UI shape.

The dashboard intentionally renders full correlations. Very narrow terminals
may wrap long entries, but identity is preserved rather than silently
truncated.

## Critical issues requiring human attention

None.

## Human review focus

Confirm the shared activity vocabulary is the right serialized boundary and
that using the existing completion-generation identity provides the desired
operator correlation. Confirm both dashboard views should retain the full
identity rather than abbreviating it.

Review is complete. This attempt remains on T-042-01-04 for Lisa to validate
the disposition, admit and publish the phase artifacts, prepare the final
completion commit, and release the seat.
