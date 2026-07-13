# Review: generated completion invariant properties

## Disposition

Pass.

T-041-02-02 meets its acceptance criterion with a generated state-machine
integration test over the public pure completion domain.

Both requested property crates are dev-only dependencies.

All ticket-owned source is committed through Lisa's isolated transaction.

Focused and workspace verification pass after the commit.

## Source commit

Commit: `5c03e6e9fe356d5a5033fd9f78a6c96682daed1a`.

Message: `test(core): generate completion invariant traces`.

The commit contains exactly:

- `Cargo.lock`.
- `crates/lisa-core/Cargo.toml`.
- `crates/lisa-core/tests/completion_state_machine.rs`.

No production source file changed.

No ordinary Git staging or commit command was used.

Post-commit status is clean for every ticket-owned source path.

## Manifest and lockfile

`crates/lisa-core/Cargo.toml` adds `proptest = "1.10"` under dev dependencies.

It adds `proptest-state-machine = "0.8"` under dev dependencies.

Cargo resolved proptest 1.11.0 and proptest-state-machine 0.8.0 in the lockfile.

Their transitive dependency graph is recorded by `Cargo.lock`.

Neither property crate is linked into `lisa-core` production dependencies.

This keeps runtime and WASM production dependency surfaces unchanged.

## Test architecture

The new integration test uses `ReferenceStateMachine` and `StateMachineTest`.

The `prop_state_machine!` runner generates sequential arbitrary-order traces.

Sequential here describes application order, not a fixed scenario order.

Each generated step is independently selected from the full disturbance
vocabulary.

The test runs 256 cases.

Each case contains from one through 63 generated transitions.

Proptest can shrink a failing sequence to a smaller counterexample.

The integration boundary proves the test needs only public completion APIs.

## Required ordering disturbances

Passing Review observation and Review-phase entry are separate transitions.

This generates both review-before-phase and phase-before-review orderings.

Stop and Poll are separate transitions.

This generates stop-before-poll, poll-before-stop, duplicates, and interleaved
disturbances.

DuplicateResult can occur before launch, during live work, or after Confirmed.

Reload can occur in any aggregate situation.

Timeout can occur before admission, during live work, or after completion.

ManualRecovery can occur prematurely, resolve live work, or repeat after Done.

All event variants are legal in every reference state.

This is important because preconditions do not filter out the malformed and
duplicate orderings the ticket intends to exercise.

## Independent reference model

The reference model does not call production `reduce` or `reconcile`.

It uses a test-local disposition enum.

It independently tracks Review observation and Review phase.

It independently derives artifact admission from both facts.

It independently tracks live effect cardinality.

It independently tracks total effects issued.

It independently tracks authoritative Done cardinality.

The first Review verdict is durable for the aggregate.

Later Review observations are duplicates rather than verdict rewrites.

This makes blocked-Review traces semantically stable.

## Concrete production harness

The SUT constructs real `DurableCompletionInputs`.

It creates a real `CurrentLeaseArtifactAdmission` only when phase and artifact
observation converge.

It maps no Review to typed Invalid, passing Review to Pass, and blocking Review
to Block.

It calls production `reconcile` after every observation.

This gives every admitted durable fact an immediate convergence opportunity.

It calls production `reduce` for Request, CommandLaunched, and
CommandSucceeded.

It verifies the LaunchCompletion effect carries the expected attempt and
completion identities.

It verifies the launch callback carries the stable correlation identity.

It preserves state on rejected premature or duplicate success events.

## Review-before-phase property

Observing Review first records the disposition but creates no admission.

Reconciliation therefore emits no completion effect yet.

Entering Review phase later creates admission from the durable observation.

The same post-step reconciliation then produces one request effect for Pass.

If phase occurs first, observing Review later follows the symmetric path.

The model and SUT facts are compared after every generated step.

## Stop-before-poll property

Stop does not erase observed Review, admission, aggregate state, or counters.

Poll likewise preserves facts and exercises level-triggered reconciliation.

Because reconciliation runs after each observation, a stop cannot strand an
already admitted passing Review while waiting for a special edge.

Repeated stop and poll observations cannot add work while a command is live or
Done is authoritative.

## Duplicate-result property

The first correlated result for CommandInFlight enters Confirmed.

It clears the single live effect and increments authoritative Done once.

A success before CommandInFlight is rejected by the reducer.

A success after Confirmed is also rejected by the reducer.

The harness leaves state and cardinalities unchanged on both rejection paths.

Generated repetitions therefore prove duplicate callbacks do not create a
second authoritative Done.

## Reload property

Reload reconstructs Confirmed when durable authoritative Done exists.

It reconstructs correlated CommandInFlight when one live command exists.

It reconstructs Eligible when neither fact exists.

Durable Review observation, phase, and counters survive reconstruction.

Immediate reconciliation then re-derives any eligible obligation.

Reload therefore cannot strand Pass and cannot invent a second live command.

## Timeout property

Timeout never increments Done.

Timeout never clears durable Review facts.

When a command is in flight, reconciliation must return the exact correlated
`CommandInFlightActionRequired` decision.

The harness checks that correlation and confirms exactly one live effect still
exists.

It does not translate timeout into another LaunchCompletion.

## Manual-recovery property

Manual recovery uses the same production correlated success gate as an
ordinary result.

It can confirm only actual CommandInFlight work.

Premature recovery does not create Done.

Repeated recovery after Confirmed does not create another Done.

This preserves correlation authority while covering the recorded operator
recovery shape.

## Required invariant: passing Review is never stranded

After every transition, an admitted Pass must have exactly one live completion
effect or exactly one authoritative Done.

The assertion uses concrete counters and is checked against the independent
reference state.

Automatic post-observation reconciliation means liveness does not depend on a
generated trace ending with Poll.

This is the level-triggered behavior the predecessor completion contract was
designed to provide.

## Required invariant: blocked Review never completes

For every admitted Block state, authoritative Done must remain zero.

Production reconciliation sees the exact typed Block disposition.

It emits no LaunchCompletion effect.

Premature results and manual recovery are reducer errors because no command is
in flight.

Generated reload, stop, poll, and timeout observations do not weaken the gate.

## Required invariant: at most one live effect

The harness increments live count only for a production LaunchCompletion
effect accepted by the reducer.

It asserts live count is never greater than one after every transition.

It also compares the count with the independent reference model.

Concrete CommandInFlight state must be equivalent to one live effect.

Reconciliation of CommandInFlight must request bounded intervention, not a
second effect.

## Required invariant: at most one authoritative Done

The harness increments Done only for an accepted correlated success transition
to Confirmed.

It asserts the count is never greater than one after every transition.

Concrete Confirmed state must be equivalent to exactly one authoritative Done.

Duplicate results, recovery, reload, timeout, stop, and poll preserve that
cardinality.

## Verification evidence

`cargo test -p lisa-core --test completion_state_machine` passed before and
after the isolated source commit.

The generated property completed all 256 configured cases.

`cargo test -p lisa-core` passed.

This included 191 unit tests and both completion integration tests.

`cargo test --workspace` passed before commit.

`cargo test --workspace --quiet` passed after commit.

The post-commit run reported 279 CLI tests, 191 core unit tests, the generated
and deterministic core integration tests, and 341 plugin tests passing.

One real-Zellij boundary test remained intentionally ignored because it
requires external runtime prerequisites.

`cargo fmt --all -- --check` passed.

Ticket-owned `git diff --check` passed before commit.

## Interaction with sibling regression work

The sibling deterministic T-041-02-01 test landed during this attempt.

It replays the recorded T-009-01-01 event trace.

This ticket's generated test passed alongside it in both core and workspace
suites.

The two tests are complementary: one preserves exact field evidence, while
this one explores arbitrary repeated and permuted observations.

No sibling-owned file was included in this ticket's source commit.

## Open concerns and limitations

The property harness is pure and does not execute Zellij, filesystem polling,
or a real Git completion transaction.

It verifies the core aggregate contract at its intended adapter-neutral
boundary.

Adapter integration remains covered by existing deterministic plugin and CLI
tests rather than this generated model.

Generated case selection is probabilistic, although the event vocabulary is
complete and failing traces are shrinkable/replayable by proptest.

The harness treats the first observed Review verdict as authoritative and
subsequent pass/block observations as duplicates; testing artifact replacement
would require a distinct completion identity and is outside this ticket.

No critical issue, TODO, or human-blocking concern remains.

## Repository hygiene

The worktree contains Lisa lifecycle/provenance changes and unrelated plugin
documentation owned outside this ticket.

They were preserved.

The phase artifacts were written only to the private attempt work directory.

Lisa detected and handled shared publication paths independently.

The ticket frontmatter phase/status was not edited by the implementation.

All ticket-owned source is durable in the isolated commit and clean in status.

The work is ready for Lisa's completion gate.
