# Structure: bounded named-state failure regressions

## Source files

### `crates/lisa-plugin/src/lib.rs`

This remains the primary scheduler module and native test module. The ticket
modifies no public crate interface.

Production-local changes:

1. `begin_startup_recovery` returns an optional completed failure outcome.
2. Its existing terminal helper calls are returned to the caller.
3. Its successful recovery-start path returns `None`.
4. `check_assignment_ack_timeouts_at` creates an outcome vector.
5. Each terminal branch pushes its helper result when present.
6. Intermediate retry/reset branches do not add an outcome.
7. The injected-time evaluator returns the vector.
8. The wall-clock poll wrapper explicitly discards it.
9. `fail_startup_recovery` uses `?` for two no-side-effect `Option` guards so
   strict clippy passes.

Test-local changes:

1. Existing non-terminal calls explicitly discard empty result batches.
2. Representative terminal calls assert exact named results.
3. Later deadline scans assert empty batches as the no-loop property.
4. Add a real deadline-driven malformed-authority startup test for
   `StartupFailed`.
5. Preserve all prior authority-vector assertions.
6. Add or retain exact outcome assertions for error, timeout, and stale reclaim.
7. Assert a second scanner call is outcome-free after automatic reclaim.

The private `FailureTransitionOutcome` definition and payloads do not change.

### `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`

This file calls the injected-time evaluator once through a characterization
fixture. If the new `Vec` return triggers `must_use`, change the exact call to an
explicit discard. No consumer ordering, fixture setup, or behavior assertion
changes.

This file is included in source ownership only if modified by compilation or
lint requirements.

## No new files

A separate failure-regression module is not necessary because the reusable
fixtures and relevant private helpers live inside the main `tests` module.
Moving them would create visibility and organization churn unrelated to the
ticket.

No new production module, core type, fixture data file, snapshot, or integration
harness is created.

## Function contracts

### `begin_startup_recovery`

New internal signature:

```text
fn begin_startup_recovery(
    &mut self,
    pane_id: u32,
    now: SystemTime,
) -> Option<FailureTransitionOutcome>
```

Return contract:

- `None`: source state rejected or bounded reset successfully began;
- `Some(StartupFailed)`: initial recovery authority was missing/stale;
- `Some(StartupRecoveryFailed)`: reset preparation terminally failed after the
  recovery boundary was established.

No caller uses the result to authorize state mutation.

### `check_assignment_ack_timeouts_at`

New internal signature:

```text
fn check_assignment_ack_timeouts_at(
    &mut self,
    now: SystemTime,
) -> Vec<FailureTransitionOutcome>
```

The batch contains only completed terminal transitions caused by this call.
Intermediate delivery retry, startup reset, or assignment recovery produces no
entry.

Multiple panes may yield multiple ordered entries.

### `check_assignment_ack_timeouts`

Signature remains unit-returning. It calls the injected-time evaluator with
`SystemTime::now()` and explicitly discards the descriptive batch.

## Branch mapping

`Starting { relaunches: 0 }` with grace readiness:

- successful first delivery: no outcome;
- failed send accepted by terminal helper: `AssignmentDeliveryFailed`.

`Starting { relaunches: 0 }` with SessionStart readiness:

- valid reset start: no outcome;
- invalid initial recovery authority: `StartupFailed`;
- preparation failure after successor boundary: `StartupRecoveryFailed`.

`Starting { relaunches >= 1 }`:

- expired replacement start: `StartupRecoveryFailed`.

`ResettingStartup`:

- expired shell proof: `StartupRecoveryFailed`.

`Delivering { retries < 1 }`:

- successful retry submission: no outcome;
- retry send failure: `AssignmentDeliveryFailed`.

`Delivering { retries >= 1 }`:

- `AssignmentDeliveryFailed`.

`AssignedPendingAck`:

- one fresh assignment recovery begins: no outcome.

`Recovering`:

- `AssignmentRecoveryFailed`.

## Representative test mapping

Assignment delivery:
`test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership`.

Assignment recovery:
`assignment_recovery_failure_retains_authority_for_operator_reset`.

Initial startup:
a new focused test using `pane_name_schedule_state`, scheduled `Starting`, and
an invalidated attempt lease before the initial deadline.

Startup recovery:
`test_missing_shell_readiness_fences_without_relaunch`, with
`missing_replacement_start_fences_without_second_relaunch` retaining the second
subpath bound.

Ordinary error:
`test_check_error_signals_fails_running_thread`.

Session timeout:
`test_check_session_timeouts_expired`.

Stale reclaim:
`test_detect_stale_threads`.

## Assertion shape

Each retained terminal assertion compares a one-element vector with its exact
variant and payload.

Each first bounded transition that merely retries or begins recovery compares
against an empty vector.

Each post-terminal poll or repeated scanner call compares against an empty
vector. This is the explicit proof that unchanged conditions cannot create an
unbounded automatic loop.

Existing launch/delivery counts establish numerical bounds:

- two total chat submissions: initial plus one retry;
- one successor assignment attempt;
- one startup successor;
- at most two provider launches: initial plus one same-pane replacement;
- one signal/scanner reclaim outcome.

## Authority preservation

The tests retain current/high-water lease assertions, thread retained/removed
assertions, slot reservation/release assertions, fence state, provenance, alert
deduplication, UI terminal status, and stale acknowledgement rejection.

Typed outcome collection occurs after the same helper effects and does not
reorder them.

## Workflow artifacts

`research.md`, `design.md`, `structure.md`, `plan.md`, `progress.md`, and
`review.md` remain in
`.lisa/attempts/T-039-03-03/1/work/`.

They are not passed to `lisa commit-ticket`; Lisa owns their admission and
publication.

## Commit boundary

The meaningful source unit is the evaluator return seam plus its colocated
regressions. Function signature and call-site updates must compile together, so
they form one atomic ticket commit.

Exact include paths will be whichever of these are modified:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`.

No broad include, ordinary index, or Lisa-managed path is permitted.
