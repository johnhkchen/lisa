# Design: bounded named-state failure regressions

## Goal

Lock all seven scheduler failure/reclaim paths to a finite transition budget and
an exact `FailureTransitionOutcome`, while leaving lease, seat, thread, pane,
provenance, and retry authority unchanged.

The design must keep the tests deterministic and native. No wall-clock sleeps,
provider processes, Zellij runtime, or model calls are needed.

## Option 1: assert only terminal seat/log text

Existing tests could be renamed and expanded to assert `DeliveryFailed`,
`RecoveryFailed`, `StartupFailed`, reclaim logs, and stable event counts after a
late poll.

Advantages:

- no production signature changes;
- minimal diff;
- existing tests already contain most assertions.

Disadvantages:

- automatic reclaim paths have no retained seat state;
- log strings are presentation, not typed transition identity;
- retained paths would still test typed variants only through isolated helper
  calls rather than the real deadline edge;
- the new predecessor enum could silently become disconnected from orchestration
  while all terminal-state assertions remained green.

This option does not fully lock the named outcome contract.

## Option 2: add a test-only outcome journal to `State`

A `#[cfg(test)]` vector could record every returned outcome. Tests would drive
the current unit-returning evaluator and inspect the journal afterward.

Advantages:

- no production return signature changes;
- all paths could be compared through one stored list.

Disadvantages:

- adds mutable observation state solely for tests;
- every transition helper must remember to record as well as return;
- creates two representations of the same event and risks divergence;
- fixture defaults and state comparisons gain unnecessary test-only baggage;
- production orchestration still visibly discards the named values.

This option is rejected because the return values are already the natural seam.

## Option 3: return a batch from the deadline evaluator

Change `check_assignment_ack_timeouts_at` to collect and return the
`FailureTransitionOutcome` values produced during its scan. Make
`begin_startup_recovery` return an optional outcome for malformed initial
startup authority and startup-recovery preparation failure. The production
wrapper explicitly discards the descriptive batch.

Advantages:

- tests observe the exact real transition edge;
- multiple panes in one scan remain representable and ordered;
- no new scheduler authority or persistent history is introduced;
- mirrors the existing vector-returning error, timeout, and stale scanners;
- no sleeps are required because time remains injected;
- no-op and intermediate retry transitions naturally produce no result.

Disadvantages:

- ignored `Vec` values are `must_use`, so existing test callers need explicit
  discard or assertions;
- `begin_startup_recovery` needs return propagation through its early branches;
- touches orchestration signatures despite no behavioral change.

This is the selected option because it makes the regression evidence direct and
uses the pattern already established by the three reclaim scanners.

## Option 4: one synthetic table test calling helpers directly

A table could create terminal source states, call each helper/scanner, and
compare the seven variants.

Advantages:

- compact inventory;
- almost no signature propagation.

Disadvantages:

- does not prove deadlines advance through retry/relaunch stages;
- can initialize counters at their maxima and bypass the behavior under test;
- duplicates predecessor T-039-03-02's direct helper test;
- weak evidence against infinite retries or silent babysitting.

This option is rejected as insufficient for the ticket's N2 focus.

## Selected result model

`check_assignment_ack_timeouts_at(now)` returns
`Vec<FailureTransitionOutcome>`.

An empty vector means the scan caused no terminal failure: the deadline was not
expired, the state changed since snapshotting, an intermediate bounded retry or
reset began, or the seat was not a scanned state.

One element means a single fixture seat completed its terminal edge. Multiple
elements remain possible in production when several seats expire in one poll.

Results preserve the evaluator's deterministic `BTreeMap` iteration order.
Scheduling does not consume that ordering as authority.

The wall-clock wrapper keeps returning unit and uses `let _ = ...` to state that
poll orchestration currently relies on scheduler mutations, alerts, and logs.

## Startup recovery propagation

`begin_startup_recovery` changes from unit to
`Option<FailureTransitionOutcome>`.

Normal successful entry into `ResettingStartup` returns `None`, because recovery
has begun but no failure transition is complete.

Invalid source state returns `None` and changes nothing.

A missing reservation, missing attempt lease, or stale predecessor returns the
`StartupFailed` value from `fail_startup`.

Successor mint, shell-probe, or recovery-preparation failure returns the
`StartupRecoveryFailed` value from `fail_startup_recovery` when that helper
completes the terminal edge.

The evaluator pushes only `Some` results.

## Other retained paths

Replacement `Starting` expiry and `ResettingStartup` expiry push the result of
`fail_startup_recovery` when present.

Exhausted `Delivering` pushes the result of `fail_assignment_delivery`.

An assignment send error on either initial grace delivery or the single retry
also pushes `AssignmentDeliveryFailed` if the helper accepts the state.

Expired `Recovering` pushes `AssignmentRecoveryFailed`.

The transition from `AssignedPendingAck` into its one fresh recovery produces no
terminal result. Only expiry of that fresh recovery does.

## Regression organization

Use the existing real-path tests as the primary fixtures and add exact batch
assertions at their terminal calls.

Delivery regression:

- first deadline returns empty and changes retry count from zero to one;
- second deadline returns exactly `AssignmentDeliveryFailed`;
- a much later poll returns empty and cannot change terminal state or launch
  count.

Assignment recovery regression:

- fresh recovery expiry returns exactly `AssignmentRecoveryFailed`;
- the successor attempt is exactly predecessor plus one;
- a much later poll returns empty and cannot mint another attempt.

Initial startup regression:

- construct a scheduled initial `Starting` seat;
- remove or invalidate the attempt lease before its first deadline;
- deadline evaluation returns exactly `StartupFailed`;
- terminal state is retained and a later poll is empty.

Startup recovery regression:

- initial deadline begins one reset and returns empty;
- reset deadline returns exactly `StartupRecoveryFailed`;
- later polls return empty and launch count cannot rise;
- retain the replacement-start companion test as proof that an admitted
  relaunch also terminates without a second relaunch.

Automatic scanner regressions retain their current exact outcome assertions and
gain explicit idempotent second-scan assertions where needed. Each scanner has
no internal automatic retry; one consumed condition yields one outcome.

## Seven-path inventory test

The suite's representative tests collectively form the matrix. A concise
comment or helper assertion should name all seven variants so future additions
or omissions are visible during review.

A separate enum-cardinality mechanism is unnecessary and brittle. Exhaustive
Rust matching in a small test helper can require every current variant and
classify retained versus reclaim behavior without assigning numeric tags.

## Clippy decision

Apply clippy's requested `?` rewrites in `fail_startup_recovery` for slot and
ticket lookup. Both expressions already return `None` without side effects.
The rewrite is semantically identical and required for the acceptance gate.

No broad lint cleanup is authorized.

## Rejected semantic changes

Do not change either retry constant, any timeout duration, or readiness mode.

Do not make terminal retained states automatically reschedulable.

Do not add provenance for retained failures.

Do not fence ordinary error panes.

Do not merge timeout and stale provenance outcomes.

Do not persist or render `FailureTransitionOutcome`.

Do not add a live provider/Zellij scenario in this fixture ticket.

## Verification

Focused tests should prove all seven exact variants plus finite counters and
idempotent later scans.

Then run the plugin library suite, workspace suite, strict workspace clippy,
format check, diff check, and `just check` for the WASM compile gate.

The source transaction must include only exact ticket-owned paths and use
`lisa commit-ticket`.
