# Research: bounded named-state failure regressions

## Ticket scope

T-039-03-03 starts in Research and closes story S-039-03's regression layer.
The requested behavior is not a new recovery policy. It is test evidence that
the seven already-characterized scheduler failure/reclaim paths terminate in
their named result without automatic retry loops or silent waiting.

The acceptance criterion requires the regression suite and clippy to be green.
The story explicitly preserves E-034 lease fencing and E-035 startup/recovery
semantics. Live-seat proof belongs to S-039-06 rather than this fixture ticket.

## Predecessor state

T-039-03-01 documented the seven-path state machine and pinned the authority
vector for lease, seat, thread, pane, provenance, and retry behavior.

T-039-03-02 added the private `FailureTransitionOutcome` enum in
`crates/lisa-plugin/src/lib.rs`. Its seven variants are:

1. `AssignmentDeliveryFailed`;
2. `AssignmentRecoveryFailed`;
3. `StartupFailed`;
4. `StartupRecoveryFailed`;
5. `ErrorReclaimed`;
6. `SessionTimedOut`;
7. `StaleThreadReclaimed`.

The type describes a mutation that has already completed. It does not replace
the scheduler's authoritative maps or persisted provenance outcome.

## Scheduler boundaries

All relevant transition code is in `crates/lisa-plugin/src/lib.rs`.

`SeatAssignmentState` contains the retained operational states. The important
terminal states are `DeliveryFailed`, `RecoveryFailed`, and `StartupFailed`.
They remain visible for an operator reset and are not candidates for later
deadline scans.

`FailureTransitionOutcome` names both retained failures and automatic reclaim
transitions. The reclaim paths remove their thread and seat, so their typed
outcome is the most direct per-poll named result.

`MAX_ASSIGNMENT_DELIVERY_RETRIES` is one. A delivery begins with `retries: 0`,
may be submitted once more with `retries: 1`, and then calls
`fail_assignment_delivery`.

`MAX_SAME_PANE_STARTUP_RELAUNCHES` is one. An initial SessionStart-mode startup
may mint one successor and enter `ResettingStartup`. Exact shell readiness may
submit one replacement launch. Missing shell readiness or replacement process
start calls `fail_startup_recovery` and fences the pane.

Assignment recovery is also single-successor. `begin_assignment_recovery`
replaces the reused-session generation with one fresh generation. An expired
fresh recovery acknowledgement calls `fail_assignment_recovery`; terminal
`RecoveryFailed` is not reconsidered by the deadline evaluator.

## Deadline evaluator

`check_assignment_ack_timeouts_at` accepts an injected `SystemTime`, allowing
native tests to advance exact deadlines without sleeping.

It snapshots expired seat states and rechecks each state before mutation. This
prevents one transition in a batch from applying a stale snapshot to a seat
that has already moved.

The evaluator currently returns unit. It invokes named-outcome helpers but
discards their values. Consequently existing real-path timeout tests assert
terminal seat state and retry counts, while a separate direct-helper test
asserts the four retained outcome variants.

This separation leaves a regression gap: a test does not directly establish
that the real injected-time bounded path produced the matching typed outcome.

`check_assignment_ack_timeouts`, called from the production poll, supplies the
current wall time to the injected-time evaluator. Production does not branch on
the typed results.

## Retained failure paths

Assignment delivery failure is exercised by
`test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership`.
It proves one retry, terminal `DeliveryFailed`, retained authority, no ownership,
and no later provider relaunch.

Assignment recovery failure is exercised by
`assignment_recovery_failure_retains_authority_for_operator_reset`. It proves a
single successor, terminal `RecoveryFailed`, retained successor authority, no
provenance, one alert, and idempotence on a much later deadline poll.

Startup recovery failure is exercised on both bounded subpaths:
`test_missing_shell_readiness_fences_without_relaunch` and
`missing_replacement_start_fences_without_second_relaunch`. They prove one
successor, at most one replacement launch, terminal `StartupFailed`, lease
revocation, and pane fencing.

Initial startup failure occurs when an initial `Starting` seat cannot establish
valid recovery authority, such as a missing reservation or attempt lease.
`begin_startup_recovery` delegates that malformed edge to `fail_startup`, which
returns `StartupFailed`. The direct retained-helper test names it, but it does
not traverse the deadline evaluator.

## Automatic reclaim paths

`check_error_signals` returns a vector of `ErrorReclaimed` outcomes. Ordinary
errors emit failed, non-fenced provenance, release the slot, remove the thread,
and leave a resident provider session reusable. Recovery errors instead route
to the retained assignment-recovery failure helper.

`check_session_timeouts` returns `SessionTimedOut` after a session is both over
budget and hard-silent. It revokes, fences, emits timed-out provenance, releases,
and removes in that order. Awaiting-human and active-session guards produce no
outcome.

`detect_stale_threads` returns `StaleThreadReclaimed` for a hard-silent running
thread. It shares the revoke/fence/release shape but emits failed provenance.
Awaiting-human, pending-completion, and active-session guards exclude reclaim.

Each automatic scanner already exposes an ordered outcome vector and has a
representative exact-variant assertion.

## Test organization

The main native tests are a child module of `lib.rs`, so they can inspect private
scheduler state and invoke private transition methods. Two extracted test files
cover signal consumer characterization and ingestion ordering. Failure-state
fixtures and helpers remain colocated in the main module.

The reusable `pane_name_schedule_state` fixture builds a one-ticket DAG and a
physical pane. `fresh_slot`, `acknowledge_assignment`, and
`exit_then_deliver_fresh_codex` support provider lifecycle tests.

Tests count `SessionLaunch` and assignment-delivery activity events to prove
retry bounds. They also poll far past terminal deadlines to prove the terminal
state cannot silently restart recovery.

## Authority constraints

Named results are observations, not permission to mutate further.

Current lease and lease high-water must remain distinct. Retained failures keep
their specified authority; hard-silence reclaims revoke current authority while
preserving high-water history.

Assignment/startup terminal states keep the failed thread and reservation for
operator inspection. Error, timeout, and stale reclaim remove scheduler
ownership and permit a later monotonic redispatch.

Ordinary errors do not fence. Timeout and stale reclaim do fence. Startup
recovery fences only after its single reset/relaunch budget is exhausted.

No retained failure emits provenance. Automatic reclaims emit provenance before
thread removal, with timeout remaining distinct from failed outcome.

## Baseline evidence

`cargo test -p lisa-plugin --lib` passes with 314 tests.

Strict `cargo clippy --workspace --all-targets -- -D warnings` fails in
`fail_startup_recovery` on two `let ... else` expressions whose `None` branch
returns from an `Option`-returning function. Clippy requests the equivalent `?`
operator form.

The clippy findings are in the predecessor's new named-outcome helper at the
same boundary this ticket tests. The rewrite does not change the returned value
or mutation order.

## Files and ownership

Expected source ownership is limited to `crates/lisa-plugin/src/lib.rs` unless
compilation reveals another exact signature consumer.

The ticket file and `.lisa/provenance.jsonl` are Lisa-managed and already
modified in the working tree. They must not be included in the ticket source
transaction.

Phase artifacts belong only in this attempt-private work directory. Lisa will
publish admitted artifacts later.

No core type, persisted schema, CLI command, UI renderer, signal grammar, or
configuration change is required by the current code shape.

## Research conclusion

The production recovery bounds and named outcomes already exist. The missing
evidence is a direct connection between real deadline-driven retained paths and
their typed results, alongside one uniform seven-path regression inventory.
Automatic reclaim paths already provide that connection. The source also has a
narrow pre-existing strict-clippy failure in the typed startup-recovery helper.
