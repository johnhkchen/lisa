# Progress: bounded named-state failure regressions

## Result

Implemented direct regression evidence that all seven characterized scheduler
failure/reclaim paths terminate in their exact `FailureTransitionOutcome` and
cannot repeat automatically after the terminal edge.

The work does not change retry limits, deadline durations, readiness policy,
lease authority, pane actions, provenance semantics, or scheduling eligibility.

## Outcome observation seam

`check_assignment_ack_timeouts_at` now returns an ordered
`Vec<FailureTransitionOutcome>` containing only terminal transitions completed
by that injected-time scan.

Intermediate bounded transitions return an empty vector:

- initial delivery to the one allowed delivery retry;
- initial startup wait to one same-pane reset;
- reused assignment wait to one fresh recovery generation.

Terminal branches append the existing named helper result after the helper has
performed all pre-existing state, lease, thread, pane, alert, and log effects.

The production `check_assignment_ack_timeouts` wrapper remains unit-returning
and explicitly discards the descriptive batch. Scheduling decisions continue to
read authoritative scheduler state rather than transition observations.

## Startup propagation

`begin_startup_recovery` now returns `Option<FailureTransitionOutcome>`.

It returns `None` when the seat is inapplicable or the single bounded reset
successfully begins.

It returns the existing `StartupFailed` result when initial recovery authority
is missing or stale.

It returns the existing `StartupRecoveryFailed` result when successor minting
or shell-reset preparation reaches the terminal recovery-failure edge.

Mutation order and all helper bodies remain unchanged.

## Seven-path regression matrix

### Assignment delivery failure

`test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership` now
asserts:

- the first deadline produces no terminal outcome;
- state advances from `retries: 0` to exactly `retries: 1`;
- two total chat submissions occur: initial plus one retry;
- the retry deadline returns exactly `AssignmentDeliveryFailed`;
- the retained seat is `DeliveryFailed`, never Owned;
- the thread, lease, and reservation remain operator-visible;
- a late matching acknowledgement is rejected;
- a poll 300 seconds later returns no outcome and cannot relaunch the provider.

### Assignment recovery failure

`assignment_recovery_failure_retains_authority_for_operator_reset` now asserts:

- the real recovery transition mints exactly predecessor plus one;
- the recovery deadline returns exactly `AssignmentRecoveryFailed`;
- the seat ends in retained `RecoveryFailed`;
- successor lease/high-water/thread/slot authority remains consistent;
- no provenance is emitted for the retained failure;
- one actionable alert remains;
- a poll 300 seconds later returns no outcome and cannot mint another attempt.

### Initial startup failure

Added `invalid_startup_recovery_authority_fails_once_in_named_state`.

The fixture schedules a real fresh Claude `Starting` seat, captures its injected
deadline, removes its attempt lease to model invalid recovery authority, and
advances the real evaluator.

It asserts exactly `StartupFailed`, retained `SeatAssignmentState::StartupFailed`,
a failed retained thread/reservation, unchanged current/high-water generation,
an unfenced pane, one alert, and no outcome on a much later poll.

This closes the prior gap where `StartupFailed` was named only by a direct helper
test rather than a deadline-driven path.

### Startup recovery failure

`test_missing_shell_readiness_fences_without_relaunch` now asserts:

- the initial deadline begins recovery and returns no terminal outcome;
- exactly one successor generation is minted;
- the reset deadline returns exactly `StartupRecoveryFailed`;
- later polls at 1, 30, and 300 seconds return no outcomes;
- no replacement provider launch occurs without exact shell proof;
- the successor is revoked, the failed reservation remains visible, and the
  pane is fenced.

`missing_replacement_start_fences_without_second_relaunch` now also asserts:

- the first deadline's reset transition is outcome-free;
- exact shell proof permits one replacement launch;
- replacement start expiry returns exactly `StartupRecoveryFailed`;
- a later poll is empty;
- total launches stay at two, so there is no second recovery relaunch.

### Ordinary error reclaim

`test_check_error_signals_fails_running_thread` retains its exact
`ErrorReclaimed` assertion and now invokes the scanner again, proving the
consumed signal and removed thread cannot yield another reclaim.

The existing non-fencing resident-session, release, removal, alert, and log
assertions remain unchanged.

### Session timeout

`test_check_session_timeouts_expired` retains its exact `SessionTimedOut` result
with `fenced: true` and now asserts a second timeout scan is empty before
redispatch.

The existing revoke/fence/release lifecycle ordering, timed-out provenance,
thread removal, permanent pane fencing, alert, high-water retention, and
monotonic successor assertions remain unchanged.

### Stale-thread reclaim

`test_detect_stale_threads` retains its exact `StaleThreadReclaimed` result with
`fenced: true` and now asserts the next stale scan is empty.

The existing thread removal, slot release/fence, lease revocation, high-water
retention, and error log assertions remain unchanged.

## Boundedness summary

The regression suite now directly proves these finite budgets:

- assignment chat: one retry after initial submission;
- reused assignment recovery: one successor attempt;
- startup recovery: one successor and at most one replacement provider launch;
- ordinary error: one reclaim per consumed signal;
- session timeout: one reclaim per removed thread;
- stale detection: one reclaim per removed thread.

Every retained terminal state is excluded from subsequent deadline snapshots.
Every automatic reclaim removes or consumes its triggering authority. Explicit
empty-result assertions make that no-loop property visible.

## Clippy gate repair

Baseline strict clippy failed on two predecessor `let ... else { return None; }`
guards in `fail_startup_recovery`.

Both were rewritten to the equivalent `?` form requested by clippy:

- missing pane slot returns `None`;
- missing reserved ticket returns `None`.

Neither guard had side effects, so mutation order and behavior are unchanged.
No unrelated lint cleanup was performed.

## Files changed

Modified:

- `crates/lisa-plugin/src/lib.rs`.

The extracted signal-consumer characterization required no edit; ignored return
values compile and lint cleanly.

## Plan deviations

The planned optional signal-consumer call-site adaptation was unnecessary.

The planned exhaustive enum inventory helper was also unnecessary: each of the
seven exact variants is asserted at its representative real scheduler boundary,
which is stronger and avoids a synthetic duplicate test.

No semantic deviation was required.

## Focused verification

The following focused regressions passed:

```text
invalid_startup_recovery_authority_fails_once_in_named_state
test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership
assignment_recovery_failure_retains_authority_for_operator_reset
test_missing_shell_readiness_fences_without_relaunch
missing_replacement_start_fences_without_second_relaunch
test_check_error_signals_fails_running_thread
test_check_session_timeouts_expired
test_detect_stale_threads
```

## Complete verification

```text
cargo test -p lisa-plugin --lib
315 passed; 0 failed; 0 ignored
```

```text
cargo test --workspace
pass; the real-Zellij integration remains intentionally ignored by its declared
external environment/wasm-target gate
```

```text
cargo clippy --workspace --all-targets -- -D warnings
pass
```

```text
cargo fmt --all -- --check
pass

git diff --check
pass
```

```text
just check
WASM check: pass
workspace tests: pass
```

## Implementation status

- Complete: outcome return seam for injected deadline scans.
- Complete: all four retained failure paths asserted at real deadline edges.
- Complete: all three automatic reclaim paths asserted once and idempotently.
- Complete: new initial startup failure regression.
- Complete: baseline strict-clippy findings repaired narrowly.
- Complete: focused tests and all project gates green.
- Complete: isolated Lisa source commit
  `55d500ea37ddef197c9e0e35a6dce18a6da2a68a`.
- Pending: Review artifact after commit verification.
