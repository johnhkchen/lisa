# Design: deadline characterization tests

## Goal

Add a compact characterization suite that makes the six existing deadline
contracts explicit before story `S-039-04` changes their traversal and clock
plumbing. The suite must prove behavior on the current implementation, without
changing production logic.

Each characterization makes three facts reviewable: which state field supplies
time, which exemption applies, and which action occurs when the policy fires.
The policies intentionally remain different. Characterization must not imply
that every path shares one threshold, exemption, or action.

## Options considered

### Rely on existing scattered tests

The module already has extensive tests for each behavior. Making no source
change would avoid duplication and wall-clock risk. However, clock, exemption,
and action are split across historical tests, and there is no discoverable
six-policy contract to bracket the upcoming refactor. Rejected because the
ticket explicitly asks for characterization tests.

### Introduce clock injection here

Adding `_at(now)` variants for every method would permit exact boundaries, but
that is the scope of dependent ticket `T-039-04-02`. It would alter production
structure before the characterization bracket exists. Rejected to preserve the
story's characterize-then-refactor sequence.

### Add one named characterization per policy

Place six focused tests together in the inline test module. Each uses current
private methods and existing helpers. A test may contain paired fixtures when
both firing and exemption are essential.

This is chosen because it creates a discoverable contract, leaves production
code untouched, and remains compatible with later internal refactoring. Some
coverage overlaps existing tests, but the new suite expresses the policy matrix
rather than isolated historical incidents.

### Table-drive a generic policy fixture

A generic enum or closure table would reduce repeated assertion structure, but
the state shapes differ genuinely: acknowledgement operates on seats,
transition on slots, and other policies primarily on threads. Generic machinery
would obscure exemptions and prematurely design the production evaluator.
Rejected in favor of explicit policy fixtures.

## Chosen contracts

### Acknowledgement

Use existing `check_assignment_ack_timeouts_at(now)`. Construct an
`AssignedPendingAck` seat with a current attempt lease. Call one nanosecond
before the deadline and assert no state change. Call exactly at the deadline and
assert recovery:

- predecessor lease loses authority;
- successor lease is installed;
- seat enters `Recovering`;
- slot enters `WaitingForExit`.

This pins the absolute deadline and inclusive comparison. An awaiting-human
marker is present to prove it is not an exemption; recovery clears the marker
because it abandons the old TUI.

### Transition

Use `WaitingForExit` with no ticket reservation to avoid provider launch and
host input. Create one slot within `AGENT_EXIT_GRACE_SECS` and one beyond it.
Assert the first remains waiting and the second becomes an idle empty shell.

Also use an expired `WaitingForClear` slot with recent activity and assert it
remains unchanged. This pins the asymmetry that quietness guards stop/clear but
not exit grace. Awaiting-human transition deferral is already covered nearby
and can be asserted again if the combined fixture remains host-safe.

### Review

Create running Review threads past `review_timeout_secs`: one with recent
activity, one quiet but awaiting-human, and one quiet/non-awaiting. Assert only
the third enters `finish_up_sent` and logs `FinishUpPromptSent`.

This pins `last_phase_change` as the phase clock, `last_activity` against
`wind_down_secs` as the active-session exemption, `awaiting_human` as the human
exemption, and the one-time finish-up record as the action.

### Health

Create a running thread whose `last_activity` is past
`stuck_threshold_secs`, and mark its pane awaiting-human. Assert
`Healthy -> Stuck` is cached and logged. The marker deliberately proves it is
not a health exemption. Health is observational, so stuck display does not
conflict with protection from destructive reclaim.

### Session

Create three globally over-budget threads: recently active; hard-silent and
awaiting-human; hard-silent and non-awaiting. Assert only the third returns
`SessionTimedOut` and is removed/fenced. Assert the exempt threads remain and
receive advisory warning tracking.

The clocks are `started_at` for budget and `last_activity` for the hard-silence
gate. Per-phase timing already has extensive nearby coverage and need not be
duplicated inside this global characterization.

### Stale thread

Create three running threads: recently active despite an old phase clock;
hard-silent and awaiting-human; hard-silent and non-awaiting. Assert only the
third returns `StaleThreadReclaimed` and is removed/fenced. This pins
`last_activity`, not phase age, as the relevant clock.

## Time determinism

Acknowledgement uses an exact injected timestamp. Other policies capture one
fixture-local `now` and place timestamps comfortably on either side of their
configured durations. Tests do not sleep and do not assert exact elapsed
seconds. The future evaluator may make these exact without requiring the
characterization suite to change.

## Source organization

Add a labeled characterization section in the existing test module. Use the
existing `install_current_attempt` helper and state types. Add no public helper,
module, dependency, clock trait, or production `_at` method.

## Verification

Run relevant existing plugin tests before source edits. After adding tests, run
the shared characterization prefix, the complete plugin target, and workspace
tests. Format the source and inspect the diff before committing through Lisa.

## Risks

- Pane I/O can fail on native tests; choose locally observable branches or the
  test-mode capture already exercised by nearby tests.
- Failure actions emit provenance; use the existing best-effort behavior.
- `HashMap` traversal is unordered; do not assert outcome order when multiple
  destructive candidates exist.
- Wall-clock tests need wide margins rather than exact-boundary assertions.
- Test names and comments must stay policy-oriented for the next ticket's gate.
