# Plan: generated completion invariant properties

## Step 1: add dev-only property dependencies

Edit `crates/lisa-core/Cargo.toml`.

Add `proptest` and `proptest-state-machine` only to `[dev-dependencies]`.

Resolve the workspace lockfile with Cargo.

Verify neither dependency appears in the core production dependency table.

Verification: `cargo metadata --no-deps` identifies them as development
dependencies for `lisa-core`.

## Step 2: create the generated transition vocabulary

Create `crates/lisa-core/tests/completion_state_machine.rs`.

Import only the public completion API and typed disposition.

Define all acceptance-named observations as `ScenarioEvent` variants.

Ensure review-before-phase is possible because Review observation and phase
observation are independent variants.

Ensure stop-before-poll is possible because both variants are independently
generated.

Ensure duplicate results, reload, timeout, and manual recovery are available in
every state.

Verification: review the transition strategy and ensure every enum variant is
present.

## Step 3: implement the independent reference machine

Define an abstract disposition that does not use production state types.

Define model fields for phase, admission, live work, and Done.

Track effect and authoritative Done cardinality.

Implement clean initial-state generation.

Implement a strategy over all scenario events.

Implement pure abstract transition application.

After each abstract observation, perform abstract reconciliation.

Treat the first correlated result or manual recovery as authoritative only
when live work exists.

Treat duplicates and premature results as no-ops.

Verification: compile trait implementation and inspect model independence from
`reduce` and `reconcile`.

## Step 4: implement the concrete completion harness

Store concrete aggregate state and stable identities.

Store adapter-level observed Review and phase state.

Build durable production inputs from admitted facts.

Map generated Review observations into typed dispositions.

On Review/phase convergence, create the current-lease admission.

Apply completion results through `reduce` with stable correlation.

Run level-triggered `reconcile` after each observation.

For a launch effect, apply the Request and CommandLaunched sequence and count
exactly one live command.

For in-flight action-required, preserve the correlated live command.

For reload, reconstruct state from durable external facts.

Verification: focused compile and deterministic short test execution.

## Step 5: encode properties

Implement `StateMachineTest` against the reference machine.

Compare model and SUT authority after every generated transition.

Assert admitted Pass has either one live completion or authoritative Done.

Assert current Block has no authoritative Done.

Assert live completion count is at most one.

Assert authoritative Done count is at most one.

Assert concrete Confirmed corresponds to authoritative Done.

Assert concrete CommandInFlight corresponds to live work.

Declare the generated sequential runner with a useful transition range.

Verification: run the test with verbose proptest output once if a failure needs
trace diagnosis.

## Step 6: format and focused verification

Run `cargo fmt --all`.

Run the integration test target directly.

Run `cargo test -p lisa-core`.

If compilation reveals a crate API mismatch, adjust only the test harness and
document the deviation in progress.

If a property finds a production bug, minimize the failing trace before
deciding whether a production fix is ticket-owned.

Verification: all focused tests exit successfully.

## Step 7: workspace acceptance verification

Run `cargo test --workspace`.

Run `cargo fmt --all -- --check`.

Run `git diff --check -- crates/lisa-core/Cargo.toml Cargo.lock
crates/lisa-core/tests/completion_state_machine.rs`.

Inspect `git status --short` and distinguish pre-existing orchestration or
other-ticket changes from owned changes.

Verification: the acceptance command passes without modifying unrelated files.

## Step 8: record implementation progress

Write `progress.md` in the attempt-private directory.

Record exact files changed.

Record the transition vocabulary and model semantics.

Record all property assertions.

Record test command results.

Record deviations from this plan, including dependency-version or API changes.

Do not publish the artifact to `docs/active/work`.

## Step 9: isolated ticket commit

Commit the meaningful source unit with:

`lisa commit-ticket --ticket-id T-041-02-02 --message "test(core): generate completion invariant traces" --include crates/lisa-core/Cargo.toml --include Cargo.lock --include crates/lisa-core/tests/completion_state_machine.rs`

Use no ordinary Git staging or commit commands.

After the command, inspect status for the exact owned paths.

If a path remains modified or untracked, diagnose and repeat only the safe
isolated transaction needed to make it durable.

Verification: `git status --short --` on the three owned paths is empty.

## Step 10: post-commit verification

Run the focused generated test again from committed state.

Run `cargo test --workspace` again if the isolated transaction rebased or
otherwise changed the base.

Inspect the ticket commit and confirm it contains only exact owned paths.

Do not amend or touch unrelated commits.

Verification: tests pass and the commit file list matches ownership.

## Step 11: Review artifact

Write `review.md` in the attempt-private directory.

Summarize manifest, lockfile, and generated test changes.

Explain how all six required disturbances are generated.

Evaluate the four required properties and their assertion points.

List focused and workspace test results.

Call out that the model is pure and does not execute real adapter I/O.

Flag any remaining coverage limitation or concern.

Write `review-disposition.json` in the exact valid shape.

Use pass only if owned changes are committed and all acceptance verification
passes.

## Step 12: handoff discipline

Remain on T-041-02-02 after both Review artifacts exist.

Do not update ticket phase or status.

Do not publish work artifacts manually.

Do not start another ticket.

Wait for Lisa to verify the lease, publish artifacts, prepare Done, and confirm
the completion commit.

## Atomicity rationale

The manifest, lockfile, and integration test form one compilable unit.

Committing the manifest without the test would add unused dependencies.

Committing the test without dependency resolution would fail compilation.

Therefore one isolated source commit is the smallest meaningful unit.

The private phase artifacts are lifecycle evidence rather than ordinary source
units and are committed later by Lisa's completion transaction.

## Rollback and failure handling

If dependency resolution fails, preserve the manifest diff and diagnose the
registry or version mismatch without changing unrelated workspace settings.

If generated cases expose a counterexample, retain the regression seed while
fixing or refining invalid model assumptions.

If workspace tests fail outside owned files, determine whether the failure was
pre-existing or caused by dependency resolution before disposition.

If the isolated commit fails, do not fall back to ordinary Git commit.

Document any unresolved actionable failure and use a blocking Review
disposition.
