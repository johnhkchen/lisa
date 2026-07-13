# Structure: completion domain types

## Created file

### `crates/lisa-core/src/completion.rs`

This new public module contains only completion-domain data types and their
small value-object helpers. It contains no reducer, filesystem access, process
launch, scheduler mutation, Zellij imports, or WASM imports.

The file is organized in this order:

1. module-level documentation describing the pure boundary;
2. identity newtypes and shared value-object implementations;
3. retryability and rejection types;
4. lifecycle state and event enums;
5. effect command and transition output;
6. colocated unit tests.

## Modified files

### `crates/lisa-core/src/lib.rs`

Add `pub mod completion;` alongside the other public core modules. Consumers
will import types through `lisa_core::completion::*`. No root-level glob re-export
is introduced because existing modules use public module exposure.

### `crates/lisa-core/Cargo.toml`

Add `thiserror = "2"` to production dependencies. The derive is used only by
the completion rejection and its launch-cause wrapper for standard Display and
Error/source behavior.

### `Cargo.lock`

Cargo may update the lisa-core package dependency list. The lockfile already
contains thiserror 2 transitively, so no new third-party package family should
be introduced.

## Public identity interface

Three distinct opaque types are exposed:

- `AttemptId`
- `CompletionId`
- `CorrelationId`

Each supports:

- `new(value: impl Into<String>) -> Self`;
- `as_str(&self) -> &str`;
- `Display`;
- `From<String>`;
- `From<&str>`;
- Debug, Clone, equality, ordering, and hashing.

Private string fields prevent accidental primitive interchange while keeping
values suitable for maps, logs, and later serialization decisions.

## Public retry and error interface

`Retryability` is a two-variant enum:

- `Retryable`
- `ActionRequired`

`LaunchFailure` is an owned error value with a message and standard Error
implementation. It allows `CompletionRejection::source` to expose a cause
without importing adapter-specific error types.

`CompletionRejection` has exactly the required named variants:

- `AlreadyPending { completion_id }`
- `StaleLease { attempt_id }`
- `DispositionBlocked { reason }`
- `DependencyBlocked { reason }`
- `LaunchFailed { source }`

The enum derives `thiserror::Error`, making each variant independently
matchable and giving it a stable human-readable Display representation.

## Public lifecycle interface

`CompletionState` is an enum with these variants:

- `Eligible`
- `Requested`
- `CommandInFlight { correlation: CorrelationId }`
- `Rejected { reason: CompletionRejection, retryability: Retryability }`
- `Confirmed`

Only applicable variants contain payloads. In particular, the in-flight variant
cannot be constructed without a concrete correlation value.

`CompletionEvent` is an enum with these variants:

- `Request { attempt_id, completion_id }`
- `CommandLaunched { correlation }`
- `CommandLaunchFailed { source }`
- `CommandSucceeded { correlation }`
- `CommandFailed { correlation, source, retryability }`

Events describe observed facts and retain asynchronous correlation. They do not
perform any action.

## Public output interface

`EffectCommand` is an enum containing:

- `LaunchCompletion { attempt_id, completion_id }`

`Transition` is a struct containing:

- `state: CompletionState`
- `effect: Option<EffectCommand>`

The optional singular field establishes the zero-or-one effect boundary that
the reducer ticket will enforce.

## Test structure

Colocated tests cover:

- independent identity construction and formatting;
- all five required lifecycle state shapes;
- required correlation extraction from the in-flight variant;
- rejected state reason and retryability retention;
- an effect-bearing transition;
- an effect-free transition;
- distinct matching and Display for every rejection variant;
- launch-failure standard error source chaining.

## Ownership and commit boundary

The meaningful source unit consists of the new module, crate-root declaration,
dependency manifest, and resulting lockfile. These exact paths are committed in
one Lisa isolated transaction because none is independently buildable without
the others. Private RDSPI artifacts are not source-committed by the agent.
