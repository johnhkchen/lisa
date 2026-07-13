# Progress: completion effect adapter seam

## Status

Implementation is complete and verified.
One ticket-owned source file is ready for the required isolated transaction:
`crates/lisa-plugin/src/lib.rs`.

## Completed work

- Imported E-041 completion reducer vocabulary into the plugin.
- Added private `CompletionInput::{Artifact, Stopped}` scheduler evidence.
- Factored passing Review admission from completion execution.
- Added `State::dispatch_completion` as the typed adapter seam.
- Mapped current pending state to core `Eligible`/`Requested` state.
- Converted attempt generation and ticket identity into typed request events.
- Called the pure reducer and handled its optional returned effect exhaustively.
- Logged typed reducer rejection text at the adapter boundary.
- Refactored completion host execution into one exhaustive
  `execute_completion_effect` method.
- Preserved lease, authority, dependency, ticket lookup, pending insertion,
  command construction, and result-gating behavior.
- Added effect identity checks against scheduler ticket and attempt authority.
- Routed Review artifact polling through `CompletionInput::Artifact`.
- Routed stopped Review completion through `CompletionInput::Stopped`.
- Retained legacy bridges for Idle, ObservedDone, and Manual sources explicitly
  owned by successor T-042-01-02.
- Added a native-test-only effect recorder at the real executor boundary.
- Extended the passing Review test to assert one exact typed launch effect.
- Drove the stopped source after artifact dispatch and asserted no duplicate
  effect is executed.

## Acceptance evidence

The single dispatch function is `State::dispatch_completion`.
Both owned sources construct `CompletionInput` and call it.
The dispatcher constructs `CompletionEvent::Request`, calls
`reduce_completion`, and passes only `transition.effect` to the executor.

The only completion-specific Zellij host launch is inside
`State::execute_completion_effect`.
The other `run_command_with_env_variables_and_cwd` call in the file belongs to
the unrelated notification hook.

`test_check_artifact_advances_review_to_done` proves an eligible current-attempt
passing Review produces one `EffectCommand::LaunchCompletion` with:

- attempt ID equal to the current lease generation;
- completion ID equal to the ticket ID;
- exactly one recorded execution after a stopped-source retry.

## Verification

Focused seam test:

`cargo test -p lisa-plugin --lib test_check_artifact_advances_review_to_done --no-fail-fast`

Result: 1 passed, 0 failed.

Focused disposition compatibility test:

`cargo test -p lisa-plugin --lib review_disposition_gates_artifact_completion_and_dependents --no-fail-fast`

Result: 1 passed, 0 failed.

Plugin library suite:

`cargo test -p lisa-plugin --lib --no-fail-fast`

Result: 341 passed, 0 failed.

Workspace suite:

`cargo test --workspace --no-fail-fast`

Result: all executed workspace tests passed. The declared real-Zellij
environment test remained ignored by its existing contract. Major suites
included 279 CLI, 191 core, and 341 plugin unit tests plus integration tests.

WASM lint gate:

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`

Result: passed.

Formatting:

`cargo fmt --all -- --check`

Result: passed.

## Deviations

The planned focused proof was added to the existing
`test_check_artifact_advances_review_to_done` fixture rather than creating a
second nearly identical fixture. This keeps the test setup focused while adding
the required effect cardinality and stopped-source assertions.

No production module was created and no dependency changed, as planned.

## Remaining work

1. Commit the exact owned source path through `lisa commit-ticket`.
2. Confirm the source path is clean and ordinary index remains untouched.
3. Write Review artifacts and remain on this ticket.

Successor tickets intentionally retain responsibility for migrating remaining
sources, eliminating the boolean legacy wrapper, level-triggered reload
reconciliation, correlation rendering, and nested Git-root command paths.
