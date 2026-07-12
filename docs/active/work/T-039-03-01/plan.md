# Plan: map and verify failure/reclaim invariants

## Step 1: identify all transition entry points

Read the assignment-state enum and the seven requested functions or consumers:

- `fail_assignment_delivery`;
- `begin_assignment_recovery` / `fail_assignment_recovery`;
- `fail_startup`;
- `begin_startup_recovery` / `fail_startup_recovery`;
- `check_error_signals`;
- `check_session_timeouts`;
- `detect_stale_threads`.

Verify their callers in acknowledgement deadlines, typed error ingestion, and
the periodic scheduler poll.

## Step 2: trace each authority mutation

For each entry point, record mutations in execution order for:

1. current and high-water attempt lease;
2. seat-assignment state;
3. thread status and map membership;
4. pane reservation, transition state, and resident session;
5. provenance invocation and outcome;
6. retry counter, successor minting, or rescheduling authority.

Include alerts and signal cleanup when they distinguish otherwise similar paths.

## Step 3: classify terminal policy

Separate operator-retained failures from automatic scheduler reclaims. Confirm
that each retained state prevents further deadline transitions and each reclaim
removes the thread and releases its reservation. Record pane fencing separately
from slot release.

## Step 4: map current deterministic tests

Find the strongest existing fixture for every path. Inspect assertions rather
than relying on test names. Record supplemental guard tests for:

- bounded delivery retry and late acknowledgement rejection;
- one same-pane startup replacement and no second relaunch;
- unknown error-signal no-op;
- active and awaiting-human timeout deferral;
- honest heartbeat versus genuine stale reclaim;
- revoke/fence/release ordering and monotonic redispatch.

Mark fields which are established only by code inspection.

## Step 5: write `progress.md`

Write the explicit transition chains first. Then write the authority matrix so
each row can be compared without reading prose. Add an ordering-invariant list
for predecessor/successor lease changes and provenance-before-removal behavior.

The matrix must use current enum/function names and must not invent the outcome
types owned by T-039-03-02.

## Step 6: run focused tests

Run exact filters for:

- `test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership`;
- `test_missing_shell_readiness_fences_without_relaunch`;
- `missing_replacement_start_fences_without_second_relaunch`;
- `test_check_error_signals_fails_running_thread`;
- `test_check_session_timeouts_expired`;
- `test_detect_stale_threads`;
- `test_codex_heartbeat_honest_then_genuine_hang_reclaimed`.

Also run relevant deferral/no-op tests if a primary fixture does not cover its
selection guard.

## Step 7: run the full plugin library suite

Execute `cargo test -p lisa-plugin --lib`. Record test count, result, and any
warnings. Because no source changes are planned, a passing suite is the required
before-refactor baseline.

## Step 8: inspect repository cleanliness

Run `git status --short`. Confirm no source/test file was changed, staged, or
created by this ticket. Preserve Lisa-managed ticket/provenance changes. Do not
use the ordinary index.

## Step 9: review

Write `review.md` with the artifact list, matrix coverage, test results, and open
gaps. Explicitly note that assignment recovery has weaker direct terminal-vector
coverage if the current suite does not assert `RecoveryFailed` field by field.

## Verification criteria

- All seven requested paths appear in the transition map.
- Every row covers lease, seat, thread, pane, provenance, and retry authority.
- Retry bounds are numeric and tied to current constants.
- Retained failures and automatic reclaims are not conflated.
- Hard-silence fence ordering is explicit.
- Existing tests cited by the matrix pass on the unmodified source tree.
- The complete native plugin library suite passes.
- No ticket-owned source is staged, modified, or untracked.

## Commit units

There are no source commit units. The only outputs are attempt-private workflow
artifacts, which Lisa admits and publishes through its completion transaction.
If inspection unexpectedly reveals that source edits are necessary, document a
plan deviation before editing and use `lisa commit-ticket` with exact paths.
