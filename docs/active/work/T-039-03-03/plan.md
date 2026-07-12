# Plan: lock bounded named-state failure regressions

## Step 1: expose deadline outcomes

Change `begin_startup_recovery` to return an optional
`FailureTransitionOutcome`.

Propagate only completed terminal helper outcomes. Return `None` when the source
state does not apply or when the one allowed reset begins successfully.

Verification: compile the production module and existing startup tests.

## Step 2: collect injected-time outcomes

Change `check_assignment_ack_timeouts_at` to return an ordered outcome vector.

Initialize the vector before iterating expired snapshots. Push results from
terminal helper calls. Leave intermediate transitions outcome-free. Return the
vector after processing all panes.

Keep `check_assignment_ack_timeouts` unit-returning and explicitly discard the
batch in production polling.

Verification: no scheduler caller branches on the new result.

## Step 3: adapt existing call sites

Use explicit `let _ =` only for tests whose purpose is unrelated to terminal
outcome identity. Prefer `assert!(...is_empty())` for intermediate and
post-terminal scans in the bounded regressions.

Update the extracted signal-consumer characterization only if its exact call
requires explicit discard.

Verification: no `unused_must_use` warning under strict clippy.

## Step 4: lock assignment delivery

Extend the real missing-ack test:

1. assert the first deadline returns an empty batch;
2. assert retry state is exactly `retries: 1`;
3. assert the retry deadline returns exactly
   `AssignmentDeliveryFailed` for pane 10 and ticket `T-NAME`;
4. retain the two-submission count;
5. assert a much later scan is empty;
6. retain terminal state, no ownership, lease, thread, UI, late-ack, and no
   relaunch assertions.

Focused command:

```text
cargo test -p lisa-plugin --lib missing_fresh_chat_ack
```

## Step 5: lock assignment recovery

Extend the characterization fixture:

1. retain the real `begin_assignment_recovery` call;
2. assert exactly one successor generation;
3. assert recovery deadline returns exactly
   `AssignmentRecoveryFailed`;
4. assert a later scan returns empty;
5. retain failed thread, reservation, lease, alert, no provenance, and no
   successor-retry assertions.

Focused command:

```text
cargo test -p lisa-plugin --lib assignment_recovery_failure
```

## Step 6: lock initial startup failure

Add a deterministic test that schedules a SessionStart-mode fresh provider,
captures its initial startup deadline, invalidates the recovery attempt lease,
and advances to the deadline.

Assert exactly `StartupFailed`, terminal `SeatAssignmentState::StartupFailed`,
failed retained thread/reservation, no successor generation, no fence, and an
empty later scan.

This traverses the real deadline evaluator rather than calling
`fail_startup` directly.

Focused command:

```text
cargo test -p lisa-plugin --lib invalid_startup_recovery_authority
```

## Step 7: lock startup recovery failure

Extend missing shell-readiness regression:

1. first deadline returns empty and creates exactly one successor;
2. reset deadline returns exactly `StartupRecoveryFailed`;
3. all later scans return empty;
4. no replacement provider launch occurs without shell proof;
5. retain lease revoke, thread failure, reservation, pane fence, UI, and alert
   assertions.

Extend replacement-start regression to assert its terminal outcome and empty
later poll while preserving exactly two total launches.

Focused command:

```text
cargo test -p lisa-plugin --lib startup_recovery
cargo test -p lisa-plugin --lib missing_replacement_start
cargo test -p lisa-plugin --lib missing_shell_readiness
```

## Step 8: lock automatic reclaim idempotence

Retain exact variants for ordinary error, session timeout, and stale reclaim.
After each first reclaim, invoke the scanner again and assert an empty vector.

Retain all prior teardown ordering, provenance, fence, release, and redispatch
assertions.

Focused commands:

```text
cargo test -p lisa-plugin --lib check_error_signals_fails_running_thread
cargo test -p lisa-plugin --lib check_session_timeouts_expired
cargo test -p lisa-plugin --lib test_detect_stale_threads
```

## Step 9: fix the acceptance-gate lint

Replace only the two clippy-reported `let ... else { return None; }` guards in
`fail_startup_recovery` with equivalent `?` expressions.

Do not perform unrelated lint cleanup.

Verification:

```text
cargo clippy --workspace --all-targets -- -D warnings
```

## Step 10: focused regression pass

Run all representative named/bounded tests with filters or the complete plugin
library suite if filtering becomes less clear than the full fast native run.

Success criteria:

- all seven exact variants are asserted at real scheduler boundaries;
- each intermediate bounded step is outcome-free;
- each terminal later poll is outcome-free;
- existing authority-vector assertions remain green.

## Step 11: complete gates

Run:

```text
cargo fmt --all -- --check
git diff --check
cargo test -p lisa-plugin --lib
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
just check
```

The acceptance criterion is satisfied only when the regression suite and strict
clippy are both green.

## Step 12: inspect the diff

Confirm the diff contains only:

- descriptive outcome propagation;
- exact result assertions and idempotence checks;
- the focused startup failure fixture;
- two behavior-preserving clippy rewrites;
- any required explicit discard in the extracted test module.

Confirm no constants, timing values, retry policies, authority mutation, logs,
UI state, provenance, signal parsing, or persisted types changed.

## Step 13: record progress

Write `progress.md` with:

- implementation details;
- seven-path test mapping;
- baseline clippy finding and resolution;
- deviations from this plan;
- exact gate results;
- source transaction identity.

The artifact stays attempt-private.

## Step 14: isolated commit

Use `lisa commit-ticket` with ticket ID `T-039-03-03`, an exact message, and only
the modified repository-relative source paths.

Expected form:

```text
lisa commit-ticket --ticket-id T-039-03-03 \
  --message "test: lock bounded named failure outcomes" \
  --include crates/lisa-plugin/src/lib.rs \
  [--include crates/lisa-plugin/src/tests/signal_consumer_characterization.rs]
```

Never use ordinary `git add` or `git commit`.

## Step 15: verify repository hygiene

Confirm ticket-owned source paths are neither staged, modified, nor untracked.
Ignore only known Lisa-managed ticket/provenance/publication changes.

## Step 16: review handoff

Write `review.md` summarizing changes, tests, exact gate outcomes, coverage
limits, and open concerns. Remain on T-039-03-03 afterward so Lisa can publish
artifacts and perform the completion gate.
