# Review: bounded named-state failure regressions

## Outcome

T-039-03-03 satisfies its acceptance criterion. The native regression suite now
asserts that each of the seven scheduler failure/reclaim paths completes with
its exact named `FailureTransitionOutcome` within its existing finite budget.

The suite, workspace tests, strict clippy, formatting, diff check, and project
WASM gate are green.

No retry bound, timeout duration, provider readiness rule, scheduler authority,
pane teardown policy, provenance behavior, or UI contract changed.

## Commit

The ticket-owned source was committed through Lisa's isolated transaction:

```text
55d500ea37ddef197c9e0e35a6dce18a6da2a68a
test: lock bounded named failure outcomes
```

The transaction included exactly:

```text
crates/lisa-plugin/src/lib.rs
```

The ordinary Git index was not used and is empty. The committed source path has
no remaining working-tree diff.

The remaining status entries are Lisa-managed provenance, ticket phase, and
work-artifact publication state; they were not included in the source commit.

## Source changes

### Deadline outcome batch

`check_assignment_ack_timeouts_at` now returns
`Vec<FailureTransitionOutcome>`.

The batch contains only terminal transitions completed during that injected-time
scan. It is empty when:

- no deadline is expired;
- a stale snapshot is rejected;
- the first bounded delivery retry is submitted;
- the single startup reset begins;
- the single assignment recovery begins;
- the seat is already terminal;
- a prior automatic reclaim removed its trigger.

This makes the existing named outcome a direct observation of the real timeout
edge rather than requiring tests to infer it from logs or invoke terminal
helpers in isolation.

### Production polling

`check_assignment_ack_timeouts` remains unit-returning and explicitly discards
the descriptive batch.

The scheduler still authorizes behavior through current leases, high-water
history, seat assignments, threads, and agent slots. No scheduling branch reads
the returned outcome vector.

### Startup recovery result

`begin_startup_recovery` now returns `Option<FailureTransitionOutcome>`.

Successful admission into the one allowed reset returns `None`. Missing or stale
initial authority returns `StartupFailed`. Terminal reset-preparation failure
returns `StartupRecoveryFailed`.

The function performs the same mutations in the same order as before. Only the
already-constructed helper result is propagated to the deadline evaluator.

### Strict clippy cleanup

Two `let ... else { return None; }` expressions in `fail_startup_recovery` were
rewritten with `?`, as required by `clippy::question_mark` under `-D warnings`.

These were the only baseline clippy failures. Both guards are side-effect-free
and semantically unchanged.

## Seven-path coverage

### 1. Assignment delivery failure

`test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership` drives
the actual process-start, chat delivery, and injected acknowledgement deadlines.

It proves:

- initial delivery plus exactly one retry;
- the intermediate retry scan has no terminal outcome;
- the retry deadline yields `AssignmentDeliveryFailed`;
- terminal seat state is `DeliveryFailed`;
- the seat never becomes Owned;
- thread, lease, and reservation remain for operator reset;
- a late acknowledgement is rejected;
- a much later poll yields no outcome or provider relaunch.

### 2. Assignment recovery failure

`assignment_recovery_failure_retains_authority_for_operator_reset` drives the
real one-successor recovery boundary and terminal recovery deadline.

It proves:

- exactly one successor attempt is minted;
- expiry yields `AssignmentRecoveryFailed`;
- terminal seat state is `RecoveryFailed`;
- successor current/high-water/slot/thread authority remains aligned;
- retained failure emits no provenance;
- later polling cannot mint or fail another recovery attempt.

### 3. Initial startup failure

New test `invalid_startup_recovery_authority_fails_once_in_named_state` schedules
a real fresh SessionStart-mode startup, invalidates its attempt authority before
the first deadline, and drives the injected-time evaluator.

It proves:

- the deadline yields `StartupFailed`;
- terminal seat state is `StartupFailed`;
- the failed thread and reservation remain inspectable;
- no successor generation is minted;
- the pane remains unfenced for this initial authority failure;
- later polling produces no further outcome.

This is the only new test, increasing the plugin library count from 314 to 315.

### 4. Startup recovery failure

`test_missing_shell_readiness_fences_without_relaunch` proves:

- the first deadline begins one reset without a terminal outcome;
- exactly one successor is minted;
- missing exact shell proof yields `StartupRecoveryFailed`;
- no replacement provider launch occurs;
- successor authority is revoked and the pane is fenced;
- repeated later polls at three offsets are outcome-free.

`missing_replacement_start_fences_without_second_relaunch` proves the companion
subpath:

- exact shell proof admits one replacement provider launch;
- missing replacement process start yields `StartupRecoveryFailed`;
- total provider launches remain exactly two;
- no second recovery relaunch occurs on a later poll.

### 5. Ordinary error reclaim

`test_check_error_signals_fails_running_thread` asserts exact
`ErrorReclaimed`, then an empty second scan.

It retains proof that the signal is one-shot, the thread is removed, the slot is
released, the resident session remains reusable, the pane is not fenced, and an
actionable alert/log is present.

### 6. Session timeout

`test_check_session_timeouts_expired` asserts exact `SessionTimedOut` with
`fenced: true`, then an empty second scan before redispatch.

It retains proof of revoke-before-fence-before-release ordering, timed-out
provenance, thread removal, permanent physical-seat fencing, high-water
retention, alerting, and monotonic successor dispatch on another pane.

### 7. Stale-thread reclaim

`test_detect_stale_threads` asserts exact `StaleThreadReclaimed` with
`fenced: true`, then an empty second scan.

It retains proof of thread removal, slot release/fencing, current lease
revocation, high-water retention, and visible error logging.

## Boundedness review

The suite now encodes the numeric recovery limits directly:

- delivery uses `retries: 0` then `retries: 1`, never 2;
- assignment recovery mints one successor, never a second;
- startup recovery mints one successor;
- startup relaunch count is at most one after the initial launch;
- error signals are consumed once;
- timeout and stale reclaims remove the triggering thread once.

Each retained terminal path is followed by a deliberately late injected-time
scan which returns empty. Each automatic reclaim scanner is invoked again and
returns empty. These assertions are the explicit N2 regression against infinite
retry or silent babysitting.

## Authority review

T-039-03-01's invariant matrix remains intact.

Retained assignment/startup failures keep their specified failed thread,
reservation, and operator-facing terminal seat.

Ordinary error remains non-fencing and automatically reschedulable.

Hard-silence timeout and stale reclaim remain fencing and automatically
reschedulable, with current authority revoked and high-water history retained.

Startup recovery remains same-pane, successor-fenced on terminal failure, and
does not consume a spare seat.

No retained failure emits provenance. Timeout remains `TimedOut`; ordinary error
and stale reclaim remain `Failed` in provenance.

E-034 lease fencing and E-035 two-stage/startup recovery semantics are unchanged.

## Verification record

Focused named/bounded regressions all passed.

```text
cargo test -p lisa-plugin --lib
315 passed; 0 failed; 0 ignored
```

```text
cargo test --workspace
pass
```

The workspace retains one intentionally ignored real-Zellij integration test,
whose definition requires external Zellij, zsh, script, jq, and the wasm target.

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
cargo check -p lisa-plugin --target wasm32-wasip1: pass
cargo test --workspace: pass
```

## Coverage limits

This is deterministic native-fixture evidence. It does not launch a live Zellij
seat or installed provider; S-039-06 owns that exercise.

The initial startup failure fixture deliberately corrupts the slot's attempt
lease after valid dispatch to reach the existing invalid-authority branch. It
does not claim that ordinary dispatch creates that inconsistency.

The outcome batch is currently discarded by production orchestration. It is a
testable typed boundary, not a persisted audit stream or dashboard history.

The tests prove existing configured retry counts and deadline-driven state
transitions. They do not unify timeout/liveness deadline systems, which the
story explicitly excludes.

## Open concerns

No critical issue, semantic regression, TODO, or follow-up fix blocks handoff.

The principal architectural constraint remains that scheduler authority is
distributed across multiple maps and records. Future refactors must preserve the
full invariant vector; comparing only `FailureTransitionOutcome` is not enough.

This attempt is complete through Review. Lisa should now admit/publish the work
artifacts and perform the final completion commit. This seat must remain on
T-039-03-03 until Lisa confirms that gate.
