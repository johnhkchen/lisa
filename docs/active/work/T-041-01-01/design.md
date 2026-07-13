# Design: completion domain types

## Decision goals

- Make lifecycle contradictions structurally difficult or impossible.
- Keep the module pure and independent of the plugin adapter.
- Give the next reducer ticket a stable, exhaustively matchable vocabulary.
- Preserve failure detail without returning booleans.
- Keep effect cardinality explicit: zero or one command per transition.

## Identity options

### Reuse primitive aliases

Attempt, completion, and correlation values could all be `String` or `u64`.
This is compact but permits argument transposition and accidental comparison of
unrelated identities. It does not meet the explicit newtype requirement.

### Public tuple fields

Tuple structs with public inner fields would distinguish the identities while
allowing unconstrained direct construction. This meets the nominal type goal,
but exposes representation and makes later validation difficult.

### Opaque newtypes with constructors and accessors

Each identity owns a private `String` and exposes `new`, `as_str`, Display, and
conversion from strings. This keeps call sites ergonomic while preserving type
separation and the option to validate later without changing all consumers.

Decision: use opaque string-backed `AttemptId`, `CompletionId`, and
`CorrelationId`. Strings accommodate current attempt attribution and future
durable identifiers without imposing a numeric generation format here.

## State-shape options

### One struct with optional fields

A struct containing a phase tag plus optional correlation, reason, and
retryability fields can encode all named states. It can also encode an in-flight
state without correlation or a confirmed state with rejection metadata.

### Enum with payloads on applicable variants

An enum attaches correlation only to `CommandInFlight` and rejection metadata
only to `Rejected`. The other variants are unit variants. Illegal combinations
cannot be constructed.

Decision: use the enum. `CommandInFlight { correlation }` requires a concrete
`CorrelationId`; `Rejected { reason, retryability }` requires both values.

## Event vocabulary

The reducer needs facts from the adapter, not instructions to mutate external
state. The minimal lifecycle facts are request admission, command launch,
command launch failure, command success, and command failure.

Events carry identities when they cross an asynchronous boundary. Request
events carry attempt and completion identities. Launch and result events carry
correlation identities. Failures carry owned error text because adapter error
types cannot enter this pure crate.

Decision: define `Request`, `CommandLaunched`, `CommandLaunchFailed`,
`CommandSucceeded`, and `CommandFailed` variants. This ticket defines only the
data contract; T-041-01-02 decides which state/event pairs are accepted.

## Effect representation

An effect could be a bare struct because completion currently has one command.
An enum is preferable because it stays exhaustively matchable when later
adapter commands are introduced.

Decision: define `EffectCommand::LaunchCompletion` carrying attempt and
completion identities. Correlation is produced by launch acknowledgement, so it
belongs to the later event rather than the launch request.

## Transition representation

A vector of effects permits accidental fan-out. A boolean `has_effect` loses
the command. A single optional command precisely states the story invariant of
at most one effect per accepted transition.

Decision: `Transition` contains the next `CompletionState` and
`Option<EffectCommand>`. Fields remain public because the type is aggregate
output data and callers need to inspect both parts.

## Retryability representation

A boolean could encode retryability, but the ticket asks for non-boolean
outcome types and named state meaning. A two-variant enum makes call sites and
diagnostics self-describing.

Decision: use `Retryability::{Retryable, ActionRequired}`. `ActionRequired`
describes a non-automatic path without claiming the failure is permanently
impossible to resolve.

## Rejection representation

Rejections could carry one free-form string or one enum plus an optional reason.
The former loses exhaustiveness; the latter permits missing details for variants
that require them.

Decision: use a `CompletionRejection` enum with the five required variants.
`AlreadyPending` carries the existing completion ID, stale lease carries the
attempt ID, disposition/dependency blocks carry reasons, and launch failure
wraps a small owned error type. `thiserror::Error` derives Display and source
behavior. No other completion types depend on thiserror functionality.

## Serialization

These types will later cross persistence boundaries, but this ticket and story
define a pure in-process aggregate contract. Premature serde naming would make a
wire format part of the public contract before the journal design lands.

Decision: do not derive serde in this ticket. Add it only when a persistence
ticket defines the durable schema.

## Rejected alternatives

- Reusing `AttemptLease` would not provide the requested attempt newtype and
  would couple aggregate identity to the scheduler's lease representation.
- Adding an `Idle` state was rejected because it is not in the settled named
  lifecycle and eligibility is derived by the later reconciliation ticket.
- Putting correlation on every state was rejected because it permits meaningless
  correlations and weakens the in-flight invariant.
- Storing multiple effects was rejected because the story caps transitions at
  one command.
- Embedding `std::io::Error` in all rejection variants was rejected because
  only launch failure has an underlying adapter cause.
- Adding reducer constructors was rejected as work owned by T-041-01-02.

## Compatibility and validation

- Add `thiserror = "2"` as a direct lisa-core dependency.
- Declare `pub mod completion` in the crate root.
- Keep every production import within `std` and `thiserror`.
- Add focused unit tests for identities, state payloads, transitions, Display,
  and source chaining.
- Run core tests, workspace tests, and the WASM target check.
