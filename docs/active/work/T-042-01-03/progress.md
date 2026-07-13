# Progress: level-triggered eligibility reconciliation

## Status

Implementation is complete and committed.

Focused acceptance verification and the full lisa-plugin unit suite pass.

All planned verification is green. Review remains.

## Ownership baseline

The ticket owns one source path:

- `crates/lisa-plugin/src/lib.rs`

At the planned source-edit boundary, this path was unexpectedly modified by a
concurrent T-042-01-04 attempt.

That ticket and T-042-01-03 both depend directly on T-042-01-02 despite sharing
the same source file.

No edit was made while the overlapping file was dirty.

T-042-01-04 completed its isolated source transaction as commit `e322a75`.

Its commit message is `feat(plugin): render correlated completion rejections`.

The focused T-042-01-04 test passed before this ticket layered changes onto the
new committed interface.

This preserved exact ownership rather than mixing the two source units.

Lisa-managed ticket/provenance changes and unrelated untracked plugin
documentation remain outside this ticket's ownership.

## Completed: domain imports

Extended the plugin completion imports with:

- `reconcile as reconcile_completion`;
- `CurrentLeaseArtifactAdmission`;
- `DurableCompletionInputs`;
- `Reconciliation`.

Retained `reduce as reduce_completion` for all event-driven completion inputs.

No core source or dependency changed.

## Completed: typed reconciliation input

Added `CompletionSource::Reconcile`.

Added `CompletionInput::Reconcile` with ticket ID and exact source lease.

The input does not accept caller-selected eligibility or disposition.

The adapter derives all durable and aggregate facts.

The Reconcile source is stored in PendingCompletion for diagnostic attribution.

## Completed: aggregate reconstruction

Added `State::reconciliation_state`.

It derives Requested when `pending_completions` contains the ticket.

It derives Confirmed only from durable DAG `phase: done` plus `status: done`.

It derives Eligible otherwise.

Pending takes precedence over Done so an in-flight command's early disk bytes
cannot bypass result verification.

An implementation review caught an important compatibility issue here.

The first draft reused this state helper for all completion events.

That would have treated externally observed Done as Confirmed and prevented the
existing ObservedDone source from entering the isolated transaction.

The event-driven branch now intentionally retains its previous mapping:
pending -> Requested, otherwise Eligible.

Only level-triggered Reconcile uses durable Done as Confirmed.

The existing Done-between-polls regression passes after this correction.

## Completed: durable Review inputs

Added `State::review_completion_inputs`.

It admits private `review.md` through `admit_artifact` with the supplied exact
current lease.

It constructs CurrentLeaseArtifactAdmission only after successful admission.

Attempt ID comes from the lease generation.

Completion ID comes from the ticket ID.

It separately admits `review-disposition.json` through the same lease.

The admitted canonical document is parsed by E-040's existing structured
parser.

Missing and admission-failed dispositions become Invalid.

Pass and Block remain typed parser results.

Review admission errors log ticket-specific activity and never fabricate
admission.

## Completed: sole-gateway reconciliation

Refactored `dispatch_completion` into Reconcile and event-driven decision
branches.

The Reconcile branch validates the current lease before filesystem admission.

It calls core `reconcile_completion` with durable inputs and reconstructed
state.

Reconciliation::Effect feeds the existing common optional effect.

Reconciliation::None emits nothing.

CommandInFlightActionRequired has an exhaustive correlation-bearing warning if
a future state mapper makes it reachable.

All existing inputs continue to construct CompletionEvent::Request and call
the pure reducer.

Both branches converge on exactly one textual call to
`execute_completion_effect`.

The existing single-gateway structural test passes.

T-042-01-04's structured rejection/correlation logging remains intact.

## Completed: level-triggered collector

Added `State::reconcile_review_completions`.

It snapshots candidates before mutable dispatch.

Candidates must have a non-completed thread and exact current attempt lease.

The lease ticket ID must match the thread-map ticket key.

Candidates are included when thread or DAG state observes Review.

Done remains included briefly so terminal Confirmed suppression is testable
before normal audit removes the thread.

Every candidate dispatches the typed Reconcile input.

The collector never launches effects directly.

## Completed: poll and load placement

`poll_tick` now invokes reconciliation after artifact and idle advancement.

It runs before transition/timeouts and specifically before Review timeout
evaluation.

This ordering makes an already-present Review obligation pending before a
generic finish-up could be considered.

`load` invokes the same collector after initial DAG construction.

A default fresh State has no reconstructed thread/current lease at that point,
so the call is an authority-safe no-op.

No lease is inferred from directory names or stale marker files.

Durable restart reconstruction remains S-042-02 scope.

## Completed: finish-up suppression

Added `State::review_completion_suppresses_finish_up`.

Pending completion suppresses the action immediately.

Otherwise the helper obtains the thread's exact current lease and re-admits
`review.md`.

An admitted current-attempt Review suppresses the generic write-Review prompt
for both Pass and Block.

An admission error logs an actionable error and also suppresses the misleading
generic prompt.

A genuinely absent Review retains all prior timeout behavior.

Suppressed actions do not mutate `finish_up_sent`, clocks, or activity prompt
records.

All existing positive timeout tests pass.

## Completed: acceptance regression

Added `poll_then_reload_reconciles_review_once_without_finish_up`.

The fixture begins in Implement with a current leased thread.

Private `review.md` is written before phase observation.

The disposition is deliberately absent during the Implement-to-Review edge,
reproducing a lost event-driven completion opportunity.

The test then writes exact Pass and invokes the production level-triggered
poll collector.

It asserts one exact LaunchCompletion effect with ticket and attempt identity.

It asserts one PendingCompletion attributed to Reconcile.

It ages the Review thread past timeout and wind-down and asserts no
FinishUpPromptSent event or sent marker.

A repeated reconciliation represents reload observation and asserts the
effect count remains exactly one while Requested.

The fixture then reconstructs durable Done with no pending transaction and
asserts Confirmed emits no additional effect.

Finally it restores Review, writes a structured Block disposition, and asserts
no effect and no finish-up prompt.

The test therefore covers all acceptance suppressors in the real adapter.

## Verification completed

Formatting was applied with `cargo fmt --all`.

The focused acceptance test passed: 1 passed, 0 failed.

The existing Done-between-polls test passed: 1 passed, 0 failed.

The full lisa-plugin unit suite passed: 348 passed, 0 failed.

Native all-target lisa-plugin Clippy passed with warnings denied.

`git diff --check` passed for the ticket-owned source path.

## Remaining

Write Review and the exact disposition JSON.

Remain on T-042-01-03 for Lisa completion handling.

## Final verification

`cargo fmt --all -- --check` passed.

The focused acceptance regression passed after its final Block timeout
assertion.

The full lisa-plugin suite passed with 348 tests and no failures.

The complete workspace suite passed with no failures; the declared real
Zellij environment test remained ignored by its existing contract.

Native `cargo clippy -p lisa-plugin --all-targets -- -D warnings` passed.

WASM `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`
passed.

The optimized release WASM build passed for `wasm32-wasip1`.

`git diff --check` passed.

## Isolated source transaction

The meaningful source unit was committed with:

```text
lisa commit-ticket --ticket-id T-042-01-03 \
  --message "fix(plugin): reconcile Review completion eligibility" \
  --include crates/lisa-plugin/src/lib.rs
```

The resulting commit is:

```text
27bddc142fd269418fb5dc463f36637fe0a0b5ef
```

`git diff-tree` confirms the commit contains exactly:

```text
crates/lisa-plugin/src/lib.rs
```

The commit changes 374 inserted and 72 deleted lines, primarily from routing
both reducer and reconciler decisions through one common dispatcher effect
boundary and adding the scenario regression.

Post-commit inspection confirms the ticket-owned source path is clean.

The ordinary index is empty.

No ordinary `git add`, broad add, or ordinary `git commit` was used.

Lisa-managed provenance/ticket changes, unrelated T-042-02-02 work, shared
admitted artifacts, and untracked plugin documentation remain untouched.
