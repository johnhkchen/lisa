# Design: level-triggered eligibility reconciliation

## Goal

Make passing Review completion an obligation derived repeatedly from admitted
current-attempt facts, rather than a one-time consequence of whichever
artifact or lifecycle edge happens to arrive last.

The plugin must emit one completion launch effect while the obligation is
eligible, emit none while the aggregate is pending or confirmed, emit none for
a blocked disposition, and avoid asking an agent to write a Review artifact
that is already admitted.

The design must preserve attempt fencing, dependency checks, the isolated
completion transaction, and the single effect-executor boundary introduced by
the two predecessor tickets.

## Grounded constraints

The core already owns the required level-triggered semantics through
`completion::reconcile`.

Changing the core reducer or reconciler would duplicate settled E-041 work and
is outside S-042-01.

The plugin has an authoritative current-attempt admission boundary in
`admit_artifact`.

The plugin has a structured E-040 disposition parser and canonical publication
path in `admit_passing_review`.

The plugin's only live transaction memory is `pending_completions`.

Durable Done frontmatter is the adapter's currently available confirmed fact.

Durable command journaling and full restart reconstruction belong to S-042-02.

The existing `CompletionInput` seam and sole `execute_completion_effect` call
site must remain structurally enforceable.

## Option 1: rely on the current artifact fixpoint

The smallest change would add only a regression test around
`check_artifact_advances`.

The fixpoint currently inspects `review.md` once as Implement's completion
artifact and again after the ticket becomes Review.

That often produces the desired pending request when Review and Pass are both
already present.

Repeated polls also re-enter the Review branch because the artifact remains on
disk.

This option has very low code risk.

It does not call E-041's `reconcile` function.

It leaves the completion obligation coupled to a phase-edge scanner.

It does not establish a named plugin-load reconciliation boundary.

It also leaves timeout delivery unaware of the admitted Review and pending
transaction.

The historical behavior could therefore regress again if artifact advancement
ordering changes.

This option is rejected because it characterizes incidental retry behavior
rather than implementing the ticket's explicit level-triggered contract.

## Option 2: add an independent reconciliation method beside dispatch

This option would add `State::reconcile_review_completions` and let it call
core `reconcile` followed by `execute_completion_effect` directly.

Artifact, stop, idle, observed-Done, and manual requests would remain in
`dispatch_completion`.

The new pass would run on load and poll.

This provides an obvious repeated reconciliation location.

It can derive durable inputs without disturbing legacy caller behavior.

However, a direct executor call outside `dispatch_completion` creates a second
effect-routing shape.

The predecessor's structural test deliberately requires the sole executor
call to live inside typed dispatch.

Adding another executor call would weaken the adapter seam and make future
completion launches harder to audit.

Wrapping the second path in a helper would obscure rather than remove that
architectural split.

This option is rejected because level-triggered reconciliation is itself a
completion input boundary and should remain inside the typed adapter.

## Option 3: add a typed reconciliation input to the existing adapter

Add a `CompletionInput::Reconcile` variant carrying ticket ID and current
attempt lease.

Add a matching `CompletionSource::Reconcile` diagnostic source for the pending
record and activity log.

Inside `dispatch_completion`, handle this input by deriving current durable
inputs and calling core `reconcile`.

All other CompletionInput variants continue through their existing reducer
Request behavior.

Both decision branches converge on the existing single call to
`execute_completion_effect`.

Add `State::reconcile_review_completions` as the candidate collector only; it
does not execute effects itself.

Call the collector from the poll boundary and plugin load boundary.

This preserves the predecessor seam while making the new obligation explicit.

It also makes the acceptance test able to invoke the real adapter
reconciliation without fabricating a signal edge.

This is the chosen option.

## Durable-input derivation

The Reconcile branch will first require an adapter-observed Review or Done
phase.

Phase is intentionally outside E-041's `DurableCompletionInputs`; the plugin
must enforce it before constructing admission evidence.

The supplied lease must still be current through `admit_artifact`.

`review.md` is admitted first.

If it is absent, durable inputs contain no artifact admission and reconciliation
returns no effect.

If it is admitted, construct `CurrentLeaseArtifactAdmission` from the lease
attempt generation and ticket completion identity.

Next, admit `review-disposition.json` from the same exact lease.

Parse the canonical disposition using `parse_review_disposition`.

Pass reaches core as Pass.

Block reaches core as Block with its actionable reason.

Missing, stale, unreadable, or invalid disposition reaches core as Invalid or
causes a visible adapter rejection before an effect can exist.

The existing `admit_passing_review` behavior for edge-triggered Artifact,
Stopped, and Idle inputs remains intact.

This avoids changing predecessor-source semantics as part of the new path.

## Aggregate-state derivation

The adapter needs a concrete CompletionState to pass to `reconcile`.

If `pending_completions` contains the ticket, derive Requested.

If the current DAG ticket has durable `phase: done` and `status: done`, derive
Confirmed.

Otherwise derive Eligible.

The plugin does not yet retain correlation-bearing CommandInFlight or rejected
aggregate variants.

Those states therefore remain outside this ticket's in-memory derivation.

This mapping matches the facts the current adapter can honestly prove.

It is sufficient for the acceptance requirement's eligible, pending, and
confirmed cases.

## Candidate collection

The collector snapshots ticket ID and attempt lease from live threads.

It requires the thread lease to equal the scheduler's current lease.

It considers a ticket when the adapter observes Review in the thread or DAG.

It may also consider Done long enough to prove Confirmed suppression before
normal audit removes the thread.

Completed threads need no new request.

Running and parked Review threads remain eligible because stopped Review
completion already treats both as completable.

The snapshot avoids mutable-borrow conflicts while each candidate performs
filesystem admission and activity logging.

Every candidate enters `dispatch_completion(CompletionInput::Reconcile)`.

## Poll and load placement

On each poll, run reconciliation after artifact and idle phase advancement.

At that point a private Review that existed before Implement-to-Review has
already been admitted and the thread phase reflects Review.

Run reconciliation before `check_review_timeouts`.

This ordering creates pending state before timeout policy evaluates the same
ticket.

Call the same reconciliation collector at the end of `load`, after the initial
DAG is available.

A fresh default plugin load normally has no reconstructed thread or current
lease, so this call is a safe no-op until authority exists.

That limitation is honest: this ticket must not invent attempt authority or
silently adopt an old private directory.

Tests and any future loader that reconstructs current attempt state receive the
same level-triggered method at the load boundary.

Durable reconstruction remains assigned to S-042-02.

## Finish-up suppression

Ordering alone is insufficient because the current deadline evaluator does
not inspect pending completion state.

Before delivering each Review timeout action, check whether the ticket already
has a pending completion.

Also re-admit `review.md` with the thread's exact current lease.

If admission succeeds, suppress the finish-up prompt regardless of Pass or
Block.

The prompt specifically asks the agent to finish Review; it is false once the
Review artifact is present.

A blocked disposition requires operator/actionable work, not a generic
artifact-writing prompt.

If Review is absent, retain all existing timeout and wind-down behavior.

Do not add suppressed tickets to `finish_up_sent`; if the artifact is later
removed and the thread is still Review, normal timeout policy may act.

Admission failures remain visible through activity logging and fail closed for
completion.

## Reconciliation result handling

`Reconciliation::Effect` supplies the exact LaunchCompletion command data.

That effect enters the existing executor with Reconcile source and attempt
authority.

`Reconciliation::None` emits nothing.

`CommandInFlightActionRequired` is not currently derivable from plugin state,
but the match remains exhaustive and logs an actionable correlation if reached
in future state mapping.

No reducer Request event is needed for Reconcile because E-041's reconcile
output is already the level-triggered command decision.

The existing event-driven inputs continue to use `reduce` exactly as before.

## Test design

Add one acceptance-focused plugin unit test using a temporary ticket and work
tree.

Start the ticket and thread in Implement with a current attempt lease.

Write private `review.md` and passing `review-disposition.json` before the
phase transition.

Run the artifact phase pass to observe Implement-to-Review.

Run the level-triggered reconciliation boundary as the poll observation.

Assert the ticket is Review, exactly one LaunchCompletion effect exists, and
one pending record exists.

Age the Review thread past timeout and run timeout evaluation.

Assert no FinishUpPromptSent event and no `finish_up_sent` entry.

Invoke reconciliation again to simulate reload observation and assert the
effect count remains exactly one.

Remove pending state only in isolated derived-state subcases or build sibling
fixture states to assert Confirmed and Block each emit zero effects.

Prefer one scenario-driven test with small helper closures over changing core
tests, because the acceptance criterion is explicitly about the real plugin
adapter.

## Risks

Repeated artifact admission performs atomic canonical publication on every
eligible poll.

This is existing behavior in the Review artifact scanner and is bounded by the
poll interval.

Repeated invalid-disposition logging can create noise; the activity log is
already capped and current artifact dispatch has similar behavior.

Calling a load-boundary method with no authority could be mistaken for durable
restart reconstruction.

Comments and Review documentation must state that the call never fabricates a
lease and that S-042-02 owns persistence/reconstruction.

The largest regression risk is accidentally creating a second executor call
site.

The structural gateway test and exact effect-count assertion directly guard
that boundary.

## Decision

Use a typed Reconcile input inside the existing completion adapter, derive
durable admission plus honest aggregate state, invoke it after phase
advancement on every poll and after initial DAG load, and suppress Review
timeout delivery whenever a completion is pending or the current attempt's
Review artifact is admitted.

Modify only `crates/lisa-plugin/src/lib.rs` and attempt-private RDSPI artifacts.
