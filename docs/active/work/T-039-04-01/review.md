# Review: characterize deadline paths

## Outcome

Ticket `T-039-04-01` is implemented and verified. The plugin now has one named
characterization test for each of the six timeout/liveness policies named by the
story: acknowledgement, transition, review, health, session, and stale thread.

The change is test-only. No production function, policy, timeout duration,
exemption, action, configuration, dependency, or public interface changed.

## Source change

Modified:

- `crates/lisa-plugin/src/lib.rs`

The inline test module gained a labeled `Deadline policy characterization`
section containing six tests and their fixtures. The source diff is 377 added
lines and no removed lines.

Committed through Lisa's isolated transaction:

```text
c56cefca4322dfc38be844a2461e5fadb16caf0e
test(plugin): characterize deadline policies
```

The commit contains exactly `crates/lisa-plugin/src/lib.rs`. The ordinary Git
index has no staged ticket-owned path, and the committed source file is clean.
Visible ticket, provenance, and published-work paths are Lisa-managed and were
not included in the source transaction.

## Characterized contracts

### Acknowledgement

`characterizes_acknowledgement_deadline_clock_and_recovery_action` proves:

- the stored acknowledgement deadline is absolute;
- one nanosecond before it does not fire;
- equality does fire;
- the predecessor attempt loses authority;
- a successor attempt is installed;
- the seat enters recovery and the pane waits for exit;
- awaiting-human is not an exemption for abandoning the timed-out TUI and is
  cleared as part of that recovery action.

This policy already had an injected `now`, so its boundary is exact and
deterministic.

### Transition

`characterizes_transition_deadline_and_active_session_exemption` proves:

- `transition_started_at` drives transition expiry;
- an expired missing-ticket exit becomes an idle empty shell;
- an unexpired exit remains waiting;
- recent pane activity defers an expired clear fallback;
- an awaiting-human marker independently defers a quiet expired clear fallback;
- exit grace does not share the stop/clear busy-pane guard.

This captures policy-specific asymmetry instead of implying one generic rule.

### Review

`characterizes_review_deadline_exemptions_and_finish_up_action` proves:

- `last_phase_change` drives review budget expiry;
- recent `last_activity` prevents prompting an active session;
- awaiting-human prevents typing over a question;
- a quiet eligible Review thread receives one recorded finish-up action;
- exempt threads remain running and unmarked as prompted.

### Health

`characterizes_health_deadline_as_observational_for_awaiting_human` proves:

- health uses `last_activity` against `stuck_threshold_secs`;
- expiry records and logs `Healthy -> Stuck`;
- the health action is observational and leaves the thread alive;
- awaiting-human is intentionally not exempt from health display.

That final distinction matters: human-blocked sessions stay visible as stuck
while destructive session/stale actions remain suppressed.

### Session

`characterizes_session_deadline_exemptions_and_timeout_action` proves:

- the global budget uses `started_at`;
- destructive timeout additionally requires hard silence from `last_activity`;
- recent activity retains an over-budget session;
- awaiting-human retains a hard-silent over-budget session;
- retained sessions get advisory warning tracking;
- an eligible timeout returns the typed action, revokes authority, fences and
  releases the pane, and removes the thread.

### Stale thread

`characterizes_stale_deadline_exemptions_and_reclaim_action` proves:

- hard staleness uses `last_activity`, not phase age;
- a recent active session survives even with a very old phase clock;
- a hard-silent awaiting-human session survives;
- an eligible stale thread returns the typed reclaim action, revokes authority,
  fences and releases the pane, and is removed.

## Test coverage

Baseline focused tests were green before source edits:

- deadline filter: 1 passed;
- session timeout filter: 5 passed;
- stale filter: 6 passed;
- health filter: 4 passed.

Post-change verification:

```text
cargo test -p lisa-plugin characterizes_
```

- 6 passed, 0 failed.

```text
cargo test -p lisa-plugin
```

- 321 passed, 0 failed.

```text
cargo test --workspace
```

- all executed workspace tests passed;
- the existing real-Zellij environment-gated test remained ignored;
- key suite totals included 274 CLI, 155 core, and 321 plugin unit tests plus
  integration targets.

Formatting was applied with `cargo fmt --all` and the formatted source compiled
and passed the complete workspace suite.

## Coverage assessment

Acceptance coverage is complete for all six named paths, including both
active-session and awaiting-human behavior. The tests assert observable policy
actions rather than only internal eligibility booleans. Destructive paths use a
current attempt lease and occupied slot, so revocation and fencing are included
in the characterization.

The acknowledgement test checks the exact inclusive boundary. The other five
production methods still read wall time internally. Their tests use one captured
fixture timestamp and wide margins, with no sleeps. Exact injected-clock checks
for those paths appropriately remain the responsibility of `T-039-04-02`.

Per-phase session budgets retain extensive pre-existing coverage and are not
duplicated in the new global session-policy test. The new test focuses on the
story-level clock/exemption/action bracket; existing tests still prove phase
override, fallback, and global-cap behavior.

## Open concerns

- Five policy wrappers remain wall-clock based until the next ticket introduces
  the clock-injected evaluator. The characterization avoids flakiness through
  generous margins but cannot yet assert their exact equality boundary.
- The test block is intentionally verbose because each policy uses a different
  state shape and action. A generic test harness was avoided so it would not
  prematurely dictate the evaluator design or conceal policy differences.
- No live-seat timing observation is included; the story explicitly defers that
  boundary to `S-039-06`.

No critical issue, TODO, failing gate, or uncommitted ticket-owned source remains.
This review is ready for Lisa's completion publication and commit gate.
