# Plan: level-triggered completion eligibility

## Step 1: add typed durable inputs

Modify `crates/lisa-core/src/completion.rs` to import
`ReviewDisposition`. Add the current-lease artifact admission value carrying
attempt and completion identities. Add the durable input aggregate containing
an optional admission plus the typed disposition.

Verification: compile-time derives and later unit fixtures can construct pass,
block, invalid, admitted, and absent inputs without any plugin type.

Atomic outcome: the pure module can represent all facts named by the acceptance
criterion without booleans or filesystem access.

## Step 2: add reconciliation result vocabulary

Add a public `Reconciliation` enum with exactly three decision categories:
one effect, no action, and unresolved command-in-flight requiring action with
its correlation ID.

Verification: equality tests can distinguish all outcomes, and the actionable
variant cannot be constructed without correlation.

Atomic outcome: reconciliation can satisfy both the no-effect and bounded
in-flight requirements without overloading reducer rejection errors.

## Step 3: implement pure reconciliation

Add `reconcile(&DurableCompletionInputs, &CompletionState)`. Fail closed before
state matching unless a current-lease artifact admission exists and the
disposition is exact Pass.

For eligible inputs, return no action for Requested and Confirmed, no action for
action-required rejection, and a correlation-tagged action-required outcome
for CommandInFlight. Delegate Eligible and retryable Rejected request behavior
through the existing reducer and return its single effect.

Verification: the function contains no I/O, no mutation, no loop, no provider
branch, and no command execution.

Atomic outcome: every call deterministically derives the current obligation
from durable inputs and aggregate transaction state.

## Step 4: cover positive eligibility

Add a unit test with a concrete admitted attempt/completion pair, exact Pass,
and Eligible state. Assert the exact `LaunchCompletion` effect and both
identities.

Add a retryable-rejection test to prove a durable obligation can request a
fresh transaction after a retryable failure and that action-required rejection
does not retry.

Verification: the output matches effect data exactly and contains at most one
command by enum construction.

## Step 5: cover ineligibility

Add tests for missing admission, explicit Block, and Invalid disposition.
Assert `Reconciliation::None` in every case. Use an Eligible aggregate state so
the tests prove durable facts override stale in-memory eligibility.

Verification: no non-pass disposition emits an effect; block and invalid
reasons remain typed in inputs but are not bypassed.

## Step 6: cover pending and confirmed suppression

With eligible durable inputs, reconcile Requested and Confirmed states. Assert
both return `Reconciliation::None`.

Verification: a repeated level-triggered call does not duplicate a pending or
successful completion transaction.

## Step 7: cover bounded in-flight reconciliation

Construct `CommandInFlight` with a named correlation and eligible durable
inputs. Assert reconciliation returns the action-required variant with that
exact correlation and no effect.

Verification: unresolved asynchronous work is not retried, is not silently
discarded, and remains attributable to one command.

## Step 8: format and run focused checks

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-core
git diff --check -- crates/lisa-core/src/completion.rs
```

If formatting check reports changes needed, run `cargo fmt --all`, then repeat
the check. Resolve any failure in the ticket-owned source before proceeding.

Verification: the new API compiles and all core tests pass.

## Step 9: run regression checks

Run:

```text
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
```

These protect downstream compilation and the pure core-to-WASM boundary. A
pre-existing environment-gated real-Zellij test may remain ignored according
to its existing contract; any actual failure must be investigated.

Verification: workspace behavior is unchanged outside the additive core API.

## Step 10: record implementation progress

Write `progress.md` in the attempt-private work directory. Record completed
steps, test counts/results, deviations, and ownership status. Do not write it
to the shared `docs/active/work` directory.

## Step 11: commit the source unit

Run:

```text
lisa commit-ticket --ticket-id T-041-01-03 \
  --message "feat(core): add completion reconciliation" \
  --include crates/lisa-core/src/completion.rs
```

Do not use ordinary `git add` or `git commit`. After the command, inspect status
and the commit to ensure the exact source path was committed and unrelated
working-tree changes remain untouched.

## Step 12: review and disposition

Write `review.md` summarizing the public API, behavior, source diff, test
coverage, repository preservation, and open concerns. Write exactly one valid
`review-disposition.json`. Use pass only when all ticket-owned source is
committed and required verification succeeds; otherwise use block with a
non-empty actionable reason.

Remain on T-041-01-03 after Review. Do not edit ticket phase/status, publish
Done, or begin a dependent ticket.

