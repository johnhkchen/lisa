# Design: cross-policy deadline regression

## Objective

Add regression coverage that makes the six deadline families comparable without
flattening their intentionally different contracts. The tests must fail if a
policy emits a different action, loses identifying payload, or changes how
recent activity and awaiting-human affect the result. Production behavior is
not changed by this ticket.

## Option 1: rely on existing characterization tests

The six `lib.rs` tests already exercise state-level outcomes.

Advantages:

- no source change;
- broad effect coverage already exists;
- provider and fencing behavior remains represented.

Disadvantages:

- each policy is isolated in its own fixture;
- comparison requires mentally joining six tests;
- exact evaluator actions are not uniformly asserted;
- recent-activity and awaiting-human differences are incomplete as a matrix;
- the ticket explicitly asks for new cross-policy regression tests.

Decision: reject. Existing characterization is necessary integration coverage,
but it does not satisfy the requested comparative regression boundary.

## Option 2: add a state-level cross-policy fixture in `lib.rs`

One test could construct slots and threads for all six policies, invoke every
`State` checker, and assert resulting state and outcomes.

Advantages:

- covers real state-machine effects;
- can observe fencing, logs, prompts, and collection mutation;
- aligns with the earlier characterization location.

Disadvantages:

- each checker mutates shared state and can interfere with later checkers;
- acknowledgement recovery and reclaim policies alter leases and slots;
- fixture setup would dominate the policy matrix;
- wall-clock entry points make exact timing less direct;
- action identity inside the evaluator remains indirect;
- existing characterization already covers the state-effect layer.

Decision: reject. It duplicates expensive setup while giving a less precise
assertion over the newly centralized policy boundary.

## Option 3: snapshot debug strings

The evaluator results could be formatted and compared with one string snapshot.

Advantages:

- compact expected value;
- visually presents a matrix-like result;
- changes to enum variant names are obvious.

Disadvantages:

- several result structs do not currently derive `Debug`;
- adding derives only for snapshots changes non-test definitions;
- debug formatting is weaker than typed assertions;
- snapshots can encourage blind regeneration;
- heterogeneous policies require normalization glue.

Decision: reject. Typed assertions provide clearer compiler-assisted failures.

## Option 4: fixed-clock evaluator regressions

Add two tests beside the evaluator's existing fixed-clock unit tests.

The first drives all six policy families with expired candidates and asserts
their exact typed actions and payloads. Transition contributes all three action
variants. Session contributes its destructive result with its complete deadline
payload.

The second constructs recent-activity and awaiting-human cases side by side and
asserts this policy matrix:

| Policy | recent activity | awaiting human |
|---|---|---|
| acknowledgement | no evaluator exemption | no evaluator exemption |
| transition exit | still `ExitReady` | still `ExitReady` |
| transition stop | suppressed | suppressed |
| transition clear | suppressed | suppressed |
| review | suppressed | suppressed |
| health | `Healthy` | still `Stuck` when quiet |
| session | `Warn` | `Warn` |
| stale | suppressed | suppressed |

Advantages:

- deterministic fixed time;
- direct coverage of the centralized boundary;
- typed actions remain distinct in test source;
- one test reads as an action catalog;
- one test reads as an exemption matrix;
- minimal setup and no mutable-state interference;
- complements rather than duplicates state characterization.

Disadvantages:

- does not itself execute state-layer effects;
- heterogeneous result types prevent one Rust literal table;
- acknowledgement has no activity or human fields to toggle.

Decision: choose this option. Existing state characterization supplies effect
coverage, while evaluator tests supply the comparative contract.

## Test 1: exact cross-policy actions

Use `evaluator(100)` and timestamps relative to Unix epoch.

Acknowledgement:

- submit an expired deadline with pane `11` and state `"expired"`;
- assert exactly one action;
- assert pane and captured state exactly.

Transition:

- submit quiet, non-human exit, stop, and clear candidates;
- give each a distinct pane and ticket identity where supported;
- use start time zero so every strict threshold is exceeded;
- assert exact ordered `ExitReady`, `StopTimedOut`, and `ClearTimedOut` actions.

Review:

- submit an eligible running Review thread;
- assert one action with exact ticket and pane.

Health:

- submit an overdue running thread with previous Healthy status;
- assert ticket, previous status, and current Stuck status.

Session:

- submit a silent, over-global-budget running thread;
- assert exact `Reclaim(SessionDeadline { ... })`;
- include elapsed seconds and phase so payload changes fail visibly.

Stale:

- submit a silent running thread;
- assert the exact `StaleAction` value.

Results remain in native typed values. No normalized generic timeout enum is
introduced because it would obscure the distinct production types being locked.

## Test 2: exemption/action matrix

Use one fixed evaluator at second 100.

Acknowledgement:

- assert expiry produces its captured action;
- document that activity and awaiting-human are intentionally not inputs.

Transition:

- create recent and awaiting-human candidates for exit, stop, and clear;
- expect both exit candidates to fire;
- expect all stop/clear candidates to be absent;
- assert the exact two exit actions.

Review:

- submit active, awaiting-human, and eligible candidates;
- assert only the eligible candidate action.

Health:

- submit a recently active thread and a quiet thread representing a pane that
  may be awaiting human;
- assert Healthy then Stuck;
- absence of an awaiting-human input is part of the contract.

Session:

- submit active, awaiting-human, and reclaimable expired candidates;
- assert ordered `Warn`, `Warn`, and `Reclaim` actions;
- use distinct identities and exact deadline payloads.

Stale:

- submit active, awaiting-human, and reclaimable candidates;
- assert only the reclaimable action.

## Assertion strategy

- Prefer `assert_eq!` where action types implement equality and debug.
- For structs without those derives, assert slice shape and exact fields.
- Use `matches!` only when it specifies every relevant payload field.
- Preserve iterator ordering in expected results.
- Give fixtures policy-specific ticket and pane identities.
- Avoid helpers that hide which input field controls each result.

## Test naming

Use names exposing the requested contract:

- `cross_policy_deadline_actions_remain_distinct`;
- `cross_policy_activity_and_human_exemptions_remain_distinct`.

The prefix supports a focused test filter and makes coverage easy to find.

## File decision

Modify only:

- `crates/lisa-plugin/src/deadline.rs`.

No production file, fixture directory, dependency, or configuration changes.
Inline placement provides private access without widening visibility.

## Verification design

Run in increasing scope:

1. current evaluator tests as a baseline;
2. new `cross_policy_` filter;
3. all `deadline::tests`;
4. all plugin tests;
5. full workspace tests;
6. workspace Clippy with all targets and features;
7. `just check` for WASM check and repeated suite;
8. formatting and whitespace checks.

Clippy is explicit acceptance, so `cargo check` is not a substitute.

## Commit decision

The two regressions form one meaningful source unit. Commit the single exact
path through Lisa:

```text
lisa commit-ticket --ticket-id T-039-04-03 \
  --message "test(plugin): lock cross-policy deadline contracts" \
  --include crates/lisa-plugin/src/deadline.rs
```

Attempt-private phase artifacts stay outside the implementation transaction.

## Risks and mitigations

- Risk: tests restate implementation rather than behavior.
  Mitigation: assert meaningful action names, identities, and matrix outcomes
  without copying evaluator control flow.
- Risk: health awaiting-human behavior appears untested because the evaluator
  has no flag.
  Mitigation: name the quiet fixture explicitly and rely on unchanged state
  characterization to prove the marker remains present.
- Risk: state effects regress while evaluator tests pass.
  Mitigation: run characterization and full plugin suites.
- Risk: a generic helper erases policy differences.
  Mitigation: keep typed per-policy assertions visible.

## Final rationale

The selected design turns the story boundary into executable evidence: one
injected clock feeds distinct inputs and produces distinct typed results. The
regression does not imply that all timeouts share one exemption or action. It
makes their differences deterministic and hard to change accidentally.
