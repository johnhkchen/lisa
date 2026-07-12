# Progress: clock-injected deadline evaluator

## Status

Implementation is complete and all verification gates are green.

## Baseline

Before source edits:

```text
cargo test -p lisa-plugin characterizes_
```

- 6 passed
- 0 failed
- 315 filtered out

The ordinary worktree already contained Lisa-managed changes to:

- `.lisa/provenance.jsonl`
- `docs/active/tickets/T-039-04-02.md`

No ticket-owned source path was modified or staged.

## Completed implementation

### Evaluator module

Created `crates/lisa-plugin/src/deadline.rs`.

It defines:

- the `Clock` trait;
- the production `SystemClock`;
- `DeadlineEvaluator`, which samples an injected clock once at construction;
- typed policy inputs;
- typed per-policy actions;
- acknowledgement, transition, review, health, session, and stale evaluation;
- a common saturating elapsed helper;
- fixed-clock unit tests.

### Acknowledgement integration

- Replaced the seat deadline filter with evaluator inputs and actions.
- Retained `check_assignment_ack_timeouts_at(now)` unchanged at its call boundary.
- Retained state revalidation before applying a captured seat action.
- Retained all seat-variant recovery and terminal effects.

### Transition integration

- Replaced slot deadline traversal with `TransitionInput` records.
- Evaluator actions distinguish exit, stop, and clear policies.
- Preserved strict whole-second `>` thresholds.
- Preserved exit's lack of active/human exemption.
- Moved stop/clear quietness and awaiting-human suppression into evaluation.
- Retained existing provider launch, clear, prompt, and state effects.

### Review integration

- Replaced Review thread filters with evaluator inputs.
- Preserved disabled-zero configuration, running/phase filters, idempotence,
  activity quietness, and awaiting-human suppression.
- Retained adapter follow-up effects and event logging.
- The evaluator's single sampled time now also resets the phase clock.

### Health integration

- Replaced health timing calculations with evaluator observations.
- Preserved transition logging and first-observation insertion.
- Preserved the legacy rule that cached parked/completed health is retained.
- Preserved awaiting-human visibility as observationally stuck.
- Retained cache pruning in `State`.

### Session integration

- Replaced global/phase budget and silence traversal with evaluator actions.
- Actions explicitly distinguish advisory warning from destructive reclaim.
- Preserved global-before-phase precedence.
- Preserved pending-completion, activity, and awaiting-human behavior.
- Retained warning idempotence and the full fencing/provenance/removal action.

### Stale integration

- Replaced hard-silence traversal with evaluator inputs/actions.
- Preserved running, pending-completion, active-session, and awaiting-human rules.
- Retained the full fencing/provenance/removal action and typed outcome.

## Added tests

`fixed_clock_drives_all_six_policies_at_their_boundaries` uses one fixed clock to
drive deterministic firing for all six policy methods.

It covers:

- acknowledgement equality and future exclusion;
- strict transition exit firing and its exemption asymmetry;
- review equality for phase budget and quietness;
- health equality at the stuck threshold;
- session equality at both budget and hard silence;
- stale equality at hard silence.

`policy_specific_exemptions_are_preserved` covers:

- awaiting-human transition stop suppression;
- awaiting-human review suppression;
- awaiting-human session conversion to advisory warning;
- awaiting-human stale suppression.

The existing health characterization continues to prove that awaiting-human is
not an exemption from observational health.

## Verification

```text
cargo test -p lisa-plugin deadline::tests --no-fail-fast
```

- 2 passed
- 0 failed

```text
cargo test -p lisa-plugin characterizes_ --no-fail-fast
```

- 6 passed
- 0 failed
- characterization source block unchanged

```text
cargo test -p lisa-plugin --no-fail-fast
```

- 323 passed
- 0 failed

```text
cargo test --workspace --no-fail-fast
```

- all workspace tests passed
- plugin: 323 passed
- CLI: 274 passed
- core: 155 passed
- environment-gated real-Zellij test remained ignored

```text
just check
```

- `cargo check -p lisa-plugin --target wasm32-wasip1` passed
- repeated full workspace tests passed

```text
git diff --check
```

- passed with no whitespace errors

## Deviations from plan

- The evaluator samples and stores its clock at construction rather than holding
  a generic clock for repeated sampling. This strengthens the intended invariant:
  every policy comparison and associated timestamp in one evaluation observes
  one instant.
- No integration-only `_with_evaluator` wrappers were added. Pure evaluator tests
  prove injected-clock firing, while the unchanged characterization tests prove
  the `State` action integrations. This keeps the state API surface small.
- Health inputs are limited to running/failed threads plus unseen cache entries,
  preserving the existing cached parked/completed behavior discovered during
  diff review.

## Remaining work

- Write `review.md` and stop on the current ticket.

## Source commit

Committed through Lisa's isolated transaction:

```text
e732266577f869dc196f4d09329fc72f2e988992
refactor(plugin): centralize deadline evaluation
```

The commit contains exactly:

- `crates/lisa-plugin/src/deadline.rs`
- `crates/lisa-plugin/src/lib.rs`

Both ticket-owned source paths are clean. The ordinary index has no staged
paths. Remaining visible changes are Lisa-managed ticket, provenance, and
published-work state.
