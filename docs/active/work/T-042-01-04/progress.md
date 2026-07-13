# Progress: rejection and correlation activity state

## Status

Implementation is complete and verified before the isolated source
transaction.

The acceptance surface is implemented across shared activity state, plugin
completion classification/projection, UI conversion, and both dashboard
activity views.

## Completed: shared activity vocabulary

Modified `crates/lisa-core/src/types.rs`.

Added serializable `CompletionRejectionKind` with the five required typed
categories:

- already-pending;
- stale-lease;
- disposition-blocked;
- dependency-blocked;
- launch-failed.

Added stable kebab-case Display labels matching the ticket language.

Added `ActivityEvent::CompletionRejected` with ticket ownership, typed kind,
stable correlation ID string, and actionable detail.

Added a core unit test pinning all five labels.

The pure reducer in `crates/lisa-core/src/completion.rs` was not modified.

## Completed: adapter correlation and classification

Modified `crates/lisa-plugin/src/lib.rs`.

Added a single `log_completion_rejection` projection helper. It maps every
named core rejection variant to the shared activity kind and retains detail.
UnexpectedEvent and CorrelationMismatch remain correlated generic warnings
because they are outside this ticket's explicit five-kind contract.

Added `completion_correlation`, which reuses the existing
`CompletionGenerationId` with ticket completion ID, authoritative attempt ID,
and generation 1. This is the same identity already supplied to the isolated
`complete-ticket` transaction.

Changed Review disposition admission to return typed
`CompletionRejection::DispositionBlocked` instead of logging generic
Warning/Error state internally.

Added correlated Review admission so stale leases are classified before
artifact publication and disposition refusals are logged once with identity.

Changed reducer AlreadyPending errors to structured correlated activity.

Changed executor stale attempt refusal to structured StaleLease activity.

Changed incomplete dependency refusal to structured DependencyBlocked
activity.

Changed completion command construction failure in production to structured
LaunchFailed activity while preserving pending rollback.

Changed failed completion command results to structured LaunchFailed activity
with the retry/recovery detail and the same generation correlation.

Changed stale asynchronous result authority to structured StaleLease activity.

No completion command argv, host launch site, pending-state insertion, commit
verification, release, or scheduling behavior was changed.

## Completed: snapshot and UI conversion

Added CompletionRejected handling to `State::format_activity_event`, retaining
ticket, stable label, correlation, and detail in state snapshots.

Added CompletionRejected handling to `activity_event_to_ui_entry`, preserving
structured fields rather than formatting them into a generic message.

Added conversion and snapshot tests.

## Completed: dashboard rendering

Modified `crates/lisa-plugin/src/ui.rs`.

Added `ActivityType::CompletionRejected` with the same structured payload.

Added one common rejection formatter used by both activity renderers. It emits
ticket, exact rejection label, full correlation, and detail. Correlation-bearing
entries bypass generic Warning/Error truncation.

Added the new entry to the full Activity view.

Added the new entry to the Operations alerts-only filter and renderer, so a
supervisor sees refused completion obligations in the default dashboard.

## Completed: acceptance regression

Added
`completion_rejections_render_distinct_kinds_and_correlations_in_both_activity_views`.

The test builds all five typed UI entries with distinct correlations. It first
asserts every entry is structurally CompletionRejected rather than generic
Error or Warning. It then renders both the full Activity feed and Operations
alerts feed and asserts every stable kind label and every correlation remains
visible in each.

Added `named_completion_rejections_become_distinct_correlated_activity_events`
to project all five core rejection variants into shared activity state and
assert exact kind, ticket, correlation, and non-empty detail.

Migrated prior generic-message assertions for disposition, dependency, stale
lease, and failed command behavior to typed activity matches.

## Deviations from plan

The plan placed failed command-result classification outside the minimum
command-construction launch failure. During implementation review, the result
path was also classified as LaunchFailed because the pure reducer maps a
matching `CommandFailed` event to the same LaunchFailed rejection state. This
ensures real native test behavior, not only the WASM-only command construction
branch, exercises the named rejection and correlation.

No reducer, scheduler policy, or persistence deviation occurred.

## Focused verification

Core label test passed:

`cargo test -p lisa-core completion_rejection_kind --no-fail-fast`

UI acceptance test passed:

`cargo test -p lisa-plugin --lib completion_rejections_render_distinct_kinds_and_correlations_in_both_activity_views --no-fail-fast`

Projection test passed:

`cargo test -p lisa-plugin --lib named_completion_rejections_become_distinct_correlated_activity_events --no-fail-fast`

UI conversion and snapshot formatting tests passed.

## Plugin verification

`cargo test -p lisa-plugin --lib --no-fail-fast`

Passed: 347.
Failed: 0.
Ignored: 0.

## Workspace verification

`cargo test --workspace --no-fail-fast`

All executed tests passed:

- CLI library: 14;
- CLI binary: 267;
- CLI integration suites: passed;
- core library: 195;
- core generated completion integration: 1;
- core recorded regression integration: 1;
- plugin library: 347;
- doc tests: passed.

The declared real-Zellij environment test remained ignored because its
external runtime prerequisites were not requested.

## Quality verification

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings` passed.

`cargo fmt --all -- --check` passed.

`git diff --check` passed.

## Repository state before transaction

The ordinary Git index is empty.

Ticket-owned modified source paths are exactly:

- `crates/lisa-core/src/types.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ui.rs`.

Lisa-managed `.lisa/provenance.jsonl`, ticket frontmatter, shared admitted work
directories, and the pre-existing untracked `crates/lisa-plugin/docs/` path are
outside the ticket transaction and were preserved.

## Isolated source transaction

Committed with the exact command and includes from the plan.

Commit:

`e322a754163e73d2f24fcd14640f04cf786e289d`

Commit message:

`feat(plugin): render correlated completion rejections`

`git show --name-only` confirms the commit contains exactly the three
ticket-owned source paths. All three paths are clean after the transaction and
the ordinary index remains empty.

The focused UI acceptance regression passed again after the transaction.

## Remaining

Write Review artifacts and remain on this ticket for Lisa's completion gate.
