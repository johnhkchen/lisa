# Research: level-triggered block parking

## Ticket boundary

T-049-05-01 addresses an orphaned blocking Review verdict.

The field incident occurred when an agent wrote a valid block disposition and
its session ended before the scheduler's next block-policy observation.

The durable ticket remained `phase: review, status: open`.

The attempt directory retained the blocking disposition.

The canonical work directory also retained the admitted disposition.

No running thread survived to drive the existing policy.

The ticket was consequently eligible for another Review assignment.

It was absent from Waiting-on-you because that projection requires blocked
ticket status.

The requested behavior is reconciliation on plugin load and every poll.

Scheduling must also refuse to seat a current durable block.

Plain-language rendering changes belong to T-049-05-02.

Pass completion, world checking, and explicit unblock commands are outside the
implementation boundary except where their existing behavior constrains it.

## Attempt identity

`lisa_core::types::AttemptLease` is the execution-generation identity.

It contains a ticket ID and a positive per-ticket `attempt_id`.

`AttemptLease::mint` creates generation one without a predecessor.

It creates the checked numeric successor when given a predecessor.

`AttemptLease::is_current` compares an exact candidate with live authority.

The plugin stores live authority in `State::current_leases`.

It retains process-local generation history in `State::lease_high_water`.

Releasing a slot revokes `current_leases` but retains high water.

Both maps start empty in a newly loaded plugin process.

Private artifacts live under
`.lisa/attempts/<ticket>/<attempt_id>/work/`.

`State::attempt_work_dir` computes that path from a lease.

Attempt directories survive thread teardown and plugin restart.

Their numeric directory component is durable generation evidence.

The current scheduler does not rebuild `lease_high_water` from those paths.

A fresh plugin can therefore mint attempt one despite an existing attempt-one
directory unless the scheduling boundary consults durable high water.

The source incident T-046-06-03 demonstrates multiple durable generations.

Its generation one contains the legacy block from the incident.

Its generation two contains the passing re-review after operator action.

## Artifact admission and canonical correlation

`State::admit_artifact` is the live-attempt publication boundary.

For a leased attempt, it requires the candidate lease to equal the current
lease.

It reads the artifact from that lease's private attempt work directory.

It publishes identical bytes atomically into the canonical work directory.

The canonical path is `config.work_dir/<ticket>/<artifact>`.

A stale live lease cannot overwrite successor output.

The unleased branch exists for historical native fixtures.

That fallback accepts an already-existing canonical artifact only when no
current lease is registered.

`State::review_completion_inputs` admits `review.md` and
`review-disposition.json` for an exact current lease.

It parses the disposition from canonical work after admission.

The canonical filename contains no attempt number or source marker.

The durable private copy provides the missing generation correlation.

Byte equality between canonical disposition and the newest private attempt's
copy proves which generation the canonical verdict represents under the
existing admission operation.

If a newer attempt directory exists without that disposition, the canonical
copy belongs to a prior generation.

If an operator edits the canonical verdict, it no longer correlates with the
old private block.

## Disposition parsing

`lisa_core::disposition::parse_review_disposition` owns the schema boundary.

A valid pass is a pass disposition with a null reason.

A valid block requires a non-empty string reason.

A structured block carries remedy owner, ask, optional steps, and optional
check.

Remedy owners are agent, operator, and world.

Missing or malformed remedy structure does not invalidate a valid block
reason.

The parser instead creates the conservative legacy fallback.

That fallback assigns operator ownership.

It copies the raw reason into the ask.

It marks the parsed value as unstructured.

Malformed JSON, missing fields, empty reasons, and unknown dispositions become
`ReviewDisposition::Invalid`.

The parser therefore already recognizes both forms required by this ticket.

## Existing live Review block policy

`State::apply_review_block_policy` lives in
`crates/lisa-plugin/src/lib.rs`.

Its candidates come exclusively from `State::threads`.

It selects running threads in Review.

Each thread must carry a lease matching both ticket and `current_leases`.

For every candidate it calls `review_completion_inputs`.

It ignores candidates without an admitted `review.md`.

It ignores Pass and Invalid dispositions.

Valid blocks enter `review_block_action`.

Operator and world blocks park immediately.

Agent blocks receive two process-local retries and then park.

The retry count lives in `agent_block_retries`.

A retry appends provenance, releases the slot, removes the thread, and leaves
status open for a successor attempt.

A park writes blocked status before teardown.

It appends a Park row, releases the slot, removes the thread, clears finish-up
state, and rebuilds the DAG.

The live thread and current lease gates explain the orphaned incident.

The ticket requires that live policy and its E-048 retry behavior to remain
unchanged.

## Provenance as generation consumption

`ParkingTransitionRecord` is the durable blocked-work transition row.

Its types are Retry, Park, and Unpark.

It carries ticket, exact attempt lease, remedy owner, retry metadata, recheck
eligibility, and timestamps.

`emit_review_block_transition` currently derives its lease from a live current
thread.

Without that thread it logs a warning and cannot append a row.

The append operation itself accepts an explicit complete record and has no
live-state dependency.

The latest parking transition for an attempt proves its block was already
consumed by policy.

A Retry row prevents the same generation from being converted immediately
into an orphan park before its successor is scheduled.

A Park row makes repeated level reconciliation idempotent.

An Unpark row records that the operator or world reopened the parked
generation and permits a new scheduling episode.

`reconcile_unpark_transitions` appends that Unpark when the latest row is Park
and durable ticket status is open.

It preserves the Park attempt lease and clears the process-local retry count.

Scheduling does not depend on successful Unpark provenance.

## Scheduling

`Dag::can_start` admits open or in-progress tickets whose dependencies are
done.

Review-phase open tickets are ordinary ready candidates.

`State::schedule_ready_tickets` reads `Dag::get_ready_tickets`.

It checks completion masks, existing threads, concurrency caps, provider caps,
and pane availability.

It does not inspect Review dispositions.

After admission gates it mints from process-local `lease_high_water`.

It then installs current authority, publishes assignment material, and creates
the thread and seat lifecycle.

An orphan block therefore consumes a seat before the existing block policy can
observe the replacement thread.

Blocked status already excludes a ticket from DAG readiness.

A reconciliation guard at the scheduling entry point covers every caller,
including permission grant, pane discovery, completion release, world recheck,
polling, and keep-working paths.

## Poll and load ordering

`poll_tick` consumes signals and advances artifacts first.

It invokes the live `apply_review_block_policy` before completion and timeout
reconciliation.

It later rebuilds the DAG, reconciles Unpark transitions, and schedules.

That catches new live-attempt blocks but not threads absent at poll start.

`State::load` scans tickets and builds the initial DAG.

It reconciles Unpark transitions and passing Review completions.

It does not reconcile blocks.

Permission grant and pane discovery can schedule immediately after load.

Startup block reconciliation must therefore occur after the DAG and attempt
paths exist but before those events can schedule.

Every poll also needs a durable reconciliation boundary independent of live
thread processing.

## Durable visibility

`lisa_core::parking::collect_parked_remedies` provides shared projection.

It starts from tickets whose status is Blocked.

It parses canonical `review-disposition.json` for each such ticket.

Only valid Block values produce a `ParkedRemedy`.

The projection carries ticket ID, owner, ask, and optional check.

The CLI status command renders it under Waiting-on-you.

The plugin dashboard maps the same collection into waiting items.

No retained parked thread or separate parking database is necessary.

Writing blocked status and retaining the canonical block makes the orphan
visible through both existing surfaces.

## Existing tests and constraints

Native scheduler tests live in the plugin `lib.rs` test module.

Helpers create real temporary ticket, work, attempt, slot, and ledger paths.

`attach_review_block_attempt` constructs a live Review attempt with exact
private artifacts and lease authority.

The two-seat replay covers operator/world parks, seat release, scheduling, and
Park provenance.

The agent replay covers two retries, final park, Unpark, and attempt four.

Dashboard coverage already proves blocked canonical blocks appear in
Waiting-on-you.

Lease tests prove predecessor private artifacts cannot be admitted over a live
successor.

The T-046-06-03 legacy reason is available as the regression fixture text.

Source changes for this ticket can remain inside
`crates/lisa-plugin/src/lib.rs`.

Ticket phase/status remain Lisa-managed during this assignment.

Artifacts must stay in this generation-two private work directory.

Source commits must use `lisa commit-ticket` with the exact owned path.

Unrelated dirty ticket, journal, provenance, and work files must remain
untouched.
