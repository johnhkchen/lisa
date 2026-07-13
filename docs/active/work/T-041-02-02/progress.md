# Progress: generated completion invariant properties

## Phase completion

Research completed in `research.md`.

Design completed in `design.md`.

Structure completed in `structure.md`.

Plan completed in `plan.md`.

Implementation is complete and verified before the isolated source commit.

## Dependency changes

Modified `crates/lisa-core/Cargo.toml`.

Added `proptest = "1.10"` under `[dev-dependencies]`.

Added `proptest-state-machine = "0.8"` under `[dev-dependencies]`.

Neither dependency was added to the production dependency table.

Cargo resolved the compatible current `proptest` release to 1.11.0.

Cargo resolved `proptest-state-machine` to 0.8.0.

The workspace `Cargo.lock` records these packages and their transitive test
dependencies.

This is a minor deviation from the Design wording, which described the
manifest constraint rather than the exact resolved proptest patch/minor.

No dependency-version workaround was required.

## Generated integration test

Created `crates/lisa-core/tests/completion_state_machine.rs`.

The test is a black-box consumer of public `lisa_core` APIs.

It imports `reconcile`, `reduce`, domain identities, aggregate states, durable
inputs, effects, and reconciliations.

It imports typed `ReviewDisposition` from the public disposition module.

No production source module was modified.

## Generated event vocabulary

`ScenarioEvent` includes `ObservePassingReview`.

It includes `ObserveBlockedReview`.

It includes `EnterReviewPhase` independently from Review observation, allowing
review-before-phase and phase-before-review traces.

It includes `StopBeforePoll` independently from `Poll`, allowing arbitrary
relative order and repetition.

It includes `DuplicateResult`.

It includes `Reload`.

It includes `Timeout`.

It includes `ManualRecovery`.

The transition strategy uses `prop_oneof!` over every variant.

Every observation is valid in every state, so generation and shrinking can
retain premature, repeated, and reordered observations.

The sequential state-machine runner generates from 1 through 63 transitions.

It runs 256 generated cases per ordinary test invocation.

## Reference model

Implemented `CompletionReferenceMachine` with the crate's
`ReferenceStateMachine` trait.

Its initial state is a clean model with no Review, no Review phase, no live
effect, and no Done.

The reference model uses its own `ModelDisposition` rather than production
`ReviewDisposition`.

The first observed Review verdict becomes the durable verdict.

Later pass/block observations are duplicates and do not rewrite authority.

This keeps the blocked property unambiguous: a blocked Review cannot later be
reclassified as a passing Review within the same aggregate trace.

Review is admitted only when both a verdict and Review phase exist.

An admitted Pass with neither live work nor Done creates one abstract live
effect.

An admitted Block creates no effect.

The first result while work is live clears live work and increments
authoritative Done.

Premature and duplicate results are model no-ops.

Reload, timeout, stop, and poll preserve durable model facts.

Abstract reconciliation runs after every generated observation.

## Concrete harness

Implemented `CompletionHarness` as the `StateMachineTest` system under test.

It stores adapter-level phase and disposition facts separately from concrete
`CompletionState`.

It constructs `CurrentLeaseArtifactAdmission` only after Review phase and an
observed verdict converge.

Before Review observation, durable disposition is production `Invalid`.

Pass maps to production `Pass`.

Block maps to production `Block` with an operator-visible reason.

Concrete reconciliation calls the production `reconcile` function.

When it returns a LaunchCompletion effect, the harness calls production
`reduce` with a Request and verifies that the transition carries that exact
single effect.

The harness immediately records CommandLaunched through the reducer with a
stable correlation identity.

This establishes one concrete CommandInFlight state and one live effect.

In-flight action-required reconciliation verifies exact correlation and
preserves the live command.

None reconciliation preserves aggregate state and counters.

## Result and recovery behavior

DuplicateResult and ManualRecovery both present a correlated
`CommandSucceeded` event through the production reducer.

Only CommandInFlight accepts this event and enters Confirmed.

An accepted transition clears the live effect and increments authoritative
Done.

Premature or repeated results return typed reducer errors.

The harness preserves concrete state and counters on those errors.

Reload reconstructs Confirmed when authoritative Done exists.

Reload reconstructs correlated CommandInFlight when live work exists.

Otherwise it reconstructs Eligible and lets production reconciliation derive
current obligation from durable facts.

Timeout reaches production reconciliation after its observation.

For live work this verifies `CommandInFlightActionRequired` rather than a new
launch.

## Invariant assertions

The SUT disposition and phase facts match the reference model after every
transition.

Live effect count matches the reference model after every transition.

Total issued effect count matches the reference model after every transition.

Authoritative Done count matches the reference model after every transition.

Live effects are always at most one.

Authoritative Done is always at most one.

An admitted passing Review has exactly one live effect or exactly one
authoritative Done after reconciliation.

An admitted blocked Review has zero authoritative Done.

Concrete Confirmed state is equivalent to one authoritative Done.

Concrete CommandInFlight state is equivalent to one live effect.

## Focused verification

Ran:

`cargo test -p lisa-core --test completion_state_machine`

Result: one generated property test passed.

The run completed 256 cases with generated traces up to 63 transitions.

Ran:

`cargo test -p lisa-core`

Result: 191 unit tests passed, the generated state-machine test passed, the
concurrently admitted recorded-livelock integration test passed, and doc tests
passed.

No core test failed.

## Workspace verification

Ran:

`cargo test --workspace`

Result: all workspace tests passed.

Observed package summaries included 279 CLI tests, 191 core unit tests plus the
two integration tests, and 341 plugin tests, with no failures.

Ran:

`cargo fmt --all -- --check`

Result: passed.

Ran:

`git diff --check -- crates/lisa-core/Cargo.toml Cargo.lock crates/lisa-core/tests/completion_state_machine.rs`

Result: passed.

## Concurrency observations

The predecessor sibling ticket's deterministic
`recorded_livelock_regression.rs` appeared during implementation and was
committed by its owner before status inspection.

The new generated test coexists with and passes alongside that deterministic
trace.

The worktree retains Lisa provenance and ticket lifecycle modifications.

It also retains unrelated plugin documentation.

Those paths were not edited, staged, or included by this ticket.

Lisa also detected phase artifacts and populated the shared active work path;
this implementation wrote artifacts only to the assigned private attempt
directory.

## Plan deviations

The planned model allowed a newer Pass to replace Block, but implementation
keeps the first Review verdict authoritative for the aggregate.

This prevents a trace from making the phrase “blocked Review” ambiguous after a
subsequent distinct passing observation.

Duplicate review events remain generated and explicitly no-op.

The plan mentioned a transition-local timeout counter as optional; none was
needed because exact in-flight reconciliation and invariant checks directly
prove the required behavior.

The implementation uses no preconditions, which broadens rather than narrows
generated orderings.

## Isolated source commit

Ran `lisa commit-ticket` for T-041-02-02 with exact includes for the core
manifest, workspace lockfile, and generated integration test.

Commit: `5c03e6e9fe356d5a5033fd9f78a6c96682daed1a`.

Message: `test(core): generate completion invariant traces`.

Inspection confirmed that the commit contains exactly the three owned paths.

Status inspection confirmed all three owned paths are clean after commit.

No ordinary Git index or commit command was used.

## Post-commit verification

Re-ran `cargo test -p lisa-core --test completion_state_machine`.

Result: passed.

Re-ran `cargo test --workspace --quiet`.

Result: passed with 279 CLI tests, auxiliary integration groups, 191 core unit
tests, both core completion integration tests, and 341 plugin tests; one
environment-dependent real-Zellij test remained intentionally ignored.

No ticket-owned source work remains.

## Remaining lifecycle actions

Write Review and the exact pass disposition JSON.

Remain on the current ticket for Lisa's completion gate.
