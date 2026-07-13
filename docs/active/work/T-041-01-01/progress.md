# Progress: completion domain types

## Status

Implementation is complete. The ticket-owned source unit has been authored and
all planned verification gates pass. No reducer, reconciliation logic, plugin
wiring, or persistence was added because those concerns belong to dependent
tickets and follow-on epics.

## Completed work

### Dependency boundary

- Added `thiserror = "2"` to `crates/lisa-core/Cargo.toml`.
- Cargo resolved the already-present lockfile package at version 2.0.18.
- The lockfile change only adds thiserror to lisa-core's dependency list.
- No CLI, plugin, Zellij, or WASM dependency was added to the module.

### Public module

- Created `crates/lisa-core/src/completion.rs`.
- Added `pub mod completion;` to `crates/lisa-core/src/lib.rs`.
- The public API is available under `lisa_core::completion`.
- The module contains no I/O, scheduler mutation, or runtime launch calls.

### Identity vocabulary

- Added opaque string-backed `AttemptId`.
- Added opaque string-backed `CompletionId`.
- Added opaque string-backed `CorrelationId`.
- Each identity exposes `new` and `as_str`.
- Each supports Display and conversion from owned/borrowed strings.
- Each derives equality, ordering, and hashing independently.

### Lifecycle vocabulary

- Added `CompletionState::Eligible`.
- Added `CompletionState::Requested`.
- Added `CompletionState::CommandInFlight { correlation }`.
- Added `CompletionState::Rejected { reason, retryability }`.
- Added `CompletionState::Confirmed`.
- Correlation is mandatory in the in-flight variant.
- Rejection reason and retryability are mandatory in the rejected variant.

### Typed input vocabulary

- Added request events carrying attempt and completion identities.
- Added command-launched events carrying correlation identity.
- Added launch-failure events carrying adapter-neutral failure detail.
- Added command-success events carrying correlation identity.
- Added command-failure events carrying correlation, detail, and retryability.
- Events are data only and perform no external action.

### Typed outcome vocabulary

- Added `Retryability::Retryable`.
- Added `Retryability::ActionRequired`.
- Added all five required `CompletionRejection` variants:
  already pending, stale lease, disposition blocked, dependency blocked, and
  launch failed.
- Each rejection retains the identity or reason relevant to its diagnosis.
- Added `LaunchFailure` as an owned adapter-neutral error cause.
- Launch rejection exposes its underlying cause through `Error::source`.
- Added `EffectCommand::LaunchCompletion` with attempt/completion identity.
- Added `Transition` with next state and one optional effect command.
- The output type cannot carry multiple effects.

## Test coverage added

Six unit tests were added to the completion module:

1. identity newtypes retain and format opaque values;
2. in-flight state always contains correlation identity;
3. rejected state retains typed reason and retryability;
4. transition represents zero or one effect;
5. every rejection is independently matchable and displayable;
6. launch rejection exposes its standard error source.

## Verification completed

- `cargo fmt --all -- --check`: passed.
- `cargo test -p lisa-core`: passed, 175 tests.
- `cargo test --workspace`: passed.
- Workspace totals observed: 279 CLI unit tests, 175 core unit tests, 341
  plugin unit tests, and 5 non-ignored integration tests.
- One real-Zellij integration test remained ignored by its existing
  environment gate.
- `cargo check -p lisa-plugin --target wasm32-wasip1`: passed.
- `git diff --check` on ticket-owned paths: passed.

## Deviations from plan

No material deviation occurred. The planned single source unit remained the
right commit boundary because the module declaration, dependency declaration,
lockfile relationship, and module implementation must land together to compile.

## Repository preservation

- Existing changes to Lisa's provenance and ticket frontmatter were not edited.
- Existing untracked `crates/lisa-plugin/docs/` content was not edited.
- Lisa-generated shared work publication was not edited directly.
- Only the new core module, core module declaration, core manifest, and lockfile
  are included in the ticket-owned source transaction.

## Remaining work

- Source commit completed through `lisa commit-ticket` as `806c7c7`.
- The committed diff and repository status were inspected successfully.
- Only Review artifacts and the final disposition remain.
