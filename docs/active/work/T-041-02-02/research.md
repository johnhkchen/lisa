# Research: generated completion invariant properties

## Ticket boundary

T-041-02-02 asks for generated state-machine regression evidence around the
completion domain introduced by story S-041-01.

The ticket begins in Research and requires all remaining RDSPI phases.

Its sole acceptance criterion names two dev-only crates: `proptest` and
`proptest-state-machine`.

The generated cases must vary six ordering disturbances:

- Review observed before the ticket phase reaches Review.
- Agent stop observed before a poll.
- Duplicate completion results.
- Process or aggregate reload.
- Timeout observation.
- Manual recovery.

The required properties are safety and liveness statements:

- An admitted passing Review is never stranded.
- A blocked Review never completes.
- At most one live completion effect exists.
- At most one authoritative Done exists.

The workspace-wide test command is the acceptance boundary.

## Workspace layout

The repository is a Cargo workspace whose members are every crate below
`crates/`.

The completion domain is in `crates/lisa-core/src/completion.rs`.

`crates/lisa-core/src/lib.rs` exports that module publicly.

The plugin and CLI are adapters around core concerns, but the completion
module deliberately contains no filesystem, Zellij, scheduler, or process I/O.

This makes `lisa-core` the narrowest test ownership boundary.

The core crate currently keeps unit tests inline in source modules.

It has no top-level `tests/` directory yet.

Its current dev dependency is `tempfile`.

The root `Cargo.lock` records exact dependency resolution for the workspace.

## Completion identities and inputs

`AttemptId`, `CompletionId`, and `CorrelationId` are opaque string newtypes.

They implement clone, equality, ordering, hashing, display, and string
conversion.

`CurrentLeaseArtifactAdmission` is the adapter-approved durable fact proving
that an artifact belongs to the authoritative attempt.

It carries both attempt and completion identities.

`DurableCompletionInputs` combines optional artifact admission with a typed
`ReviewDisposition`.

`ReviewDisposition` is defined in `crates/lisa-core/src/disposition.rs`.

Its `Pass`, `Block`, and `Invalid` variants make fail-closed authorization
visible to the completion domain.

Only exact `Pass` is eligible in reconciliation.

## Aggregate states

`CompletionState::Eligible` means durable facts may authorize a request.

`Requested` means a request was accepted and its launch effect emitted.

`CommandInFlight` contains a mandatory correlation identity.

`Rejected` retains both a typed rejection and retryability.

`Confirmed` is the authoritative successful terminal state.

`Retryability::Retryable` permits reconciliation to request again.

`Retryability::ActionRequired` suppresses automatic retry.

The state values are owned and clonable, so a test harness can simulate
persisting and reloading them without an adapter.

## Reducer events and effects

`CompletionEvent::Request` accepts the attempt and completion identities.

`CommandLaunched` moves Requested to correlated in-flight state.

`CommandLaunchFailed` moves Requested to retryable rejection.

`CommandSucceeded` confirms only when its correlation matches the in-flight
correlation.

`CommandFailed` retains an adapter-neutral failure and supplied retryability.

`EffectCommand` currently has one variant, `LaunchCompletion`.

`Transition` can carry at most one effect structurally because its effect
field is an `Option<EffectCommand>` rather than a collection.

`reduce` consumes a state and event and returns either one transition or a
typed rejection.

Requests in Requested, CommandInFlight, or Confirmed return AlreadyPending.

Unexpected or duplicate callback events return a typed error without a state
transition.

Mismatched correlated results similarly leave the caller without a new state.

## Level-triggered reconciliation

`reconcile` derives current obligation from durable inputs plus aggregate
state.

Eligible and retryable-rejected states emit LaunchCompletion only when an
artifact admission exists and disposition is Pass.

Requested and Confirmed suppress a new effect.

Action-required rejection suppresses a new effect.

CommandInFlight returns `CommandInFlightActionRequired` with the exact
correlation rather than another launch.

This function is the seam through which poll, reload, timeout, and recovery
observations can be represented without performing real I/O.

The pure API does not contain phase, stop, poll, reload, or timeout events.

Those are adapter observations whose completion significance is whether they
cause durable facts to change or reconciliation to run.

## Existing deterministic coverage

Inline completion tests cover identity preservation and correlation presence.

They cover eligible reconciliation and missing admission.

They cover blocked and invalid dispositions.

They cover suppression in Requested and Confirmed.

They cover retryable versus action-required rejection.

They cover in-flight action-required reporting.

Reducer tests cover the legal request, launch, success, and failure path.

They cover retry after retryable rejection.

They cover duplicate requests in all pending or terminal states.

They cover mismatched results and illegal callbacks.

These are example traces rather than generated sequences.

They do not permute adapter-level disturbances around durable Review facts.

## State-machine testing crate contract

Current `proptest-state-machine` exposes `ReferenceStateMachine`,
`StateMachineTest`, and `prop_state_machine!`.

The reference machine supplies an initial-state strategy, a transition
strategy, and a pure transition application.

The test machine initializes the system under test from reference state,
applies the same generated transitions, and checks invariants after each step.

The supported runner is sequential, which still generates arbitrary event
orderings and shrinks failing traces.

The transition strategy can depend on current reference state.

Preconditions can preserve meaningful traces during generation and shrinking.

The macro accepts a transition-count range.

The crate depends on proptest but the ticket explicitly requires both names as
dev-only dependencies.

## Repository and concurrency constraints

The worktree already contains modifications to Lisa provenance and ticket
files owned by orchestration.

It also contains an untracked plugin documentation directory unrelated to this
ticket.

Those files must remain untouched.

Ticket source ownership can be confined to the core manifest, lockfile, and a
new core integration test.

Every source unit must be committed with `lisa commit-ticket` and exact include
paths.

Ordinary staging and commits are prohibited.

Phase artifacts belong only in this attempt-private work directory.

Lisa will publish admitted artifacts and update ticket phase/status itself.

## Test-model constraints

The model must distinguish an observed Review from an admitted Review because
Review-before-phase is explicitly required.

The model must distinguish a blocked disposition from Pass.

The model must count emitted launch effects separately from live effects.

A launch effect becomes live when issued and ceases to be live when a result is
accepted or manual recovery resolves it.

Duplicate result observations must not increment authoritative Done.

Reload must preserve durable admission, disposition, and authoritative Done.

Timeout must not create Done or an additional live launch.

Manual recovery must resolve uncertain in-flight work with correlation-aware
semantics.

Generated traces require a final convergence opportunity; otherwise a trace
ending immediately after Review admission cannot establish a liveness claim.

Safety invariants can be checked after every transition.

Liveness can be checked after each transition that creates eligible durable
facts and again after a final reconciliation in teardown or a terminal probe.

## Verification boundaries

The focused command is `cargo test -p lisa-core`.

The acceptance command is `cargo test --workspace`.

`cargo fmt --all -- --check` checks formatting across the workspace.

`git diff --check` checks whitespace in ticket-owned changes.

The generated test should use a bounded case count and transition range so it
remains appropriate for the ordinary workspace suite.

No production API change is inherently required because the public pure
completion seam already exposes all reducer and reconciliation decisions.
