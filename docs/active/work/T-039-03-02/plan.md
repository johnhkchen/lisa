# Plan: implement named failure transition outcomes

## Step 1: define the typed outcome

Add the private seven-variant `FailureTransitionOutcome` enum next to existing
failure-boundary types in `crates/lisa-plugin/src/lib.rs`.

Verification: compile derives and exact variant construction in tests.

Atomic source unit: the entire plugin source refactor and its colocated tests,
because the private enum, changed signatures, and assertions cannot compile
independently as separate file commits.

## Step 2: type retained assignment failures

Make `fail_assignment_delivery` and `fail_assignment_recovery` return an
optional named outcome.

Preserve source-state guards, state insertion, ticket lookup, thread failure,
alert deduplication, lease retention, and logging. Return a successful outcome
even when a malformed missing reservation prevents ticket identification,
because the seat transition still occurred.

Verification: delivery and assignment-recovery characterization tests assert
the named variant plus all prior authority facts.

## Step 3: type retained startup failures

Make `fail_startup` and `fail_startup_recovery` return optional named outcomes.

Preserve the distinction between initial startup failure and exhausted recovery:
the former retains an unfenced pane and current lease; the latter revokes the
successor and fences the pane while retaining the failed reservation.

Verification: direct initial failure coverage asserts `StartupFailed`;
shell-readiness and replacement-start tests assert `StartupRecoveryFailed`
while retaining lifecycle assertions.

## Step 4: type ordinary error reclaim

Make `check_error_signals` collect outcomes. Append `ErrorReclaimed` only for a
running thread after failed provenance, lease/slot release, thread removal,
alert, and log have completed.

Route a recovery `.error` through `AssignmentRecoveryFailed`. Do not create an
outcome for an unknown pane.

Verification: existing ordinary error and recovery error tests assert the
correct distinction and unchanged state.

## Step 5: type timeout and stale reclaims

Make `check_session_timeouts` and `detect_stale_threads` return vectors. Capture
pane identity before thread removal. Append `SessionTimedOut` or
`StaleThreadReclaimed` after each existing teardown completes, including the
actual fence boolean.

Keep disabled timeout, active over-budget, pending-completion, and awaiting-
human branches outcome-free.

Verification: matrix tests assert exact variants and continue asserting revoke,
fence, provenance, release, removal, alerts, and monotonic redispatch.

## Step 6: preserve consumers

Compile all existing scheduler call sites. Do not branch scheduling decisions on
the new descriptive values. Address only signature-induced compiler errors.

Verification: focused native test compilation.

## Step 7: focused test pass

Run tests for missing fresh chat acknowledgement, assignment recovery terminal
failure, missing shell readiness and replacement startup, error signal reclaim,
expired session timeout, and stale detection.

Fix only outcome-model or regression issues owned by this ticket.

## Step 8: full verification

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin --lib
cargo test --workspace
just check
```

Success criteria:

1. all seven paths have distinct typed returned outcomes;
2. invalid/no-op paths return no outcome;
3. the T-039-03-01 matrix passes unchanged;
4. no lease, retry, pane, provenance, or scheduling semantics change;
5. formatting and project gates are green.

## Step 9: document implementation

Write `progress.md` with completed work, deviations, commit identity, and gate
results. The attempt-private artifact itself is not passed to the source commit;
Lisa publishes workflow artifacts after lease verification.

## Step 10: isolated source commit

Use exactly:

```text
lisa commit-ticket --ticket-id T-039-03-02 \
  --message "refactor: name failure transition outcomes" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use the ordinary index. Confirm the source file is no longer modified,
staged, or untracked after the transaction.

## Step 11: review handoff

Write `review.md` summarizing enum/signature changes, source diff, test coverage,
exact gate results, and open concerns. Recheck repository status, then remain on
this ticket for Lisa's completion publication.
