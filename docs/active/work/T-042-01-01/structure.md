# Structure: completion effect adapter seam

## Modified source

Only `crates/lisa-plugin/src/lib.rs` is modified. The seam stays beside the
private completion state it coordinates; no new production module or dependency
is needed.

## New input type

Add `CompletionInput` near `CompletionSource` with:

- `Artifact { ticket_id, source_lease }`
- `Stopped { ticket_id, pane_id, source_lease }`

Both require `AttemptLease`; neither can express operator authority. The pane
retains stopped-source diagnostics.

Import `reduce`, `AttemptId`, `CompletionEvent`, `CompletionId`,
`CompletionState`, and `EffectCommand` from `lisa_core::completion`.

## Test observable

Add `#[cfg(test)] launched_completion_effects: Vec<EffectCommand>` to State.
Derived Default initializes it. Production layout is unaffected.

## Admission helper

Extract disposition work into
`admit_passing_review(&mut self, ticket_id, source_lease) -> bool`.
It owns disposition artifact admission, parsing, and existing activity messages.
It does not construct events or launch effects. The legacy Review helper becomes
a composition of admission and the centralized executor.

## Dispatch seam

Add `dispatch_completion(&mut self, input: CompletionInput) -> bool`.
It destructures the input, admits passing Review evidence, chooses Eligible or
Requested from the pending map, builds `CompletionEvent::Request`, calls
`reduce`, logs typed rejection, exhaustively extracts the optional effect, and
calls the executor once when present.

## Effect executor

Refactor the current request body into
`execute_completion_effect(effect, ticket_id, source, authority) -> bool`.
It destructures `LaunchCompletion`, validates typed identities against attempt
authority, and preserves already-pending, authority, dependency, ticket-file,
prior-state, pending insertion, command construction, launch, and activity
behavior. It contains the only completion-specific host launch.

A temporary legacy wrapper constructs `LaunchCompletion` and delegates to this
executor for un-migrated sources. It contains no host launch.

## Caller rewiring

`check_artifact_advances` constructs `CompletionInput::Artifact`.
`auto_complete_review` obtains the assigned slot's lease and constructs
`CompletionInput::Stopped`. A missing lease cannot form typed attempt input and
does not request completion, matching current authority rejection behavior.
Idle, ObservedDone, and Manual remain on the legacy wrapper.

## Tests

Add a colocated leased Review fixture with private `review.md` and passing
disposition. Drive artifact polling and assert one pending entry plus exactly
one recorded `LaunchCompletion` whose attempt ID equals the lease generation
and completion ID equals the ticket. Then install/use the stopped path and
assert the recorded count remains one.

Existing tests verify admission failures, block/invalid dispositions,
transaction results, and stopped transition behavior.

## Verification and commit

Run formatting, the focused test, plugin library tests, workspace tests, and
WASM Clippy. Search for completion command launches to confirm one location.
Commit the single owned source path with exact-path `lisa commit-ticket`.
Attempt-private artifacts are excluded from that transaction.
