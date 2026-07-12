# Progress: characterize deadline paths

## Status

Implementation is complete. Six characterization tests were added to the
existing plugin test module. No production code was changed.

## Completed work

### Baseline

Before editing ticket-owned source, ran the existing focused coverage:

```text
cargo test -p lisa-plugin deadline
cargo test -p lisa-plugin check_session_timeouts
cargo test -p lisa-plugin detect_stale
cargo test -p lisa-plugin evaluate_health
```

Results:

- deadline filter: 1 passed;
- session timeout filter: 5 passed;
- stale filter: 6 passed;
- health filter: 4 passed;
- no failures.

This confirmed the characterized behavior passed on the unmodified production
tree.

### Acknowledgement policy

Added `characterizes_acknowledgement_deadline_clock_and_recovery_action`.

The test pins:

- stored `ack_deadline` as an absolute clock;
- no firing one nanosecond before the deadline;
- inclusive firing exactly at the deadline;
- successor lease mint and predecessor authority revocation;
- `AssignedPendingAck -> Recovering` action;
- `Idle -> WaitingForExit` slot transition;
- awaiting-human as a deliberate non-exemption that is cleared when the old TUI
  is abandoned.

### Transition policy

Added `characterizes_transition_deadline_and_active_session_exemption`.

The test pins:

- `transition_started_at` as the elapsed transition clock;
- expired exit grace restoring a missing-ticket pane to idle;
- an unexpired exit grace remaining unchanged;
- recent `last_activity_at` exempting an expired clear transition;
- a quiet awaiting-human pane exempting an expired clear transition;
- exit grace intentionally not using the busy-pane exemption.

### Review policy

Added `characterizes_review_deadline_exemptions_and_finish_up_action`.

The test pins:

- `last_phase_change` as the review budget clock;
- recent `last_activity` as the active-session exemption;
- `awaiting_human` as the question exemption;
- `finish_up_sent` and `FinishUpPromptSent` as the qualifying action;
- exempt threads remaining running.

### Health policy

Added
`characterizes_health_deadline_as_observational_for_awaiting_human`.

The test pins:

- `last_activity` against `stuck_threshold_secs` as the health clock;
- cached and logged `Healthy -> Stuck` transition;
- no destructive thread action;
- awaiting-human as intentionally not exempt from observational health display.

### Session policy

Added `characterizes_session_deadline_exemptions_and_timeout_action`.

The test pins:

- `started_at` against the global budget;
- `last_activity` against twice the stuck threshold for destructive action;
- recent activity as the active-session exemption;
- awaiting-human as the hard-silence kill exemption;
- advisory warning tracking for exempt over-budget threads;
- typed timeout outcome, lease revocation, pane fencing, slot release, and thread
  removal for the eligible thread.

### Stale-thread policy

Added `characterizes_stale_deadline_exemptions_and_reclaim_action`.

The test pins:

- `last_activity` against twice the stuck threshold as the stale clock;
- old phase age alone not causing reclamation when activity is recent;
- awaiting-human as the destructive reclaim exemption;
- typed stale outcome, lease revocation, pane fencing, slot release, and thread
  removal for the eligible thread.

## Verification

Formatting:

```text
cargo fmt --all
```

Focused characterization:

```text
cargo test -p lisa-plugin characterizes_
```

Result: 6 passed, 0 failed.

Full plugin suite:

```text
cargo test -p lisa-plugin
```

Result: 321 passed, 0 failed.

Workspace suite:

```text
cargo test --workspace
```

Result: all executed tests passed. The real-Zellij delivery test remained
ignored by its existing environment gate. Relevant totals included 274 CLI
unit tests, 155 core tests, 321 plugin tests, and integration test targets.

## Deviations from plan

- The transition characterization includes an explicit awaiting-human fixture
  in addition to the planned active-session fixture. This makes the policy's two
  clear-transition exemptions independently visible in the new bracket.
- `cargo fmt --all -- --check` initially reported formatting differences in the
  new test block. `cargo fmt --all` applied only mechanical formatting, after
  which tests passed.
- `just check` was not needed because the stronger native workspace test gate
  passed; the assignment acceptance is test-only and does not change WASM code.

## Remaining

- Commit `crates/lisa-plugin/src/lib.rs` through the isolated
  `lisa commit-ticket` transaction with the exact include path.
- Verify no ticket-owned source remains modified or staged.
- Write `review.md` and remain on this ticket for Lisa's completion transaction.
