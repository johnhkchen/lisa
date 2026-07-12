# Review: clock-injected deadline evaluator

## Outcome

Ticket `T-039-04-02` is implemented and verified.

All six deadline paths now evaluate through one clock-injected evaluator:

- acknowledgement;
- transition;
- review;
- health;
- session;
- stale thread.

The evaluator emits typed per-policy actions. `State` applies those actions
through the existing state-machine effects, keeping adapter I/O, lease fencing,
provenance, logging, and collection mutation outside the pure timing layer.

The T-039-04-01 characterization suite passes unchanged.

## Source changes

Created:

- `crates/lisa-plugin/src/deadline.rs`

Modified:

- `crates/lisa-plugin/src/lib.rs`

No file was deleted. No dependency, configuration field, serialized schema, or
public crate API changed.

Committed through Lisa's isolated transaction:

```text
e732266577f869dc196f4d09329fc72f2e988992
refactor(plugin): centralize deadline evaluation
```

The commit contains exactly the two source paths above. Both are clean, and the
ordinary Git index has no staged path.

## Evaluator architecture

The new private module defines a small `Clock` trait and production
`SystemClock`. `DeadlineEvaluator::new(clock)` samples the injected clock once
and stores the resulting `SystemTime`.

One stored instant is used for every comparison in an evaluation call. Review
also uses that instant for its resulting phase-clock reset. This eliminates the
previous independent wall-clock samples inside the six deadline traversals.

Each policy has a purpose-built input and action type. The design intentionally
does not flatten materially different policies into a generic timeout event.

## Preserved policy contracts

### Acknowledgement

- Absolute deadline comparison remains inclusive.
- The existing `_at(now)` seam remains available.
- Captured seat state is still revalidated before action.
- Startup, delivery, recovery, and failure branches are unchanged.
- Active and awaiting-human sessions are not exempt.

### Transition

- Transition age still uses whole seconds and strict `>` thresholds.
- Exit expiry still ignores recent activity and awaiting-human state.
- Stop and clear still require wind-down quietness.
- Stop and clear remain awaiting-human exempt.
- Provider launch, `/clear`, prompt delivery, and slot effects are unchanged.

### Review

- Zero continues to disable review prompting.
- Eligibility still requires a running Review thread.
- Phase age and activity quietness remain independent clocks.
- Previously prompted and awaiting-human threads remain exempt.
- Adapter follow-up and idempotence effects are unchanged.

### Health

- Running health still uses `last_activity` and an inclusive threshold.
- Failed health remains immediate.
- Awaiting-human remains visible as observationally stuck.
- First observation, transition logging, removal pruning, and cached
  parked/completed behavior are preserved.

### Session

- Global budget retains precedence over the per-phase budget.
- Pending completions remain excluded.
- Budget overrun remains advisory until hard silence.
- Active and awaiting-human overruns remain retained and warned once.
- Eligible reclaim still fails, fences, records provenance, releases, removes,
  alerts, logs, and returns a typed outcome.

### Stale thread

- Hard silence remains twice the stuck threshold.
- Pending completions, recent activity, and awaiting-human remain exempt.
- Eligible reclaim retains the full fail/fence/provenance/release/remove action.

## Test coverage

New fixed-clock unit coverage directly drives all six evaluator methods at
deterministic boundaries. It checks acknowledgement future/equality behavior,
transition strictness, Review/health/session/stale inclusive thresholds, and
the policy-specific awaiting-human actions.

The unchanged characterization suite continues to cover end-to-end state
effects for each policy, including lease revocation and fencing on destructive
paths.

Verification results:

```text
cargo test -p lisa-plugin deadline::tests --no-fail-fast
2 passed, 0 failed

cargo test -p lisa-plugin characterizes_ --no-fail-fast
6 passed, 0 failed

cargo test -p lisa-plugin --no-fail-fast
323 passed, 0 failed

cargo test --workspace --no-fail-fast
all executed tests passed

just check
WASM check passed; repeated workspace suite passed

git diff --check
passed
```

The existing environment-gated real-Zellij test remained ignored. No new live
Zellij timing test was needed because the evaluator is pure and all stateful
effects retain native coverage.

## Acceptance assessment

The acceptance criterion is satisfied:

- all six named paths delegate evaluation to `DeadlineEvaluator`;
- an injected fixed clock deterministically fires all six policies;
- each policy returns a typed action matching its distinct behavior;
- active-session and awaiting-human exemptions are preserved;
- the T-039-04-01 characterization tests are unchanged and green;
- plugin, workspace, and WASM gates are green.

## Open concerns

- The evaluator inputs intentionally copy selected state fields. Adding a new
  deadline-relevant state field requires updating the corresponding input
  construction and policy test.
- Transition preserves its historical strict whole-second comparison, unlike
  the inclusive comparisons used by the other policies. This asymmetry is now
  explicit and tested, but a future behavior change would need a separate ticket.
- Stateful action bodies remain in `State`. This is deliberate: moving pane I/O,
  fencing, provenance, and adapter logic into a timing evaluator would couple
  unrelated responsibilities and enlarge the mutation boundary.

There is no critical issue, failing gate, uncommitted ticket-owned source, or
known acceptance gap. This review is ready for Lisa's completion publication
and commit gate.
