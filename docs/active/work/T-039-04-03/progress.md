# Progress: cross-policy deadline regression

## Status

Implementation is complete and all pre-commit verification gates are green.

## Baseline

Before editing ticket-owned source, the worktree contained Lisa-managed changes:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-039-04-03.md`;
- Lisa-published phase artifacts under `docs/active/work/T-039-04-03/`.

`crates/lisa-plugin/src/deadline.rs` was clean and unstaged.

Baseline evaluator coverage:

```text
cargo test -p lisa-plugin deadline::tests --no-fail-fast
2 passed, 0 failed
```

Baseline state characterization:

```text
cargo test -p lisa-plugin characterizes_ --no-fail-fast
6 passed, 0 failed
```

## Implemented source unit

Modified only:

- `crates/lisa-plugin/src/deadline.rs`.

No production evaluator logic, input type, action type, visibility, configuration,
dependency, or state integration changed.

Added two fixed-clock unit tests.

## Exact action regression

Added:

```text
cross_policy_deadline_actions_remain_distinct
```

The test uses a fixed evaluator at Unix second 100 and asserts the native typed
result of every deadline family.

Acknowledgement coverage:

- inclusive expired deadline;
- exact pane identity;
- exact captured assignment state.

Transition coverage:

- exact `ExitReady` action with pane and ticket;
- exact `StopTimedOut` action with pane;
- exact `ClearTimedOut` action with pane and ticket;
- preserved input/action order.

Review coverage:

- exact eligible ticket identity;
- exact pane identity;
- dedicated `ReviewAction`, rather than a generic timeout result.

Health coverage:

- exact ticket identity;
- previous Healthy observation;
- current Stuck observation;
- observational result remains distinct from a command.

Session coverage:

- exact destructive `Reclaim` variant;
- ticket and pane identity;
- elapsed seconds from the global budget;
- Implement phase payload;
- global-before-phase precedence in the fixture.

Stale coverage:

- exact `StaleAction` ticket and pane identity.

## Cross-policy exemption matrix

Added:

```text
cross_policy_activity_and_human_exemptions_remain_distinct
```

The test compares recent activity and awaiting-human behavior across policies.

Acknowledgement:

- still emits at expiry;
- evaluator intentionally has no activity/human exemption inputs.

Transition exit:

- recent activity does not exempt;
- awaiting-human does not exempt;
- both emit exact `ExitReady` actions.

Transition stop and clear:

- recent activity suppresses both;
- awaiting-human suppresses both;
- none of their four candidates appears in output.

Review:

- recent activity suppresses action;
- awaiting-human suppresses action;
- quiet non-human candidate alone emits the exact Review action.

Health:

- recent activity produces Healthy;
- quiet activity produces Stuck;
- awaiting-human remains deliberately outside the evaluator input, preserving
  the observational non-exemption proved by the state characterization.

Session:

- recent activity converts expiry to `Warn`;
- awaiting-human converts expiry to `Warn`;
- quiet non-human expiry emits `Reclaim`;
- all three exact deadline payloads are asserted in order.

Stale:

- recent activity suppresses reclaim;
- awaiting-human suppresses reclaim;
- quiet non-human candidate alone emits the exact stale action.

## Focused verification

```text
cargo test -p lisa-plugin cross_policy_ --no-fail-fast
2 passed, 0 failed
```

```text
cargo test -p lisa-plugin deadline::tests --no-fail-fast
4 passed, 0 failed
```

```text
cargo test -p lisa-plugin characterizes_ --no-fail-fast
6 passed, 0 failed
```

The six existing state characterization tests were not edited.

## Broad verification

```text
cargo test -p lisa-plugin --no-fail-fast
325 passed, 0 failed
```

```text
cargo test --workspace --no-fail-fast
all executed workspace tests passed
plugin: 325 passed
CLI: 274 passed
core: 155 passed
```

The existing environment-gated real-Zellij test remained ignored.

## Acceptance gates

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
passed
```

```text
just check
WASM check passed
workspace tests passed
```

```text
cargo fmt --all -- --check
passed
```

```text
git diff --check
passed
```

## Diff assessment

- One ticket-owned source file is modified.
- The source diff contains test code only.
- No existing test or production line was changed.
- The ordinary Git index has no staged path.
- Lisa-managed ticket, provenance, and published artifact changes remain outside
  the ticket-owned source unit.

## Deviations from plan

- Both tests were added in one edit before the first focused compile, rather
  than compiling between their additions. They form one atomic test unit and
  the combined focused run passed immediately.
- Lisa automatically materialized admitted phase artifacts under the shared
  active-work path while the attempt continued. The agent wrote artifacts only
  to the required attempt-private directory.
- No helper extraction was needed; policy-specific inputs and typed assertions
  remain inline and visible.

## Source transaction

```text
lisa commit-ticket --ticket-id T-039-04-03 \
  --message "test(plugin): lock cross-policy deadline contracts" \
  --include crates/lisa-plugin/src/deadline.rs
```

Lisa committed:

```text
62234bc69c904ef7440d41e4af60b92a5c78948d
test(plugin): lock cross-policy deadline contracts
```

The commit contains exactly:

- `crates/lisa-plugin/src/deadline.rs`.

Post-commit verification confirmed:

- the ticket-owned source path is clean;
- the ordinary index is empty;
- remaining status entries are Lisa-managed only;
- both `cross_policy_` tests pass from committed source.

## Remaining work

- Write `review.md` and stop on this ticket.
