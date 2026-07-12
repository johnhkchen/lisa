# Review: failure/reclaim state-machine map

## Outcome

The ticket acceptance criteria are satisfied. `progress.md` contains a documented
state-machine map for all seven required failure/reclaim paths and an invariant
test matrix comparing lease, seat, thread, pane, provenance, and retry teardown.
The matrix passes on the current production implementation.

The work also closes the only direct characterization gap found during baseline
inspection: assignment recovery failure now has a native test which pins its
complete terminal authority vector without changing production behavior.

## Artifacts produced

- `research.md` maps the current scheduler authorities, functions, and test
  surfaces.
- `design.md` evaluates documentation formats and chooses a transition map plus
  authority matrix.
- `structure.md` defines artifact boundaries, matrix columns, and downstream
  use.
- `plan.md` sequences characterization and records the evidence-driven test-only
  deviation.
- `progress.md` is the acceptance-facing state map, invariant matrix, ordering
  contract, test mapping, and verification record.
- `review.md` is this handoff.

All phase artifacts were written to the attempt-private work directory. Lisa is
responsible for admission and publication to `docs/active/work/T-039-03-01/`.

## Source change

One test-only change was made in `crates/lisa-plugin/src/lib.rs`:

`assignment_recovery_failure_retains_authority_for_operator_reset`

The test establishes a current reused-Codex assignment, drives the real
`begin_assignment_recovery` transition, arms the real recovery deadline, and
drives `check_assignment_ack_timeouts_at` to `RecoveryFailed`.

It asserts:

- exactly one successor attempt is minted;
- the successor is current and high-water;
- slot and thread attempt authority are updated to the successor;
- terminal seat state is `RecoveryFailed`;
- the thread is failed but remains in the thread map;
- the ticket and attempt reservation remain on the slot;
- the abandoned resident session is not treated as live or fenced;
- no provenance record is written for the retained failure;
- the error alert is present exactly once;
- a later deadline poll cannot create another recovery attempt.

No production function, enum, constant, consumer order, retry bound, lease rule,
pane action, or provenance behavior changed.

## Commit

The ticket-owned test source was committed through Lisa's isolated transaction:

```text
ea0d9af6e7bc1abf86b2c8114341d2eab9981a75
test: pin assignment recovery failure invariants
```

The transaction included exactly:

```text
crates/lisa-plugin/src/lib.rs
```

The ordinary Git index was not used and is empty. No ticket-owned source remains
modified or untracked. The remaining working-tree entries are Lisa-managed
ticket, provenance, and work-publication state.

## State-machine findings

Four paths are operator-retained terminal failures:

1. Assignment delivery exhausts one same-generation chat retry and ends at
   `DeliveryFailed` with lease, failed thread, reservation, and pane retained.
2. Assignment recovery mints one successor and ends at `RecoveryFailed` with
   successor authority and reservation retained.
3. Initial startup failure ends at `StartupFailed` without automatically
   releasing the pane.
4. Exhausted same-pane startup recovery ends at `StartupFailed`, revokes the
   successor, and fences the pane while retaining the failed reservation for
   operator inspection.

Three paths automatically reclaim scheduler ownership:

1. Ordinary `.error` emits failed/non-fenced provenance, releases the slot,
   removes the thread, and preserves a reusable resident pane session.
2. Session timeout requires both budget exhaustion and hard silence, then
   revokes, fences, emits timed-out provenance, releases, and removes.
3. Stale-thread detection uses the same hard-silence fence/release shape but
   emits failed provenance and no timeout alert.

These differences are load-bearing. A later outcome type must expose them rather
than normalizing every failure into one cleanup routine.

## Test coverage

Baseline production tree before the added characterization:

```text
cargo test -p lisa-plugin --lib
312 passed; 0 failed
```

Focused characterization:

```text
cargo test -p lisa-plugin --lib \
  assignment_recovery_failure_retains_authority_for_operator_reset
1 passed; 0 failed
```

Final complete plugin library suite:

```text
cargo test -p lisa-plugin --lib
313 passed; 0 failed
```

Formatting:

```text
cargo fmt --all -- --check
pass
```

Existing fixtures already pin delivery exhaustion, startup reset/fence behavior,
error consumption and provenance, hard-silence ordering, timeout guards,
staleness guards, monotonic redispatch, and stale-attempt rejection. The new
fixture supplies the missing direct assignment-recovery terminal vector.

## Coverage limits

- The matrix is native-fixture proof, not a live Zellij/provider run. S-039-06
  owns live-seat evidence.
- The new assignment recovery test arms the recovery acknowledgement deadline
  directly after driving the real recovery start. Existing transition fixtures
  separately cover exit-grace launch mechanics.
- Retained-failure provenance is asserted as absence. Disk-write failure behavior
  remains covered by the general provenance contract, not repeated per path.
- The startup-failure row includes both invalid initial recovery authority and
  exhausted replacement startup; the matrix keeps their pane-fencing difference
  explicit.
- Timeout/liveness deadline unification is intentionally outside this story.

## Downstream requirements

T-039-03-02 must keep this matrix green while adding named/typed transition
outcomes. It must preserve:

- one delivery retry, one assignment successor, and one startup relaunch;
- operator retention for terminal assignment/startup failures;
- automatic release only for error/timeout/stale reclaim;
- fence-before-release ordering for hard silence;
- non-fencing behavior for ordinary error signals;
- `TimedOut` versus `Failed` provenance outcomes;
- provenance-before-thread-removal ordering;
- high-water retention and monotonic successor minting;
- awaiting-human and pending-completion reclaim guards.

T-039-03-03 may reorganize the tests around the new named outcomes, but should
retain this test's full authority assertions rather than reducing it to a single
enum comparison.

## Open concerns

No critical issue blocks handoff. The principal architectural concern is that
authority remains distributed across independent maps and structs, so later
refactoring must compare the full invariant vector—not only the returned outcome
name. The documented matrix is intended to make that review mechanical.

This attempt is complete through Review. Lisa should now perform its completion
publication/commit gate; this seat must remain on T-039-03-01 until that succeeds.
