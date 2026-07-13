# Structure: generated completion invariant properties

## File inventory

Modify `crates/lisa-core/Cargo.toml`.

Add `proptest` under `[dev-dependencies]`.

Add `proptest-state-machine` under `[dev-dependencies]`.

Do not add either crate to production dependencies.

Modify the workspace `Cargo.lock` through normal Cargo resolution.

Add `crates/lisa-core/tests/completion_state_machine.rs`.

Do not modify `crates/lisa-core/src/completion.rs` unless compilation exposes a
missing public seam.

Do not modify plugin or CLI adapters.

## Integration-test boundary

The new file is an external consumer of `lisa_core::completion`.

This verifies the state-machine contract through public API rather than access
to private helpers.

The file imports production types and functions explicitly.

It imports proptest strategies and the state-machine traits/macro.

All harness-only types remain private to the integration test.

## Transition vocabulary

Define `ScenarioEvent` as a clonable, debuggable enum.

Variants represent passing Review, blocked Review, Review phase, stop, poll,
duplicate result, reload, timeout, and manual recovery.

The enum contains no random opaque strings; stable IDs make shrunk traces
readable.

Every variant is always legal in the reference model.

Repeated variants intentionally represent duplicate observations.

## Abstract disposition

Define a small test-local disposition enum or optional boolean authority.

It distinguishes no artifact, Pass, and Block.

It must not reuse production `ReviewDisposition` in the reference state,
keeping the oracle structurally independent.

Conversion to production disposition occurs only in the SUT harness.

## Reference model

Define `ModelState` with adapter durable facts and expected cardinalities.

Fields include phase readiness and observed disposition.

Fields include admission, live effect, and authoritative Done.

Fields include total accepted live launches and total authoritative Done
confirmations as needed for invariants.

Define an empty `CompletionReferenceMachine` marker.

Implement `ReferenceStateMachine` for that marker.

`init_state` returns a constant clean model.

`transitions` returns a weighted or uniform `prop_oneof!` strategy containing
all required variants.

`apply` mutates the abstract model for one observation and performs abstract
level-triggered reconciliation.

No transition calls production code.

## Concrete harness

Define `CompletionHarness` as `StateMachineTest::SystemUnderTest`.

Store concrete `CompletionState`.

Store observed production `ReviewDisposition` and phase readiness.

Store admission state and fixed identities.

Store live launch and authoritative Done counters.

Store timeout/action-required observations only if useful for assertions.

Provide constructors for the initial state and durable inputs.

## Concrete observation application

Add one method that applies a generated `ScenarioEvent`.

Review observations update disposition.

Review phase enables admission from the currently observed artifact.

Stop and Poll preserve durable state.

DuplicateResult attempts a correlated success.

Timeout probes reconciliation without launching duplicate work.

ManualRecovery attempts the same correlated success gate.

Reload reconstructs the aggregate from durable state.

After the observation-specific action, run a reconciliation helper.

## Reconciliation helper

Build `DurableCompletionInputs` from concrete adapter facts.

Call `reconcile` with current aggregate state.

For `Reconciliation::Effect`, call `reduce` with Request.

Require the transition effect to be exactly one matching
`LaunchCompletion`.

Count the effect, store Requested, then apply CommandLaunched.

Store CommandInFlight with the stable correlation.

For `None`, preserve state.

For `CommandInFlightActionRequired`, require correlation equality and preserve
the one live command.

## Result helper

Only issue success against the stable correlation.

Feed the event to the production reducer regardless of current state so
duplicate and premature callbacks exercise rejection paths.

On accepted Confirmed, increment authoritative Done once and clear live work.

On a typed rejection, preserve state and counters.

Never infer success from an error string.

## Reload helper

If authoritative Done exists, restore Confirmed.

If a live command exists, restore CommandInFlight with the stable correlation.

Otherwise restore Eligible and allow reconciliation to re-derive obligation.

Observed disposition and phase/admission facts survive reload.

Counters survive reload because they represent durable external facts.

## State-machine test implementation

Define `CompletionStateMachineTest` marker.

Implement `StateMachineTest` with the concrete harness and reference marker.

`init_test` creates a harness corresponding to initial reference state.

`apply` consumes one generated event through the harness.

It may assert event-local postconditions for blocked results and duplicate
callbacks.

`check_invariants` compares reference and concrete authority/cardinality.

`teardown` is unnecessary because the harness owns no external resources.

## Invariant organization

Create small assertion helpers for readability.

The liveness assertion checks admitted Pass is live or Done after the automatic
reconciliation opportunity.

The blocked assertion checks no authoritative Done in any model state where
the currently admitted disposition is Block.

The live-effect assertion checks a boolean or count never exceeds one.

The Done assertion checks the authoritative count never exceeds one.

Concrete and reference counts must agree after every transition.

Concrete state classification must agree with live/Done reference facts.

## Runner declaration

Invoke `prop_state_machine!` in the integration test.

Use sequential generation because the crate currently supports sequential
state-machine execution.

Choose a range such as `1..64` transitions.

Ensure the macro-generated test name states completion invariants.

Optionally configure case count through proptest configuration if runtime
measurement warrants it.

## Ownership and commits

The implementation unit owns exactly:

- `crates/lisa-core/Cargo.toml`.
- `Cargo.lock`.
- `crates/lisa-core/tests/completion_state_machine.rs`.

Commit them together because the test cannot compile without both dev
dependencies and their lock resolution.

Use one `lisa commit-ticket` invocation with those exact includes.

The phase artifacts remain uncommitted in the attempt-private directory for
Lisa publication.

## Verification structure

Run formatting on the new Rust file via workspace formatting.

Run the focused generated test by package and test target.

Run all `lisa-core` tests.

Run all workspace tests.

Run `git diff --check` on owned paths.

Inspect status before and after the isolated commit.

Confirm no owned file is modified, staged, or untracked afterward.

## Non-goals

No real Zellij session is spawned.

No Git completion command executes.

No ticket frontmatter is changed manually.

No process timeout is slept in real time.

No property depends on filesystem timing.

No unrelated plugin documentation is incorporated.

No deterministic predecessor trace is duplicated verbatim; this test supplies
the arbitrary-ordering complement requested by this ticket.
